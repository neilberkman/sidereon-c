use super::*;

// === Round-2 NMEA parse, accumulation, and GGA writing =======================

pub const NMEA_TALKER_C_BYTES: usize = 3;

pub struct SidereonNmeaLog {
    pub(crate) epochs: Vec<sidereon_core::nmea::EpochSnapshot>,
    pub(crate) sentence_count: usize,
    pub(crate) skip_count: usize,
    pub(crate) warning_count: usize,
}

pub struct SidereonNmeaAccumulator {
    pub(crate) inner: sidereon_core::nmea::NmeaAccumulator,
    pub(crate) epochs: Vec<sidereon_core::nmea::EpochSnapshot>,
    pub(crate) sentence_count: usize,
    pub(crate) skip_count: usize,
    pub(crate) warning_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonNmeaSummary {
    pub sentence_count: usize,
    pub epoch_count: usize,
    pub skip_count: usize,
    pub warning_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonNmeaChunkSummary {
    pub sentence_count: usize,
    pub completed_epoch_count: usize,
    pub skip_count: usize,
    pub warning_count: usize,
    pub retained_len: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonNmeaEpochSummary {
    pub has_calendar_epoch: bool,
    pub calendar_epoch: SidereonCalendarEpoch,
    pub has_position: bool,
    pub position: SidereonGeodetic,
    pub has_instant_j2000_s: bool,
    pub instant_j2000_s: f64,
    pub has_pdop: bool,
    pub pdop: f64,
    pub has_hdop: bool,
    pub hdop: f64,
    pub has_vdop: bool,
    pub vdop: f64,
    pub used_satellite_count: usize,
    pub satellites_in_view: usize,
    pub sentence_count: usize,
    pub skip_count: usize,
    pub warning_count: usize,
    pub has_gga: bool,
    pub has_rmc: bool,
    pub has_gll: bool,
    pub gsa_count: usize,
    pub gsv_group_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonNmeaGgaOptions {
    pub talker: [c_char; NMEA_TALKER_C_BYTES],
    pub utc_seconds_of_day: f64,
    pub position: SidereonGeodetic,
    pub quality: u32,
    pub satellites_used: u8,
    pub hdop: f64,
    pub coordinate_decimals: u8,
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_parse(
    data: *const u8,
    len: usize,
    out_log: *mut *mut SidereonNmeaLog,
) -> SidereonStatus {
    ffi_boundary("sidereon_nmea_parse", SidereonStatus::Panic, || {
        let out_log = c_try!(require_out(out_log, "sidereon_nmea_parse", "out_log"));
        *out_log = ptr::null_mut();
        let bytes = c_try!(require_slice(data, len, "sidereon_nmea_parse", "data"));
        let log = nmea_log_from_parsed(sidereon_core::nmea::parse_nmea(bytes));
        write_boxed_handle(out_log, log);
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_log_summary(
    log: *const SidereonNmeaLog,
    out_summary: *mut SidereonNmeaSummary,
) -> SidereonStatus {
    ffi_boundary("sidereon_nmea_log_summary", SidereonStatus::Panic, || {
        let out = c_try!(require_out(
            out_summary,
            "sidereon_nmea_log_summary",
            "out_summary"
        ));
        let log = c_try!(require_ref(log, "sidereon_nmea_log_summary", "log"));
        *out = nmea_summary(
            log.sentence_count,
            &log.epochs,
            log.skip_count,
            log.warning_count,
        );
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_log_epochs(
    log: *const SidereonNmeaLog,
    out: *mut SidereonNmeaEpochSummary,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_nmea_log_epochs", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_nmea_log_epochs",
            out_written,
            out_required
        ));
        let log = c_try!(require_ref(log, "sidereon_nmea_log_epochs", "log"));
        let values: Vec<_> = log.epochs.iter().map(nmea_epoch_summary_to_c).collect();
        c_try!(copy_prefix_to_c(
            "sidereon_nmea_log_epochs",
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
pub unsafe extern "C" fn sidereon_nmea_log_free(log: *mut SidereonNmeaLog) {
    free_boxed(log);
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_accumulator_new(
    out_accumulator: *mut *mut SidereonNmeaAccumulator,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_nmea_accumulator_new",
        SidereonStatus::Panic,
        || {
            let out_accumulator = c_try!(require_out(
                out_accumulator,
                "sidereon_nmea_accumulator_new",
                "out_accumulator"
            ));
            *out_accumulator = ptr::null_mut();
            write_boxed_handle(
                out_accumulator,
                SidereonNmeaAccumulator {
                    inner: sidereon_core::nmea::NmeaAccumulator::new(),
                    epochs: Vec::new(),
                    sentence_count: 0,
                    skip_count: 0,
                    warning_count: 0,
                },
            );
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_accumulator_push(
    accumulator: *mut SidereonNmeaAccumulator,
    data: *const u8,
    len: usize,
    out_summary: *mut SidereonNmeaChunkSummary,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_nmea_accumulator_push",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_summary,
                "sidereon_nmea_accumulator_push",
                "out_summary"
            ));
            *out = SidereonNmeaChunkSummary {
                sentence_count: 0,
                completed_epoch_count: 0,
                skip_count: 0,
                warning_count: 0,
                retained_len: 0,
            };
            let accumulator = c_try!(require_mut(
                accumulator,
                "sidereon_nmea_accumulator_push",
                "accumulator"
            ));
            let bytes = c_try!(require_slice(
                data,
                len,
                "sidereon_nmea_accumulator_push",
                "data"
            ));
            let output = accumulator.inner.push_bytes(bytes);
            let sentence_count = output.sentences.len();
            let mut skip_count = output.diagnostics.skips.len();
            let mut warning_count = output.diagnostics.warnings.len();
            let completed_epoch_count = output.snapshots.len();
            for epoch in &output.snapshots {
                let (epoch_skips, epoch_warnings) = nmea_epoch_diagnostic_counts(epoch);
                skip_count += epoch_skips;
                warning_count += epoch_warnings;
            }
            accumulator.sentence_count += sentence_count;
            accumulator.skip_count += skip_count;
            accumulator.warning_count += warning_count;
            accumulator.epochs.extend(output.snapshots);
            *out = SidereonNmeaChunkSummary {
                sentence_count,
                completed_epoch_count,
                skip_count,
                warning_count,
                retained_len: accumulator.inner.retained_len(),
            };
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_accumulator_finish(
    accumulator: *mut SidereonNmeaAccumulator,
    out_summary: *mut SidereonNmeaChunkSummary,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_nmea_accumulator_finish",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_summary,
                "sidereon_nmea_accumulator_finish",
                "out_summary"
            ));
            *out = SidereonNmeaChunkSummary {
                sentence_count: 0,
                completed_epoch_count: 0,
                skip_count: 0,
                warning_count: 0,
                retained_len: 0,
            };
            let accumulator = c_try!(require_mut(
                accumulator,
                "sidereon_nmea_accumulator_finish",
                "accumulator"
            ));
            let Some(epoch) = accumulator.inner.finish() else {
                return SidereonStatus::Ok;
            };
            let (skip_count, warning_count) = nmea_epoch_diagnostic_counts(&epoch);
            accumulator.skip_count += skip_count;
            accumulator.warning_count += warning_count;
            accumulator.epochs.push(epoch);
            *out = SidereonNmeaChunkSummary {
                sentence_count: 0,
                completed_epoch_count: 1,
                skip_count,
                warning_count,
                retained_len: accumulator.inner.retained_len(),
            };
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_accumulator_summary(
    accumulator: *const SidereonNmeaAccumulator,
    out_summary: *mut SidereonNmeaSummary,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_nmea_accumulator_summary",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_summary,
                "sidereon_nmea_accumulator_summary",
                "out_summary"
            ));
            let accumulator = c_try!(require_ref(
                accumulator,
                "sidereon_nmea_accumulator_summary",
                "accumulator"
            ));
            *out = nmea_summary(
                accumulator.sentence_count,
                &accumulator.epochs,
                accumulator.skip_count,
                accumulator.warning_count,
            );
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_accumulator_epochs(
    accumulator: *const SidereonNmeaAccumulator,
    out: *mut SidereonNmeaEpochSummary,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_nmea_accumulator_epochs",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_nmea_accumulator_epochs",
                out_written,
                out_required
            ));
            let accumulator = c_try!(require_ref(
                accumulator,
                "sidereon_nmea_accumulator_epochs",
                "accumulator"
            ));
            let values: Vec<_> = accumulator
                .epochs
                .iter()
                .map(nmea_epoch_summary_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_nmea_accumulator_epochs",
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
pub unsafe extern "C" fn sidereon_nmea_accumulator_retained_len(
    accumulator: *const SidereonNmeaAccumulator,
    out_len: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_nmea_accumulator_retained_len",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_len,
                "sidereon_nmea_accumulator_retained_len",
                "out_len"
            ));
            let accumulator = c_try!(require_ref(
                accumulator,
                "sidereon_nmea_accumulator_retained_len",
                "accumulator"
            ));
            *out = accumulator.inner.retained_len();
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_accumulator_free(accumulator: *mut SidereonNmeaAccumulator) {
    free_boxed(accumulator);
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_write_gga(
    options: *const SidereonNmeaGgaOptions,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_nmea_write_gga", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_nmea_write_gga",
            out_written,
            out_required
        ));
        let options = c_try!(require_ref(options, "sidereon_nmea_write_gga", "options"));
        let talker = c_try!(nmea_talker_from_c(
            "sidereon_nmea_write_gga",
            &options.talker
        ));
        let time = match sidereon_core::nmea::NmeaTime::from_seconds_of_day_floor_centis(
            options.utc_seconds_of_day,
        ) {
            Ok(time) => time,
            Err(err) => {
                set_last_error(format!("sidereon_nmea_write_gga: {err}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        let position = c_try!(geodetic_to_wgs84(
            "sidereon_nmea_write_gga",
            "position",
            options.position
        ));
        let quality = c_try!(nmea_gga_quality_from_c(
            "sidereon_nmea_write_gga",
            options.quality
        ));
        let gga = match sidereon_core::nmea::Gga::vrs_position(
            position,
            time,
            quality,
            options.satellites_used,
            options.hdop,
            options.coordinate_decimals,
        ) {
            Ok(gga) => gga,
            Err(err) => {
                set_last_error(format!("sidereon_nmea_write_gga: {err}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        let sentence = match sidereon_core::nmea::write_gga(talker, &gga) {
            Ok(sentence) => sentence,
            Err(err) => {
                set_last_error(format!("sidereon_nmea_write_gga: {err}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        c_try!(copy_prefix_to_c(
            "sidereon_nmea_write_gga",
            "out",
            sentence.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

fn nmea_summary(
    sentence_count: usize,
    epochs: &[sidereon_core::nmea::EpochSnapshot],
    skip_count: usize,
    warning_count: usize,
) -> SidereonNmeaSummary {
    SidereonNmeaSummary {
        sentence_count,
        epoch_count: epochs.len(),
        skip_count,
        warning_count,
    }
}

fn nmea_epoch_summary_to_c(epoch: &sidereon_core::nmea::EpochSnapshot) -> SidereonNmeaEpochSummary {
    let calendar_epoch = nmea_epoch_calendar(epoch);
    let position = epoch.position();
    let pdop = epoch.pdop();
    let hdop = epoch.hdop();
    let vdop = epoch.vdop();
    let (skip_count, warning_count) = nmea_epoch_diagnostic_counts(epoch);
    let instant_j2000_s = calendar_epoch
        .map(|epoch| {
            sidereon_core::astro::time::civil::j2000_seconds(
                epoch.year,
                epoch.month,
                epoch.day,
                epoch.hour,
                epoch.minute,
                epoch.second,
            )
        })
        .unwrap_or(0.0);

    SidereonNmeaEpochSummary {
        has_calendar_epoch: calendar_epoch.is_some(),
        calendar_epoch: calendar_epoch.unwrap_or(SidereonCalendarEpoch {
            year: 0,
            month: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0.0,
        }),
        has_position: position.is_some(),
        position: position
            .as_ref()
            .map(geodetic_to_c)
            .unwrap_or_else(empty_geodetic),
        has_instant_j2000_s: calendar_epoch.is_some(),
        instant_j2000_s,
        has_pdop: pdop.is_some(),
        pdop: pdop.unwrap_or(0.0),
        has_hdop: hdop.is_some(),
        hdop: hdop.unwrap_or(0.0),
        has_vdop: vdop.is_some(),
        vdop: vdop.unwrap_or(0.0),
        used_satellite_count: epoch.used_satellites().count(),
        satellites_in_view: epoch.satellites_in_view(),
        sentence_count: epoch.sentence_count,
        skip_count,
        warning_count,
        has_gga: epoch.gga.is_some(),
        has_rmc: epoch.rmc.is_some(),
        has_gll: epoch.gll.is_some(),
        gsa_count: epoch.gsa.len(),
        gsv_group_count: epoch.gsv.len(),
    }
}

fn nmea_gga_quality_from_c(
    fn_name: &str,
    quality: u32,
) -> Result<sidereon_core::nmea::GgaQuality, SidereonStatus> {
    if quality > u32::from(u8::MAX) {
        set_last_error(format!("{fn_name}: invalid GGA quality {quality}"));
        return Err(SidereonStatus::InvalidArgument);
    }
    Ok(match quality as u8 {
        0 => sidereon_core::nmea::GgaQuality::Invalid,
        1 => sidereon_core::nmea::GgaQuality::GpsSps,
        2 => sidereon_core::nmea::GgaQuality::Differential,
        3 => sidereon_core::nmea::GgaQuality::Pps,
        4 => sidereon_core::nmea::GgaQuality::RtkFixed,
        5 => sidereon_core::nmea::GgaQuality::RtkFloat,
        6 => sidereon_core::nmea::GgaQuality::Estimated,
        7 => sidereon_core::nmea::GgaQuality::Manual,
        8 => sidereon_core::nmea::GgaQuality::Simulator,
        other => sidereon_core::nmea::GgaQuality::Other(other),
    })
}

fn nmea_talker_from_c(
    fn_name: &str,
    talker: &[c_char; NMEA_TALKER_C_BYTES],
) -> Result<sidereon_core::nmea::NmeaTalker, SidereonStatus> {
    let talker = fixed_c_array_to_string(fn_name, "talker", talker)?;
    if talker.len() != 2 || !talker.bytes().all(|byte| byte.is_ascii()) {
        set_last_error(format!("{fn_name}: talker must be exactly two ASCII bytes"));
        return Err(SidereonStatus::InvalidArgument);
    }
    Ok(sidereon_core::nmea::NmeaTalker::parse(&talker))
}

fn nmea_log_from_parsed(
    parsed: sidereon_core::nmea::Parsed<sidereon_core::nmea::NmeaLog>,
) -> SidereonNmeaLog {
    let sentence_count = parsed.value.sentences.len();
    let mut skip_count = parsed.diagnostics.skips.len();
    let mut warning_count = parsed.diagnostics.warnings.len();
    let epochs = sidereon_core::nmea::group_epochs(&parsed.value);
    for epoch in &epochs {
        let (epoch_skips, epoch_warnings) = nmea_epoch_diagnostic_counts(epoch);
        skip_count += epoch_skips;
        warning_count += epoch_warnings;
    }
    SidereonNmeaLog {
        epochs,
        sentence_count,
        skip_count,
        warning_count,
    }
}

fn nmea_epoch_diagnostic_counts(epoch: &sidereon_core::nmea::EpochSnapshot) -> (usize, usize) {
    (
        epoch.diagnostics.skips.len(),
        epoch.diagnostics.warnings.len(),
    )
}

fn nmea_epoch_calendar(
    epoch: &sidereon_core::nmea::EpochSnapshot,
) -> Option<SidereonCalendarEpoch> {
    let date = epoch.date?;
    let time = epoch.time_of_day?;
    Some(SidereonCalendarEpoch {
        year: i32::from(date.year),
        month: i32::from(date.month),
        day: i32::from(date.day),
        hour: i32::from(time.hour),
        minute: i32::from(time.minute),
        second: f64::from(time.second) + f64::from(time.nanos) * 1.0e-9,
    })
}
