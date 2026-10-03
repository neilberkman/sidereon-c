use super::*;

pub struct SidereonExactEpoch {
    pub(crate) inner: sidereon_core::astro::time::ExactEpoch,
}

pub struct SidereonExactEpochQuery {
    pub(crate) inner: sidereon_core::astro::time::ExactEpochQuery,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonExactOrdering {
    Less = -1,
    Equal = 0,
    Greater = 1,
}

/// Write the fixed inter-system time-scale offset to_reading - from_reading
/// (seconds) to *out_offset_s: the value that, added to a from-scale reading,
/// yields the to-scale reading of the same instant. Both scales are
/// SidereonTimeScale values. Fails with SIDEREON_STATUS_INVALID_ARGUMENT if
/// either scale is UTC-based (UTC/GLONASST), whose offset is epoch-dependent (use
/// sidereon_timescale_offset_at_s), or for TDB.
///
/// Safety: out_offset_s must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_timescale_offset_s(
    from: u32,
    to: u32,
    out_offset_s: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_timescale_offset_s", SidereonStatus::Panic, || {
        let out_offset_s = c_try!(require_out(
            out_offset_s,
            "sidereon_timescale_offset_s",
            "out_offset_s"
        ));
        *out_offset_s = 0.0;
        let from = c_try!(time_scale_from_c_code(
            "sidereon_timescale_offset_s",
            "from",
            from
        ));
        let to = c_try!(time_scale_from_c_code(
            "sidereon_timescale_offset_s",
            "to",
            to
        ));
        match timescale_offset_s(from, to) {
            Ok(offset) => {
                *out_offset_s = offset;
                SidereonStatus::Ok
            }
            Err(err) => time_offset_error_to_status("sidereon_timescale_offset_s", err),
        }
    })
}

/// Write the leap-aware inter-system time-scale offset to_reading - from_reading
/// (seconds) at the UTC instant utc_jd to *out_offset_s. utc_jd is the UTC
/// Julian date, used only to resolve the leap-second count when from or to is
/// UTC-based (UTC/GLONASST); it is ignored for purely atomic pairs. Fails with
/// SIDEREON_STATUS_INVALID_ARGUMENT for TDB or when a UTC-based scale is named
/// with a non-finite utc_jd.
///
/// Safety: out_offset_s must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_timescale_offset_at_s(
    from: u32,
    to: u32,
    utc_jd: f64,
    out_offset_s: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_timescale_offset_at_s",
        SidereonStatus::Panic,
        || {
            let out_offset_s = c_try!(require_out(
                out_offset_s,
                "sidereon_timescale_offset_at_s",
                "out_offset_s"
            ));
            *out_offset_s = 0.0;
            let from = c_try!(time_scale_from_c_code(
                "sidereon_timescale_offset_at_s",
                "from",
                from
            ));
            let to = c_try!(time_scale_from_c_code(
                "sidereon_timescale_offset_at_s",
                "to",
                to
            ));
            match timescale_offset_at_s(from, to, utc_jd) {
                Ok(offset) => {
                    *out_offset_s = offset;
                    SidereonStatus::Ok
                }
                Err(err) => time_offset_error_to_status("sidereon_timescale_offset_at_s", err),
            }
        },
    )
}

/// Leap-second table metadata.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonLeapSecondTableInfo {
    /// First Modified Julian Date covered by the table.
    pub first_mjd: i32,
    /// Last Modified Julian Date with a leap-second step.
    pub last_mjd: i32,
    /// Number of table entries.
    pub entries: usize,
    /// Byte length of the provenance string, excluding a terminator.
    pub source_len: usize,
}

/// UT1 and delta-T table metadata.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonUt1CoverageInfo {
    /// First Modified Julian Date in the UT1 table.
    pub first_mjd: i32,
    /// Last Modified Julian Date in the UT1 table.
    pub last_mjd: i32,
    /// First covered instant, TT Julian date.
    pub first_jd_tt: f64,
    /// Last covered instant, TT Julian date.
    pub last_jd_tt: f64,
    /// Number of table entries.
    pub entries: usize,
    /// Byte length of the provenance string, excluding a terminator.
    pub source_len: usize,
}

/// GNSS week and time-of-week value.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonGnssWeekTow {
    /// Time scale code, one of SidereonTimeScale.
    pub system: u32,
    /// Week number.
    pub week: u32,
    /// Seconds of week.
    pub tow_s: f64,
}

/// Copy a time-scale abbreviation such as "GPST". Delegates to
/// sidereon_core::astro::time::TimeScale::abbrev.
///
/// Safety: out must point to out_len writable bytes or be NULL when out_len is
/// zero; out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_time_scale_abbrev(
    scale: u32,
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_time_scale_abbrev", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_time_scale_abbrev",
            out_written,
            out_required
        ));
        let scale = c_try!(time_scale_from_c_code(
            "sidereon_time_scale_abbrev",
            "scale",
            scale
        ));
        c_try!(copy_prefix_to_c(
            "sidereon_time_scale_abbrev",
            "out",
            scale.abbrev().as_bytes(),
            out,
            out_len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// TAI minus UTC leap seconds in effect at UTC midnight for a calendar date.
/// Delegates to sidereon_core::astro::time::{julian_day_number,find_leap_seconds}.
///
/// Safety: out_leap_seconds must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_leap_seconds(
    year: i32,
    month: i32,
    day: i32,
    out_leap_seconds: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_leap_seconds", SidereonStatus::Panic, || {
        let out = c_try!(require_out(
            out_leap_seconds,
            "sidereon_leap_seconds",
            "out_leap_seconds"
        ));
        *out = 0.0;
        let jd_utc_midnight = julian_day_number(year, month, day) as f64 - 0.5;
        *out = find_leap_seconds(jd_utc_midnight);
        SidereonStatus::Ok
    })
}

/// Write leap-second table metadata.
///
/// Safety: out must point to a SidereonLeapSecondTableInfo.
#[no_mangle]
pub unsafe extern "C" fn sidereon_leap_second_table_info(
    out: *mut SidereonLeapSecondTableInfo,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_leap_second_table_info",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_leap_second_table_info", "out"));
            let table = leap_second_table();
            *out = SidereonLeapSecondTableInfo {
                first_mjd: table.first_mjd,
                last_mjd: table.last_mjd,
                entries: table.entries,
                source_len: table.source.len(),
            };
            SidereonStatus::Ok
        },
    )
}

/// Copy leap-second table provenance text.
///
/// Safety: out must point to out_len writable bytes or be NULL when out_len is
/// zero; out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_leap_second_table_source(
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_leap_second_table_source",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_leap_second_table_source",
                out_written,
                out_required
            ));
            let table = leap_second_table();
            c_try!(copy_prefix_to_c(
                "sidereon_leap_second_table_source",
                "out",
                table.source.as_bytes(),
                out,
                out_len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Write UT1 coverage metadata.
///
/// Safety: out must point to a SidereonUt1CoverageInfo.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ut1_coverage_info(
    out: *mut SidereonUt1CoverageInfo,
) -> SidereonStatus {
    ffi_boundary("sidereon_ut1_coverage_info", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_ut1_coverage_info", "out"));
        let info = ut1_coverage();
        *out = SidereonUt1CoverageInfo {
            first_mjd: info.first_mjd,
            last_mjd: info.last_mjd,
            first_jd_tt: info.first_jd_tt,
            last_jd_tt: info.last_jd_tt,
            entries: info.entries,
            source_len: info.source.len(),
        };
        SidereonStatus::Ok
    })
}

