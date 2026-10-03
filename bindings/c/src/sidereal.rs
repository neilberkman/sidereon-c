use super::*;
use crate::engine_error::{
    engine_error_operation_boundary, record_engine_error, SidereonEngineErrorFamily,
};

/// Template estimator for sidereal residual filtering.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSiderealTemplateMethod {
    /// Arithmetic mean of prior values in each phase bin.
    Mean = 0,
    /// MAD-gated mean of prior values in each phase bin.
    RobustMad = 1,
    /// Exponentially weighted mean of prior values in each phase bin.
    Ewma = 2,
}

/// Options for sidereal residual filtering.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSiderealFilterOptions {
    /// Sampling interval of the residual series, seconds.
    pub sample_interval_s: f64,
    /// Maximum number of prior repeats retained per phase bin.
    pub prior_periods: usize,
    /// Minimum prior samples required before applying a correction.
    pub min_coverage: usize,
    /// Template method, as SidereonSiderealTemplateMethod.
    pub template_method: u32,
    /// EWMA gain used when template_method is Ewma.
    pub ewma_alpha: f64,
}

/// One period-strength score from sidereon_sidereal_periodicity_strength.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSiderealPeriodicityStrength {
    /// Candidate period, seconds.
    pub period_s: f64,
    /// Robust variance-reduction score in [0, 1].
    pub strength: f64,
}

/// Sidereal filter output. Opaque to C. Create with sidereon_sidereal_filter
/// and release with sidereon_sidereal_filter_output_free.
pub struct SidereonSiderealFilterOutput {
    pub(crate) inner: sidereon_core::sidereal::SiderealFilterOutput,
}

/// Initialize sidereal filter options with the core defaults.
///
/// Safety: out_options must point to a SidereonSiderealFilterOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sidereal_filter_options_init(
    out_options: *mut SidereonSiderealFilterOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sidereal_filter_options_init",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_options,
                "sidereon_sidereal_filter_options_init",
                "out_options"
            ));
            *out = sidereal_filter_options_to_c(
                sidereon_core::sidereal::SiderealFilterOptions::default(),
            );
            SidereonStatus::Ok
        },
    )
}

/// Return the default constellation repeat period, in seconds.
///
/// Safety: out_period_s must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sidereal_repeat_period(
    system: u32,
    out_period_s: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sidereal_repeat_period",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_period_s,
                "sidereon_sidereal_repeat_period",
                "out_period_s"
            ));
            *out = 0.0;
            let system = c_try!(gnss_system_from_c_code(
                "sidereon_sidereal_repeat_period",
                "system",
                system,
            ));
            *out = sidereon_core::sidereal::repeat_period(system).as_seconds();
            SidereonStatus::Ok
        },
    )
}

/// Compute a broadcast-orbit repeat lag for one satellite, in seconds.
///
/// Safety: broadcast must be a live handle; sat_id must be a null-terminated
/// satellite token; out_period_s must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sidereal_orbit_repeat_lag(
    broadcast: *const SidereonBroadcastEphemeris,
    sat_id: *const c_char,
    near_epoch_j2000_s: f64,
    out_period_s: *mut f64,
) -> SidereonStatus {
    engine_error_operation_boundary(
        "sidereon_sidereal_orbit_repeat_lag",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_period_s,
                "sidereon_sidereal_orbit_repeat_lag",
                "out_period_s"
            ));
            *out = 0.0;
            let broadcast = c_try!(require_ref(
                broadcast,
                "sidereon_sidereal_orbit_repeat_lag",
                "broadcast"
            ));
            let sat = c_try!(parse_satellite_token(
                "sidereon_sidereal_orbit_repeat_lag",
                sat_id,
            ));
            match sidereon_core::sidereal::orbit_repeat_lag(
                &broadcast.inner,
                sat,
                near_epoch_j2000_s,
            ) {
                Ok(period) => {
                    *out = period.as_seconds();
                    SidereonStatus::Ok
                }
                Err(err) => map_sidereal_error("sidereon_sidereal_orbit_repeat_lag", err),
            }
        },
    )
}

