use super::*;

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
    ffi_boundary("sidereon_sgp4_fit_tle", SidereonStatus::Panic, || {
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
                set_last_error(format!("sidereon_sgp4_fit_tle: {err}"));
                match err {
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
    Ok(sidereon_core::astro::sgp4::FitConfig {
        epoch: sgp4_fit_epoch_from_c(fn_name, config)?,
        fit_bstar: config.fit_bstar,
        bstar_seed: config.bstar_seed,
        use_velocity: config.use_velocity,
        velocity_weight_s: config
            .has_velocity_weight_s
            .then_some(config.velocity_weight_s),
        weights,
        opsmode: tle_ops_mode_from_c(fn_name, config.opsmode)?,
        ftol: config.has_ftol.then_some(config.ftol),
        xtol: config.has_xtol.then_some(config.xtol),
        gtol: config.has_gtol.then_some(config.gtol),
        max_nfev: config.has_max_nfev.then_some(config.max_nfev),
        x_scale: sgp4_x_scale_from_c(fn_name, config)?,
        loss: sgp4_loss_from_c(fn_name, config.loss)?,
        f_scale: config.f_scale,
        metadata: sidereon_core::astro::sgp4::TleMetadata {
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
        },
    })
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