/// Copy UT1 coverage provenance text.
///
/// Safety: out must point to out_len writable bytes or be NULL when out_len is
/// zero; out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ut1_coverage_source(
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ut1_coverage_source",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ut1_coverage_source",
                out_written,
                out_required
            ));
            let info = ut1_coverage();
            c_try!(copy_prefix_to_c(
                "sidereon_ut1_coverage_source",
                "out",
                info.source.as_bytes(),
                out,
                out_len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Write whether a TT Julian date is inside UT1 coverage.
///
/// Safety: out must point to a bool.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ut1_coverage_covers_jd_tt(
    jd_tt: f64,
    out: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ut1_coverage_covers_jd_tt",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_ut1_coverage_covers_jd_tt",
                "out"
            ));
            *out = ut1_coverage().covers_jd_tt(jd_tt);
            SidereonStatus::Ok
        },
    )
}

/// Construct a GNSS week/TOW value.
///
/// Safety: out must point to a SidereonGnssWeekTow.
#[no_mangle]
pub unsafe extern "C" fn sidereon_gnss_week_tow_new(
    system: u32,
    week: u32,
    tow_s: f64,
    out: *mut SidereonGnssWeekTow,
) -> SidereonStatus {
    ffi_boundary("sidereon_gnss_week_tow_new", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_gnss_week_tow_new", "out"));
        *out = SidereonGnssWeekTow {
            system,
            week: 0,
            tow_s: 0.0,
        };
        let scale = c_try!(time_scale_from_c_code(
            "sidereon_gnss_week_tow_new",
            "system",
            system
        ));
        match GnssWeekTow::new(scale, week, tow_s) {
            Ok(value) => {
                *out = gnss_week_tow_to_c(value);
                SidereonStatus::Ok
            }
            Err(err) => crate::engine_error::time_model_error(
                "sidereon_gnss_week_tow_new",
                err,
                format!("sidereon_gnss_week_tow_new: {err}"),
            ),
        }
    })
}

/// Normalize a GNSS week/TOW value.
///
/// Safety: value and out must point to SidereonGnssWeekTow values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_gnss_week_tow_normalized(
    value: *const SidereonGnssWeekTow,
    out: *mut SidereonGnssWeekTow,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_gnss_week_tow_normalized",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_gnss_week_tow_normalized", "out"));
            *out = SidereonGnssWeekTow {
                system: SidereonTimeScale::Utc as u32,
                week: 0,
                tow_s: 0.0,
            };
            let value = c_try!(require_ref(
                value,
                "sidereon_gnss_week_tow_normalized",
                "value"
            ));
            let value = c_try!(gnss_week_tow_from_c(
                "sidereon_gnss_week_tow_normalized",
                value
            ));
            match value.normalized() {
                Ok(value) => {
                    *out = gnss_week_tow_to_c(value);
                    SidereonStatus::Ok
                }
                Err(err) => crate::engine_error::time_model_error(
                    "sidereon_gnss_week_tow_normalized",
                    err,
                    format!("sidereon_gnss_week_tow_normalized: {err}"),
                ),
            }
        },
    )
}

/// Apply 1024-week rollovers to a GNSS week/TOW value.
///
/// Safety: value points to a SidereonGnssWeekTow; out_week points to a uint32_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_gnss_week_tow_unrolled_week(
    value: *const SidereonGnssWeekTow,
    rollovers: u32,
    out_week: *mut u32,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_gnss_week_tow_unrolled_week",
        SidereonStatus::Panic,
        || {
            let out_week = c_try!(require_out(
                out_week,
                "sidereon_gnss_week_tow_unrolled_week",
                "out_week"
            ));
            *out_week = 0;
            let value = c_try!(require_ref(
                value,
                "sidereon_gnss_week_tow_unrolled_week",
                "value"
            ));
            let value = c_try!(gnss_week_tow_from_c(
                "sidereon_gnss_week_tow_unrolled_week",
                value
            ));
            match value.unrolled_week(rollovers) {
                Ok(week) => {
                    *out_week = week;
                    SidereonStatus::Ok
                }
                Err(err) => crate::engine_error::time_model_error(
                    "sidereon_gnss_week_tow_unrolled_week",
                    err,
                    format!("sidereon_gnss_week_tow_unrolled_week: {err}"),
                ),
            }
        },
    )
}

/// Write the Julian Day Number of a system's week epoch, when present.
///
/// Safety: out_present points to a bool; out_jdn points to an int64_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_gnss_week_epoch_julian_day_number(
    system: u32,
    out_present: *mut bool,
    out_jdn: *mut i64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_gnss_week_epoch_julian_day_number",
        SidereonStatus::Panic,
        || {
            if !out_present.is_null() && !out_jdn.is_null() {
                let outputs = [
                    Some((
                        c_try!(super::checked_output_range(
                            "sidereon_gnss_week_epoch_julian_day_number",
                            out_present,
                            1,
                            "out_present"
                        )),
                        "out_present",
                    )),
                    Some((
                        c_try!(super::checked_output_range(
                            "sidereon_gnss_week_epoch_julian_day_number",
                            out_jdn,
                            1,
                            "out_jdn"
                        )),
                        "out_jdn",
                    )),
                ];
                c_try!(super::reject_overlapping_optional_outputs(
                    "sidereon_gnss_week_epoch_julian_day_number",
                    &outputs
                ));
            }
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_gnss_week_epoch_julian_day_number",
                "out_present"
            ));
            let out_jdn = c_try!(require_out(
                out_jdn,
                "sidereon_gnss_week_epoch_julian_day_number",
                "out_jdn"
            ));
            *out_present = false;
            *out_jdn = 0;
            let system = c_try!(time_scale_from_c_code(
                "sidereon_gnss_week_epoch_julian_day_number",
                "system",
                system
            ));
            if let Some(jdn) = week_epoch_julian_day_number(system) {
                *out_present = true;
                *out_jdn = jdn;
            }
            SidereonStatus::Ok
        },
    )
}

/// Write the GNSS week for a calendar date, when present.
///
/// `*out_present` is false, with `*out_week` 0, for a date before the system's
/// week epoch, for a scale without GNSS weeks, and for fields that name no
/// calendar date: a month outside 1..=12 or a day outside that month. The
/// engine reports all three as one absence, so this call does not tell them
/// apart.
///
/// Safety: out_present points to a bool; out_week points to a uint32_t; their
/// output ranges must be disjoint.
#[no_mangle]
pub unsafe extern "C" fn sidereon_gnss_week_from_calendar(
    system: u32,
    year: i64,
    month: i64,
    day: i64,
    out_present: *mut bool,
    out_week: *mut u32,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_gnss_week_from_calendar",
        SidereonStatus::Panic,
        || {
            if !out_present.is_null() && !out_week.is_null() {
                let outputs = [
                    Some((
                        c_try!(super::checked_output_range(
                            "sidereon_gnss_week_from_calendar",
                            out_present,
                            1,
                            "out_present"
                        )),
                        "out_present",
                    )),
                    Some((
                        c_try!(super::checked_output_range(
                            "sidereon_gnss_week_from_calendar",
                            out_week,
                            1,
                            "out_week"
                        )),
                        "out_week",
                    )),
                ];
                c_try!(super::reject_overlapping_optional_outputs(
                    "sidereon_gnss_week_from_calendar",
                    &outputs
                ));
            }
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_gnss_week_from_calendar",
                "out_present"
            ));
            let out_week = c_try!(require_out(
                out_week,
                "sidereon_gnss_week_from_calendar",
                "out_week"
            ));
            *out_present = false;
            *out_week = 0;
            let system = c_try!(time_scale_from_c_code(
                "sidereon_gnss_week_from_calendar",
                "system",
                system
            ));
            if let Some(week) = week_from_calendar(system, year, month, day) {
                *out_present = true;
                *out_week = week;
            }
            SidereonStatus::Ok
        },
    )
}