/// Filter a residual series by phase-stacked sidereal repeat templates. On
/// success writes a handle to *out_output; release it with
/// sidereon_sidereal_filter_output_free.
///
/// Safety: series points to count doubles, options may be NULL for defaults,
/// and out_output must point to SidereonSiderealFilterOutput* storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sidereal_filter(
    series: *const f64,
    count: usize,
    period_s: f64,
    options: *const SidereonSiderealFilterOptions,
    out_output: *mut *mut SidereonSiderealFilterOutput,
) -> SidereonStatus {
    engine_error_operation_boundary("sidereon_sidereal_filter", SidereonStatus::Panic, || {
        let out_output = c_try!(require_out(
            out_output,
            "sidereon_sidereal_filter",
            "out_output"
        ));
        *out_output = ptr::null_mut();
        let series = c_try!(require_slice(
            series,
            count,
            "sidereon_sidereal_filter",
            "series"
        ));
        let period = c_try!(duration_from_seconds(
            "sidereon_sidereal_filter",
            "period_s",
            period_s,
        ));
        let options = c_try!(sidereal_filter_options_from_c(
            "sidereon_sidereal_filter",
            options,
        ));
        match sidereon_core::sidereal::sidereal_filter(series, period, options) {
            Ok(inner) => {
                write_boxed_handle(out_output, SidereonSiderealFilterOutput { inner });
                SidereonStatus::Ok
            }
            Err(err) => map_sidereal_error("sidereon_sidereal_filter", err),
        }
    })
}

/// Copy sidereal-filtered residuals. Uses the variable-length output contract.
///
/// Safety: output must be a live handle; out may be NULL only when len is zero.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sidereal_filter_output_filtered(
    output: *const SidereonSiderealFilterOutput,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    sidereal_output_f64(
        "sidereon_sidereal_filter_output_filtered",
        output,
        out,
        len,
        out_written,
        out_required,
        |inner| &inner.filtered,
    )
}

/// Copy sidereal template values by phase bin. Uses the variable-length output
/// contract.
///
/// Safety: output must be a live handle; out may be NULL only when len is zero.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sidereal_filter_output_template(
    output: *const SidereonSiderealFilterOutput,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    sidereal_output_f64(
        "sidereon_sidereal_filter_output_template",
        output,
        out,
        len,
        out_written,
        out_required,
        |inner| &inner.template,
    )
}

