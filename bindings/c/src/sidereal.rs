use super::*;

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
    ffi_boundary(
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
    ffi_boundary("sidereon_sidereal_filter", SidereonStatus::Panic, || {
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
    ffi_boundary(
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
                    return map_sidereal_error("sidereon_sidereal_periodicity_strength", err)
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
    Ok(sidereon_core::sidereal::SiderealFilterOptions {
        sample_interval: duration_from_seconds(
            fn_name,
            "options.sample_interval_s",
            options.sample_interval_s,
        )?,
        prior_periods: options.prior_periods,
        min_coverage: options.min_coverage,
        template_method: sidereal_template_method_from_c(fn_name, options)?,
    })
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
        set_last_error(format!("{fn_name}: invalid {arg_name}: {err}"));
        SidereonStatus::InvalidArgument
    })
}

fn map_sidereal_error(
    fn_name: &str,
    err: sidereon_core::sidereal::SiderealFilterError,
) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}
