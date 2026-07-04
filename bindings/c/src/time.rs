use super::*;

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
            Err(err) => extra_invalid_arg("sidereon_gnss_week_tow_new", err),
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
                Err(err) => extra_invalid_arg("sidereon_gnss_week_tow_normalized", err),
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
                Err(err) => extra_invalid_arg("sidereon_gnss_week_tow_unrolled_week", err),
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
/// Safety: out_present points to a bool; out_week points to a uint32_t.
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

/// Write seconds of week for a calendar date and time.
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
            *out_sow_s = seconds_of_week_from_calendar(year, month, day, hour, minute, second);
            SidereonStatus::Ok
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

/// GPS seconds for a civil instant (used to query RINEX clock series). Writes the
/// value to *out_gps_seconds and *out_available (false if the date is invalid).
/// Delegates to sidereon_core::rinex::clock::civil_to_gps_seconds.
///
/// Safety: out_gps_seconds points to a double; out_available points to a bool.
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
            if let Some(out) = out_jd_whole.as_mut() {
                *out = jd.jd_whole;
            }
            if let Some(out) = out_jd_fraction.as_mut() {
                *out = jd.fraction;
            }
            if let Some(out) = out_j2000_seconds.as_mut() {
                *out = instant_to_j2000_seconds(&instant).unwrap_or(f64::NAN);
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
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}

fn gnss_week_tow_to_c(value: GnssWeekTow) -> SidereonGnssWeekTow {
    SidereonGnssWeekTow {
        system: time_scale_to_c_code(value.system),
        week: value.week,
        tow_s: value.tow_s,
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
        }
    }

    pub(crate) fn to_core(self) -> CoreTimeScales {
        CoreTimeScales {
            jd_whole: self.jd_whole,
            ut1_fraction: self.ut1_fraction,
            tt_fraction: self.tt_fraction,
            tdb_fraction: self.tdb_fraction,
            jd_ut1: self.jd_ut1,
            jd_tt: self.jd_tt,
            jd_tdb: self.jd_tdb,
        }
    }
}
