use super::*;
use crate::engine_error::{
    engine_error_operation_boundary, engine_f64, record_engine_error, SidereonEngineErrorFamily,
};

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
    /// An observed-section row whose flux qualifier states the day had no
    /// flux observation (Q 3).
    NotObserved = 4,
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
    /// Permit rows whose flux qualifier states that the day had no
    /// observation. Refused by default.
    pub allow_not_observed: bool,
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
    engine_error_operation_boundary(
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
    engine_error_operation_boundary(
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
    engine_error_operation_boundary(
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
    engine_error_operation_boundary(
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
    engine_error_operation_boundary(
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
    engine_error_operation_boundary(
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
    engine_error_operation_boundary(
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

fn space_weather_policy_to_c(
    policy: sidereon_core::astro::space_weather::SpaceWeatherPolicy,
) -> SidereonSpaceWeatherPolicy {
    SidereonSpaceWeatherPolicy {
        allow_interpolated: policy.allow_interpolated,
        allow_not_observed: policy.allow_not_observed,
        allow_daily_predicted: policy.allow_daily_predicted,
        allow_monthly_predicted: policy.allow_monthly_predicted,
        require_geomagnetic: policy.require_geomagnetic,
    }
}

/// Write the engine's default space-weather policy to *out_policy: every row
/// class but the ones that state no observation, and no geomagnetic value the
/// file does not state. This is the policy a NULL policy argument selects.
///
/// Safety: out_policy points to a SidereonSpaceWeatherPolicy.
#[no_mangle]
pub unsafe extern "C" fn sidereon_space_weather_policy_default(
    out_policy: *mut SidereonSpaceWeatherPolicy,
) -> SidereonStatus {
    let fn_name = "sidereon_space_weather_policy_default";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_policy, fn_name, "out_policy"));
        *out = space_weather_policy_to_c(
            sidereon_core::astro::space_weather::SpaceWeatherPolicy::default(),
        );
        SidereonStatus::Ok
    })
}

/// Write the engine's lenient space-weather policy to *out_policy: every row
/// class, with the quiet default Ap or the daily Ap substituted where the file
/// leaves a geomagnetic value blank and each substitution reported on the
/// returned sample.
///
/// Safety: out_policy points to a SidereonSpaceWeatherPolicy.
#[no_mangle]
pub unsafe extern "C" fn sidereon_space_weather_policy_lenient(
    out_policy: *mut SidereonSpaceWeatherPolicy,
) -> SidereonStatus {
    let fn_name = "sidereon_space_weather_policy_lenient";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_policy, fn_name, "out_policy"));
        *out = space_weather_policy_to_c(
            sidereon_core::astro::space_weather::SpaceWeatherPolicy::lenient(),
        );
        SidereonStatus::Ok
    })
}

fn space_weather_policy_from_c(
    policy: *const SidereonSpaceWeatherPolicy,
) -> sidereon_core::astro::space_weather::SpaceWeatherPolicy {
    let Some(policy) = (unsafe { policy.as_ref() }) else {
        return sidereon_core::astro::space_weather::SpaceWeatherPolicy::default();
    };
    sidereon_core::astro::space_weather::SpaceWeatherPolicy {
        allow_interpolated: policy.allow_interpolated,
        allow_not_observed: policy.allow_not_observed,
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

fn space_weather_node(kind: &str, fields: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "kind": kind,
        "fields": fields,
    })
}

fn observation_class_name(
    class: sidereon_core::astro::space_weather::ObservationClass,
) -> &'static str {
    use sidereon_core::astro::space_weather::ObservationClass as C;
    match class {
        C::Observed => "observed",
        C::Interpolated => "interpolated",
        C::NotObserved => "not_observed",
        C::DailyPredicted => "daily_predicted",
        C::MonthlyPredicted => "monthly_predicted",
    }
}