/// Write seconds of week for a calendar date and time, counted from Sunday
/// 00:00 in the date's own system time.
///
/// The fields must name a calendar date and clock time: `month` in 1..=12,
/// `day` in 1 through the length of that month in the Gregorian calendar,
/// `hour` in 0..=23, `minute` in 0..=59 and `second` in 0..=60, where 60 is a
/// leap-second label. Any other field is refused with
/// SIDEREON_STATUS_INVALID_ARGUMENT and a thread-local message naming the
/// fields. `*out_sow_s` is set to NaN before the fields are read and keeps it on
/// a refusal, so a refused date never reads as a seconds-of-week value.
///
/// Safety: out_sow_s points to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_gnss_seconds_of_week_from_calendar(
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    minute: i64,
    second: i64,
    out_sow_s: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_gnss_seconds_of_week_from_calendar",
        SidereonStatus::Panic,
        || {
            let out_sow_s = c_try!(require_out(
                out_sow_s,
                "sidereon_gnss_seconds_of_week_from_calendar",
                "out_sow_s"
            ));
            *out_sow_s = f64::NAN;
            match seconds_of_week_from_calendar(year, month, day, hour, minute, second) {
                Some(sow) => {
                    *out_sow_s = sow;
                    SidereonStatus::Ok
                }
                None => {
                    set_last_error(format!(
                        "sidereon_gnss_seconds_of_week_from_calendar: year {year} month {month} \
                         day {day} hour {hour} minute {minute} second {second} is not a calendar \
                         date and time"
                    ));
                    SidereonStatus::InvalidArgument
                }
            }
        },
    )
}

/// Split continuous seconds since a system week epoch into week and seconds of
/// week.
///
/// Safety: out_week and out_sow_s point to doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_gnss_week_and_seconds_of_week(
    continuous_seconds: f64,
    out_week: *mut f64,
    out_sow_s: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_gnss_week_and_seconds_of_week",
        SidereonStatus::Panic,
        || {
            if !out_week.is_null() && !out_sow_s.is_null() {
                let outputs = [
                    Some((
                        c_try!(super::checked_output_range(
                            "sidereon_gnss_week_and_seconds_of_week",
                            out_week,
                            1,
                            "out_week"
                        )),
                        "out_week",
                    )),
                    Some((
                        c_try!(super::checked_output_range(
                            "sidereon_gnss_week_and_seconds_of_week",
                            out_sow_s,
                            1,
                            "out_sow_s"
                        )),
                        "out_sow_s",
                    )),
                ];
                c_try!(super::reject_overlapping_optional_outputs(
                    "sidereon_gnss_week_and_seconds_of_week",
                    &outputs
                ));
            }
            let out_week = c_try!(require_out(
                out_week,
                "sidereon_gnss_week_and_seconds_of_week",
                "out_week"
            ));
            let out_sow_s = c_try!(require_out(
                out_sow_s,
                "sidereon_gnss_week_and_seconds_of_week",
                "out_sow_s"
            ));
            let (week, sow) = week_and_seconds_of_week(continuous_seconds);
            *out_week = week;
            *out_sow_s = sow;
            SidereonStatus::Ok
        },
    )
}

/// Copy the core conventional GNSS system label into out.
#[no_mangle]
pub unsafe extern "C" fn sidereon_gnss_system_label(
    system: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_gnss_system_label", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_gnss_system_label",
            out_written,
            out_required
        ));
        let system = c_try!(gnss_system_from_c_code(
            "sidereon_gnss_system_label",
            "system",
            system
        ));
        c_try!(copy_prefix_to_c(
            "sidereon_gnss_system_label",
            "out",
            system.as_str().as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

// --- Civil <-> J2000 time conversions (sidereon_core::astro::time::civil) -----

/// Write the second-of-day value formed by the civil clock fields. Delegates to
/// `sidereon_core::astro::time::civil::second_of_day` and preserves fractional
/// seconds.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_second_of_day(
    hour: i32,
    minute: i32,
    second: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_second_of_day", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_second_of_day", "out"));
        *out = sidereon_core::astro::time::civil::second_of_day(hour, minute, second);
        SidereonStatus::Ok
    })
}

/// Write the fractional day-of-year for a civil instant. January 1 at midnight
/// is `1.0`; fractional seconds are retained. The civil date is validated by
/// the public data-catalog date constructor before delegating to
/// `sidereon_core::astro::time::civil::day_of_year`.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_day_of_year(
    year: i32,
    month: i32,
    day: i32,
    hour: i32,
    minute: i32,
    second: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_day_of_year", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_day_of_year", "out"));
        *out = 0.0;
        let month = match u8::try_from(month) {
            Ok(month) => month,
            Err(_) => {
                set_last_error("sidereon_day_of_year: month is outside uint8 range".to_string());
                return SidereonStatus::InvalidArgument;
            }
        };
        let day = match u8::try_from(day) {
            Ok(day) => day,
            Err(_) => {
                set_last_error("sidereon_day_of_year: day is outside uint8 range".to_string());
                return SidereonStatus::InvalidArgument;
            }
        };
        c_try!(
            sidereon_core::data::ProductDate::new(year, month, day).map_err(|err| {
                set_last_error(format!("sidereon_day_of_year: {err}"));
                SidereonStatus::InvalidArgument
            })
        );
        *out = sidereon_core::astro::time::civil::day_of_year(
            year,
            i32::from(month),
            i32::from(day),
            hour,
            minute,
            second,
        );
        SidereonStatus::Ok
    })
}

/// Write the integer day-of-year for a validated product date. January 1 is
/// `1`. Delegates to `sidereon_core::data::day_of_year` and preserves the
/// product-date integer semantics.
///
/// Safety: out must point to a uint16_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_data_day_of_year(
    year: i32,
    month: u8,
    day: u8,
    out: *mut u16,
) -> SidereonStatus {
    ffi_boundary("sidereon_data_day_of_year", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_data_day_of_year", "out"));
        *out = 0;
        let date = c_try!(
            sidereon_core::data::ProductDate::new(year, month, day).map_err(|err| {
                set_last_error(format!("sidereon_data_day_of_year: {err}"));
                SidereonStatus::InvalidArgument
            },)
        );
        *out = sidereon_core::data::day_of_year(date);
        SidereonStatus::Ok
    })
}

/// J2000 seconds for a civil UTC-like calendar instant (the engine's
/// proleptic-Gregorian count). Delegates to
/// sidereon_core::astro::time::civil::j2000_seconds (infallible).
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_civil_to_j2000_seconds(
    year: i32,
    month: i32,
    day: i32,
    hour: i32,
    minute: i32,
    second: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_civil_to_j2000_seconds",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_civil_to_j2000_seconds", "out"));
            *out = sidereon_core::astro::time::civil::j2000_seconds(
                year, month, day, hour, minute, second,
            );
            SidereonStatus::Ok
        },
    )
}

/// J2000 seconds for a split Julian date. Delegates to
/// sidereon_core::astro::time::civil::j2000_seconds_from_split (infallible).
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_split_jd_to_j2000_seconds(
    jd_whole: f64,
    jd_fraction: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_split_jd_to_j2000_seconds",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_split_jd_to_j2000_seconds",
                "out"
            ));
            *out =
                sidereon_core::astro::time::civil::j2000_seconds_from_split(jd_whole, jd_fraction);
            SidereonStatus::Ok
        },
    )
}

/// Civil calendar instant from integer J2000 seconds. Delegates to
/// sidereon_core::astro::time::civil::civil_from_j2000_seconds (infallible).
///
/// Safety: each out pointer must point to an int64_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_j2000_seconds_to_civil(
    seconds: i64,
    out_year: *mut i64,
    out_month: *mut i64,
    out_day: *mut i64,
    out_hour: *mut i64,
    out_minute: *mut i64,
    out_second: *mut i64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_j2000_seconds_to_civil",
        SidereonStatus::Panic,
        || {
            let oy = c_try!(require_out(
                out_year,
                "sidereon_j2000_seconds_to_civil",
                "out_year"
            ));
            let om = c_try!(require_out(
                out_month,
                "sidereon_j2000_seconds_to_civil",
                "out_month"
            ));
            let od = c_try!(require_out(
                out_day,
                "sidereon_j2000_seconds_to_civil",
                "out_day"
            ));
            let oh = c_try!(require_out(
                out_hour,
                "sidereon_j2000_seconds_to_civil",
                "out_hour"
            ));
            let omin = c_try!(require_out(
                out_minute,
                "sidereon_j2000_seconds_to_civil",
                "out_minute"
            ));
            let os = c_try!(require_out(
                out_second,
                "sidereon_j2000_seconds_to_civil",
                "out_second"
            ));
            let (y, mo, d, h, mi, s) =
                sidereon_core::astro::time::civil::civil_from_j2000_seconds(seconds);
            *oy = y;
            *om = mo;
            *od = d;
            *oh = h;
            *omin = mi;
            *os = s;
            SidereonStatus::Ok
        },
    )
}

