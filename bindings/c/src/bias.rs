use super::*;

// --- GNSS code and phase bias products ---------------------------------------

pub const MAX_BIAS_OBS_BYTES: usize = 16;

pub const BIAS_OBS_C_BYTES: usize = MAX_BIAS_OBS_BYTES + 1;

pub const MAX_BIAS_TEXT_BYTES: usize = 64;

pub const BIAS_TEXT_C_BYTES: usize = MAX_BIAS_TEXT_BYTES + 1;

pub struct SidereonBiasSet {
    pub(crate) inner: BiasSet,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonBiasKind {
    Osb = 0,
    Dsb = 1,
    Isb = 2,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonBiasMode {
    Absolute = 0,
    Relative = 1,
    Unspecified = 2,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonBiasTargetKind {
    System = 0,
    Satellite = 1,
    Receiver = 2,
    SatelliteReceiver = 3,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonBiasEpoch {
    pub year: i32,
    pub day_of_year: u16,
    pub second_of_day: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonBiasRecord {
    pub kind: SidereonBiasKind,
    pub target_kind: SidereonBiasTargetKind,
    pub system: SidereonGnssSystem,
    pub has_sat_id: bool,
    pub sat_id: SidereonSatelliteToken,
    pub station: [c_char; BIAS_TEXT_C_BYTES],
    pub svn: [c_char; BIAS_TEXT_C_BYTES],
    pub obs1: [c_char; BIAS_OBS_C_BYTES],
    pub has_obs2: bool,
    pub obs2: [c_char; BIAS_OBS_C_BYTES],
    pub has_valid_from: bool,
    pub valid_from: SidereonBiasEpoch,
    pub has_valid_until: bool,
    pub valid_until: SidereonBiasEpoch,
    pub value: f64,
    pub has_sigma: bool,
    pub sigma: f64,
    pub has_slope: bool,
    pub slope: f64,
    pub has_slope_sigma: bool,
    pub slope_sigma: f64,
    pub is_phase: bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonCodeDcbOptions {
    pub obs1: *const c_char,
    pub obs2: *const c_char,
    pub year: i32,
    pub month: u8,
    pub time_scale: u32,
    pub has_receiver_system: bool,
    pub receiver_system: u32,
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_bias_sinex_parse(
    bytes: *const u8,
    len: usize,
    out: *mut *mut SidereonBiasSet,
) -> SidereonStatus {
    ffi_boundary("sidereon_bias_sinex_parse", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_bias_sinex_parse", "out"));
        *out = ptr::null_mut();
        let data = c_try!(require_slice(
            bytes,
            len,
            "sidereon_bias_sinex_parse",
            "bytes"
        ));
        let inner = c_try!(guard(SidereonStatus::InvalidArgument, || {
            sidereon::parse_bias_sinex(data)
        }));
        write_bias_handle(out, inner);
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_bias_sinex_parse_lossy(
    bytes: *const u8,
    len: usize,
    out: *mut *mut SidereonBiasSet,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_bias_sinex_parse_lossy",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_bias_sinex_parse_lossy", "out"));
            *out = ptr::null_mut();
            let data = c_try!(require_slice(
                bytes,
                len,
                "sidereon_bias_sinex_parse_lossy",
                "bytes"
            ));
            let parsed = c_try!(guard(SidereonStatus::InvalidArgument, || {
                sidereon::parse_bias_sinex_lossy(data)
            }));
            write_bias_handle(out, parsed.value);
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_bias_sinex_load(
    path: *const c_char,
    out: *mut *mut SidereonBiasSet,
) -> SidereonStatus {
    ffi_boundary("sidereon_bias_sinex_load", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_bias_sinex_load", "out"));
        *out = ptr::null_mut();
        let path = c_try!(parse_c_string("sidereon_bias_sinex_load", "path", path));
        let inner = c_try!(guard(SidereonStatus::InvalidArgument, || {
            sidereon::load_bias_sinex(&path)
        }));
        write_bias_handle(out, inner);
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_bias_sinex_load_lossy(
    path: *const c_char,
    out: *mut *mut SidereonBiasSet,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_bias_sinex_load_lossy",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_bias_sinex_load_lossy", "out"));
            *out = ptr::null_mut();
            let path = c_try!(parse_c_string(
                "sidereon_bias_sinex_load_lossy",
                "path",
                path
            ));
            let parsed = c_try!(guard(SidereonStatus::InvalidArgument, || {
                sidereon::load_bias_sinex_lossy(&path)
            }));
            write_bias_handle(out, parsed.value);
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_code_dcb_parse(
    bytes: *const u8,
    len: usize,
    options: *const SidereonCodeDcbOptions,
    out: *mut *mut SidereonBiasSet,
) -> SidereonStatus {
    ffi_boundary("sidereon_code_dcb_parse", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_code_dcb_parse", "out"));
        *out = ptr::null_mut();
        let data = c_try!(require_slice(
            bytes,
            len,
            "sidereon_code_dcb_parse",
            "bytes"
        ));
        let options = c_try!(code_dcb_options_from_c("sidereon_code_dcb_parse", options));
        let inner = c_try!(guard(SidereonStatus::InvalidArgument, || {
            sidereon::parse_code_dcb(data, options)
        }));
        write_bias_handle(out, inner);
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_code_dcb_parse_lossy(
    bytes: *const u8,
    len: usize,
    options: *const SidereonCodeDcbOptions,
    out: *mut *mut SidereonBiasSet,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_code_dcb_parse_lossy",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_code_dcb_parse_lossy", "out"));
            *out = ptr::null_mut();
            let data = c_try!(require_slice(
                bytes,
                len,
                "sidereon_code_dcb_parse_lossy",
                "bytes"
            ));
            let options = c_try!(code_dcb_options_from_c(
                "sidereon_code_dcb_parse_lossy",
                options
            ));
            let parsed = c_try!(guard(SidereonStatus::InvalidArgument, || {
                sidereon::parse_code_dcb_lossy(data, options)
            }));
            write_bias_handle(out, parsed.value);
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_code_dcb_load(
    path: *const c_char,
    options: *const SidereonCodeDcbOptions,
    out: *mut *mut SidereonBiasSet,
) -> SidereonStatus {
    ffi_boundary("sidereon_code_dcb_load", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_code_dcb_load", "out"));
        *out = ptr::null_mut();
        let path = c_try!(parse_c_string("sidereon_code_dcb_load", "path", path));
        let options = c_try!(code_dcb_options_from_c("sidereon_code_dcb_load", options));
        let inner = c_try!(guard(SidereonStatus::InvalidArgument, || {
            sidereon::load_code_dcb(&path, options)
        }));
        write_bias_handle(out, inner);
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_code_dcb_load_lossy(
    path: *const c_char,
    options: *const SidereonCodeDcbOptions,
    out: *mut *mut SidereonBiasSet,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_code_dcb_load_lossy",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_code_dcb_load_lossy", "out"));
            *out = ptr::null_mut();
            let path = c_try!(parse_c_string("sidereon_code_dcb_load_lossy", "path", path));
            let options = c_try!(code_dcb_options_from_c(
                "sidereon_code_dcb_load_lossy",
                options
            ));
            let parsed = c_try!(guard(SidereonStatus::InvalidArgument, || {
                sidereon::load_code_dcb_lossy(&path, options)
            }));
            write_bias_handle(out, parsed.value);
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_bias_set_record_count(
    set: *const SidereonBiasSet,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_bias_set_record_count",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_count,
                "sidereon_bias_set_record_count",
                "out_count"
            ));
            *out = 0;
            let set = c_try!(require_ref(set, "sidereon_bias_set_record_count", "set"));
            *out = set.inner.records().len();
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_bias_set_skipped_record_count(
    set: *const SidereonBiasSet,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_bias_set_skipped_record_count",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_count,
                "sidereon_bias_set_skipped_record_count",
                "out_count"
            ));
            *out = 0;
            let set = c_try!(require_ref(
                set,
                "sidereon_bias_set_skipped_record_count",
                "set"
            ));
            *out = set.inner.skipped_records();
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_bias_set_warning_count(
    set: *const SidereonBiasSet,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_bias_set_warning_count",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_count,
                "sidereon_bias_set_warning_count",
                "out_count"
            ));
            *out = 0;
            let set = c_try!(require_ref(set, "sidereon_bias_set_warning_count", "set"));
            *out = set.inner.diagnostics().warnings.len();
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_bias_set_mode(
    set: *const SidereonBiasSet,
    out_mode: *mut SidereonBiasMode,
    out_time_scale: *mut u32,
) -> SidereonStatus {
    ffi_boundary("sidereon_bias_set_mode", SidereonStatus::Panic, || {
        let out_mode = c_try!(require_out(out_mode, "sidereon_bias_set_mode", "out_mode"));
        let out_time_scale = c_try!(require_out(
            out_time_scale,
            "sidereon_bias_set_mode",
            "out_time_scale"
        ));
        let set = c_try!(require_ref(set, "sidereon_bias_set_mode", "set"));
        *out_mode = bias_mode_to_c(set.inner.mode);
        *out_time_scale = time_scale_to_c_code(set.inner.time_scale);
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_bias_set_record(
    set: *const SidereonBiasSet,
    index: usize,
    out_record: *mut SidereonBiasRecord,
) -> SidereonStatus {
    ffi_boundary("sidereon_bias_set_record", SidereonStatus::Panic, || {
        let out = c_try!(require_out(
            out_record,
            "sidereon_bias_set_record",
            "out_record"
        ));
        let set = c_try!(require_ref(set, "sidereon_bias_set_record", "set"));
        let Some(record) = set.inner.records().get(index) else {
            set_last_error(format!(
                "sidereon_bias_set_record: index {index} out of range ({} records)",
                set.inner.records().len()
            ));
            return SidereonStatus::InvalidArgument;
        };
        *out = bias_record_to_c(record);
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_bias_set_code_osb_seconds(
    set: *const SidereonBiasSet,
    sat_id: *const c_char,
    obs: *const c_char,
    epoch: SidereonBiasEpoch,
    out_present: *mut bool,
    out_seconds: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_bias_set_code_osb_seconds",
        SidereonStatus::Panic,
        || {
            bias_lookup_sat_obs(
                "sidereon_bias_set_code_osb_seconds",
                set,
                sat_id,
                obs,
                epoch,
                out_present,
                out_seconds,
                BiasSet::code_osb_seconds,
            )
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_bias_set_phase_osb_cycles(
    set: *const SidereonBiasSet,
    sat_id: *const c_char,
    obs: *const c_char,
    epoch: SidereonBiasEpoch,
    out_present: *mut bool,
    out_cycles: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_bias_set_phase_osb_cycles",
        SidereonStatus::Panic,
        || {
            bias_lookup_sat_obs(
                "sidereon_bias_set_phase_osb_cycles",
                set,
                sat_id,
                obs,
                epoch,
                out_present,
                out_cycles,
                BiasSet::phase_osb_cycles,
            )
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_bias_set_code_dsb_seconds(
    set: *const SidereonBiasSet,
    sat_id: *const c_char,
    obs1: *const c_char,
    obs2: *const c_char,
    epoch: SidereonBiasEpoch,
    out_present: *mut bool,
    out_seconds: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_bias_set_code_dsb_seconds",
        SidereonStatus::Panic,
        || {
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_bias_set_code_dsb_seconds",
                "out_present"
            ));
            *out_present = false;
            let out_seconds = c_try!(require_out(
                out_seconds,
                "sidereon_bias_set_code_dsb_seconds",
                "out_seconds"
            ));
            *out_seconds = 0.0;
            let set = c_try!(require_ref(
                set,
                "sidereon_bias_set_code_dsb_seconds",
                "set"
            ));
            let sat = c_try!(parse_satellite_token(
                "sidereon_bias_set_code_dsb_seconds",
                sat_id
            ));
            let obs1 = c_try!(parse_bounded_c_string(
                "sidereon_bias_set_code_dsb_seconds",
                "obs1",
                obs1,
                MAX_BIAS_OBS_BYTES
            ));
            let obs2 = c_try!(parse_bounded_c_string(
                "sidereon_bias_set_code_dsb_seconds",
                "obs2",
                obs2,
                MAX_BIAS_OBS_BYTES
            ));
            let instant = c_try!(bias_epoch_to_instant(
                "sidereon_bias_set_code_dsb_seconds",
                &set.inner,
                epoch
            ));
            if let Some(value) = set.inner.code_dsb_seconds(sat, &obs1, &obs2, instant) {
                *out_present = true;
                *out_seconds = value;
            }
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_bias_set_free(set: *mut SidereonBiasSet) {
    free_boxed(set);
}

fn bias_mode_to_c(mode: BiasMode) -> SidereonBiasMode {
    match mode {
        BiasMode::Absolute => SidereonBiasMode::Absolute,
        BiasMode::Relative => SidereonBiasMode::Relative,
        BiasMode::Unspecified => SidereonBiasMode::Unspecified,
    }
}

fn bias_record_to_c(record: &BiasRecord) -> SidereonBiasRecord {
    let (target_kind, system, sat, station) = match &record.target {
        BiasTarget::System(system) => {
            (SidereonBiasTargetKind::System, *system, None, String::new())
        }
        BiasTarget::Satellite(sat) => (
            SidereonBiasTargetKind::Satellite,
            sat.system,
            Some(*sat),
            String::new(),
        ),
        BiasTarget::Receiver { system, station } => (
            SidereonBiasTargetKind::Receiver,
            *system,
            None,
            station.clone(),
        ),
        BiasTarget::SatelliteReceiver { sat, station } => (
            SidereonBiasTargetKind::SatelliteReceiver,
            sat.system,
            Some(*sat),
            station.clone(),
        ),
    };
    SidereonBiasRecord {
        kind: bias_kind_to_c(record.kind),
        target_kind,
        system: gnss_system_to_c(system),
        has_sat_id: sat.is_some(),
        sat_id: sat
            .map(satellite_token)
            .unwrap_or_else(|| satellite_token_from_text("")),
        station: fixed_c_chars(&station),
        svn: fixed_c_chars(record.svn.as_deref().unwrap_or("")),
        obs1: fixed_c_chars(&record.obs1),
        has_obs2: record.obs2.is_some(),
        obs2: fixed_c_chars(record.obs2.as_deref().unwrap_or("")),
        has_valid_from: record.valid_from.is_some(),
        valid_from: record
            .valid_from
            .map(bias_epoch_to_c)
            .unwrap_or_else(empty_bias_epoch),
        has_valid_until: record.valid_until.is_some(),
        valid_until: record
            .valid_until
            .map(bias_epoch_to_c)
            .unwrap_or_else(empty_bias_epoch),
        value: record.value,
        has_sigma: record.sigma.is_some(),
        sigma: record.sigma.unwrap_or(0.0),
        has_slope: record.slope.is_some(),
        slope: record.slope.unwrap_or(0.0),
        has_slope_sigma: record.slope_sigma.is_some(),
        slope_sigma: record.slope_sigma.unwrap_or(0.0),
        is_phase: record.is_phase,
    }
}

unsafe fn code_dcb_options_from_c(
    fn_name: &str,
    options: *const SidereonCodeDcbOptions,
) -> Result<Option<CodeDcbOptions>, SidereonStatus> {
    let Some(options) = options.as_ref() else {
        return Ok(None);
    };
    let pair = (
        parse_bounded_c_string(fn_name, "options.obs1", options.obs1, MAX_BIAS_OBS_BYTES)?,
        parse_bounded_c_string(fn_name, "options.obs2", options.obs2, MAX_BIAS_OBS_BYTES)?,
    );
    let time_scale = time_scale_from_c_code(fn_name, "options.time_scale", options.time_scale)?;
    let receiver_system = if options.has_receiver_system {
        Some(gnss_system_from_c_code(
            fn_name,
            "options.receiver_system",
            options.receiver_system,
        )?)
    } else {
        None
    };
    Ok(Some(CodeDcbOptions {
        pair,
        year: options.year,
        month: options.month,
        time_scale,
        receiver_system,
    }))
}

fn write_bias_handle(out: &mut *mut SidereonBiasSet, inner: BiasSet) {
    write_boxed_handle(out, SidereonBiasSet { inner });
}

#[allow(clippy::too_many_arguments)]
unsafe fn bias_lookup_sat_obs(
    fn_name: &str,
    set: *const SidereonBiasSet,
    sat_id: *const c_char,
    obs: *const c_char,
    epoch: SidereonBiasEpoch,
    out_present: *mut bool,
    out_value: *mut f64,
    lookup: impl FnOnce(&BiasSet, GnssSatelliteId, &str, Instant) -> Option<f64>,
) -> SidereonStatus {
    let out_present = c_try!(require_out(out_present, fn_name, "out_present"));
    *out_present = false;
    let out_value = c_try!(require_out(out_value, fn_name, "out_value"));
    *out_value = 0.0;
    let set = c_try!(require_ref(set, fn_name, "set"));
    let sat = c_try!(parse_satellite_token(fn_name, sat_id));
    let obs = c_try!(parse_bounded_c_string(
        fn_name,
        "obs",
        obs,
        MAX_BIAS_OBS_BYTES
    ));
    let instant = c_try!(bias_epoch_to_instant(fn_name, &set.inner, epoch));
    if let Some(value) = lookup(&set.inner, sat, &obs, instant) {
        *out_present = true;
        *out_value = value;
    }
    SidereonStatus::Ok
}

fn bias_kind_to_c(kind: BiasKind) -> SidereonBiasKind {
    match kind {
        BiasKind::Osb => SidereonBiasKind::Osb,
        BiasKind::Dsb => SidereonBiasKind::Dsb,
        BiasKind::Isb => SidereonBiasKind::Isb,
    }
}

fn bias_epoch_to_c(epoch: BiasEpoch) -> SidereonBiasEpoch {
    SidereonBiasEpoch {
        year: epoch.year,
        day_of_year: epoch.day_of_year,
        second_of_day: epoch.second_of_day,
    }
}

fn empty_bias_epoch() -> SidereonBiasEpoch {
    SidereonBiasEpoch {
        year: 0,
        day_of_year: 0,
        second_of_day: 0,
    }
}

fn bias_epoch_to_instant(
    fn_name: &str,
    set: &BiasSet,
    epoch: SidereonBiasEpoch,
) -> Result<Instant, SidereonStatus> {
    let epoch = bias_epoch_from_c(fn_name, epoch)?;
    bias_epoch_instant(epoch, set.time_scale).map_err(|err| {
        set_last_error(format!("{fn_name}: invalid bias epoch: {err}"));
        SidereonStatus::InvalidArgument
    })
}