pub(crate) fn space_weather_error_value(
    error: &sidereon_core::astro::space_weather::SpaceWeatherError,
) -> serde_json::Value {
    use sidereon_core::astro::space_weather::SpaceWeatherError as E;
    match error {
        E::UnrecognizedFormat => space_weather_node("unrecognized_format", serde_json::json!({})),
        E::Malformed { line, reason } => space_weather_node(
            "malformed",
            serde_json::json!({
                "line": line,
                "reason": reason,
            }),
        ),
        E::NotText => space_weather_node("not_text", serde_json::json!({})),
        E::BeforeCoverage {
            requested_j2000_s,
            first_j2000_s,
        } => space_weather_node(
            "before_coverage",
            serde_json::json!({
                "requested_j2000_s": engine_f64(*requested_j2000_s),
                "first_j2000_s": engine_f64(*first_j2000_s),
            }),
        ),
        E::AfterCoverage {
            requested_j2000_s,
            end_j2000_s,
        } => space_weather_node(
            "after_coverage",
            serde_json::json!({
                "requested_j2000_s": engine_f64(*requested_j2000_s),
                "end_j2000_s": engine_f64(*end_j2000_s),
            }),
        ),
        E::MissingData {
            year,
            month,
            day,
            field,
        } => space_weather_node(
            "missing_data",
            serde_json::json!({
                "year": year,
                "month": month,
                "day": day,
                "field": field,
            }),
        ),
        E::RejectedByPolicy {
            class,
            year,
            month,
            day,
        } => space_weather_node(
            "rejected_by_policy",
            serde_json::json!({
                "class": observation_class_name(*class),
                "year": year,
                "month": month,
                "day": day,
            }),
        ),
        E::InvalidEpoch { epoch_j2000_s_bits } => space_weather_node(
            "invalid_epoch",
            serde_json::json!({
                "epoch_j2000_s_bits": epoch_j2000_s_bits,
                "bits_hex": format!("{:016x}", epoch_j2000_s_bits),
            }),
        ),
    }
}

fn space_weather_error_to_status(
    err: &sidereon_core::astro::space_weather::SpaceWeatherError,
) -> SidereonStatus {
    match err {
        sidereon_core::astro::space_weather::SpaceWeatherError::NotText => {
            SidereonStatus::InvalidToken
        }
        sidereon_core::astro::space_weather::SpaceWeatherError::UnrecognizedFormat
        | sidereon_core::astro::space_weather::SpaceWeatherError::Malformed { .. }
        | sidereon_core::astro::space_weather::SpaceWeatherError::BeforeCoverage { .. }
        | sidereon_core::astro::space_weather::SpaceWeatherError::AfterCoverage { .. }
        | sidereon_core::astro::space_weather::SpaceWeatherError::MissingData { .. }
        | sidereon_core::astro::space_weather::SpaceWeatherError::RejectedByPolicy { .. }
        | sidereon_core::astro::space_weather::SpaceWeatherError::InvalidEpoch { .. } => {
            SidereonStatus::InvalidArgument
        }
    }
}

fn map_space_weather_error_retaining(
    fn_name: &str,
    err: &sidereon_core::astro::space_weather::SpaceWeatherError,
) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    space_weather_error_to_status(err)
}