/// GPS seconds for a civil GPS-time instant (used to query RINEX clock series).
/// Writes the value to *out_gps_seconds and *out_available (false if the date is
/// invalid). The second is read as the shortest decimal of the double given,
/// every digit kept, so 59.9999996 names that epoch rather than rounding into the
/// next minute. GPS time has no leap-second label; for a civil epoch in another
/// time scale, such as UTC with its 23:59:60, use sidereon_civil_to_clock_epoch.
/// Delegates to sidereon_core::rinex::clock::civil_to_gps_seconds.
///
/// Safety: out_gps_seconds points to a double; out_available points to a bool;
/// their output ranges must be disjoint.
#[no_mangle]
pub unsafe extern "C" fn sidereon_civil_to_gps_seconds(
    year: i32,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: f64,
    out_gps_seconds: *mut f64,
    out_available: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_civil_to_gps_seconds",
        SidereonStatus::Panic,
        || {
            if !out_gps_seconds.is_null() && !out_available.is_null() {
                let outputs = [
                    Some((
                        c_try!(super::checked_output_range(
                            "sidereon_civil_to_gps_seconds",
                            out_gps_seconds,
                            1,
                            "out_gps_seconds"
                        )),
                        "out_gps_seconds",
                    )),
                    Some((
                        c_try!(super::checked_output_range(
                            "sidereon_civil_to_gps_seconds",
                            out_available,
                            1,
                            "out_available"
                        )),
                        "out_available",
                    )),
                ];
                c_try!(super::reject_overlapping_optional_outputs(
                    "sidereon_civil_to_gps_seconds",
                    &outputs
                ));
            }
            let out_gps_seconds = c_try!(require_out(
                out_gps_seconds,
                "sidereon_civil_to_gps_seconds",
                "out_gps_seconds"
            ));
            *out_gps_seconds = 0.0;
            let out_available = c_try!(require_out(
                out_available,
                "sidereon_civil_to_gps_seconds",
                "out_available"
            ));
            *out_available = false;
            if let Some(v) = sidereon_core::rinex::clock::civil_to_gps_seconds(
                year, month, day, hour, minute, second,
            ) {
                *out_gps_seconds = v;
                *out_available = true;
            }
            SidereonStatus::Ok
        },
    )
}

/// Split-Julian-date time scales, mirroring
/// sidereon_core::astro::time::scales::TimeScales. Build one with
/// sidereon_timescales_from_utc, then pass it to the frame-transform entry
/// points below.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTimeScales {
    /// Integer Julian day boundary (TAI-aligned), shared by all scales.
    pub jd_whole: f64,
    /// UT1 day fraction relative to jd_whole.
    pub ut1_fraction: f64,
    /// TT day fraction relative to jd_whole.
    pub tt_fraction: f64,
    /// TDB day fraction relative to jd_whole.
    pub tdb_fraction: f64,
    /// Full UT1 Julian date.
    pub jd_ut1: f64,
    /// Full TT Julian date.
    pub jd_tt: f64,
    /// Full TDB Julian date.
    pub jd_tdb: f64,
    /// Whether UT1 lies outside the UT1 table and was taken from the long-term
    /// delta-T curve. The frame transforms that read UT1 refuse time scales
    /// marked this way. A SidereonUt1Degradation value; an unknown value is
    /// refused where the time scales are read.
    pub ut1_degraded: u32,
}

/// Which side of the UT1 table an instant lies on when its UT1 came from the
/// long-term delta-T curve rather than the table. Mirrors
/// sidereon_core::astro::time::DegradeReason, with None for an instant inside
/// the table.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonUt1Degradation {
    /// UT1 comes from the table.
    None = 0,
    /// The instant precedes the first covered table entry.
    BeforeCoverage = 1,
    /// The instant follows the last covered table entry.
    AfterCoverage = 2,
}

impl SidereonUt1Degradation {
    pub(crate) fn from_core(value: Option<sidereon_core::astro::time::DegradeReason>) -> Self {
        match value {
            None => Self::None,
            Some(sidereon_core::astro::time::DegradeReason::BeforeCoverage) => Self::BeforeCoverage,
            Some(sidereon_core::astro::time::DegradeReason::AfterCoverage) => Self::AfterCoverage,
        }
    }

    pub(crate) fn to_core(self) -> Option<sidereon_core::astro::time::DegradeReason> {
        match self {
            Self::None => None,
            Self::BeforeCoverage => Some(sidereon_core::astro::time::DegradeReason::BeforeCoverage),
            Self::AfterCoverage => Some(sidereon_core::astro::time::DegradeReason::AfterCoverage),
        }
    }
}

/// Resolve the split-Julian-date time scales for a UTC calendar instant.
/// Delegates to sidereon_core::astro::time::scales::TimeScales::from_utc.
///
/// Safety: out must point to a SidereonTimeScales.
#[no_mangle]
pub unsafe extern "C" fn sidereon_timescales_from_utc(
    year: i32,
    month: i32,
    day: i32,
    hour: i32,
    minute: i32,
    second: f64,
    out: *mut SidereonTimeScales,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_timescales_from_utc",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_timescales_from_utc", "out"));
            *out = SidereonTimeScales {
                jd_whole: 0.0,
                ut1_fraction: 0.0,
                tt_fraction: 0.0,
                tdb_fraction: 0.0,
                jd_ut1: 0.0,
                jd_tt: 0.0,
                jd_tdb: 0.0,
                ut1_degraded: SidereonUt1Degradation::None as u32,
            };
            match CoreTimeScales::from_utc(year, month, day, hour, minute, second) {
                Ok(ts) => {
                    *out = SidereonTimeScales::from_core(&ts);
                    SidereonStatus::Ok
                }
                Err(err) => extra_invalid_arg("sidereon_timescales_from_utc", err),
            }
        },
    )
}

// Shared body for the time-scales-only frame matrix entry points. cbindgen does
// not expand macros, so each public function below is written out explicitly and
// delegates here.

// --- Civil instant construction (sidereon_core::astro::time::Instant) ---------

/// Build a UTC Instant from civil-calendar fields and report its split Julian
/// date and continuous J2000 seconds. No leap second is applied. Delegates to
/// sidereon_core::astro::time::Instant::from_utc_civil.
///
/// Safety: each non-null out pointer points to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_instant_from_utc_civil(
    year: i32,
    month: i32,
    day: i32,
    hour: i32,
    minute: i32,
    second: f64,
    out_jd_whole: *mut f64,
    out_jd_fraction: *mut f64,
    out_j2000_seconds: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_instant_from_utc_civil",
        SidereonStatus::Panic,
        || {
            let instant = match Instant::from_utc_civil(year, month, day, hour, minute, second) {
                Ok(instant) => instant,
                Err(err) => return extra_invalid_arg("sidereon_instant_from_utc_civil", err),
            };
            let jd = match instant.julian_date() {
                Some(jd) => jd,
                None => {
                    set_last_error(
                        "sidereon_instant_from_utc_civil: instant is not a Julian-date representation"
                            .to_string(),
                    );
                    return SidereonStatus::Solve;
                }
            };
            if !out_jd_whole.is_null() {
                out_jd_whole.write(jd.jd_whole);
            }
            if !out_jd_fraction.is_null() {
                out_jd_fraction.write(jd.fraction);
            }
            if !out_j2000_seconds.is_null() {
                out_j2000_seconds.write(instant_to_j2000_seconds(&instant).unwrap_or(f64::NAN));
            }
            SidereonStatus::Ok
        },
    )
}

