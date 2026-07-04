use super::*;

/// Space-weather inputs used by the drag model.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSpaceWeather {
    /// Daily F10.7 from the previous day.
    pub f107: f64,
    /// 81-day centered average F10.7.
    pub f107a: f64,
    /// Daily Ap index.
    pub ap: f64,
}

/// Fill *out_weather with the core default quiet-Sun drag inputs.
///
/// Safety: out_weather must point to a SidereonSpaceWeather.
#[no_mangle]
pub unsafe extern "C" fn sidereon_space_weather_default(
    out_weather: *mut SidereonSpaceWeather,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_space_weather_default",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_weather,
                "sidereon_space_weather_default",
                "out_weather"
            ));
            *out = space_weather_to_c(SpaceWeather::default());
            SidereonStatus::Ok
        },
    )
}

// === Round-2 space-weather table and table-backed decay ======================

pub struct SidereonSpaceWeatherTable {
    pub(crate) inner: Arc<sidereon_core::astro::space_weather::SpaceWeatherTable>,
    pub(crate) skip_count: usize,
    pub(crate) warning_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSpaceWeatherObservationClass {
    Observed = 0,
    Interpolated = 1,
    DailyPredicted = 2,
    MonthlyPredicted = 3,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSpaceWeatherTableSummary {
    pub day_count: usize,
    pub monthly_count: usize,
    pub skip_count: usize,
    pub warning_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSpaceWeatherCoverage {
    pub first_j2000_s: f64,
    pub has_last_observed_j2000_s: bool,
    pub last_observed_j2000_s: f64,
    pub has_last_daily_predicted_j2000_s: bool,
    pub last_daily_predicted_j2000_s: f64,
    pub end_j2000_s: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSpaceWeatherPolicy {
    pub allow_interpolated: bool,
    pub allow_daily_predicted: bool,
    pub allow_monthly_predicted: bool,
    pub require_geomagnetic: bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSpaceWeatherSample {
    pub weather: SidereonSpaceWeather,
    pub class: u32,
    pub ap_defaulted: bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSpaceWeatherDay {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub class: u32,
    pub has_bsrn: bool,
    pub bsrn: u16,
    pub has_nd: bool,
    pub nd: u8,
    pub has_kp: [bool; 8],
    pub kp_10: [u16; 8],
    pub has_kp_sum_10: bool,
    pub kp_sum_10: u16,
    pub has_ap: [bool; 8],
    pub ap: [u16; 8],
    pub has_ap_avg: bool,
    pub ap_avg: u16,
    pub has_cp_10: bool,
    pub cp_10: u8,
    pub has_c9: bool,
    pub c9: u8,
    pub has_isn: bool,
    pub isn: u16,
    pub has_flux_qualifier: bool,
    pub flux_qualifier: u8,
    pub has_f107_obs: bool,
    pub f107_obs: f64,
    pub has_f107_adj: bool,
    pub f107_adj: f64,
    pub has_f107_obs_center81: bool,
    pub f107_obs_center81: f64,
    pub has_f107_obs_last81: bool,
    pub f107_obs_last81: f64,
    pub has_f107_adj_center81: bool,
    pub f107_adj_center81: f64,
    pub has_f107_adj_last81: bool,
    pub f107_adj_last81: f64,
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_space_weather_table_parse(
    data: *const u8,
    len: usize,
    out_table: *mut *mut SidereonSpaceWeatherTable,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_space_weather_table_parse",
        SidereonStatus::Panic,
        || {
            let out_table = c_try!(require_out(
                out_table,
                "sidereon_space_weather_table_parse",
                "out_table"
            ));
            *out_table = ptr::null_mut();
            let bytes = c_try!(require_slice(
                data,
                len,
                "sidereon_space_weather_table_parse",
                "data"
            ));
            match sidereon_core::astro::space_weather::parse(bytes) {
                Ok(parsed) => {
                    write_boxed_handle(out_table, space_weather_table_from_parsed(parsed));
                    SidereonStatus::Ok
                }
                Err(err) => map_space_weather_error("sidereon_space_weather_table_parse", err),
            }
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_space_weather_table_parse_csv(
    data: *const u8,
    len: usize,
    out_table: *mut *mut SidereonSpaceWeatherTable,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_space_weather_table_parse_csv",
        SidereonStatus::Panic,
        || {
            let out_table = c_try!(require_out(
                out_table,
                "sidereon_space_weather_table_parse_csv",
                "out_table"
            ));
            *out_table = ptr::null_mut();
            let text = c_try!(text_bytes_from_c(
                "sidereon_space_weather_table_parse_csv",
                data,
                len
            ));
            match sidereon_core::astro::space_weather::parse_csv(text) {
                Ok(parsed) => {
                    write_boxed_handle(out_table, space_weather_table_from_parsed(parsed));
                    SidereonStatus::Ok
                }
                Err(err) => map_space_weather_error("sidereon_space_weather_table_parse_csv", err),
            }
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_space_weather_table_parse_txt(
    data: *const u8,
    len: usize,
    out_table: *mut *mut SidereonSpaceWeatherTable,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_space_weather_table_parse_txt",
        SidereonStatus::Panic,
        || {
            let out_table = c_try!(require_out(
                out_table,
                "sidereon_space_weather_table_parse_txt",
                "out_table"
            ));
            *out_table = ptr::null_mut();
            let text = c_try!(text_bytes_from_c(
                "sidereon_space_weather_table_parse_txt",
                data,
                len
            ));
            match sidereon_core::astro::space_weather::parse_txt(text) {
                Ok(parsed) => {
                    write_boxed_handle(out_table, space_weather_table_from_parsed(parsed));
                    SidereonStatus::Ok
                }
                Err(err) => map_space_weather_error("sidereon_space_weather_table_parse_txt", err),
            }
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_space_weather_table_summary(
    table: *const SidereonSpaceWeatherTable,
    out_summary: *mut SidereonSpaceWeatherTableSummary,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_space_weather_table_summary",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_summary,
                "sidereon_space_weather_table_summary",
                "out_summary"
            ));
            let table = c_try!(require_ref(
                table,
                "sidereon_space_weather_table_summary",
                "table"
            ));
            *out = space_weather_table_summary(table);
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_space_weather_table_coverage(
    table: *const SidereonSpaceWeatherTable,
    out_coverage: *mut SidereonSpaceWeatherCoverage,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_space_weather_table_coverage",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_coverage,
                "sidereon_space_weather_table_coverage",
                "out_coverage"
            ));
            let table = c_try!(require_ref(
                table,
                "sidereon_space_weather_table_coverage",
                "table"
            ));
            let coverage = table.inner.coverage();
            *out = SidereonSpaceWeatherCoverage {
                first_j2000_s: coverage.first_j2000_s,
                has_last_observed_j2000_s: coverage.last_observed_j2000_s.is_some(),
                last_observed_j2000_s: coverage.last_observed_j2000_s.unwrap_or(0.0),
                has_last_daily_predicted_j2000_s: coverage.last_daily_predicted_j2000_s.is_some(),
                last_daily_predicted_j2000_s: coverage.last_daily_predicted_j2000_s.unwrap_or(0.0),
                end_j2000_s: coverage.end_j2000_s,
            };
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_space_weather_table_days(
    table: *const SidereonSpaceWeatherTable,
    out: *mut SidereonSpaceWeatherDay,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_space_weather_table_days",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_space_weather_table_days",
                out_written,
                out_required
            ));
            let table = c_try!(require_ref(
                table,
                "sidereon_space_weather_table_days",
                "table"
            ));
            let values: Vec<_> = table
                .inner
                .days()
                .iter()
                .map(space_weather_day_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_space_weather_table_days",
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
pub unsafe extern "C" fn sidereon_space_weather_table_monthly(
    table: *const SidereonSpaceWeatherTable,
    out: *mut SidereonSpaceWeatherDay,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_space_weather_table_monthly",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_space_weather_table_monthly",
                out_written,
                out_required
            ));
            let table = c_try!(require_ref(
                table,
                "sidereon_space_weather_table_monthly",
                "table"
            ));
            let values: Vec<_> = table
                .inner
                .monthly()
                .iter()
                .map(space_weather_day_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_space_weather_table_monthly",
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
pub unsafe extern "C" fn sidereon_space_weather_table_day(
    table: *const SidereonSpaceWeatherTable,
    year: i32,
    month: u8,
    day: u8,
    out_present: *mut bool,
    out_day: *mut SidereonSpaceWeatherDay,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_space_weather_table_day",
        SidereonStatus::Panic,
        || {
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_space_weather_table_day",
                "out_present"
            ));
            *out_present = false;
            let out_day = c_try!(require_out(
                out_day,
                "sidereon_space_weather_table_day",
                "out_day"
            ));
            *out_day = SidereonSpaceWeatherDay {
                year: 0,
                month: 0,
                day: 0,
                class: 0,
                has_bsrn: false,
                bsrn: 0,
                has_nd: false,
                nd: 0,
                has_kp: [false; 8],
                kp_10: [0; 8],
                has_kp_sum_10: false,
                kp_sum_10: 0,
                has_ap: [false; 8],
                ap: [0; 8],
                has_ap_avg: false,
                ap_avg: 0,
                has_cp_10: false,
                cp_10: 0,
                has_c9: false,
                c9: 0,
                has_isn: false,
                isn: 0,
                has_flux_qualifier: false,
                flux_qualifier: 0,
                has_f107_obs: false,
                f107_obs: 0.0,
                has_f107_adj: false,
                f107_adj: 0.0,
                has_f107_obs_center81: false,
                f107_obs_center81: 0.0,
                has_f107_obs_last81: false,
                f107_obs_last81: 0.0,
                has_f107_adj_center81: false,
                f107_adj_center81: 0.0,
                has_f107_adj_last81: false,
                f107_adj_last81: 0.0,
            };
            let table = c_try!(require_ref(
                table,
                "sidereon_space_weather_table_day",
                "table"
            ));
            if let Some(row) = table.inner.day(year, month, day) {
                *out_present = true;
                *out_day = space_weather_day_to_c(row);
            }
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_space_weather_table_sample_at(
    table: *const SidereonSpaceWeatherTable,
    epoch_j2000_s: f64,
    out_sample: *mut SidereonSpaceWeatherSample,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_space_weather_table_sample_at",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_sample,
                "sidereon_space_weather_table_sample_at",
                "out_sample"
            ));
            *out = SidereonSpaceWeatherSample {
                weather: SidereonSpaceWeather {
                    f107: 0.0,
                    f107a: 0.0,
                    ap: 0.0,
                },
                class: 0,
                ap_defaulted: false,
            };
            let table = c_try!(require_ref(
                table,
                "sidereon_space_weather_table_sample_at",
                "table"
            ));
            match table.inner.sample_at(epoch_j2000_s) {
                Ok(sample) => {
                    *out = space_weather_sample_to_c(sample);
                    SidereonStatus::Ok
                }
                Err(err) => map_space_weather_error("sidereon_space_weather_table_sample_at", err),
            }
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_space_weather_table_sample_at_with_policy(
    table: *const SidereonSpaceWeatherTable,
    epoch_j2000_s: f64,
    policy: *const SidereonSpaceWeatherPolicy,
    out_sample: *mut SidereonSpaceWeatherSample,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_space_weather_table_sample_at_with_policy",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_sample,
                "sidereon_space_weather_table_sample_at_with_policy",
                "out_sample"
            ));
            *out = SidereonSpaceWeatherSample {
                weather: SidereonSpaceWeather {
                    f107: 0.0,
                    f107a: 0.0,
                    ap: 0.0,
                },
                class: 0,
                ap_defaulted: false,
            };
            let table = c_try!(require_ref(
                table,
                "sidereon_space_weather_table_sample_at_with_policy",
                "table"
            ));
            let policy = space_weather_policy_from_c(policy);
            match table.inner.sample_at_with_policy(epoch_j2000_s, policy) {
                Ok(sample) => {
                    *out = space_weather_sample_to_c(sample);
                    SidereonStatus::Ok
                }
                Err(err) => map_space_weather_error(
                    "sidereon_space_weather_table_sample_at_with_policy",
                    err,
                ),
            }
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_space_weather_table_space_weather_at(
    table: *const SidereonSpaceWeatherTable,
    epoch_j2000_s: f64,
    out_weather: *mut SidereonSpaceWeather,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_space_weather_table_space_weather_at",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_weather,
                "sidereon_space_weather_table_space_weather_at",
                "out_weather"
            ));
            *out = SidereonSpaceWeather {
                f107: 0.0,
                f107a: 0.0,
                ap: 0.0,
            };
            let table = c_try!(require_ref(
                table,
                "sidereon_space_weather_table_space_weather_at",
                "table"
            ));
            match table.inner.space_weather_at(epoch_j2000_s) {
                Ok(weather) => {
                    *out = space_weather_to_c(weather);
                    SidereonStatus::Ok
                }
                Err(err) => {
                    map_space_weather_error("sidereon_space_weather_table_space_weather_at", err)
                }
            }
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_space_weather_table_ap_array_at(
    table: *const SidereonSpaceWeatherTable,
    epoch_j2000_s: f64,
    out_ap_array: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_space_weather_table_ap_array_at",
        SidereonStatus::Panic,
        || {
            c_try!(require_out(
                out_ap_array,
                "sidereon_space_weather_table_ap_array_at",
                "out_ap_array"
            ));
            for idx in 0..SIDEREON_ATMOSPHERE_AP_ARRAY_LEN {
                *out_ap_array.add(idx) = 0.0;
            }
            let table = c_try!(require_ref(
                table,
                "sidereon_space_weather_table_ap_array_at",
                "table"
            ));
            match table.inner.ap_array_at(epoch_j2000_s) {
                Ok(ap) => {
                    ptr::copy_nonoverlapping(
                        ap.as_ptr(),
                        out_ap_array,
                        SIDEREON_ATMOSPHERE_AP_ARRAY_LEN,
                    );
                    SidereonStatus::Ok
                }
                Err(err) => {
                    map_space_weather_error("sidereon_space_weather_table_ap_array_at", err)
                }
            }
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_space_weather_table_to_csv(
    table: *const SidereonSpaceWeatherTable,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_space_weather_table_to_csv",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_space_weather_table_to_csv",
                out_written,
                out_required
            ));
            let table = c_try!(require_ref(
                table,
                "sidereon_space_weather_table_to_csv",
                "table"
            ));
            let text = sidereon_core::astro::space_weather::encode_csv(&table.inner);
            c_try!(copy_prefix_to_c(
                "sidereon_space_weather_table_to_csv",
                "out",
                text.as_bytes(),
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
pub unsafe extern "C" fn sidereon_space_weather_table_to_txt(
    table: *const SidereonSpaceWeatherTable,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_space_weather_table_to_txt",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_space_weather_table_to_txt",
                out_written,
                out_required
            ));
            let table = c_try!(require_ref(
                table,
                "sidereon_space_weather_table_to_txt",
                "table"
            ));
            let text = sidereon_core::astro::space_weather::encode_txt(&table.inner);
            c_try!(copy_prefix_to_c(
                "sidereon_space_weather_table_to_txt",
                "out",
                text.as_bytes(),
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
pub unsafe extern "C" fn sidereon_space_weather_table_free(table: *mut SidereonSpaceWeatherTable) {
    free_boxed(table);
}

fn space_weather_policy_from_c(
    policy: *const SidereonSpaceWeatherPolicy,
) -> sidereon_core::astro::space_weather::SpaceWeatherPolicy {
    let Some(policy) = (unsafe { policy.as_ref() }) else {
        return sidereon_core::astro::space_weather::SpaceWeatherPolicy::default();
    };
    sidereon_core::astro::space_weather::SpaceWeatherPolicy {
        allow_interpolated: policy.allow_interpolated,
        allow_daily_predicted: policy.allow_daily_predicted,
        allow_monthly_predicted: policy.allow_monthly_predicted,
        require_geomagnetic: policy.require_geomagnetic,
    }
}

fn space_weather_sample_to_c(
    sample: sidereon_core::astro::space_weather::SpaceWeatherSample,
) -> SidereonSpaceWeatherSample {
    SidereonSpaceWeatherSample {
        weather: space_weather_to_c(sample.space_weather),
        class: space_weather_observation_class_to_c(sample.class),
        ap_defaulted: sample.ap_defaulted,
    }
}

fn space_weather_day_to_c(
    row: &sidereon_core::astro::space_weather::SpaceWeatherDay,
) -> SidereonSpaceWeatherDay {
    let (has_kp, kp_10) = option_u16_array_to_c(row.kp_10);
    let (has_ap, ap) = option_u16_array_to_c(row.ap);
    SidereonSpaceWeatherDay {
        year: row.year,
        month: row.month,
        day: row.day,
        class: space_weather_observation_class_to_c(row.class),
        has_bsrn: row.bsrn.is_some(),
        bsrn: row.bsrn.unwrap_or(0),
        has_nd: row.nd.is_some(),
        nd: row.nd.unwrap_or(0),
        has_kp,
        kp_10,
        has_kp_sum_10: row.kp_sum_10.is_some(),
        kp_sum_10: row.kp_sum_10.unwrap_or(0),
        has_ap,
        ap,
        has_ap_avg: row.ap_avg.is_some(),
        ap_avg: row.ap_avg.unwrap_or(0),
        has_cp_10: row.cp_10.is_some(),
        cp_10: row.cp_10.unwrap_or(0),
        has_c9: row.c9.is_some(),
        c9: row.c9.unwrap_or(0),
        has_isn: row.isn.is_some(),
        isn: row.isn.unwrap_or(0),
        has_flux_qualifier: row.flux_qualifier.is_some(),
        flux_qualifier: row.flux_qualifier.unwrap_or(0),
        has_f107_obs: row.f107_obs.is_some(),
        f107_obs: row.f107_obs.unwrap_or(0.0),
        has_f107_adj: row.f107_adj.is_some(),
        f107_adj: row.f107_adj.unwrap_or(0.0),
        has_f107_obs_center81: row.f107_obs_center81.is_some(),
        f107_obs_center81: row.f107_obs_center81.unwrap_or(0.0),
        has_f107_obs_last81: row.f107_obs_last81.is_some(),
        f107_obs_last81: row.f107_obs_last81.unwrap_or(0.0),
        has_f107_adj_center81: row.f107_adj_center81.is_some(),
        f107_adj_center81: row.f107_adj_center81.unwrap_or(0.0),
        has_f107_adj_last81: row.f107_adj_last81.is_some(),
        f107_adj_last81: row.f107_adj_last81.unwrap_or(0.0),
    }
}

fn space_weather_table_summary(
    table: &SidereonSpaceWeatherTable,
) -> SidereonSpaceWeatherTableSummary {
    SidereonSpaceWeatherTableSummary {
        day_count: table.inner.days().len(),
        monthly_count: table.inner.monthly().len(),
        skip_count: table.skip_count,
        warning_count: table.warning_count,
    }
}

fn space_weather_table_from_parsed(
    parsed: sidereon_core::astro::space_weather::Parsed<
        sidereon_core::astro::space_weather::SpaceWeatherTable,
    >,
) -> SidereonSpaceWeatherTable {
    SidereonSpaceWeatherTable {
        inner: Arc::new(parsed.value),
        skip_count: parsed.diagnostics.skips.len(),
        warning_count: parsed.diagnostics.warnings.len(),
    }
}

fn map_space_weather_error(
    fn_name: &str,
    err: sidereon_core::astro::space_weather::SpaceWeatherError,
) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        sidereon_core::astro::space_weather::SpaceWeatherError::NotText => {
            SidereonStatus::InvalidToken
        }
        _ => SidereonStatus::InvalidArgument,
    }
}

fn space_weather_observation_class_to_c(
    class: sidereon_core::astro::space_weather::ObservationClass,
) -> u32 {
    match class {
        sidereon_core::astro::space_weather::ObservationClass::Observed => {
            SidereonSpaceWeatherObservationClass::Observed as u32
        }
        sidereon_core::astro::space_weather::ObservationClass::Interpolated => {
            SidereonSpaceWeatherObservationClass::Interpolated as u32
        }
        sidereon_core::astro::space_weather::ObservationClass::DailyPredicted => {
            SidereonSpaceWeatherObservationClass::DailyPredicted as u32
        }
        sidereon_core::astro::space_weather::ObservationClass::MonthlyPredicted => {
            SidereonSpaceWeatherObservationClass::MonthlyPredicted as u32
        }
    }
}

fn option_u16_array_to_c(values: [Option<u16>; 8]) -> ([bool; 8], [u16; 8]) {
    let mut present = [false; 8];
    let mut out = [0u16; 8];
    for (idx, value) in values.into_iter().enumerate() {
        if let Some(value) = value {
            present[idx] = true;
            out[idx] = value;
        }
    }
    (present, out)
}