fn map_space_weather_error(
    fn_name: &str,
    err: sidereon_core::astro::space_weather::SpaceWeatherError,
) -> SidereonStatus {
    record_engine_error(
        SidereonEngineErrorFamily::SpaceWeather,
        fn_name,
        space_weather_error_value(&err),
    );
    map_space_weather_error_retaining(fn_name, &err)
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
        sidereon_core::astro::space_weather::ObservationClass::NotObserved => {
            SidereonSpaceWeatherObservationClass::NotObserved as u32
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_error::{
        clear_engine_error, sidereon_last_engine_error_info, sidereon_last_engine_error_payload,
        SidereonEngineErrorFamily, SidereonEngineErrorInfo,
    };
    use serde_json::{json, Value};
    use sidereon_core::astro::space_weather::{ObservationClass, SpaceWeatherError};
    use std::ptr;

    #[test]
    fn table_driven_space_weather_error_mapping() {
        let cases: Vec<(SpaceWeatherError, &'static str)> = vec![
            (SpaceWeatherError::UnrecognizedFormat, "unrecognized_format"),
            (
                SpaceWeatherError::Malformed {
                    line: 4,
                    reason: "missing CSV headers".into(),
                },
                "malformed",
            ),
            (SpaceWeatherError::NotText, "not_text"),
            (
                SpaceWeatherError::BeforeCoverage {
                    requested_j2000_s: 100.0,
                    first_j2000_s: 200.0,
                },
                "before_coverage",
            ),
            (
                SpaceWeatherError::AfterCoverage {
                    requested_j2000_s: 500.0,
                    end_j2000_s: 400.0,
                },
                "after_coverage",
            ),
            (
                SpaceWeatherError::MissingData {
                    year: 2024,
                    month: 5,
                    day: 10,
                    field: "f107_obs",
                },
                "missing_data",
            ),
            (
                SpaceWeatherError::RejectedByPolicy {
                    class: ObservationClass::NotObserved,
                    year: 2024,
                    month: 5,
                    day: 11,
                },
                "rejected_by_policy",
            ),
            (
                SpaceWeatherError::InvalidEpoch {
                    epoch_j2000_s_bits: f64::NAN.to_bits(),
                },
                "invalid_epoch",
            ),
        ];

        for (err, kind) in cases {
            let v = space_weather_error_value(&err);
            assert_eq!(v["kind"], json!(kind));
            match &err {
                SpaceWeatherError::Malformed { line, reason } => {
                    assert_eq!(v["fields"]["line"], json!(line));
                    assert_eq!(v["fields"]["reason"], json!(reason));
                }
                SpaceWeatherError::BeforeCoverage {
                    requested_j2000_s,
                    first_j2000_s,
                } => {
                    assert_eq!(v["fields"]["requested_j2000_s"]["decimal"], json!("100"));
                    assert_eq!(
                        v["fields"]["requested_j2000_s"]["bits_hex"],
                        json!(format!("{:016x}", requested_j2000_s.to_bits()))
                    );
                    assert_eq!(v["fields"]["first_j2000_s"]["decimal"], json!("200"));
                    assert_eq!(
                        v["fields"]["first_j2000_s"]["bits_hex"],
                        json!(format!("{:016x}", first_j2000_s.to_bits()))
                    );
                }
                SpaceWeatherError::AfterCoverage {
                    requested_j2000_s,
                    end_j2000_s,
                } => {
                    assert_eq!(v["fields"]["requested_j2000_s"]["decimal"], json!("500"));
                    assert_eq!(
                        v["fields"]["requested_j2000_s"]["bits_hex"],
                        json!(format!("{:016x}", requested_j2000_s.to_bits()))
                    );
                    assert_eq!(v["fields"]["end_j2000_s"]["decimal"], json!("400"));
                    assert_eq!(
                        v["fields"]["end_j2000_s"]["bits_hex"],
                        json!(format!("{:016x}", end_j2000_s.to_bits()))
                    );
                }
                SpaceWeatherError::MissingData {
                    year,
                    month,
                    day,
                    field,
                } => {
                    assert_eq!(v["fields"]["year"], json!(year));
                    assert_eq!(v["fields"]["month"], json!(month));
                    assert_eq!(v["fields"]["day"], json!(day));
                    assert_eq!(v["fields"]["field"], json!(field));
                }
                SpaceWeatherError::RejectedByPolicy {
                    class: _,
                    year,
                    month,
                    day,
                } => {
                    assert_eq!(v["fields"]["class"], json!("not_observed"));
                    assert_eq!(v["fields"]["year"], json!(year));
                    assert_eq!(v["fields"]["month"], json!(month));
                    assert_eq!(v["fields"]["day"], json!(day));
                }
                SpaceWeatherError::InvalidEpoch { epoch_j2000_s_bits } => {
                    assert_eq!(v["fields"]["epoch_j2000_s_bits"], json!(epoch_j2000_s_bits));
                    assert_eq!(
                        v["fields"]["bits_hex"],
                        json!(format!("{:016x}", epoch_j2000_s_bits))
                    );
                }
                _ => {}
            }
        }
    }

    const VALID_CSV: &str = "DATE,BSRN,ND,KP1,KP2,KP3,KP4,KP5,KP6,KP7,KP8,KP_SUM,AP1,AP2,AP3,AP4,AP5,AP6,AP7,AP8,AP_AVG,CP,C9,ISN,F10.7_OBS,F10.7_ADJ,F10.7_DATA_TYPE,F10.7_OBS_CENTER81,F10.7_OBS_LAST81,F10.7_ADJ_CENTER81,F10.7_ADJ_LAST81\n\
        2024-05-09,2556,1,23,27,30,33,40,50,47,37,287,9,12,15,18,27,48,39,22,24,1.2,5,120,165.1,162.0,OBS,150.1,149.8,147.0,146.6\n\
        2024-05-10,2556,2,40,50,60,70,67,57,47,37,428,27,48,80,132,111,67,39,22,66,1.8,7,121,190.2,187.1,OBS,151.2,150.9,148.0,147.6\n";

    #[test]
    fn space_weather_producer_real_refusal_valid_control_and_retention() {
        clear_engine_error();

        unsafe {
            // Real public refusal 1: parse invalid format bytes
            let bad_data = b"not_a_valid_space_weather_file";
            let mut out_table: *mut SidereonSpaceWeatherTable = ptr::null_mut();
            assert_eq!(
                sidereon_space_weather_table_parse(
                    bad_data.as_ptr(),
                    bad_data.len(),
                    &mut out_table,
                ),
                SidereonStatus::InvalidArgument
            );
            assert!(out_table.is_null());

            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::SpaceWeather);
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
            assert_eq!(payload["family"], "space_weather");
            assert_eq!(payload["operation"], "sidereon_space_weather_table_parse");
            assert_eq!(payload["error"]["kind"], "unrecognized_format");

            // Build valid table handle for reader retention and valid control
            let mut live_table: *mut SidereonSpaceWeatherTable = ptr::null_mut();
            assert_eq!(
                sidereon_space_weather_table_parse_csv(
                    VALID_CSV.as_ptr(),
                    VALID_CSV.len(),
                    &mut live_table,
                ),
                SidereonStatus::Ok
            );
            assert!(!live_table.is_null());

            // Real public refusal 2: lookup before coverage
            let mut sample = SidereonSpaceWeatherSample {
                weather: SidereonSpaceWeather {
                    f107: 0.0,
                    f107a: 0.0,
                    ap: 0.0,
                },
                class: 0,
                ap_defaulted: false,
            };
            assert_eq!(
                sidereon_space_weather_table_sample_at(live_table, -1_000_000_000.0, &mut sample),
                SidereonStatus::InvalidArgument
            );
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::SpaceWeather);
            assert!(info.payload_len > 0);
            let sample_err_len = info.payload_len;

            let mut sample_buf = vec![0u8; sample_err_len];
            assert_eq!(
                sidereon_last_engine_error_payload(
                    sample_buf.as_mut_ptr(),
                    sample_buf.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            let sample_payload: Value = serde_json::from_slice(&sample_buf).expect("valid JSON");
            assert_eq!(sample_payload["error"]["kind"], "before_coverage");
            assert!(sample_payload["error"]["fields"]["requested_j2000_s"].is_object());
            assert!(sample_payload["error"]["fields"]["first_j2000_s"].is_object());

            // Livehandle reader retains TLS error: sidereon_space_weather_table_summary
            let mut summary = SidereonSpaceWeatherTableSummary {
                day_count: 0,
                monthly_count: 0,
                skip_count: 0,
                warning_count: 0,
            };
            assert_eq!(
                sidereon_space_weather_table_summary(live_table, &mut summary),
                SidereonStatus::Ok
            );
            assert_eq!(summary.day_count, 2);
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::SpaceWeather);
            assert_eq!(info.payload_len, sample_err_len);

            // Buffer count/copy retains TLS error: sidereon_space_weather_table_to_csv
            let mut csv_written = 999;
            let mut csv_required = 0;
            assert_eq!(
                sidereon_space_weather_table_to_csv(
                    live_table,
                    ptr::null_mut(),
                    0,
                    &mut csv_written,
                    &mut csv_required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(csv_written, 0);
            assert!(csv_required > 0);
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::SpaceWeather);
            assert_eq!(info.payload_len, sample_err_len);

            // Free retains TLS error
            sidereon_space_weather_table_free(live_table);
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::SpaceWeather);
            assert_eq!(info.payload_len, sample_err_len);

            // Valid success control clears TLS error
            let mut valid_table: *mut SidereonSpaceWeatherTable = ptr::null_mut();
            assert_eq!(
                sidereon_space_weather_table_parse_csv(
                    VALID_CSV.as_ptr(),
                    VALID_CSV.len(),
                    &mut valid_table,
                ),
                SidereonStatus::Ok
            );
            assert!(!valid_table.is_null());
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::None);
            assert_eq!(info.payload_len, 0);
            sidereon_space_weather_table_free(valid_table);

            // Re-seed error for early argument reset check
            assert_eq!(
                sidereon_space_weather_table_parse(
                    bad_data.as_ptr(),
                    bad_data.len(),
                    &mut out_table,
                ),
                SidereonStatus::InvalidArgument
            );
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::SpaceWeather);

            // Early argument refusal (null pointer) clears slot before checks!
            assert_eq!(
                sidereon_space_weather_table_parse_csv(
                    VALID_CSV.as_ptr(),
                    VALID_CSV.len(),
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