/// Copy per-bin coverage counts. Uses the variable-length output contract.
///
/// Safety: output must be a live handle; out may be NULL only when len is zero.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sidereal_filter_output_coverage(
    output: *const SidereonSiderealFilterOutput,
    out: *mut usize,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sidereal_filter_output_coverage",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_sidereal_filter_output_coverage",
                out_written,
                out_required
            ));
            let output = c_try!(require_ref(
                output,
                "sidereon_sidereal_filter_output_coverage",
                "output"
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_sidereal_filter_output_coverage",
                "out",
                &output.inner.coverage,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy per-bin under-covered flags. Uses the variable-length output contract.
///
/// Safety: output must be a live handle; out may be NULL only when len is zero.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sidereal_filter_output_under_covered(
    output: *const SidereonSiderealFilterOutput,
    out: *mut bool,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sidereal_filter_output_under_covered",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_sidereal_filter_output_under_covered",
                out_written,
                out_required
            ));
            let output = c_try!(require_ref(
                output,
                "sidereon_sidereal_filter_output_under_covered",
                "output"
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_sidereal_filter_output_under_covered",
                "out",
                &output.inner.under_covered,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Release a sidereal filter output handle. Null is a no-op.
///
/// Safety: output must be NULL or a live handle from sidereon_sidereal_filter.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sidereal_filter_output_free(
    output: *mut SidereonSiderealFilterOutput,
) {
    ffi_boundary("sidereon_sidereal_filter_output_free", (), || {
        free_boxed(output);
    });
}

/// Score repeating components at candidate periods for 1 Hz samples. Uses the
/// variable-length output contract.
///
/// Safety: series and candidate_periods_s point to their counts; out may be
/// NULL only when len is zero.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sidereal_periodicity_strength(
    series: *const f64,
    count: usize,
    candidate_periods_s: *const f64,
    candidate_count: usize,
    out: *mut SidereonSiderealPeriodicityStrength,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    engine_error_operation_boundary(
        "sidereon_sidereal_periodicity_strength",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_sidereal_periodicity_strength",
                out_written,
                out_required
            ));
            let series = c_try!(require_slice(
                series,
                count,
                "sidereon_sidereal_periodicity_strength",
                "series"
            ));
            let raw_periods = c_try!(require_slice(
                candidate_periods_s,
                candidate_count,
                "sidereon_sidereal_periodicity_strength",
                "candidate_periods_s"
            ));
            let mut periods = Vec::with_capacity(raw_periods.len());
            for &period_s in raw_periods {
                periods.push(c_try!(duration_from_seconds(
                    "sidereon_sidereal_periodicity_strength",
                    "candidate_periods_s",
                    period_s,
                )));
            }
            let scores = match sidereon_core::sidereal::periodicity_strength(series, &periods) {
                Ok(scores) => scores,
                Err(err) => {
                    return map_sidereal_error("sidereon_sidereal_periodicity_strength", err);
                }
            };
            let values: Vec<SidereonSiderealPeriodicityStrength> = scores
                .into_iter()
                .map(|(period, strength)| SidereonSiderealPeriodicityStrength {
                    period_s: period.as_seconds(),
                    strength,
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_sidereal_periodicity_strength",
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

unsafe fn sidereal_output_f64(
    fn_name: &str,
    output: *const SidereonSiderealFilterOutput,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
    select: impl FnOnce(&sidereon_core::sidereal::SiderealFilterOutput) -> &[f64],
) -> SidereonStatus {
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(fn_name, out_written, out_required));
        let output = c_try!(require_ref(output, fn_name, "output"));
        c_try!(copy_prefix_to_c(
            fn_name,
            "out",
            select(&output.inner),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

fn sidereal_filter_options_to_c(
    options: sidereon_core::sidereal::SiderealFilterOptions,
) -> SidereonSiderealFilterOptions {
    let (template_method, ewma_alpha) = match options.template_method {
        sidereon_core::sidereal::SiderealTemplateMethod::Mean => {
            (SidereonSiderealTemplateMethod::Mean as u32, 0.0)
        }
        sidereon_core::sidereal::SiderealTemplateMethod::RobustMad => {
            (SidereonSiderealTemplateMethod::RobustMad as u32, 0.0)
        }
        sidereon_core::sidereal::SiderealTemplateMethod::Ewma { alpha } => {
            (SidereonSiderealTemplateMethod::Ewma as u32, alpha)
        }
    };
    SidereonSiderealFilterOptions {
        sample_interval_s: options.sample_interval.as_seconds(),
        prior_periods: options.prior_periods,
        min_coverage: options.min_coverage,
        template_method,
        ewma_alpha,
    }
}

unsafe fn sidereal_filter_options_from_c(
    fn_name: &str,
    options: *const SidereonSiderealFilterOptions,
) -> Result<sidereon_core::sidereal::SiderealFilterOptions, SidereonStatus> {
    if options.is_null() {
        return Ok(sidereon_core::sidereal::SiderealFilterOptions::default());
    }
    let options = require_ref(options, fn_name, "options")?;
    let mut o = sidereon_core::sidereal::SiderealFilterOptions::default();
    o.sample_interval = duration_from_seconds(
        fn_name,
        "options.sample_interval_s",
        options.sample_interval_s,
    )?;
    o.prior_periods = options.prior_periods;
    o.min_coverage = options.min_coverage;
    o.template_method = sidereal_template_method_from_c(fn_name, options)?;
    Ok(o)
}

fn sidereal_template_method_from_c(
    fn_name: &str,
    options: &SidereonSiderealFilterOptions,
) -> Result<sidereon_core::sidereal::SiderealTemplateMethod, SidereonStatus> {
    match options.template_method {
        value if value == SidereonSiderealTemplateMethod::Mean as u32 => {
            Ok(sidereon_core::sidereal::SiderealTemplateMethod::Mean)
        }
        value if value == SidereonSiderealTemplateMethod::RobustMad as u32 => {
            Ok(sidereon_core::sidereal::SiderealTemplateMethod::RobustMad)
        }
        value if value == SidereonSiderealTemplateMethod::Ewma as u32 => {
            Ok(sidereon_core::sidereal::SiderealTemplateMethod::Ewma {
                alpha: options.ewma_alpha,
            })
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid sidereal template method"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn duration_from_seconds(
    fn_name: &str,
    arg_name: &str,
    seconds: f64,
) -> Result<sidereon_core::astro::time::Duration, SidereonStatus> {
    sidereon_core::astro::time::Duration::from_seconds(seconds).map_err(|err| {
        let legacy_message = format!("{fn_name}: invalid {arg_name}: {err}");
        crate::engine_error::time_model_error(fn_name, err, legacy_message)
    })
}

fn sidereal_node(kind: &str, fields: serde_json::Value) -> serde_json::Value {
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

/// Convert a `SiderealFilterError` into a structured JSON error node.
pub(crate) fn sidereal_error_value(
    error: &sidereon_core::sidereal::SiderealFilterError,
) -> serde_json::Value {
    use sidereon_core::sidereal::SiderealFilterError as E;
    match error {
        E::InvalidInput { field, reason } => sidereal_node(
            "invalid_input",
            serde_json::json!({
                "field": *field,
                "reason": *reason,
            }),
        ),
        E::NoBroadcastRecord { sat } => sidereal_node(
            "no_broadcast_record",
            serde_json::json!({
                "sat": gnss_satellite_id_value(sat),
            }),
        ),
        E::UnsupportedConstellation { system } => sidereal_node(
            "unsupported_constellation",
            serde_json::json!({
                "system": gnss_system_name(*system),
            }),
        ),
    }
}

fn map_sidereal_error(
    fn_name: &str,
    err: sidereon_core::sidereal::SiderealFilterError,
) -> SidereonStatus {
    record_engine_error(
        SidereonEngineErrorFamily::Sidereal,
        fn_name,
        sidereal_error_value(&err),
    );
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_error::{
        clear_engine_error, sidereon_last_engine_error_info, sidereon_last_engine_error_payload,
        SidereonEngineErrorFamily, SidereonEngineErrorInfo,
    };
    use crate::SidereonStatus;
    use serde_json::Value;
    use std::ptr;

    #[test]
    fn table_driven_sidereal_error_mapping() {
        use sidereon_core::sidereal::SiderealFilterError as E;
        use sidereon_core::{GnssSatelliteId, GnssSystem};

        let systems = [
            (GnssSystem::Gps, "gps"),
            (GnssSystem::Glonass, "glonass"),
            (GnssSystem::Galileo, "galileo"),
            (GnssSystem::BeiDou, "beidou"),
            (GnssSystem::Qzss, "qzss"),
            (GnssSystem::Navic, "navic"),
            (GnssSystem::Sbas, "sbas"),
        ];

        // 1. InvalidInput
        let err = E::InvalidInput {
            field: "series",
            reason: "must be finite",
        };
        let val = sidereal_error_value(&err);
        assert_eq!(val["kind"], "invalid_input");
        assert_eq!(val["fields"]["field"], "series");
        assert_eq!(val["fields"]["reason"], "must be finite");

        let err = E::InvalidInput {
            field: "options.min_coverage",
            reason: "must be positive",
        };
        let val = sidereal_error_value(&err);
        assert_eq!(val["kind"], "invalid_input");
        assert_eq!(val["fields"]["field"], "options.min_coverage");
        assert_eq!(val["fields"]["reason"], "must be positive");

        // 2. NoBroadcastRecord with typedSatelliteId
        for (sys, sys_name) in systems {
            for prn in [1u8, 24, 32, 99] {
                let err = E::NoBroadcastRecord {
                    sat: GnssSatelliteId { system: sys, prn },
                };
                let val = sidereal_error_value(&err);
                assert_eq!(val["kind"], "no_broadcast_record");
                assert_eq!(val["fields"]["sat"]["system"], sys_name);
                assert_eq!(val["fields"]["sat"]["prn"], prn);
            }
        }

        // 3. UnsupportedConstellation with typed System
        for (sys, sys_name) in systems {
            let err = E::UnsupportedConstellation { system: sys };
            let val = sidereal_error_value(&err);
            assert_eq!(val["kind"], "unsupported_constellation");
            assert_eq!(val["fields"]["system"], sys_name);
        }
    }

    #[test]
    fn sidereal_public_producer_control_and_refusals() {
        clear_engine_error();

        unsafe {
            // Seed real existing domain refusal immediately before valid success control
            let bad_series = [0.1, f64::NAN, 0.3];
            let mut out_output: *mut SidereonSiderealFilterOutput = ptr::null_mut();
            assert_eq!(
                sidereon_sidereal_filter(
                    bad_series.as_ptr(),
                    bad_series.len(),
                    86164.0,
                    ptr::null(),
                    &mut out_output,
                ),
                SidereonStatus::InvalidArgument
            );
            assert!(out_output.is_null());

            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Sidereal);
            assert!(info.payload_len > 0);

            // Valid success control clears TLS error
            let series = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8];
            assert_eq!(
                sidereon_sidereal_filter(
                    series.as_ptr(),
                    series.len(),
                    86164.0,
                    ptr::null(),
                    &mut out_output,
                ),
                SidereonStatus::Ok
            );
            assert!(!out_output.is_null());

            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::None);
            assert_eq!(info.payload_len, 0);

            // Free output and verify free/reader retention
            sidereon_sidereal_filter_output_free(out_output);
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::None);
            assert_eq!(info.payload_len, 0);
        }

        // Real public refusal: non-finite sample in series
        unsafe {
            let bad_series = [0.1, f64::NAN, 0.3];
            let mut out_output: *mut SidereonSiderealFilterOutput = ptr::null_mut();
            assert_eq!(
                sidereon_sidereal_filter(
                    bad_series.as_ptr(),
                    bad_series.len(),
                    86164.0,
                    ptr::null(),
                    &mut out_output,
                ),
                SidereonStatus::InvalidArgument
            );
            assert!(out_output.is_null());

            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Sidereal);
            assert!(info.payload_len > 0);

            let mut written = 0;
            let mut required = 0;
            let mut buf = vec![0u8; info.payload_len];
            assert_eq!(
                sidereon_last_engine_error_payload(
                    buf.as_mut_ptr(),
                    buf.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, info.payload_len);
            let payload: Value = serde_json::from_slice(&buf).expect("valid JSON");
            assert_eq!(payload["schema_version"], 1);
            assert_eq!(payload["family"], "sidereal");
            assert_eq!(payload["operation"], "sidereon_sidereal_filter");
            assert_eq!(payload["error"]["kind"], "invalid_input");
            assert_eq!(payload["error"]["fields"]["field"], "series");
            assert_eq!(payload["error"]["fields"]["reason"], "must be finite");
        }
    }

    #[test]
    fn sidereal_producer_early_clearing_and_retention() {
        clear_engine_error();

        unsafe {
            // Seed error via real refusal
            let bad_series = [0.1, f64::NAN, 0.3];
            let mut out_output: *mut SidereonSiderealFilterOutput = ptr::null_mut();
            assert_eq!(
                sidereon_sidereal_filter(
                    bad_series.as_ptr(),
                    bad_series.len(),
                    86164.0,
                    ptr::null(),
                    &mut out_output,
                ),
                SidereonStatus::InvalidArgument
            );

            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Sidereal);
            let expected_len = info.payload_len;
            assert!(expected_len > 0);

            // Inspection calls retain error: sidereon_sidereal_repeat_period
            let mut period_s = 0.0;
            assert_eq!(
                sidereon_sidereal_repeat_period(0, &mut period_s),
                SidereonStatus::Ok
            );
            assert!(period_s > 0.0);
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Sidereal);
            assert_eq!(info.payload_len, expected_len);

            // Destructor retains error: sidereon_sidereal_filter_output_free
            sidereon_sidereal_filter_output_free(ptr::null_mut());
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Sidereal);
            assert_eq!(info.payload_len, expected_len);

            // Two-pass payload retrieval: Pass 1 query length
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

            // Slot still retains
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Sidereal);

            // Producer early argument failure (null pointer) clears slot before checks!
            let valid_series = [1.0, 2.0];
            assert_eq!(
                sidereon_sidereal_filter(
                    valid_series.as_ptr(),
                    valid_series.len(),
                    86164.0,
                    ptr::null(),
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
