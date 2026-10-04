use super::*;
use crate::engine_error::{
    engine_error_operation_boundary, record_engine_error, tle_fit_error_value,
    tle_fit_result_value, SidereonEngineErrorFamily,
};

// === Round-2 SGP4 TLE fitting ===============================================

pub const SGP4_FIT_OBJECT_NAME_C_BYTES: usize = 65;

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSgp4FitEpochKind {
    Midpoint = 0,
    First = 1,
    Last = 2,
    Sample = 3,
    Jd = 4,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSgp4Loss {
    Linear = 0,
    SoftL1 = 1,
    Huber = 2,
    Cauchy = 3,
    Arctan = 4,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSgp4XScaleKind {
    None = 0,
    Unit = 1,
    Values = 2,
    Jacobian = 3,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSgp4FitSample {
    pub jd_whole: f64,
    pub jd_fraction: f64,
    pub position_teme_km: [f64; 3],
    pub has_velocity_teme_km_s: bool,
    pub velocity_teme_km_s: [f64; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSgp4FitConfig {
    pub epoch_kind: u32,
    pub epoch_sample_index: usize,
    pub epoch_jd_whole: f64,
    pub epoch_jd_fraction: f64,
    pub fit_bstar: bool,
    pub bstar_seed: f64,
    pub use_velocity: bool,
    pub has_velocity_weight_s: bool,
    pub velocity_weight_s: f64,
    pub weights: *const f64,
    pub weight_count: usize,
    pub opsmode: u32,
    pub has_ftol: bool,
    pub ftol: f64,
    pub has_xtol: bool,
    pub xtol: f64,
    pub has_gtol: bool,
    pub gtol: f64,
    pub has_max_nfev: bool,
    pub max_nfev: usize,
    pub x_scale_kind: u32,
    pub x_scale_values: *const f64,
    pub x_scale_value_count: usize,
    pub loss: u32,
    pub f_scale: f64,
    pub catalog_number: u32,
    pub classification: [c_char; TLE_FIELD_C_BYTES],
    pub international_designator: [c_char; TLE_FIELD_C_BYTES],
    pub element_set_number: i32,
    pub rev_at_epoch: i64,
    pub object_name: [c_char; SGP4_FIT_OBJECT_NAME_C_BYTES],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSgp4FitStatistics {
    pub rms_position_km: f64,
    pub max_position_km: f64,
    pub rms_position_axes_km: [f64; 3],
    pub has_rms_velocity_km_s: bool,
    pub rms_velocity_km_s: f64,
    pub tle_rms_position_km: f64,
    pub status: i32,
    pub nfev: usize,
    pub njev: usize,
    pub cost: f64,
    pub optimality: f64,
    pub bstar_observable: bool,
    pub seed_refine_passes: usize,
}

pub struct SidereonSgp4TleFit {
    pub(crate) inner: sidereon_core::astro::sgp4::TleFit,
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_sgp4_fit_config_init(
    out_config: *mut SidereonSgp4FitConfig,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sgp4_fit_config_init",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_config,
                "sidereon_sgp4_fit_config_init",
                "out_config"
            ));
            *out = SidereonSgp4FitConfig {
                epoch_kind: SidereonSgp4FitEpochKind::Midpoint as u32,
                epoch_sample_index: 0,
                epoch_jd_whole: 0.0,
                epoch_jd_fraction: 0.0,
                fit_bstar: true,
                bstar_seed: 0.0,
                use_velocity: true,
                has_velocity_weight_s: false,
                velocity_weight_s: 0.0,
                weights: ptr::null(),
                weight_count: 0,
                opsmode: SidereonTleOpsMode::Improved as u32,
                has_ftol: false,
                ftol: 0.0,
                has_xtol: false,
                xtol: 0.0,
                has_gtol: false,
                gtol: 0.0,
                has_max_nfev: false,
                max_nfev: 0,
                x_scale_kind: SidereonSgp4XScaleKind::None as u32,
                x_scale_values: ptr::null(),
                x_scale_value_count: 0,
                loss: SidereonSgp4Loss::Linear as u32,
                f_scale: 1.0,
                catalog_number: 0,
                classification: fixed_c_chars::<TLE_FIELD_C_BYTES>("U"),
                international_designator: [0; TLE_FIELD_C_BYTES],
                element_set_number: 999,
                rev_at_epoch: 0,
                object_name: [0; SGP4_FIT_OBJECT_NAME_C_BYTES],
            };
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_sgp4_fit_tle(
    samples: *const SidereonSgp4FitSample,
    sample_count: usize,
    config: *const SidereonSgp4FitConfig,
    out_fit: *mut *mut SidereonSgp4TleFit,
) -> SidereonStatus {
    engine_error_operation_boundary("sidereon_sgp4_fit_tle", SidereonStatus::Panic, || {
        let out_fit = c_try!(require_out(out_fit, "sidereon_sgp4_fit_tle", "out_fit"));
        *out_fit = ptr::null_mut();
        let raw_samples = c_try!(require_slice(
            samples,
            sample_count,
            "sidereon_sgp4_fit_tle",
            "samples"
        ));
        let config = c_try!(require_ref(config, "sidereon_sgp4_fit_tle", "config"));
        let samples: Vec<_> = raw_samples
            .iter()
            .map(|sample| sidereon_core::astro::sgp4::FitSample {
                epoch: sidereon_core::astro::sgp4::JulianDate(sample.jd_whole, sample.jd_fraction),
                position_teme_km: sample.position_teme_km,
                velocity_teme_km_s: sample
                    .has_velocity_teme_km_s
                    .then_some(sample.velocity_teme_km_s),
            })
            .collect();
        let config = c_try!(sgp4_fit_config_from_c("sidereon_sgp4_fit_tle", config));
        match sidereon_core::astro::sgp4::fit_tle(&samples, &config) {
            Ok(inner) => {
                write_boxed_handle(out_fit, SidereonSgp4TleFit { inner });
                SidereonStatus::Ok
            }
            Err(err) => {
                record_engine_error(
                    SidereonEngineErrorFamily::TleFit,
                    "sidereon_sgp4_fit_tle",
                    tle_fit_error_value(&err),
                );
                set_last_error(format!("sidereon_sgp4_fit_tle: {err}"));
                match &err {
                    sidereon_core::astro::sgp4::TleFitError::ArcTooShort { .. }
                    | sidereon_core::astro::sgp4::TleFitError::InvalidInput { .. }
                    | sidereon_core::astro::sgp4::TleFitError::EpochsNotIncreasing { .. }
                    | sidereon_core::astro::sgp4::TleFitError::EpochOutsideArc
                    | sidereon_core::astro::sgp4::TleFitError::MixedVelocityPresence => {
                        SidereonStatus::InvalidArgument
                    }
                    _ => SidereonStatus::Solve,
                }
            }
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_sgp4_tle_fit_statistics(
    fit: *const SidereonSgp4TleFit,
    out_stats: *mut SidereonSgp4FitStatistics,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sgp4_tle_fit_statistics",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_stats,
                "sidereon_sgp4_tle_fit_statistics",
                "out_stats"
            ));
            let fit = c_try!(require_ref(fit, "sidereon_sgp4_tle_fit_statistics", "fit"));
            *out = sgp4_fit_stats_to_c(&fit.inner.stats);
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_sgp4_tle_fit_lines(
    fit: *const SidereonSgp4TleFit,
    out_lines: *mut SidereonTleLines,
) -> SidereonStatus {
    ffi_boundary("sidereon_sgp4_tle_fit_lines", SidereonStatus::Panic, || {
        let out = c_try!(require_out(
            out_lines,
            "sidereon_sgp4_tle_fit_lines",
            "out_lines"
        ));
        let fit = c_try!(require_ref(fit, "sidereon_sgp4_tle_fit_lines", "fit"));
        *out = SidereonTleLines {
            line1: SidereonTleLine {
                bytes: fixed_c_chars(&fit.inner.line1),
            },
            line2: SidereonTleLine {
                bytes: fixed_c_chars(&fit.inner.line2),
            },
        };
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_sgp4_tle_fit_omm(
    fit: *const SidereonSgp4TleFit,
    out_omm: *mut *mut SidereonOmm,
) -> SidereonStatus {
    ffi_boundary("sidereon_sgp4_tle_fit_omm", SidereonStatus::Panic, || {
        let out_omm = c_try!(require_out(out_omm, "sidereon_sgp4_tle_fit_omm", "out_omm"));
        *out_omm = ptr::null_mut();
        let fit = c_try!(require_ref(fit, "sidereon_sgp4_tle_fit_omm", "fit"));
        write_boxed_handle(
            out_omm,
            SidereonOmm {
                inner: fit.inner.omm.clone(),
            },
        );
        SidereonStatus::Ok
    })
}

/// Copy the complete lossless fit result as JSON. The payload includes every
/// ElementSet field, OMM field, generated TLE line, and fit statistic. Query
/// the required byte count with a NULL output and zero capacity, then retry
/// with a caller-owned buffer. A short-buffer retry leaves the fit unchanged.
///
/// Safety: fit must be a live handle. If len is nonzero, out_payload must point
/// to len writable bytes. Both count outputs must be writable.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sgp4_tle_fit_result_payload(
    fit: *const SidereonSgp4TleFit,
    out_payload: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sgp4_tle_fit_result_payload";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        if let Err(status) = init_copy_counts(FN_NAME, out_written, out_required) {
            return status;
        }
        let fit = match require_ref(fit, FN_NAME, "fit") {
            Ok(fit) => fit,
            Err(status) => return status,
        };
        let value = tle_fit_result_value(&fit.inner);
        let payload = match serde_json::to_vec(&value) {
            Ok(payload) => payload,
            Err(error) => {
                set_last_error(format!(
                    "{FN_NAME}: could not serialize fit result: {error}"
                ));
                return SidereonStatus::Panic;
            }
        };
        match copy_prefix_to_c(
            FN_NAME,
            "out_payload",
            &payload,
            out_payload,
            len,
            out_written,
            out_required,
        ) {
            Ok(()) => SidereonStatus::Ok,
            Err(status) => status,
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_sgp4_tle_fit_free(fit: *mut SidereonSgp4TleFit) {
    free_boxed(fit);
}

unsafe fn sgp4_fit_config_from_c(
    fn_name: &str,
    config: &SidereonSgp4FitConfig,
) -> Result<sidereon_core::astro::sgp4::FitConfig, SidereonStatus> {
    let classification =
        fixed_c_array_to_string(fn_name, "classification", &config.classification)?;
    let international_designator = fixed_c_array_to_string(
        fn_name,
        "international_designator",
        &config.international_designator,
    )?;
    let object_name = fixed_c_array_to_string(fn_name, "object_name", &config.object_name)?;
    let weights = if config.weight_count == 0 {
        None
    } else {
        Some(require_slice(config.weights, config.weight_count, fn_name, "weights")?.to_vec())
    };
    let mut o = sidereon_core::astro::sgp4::FitConfig::default();
    o.epoch = sgp4_fit_epoch_from_c(fn_name, config)?;
    o.fit_bstar = config.fit_bstar;
    o.bstar_seed = config.bstar_seed;
    o.use_velocity = config.use_velocity;
    o.velocity_weight_s = config
        .has_velocity_weight_s
        .then_some(config.velocity_weight_s);
    o.weights = weights;
    o.opsmode = tle_ops_mode_from_c(fn_name, config.opsmode)?;
    o.ftol = config.has_ftol.then_some(config.ftol);
    o.xtol = config.has_xtol.then_some(config.xtol);
    o.gtol = config.has_gtol.then_some(config.gtol);
    o.max_nfev = config.has_max_nfev.then_some(config.max_nfev);
    o.x_scale = sgp4_x_scale_from_c(fn_name, config)?;
    o.loss = sgp4_loss_from_c(fn_name, config.loss)?;
    o.f_scale = config.f_scale;
    o.metadata = sidereon_core::astro::sgp4::TleMetadata {
        catalog_number: config.catalog_number,
        classification: if classification.is_empty() {
            "U".to_string()
        } else {
            classification
        },
        international_designator,
        element_set_number: config.element_set_number,
        rev_at_epoch: config.rev_at_epoch,
        object_name,
    };
    Ok(o)
}

fn sgp4_fit_stats_to_c(
    stats: &sidereon_core::astro::sgp4::FitStatistics,
) -> SidereonSgp4FitStatistics {
    SidereonSgp4FitStatistics {
        rms_position_km: stats.rms_position_km,
        max_position_km: stats.max_position_km,
        rms_position_axes_km: stats.rms_position_axes_km,
        has_rms_velocity_km_s: stats.rms_velocity_km_s.is_some(),
        rms_velocity_km_s: stats.rms_velocity_km_s.unwrap_or(0.0),
        tle_rms_position_km: stats.tle_rms_position_km,
        status: stats.status,
        nfev: stats.nfev,
        njev: stats.njev,
        cost: stats.cost,
        optimality: stats.optimality,
        bstar_observable: stats.bstar_observable,
        seed_refine_passes: stats.seed_refine_passes,
    }
}

fn sgp4_loss_from_c(
    fn_name: &str,
    loss: u32,
) -> Result<sidereon_core::astro::sgp4::Loss, SidereonStatus> {
    use sidereon_core::astro::sgp4::Loss as L;
    match loss {
        x if x == SidereonSgp4Loss::Linear as u32 => Ok(L::Linear),
        x if x == SidereonSgp4Loss::SoftL1 as u32 => Ok(L::SoftL1),
        x if x == SidereonSgp4Loss::Huber as u32 => Ok(L::Huber),
        x if x == SidereonSgp4Loss::Cauchy as u32 => Ok(L::Cauchy),
        x if x == SidereonSgp4Loss::Arctan as u32 => Ok(L::Arctan),
        _ => {
            set_last_error(format!("{fn_name}: invalid loss {loss}"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

unsafe fn sgp4_x_scale_from_c(
    fn_name: &str,
    config: &SidereonSgp4FitConfig,
) -> Result<Option<sidereon_core::astro::sgp4::XScale>, SidereonStatus> {
    use sidereon_core::astro::sgp4::XScale as X;
    match config.x_scale_kind {
        x if x == SidereonSgp4XScaleKind::None as u32 => Ok(None),
        x if x == SidereonSgp4XScaleKind::Unit as u32 => Ok(Some(X::Unit)),
        x if x == SidereonSgp4XScaleKind::Jacobian as u32 => Ok(Some(X::Jac)),
        x if x == SidereonSgp4XScaleKind::Values as u32 => {
            let values = require_slice(
                config.x_scale_values,
                config.x_scale_value_count,
                fn_name,
                "x_scale_values",
            )?;
            Ok(Some(X::Values(values.to_vec())))
        }
        _ => {
            set_last_error(format!(
                "{fn_name}: invalid x_scale_kind {}",
                config.x_scale_kind
            ));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn sgp4_fit_epoch_from_c(
    fn_name: &str,
    config: &SidereonSgp4FitConfig,
) -> Result<sidereon_core::astro::sgp4::FitEpoch, SidereonStatus> {
    use sidereon_core::astro::sgp4::{FitEpoch, JulianDate};
    match config.epoch_kind {
        x if x == SidereonSgp4FitEpochKind::Midpoint as u32 => Ok(FitEpoch::Midpoint),
        x if x == SidereonSgp4FitEpochKind::First as u32 => Ok(FitEpoch::First),
        x if x == SidereonSgp4FitEpochKind::Last as u32 => Ok(FitEpoch::Last),
        x if x == SidereonSgp4FitEpochKind::Sample as u32 => {
            Ok(FitEpoch::Sample(config.epoch_sample_index))
        }
        x if x == SidereonSgp4FitEpochKind::Jd as u32 => Ok(FitEpoch::Jd(JulianDate(
            config.epoch_jd_whole,
            config.epoch_jd_fraction,
        ))),
        _ => {
            set_last_error(format!(
                "{fn_name}: invalid epoch_kind {}",
                config.epoch_kind
            ));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_error::{snapshot_engine_error_for_test, SidereonEngineErrorFamily};
    use serde_json::{json, Value};

    const ISS_LINE1: &str = "1 25544U 98067A   26168.18949189  .00009113  00000+0  17172-3 0  9996";
    const ISS_LINE2: &str = "2 25544  51.6332 300.0813 0004737 195.1146 164.9702 15.49273435571752";

    fn existing_fit_samples() -> Vec<SidereonSgp4FitSample> {
        use sidereon_core::astro::sgp4::{MinutesSinceEpoch, Satellite};
        let satellite =
            Satellite::from_tle(ISS_LINE1, ISS_LINE2).expect("existing ISS fit fixture");
        let epoch = satellite.epoch_jd();
        [-10.0, 0.0, 10.0]
            .into_iter()
            .map(|minutes| {
                let state = satellite
                    .propagate(MinutesSinceEpoch(minutes))
                    .expect("fixture sample propagates");
                SidereonSgp4FitSample {
                    jd_whole: epoch.0,
                    jd_fraction: epoch.1 + minutes / 1440.0,
                    position_teme_km: state.position,
                    has_velocity_teme_km_s: true,
                    velocity_teme_km_s: state.velocity,
                }
            })
            .collect()
    }

    unsafe fn valid_config(max_nfev: Option<usize>) -> SidereonSgp4FitConfig {
        let mut out = std::mem::MaybeUninit::<SidereonSgp4FitConfig>::uninit();
        assert_eq!(
            sidereon_sgp4_fit_config_init(out.as_mut_ptr()),
            SidereonStatus::Ok
        );
        let mut config = out.assume_init();
        config.has_max_nfev = max_nfev.is_some();
        config.max_nfev = max_nfev.unwrap_or(0);
        config.catalog_number = 25544;
        config.element_set_number = 999;
        config.rev_at_epoch = 57175;
        config
    }

    unsafe fn last_legacy_error() -> String {
        let required = crate::sidereon_last_error_message(ptr::null_mut(), 0);
        let mut bytes = vec![0 as c_char; required + 1];
        assert_eq!(
            crate::sidereon_last_error_message(bytes.as_mut_ptr(), bytes.len()),
            required
        );
        std::ffi::CStr::from_ptr(bytes.as_ptr())
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn fit_tle_records_arc_and_best_effort_errors_with_lifecycle_boundaries() {
        unsafe {
            let samples = existing_fit_samples();
            let mut config = valid_config(None);
            let mut fit = ptr::null_mut();

            assert_eq!(
                sidereon_sgp4_fit_tle(samples.as_ptr(), 2, &config, &mut fit),
                SidereonStatus::InvalidArgument
            );
            assert!(fit.is_null());
            assert_eq!(
                last_legacy_error(),
                "sidereon_sgp4_fit_tle: fit arc has 2 samples; need at least 3"
            );
            let (info, payload) =
                snapshot_engine_error_for_test().expect("typed arc-too-short error");
            assert_eq!(info.family, SidereonEngineErrorFamily::TleFit);
            let json: Value = serde_json::from_str(&payload).expect("error payload JSON");
            assert_eq!(json["family"], "tle_fit");
            assert_eq!(json["operation"], "sidereon_sgp4_fit_tle");
            assert_eq!(json["error"]["kind"], "arc_too_short");
            assert_eq!(json["error"]["fields"]["samples"], 2);
            assert_eq!(json["error"]["fields"]["needed"], 3);

            assert_eq!(
                sidereon_sgp4_fit_tle(samples.as_ptr(), 2, &config, &mut fit),
                SidereonStatus::InvalidArgument
            );
            assert!(snapshot_engine_error_for_test().is_some());
            assert_eq!(
                sidereon_sgp4_fit_tle(samples.as_ptr(), samples.len(), &config, ptr::null_mut()),
                SidereonStatus::NullPointer
            );
            assert!(snapshot_engine_error_for_test().is_none());

            config = valid_config(Some(80));
            assert_eq!(
                sidereon_sgp4_fit_tle(samples.as_ptr(), samples.len(), &config, &mut fit),
                SidereonStatus::Ok
            );
            assert!(!fit.is_null());
            assert!(snapshot_engine_error_for_test().is_none());

            let mut written = usize::MAX;
            let mut required = 0;
            assert_eq!(
                sidereon_sgp4_tle_fit_result_payload(
                    fit,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, 0);
            assert!(required > 0);
            let mut short = vec![0xA5; required - 1];
            assert_eq!(
                sidereon_sgp4_tle_fit_result_payload(
                    fit,
                    short.as_mut_ptr(),
                    short.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::InvalidArgument
            );
            assert_eq!(written, 0);
            assert_eq!(required, short.len() + 1);
            assert!(short.iter().all(|byte| *byte == 0xA5));

            let mut owned_full_fit_payload = vec![0; required];
            assert_eq!(
                sidereon_sgp4_tle_fit_result_payload(
                    fit,
                    owned_full_fit_payload.as_mut_ptr(),
                    owned_full_fit_payload.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, required);
            let full_fit: Value =
                serde_json::from_slice(&owned_full_fit_payload).expect("full fit JSON");
            for key in [
                "epoch",
                "bstar",
                "mean_motion_dot",
                "mean_motion_double_dot",
                "eccentricity",
                "argument_of_perigee_deg",
                "inclination_deg",
                "mean_anomaly_deg",
                "mean_motion_rev_per_day",
                "right_ascension_deg",
                "catalog_number",
                "omm_epoch_days",
            ] {
                assert!(
                    full_fit["elements"].get(key).is_some(),
                    "missing element {key}"
                );
            }
            assert!(full_fit["omm"]["exact_sgp4_epoch"].is_array());
            assert!(full_fit["omm"].get("quantize_tle_derived_fields").is_some());

            let mut null_output_written = usize::MAX;
            let mut null_output_required = 0;
            assert_eq!(
                sidereon_sgp4_tle_fit_result_payload(
                    fit,
                    ptr::null_mut(),
                    1,
                    &mut null_output_written,
                    &mut null_output_required,
                ),
                SidereonStatus::NullPointer
            );
            assert_eq!(null_output_written, 0);
            assert_eq!(null_output_required, owned_full_fit_payload.len());

            written = usize::MAX;
            required = usize::MAX;
            assert_eq!(
                sidereon_sgp4_tle_fit_result_payload(
                    ptr::null(),
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::NullPointer
            );
            assert_eq!(written, 0);
            assert_eq!(required, 0);

            let mut lines = std::mem::MaybeUninit::<SidereonTleLines>::uninit();
            config = valid_config(Some(1));
            let mut failed_fit = ptr::null_mut();
            assert_eq!(
                sidereon_sgp4_fit_tle(samples.as_ptr(), samples.len(), &config, &mut failed_fit,),
                SidereonStatus::Solve
            );
            assert!(failed_fit.is_null());
            assert_eq!(
                last_legacy_error(),
                "sidereon_sgp4_fit_tle: evaluation budget exhausted before convergence (best-effort result attached)"
            );
            let (best_effort_info, best_effort_payload) =
                snapshot_engine_error_for_test().expect("typed best-effort fit error");
            assert_eq!(best_effort_info.family, SidereonEngineErrorFamily::TleFit);
            let detail: Value =
                serde_json::from_str(&best_effort_payload).expect("best-effort JSON");
            assert_eq!(detail["error"]["kind"], "did_not_converge");
            let best = &detail["error"]["fields"]["best_effort_fit"];
            assert_eq!(
                best["line1"]
                    .as_str()
                    .map(|line| line.starts_with("1 25544")),
                Some(true)
            );
            assert_eq!(
                best["line2"]
                    .as_str()
                    .map(|line| line.starts_with("2 25544")),
                Some(true)
            );
            for key in [
                "epoch",
                "bstar",
                "mean_motion_dot",
                "mean_motion_double_dot",
                "eccentricity",
                "argument_of_perigee_deg",
                "inclination_deg",
                "mean_anomaly_deg",
                "mean_motion_rev_per_day",
                "right_ascension_deg",
                "catalog_number",
                "omm_epoch_days",
            ] {
                assert!(best["elements"].get(key).is_some(), "missing element {key}");
            }
            assert_eq!(best["elements"]["omm_epoch_days"], Value::Null);
            for key in [
                "ccsds_omm_vers",
                "classification",
                "creation_date",
                "originator",
                "message_id",
                "object_name",
                "object_id",
                "center_name",
                "ref_frame",
                "ref_frame_epoch",
                "time_system",
                "mean_element_theory",
                "epoch",
                "mean_motion",
                "semi_major_axis_km",
                "eccentricity",
                "inclination_deg",
                "ra_of_asc_node_deg",
                "arg_of_pericenter_deg",
                "mean_anomaly_deg",
                "gm_km3_s2",
                "spacecraft",
                "ephemeris_type",
                "classification_type",
                "norad_cat_id",
                "element_set_no",
                "rev_at_epoch",
                "bstar",
                "bterm_m2_kg",
                "mean_motion_dot",
                "mean_motion_ddot",
                "agom_m2_kg",
                "covariance",
                "user_defined",
                "comments",
                "exact_sgp4_epoch",
                "quantize_tle_derived_fields",
            ] {
                assert!(best["omm"].get(key).is_some(), "missing OMM field {key}");
            }
            assert_eq!(best["omm"]["spacecraft"], Value::Null);
            assert_eq!(best["omm"]["covariance"], Value::Null);
            assert_eq!(best["omm"]["user_defined"], json!([]));
            assert_eq!(best["omm"]["comments"]["header"], json!([]));
            assert_eq!(
                best["omm"]["exact_sgp4_epoch"]
                    .as_array()
                    .map(|epoch| epoch.len()),
                Some(2)
            );
            assert_eq!(best["omm"]["quantize_tle_derived_fields"], false);
            assert_eq!(best["stats"]["status"], 0);
            assert_eq!(best["stats"]["nfev"], 1);
            assert_eq!(
                best["stats"]["rms_position_km"]["bits_hex"]
                    .as_str()
                    .map(str::len),
                Some(16)
            );

            assert_eq!(
                sidereon_sgp4_tle_fit_lines(fit, lines.as_mut_ptr()),
                SidereonStatus::Ok
            );
            assert_eq!(
                snapshot_engine_error_for_test()
                    .expect("getter retains typed detail")
                    .1,
                best_effort_payload
            );
            sidereon_sgp4_tle_fit_free(fit);
            let retained_after_free: Value =
                serde_json::from_slice(&owned_full_fit_payload).expect("retained full fit JSON");
            assert_eq!(retained_after_free, full_fit);
            assert_eq!(
                snapshot_engine_error_for_test()
                    .expect("free retains typed detail")
                    .1,
                best_effort_payload
            );

            config = valid_config(Some(80));
            assert_eq!(
                sidereon_sgp4_fit_tle(samples.as_ptr(), samples.len(), &config, &mut fit),
                SidereonStatus::Ok
            );
            assert!(snapshot_engine_error_for_test().is_none());
            sidereon_sgp4_tle_fit_free(fit);
        }
    }
}