// --- Leap-second accessors --------------------------------------------------

/// GPS - UTC (the GNSS leap-second offset a GPS receiver applies, IS-GPS-200) at
/// a UTC Julian date, written to *out. 18 s from 2017-01-01. Delegates to the
/// core `gps_utc_offset_s`. This is NOT TAI - UTC (use
/// sidereon_tai_utc_offset_s); the two differ by a constant 19 s.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_gps_utc_offset_s(jd_utc: f64, out: *mut f64) -> SidereonStatus {
    ffi_boundary("sidereon_gps_utc_offset_s", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_gps_utc_offset_s", "out"));
        *out = core_gps_utc_offset_s(jd_utc);
        SidereonStatus::Ok
    })
}

/// TAI - UTC (the IERS / Bulletin C leap-second count) at a UTC Julian date,
/// written to *out. 37 s from 2017-01-01. Delegates to the core
/// `tai_utc_offset_s`. For the GNSS "GPS - UTC" quantity use
/// sidereon_gps_utc_offset_s instead.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tai_utc_offset_s(jd_utc: f64, out: *mut f64) -> SidereonStatus {
    ffi_boundary("sidereon_tai_utc_offset_s", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_tai_utc_offset_s", "out"));
        *out = core_tai_utc_offset_s(jd_utc);
        SidereonStatus::Ok
    })
}

fn time_offset_error_to_status(fn_name: &str, err: TimeOffsetError) -> SidereonStatus {
    let legacy_message = format!("{fn_name}: {err}");
    crate::engine_error::time_offset_error(fn_name, err, legacy_message)
}

fn gnss_week_tow_to_c(value: GnssWeekTow) -> SidereonGnssWeekTow {
    SidereonGnssWeekTow {
        system: time_scale_to_c_code(value.system),
        week: value.week,
        tow_s: value.tow_s,
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;

    #[test]
    fn calendar_routes_preserve_fractional_and_product_date_semantics() {
        use sidereon_core::astro::time::civil;
        use sidereon_core::data::{self as core_data, ProductDate};
        // Every expected value is sidereon-core's own result for the same
        // fields.
        let mut second_of_day = 0.0;
        assert_eq!(
            unsafe { sidereon_second_of_day(1, 2, 3.5, &mut second_of_day) },
            SidereonStatus::Ok
        );
        assert_eq!(
            second_of_day.to_bits(),
            civil::second_of_day(1, 2, 3.5).to_bits()
        );

        let mut day_of_year = 0.0;
        assert_eq!(
            unsafe { sidereon_day_of_year(2024, 1, 1, 0, 0, 0.0, &mut day_of_year) },
            SidereonStatus::Ok
        );
        assert_eq!(
            day_of_year.to_bits(),
            civil::day_of_year(2024, 1, 1, 0, 0, 0.0).to_bits()
        );
        assert_eq!(
            unsafe { sidereon_day_of_year(2024, 2, 29, 12, 0, 0.25, &mut day_of_year) },
            SidereonStatus::Ok
        );
        assert_eq!(
            day_of_year.to_bits(),
            civil::day_of_year(2024, 2, 29, 12, 0, 0.25).to_bits()
        );

        let mut product_day = 0;
        assert_eq!(
            unsafe { sidereon_data_day_of_year(2020, 3, 1, &mut product_day) },
            SidereonStatus::Ok
        );
        assert_eq!(
            product_day,
            core_data::day_of_year(ProductDate::new(2020, 3, 1).expect("core date"))
        );

        // sidereon-core refuses 2023-02-29 as a product date.
        assert!(ProductDate::new(2023, 2, 29).is_err());

        assert_eq!(
            unsafe { sidereon_day_of_year(2023, 2, 29, 0, 0, 0.0, &mut day_of_year) },
            SidereonStatus::InvalidArgument
        );
        assert_eq!(
            unsafe { sidereon_data_day_of_year(2023, 2, 29, &mut product_day) },
            SidereonStatus::InvalidArgument
        );
    }
}

impl SidereonTimeScales {
    pub(crate) fn from_core(ts: &CoreTimeScales) -> Self {
        Self {
            jd_whole: ts.jd_whole,
            ut1_fraction: ts.ut1_fraction,
            tt_fraction: ts.tt_fraction,
            tdb_fraction: ts.tdb_fraction,
            jd_ut1: ts.jd_ut1,
            jd_tt: ts.jd_tt,
            jd_tdb: ts.jd_tdb,
            ut1_degraded: SidereonUt1Degradation::from_core(ts.ut1_degraded) as u32,
        }
    }

    /// The engine time scales, refusing a `ut1_degraded` value that names no
    /// SidereonUt1Degradation.
    pub(crate) fn to_core(self, fn_name: &str) -> Result<CoreTimeScales, SidereonStatus> {
        let ut1_degraded = match self.ut1_degraded {
            x if x == SidereonUt1Degradation::None as u32 => SidereonUt1Degradation::None,
            x if x == SidereonUt1Degradation::BeforeCoverage as u32 => {
                SidereonUt1Degradation::BeforeCoverage
            }
            x if x == SidereonUt1Degradation::AfterCoverage as u32 => {
                SidereonUt1Degradation::AfterCoverage
            }
            other => {
                set_last_error(format!("{fn_name}: unknown ts.ut1_degraded {other}"));
                return Err(SidereonStatus::InvalidArgument);
            }
        };
        Ok(CoreTimeScales {
            jd_whole: self.jd_whole,
            ut1_fraction: self.ut1_fraction,
            tt_fraction: self.tt_fraction,
            tdb_fraction: self.tdb_fraction,
            jd_ut1: self.jd_ut1,
            jd_tt: self.jd_tt,
            jd_tdb: self.jd_tdb,
            ut1_degraded: ut1_degraded.to_core(),
        })
    }
}

/// Create an exact epoch from whole J2000 seconds and attoseconds.
///
/// Safety: `out_epoch` must point to one writable handle pointer.
#[no_mangle]
pub unsafe extern "C" fn sidereon_exact_epoch_new(
    seconds: i64,
    attoseconds: u64,
    out_epoch: *mut *mut SidereonExactEpoch,
) -> SidereonStatus {
    ffi_boundary("sidereon_exact_epoch_new", SidereonStatus::Panic, || {
        let out_epoch = c_try!(require_out(
            out_epoch,
            "sidereon_exact_epoch_new",
            "out_epoch"
        ));
        *out_epoch = ptr::null_mut();
        let Some(inner) = sidereon_core::astro::time::ExactEpoch::new(seconds, attoseconds) else {
            set_last_error("sidereon_exact_epoch_new: attoseconds out of range".to_owned());
            return SidereonStatus::InvalidArgument;
        };
        write_boxed_handle(out_epoch, SidereonExactEpoch { inner });
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_exact_epoch_j2000(
    out_epoch: *mut *mut SidereonExactEpoch,
) -> SidereonStatus {
    ffi_boundary("sidereon_exact_epoch_j2000", SidereonStatus::Panic, || {
        let out_epoch = c_try!(require_out(
            out_epoch,
            "sidereon_exact_epoch_j2000",
            "out_epoch"
        ));
        *out_epoch = ptr::null_mut();
        write_boxed_handle(
            out_epoch,
            SidereonExactEpoch {
                inner: sidereon_core::astro::time::ExactEpoch::J2000,
            },
        );
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_exact_epoch_attoseconds_per_second(
    out_attoseconds: *mut u64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_exact_epoch_attoseconds_per_second",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_attoseconds,
                "sidereon_exact_epoch_attoseconds_per_second",
                "out_attoseconds"
            ));
            *out = sidereon_core::astro::time::ExactEpoch::ATTOSECONDS_PER_SECOND;
            SidereonStatus::Ok
        },
    )
}

/// Parse a shortest-decimal J2000 second value as an exact epoch.
///
/// Safety: `out_epoch` must point to one writable handle pointer.
#[no_mangle]
pub unsafe extern "C" fn sidereon_exact_epoch_from_j2000_seconds(
    seconds: f64,
    out_epoch: *mut *mut SidereonExactEpoch,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_exact_epoch_from_j2000_seconds",
        SidereonStatus::Panic,
        || {
            let out_epoch = c_try!(require_out(
                out_epoch,
                "sidereon_exact_epoch_from_j2000_seconds",
                "out_epoch"
            ));
            *out_epoch = ptr::null_mut();
            let Some(inner) = sidereon_core::astro::time::ExactEpoch::from_j2000_seconds(seconds)
            else {
                set_last_error(
                    "sidereon_exact_epoch_from_j2000_seconds: invalid or unrepresentable epoch"
                        .to_owned(),
                );
                return SidereonStatus::InvalidArgument;
            };
            write_boxed_handle(out_epoch, SidereonExactEpoch { inner });
            SidereonStatus::Ok
        },
    )
}

/// Build an exact epoch from a civil date and the shortest-decimal second label.
///
/// Safety: `out_epoch` must point to one writable handle pointer.
#[no_mangle]
pub unsafe extern "C" fn sidereon_exact_epoch_from_civil(
    year: i32,
    month: i32,
    day: i32,
    hour: i32,
    minute: i32,
    second: f64,
    out_epoch: *mut *mut SidereonExactEpoch,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_exact_epoch_from_civil",
        SidereonStatus::Panic,
        || {
            let out_epoch = c_try!(require_out(
                out_epoch,
                "sidereon_exact_epoch_from_civil",
                "out_epoch"
            ));
            *out_epoch = ptr::null_mut();
            let Some(inner) = sidereon_core::astro::time::ExactEpoch::from_civil(
                year, month, day, hour, minute, second,
            ) else {
                set_last_error(
                    "sidereon_exact_epoch_from_civil: invalid or unrepresentable epoch".to_owned(),
                );
                return SidereonStatus::InvalidArgument;
            };
            write_boxed_handle(out_epoch, SidereonExactEpoch { inner });
            SidereonStatus::Ok
        },
    )
}

/// Create a query at an exact civil epoch without adding a binary offset.
///
/// Safety: `epoch` must be live; `out_query` must point to one writable handle pointer.
#[no_mangle]
pub unsafe extern "C" fn sidereon_exact_epoch_query(
    epoch: *const SidereonExactEpoch,
    out_query: *mut *mut SidereonExactEpochQuery,
) -> SidereonStatus {
    ffi_boundary("sidereon_exact_epoch_query", SidereonStatus::Panic, || {
        let out_query = c_try!(require_out(
            out_query,
            "sidereon_exact_epoch_query",
            "out_query"
        ));
        *out_query = ptr::null_mut();
        let epoch = c_try!(require_ref(epoch, "sidereon_exact_epoch_query", "epoch"));
        write_boxed_handle(
            out_query,
            SidereonExactEpochQuery {
                inner: epoch.inner.query(),
            },
        );
        SidereonStatus::Ok
    })
}

#[no_mangle]
/// Create an owned exact epoch from a query's exact origin and offset.
/// Release the result with sidereon_exact_epoch_free.
///
/// Safety: query must be a live handle and out_epoch must point to writable
/// handle-pointer storage.
pub unsafe extern "C" fn sidereon_exact_epoch_query_epoch(
    query: *const SidereonExactEpochQuery,
    out_epoch: *mut *mut SidereonExactEpoch,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_exact_epoch_query_epoch",
        SidereonStatus::Panic,
        || {
            let out_epoch = c_try!(require_out(
                out_epoch,
                "sidereon_exact_epoch_query_epoch",
                "out_epoch"
            ));
            *out_epoch = ptr::null_mut();
            let query = c_try!(require_ref(
                query,
                "sidereon_exact_epoch_query_epoch",
                "query"
            ));
            write_boxed_handle(
                out_epoch,
                SidereonExactEpoch {
                    inner: query.inner.epoch(),
                },
            );
            SidereonStatus::Ok
        },
    )
}

