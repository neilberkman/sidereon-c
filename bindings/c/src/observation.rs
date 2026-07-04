use super::*;

/// One pseudorange observation: a null-terminated satellite token, for example
/// G08, and its measured pseudorange in meters.
#[repr(C)]
pub struct SidereonObservation {
    /// Null-terminated satellite token, for example G08. The terminator must
    /// appear within 16 bytes.
    pub sat_id: *const c_char,
    /// Measured pseudorange in meters.
    pub pseudorange_m: f64,
}

pub struct SidereonObservationQcReport {
    pub(crate) inner: sidereon_core::observation_qc::ObservationQcReport,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonObservationQcIntervalSource {
    Override = 0,
    Header = 1,
    Inferred = 2,
    Unresolved = 3,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonObservationQcOptions {
    pub has_interval_override_s: bool,
    pub interval_override_s: f64,
    pub gap_factor: f64,
    pub clock_jump_threshold_s: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonObservationQcSummary {
    pub total_epoch_records: usize,
    pub observation_epochs: usize,
    pub event_records: usize,
    pub power_failure_epochs: usize,
    pub skipped_records: usize,
    pub has_interval_s: bool,
    pub interval_s: f64,
    pub interval_source: u32,
    pub missing_epochs: usize,
    pub data_gap_count: usize,
    pub satellite_count: usize,
    pub satellite_signal_count: usize,
    pub system_signal_count: usize,
    pub note_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonObservationQcDataGap {
    pub start_epoch: SidereonCalendarEpoch,
    pub end_epoch: SidereonCalendarEpoch,
    pub nominal_interval_s: f64,
    pub observed_delta_s: f64,
    pub missing_epochs: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonObservationQcClockJump {
    pub epoch_index: usize,
    pub epoch: SidereonCalendarEpoch,
    pub delta_s: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonObservationQcCycleSlips {
    pub observations: usize,
    pub total_slips: usize,
    pub has_observations_per_slip: bool,
    pub observations_per_slip: f64,
    pub system_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonObservationQcSystemCycleSlip {
    pub system: u32,
    pub observations: usize,
    pub slips: usize,
    pub has_observations_per_slip: bool,
    pub observations_per_slip: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonObservationQcMpStats {
    pub n: usize,
    pub rms_m: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonObservationQcSatelliteMultipath {
    pub sat_id: SidereonSatelliteToken,
    pub has_mp1: bool,
    pub mp1: SidereonObservationQcMpStats,
    pub has_mp2: bool,
    pub mp2: SidereonObservationQcMpStats,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonObservationQcSystemMultipath {
    pub system: u32,
    pub has_mp1: bool,
    pub mp1: SidereonObservationQcMpStats,
    pub has_mp2: bool,
    pub mp2: SidereonObservationQcMpStats,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonObservationQcSatellite {
    pub sat_id: SidereonSatelliteToken,
    pub epochs_with_observations: usize,
    pub value_observations: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonObservationQcSignal {
    pub sat_id: SidereonSatelliteToken,
    pub system: u32,
    pub code: [c_char; RINEX_OBS_CODE_C_BYTES],
    pub value_observations: usize,
    pub has_ssi: bool,
    pub ssi_counts: [u64; 10],
    pub has_snr: bool,
    pub snr_n: usize,
    pub snr_mean: f64,
    pub snr_min: f64,
    pub snr_max: f64,
    pub has_snr_std: bool,
    pub snr_std: f64,
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_observation_qc_options_init(
    out_options: *mut SidereonObservationQcOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_observation_qc_options_init",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_options,
                "sidereon_observation_qc_options_init",
                "out_options"
            ));
            let defaults = sidereon_core::observation_qc::ObservationQcOptions::default();
            *out = SidereonObservationQcOptions {
                has_interval_override_s: defaults.interval_override_s.is_some(),
                interval_override_s: defaults.interval_override_s.unwrap_or(0.0),
                gap_factor: defaults.gap_factor,
                clock_jump_threshold_s: defaults.clock_jump_threshold_s,
            };
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_observation_qc_from_obs(
    obs: *const SidereonRinexObs,
    options: *const SidereonObservationQcOptions,
    out_report: *mut *mut SidereonObservationQcReport,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_observation_qc_from_obs",
        SidereonStatus::Panic,
        || {
            let out_report = c_try!(require_out(
                out_report,
                "sidereon_observation_qc_from_obs",
                "out_report"
            ));
            *out_report = ptr::null_mut();
            let obs = c_try!(require_ref(obs, "sidereon_observation_qc_from_obs", "obs"));
            match sidereon_core::observation_qc::observation_qc_with_options(
                &obs.inner,
                observation_qc_options_from_c(options),
            ) {
                Ok(inner) => {
                    write_boxed_handle(out_report, SidereonObservationQcReport { inner });
                    SidereonStatus::Ok
                }
                Err(err) => {
                    set_last_error(format!("sidereon_observation_qc_from_obs: {err}"));
                    SidereonStatus::InvalidArgument
                }
            }
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_observation_qc_parse(
    data: *const u8,
    len: usize,
    options: *const SidereonObservationQcOptions,
    out_report: *mut *mut SidereonObservationQcReport,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_observation_qc_parse",
        SidereonStatus::Panic,
        || {
            let out_report = c_try!(require_out(
                out_report,
                "sidereon_observation_qc_parse",
                "out_report"
            ));
            *out_report = ptr::null_mut();
            let text = c_try!(text_bytes_from_c(
                "sidereon_observation_qc_parse",
                data,
                len
            ));
            let obs = match RinexObs::parse(text) {
                Ok(obs) => obs,
                Err(err) => {
                    set_last_error(format!("sidereon_observation_qc_parse: {err}"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            match sidereon_core::observation_qc::observation_qc_with_options(
                &obs,
                observation_qc_options_from_c(options),
            ) {
                Ok(inner) => {
                    write_boxed_handle(out_report, SidereonObservationQcReport { inner });
                    SidereonStatus::Ok
                }
                Err(err) => {
                    set_last_error(format!("sidereon_observation_qc_parse: {err}"));
                    SidereonStatus::InvalidArgument
                }
            }
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_observation_qc_summary(
    report: *const SidereonObservationQcReport,
    out_summary: *mut SidereonObservationQcSummary,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_observation_qc_summary",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_summary,
                "sidereon_observation_qc_summary",
                "out_summary"
            ));
            let report = c_try!(require_ref(
                report,
                "sidereon_observation_qc_summary",
                "report"
            ));
            *out = qc_summary_to_c(&report.inner);
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_observation_qc_gaps(
    report: *const SidereonObservationQcReport,
    out: *mut SidereonObservationQcDataGap,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_observation_qc_gaps",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_observation_qc_gaps",
                out_written,
                out_required
            ));
            let report = c_try!(require_ref(
                report,
                "sidereon_observation_qc_gaps",
                "report"
            ));
            let values: Vec<_> = report
                .inner
                .data_gaps
                .iter()
                .map(|gap| SidereonObservationQcDataGap {
                    start_epoch: rinex_epoch_time_to_c(gap.start_epoch),
                    end_epoch: rinex_epoch_time_to_c(gap.end_epoch),
                    nominal_interval_s: gap.nominal_interval_s,
                    observed_delta_s: gap.observed_delta_s,
                    missing_epochs: gap.missing_epochs,
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_observation_qc_gaps",
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
pub unsafe extern "C" fn sidereon_observation_qc_satellites(
    report: *const SidereonObservationQcReport,
    out: *mut SidereonObservationQcSatellite,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_observation_qc_satellites",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_observation_qc_satellites",
                out_written,
                out_required
            ));
            let report = c_try!(require_ref(
                report,
                "sidereon_observation_qc_satellites",
                "report"
            ));
            let values: Vec<_> = report
                .inner
                .satellites
                .iter()
                .map(|row| SidereonObservationQcSatellite {
                    sat_id: satellite_token(row.satellite),
                    epochs_with_observations: row.epochs_with_observations,
                    value_observations: row.value_observations,
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_observation_qc_satellites",
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
pub unsafe extern "C" fn sidereon_observation_qc_satellite_signals(
    report: *const SidereonObservationQcReport,
    out: *mut SidereonObservationQcSignal,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_observation_qc_satellite_signals",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_observation_qc_satellite_signals",
                out_written,
                out_required
            ));
            let report = c_try!(require_ref(
                report,
                "sidereon_observation_qc_satellite_signals",
                "report"
            ));
            let values: Vec<_> = report
                .inner
                .satellite_signals
                .iter()
                .map(|row| {
                    observation_qc_signal_from_parts(
                        satellite_token(row.satellite),
                        row.satellite.system,
                        &row.code,
                        row.value_observations,
                        row.ssi,
                        row.snr,
                    )
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_observation_qc_satellite_signals",
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
pub unsafe extern "C" fn sidereon_observation_qc_system_signals(
    report: *const SidereonObservationQcReport,
    out: *mut SidereonObservationQcSignal,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_observation_qc_system_signals",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_observation_qc_system_signals",
                out_written,
                out_required
            ));
            let report = c_try!(require_ref(
                report,
                "sidereon_observation_qc_system_signals",
                "report"
            ));
            let values: Vec<_> = report
                .inner
                .system_signals
                .iter()
                .map(|row| {
                    observation_qc_signal_from_parts(
                        observation_qc_signal_empty_sat(),
                        row.system,
                        &row.code,
                        row.value_observations,
                        row.ssi,
                        row.snr,
                    )
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_observation_qc_system_signals",
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
pub unsafe extern "C" fn sidereon_observation_qc_clock_jumps(
    report: *const SidereonObservationQcReport,
    out: *mut SidereonObservationQcClockJump,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_observation_qc_clock_jumps",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_observation_qc_clock_jumps",
                out_written,
                out_required
            ));
            let report = c_try!(require_ref(
                report,
                "sidereon_observation_qc_clock_jumps",
                "report"
            ));
            let values: Vec<_> = report
                .inner
                .clock_jumps
                .iter()
                .map(|row| SidereonObservationQcClockJump {
                    epoch_index: row.epoch_index,
                    epoch: rinex_epoch_time_to_c(row.epoch),
                    delta_s: row.delta_s,
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_observation_qc_clock_jumps",
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
pub unsafe extern "C" fn sidereon_observation_qc_cycle_slips(
    report: *const SidereonObservationQcReport,
    out_cycle_slips: *mut SidereonObservationQcCycleSlips,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_observation_qc_cycle_slips",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_cycle_slips,
                "sidereon_observation_qc_cycle_slips",
                "out_cycle_slips"
            ));
            let report = c_try!(require_ref(
                report,
                "sidereon_observation_qc_cycle_slips",
                "report"
            ));
            *out = observation_qc_cycle_slips_to_c(&report.inner.cycle_slips);
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_observation_qc_cycle_slip_systems(
    report: *const SidereonObservationQcReport,
    out: *mut SidereonObservationQcSystemCycleSlip,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_observation_qc_cycle_slip_systems",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_observation_qc_cycle_slip_systems",
                out_written,
                out_required
            ));
            let report = c_try!(require_ref(
                report,
                "sidereon_observation_qc_cycle_slip_systems",
                "report"
            ));
            let values: Vec<_> = report
                .inner
                .cycle_slips
                .by_system
                .iter()
                .map(observation_qc_system_cycle_slip_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_observation_qc_cycle_slip_systems",
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
pub unsafe extern "C" fn sidereon_observation_qc_multipath_satellites(
    report: *const SidereonObservationQcReport,
    out: *mut SidereonObservationQcSatelliteMultipath,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_observation_qc_multipath_satellites",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_observation_qc_multipath_satellites",
                out_written,
                out_required
            ));
            let report = c_try!(require_ref(
                report,
                "sidereon_observation_qc_multipath_satellites",
                "report"
            ));
            let values: Vec<_> = report
                .inner
                .multipath
                .satellites
                .iter()
                .map(|row| {
                    let (has_mp1, mp1) = observation_qc_mp_stats_to_c(row.mp1);
                    let (has_mp2, mp2) = observation_qc_mp_stats_to_c(row.mp2);
                    SidereonObservationQcSatelliteMultipath {
                        sat_id: satellite_token(row.satellite),
                        has_mp1,
                        mp1,
                        has_mp2,
                        mp2,
                    }
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_observation_qc_multipath_satellites",
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
pub unsafe extern "C" fn sidereon_observation_qc_multipath_systems(
    report: *const SidereonObservationQcReport,
    out: *mut SidereonObservationQcSystemMultipath,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_observation_qc_multipath_systems",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_observation_qc_multipath_systems",
                out_written,
                out_required
            ));
            let report = c_try!(require_ref(
                report,
                "sidereon_observation_qc_multipath_systems",
                "report"
            ));
            let values: Vec<_> = report
                .inner
                .multipath
                .systems
                .iter()
                .map(|row| {
                    let (has_mp1, mp1) = observation_qc_mp_stats_to_c(row.mp1);
                    let (has_mp2, mp2) = observation_qc_mp_stats_to_c(row.mp2);
                    SidereonObservationQcSystemMultipath {
                        system: gnss_system_to_c(row.system) as u32,
                        has_mp1,
                        mp1,
                        has_mp2,
                        mp2,
                    }
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_observation_qc_multipath_systems",
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

/// Render an observation QC report as text. Variable-length output contract.
/// Delegates to sidereon_core::observation_qc::render_text.
///
/// Safety: report is a live handle; out points to len writable bytes or NULL
/// when len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_observation_qc_render_text(
    report: *const SidereonObservationQcReport,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_observation_qc_render_text",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_observation_qc_render_text",
                out_written,
                out_required
            ));
            let report = c_try!(require_ref(
                report,
                "sidereon_observation_qc_render_text",
                "report"
            ));
            observation_qc_copy_string(
                "sidereon_observation_qc_render_text",
                sidereon_core::observation_qc::render_text(&report.inner),
                out,
                len,
                out_written,
                out_required,
            )
        },
    )
}

/// Render an observation QC report as HTML. Variable-length output contract.
/// Delegates to sidereon_core::observation_qc::render_html.
///
/// Safety: report is a live handle; out points to len writable bytes or NULL
/// when len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_observation_qc_render_html(
    report: *const SidereonObservationQcReport,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_observation_qc_render_html",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_observation_qc_render_html",
                out_written,
                out_required
            ));
            let report = c_try!(require_ref(
                report,
                "sidereon_observation_qc_render_html",
                "report"
            ));
            observation_qc_copy_string(
                "sidereon_observation_qc_render_html",
                sidereon_core::observation_qc::render_html(&report.inner),
                out,
                len,
                out_written,
                out_required,
            )
        },
    )
}

/// Serialize an observation QC report as JSON. Variable-length output contract.
///
/// Safety: report is a live handle; out points to len writable bytes or NULL
/// when len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_observation_qc_to_json(
    report: *const SidereonObservationQcReport,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_observation_qc_to_json",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_observation_qc_to_json",
                out_written,
                out_required
            ));
            let report = c_try!(require_ref(
                report,
                "sidereon_observation_qc_to_json",
                "report"
            ));
            let text = match serde_json::to_string(&report.inner) {
                Ok(text) => text,
                Err(err) => {
                    set_last_error(format!("sidereon_observation_qc_to_json: {err}"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            observation_qc_copy_string(
                "sidereon_observation_qc_to_json",
                text,
                out,
                len,
                out_written,
                out_required,
            )
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_observation_qc_report_free(
    report: *mut SidereonObservationQcReport,
) {
    free_boxed(report);
}

fn observation_qc_options_from_c(
    options: *const SidereonObservationQcOptions,
) -> sidereon_core::observation_qc::ObservationQcOptions {
    let options = unsafe { options.as_ref() }
        .copied()
        .unwrap_or(SidereonObservationQcOptions {
            has_interval_override_s: false,
            interval_override_s: 0.0,
            gap_factor: 1.5,
            clock_jump_threshold_s: sidereon_core::observation_qc::DEFAULT_CLOCK_JUMP_THRESHOLD_S,
        });
    sidereon_core::observation_qc::ObservationQcOptions {
        interval_override_s: options
            .has_interval_override_s
            .then_some(options.interval_override_s),
        gap_factor: options.gap_factor,
        clock_jump_threshold_s: options.clock_jump_threshold_s,
    }
}

fn qc_summary_to_c(
    report: &sidereon_core::observation_qc::ObservationQcReport,
) -> SidereonObservationQcSummary {
    SidereonObservationQcSummary {
        total_epoch_records: report.total_epoch_records,
        observation_epochs: report.observation_epochs,
        event_records: report.event_records,
        power_failure_epochs: report.power_failure_epochs,
        skipped_records: report.skipped_records,
        has_interval_s: report.interval_s.is_some(),
        interval_s: report.interval_s.unwrap_or(0.0),
        interval_source: qc_interval_source_to_c(report.interval_source),
        missing_epochs: report.missing_epochs,
        data_gap_count: report.data_gaps.len(),
        satellite_count: report.satellites.len(),
        satellite_signal_count: report.satellite_signals.len(),
        system_signal_count: report.system_signals.len(),
        note_count: report.notes.len(),
    }
}

fn observation_qc_signal_from_parts(
    sat_id: SidereonSatelliteToken,
    system: GnssSystem,
    code: &str,
    value_observations: usize,
    ssi: Option<sidereon_core::observation_qc::SsiHistogram>,
    snr: Option<sidereon_core::observation_qc::SnrStats>,
) -> SidereonObservationQcSignal {
    let mut out = SidereonObservationQcSignal {
        sat_id,
        system: gnss_system_to_c(system) as u32,
        code: fixed_c_chars::<RINEX_OBS_CODE_C_BYTES>(code),
        value_observations,
        has_ssi: false,
        ssi_counts: [0; 10],
        has_snr: false,
        snr_n: 0,
        snr_mean: 0.0,
        snr_min: 0.0,
        snr_max: 0.0,
        has_snr_std: false,
        snr_std: 0.0,
    };
    if let Some(ssi) = ssi {
        out.has_ssi = true;
        out.ssi_counts = ssi.counts;
    }
    if let Some(snr) = snr {
        out.has_snr = true;
        out.snr_n = snr.n;
        out.snr_mean = snr.mean;
        out.snr_min = snr.min;
        out.snr_max = snr.max;
        out.has_snr_std = snr.std.is_some();
        out.snr_std = snr.std.unwrap_or(0.0);
    }
    out
}

fn observation_qc_cycle_slips_to_c(
    cycle_slips: &sidereon_core::observation_qc::CycleSlipQc,
) -> SidereonObservationQcCycleSlips {
    SidereonObservationQcCycleSlips {
        observations: cycle_slips.observations,
        total_slips: cycle_slips.total_slips,
        has_observations_per_slip: cycle_slips.observations_per_slip.is_some(),
        observations_per_slip: cycle_slips.observations_per_slip.unwrap_or(0.0),
        system_count: cycle_slips.by_system.len(),
    }
}

fn observation_qc_system_cycle_slip_to_c(
    row: &sidereon_core::observation_qc::SystemCycleSlipQc,
) -> SidereonObservationQcSystemCycleSlip {
    SidereonObservationQcSystemCycleSlip {
        system: gnss_system_to_c(row.system) as u32,
        observations: row.observations,
        slips: row.slips,
        has_observations_per_slip: row.observations_per_slip.is_some(),
        observations_per_slip: row.observations_per_slip.unwrap_or(0.0),
    }
}

fn observation_qc_mp_stats_to_c(
    stats: Option<sidereon_core::observation_qc::MpStats>,
) -> (bool, SidereonObservationQcMpStats) {
    match stats {
        Some(stats) => (
            true,
            SidereonObservationQcMpStats {
                n: stats.n,
                rms_m: stats.rms_m,
            },
        ),
        None => (false, SidereonObservationQcMpStats { n: 0, rms_m: 0.0 }),
    }
}

unsafe fn observation_qc_copy_string(
    fn_name: &str,
    text: String,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
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
}

fn qc_interval_source_to_c(source: sidereon_core::observation_qc::IntervalSource) -> u32 {
    match source {
        sidereon_core::observation_qc::IntervalSource::Override => {
            SidereonObservationQcIntervalSource::Override as u32
        }
        sidereon_core::observation_qc::IntervalSource::Header => {
            SidereonObservationQcIntervalSource::Header as u32
        }
        sidereon_core::observation_qc::IntervalSource::Inferred => {
            SidereonObservationQcIntervalSource::Inferred as u32
        }
        sidereon_core::observation_qc::IntervalSource::Unresolved => {
            SidereonObservationQcIntervalSource::Unresolved as u32
        }
    }
}