/// Interpret a finite `f64` as its exact binary J2000-second value.
///
/// Safety: `out_query` must point to one writable handle pointer.
#[no_mangle]
pub unsafe extern "C" fn sidereon_exact_epoch_query_from_binary_j2000_seconds(
    seconds: f64,
    out_query: *mut *mut SidereonExactEpochQuery,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_exact_epoch_query_from_binary_j2000_seconds",
        SidereonStatus::Panic,
        || {
            let out_query = c_try!(require_out(
                out_query,
                "sidereon_exact_epoch_query_from_binary_j2000_seconds",
                "out_query"
            ));
            *out_query = ptr::null_mut();
            let Some(inner) =
                sidereon_core::astro::time::ExactEpoch::from_binary_j2000_seconds(seconds)
            else {
                set_last_error(
                    "sidereon_exact_epoch_query_from_binary_j2000_seconds: non-finite seconds"
                        .to_owned(),
                );
                return SidereonStatus::InvalidArgument;
            };
            write_boxed_handle(out_query, SidereonExactEpochQuery { inner });
            SidereonStatus::Ok
        },
    )
}

/// Add a shortest-decimal second offset while retaining the exact epoch result.
///
/// Safety: `epoch` must be live; `out_epoch` must point to one writable handle pointer.
#[no_mangle]
pub unsafe extern "C" fn sidereon_exact_epoch_checked_add_seconds(
    epoch: *const SidereonExactEpoch,
    seconds: f64,
    out_epoch: *mut *mut SidereonExactEpoch,
) -> SidereonStatus {
    exact_epoch_decimal_offset(
        "sidereon_exact_epoch_checked_add_seconds",
        epoch,
        seconds,
        out_epoch,
        true,
    )
}

/// Subtract a shortest-decimal second offset while retaining the exact epoch result.
///
/// Safety: `epoch` must be live; `out_epoch` must point to one writable handle pointer.
#[no_mangle]
pub unsafe extern "C" fn sidereon_exact_epoch_checked_sub_seconds(
    epoch: *const SidereonExactEpoch,
    seconds: f64,
    out_epoch: *mut *mut SidereonExactEpoch,
) -> SidereonStatus {
    exact_epoch_decimal_offset(
        "sidereon_exact_epoch_checked_sub_seconds",
        epoch,
        seconds,
        out_epoch,
        false,
    )
}

unsafe fn exact_epoch_decimal_offset(
    fn_name: &str,
    epoch: *const SidereonExactEpoch,
    seconds: f64,
    out_epoch: *mut *mut SidereonExactEpoch,
    add: bool,
) -> SidereonStatus {
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let out_epoch = c_try!(require_out(out_epoch, fn_name, "out_epoch"));
        *out_epoch = ptr::null_mut();
        let epoch = c_try!(require_ref(epoch, fn_name, "epoch"));
        let inner = if add {
            epoch.inner.checked_add_seconds(seconds)
        } else {
            epoch.inner.checked_sub_seconds(seconds)
        };
        let Some(inner) = inner else {
            set_last_error(format!("{fn_name}: offset is invalid or unrepresentable"));
            return SidereonStatus::InvalidArgument;
        };
        write_boxed_handle(out_epoch, SidereonExactEpoch { inner });
        SidereonStatus::Ok
    })
}

/// Add an exact binary `f64` offset to a query.
///
/// Safety: `query` must be live; `out_query` must point to one writable handle pointer.
#[no_mangle]
pub unsafe extern "C" fn sidereon_exact_epoch_query_checked_add_binary_seconds(
    query: *const SidereonExactEpochQuery,
    seconds: f64,
    out_query: *mut *mut SidereonExactEpochQuery,
) -> SidereonStatus {
    exact_epoch_query_binary_offset(
        "sidereon_exact_epoch_query_checked_add_binary_seconds",
        query,
        seconds,
        out_query,
        true,
    )
}

/// Subtract an exact binary `f64` offset from a query.
///
/// Safety: `query` must be live; `out_query` must point to one writable handle pointer.
#[no_mangle]
pub unsafe extern "C" fn sidereon_exact_epoch_query_checked_sub_binary_seconds(
    query: *const SidereonExactEpochQuery,
    seconds: f64,
    out_query: *mut *mut SidereonExactEpochQuery,
) -> SidereonStatus {
    exact_epoch_query_binary_offset(
        "sidereon_exact_epoch_query_checked_sub_binary_seconds",
        query,
        seconds,
        out_query,
        false,
    )
}

unsafe fn exact_epoch_query_binary_offset(
    fn_name: &str,
    query: *const SidereonExactEpochQuery,
    seconds: f64,
    out_query: *mut *mut SidereonExactEpochQuery,
    add: bool,
) -> SidereonStatus {
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let out_query = c_try!(require_out(out_query, fn_name, "out_query"));
        *out_query = ptr::null_mut();
        let query = c_try!(require_ref(query, fn_name, "query"));
        let inner = if add {
            query.inner.clone().checked_add_binary_seconds(seconds)
        } else {
            query.inner.clone().checked_sub_binary_seconds(seconds)
        };
        let Some(inner) = inner else {
            set_last_error(format!("{fn_name}: binary offset must be finite"));
            return SidereonStatus::InvalidArgument;
        };
        write_boxed_handle(out_query, SidereonExactEpochQuery { inner });
        SidereonStatus::Ok
    })
}

/// Write the exact epoch components and sub-attosecond decimal residue.
///
/// Safety: `epoch` must be live; all output pointers must be writable.
#[no_mangle]
pub unsafe extern "C" fn sidereon_exact_epoch_components(
    epoch: *const SidereonExactEpoch,
    out_seconds: *mut i64,
    out_attoseconds: *mut u64,
    out_residue_digits: *mut i64,
    out_residue_places: *mut u16,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_exact_epoch_components",
        SidereonStatus::Panic,
        || {
            let epoch = c_try!(require_ref(
                epoch,
                "sidereon_exact_epoch_components",
                "epoch"
            ));
            let out_seconds = c_try!(require_out(
                out_seconds,
                "sidereon_exact_epoch_components",
                "out_seconds"
            ));
            let out_attoseconds = c_try!(require_out(
                out_attoseconds,
                "sidereon_exact_epoch_components",
                "out_attoseconds"
            ));
            let out_residue_digits = c_try!(require_out(
                out_residue_digits,
                "sidereon_exact_epoch_components",
                "out_residue_digits"
            ));
            let out_residue_places = c_try!(require_out(
                out_residue_places,
                "sidereon_exact_epoch_components",
                "out_residue_places"
            ));
            let (residue_digits, residue_places) = epoch.inner.sub_attosecond();
            *out_seconds = epoch.inner.whole_seconds();
            *out_attoseconds = epoch.inner.attoseconds();
            *out_residue_digits = residue_digits;
            *out_residue_places = residue_places;
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
/// Compare exact epoch values without converting them to rounded seconds.
///
/// Safety: both epoch handles must be live and out_ordering must be writable.
pub unsafe extern "C" fn sidereon_exact_epoch_compare(
    epoch: *const SidereonExactEpoch,
    other: *const SidereonExactEpoch,
    out_ordering: *mut SidereonExactOrdering,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_exact_epoch_compare",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_ordering,
                "sidereon_exact_epoch_compare",
                "out_ordering"
            ));
            *out = SidereonExactOrdering::Equal;
            let epoch = c_try!(require_ref(epoch, "sidereon_exact_epoch_compare", "epoch"));
            let other = c_try!(require_ref(other, "sidereon_exact_epoch_compare", "other"));
            *out = match epoch.inner.cmp(&other.inner) {
                std::cmp::Ordering::Less => SidereonExactOrdering::Less,
                std::cmp::Ordering::Equal => SidereonExactOrdering::Equal,
                std::cmp::Ordering::Greater => SidereonExactOrdering::Greater,
            };
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
/// Compare exact epoch values for equality without converting them to seconds.
///
/// Safety: both epoch handles must be live and out_equal must be writable.
pub unsafe extern "C" fn sidereon_exact_epoch_equal(
    epoch: *const SidereonExactEpoch,
    other: *const SidereonExactEpoch,
    out_equal: *mut bool,
) -> SidereonStatus {
    ffi_boundary("sidereon_exact_epoch_equal", SidereonStatus::Panic, || {
        let out = c_try!(require_out(
            out_equal,
            "sidereon_exact_epoch_equal",
            "out_equal"
        ));
        *out = false;
        let epoch = c_try!(require_ref(epoch, "sidereon_exact_epoch_equal", "epoch"));
        let other = c_try!(require_ref(other, "sidereon_exact_epoch_equal", "other"));
        *out = epoch.inner == other.inner;
        SidereonStatus::Ok
    })
}

#[no_mangle]
/// Compare query values by their exact mathematical value, independent of
/// their epoch origin and binary-offset representation.
///
/// Safety: both query handles must be live and out_equal must be writable.
pub unsafe extern "C" fn sidereon_exact_epoch_query_equal(
    query: *const SidereonExactEpochQuery,
    other: *const SidereonExactEpochQuery,
    out_equal: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_exact_epoch_query_equal",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_equal,
                "sidereon_exact_epoch_query_equal",
                "out_equal"
            ));
            *out = false;
            let query = c_try!(require_ref(
                query,
                "sidereon_exact_epoch_query_equal",
                "query"
            ));
            let other = c_try!(require_ref(
                other,
                "sidereon_exact_epoch_query_equal",
                "other"
            ));
            *out = query.inner == other.inner;
            SidereonStatus::Ok
        },
    )
}

/// Write an epoch's J2000 seconds rounded once from its exact value.
///
/// Safety: `epoch` must be live; `out_seconds` must be writable.
#[no_mangle]
pub unsafe extern "C" fn sidereon_exact_epoch_j2000_seconds(
    epoch: *const SidereonExactEpoch,
    out_seconds: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_exact_epoch_j2000_seconds",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_seconds,
                "sidereon_exact_epoch_j2000_seconds",
                "out_seconds"
            ));
            *out = 0.0;
            let epoch = c_try!(require_ref(
                epoch,
                "sidereon_exact_epoch_j2000_seconds",
                "epoch"
            ));
            *out = epoch.inner.j2000_seconds();
            SidereonStatus::Ok
        },
    )
}

/// Write an exact epoch as the nearest whole/fraction Julian-date pair.
///
/// Safety: `epoch` must be live; both outputs must be writable.
#[no_mangle]
pub unsafe extern "C" fn sidereon_exact_epoch_split_julian_date(
    epoch: *const SidereonExactEpoch,
    out_jd_whole: *mut f64,
    out_fraction: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_exact_epoch_split_julian_date",
        SidereonStatus::Panic,
        || {
            let out_jd_whole = c_try!(require_out(
                out_jd_whole,
                "sidereon_exact_epoch_split_julian_date",
                "out_jd_whole"
            ));
            let out_fraction = c_try!(require_out(
                out_fraction,
                "sidereon_exact_epoch_split_julian_date",
                "out_fraction"
            ));
            *out_jd_whole = 0.0;
            *out_fraction = 0.0;
            let epoch = c_try!(require_ref(
                epoch,
                "sidereon_exact_epoch_split_julian_date",
                "epoch"
            ));
            (*out_jd_whole, *out_fraction) = epoch.inner.split_julian_date();
            SidereonStatus::Ok
        },
    )
}

/// Write seconds from `earlier` to `epoch`, rounded once from the exact difference.
///
/// Safety: both handles must be live; `out_seconds` must be writable.
#[no_mangle]
pub unsafe extern "C" fn sidereon_exact_epoch_seconds_since(
    epoch: *const SidereonExactEpoch,
    earlier: *const SidereonExactEpoch,
    out_seconds: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_exact_epoch_seconds_since",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_seconds,
                "sidereon_exact_epoch_seconds_since",
                "out_seconds"
            ));
            *out = 0.0;
            let epoch = c_try!(require_ref(
                epoch,
                "sidereon_exact_epoch_seconds_since",
                "epoch"
            ));
            let earlier = c_try!(require_ref(
                earlier,
                "sidereon_exact_epoch_seconds_since",
                "earlier"
            ));
            *out = epoch.inner.seconds_since(earlier.inner);
            SidereonStatus::Ok
        },
    )
}

/// Write seconds from an exact epoch query to another exact epoch.
///
/// Safety: both handles must be live; `out_seconds` must be writable.
#[no_mangle]
pub unsafe extern "C" fn sidereon_exact_epoch_query_seconds_since(
    query: *const SidereonExactEpochQuery,
    earlier: *const SidereonExactEpoch,
    out_seconds: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_exact_epoch_query_seconds_since",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_seconds,
                "sidereon_exact_epoch_query_seconds_since",
                "out_seconds"
            ));
            *out = 0.0;
            let query = c_try!(require_ref(
                query,
                "sidereon_exact_epoch_query_seconds_since",
                "query"
            ));
            let earlier = c_try!(require_ref(
                earlier,
                "sidereon_exact_epoch_query_seconds_since",
                "earlier"
            ));
            *out = query.inner.seconds_since(earlier.inner);
            SidereonStatus::Ok
        },
    )
}

/// Write seconds between exact queries, rounded once from their exact difference.
///
/// Safety: both handles must be live; `out_seconds` must be writable.
#[no_mangle]
pub unsafe extern "C" fn sidereon_exact_epoch_query_seconds_since_query(
    query: *const SidereonExactEpochQuery,
    earlier: *const SidereonExactEpochQuery,
    out_seconds: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_exact_epoch_query_seconds_since_query",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_seconds,
                "sidereon_exact_epoch_query_seconds_since_query",
                "out_seconds"
            ));
            *out = 0.0;
            let query = c_try!(require_ref(
                query,
                "sidereon_exact_epoch_query_seconds_since_query",
                "query"
            ));
            let earlier = c_try!(require_ref(
                earlier,
                "sidereon_exact_epoch_query_seconds_since_query",
                "earlier"
            ));
            *out = query.inner.seconds_since_query(&earlier.inner);
            SidereonStatus::Ok
        },
    )
}

/// Write query seconds since J2000 rounded once from its exact value.
///
/// Safety: `query` must be live; `out_seconds` must be writable.
#[no_mangle]
pub unsafe extern "C" fn sidereon_exact_epoch_query_j2000_seconds(
    query: *const SidereonExactEpochQuery,
    out_seconds: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_exact_epoch_query_j2000_seconds",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_seconds,
                "sidereon_exact_epoch_query_j2000_seconds",
                "out_seconds"
            ));
            *out = 0.0;
            let query = c_try!(require_ref(
                query,
                "sidereon_exact_epoch_query_j2000_seconds",
                "query"
            ));
            *out = query.inner.j2000_seconds();
            SidereonStatus::Ok
        },
    )
}

/// Release an exact epoch handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_exact_epoch_free(epoch: *mut SidereonExactEpoch) {
    free_boxed(epoch);
}

/// Release an exact epoch query handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_exact_epoch_query_free(query: *mut SidereonExactEpochQuery) {
    free_boxed(query);
}

#[cfg(test)]
mod exact_epoch_semantics_tests {
    use super::*;
    use sidereon_core::astro::time::ExactEpoch;

    #[test]
    fn exact_epoch_and_query_semantics_are_not_derived_from_rounded_seconds() {
        let earliest = SidereonExactEpoch {
            inner: ExactEpoch::J2000,
        };
        let next = SidereonExactEpoch {
            inner: ExactEpoch::from_j2000_seconds(1.0e-30).expect("representable exact epoch"),
        };
        let mut ordering = SidereonExactOrdering::Equal;
        assert_eq!(
            unsafe { sidereon_exact_epoch_compare(&next, &earliest, &mut ordering) },
            SidereonStatus::Ok
        );
        assert_eq!(ordering, SidereonExactOrdering::Greater);

        let coarse = SidereonExactEpoch {
            inner: ExactEpoch::new(i64::MAX, 0).expect("valid whole-second epoch"),
        };
        let coarse_query = SidereonExactEpochQuery {
            inner: coarse
                .inner
                .query()
                .checked_add_binary_seconds(0.25)
                .expect("finite query offset"),
        };
        let base_query = SidereonExactEpochQuery {
            inner: coarse.inner.query(),
        };
        let mut rounded_base = 0.0;
        let mut rounded_offset = 0.0;
        assert_eq!(
            unsafe { sidereon_exact_epoch_query_j2000_seconds(&base_query, &mut rounded_base) },
            SidereonStatus::Ok
        );
        assert_eq!(
            unsafe { sidereon_exact_epoch_query_j2000_seconds(&coarse_query, &mut rounded_offset) },
            SidereonStatus::Ok
        );
        assert_eq!(rounded_base.to_bits(), rounded_offset.to_bits());
        let mut queries_equal = true;
        assert_eq!(
            unsafe {
                sidereon_exact_epoch_query_equal(&base_query, &coarse_query, &mut queries_equal)
            },
            SidereonStatus::Ok
        );
        assert!(!queries_equal);

        let integer_epoch = SidereonExactEpoch {
            inner: ExactEpoch::new(15, 0).expect("valid whole-second epoch"),
        };
        let civil_query = SidereonExactEpochQuery {
            inner: integer_epoch.inner.query(),
        };
        let binary_query = SidereonExactEpochQuery {
            inner: ExactEpoch::from_binary_j2000_seconds(15.0).expect("finite query"),
        };
        assert_eq!(
            unsafe {
                sidereon_exact_epoch_query_equal(&civil_query, &binary_query, &mut queries_equal)
            },
            SidereonStatus::Ok
        );
        assert!(queries_equal);

        let mut returned_epoch = ptr::null_mut();
        assert_eq!(
            unsafe { sidereon_exact_epoch_query_epoch(&coarse_query, &mut returned_epoch) },
            SidereonStatus::Ok
        );
        let returned_epoch_ref = unsafe { &*returned_epoch };
        let mut epochs_equal = false;
        assert_eq!(
            unsafe { sidereon_exact_epoch_equal(returned_epoch_ref, &coarse, &mut epochs_equal) },
            SidereonStatus::Ok
        );
        assert!(epochs_equal);
        unsafe { sidereon_exact_epoch_free(returned_epoch) };

        let mut j2000 = ptr::null_mut();
        assert_eq!(
            unsafe { sidereon_exact_epoch_j2000(&mut j2000) },
            SidereonStatus::Ok
        );
        let mut epochs_equal = false;
        assert_eq!(
            unsafe { sidereon_exact_epoch_equal(&earliest, &*j2000, &mut epochs_equal) },
            SidereonStatus::Ok
        );
        assert!(epochs_equal);
        unsafe { sidereon_exact_epoch_free(j2000) };

        let mut attoseconds_per_second = 0;
        assert_eq!(
            unsafe { sidereon_exact_epoch_attoseconds_per_second(&mut attoseconds_per_second) },
            SidereonStatus::Ok
        );
        assert_eq!(attoseconds_per_second, ExactEpoch::ATTOSECONDS_PER_SECOND);
    }
}
