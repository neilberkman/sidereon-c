use super::*;

/// A parsed RINEX 3 observation product. Create with sidereon_rinex_obs_parse and
/// release with sidereon_rinex_obs_free.
pub struct SidereonRinexObs {
    pub(crate) inner: RinexObs,
}

pub const RINEX_OBS_CODE_C_BYTES: usize = 9;

pub const RINEX_OBS_MARKER_C_BYTES: usize = 65;

/// RINEX observation kind inferred from the observation-code leading letter.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonRinexObsKind {
    /// Code pseudorange.
    Pseudorange = 0,
    /// Carrier phase.
    CarrierPhase = 1,
    /// Doppler.
    Doppler = 2,
    /// Signal strength.
    SignalStrength = 3,
    /// Unknown or unsupported leading code letter.
    Unknown = 4,
}

/// Parsed RINEX observation header summary.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRinexObsHeader {
    /// Full RINEX version.
    pub version: f64,
    /// Whether approx_position_m is present.
    pub has_approx_position_m: bool,
    /// Surveyed a-priori receiver position, ECEF meters.
    pub approx_position_m: [f64; 3],
    /// Whether antenna_delta_hen_m is present.
    pub has_antenna_delta_hen_m: bool,
    /// Antenna offset in RINEX height/east/north convention, meters.
    pub antenna_delta_hen_m: [f64; 3],
    /// Whether interval_s is present.
    pub has_interval_s: bool,
    /// Nominal epoch spacing, seconds.
    pub interval_s: f64,
    /// Whether time_of_first_obs is present.
    pub has_time_of_first_obs: bool,
    /// First observation epoch.
    pub time_of_first_obs: SidereonCalendarEpoch,
    /// Time scale of time_of_first_obs as SidereonTimeScale.
    pub time_of_first_obs_scale: u32,
    /// Number of per-system observation-code rows.
    pub obs_code_count: usize,
    /// Number of phase-shift header rows.
    pub phase_shift_count: usize,
    /// Number of scale-factor header rows.
    pub scale_factor_count: usize,
    /// Number of GLONASS slot/channel rows.
    pub glonass_slot_count: usize,
    /// Whether marker_name is present.
    pub has_marker_name: bool,
    /// Marker name, null-terminated when present.
    pub marker_name: [c_char; RINEX_OBS_MARKER_C_BYTES],
}

/// One per-system RINEX observation code from the header.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRinexObsCode {
    /// GNSS system as SidereonGnssSystem.
    pub system: u32,
    /// Observation code, null-terminated.
    pub code: [c_char; RINEX_OBS_CODE_C_BYTES],
}

/// One parsed RINEX observation epoch summary.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRinexObsEpoch {
    /// Civil epoch in the file's time scale.
    pub epoch: SidereonCalendarEpoch,
    /// RINEX epoch flag.
    pub flag: u8,
    /// Number of satellites observed at this epoch.
    pub satellite_count: usize,
}

/// One labelled raw RINEX observation value.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRinexObsValue {
    /// Satellite token.
    pub sat_id: SidereonSatelliteToken,
    /// RINEX observation code.
    pub code: [c_char; RINEX_OBS_CODE_C_BYTES],
    /// Observation kind as SidereonRinexObsKind.
    pub kind: u32,
    /// Whether value is present. False means the field was blank.
    pub has_value: bool,
    /// Parsed value when present.
    pub value: f64,
    /// Loss-of-lock indicator, or -1 when absent.
    pub lli: i32,
    /// Signal-strength indicator, or -1 when absent.
    pub ssi: i32,
}

/// One selected single-frequency pseudorange row from a RINEX OBS epoch.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRinexObsPseudorange {
    /// Satellite token.
    pub sat_id: SidereonSatelliteToken,
    /// Selected code pseudorange, meters.
    pub pseudorange_m: f64,
}

/// One carrier-phase row with carrier metadata.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRinexObsCarrierPhase {
    /// Satellite token.
    pub sat_id: SidereonSatelliteToken,
    /// RINEX carrier observation code.
    pub code: [c_char; RINEX_OBS_CODE_C_BYTES],
    /// Whether value_cycles is present.
    pub has_value_cycles: bool,
    /// Phase in cycles when present.
    pub value_cycles: f64,
    /// Loss-of-lock indicator, or -1 when absent.
    pub lli: i32,
    /// Signal-strength indicator, or -1 when absent.
    pub ssi: i32,
    /// Whether frequency_hz is present.
    pub has_frequency_hz: bool,
    /// Carrier frequency, hertz.
    pub frequency_hz: f64,
    /// Whether wavelength_m is present.
    pub has_wavelength_m: bool,
    /// Carrier wavelength, meters.
    pub wavelength_m: f64,
    /// Whether value_m is present.
    pub has_value_m: bool,
    /// Carrier phase converted to meters.
    pub value_m: f64,
    /// Header phase-shift metadata, cycles.
    pub phase_shift_cycles: f64,
}

/// Parse RINEX 3 observation text into a typed product. On success writes a newly
/// owned handle to *out_obs. Release it with sidereon_rinex_obs_free.
///
/// Safety: data must point to len readable bytes; out_obs must point to storage
/// for a SidereonRinexObs*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_parse(
    data: *const u8,
    len: usize,
    out_obs: *mut *mut SidereonRinexObs,
) -> SidereonStatus {
    ffi_boundary("sidereon_rinex_obs_parse", SidereonStatus::Panic, || {
        let out_obs = c_try!(require_out(out_obs, "sidereon_rinex_obs_parse", "out_obs"));
        *out_obs = ptr::null_mut();
        let bytes = c_try!(require_slice(data, len, "sidereon_rinex_obs_parse", "data"));
        let text = match str::from_utf8(bytes) {
            Ok(text) => text,
            Err(_) => {
                set_last_error("sidereon_rinex_obs_parse: data is not valid UTF-8".to_string());
                return SidereonStatus::InvalidToken;
            }
        };
        let inner = match RinexObs::parse(text) {
            Ok(obs) => obs,
            Err(err) => {
                set_last_error(format!("sidereon_rinex_obs_parse: {err}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        write_boxed_handle(out_obs, SidereonRinexObs { inner });
        SidereonStatus::Ok
    })
}

/// Read and parse a RINEX observation file from a UTF-8 filesystem path. On
/// success writes a newly owned handle to *out_obs. Release it with
/// sidereon_rinex_obs_free. Delegates to sidereon::load_rinex_obs.
///
/// Safety: path must be a non-empty UTF-8 C string; out_obs must point to
/// storage for a SidereonRinexObs*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_load(
    path: *const c_char,
    out_obs: *mut *mut SidereonRinexObs,
) -> SidereonStatus {
    ffi_boundary("sidereon_rinex_obs_load", SidereonStatus::Panic, || {
        let out_obs = c_try!(require_out(out_obs, "sidereon_rinex_obs_load", "out_obs"));
        *out_obs = ptr::null_mut();
        let path = c_try!(parse_c_string("sidereon_rinex_obs_load", "path", path));
        let inner = match sidereon::load_rinex_obs(&path) {
            Ok(obs) => obs,
            Err(err) => {
                set_last_error(format!("sidereon_rinex_obs_load: {err}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        write_boxed_handle(out_obs, SidereonRinexObs { inner });
        SidereonStatus::Ok
    })
}

/// Write the parsed RINEX version (e.g. 3.05) to *out_version.
///
/// Safety: obs must be a live handle from sidereon_rinex_obs_parse; out_version
/// must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_version(
    obs: *const SidereonRinexObs,
    out_version: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_rinex_obs_version", SidereonStatus::Panic, || {
        let out_version = c_try!(require_out(
            out_version,
            "sidereon_rinex_obs_version",
            "out_version"
        ));
        *out_version = 0.0;
        let obs = c_try!(require_ref(obs, "sidereon_rinex_obs_version", "obs"));
        *out_version = obs.inner.header().version;
        SidereonStatus::Ok
    })
}

/// Write the number of epoch records (file order, event records included) to
/// *out_count.
///
/// Safety: obs must be a live handle from sidereon_rinex_obs_parse; out_count
/// must point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_epoch_count(
    obs: *const SidereonRinexObs,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_obs_epoch_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_rinex_obs_epoch_count",
                "out_count"
            ));
            *out_count = 0;
            let obs = c_try!(require_ref(obs, "sidereon_rinex_obs_epoch_count", "obs"));
            *out_count = obs.inner.epochs().len();
            SidereonStatus::Ok
        },
    )
}

/// Copy the parsed RINEX observation header summary.
///
/// Safety: obs must be a live handle from sidereon_rinex_obs_parse; out_header
/// must point to a SidereonRinexObsHeader.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_header(
    obs: *const SidereonRinexObs,
    out_header: *mut SidereonRinexObsHeader,
) -> SidereonStatus {
    ffi_boundary("sidereon_rinex_obs_header", SidereonStatus::Panic, || {
        let out_header = c_try!(require_out(
            out_header,
            "sidereon_rinex_obs_header",
            "out_header"
        ));
        *out_header = empty_rinex_obs_header();
        let obs = c_try!(require_ref(obs, "sidereon_rinex_obs_header", "obs"));
        let header = obs.inner.header();
        let mut out = empty_rinex_obs_header();
        out.version = header.version;
        if let Some(position) = header.approx_position_m {
            out.has_approx_position_m = true;
            out.approx_position_m = position;
        }
        if let Some(delta) = header.antenna_delta_hen_m {
            out.has_antenna_delta_hen_m = true;
            out.antenna_delta_hen_m = delta;
        }
        if let Some(interval_s) = header.interval_s {
            out.has_interval_s = true;
            out.interval_s = interval_s;
        }
        if let Some((epoch, scale)) = header.time_of_first_obs {
            out.has_time_of_first_obs = true;
            out.time_of_first_obs = rinex_epoch_time_to_c(epoch);
            out.time_of_first_obs_scale = time_scale_to_c_code(scale);
        }
        out.obs_code_count = header.obs_codes.values().map(Vec::len).sum();
        out.phase_shift_count = header.phase_shifts.len();
        out.scale_factor_count = header.scale_factors.len();
        out.glonass_slot_count = header.glonass_slots.len();
        if let Some(marker_name) = &header.marker_name {
            out.has_marker_name = true;
            out.marker_name = fixed_c_chars::<RINEX_OBS_MARKER_C_BYTES>(marker_name);
        }
        *out_header = out;
        SidereonStatus::Ok
    })
}

/// Copy the per-system observation-code table from the header. Uses the
/// variable-length output contract documented at the top of the header.
///
/// Safety: obs must be a live handle from sidereon_rinex_obs_parse; out must
/// point to at least len writable SidereonRinexObsCode entries or be NULL when
/// len is 0; out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_codes(
    obs: *const SidereonRinexObs,
    out: *mut SidereonRinexObsCode,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_rinex_obs_codes", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_rinex_obs_codes",
            out_written,
            out_required
        ));
        let obs = c_try!(require_ref(obs, "sidereon_rinex_obs_codes", "obs"));
        let values: Vec<SidereonRinexObsCode> = obs
            .inner
            .header()
            .obs_codes
            .iter()
            .flat_map(|(system, codes)| {
                codes.iter().map(move |code| SidereonRinexObsCode {
                    system: gnss_system_to_c(*system) as u32,
                    code: rinex_obs_code_to_c(code),
                })
            })
            .collect();
        c_try!(copy_prefix_to_c(
            "sidereon_rinex_obs_codes",
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

/// Copy parsed epoch summaries in file order. Uses the variable-length output
/// contract documented at the top of the header.
///
/// Safety: obs must be a live handle from sidereon_rinex_obs_parse; out must
/// point to at least len writable SidereonRinexObsEpoch entries or be NULL when
/// len is 0; out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_epochs(
    obs: *const SidereonRinexObs,
    out: *mut SidereonRinexObsEpoch,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_rinex_obs_epochs", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_rinex_obs_epochs",
            out_written,
            out_required
        ));
        let obs = c_try!(require_ref(obs, "sidereon_rinex_obs_epochs", "obs"));
        let values: Vec<SidereonRinexObsEpoch> = obs
            .inner
            .epochs()
            .iter()
            .map(|epoch| SidereonRinexObsEpoch {
                epoch: rinex_epoch_time_to_c(epoch.epoch),
                flag: epoch.flag,
                satellite_count: epoch.sats.len(),
            })
            .collect();
        c_try!(copy_prefix_to_c(
            "sidereon_rinex_obs_epochs",
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

/// Copy flattened raw observation values for one epoch. Uses every observation
/// code in the header, in satellite and header-code order.
///
/// Safety: obs must be a live handle from sidereon_rinex_obs_parse; out must
/// point to at least len writable SidereonRinexObsValue entries or be NULL when
/// len is 0; out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_values(
    obs: *const SidereonRinexObs,
    epoch_index: usize,
    out: *mut SidereonRinexObsValue,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_rinex_obs_values", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_rinex_obs_values",
            out_written,
            out_required
        ));
        let obs = c_try!(require_ref(obs, "sidereon_rinex_obs_values", "obs"));
        let Some(epoch) = obs.inner.epochs().get(epoch_index) else {
            set_last_error(format!(
                "sidereon_rinex_obs_values: epoch_index {epoch_index} out of range ({})",
                obs.inner.epochs().len()
            ));
            return SidereonStatus::InvalidArgument;
        };
        let rows =
            match rinex_obs_observation_values(&obs.inner, epoch, &RinexObservationFilter::all()) {
                Ok(rows) => rows,
                Err(err) => return rinex_obs_error("sidereon_rinex_obs_values", err),
            };
        let values: Vec<SidereonRinexObsValue> = rows
            .into_iter()
            .flat_map(|(sat, rows)| {
                rows.into_iter().map(move |row| SidereonRinexObsValue {
                    sat_id: satellite_token(sat),
                    code: rinex_obs_code_to_c(&row.code),
                    kind: rinex_obs_kind_to_c(row.kind),
                    has_value: row.value.is_some(),
                    value: row.value.unwrap_or(0.0),
                    lli: row.lli.map(i32::from).unwrap_or(-1),
                    ssi: row.ssi.map(i32::from).unwrap_or(-1),
                })
            })
            .collect();
        c_try!(copy_prefix_to_c(
            "sidereon_rinex_obs_values",
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

/// Copy flattened default-policy single-frequency pseudoranges for one epoch.
///
/// Safety: obs must be a live handle from sidereon_rinex_obs_parse; out must
/// point to at least len writable SidereonRinexObsPseudorange entries or be NULL
/// when len is 0; out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_pseudoranges(
    obs: *const SidereonRinexObs,
    epoch_index: usize,
    out: *mut SidereonRinexObsPseudorange,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_obs_pseudoranges",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rinex_obs_pseudoranges",
                out_written,
                out_required
            ));
            let obs = c_try!(require_ref(obs, "sidereon_rinex_obs_pseudoranges", "obs"));
            let Some(epoch) = obs.inner.epochs().get(epoch_index) else {
                set_last_error(format!(
                    "sidereon_rinex_obs_pseudoranges: epoch_index {epoch_index} out of range ({})",
                    obs.inner.epochs().len()
                ));
                return SidereonStatus::InvalidArgument;
            };
            let policy = match RinexSignalPolicy::default_for(obs.inner.header().version) {
                Ok(policy) => policy,
                Err(err) => return rinex_obs_error("sidereon_rinex_obs_pseudoranges", err),
            };
            let rows = match rinex_obs_pseudoranges(&obs.inner, epoch, &policy) {
                Ok(rows) => rows,
                Err(err) => return rinex_obs_error("sidereon_rinex_obs_pseudoranges", err),
            };
            let values: Vec<SidereonRinexObsPseudorange> = rows
                .into_iter()
                .map(|(sat, pseudorange_m)| SidereonRinexObsPseudorange {
                    sat_id: satellite_token(sat),
                    pseudorange_m,
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_rinex_obs_pseudoranges",
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

/// Copy flattened carrier-phase rows for one epoch.
///
/// Safety: obs must be a live handle from sidereon_rinex_obs_parse; out must
/// point to at least len writable SidereonRinexObsCarrierPhase entries or be NULL
/// when len is 0; out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_carrier_phase(
    obs: *const SidereonRinexObs,
    epoch_index: usize,
    out: *mut SidereonRinexObsCarrierPhase,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_obs_carrier_phase",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rinex_obs_carrier_phase",
                out_written,
                out_required
            ));
            let obs = c_try!(require_ref(obs, "sidereon_rinex_obs_carrier_phase", "obs"));
            let Some(epoch) = obs.inner.epochs().get(epoch_index) else {
                set_last_error(format!(
                    "sidereon_rinex_obs_carrier_phase: epoch_index {epoch_index} out of range ({})",
                    obs.inner.epochs().len()
                ));
                return SidereonStatus::InvalidArgument;
            };
            let rows = match rinex_obs_carrier_phase_rows(
                &obs.inner,
                epoch,
                &RinexObservationFilter::all(),
            ) {
                Ok(rows) => rows,
                Err(err) => return rinex_obs_error("sidereon_rinex_obs_carrier_phase", err),
            };
            let values: Vec<SidereonRinexObsCarrierPhase> = rows
                .into_iter()
                .flat_map(|(sat, rows)| {
                    rows.into_iter()
                        .map(move |row| SidereonRinexObsCarrierPhase {
                            sat_id: satellite_token(sat),
                            code: rinex_obs_code_to_c(&row.code),
                            has_value_cycles: row.value_cycles.is_some(),
                            value_cycles: row.value_cycles.unwrap_or(0.0),
                            lli: row.lli.map(i32::from).unwrap_or(-1),
                            ssi: row.ssi.map(i32::from).unwrap_or(-1),
                            has_frequency_hz: row.frequency_hz.is_some(),
                            frequency_hz: row.frequency_hz.unwrap_or(0.0),
                            has_wavelength_m: row.wavelength_m.is_some(),
                            wavelength_m: row.wavelength_m.unwrap_or(0.0),
                            has_value_m: row.value_m.is_some(),
                            value_m: row.value_m.unwrap_or(0.0),
                            phase_shift_cycles: row.phase_shift_cycles,
                        })
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_rinex_obs_carrier_phase",
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

/// Look up one observation value at `epoch_index` for satellite `sat_id` and
/// observation `code` (e.g. "C1C"). On success writes the value to *out_value and
/// whether the field was present to *out_present (a blank field is present=false
/// with out_value=0). The loss-of-lock and signal-strength indicators are written
/// to *out_lli and *out_ssi as -1 when absent. The numbers are exactly what the
/// engine parsed.
///
/// Returns SIDEREON_STATUS_INVALID_ARGUMENT if epoch_index is out of range, the
/// satellite is not observed at that epoch, or `code` is not a declared code for
/// that satellite's constellation.
///
/// Safety: obs must be a live handle from sidereon_rinex_obs_parse; sat_id and
/// code must be null-terminated C strings; out_value, out_present, out_lli and
/// out_ssi must each point to writable storage of the documented type.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_observation(
    obs: *const SidereonRinexObs,
    epoch_index: usize,
    sat_id: *const c_char,
    code: *const c_char,
    out_value: *mut f64,
    out_present: *mut bool,
    out_lli: *mut i32,
    out_ssi: *mut i32,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_obs_observation",
        SidereonStatus::Panic,
        || {
            let out_value = c_try!(require_out(
                out_value,
                "sidereon_rinex_obs_observation",
                "out_value"
            ));
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_rinex_obs_observation",
                "out_present"
            ));
            let out_lli = c_try!(require_out(
                out_lli,
                "sidereon_rinex_obs_observation",
                "out_lli"
            ));
            let out_ssi = c_try!(require_out(
                out_ssi,
                "sidereon_rinex_obs_observation",
                "out_ssi"
            ));
            *out_value = 0.0;
            *out_present = false;
            *out_lli = -1;
            *out_ssi = -1;
            let obs = c_try!(require_ref(obs, "sidereon_rinex_obs_observation", "obs"));
            let sat = c_try!(parse_satellite_token(
                "sidereon_rinex_obs_observation",
                sat_id
            ));
            let code = c_try!(parse_bounded_c_string(
                "sidereon_rinex_obs_observation",
                "code",
                code,
                MAX_ANTEX_FREQUENCY_BYTES
            ));

            let epochs = obs.inner.epochs();
            let Some(epoch) = epochs.get(epoch_index) else {
                set_last_error(format!(
                    "sidereon_rinex_obs_observation: epoch_index {epoch_index} out of range ({})",
                    epochs.len()
                ));
                return SidereonStatus::InvalidArgument;
            };
            let Some(values) = epoch.sats.get(&sat) else {
                set_last_error(format!(
                "sidereon_rinex_obs_observation: satellite {sat} not observed at epoch {epoch_index}"
            ));
                return SidereonStatus::InvalidArgument;
            };
            let Some(codes) = obs.inner.obs_codes(sat.system) else {
                set_last_error(format!(
                    "sidereon_rinex_obs_observation: no observation codes for {}",
                    sat.system
                ));
                return SidereonStatus::InvalidArgument;
            };
            let Some(code_index) = codes.iter().position(|c| c == &code) else {
                set_last_error(format!(
                    "sidereon_rinex_obs_observation: code {code} not declared for {}",
                    sat.system
                ));
                return SidereonStatus::InvalidArgument;
            };
            let Some(value) = values.get(code_index) else {
                // The satellite row is shorter than the declared code list (trailing
                // blanks), so this code has no field at this epoch.
                return SidereonStatus::Ok;
            };
            if let Some(v) = value.value {
                *out_value = v;
                *out_present = true;
            }
            if let Some(lli) = value.lli {
                *out_lli = i32::from(lli);
            }
            if let Some(ssi) = value.ssi {
                *out_ssi = i32::from(ssi);
            }
            SidereonStatus::Ok
        },
    )
}

/// Serialize a RINEX 3 observation product back to RINEX text. The output is not
/// null-terminated. Uses the variable-length output contract documented at the
/// top of the header: call once with out=NULL to learn *out_required, then again
/// with a buffer of that size. Round-trips with sidereon_rinex_obs_parse.
///
/// Safety: obs must be a live handle from sidereon_rinex_obs_parse; out must point
/// to at least len writable bytes or be NULL when len is 0; out_written and
/// out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_to_rinex_text(
    obs: *const SidereonRinexObs,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_obs_to_rinex_text",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rinex_obs_to_rinex_text",
                out_written,
                out_required
            ));
            let obs = c_try!(require_ref(obs, "sidereon_rinex_obs_to_rinex_text", "obs"));
            let text = obs.inner.to_rinex_string();
            c_try!(copy_prefix_to_c(
                "sidereon_rinex_obs_to_rinex_text",
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

/// Release a RINEX observation handle from sidereon_rinex_obs_parse. Passing NULL
/// is a no-op.
///
/// Safety: obs must be NULL or a live handle from sidereon_rinex_obs_parse that
/// has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_free(obs: *mut SidereonRinexObs) {
    ffi_boundary("sidereon_rinex_obs_free", (), || {
        free_boxed(obs);
    });
}

/// Extract RINEX receiver-clock offsets as phase deviations in seconds. Event
/// epochs are returned with `has_phase_s == false`.
///
/// Safety: obs must be a live RINEX OBS handle; out points to len
/// SidereonClockPhaseSample entries or NULL when len is 0; out_written and
/// out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_receiver_clock_phase_deviations(
    obs: *const SidereonRinexObs,
    out: *mut SidereonClockPhaseSample,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_obs_receiver_clock_phase_deviations",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rinex_obs_receiver_clock_phase_deviations",
                out_written,
                out_required
            ));
            let obs = c_try!(require_ref(
                obs,
                "sidereon_rinex_obs_receiver_clock_phase_deviations",
                "obs"
            ));
            let values: Vec<SidereonClockPhaseSample> =
                core_receiver_clock_phase_deviations(&obs.inner)
                    .into_iter()
                    .map(|value| SidereonClockPhaseSample {
                        has_phase_s: value.is_some(),
                        phase_s: value.unwrap_or(0.0),
                    })
                    .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_rinex_obs_receiver_clock_phase_deviations",
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

// === GNSS constellation identity catalog (CelesTrak + NAVCEN) ==============
//
// Wraps sidereon_core::constellation: build a merged GPS identity catalog from
// CelesTrak gps-ops OMM/JSON and an optional NAVCEN status overlay, export the
// compact mapping CSV, and validate the catalog against a list of SP3/RINEX
// satellite ids. The catalog and the validation report are opaque handles whose
// fields are read back through accessor functions using the variable-length
// output contract documented at the top of the header.

// --- RINEX NAV auxiliary and raw-record routes -------------------------------

/// A lenient RINEX NAV parse result. The owned handle retains both successfully
/// parsed records and the core parser's skipped block diagnostics.
pub struct SidereonRinexNavParse {
    pub(crate) inner: sidereon_core::rinex::nav::NavParse,
}

/// One diagnostic for a NAV block skipped by the lenient parser.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSkippedNavBlock {
    /// Satellite token from the skipped block.
    pub satellite: SidereonSatelliteToken,
    /// Null-terminated core diagnostic text from the skipped block.
    pub message: [c_char; 256],
}

/// Owned list of full, pre-filter RINEX NAV broadcast records.
pub struct SidereonRinexNavRecords {
    pub(crate) records: Vec<sidereon_core::rinex::nav::BroadcastRecord>,
}

/// Owned list of representable parsed GLONASS RINEX state-vector records and
/// separately inspectable skipped extended-slot diagnostics.
pub struct SidereonRinexGlonassRecords {
    pub(crate) records: Vec<sidereon_core::rinex::nav::GlonassRecord>,
    pub(crate) skipped: Vec<sidereon_core::rinex::nav::SkippedGlonass>,
}

/// One GLONASS RINEX record skipped because its extended satellite slot cannot
/// be represented by the core satellite identifier.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSkippedGlonassRecord {
    /// Raw satellite token from the skipped input record, such as `R28`.
    pub satellite: SidereonSatelliteToken,
}

/// Parse a RINEX NAV source while retaining core skipped-block diagnostics.
/// Header failures remain errors; malformed supported body blocks are retained
/// in the returned `SidereonRinexNavParse` diagnostics.
///
/// Safety: data points to len readable bytes; out_parse points to an owned
/// SidereonRinexNavParse handle slot.
#[no_mangle]
pub unsafe extern "C" fn sidereon_parse_rinex_nav_lenient(
    data: *const u8,
    len: usize,
    out_parse: *mut *mut SidereonRinexNavParse,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_parse_rinex_nav_lenient",
        SidereonStatus::Panic,
        || {
            let out_parse = c_try!(require_out(
                out_parse,
                "sidereon_parse_rinex_nav_lenient",
                "out_parse"
            ));
            *out_parse = ptr::null_mut();
            let bytes = c_try!(require_slice(
                data,
                len,
                "sidereon_parse_rinex_nav_lenient",
                "data"
            ));
            let text = match str::from_utf8(bytes) {
                Ok(text) => text,
                Err(_) => {
                    set_last_error(
                        "sidereon_parse_rinex_nav_lenient: data is not valid UTF-8".to_string(),
                    );
                    return SidereonStatus::InvalidToken;
                }
            };
            let inner = match sidereon_core::rinex::nav::parse_nav_lenient(text) {
                Ok(inner) => inner,
                Err(err) => {
                    set_last_error(format!("sidereon_parse_rinex_nav_lenient: {err}"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            write_boxed_handle(out_parse, SidereonRinexNavParse { inner });
            SidereonStatus::Ok
        },
    )
}

/// Release a lenient NAV parse handle. Passing NULL is a no-op.
///
/// Safety: parse is NULL or a live handle from
/// sidereon_parse_rinex_nav_lenient.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nav_parse_free(parse: *mut SidereonRinexNavParse) {
    free_boxed(parse);
}

/// Write the number of successfully parsed records in a lenient NAV result.
///
/// Safety: parse is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nav_parse_record_count(
    parse: *const SidereonRinexNavParse,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_nav_parse_record_count",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_count,
                "sidereon_nav_parse_record_count",
                "out_count"
            ));
            *out = 0;
            let parse = c_try!(require_ref(
                parse,
                "sidereon_nav_parse_record_count",
                "parse"
            ));
            *out = parse.inner.records.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy one full successfully parsed NAV record by deterministic file-order
/// index.
///
/// Safety: parse is a live handle; out_record points to a writable full record.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nav_parse_record(
    parse: *const SidereonRinexNavParse,
    index: usize,
    out_record: *mut SidereonBroadcastRecord,
) -> SidereonStatus {
    ffi_boundary("sidereon_nav_parse_record", SidereonStatus::Panic, || {
        let out = c_try!(require_out(
            out_record,
            "sidereon_nav_parse_record",
            "out_record"
        ));
        *out = empty_broadcast_record();
        let parse = c_try!(require_ref(parse, "sidereon_nav_parse_record", "parse"));
        let Some(record) = parse.inner.records.get(index) else {
            set_last_error(format!(
                "sidereon_nav_parse_record: index {index} out of range"
            ));
            return SidereonStatus::InvalidArgument;
        };
        *out = broadcast_record_to_c_full(record);
        SidereonStatus::Ok
    })
}

/// Write the number of skipped NAV block diagnostics in a lenient result.
///
/// Safety: parse is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nav_parse_skipped_count(
    parse: *const SidereonRinexNavParse,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_nav_parse_skipped_count",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_count,
                "sidereon_nav_parse_skipped_count",
                "out_count"
            ));
            *out = 0;
            let parse = c_try!(require_ref(
                parse,
                "sidereon_nav_parse_skipped_count",
                "parse"
            ));
            *out = parse.inner.skipped.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy one skipped NAV block diagnostic by deterministic file-order index.
///
/// Safety: parse is a live handle; out_skipped points to a writable diagnostic.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nav_parse_skipped(
    parse: *const SidereonRinexNavParse,
    index: usize,
    out_skipped: *mut SidereonSkippedNavBlock,
) -> SidereonStatus {
    ffi_boundary("sidereon_nav_parse_skipped", SidereonStatus::Panic, || {
        let out = c_try!(require_out(
            out_skipped,
            "sidereon_nav_parse_skipped",
            "out_skipped"
        ));
        *out = SidereonSkippedNavBlock {
            satellite: satellite_token_from_text(""),
            message: [0; 256],
        };
        let parse = c_try!(require_ref(parse, "sidereon_nav_parse_skipped", "parse"));
        let Some(skipped) = parse.inner.skipped.get(index) else {
            set_last_error(format!(
                "sidereon_nav_parse_skipped: index {index} out of range"
            ));
            return SidereonStatus::InvalidArgument;
        };
        *out = SidereonSkippedNavBlock {
            satellite: satellite_token_from_text(&skipped.satellite),
            message: fixed_c_chars::<256>(&skipped.message),
        };
        SidereonStatus::Ok
    })
}

/// Copy the complete core diagnostic text for one skipped NAV block. This
/// variable-length accessor complements the bounded C display field in
/// `SidereonSkippedNavBlock`.
///
/// Safety: parse is a live handle; out points to len writable bytes or is NULL
/// when len is zero; count pointers point to writable size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nav_parse_skipped_message(
    parse: *const SidereonRinexNavParse,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_nav_parse_skipped_message",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_nav_parse_skipped_message",
                out_written,
                out_required
            ));
            let parse = c_try!(require_ref(
                parse,
                "sidereon_nav_parse_skipped_message",
                "parse"
            ));
            let Some(skipped) = parse.inner.skipped.get(index) else {
                set_last_error(format!(
                    "sidereon_nav_parse_skipped_message: index {index} out of range"
                ));
                return SidereonStatus::InvalidArgument;
            };
            c_try!(copy_prefix_to_c(
                "sidereon_nav_parse_skipped_message",
                "out",
                skipped.message.as_bytes(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Parse all supported raw RINEX NAV records before the broadcast-store
/// health/message policy filter.
///
/// Safety: data points to len readable bytes; out_records points to an owned
/// list handle slot.
#[no_mangle]
pub unsafe extern "C" fn sidereon_parse_rinex_nav_records(
    data: *const u8,
    len: usize,
    out_records: *mut *mut SidereonRinexNavRecords,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_parse_rinex_nav_records",
        SidereonStatus::Panic,
        || {
            let out_records = c_try!(require_out(
                out_records,
                "sidereon_parse_rinex_nav_records",
                "out_records"
            ));
            *out_records = ptr::null_mut();
            let bytes = c_try!(require_slice(
                data,
                len,
                "sidereon_parse_rinex_nav_records",
                "data"
            ));
            let text = match str::from_utf8(bytes) {
                Ok(text) => text,
                Err(_) => {
                    set_last_error(
                        "sidereon_parse_rinex_nav_records: data is not valid UTF-8".to_string(),
                    );
                    return SidereonStatus::InvalidToken;
                }
            };
            let records = match sidereon_core::rinex::nav::parse_nav(text) {
                Ok(records) => records,
                Err(err) => {
                    set_last_error(format!("sidereon_parse_rinex_nav_records: {err}"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            write_boxed_handle(out_records, SidereonRinexNavRecords { records });
            SidereonStatus::Ok
        },
    )
}

/// Release a raw NAV record-list handle. Passing NULL is a no-op.
///
/// Safety: records is NULL or a live handle returned by
/// sidereon_parse_rinex_nav_records.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_nav_records_free(records: *mut SidereonRinexNavRecords) {
    free_boxed(records);
}

/// Write the number of records in a raw NAV record list.
///
/// Safety: records is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_nav_records_count(
    records: *const SidereonRinexNavRecords,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_nav_records_count",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_count,
                "sidereon_rinex_nav_records_count",
                "out_count"
            ));
            *out = 0;
            let records = c_try!(require_ref(
                records,
                "sidereon_rinex_nav_records_count",
                "records"
            ));
            *out = records.records.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy one raw full NAV record by deterministic file-order index.
///
/// Safety: records is a live handle; out_record points to a writable full
/// record.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_nav_records_item(
    records: *const SidereonRinexNavRecords,
    index: usize,
    out_record: *mut SidereonBroadcastRecord,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_nav_records_item",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_record,
                "sidereon_rinex_nav_records_item",
                "out_record"
            ));
            *out = empty_broadcast_record();
            let records = c_try!(require_ref(
                records,
                "sidereon_rinex_nav_records_item",
                "records"
            ));
            let Some(record) = records.records.get(index) else {
                set_last_error(format!(
                    "sidereon_rinex_nav_records_item: index {index} out of range"
                ));
                return SidereonStatus::InvalidArgument;
            };
            *out = broadcast_record_to_c_full(record);
            SidereonStatus::Ok
        },
    )
}

/// Encode an arbitrary caller-supplied full NAV record list. The list is
/// validated by the public BroadcastStore constructor before delegation to the
/// core `encode_nav` writer, preventing an invalid CNAV record from reaching a
/// panic-only encoder path.
///
/// Safety: records points to `record_count` readable full records (or is NULL
/// when the count is zero); out follows the standard variable-output contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_encode_rinex_nav(
    records: *const SidereonBroadcastRecord,
    record_count: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_encode_rinex_nav", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_encode_rinex_nav",
            out_written,
            out_required
        ));
        let records = c_try!(require_slice(
            records,
            record_count,
            "sidereon_encode_rinex_nav",
            "records"
        ));
        let mut core_records = Vec::with_capacity(record_count);
        for record in records {
            core_records.push(c_try!(broadcast_record_from_c(
                "sidereon_encode_rinex_nav",
                record
            )));
        }
        if let Err(err) = BroadcastEphemeris::new(core_records.clone()) {
            set_last_error(format!("sidereon_encode_rinex_nav: {err}"));
            return SidereonStatus::InvalidArgument;
        }
        let text = sidereon_core::rinex::nav::encode_nav(&core_records);
        c_try!(copy_prefix_to_c(
            "sidereon_encode_rinex_nav",
            "out",
            text.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Parse RINEX GLONASS state-vector records into an owned list. Representable
/// records are retained and malformed representable records fail the parse.
/// Extended slots that cannot be represented by the core satellite identifier
/// are skipped, with their raw satellite tokens retained for inspection by
/// `sidereon_rinex_glonass_records_skipped_count` and
/// `sidereon_rinex_glonass_records_skipped_item`.
///
/// Safety: data points to len readable UTF-8 bytes; out_records points to an
/// owned list handle slot.
#[no_mangle]
pub unsafe extern "C" fn sidereon_parse_rinex_glonass_records(
    data: *const u8,
    len: usize,
    out_records: *mut *mut SidereonRinexGlonassRecords,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_parse_rinex_glonass_records",
        SidereonStatus::Panic,
        || {
            let out_records = c_try!(require_out(
                out_records,
                "sidereon_parse_rinex_glonass_records",
                "out_records"
            ));
            *out_records = ptr::null_mut();
            let bytes = c_try!(require_slice(
                data,
                len,
                "sidereon_parse_rinex_glonass_records",
                "data"
            ));
            let text = match str::from_utf8(bytes) {
                Ok(text) => text,
                Err(_) => {
                    set_last_error(
                        "sidereon_parse_rinex_glonass_records: data is not valid UTF-8".to_string(),
                    );
                    return SidereonStatus::InvalidToken;
                }
            };
            let parsed = match sidereon_core::rinex::nav::parse_glonass_lenient(text) {
                Ok(parsed) => parsed,
                Err(err) => {
                    set_last_error(format!("sidereon_parse_rinex_glonass_records: {err}"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            write_boxed_handle(
                out_records,
                SidereonRinexGlonassRecords {
                    records: parsed.records,
                    skipped: parsed.skipped,
                },
            );
            SidereonStatus::Ok
        },
    )
}

/// Release a GLONASS record-list handle. Passing NULL is a no-op.
///
/// Safety: records is NULL or a live handle returned by
/// sidereon_parse_rinex_glonass_records.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_glonass_records_free(
    records: *mut SidereonRinexGlonassRecords,
) {
    free_boxed(records);
}

/// Write the number of representable parsed GLONASS records.
///
/// Safety: records is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_glonass_records_count(
    records: *const SidereonRinexGlonassRecords,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_glonass_records_count",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_count,
                "sidereon_rinex_glonass_records_count",
                "out_count"
            ));
            *out = 0;
            let records = c_try!(require_ref(
                records,
                "sidereon_rinex_glonass_records_count",
                "records"
            ));
            *out = records.records.len();
            SidereonStatus::Ok
        },
    )
}

/// Write the number of GLONASS records skipped because their extended slots
/// are not representable by the core satellite identifier.
///
/// Safety: records is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_glonass_records_skipped_count(
    records: *const SidereonRinexGlonassRecords,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_glonass_records_skipped_count",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_count,
                "sidereon_rinex_glonass_records_skipped_count",
                "out_count"
            ));
            *out = 0;
            let records = c_try!(require_ref(
                records,
                "sidereon_rinex_glonass_records_skipped_count",
                "records"
            ));
            *out = records.skipped.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy one parsed GLONASS record by deterministic file-order index.
///
/// Safety: records is a live handle; out_record points to a writable record.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_glonass_records_item(
    records: *const SidereonRinexGlonassRecords,
    index: usize,
    out_record: *mut SidereonGlonassRecord,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_glonass_records_item",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_record,
                "sidereon_rinex_glonass_records_item",
                "out_record"
            ));
            *out = empty_glonass_record();
            let records = c_try!(require_ref(
                records,
                "sidereon_rinex_glonass_records_item",
                "records"
            ));
            let Some(record) = records.records.get(index) else {
                set_last_error(format!(
                    "sidereon_rinex_glonass_records_item: index {index} out of range"
                ));
                return SidereonStatus::InvalidArgument;
            };
            *out = glonass_record_to_c(record);
            SidereonStatus::Ok
        },
    )
}

/// Copy one skipped GLONASS record by deterministic file-order index. The
/// returned satellite token is the raw token from the input record, including
/// extended slots such as `R28`.
///
/// Safety: records is a live handle; out_skipped points to a writable value.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_glonass_records_skipped_item(
    records: *const SidereonRinexGlonassRecords,
    index: usize,
    out_skipped: *mut SidereonSkippedGlonassRecord,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_glonass_records_skipped_item",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_skipped,
                "sidereon_rinex_glonass_records_skipped_item",
                "out_skipped"
            ));
            *out = SidereonSkippedGlonassRecord {
                satellite: satellite_token_from_text(""),
            };
            let records = c_try!(require_ref(
                records,
                "sidereon_rinex_glonass_records_skipped_item",
                "records"
            ));
            let Some(skipped) = records.skipped.get(index) else {
                set_last_error(format!(
                    "sidereon_rinex_glonass_records_skipped_item: index {index} out of range"
                ));
                return SidereonStatus::InvalidArgument;
            };
            *out = SidereonSkippedGlonassRecord {
                satellite: satellite_token_from_text(&skipped.token),
            };
            SidereonStatus::Ok
        },
    )
}

/// Parse RINEX NAV-header ionosphere corrections into a fixed, presence-tagged
/// value. Empty headers are valid and return all presence flags false.
///
/// Safety: data points to len readable UTF-8 bytes; out points to a writable
/// SidereonIonoCorrections.
#[no_mangle]
pub unsafe extern "C" fn sidereon_parse_rinex_iono_corrections(
    data: *const u8,
    len: usize,
    out: *mut SidereonIonoCorrections,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_parse_rinex_iono_corrections",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_parse_rinex_iono_corrections",
                "out"
            ));
            *out = empty_iono_corrections();
            let bytes = c_try!(require_slice(
                data,
                len,
                "sidereon_parse_rinex_iono_corrections",
                "data"
            ));
            let text = match str::from_utf8(bytes) {
                Ok(text) => text,
                Err(_) => {
                    set_last_error(
                        "sidereon_parse_rinex_iono_corrections: data is not valid UTF-8"
                            .to_string(),
                    );
                    return SidereonStatus::InvalidToken;
                }
            };
            let iono = match sidereon_core::rinex::nav::parse_iono_corrections(text) {
                Ok(iono) => iono,
                Err(err) => {
                    set_last_error(format!("sidereon_parse_rinex_iono_corrections: {err}"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            *out = iono_corrections_to_c(&iono);
            SidereonStatus::Ok
        },
    )
}

/// Parse the optional RINEX NAV-header GPS-minus-UTC leap-second value.
/// `out_present` distinguishes an absent header from a present zero value.
///
/// Safety: data points to len readable UTF-8 bytes; output pointers point to
/// writable values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_parse_rinex_leap_seconds(
    data: *const u8,
    len: usize,
    out_leap_seconds: *mut f64,
    out_present: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_parse_rinex_leap_seconds",
        SidereonStatus::Panic,
        || {
            let out_value = c_try!(require_out(
                out_leap_seconds,
                "sidereon_parse_rinex_leap_seconds",
                "out_leap_seconds"
            ));
            *out_value = 0.0;
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_parse_rinex_leap_seconds",
                "out_present"
            ));
            *out_present = false;
            let bytes = c_try!(require_slice(
                data,
                len,
                "sidereon_parse_rinex_leap_seconds",
                "data"
            ));
            let text = match str::from_utf8(bytes) {
                Ok(text) => text,
                Err(_) => {
                    set_last_error(
                        "sidereon_parse_rinex_leap_seconds: data is not valid UTF-8".to_string(),
                    );
                    return SidereonStatus::InvalidToken;
                }
            };
            let value = match sidereon_core::rinex::nav::parse_leap_seconds(text) {
                Ok(value) => value,
                Err(err) => {
                    set_last_error(format!("sidereon_parse_rinex_leap_seconds: {err}"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            if let Some(value) = value {
                *out_value = value;
                *out_present = true;
            }
            SidereonStatus::Ok
        },
    )
}

// --- RINEX clock (sidereon_core::rinex::clock) -------------------------------

/// A parsed RINEX clock product. Opaque to C. Create with
/// sidereon_rinex_clock_parse; release with sidereon_rinex_clock_free.
pub struct SidereonRinexClock {
    pub(crate) inner: sidereon_core::rinex::clock::RinexClock,
}

/// Representation tag for a scale-tagged RINEX clock instant.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonRinexClockInstantRepresentation {
    /// The instant is represented by `jd_whole` plus `jd_fraction`.
    JulianDate = 0,
    /// The instant is represented by the signed 128-bit nanosecond pair.
    Nanos = 1,
}

/// Full precision, scale-tagged clock epoch. For `JulianDate`, the nanosecond
/// pair is zero; for `Nanos`, the Julian fields are zero. The signed 128-bit
/// value is transported losslessly as two's-complement high/low words.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonClockEpoch {
    /// TimeScale code.
    pub scale: u32,
    /// SidereonRinexClockInstantRepresentation code.
    pub representation: u32,
    /// Whole Julian date for the JulianDate representation.
    pub jd_whole: f64,
    /// Residual Julian-day fraction for the JulianDate representation.
    pub jd_fraction: f64,
    /// Signed high 64 bits of the Nanos representation.
    pub nanos_high: i64,
    /// Low 64 bits of the Nanos representation.
    pub nanos_low: u64,
}

/// One complete RINEX clock series sample.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonClockPoint {
    /// Scale-tagged sample epoch.
    pub epoch: SidereonClockEpoch,
    /// Satellite clock bias, seconds.
    pub bias_s: f64,
}

/// An owned per-satellite RINEX clock series. Samples remain valid until this
/// handle is released.
pub struct SidereonClockSeries {
    pub(crate) satellite: SidereonSatelliteToken,
    pub(crate) samples: Vec<sidereon_core::rinex::clock::ClockPoint>,
}

/// Parse a RINEX clock source lossily, skipping malformed and non-AS rows as
/// defined by the public core parser.
///
/// Safety: text points to len readable UTF-8 bytes; out_clock points to an
/// owned clock handle slot.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_parse_lossy(
    text: *const u8,
    len: usize,
    out_clock: *mut *mut SidereonRinexClock,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_clock_parse_lossy",
        SidereonStatus::Panic,
        || {
            let out_clock = c_try!(require_out(
                out_clock,
                "sidereon_rinex_clock_parse_lossy",
                "out_clock"
            ));
            *out_clock = ptr::null_mut();
            let bytes = c_try!(require_slice(
                text,
                len,
                "sidereon_rinex_clock_parse_lossy",
                "text"
            ));
            let text = match str::from_utf8(bytes) {
                Ok(text) => text,
                Err(_) => {
                    set_last_error(
                        "sidereon_rinex_clock_parse_lossy: text is not valid UTF-8".to_string(),
                    );
                    return SidereonStatus::InvalidToken;
                }
            };
            let inner = sidereon_core::rinex::clock::RinexClock::parse_lossy(text);
            write_boxed_handle(out_clock, SidereonRinexClock { inner });
            SidereonStatus::Ok
        },
    )
}

/// Copy the deterministic satellite-token enumeration of a RINEX clock file.
/// The core `series_rows` API supplies the series enumeration; complete
/// scale-tagged samples are available through the series handle routes below.
///
/// Safety: clock is a live handle; out points to len writable tokens or is
/// NULL when len is zero; count pointers point to writable size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_satellites(
    clock: *const SidereonRinexClock,
    out: *mut SidereonSatelliteToken,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_clock_satellites",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rinex_clock_satellites",
                out_written,
                out_required
            ));
            let clock = c_try!(require_ref(
                clock,
                "sidereon_rinex_clock_satellites",
                "clock"
            ));
            let rows = clock.inner.series_rows();
            let values: Vec<_> = rows
                .iter()
                .map(|(satellite, _)| satellite_token_from_text(satellite))
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_rinex_clock_satellites",
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

/// Write the number of satellite series in a RINEX clock file.
///
/// Safety: clock is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_series_count(
    clock: *const SidereonRinexClock,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_clock_series_count",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_count,
                "sidereon_rinex_clock_series_count",
                "out_count"
            ));
            *out = 0;
            let clock = c_try!(require_ref(
                clock,
                "sidereon_rinex_clock_series_count",
                "clock"
            ));
            *out = clock.inner.series_rows().len();
            SidereonStatus::Ok
        },
    )
}

/// Write the total number of complete scale-tagged samples in a RINEX clock.
///
/// Safety: clock is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_sample_count(
    clock: *const SidereonRinexClock,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_clock_sample_count",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_count,
                "sidereon_rinex_clock_sample_count",
                "out_count"
            ));
            *out = 0;
            let clock = c_try!(require_ref(
                clock,
                "sidereon_rinex_clock_sample_count",
                "clock"
            ));
            *out = clock.inner.series.values().map(Vec::len).sum();
            SidereonStatus::Ok
        },
    )
}

/// Return one complete RINEX clock series by deterministic satellite-order
/// index. The returned handle owns a clone of the core series.
///
/// Safety: clock is a live handle; out_series points to a writable handle slot.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_series(
    clock: *const SidereonRinexClock,
    index: usize,
    out_series: *mut *mut SidereonClockSeries,
) -> SidereonStatus {
    ffi_boundary("sidereon_rinex_clock_series", SidereonStatus::Panic, || {
        let out_series = c_try!(require_out(
            out_series,
            "sidereon_rinex_clock_series",
            "out_series"
        ));
        *out_series = ptr::null_mut();
        let clock = c_try!(require_ref(clock, "sidereon_rinex_clock_series", "clock"));
        let Some((satellite, samples)) = clock.inner.instant_series_rows().into_iter().nth(index)
        else {
            set_last_error(format!(
                "sidereon_rinex_clock_series: index {index} out of range"
            ));
            return SidereonStatus::InvalidArgument;
        };
        write_boxed_handle(
            out_series,
            SidereonClockSeries {
                satellite: satellite_token_from_text(&satellite),
                samples: samples
                    .into_iter()
                    .map(|(epoch, bias_s)| sidereon_core::rinex::clock::ClockPoint {
                        epoch,
                        bias_s,
                    })
                    .collect(),
            },
        );
        SidereonStatus::Ok
    })
}

/// Return one complete RINEX clock series for a satellite, or a null output
/// handle with status Ok when the satellite has no series.
///
/// Safety: clock is a live handle; satellite_id is a non-empty UTF-8 C string;
/// out_series points to a writable handle slot.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_series_for(
    clock: *const SidereonRinexClock,
    satellite_id: *const c_char,
    out_series: *mut *mut SidereonClockSeries,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_clock_series_for",
        SidereonStatus::Panic,
        || {
            let out_series = c_try!(require_out(
                out_series,
                "sidereon_rinex_clock_series_for",
                "out_series"
            ));
            *out_series = ptr::null_mut();
            let clock = c_try!(require_ref(
                clock,
                "sidereon_rinex_clock_series_for",
                "clock"
            ));
            let satellite = c_try!(parse_c_string(
                "sidereon_rinex_clock_series_for",
                "satellite_id",
                satellite_id
            ));
            let Some(samples) = clock.inner.series.get(&satellite) else {
                return SidereonStatus::Ok;
            };
            write_boxed_handle(
                out_series,
                SidereonClockSeries {
                    satellite: satellite_token_from_text(&satellite),
                    samples: samples.clone(),
                },
            );
            SidereonStatus::Ok
        },
    )
}

/// Release a RINEX clock series handle. Passing NULL is a no-op.
///
/// Safety: series is NULL or a live handle returned by a clock-series route.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_series_free(series: *mut SidereonClockSeries) {
    free_boxed(series);
}

/// Write the number of samples in one complete RINEX clock series.
///
/// Safety: series is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_series_sample_count(
    series: *const SidereonClockSeries,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_clock_series_sample_count",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_count,
                "sidereon_rinex_clock_series_sample_count",
                "out_count"
            ));
            *out = 0;
            let series = c_try!(require_ref(
                series,
                "sidereon_rinex_clock_series_sample_count",
                "series"
            ));
            *out = series.samples.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy the satellite identity carried by a series handle.
///
/// Safety: series is a live handle; out_satellite points to a writable token.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_series_satellite(
    series: *const SidereonClockSeries,
    out_satellite: *mut SidereonSatelliteToken,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_clock_series_satellite",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_satellite,
                "sidereon_rinex_clock_series_satellite",
                "out_satellite"
            ));
            *out = satellite_token_from_text("");
            let series = c_try!(require_ref(
                series,
                "sidereon_rinex_clock_series_satellite",
                "series"
            ));
            *out = series.satellite;
            SidereonStatus::Ok
        },
    )
}

/// Copy complete scale-tagged samples from one clock series using the standard
/// caller-buffer convention.
///
/// Safety: series is a live handle; out points to len writable samples or is
/// NULL when len is zero; count pointers point to writable size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_series_samples(
    series: *const SidereonClockSeries,
    out: *mut SidereonClockPoint,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_clock_series_samples",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rinex_clock_series_samples",
                out_written,
                out_required
            ));
            let series = c_try!(require_ref(
                series,
                "sidereon_rinex_clock_series_samples",
                "series"
            ));
            let values: Vec<_> = series.samples.iter().map(clock_point_to_c).collect();
            c_try!(copy_prefix_to_c(
                "sidereon_rinex_clock_series_samples",
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

/// Parse a RINEX clock file. On success writes a newly owned handle to
/// *out_clock. Delegates to sidereon_core::rinex::clock::RinexClock::parse.
///
/// Safety: text points to len readable bytes; out_clock points to a
/// SidereonRinexClock*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_parse(
    text: *const u8,
    len: usize,
    out_clock: *mut *mut SidereonRinexClock,
) -> SidereonStatus {
    ffi_boundary("sidereon_rinex_clock_parse", SidereonStatus::Panic, || {
        let out_clock = c_try!(require_out(
            out_clock,
            "sidereon_rinex_clock_parse",
            "out_clock"
        ));
        *out_clock = ptr::null_mut();
        let bytes = c_try!(require_slice(
            text,
            len,
            "sidereon_rinex_clock_parse",
            "text"
        ));
        let text = match str::from_utf8(bytes) {
            Ok(s) => s,
            Err(_) => {
                set_last_error("sidereon_rinex_clock_parse: text is not valid UTF-8".to_string());
                return SidereonStatus::InvalidToken;
            }
        };
        match sidereon_core::rinex::clock::RinexClock::parse(text) {
            Ok(inner) => {
                write_boxed_handle(out_clock, SidereonRinexClock { inner });
                SidereonStatus::Ok
            }
            Err(err) => {
                set_last_error(format!("sidereon_rinex_clock_parse: {err:?}"));
                SidereonStatus::InvalidArgument
            }
        }
    })
}

/// Release a RINEX clock handle. Passing NULL is a no-op.
///
/// Safety: clock must be a handle from sidereon_rinex_clock_parse or NULL.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_free(clock: *mut SidereonRinexClock) {
    free_boxed(clock);
}

/// Write the number of satellites with a clock series to *out_count.
///
/// Safety: clock is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_satellite_count(
    clock: *const SidereonRinexClock,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_clock_satellite_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_rinex_clock_satellite_count",
                "out_count"
            ));
            *out_count = 0;
            let clock = c_try!(require_ref(
                clock,
                "sidereon_rinex_clock_satellite_count",
                "clock"
            ));
            *out_count = clock.inner.series.len();
            SidereonStatus::Ok
        },
    )
}

/// Interpolate a satellite clock bias (seconds) at a GPS-seconds epoch. Writes
/// the bias to *out_bias_s and sets *out_available to whether the satellite has a
/// usable value at that epoch. Delegates to
/// sidereon_core::rinex::clock::RinexClock::clock_s_at_gps_seconds.
///
/// Safety: clock is a live handle; satellite_id is a null-terminated token;
/// out_bias_s points to a double; out_available points to a bool.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_bias_at_gps_seconds(
    clock: *const SidereonRinexClock,
    satellite_id: *const c_char,
    gps_seconds: f64,
    out_bias_s: *mut f64,
    out_available: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_clock_bias_at_gps_seconds",
        SidereonStatus::Panic,
        || {
            let out_bias_s = c_try!(require_out(
                out_bias_s,
                "sidereon_rinex_clock_bias_at_gps_seconds",
                "out_bias_s"
            ));
            *out_bias_s = 0.0;
            let out_available = c_try!(require_out(
                out_available,
                "sidereon_rinex_clock_bias_at_gps_seconds",
                "out_available"
            ));
            *out_available = false;
            let clock = c_try!(require_ref(
                clock,
                "sidereon_rinex_clock_bias_at_gps_seconds",
                "clock"
            ));
            if satellite_id.is_null() {
                set_last_error(
                    "sidereon_rinex_clock_bias_at_gps_seconds: null satellite_id".to_string(),
                );
                return SidereonStatus::NullPointer;
            }
            let sat = match CStr::from_ptr(satellite_id).to_str() {
                Ok(s) => s,
                Err(_) => {
                    set_last_error(
                        "sidereon_rinex_clock_bias_at_gps_seconds: satellite_id not UTF-8"
                            .to_string(),
                    );
                    return SidereonStatus::InvalidToken;
                }
            };
            match clock.inner.clock_s_at_gps_seconds(sat, gps_seconds) {
                Ok(Some(bias)) => {
                    *out_bias_s = bias;
                    *out_available = true;
                    SidereonStatus::Ok
                }
                Ok(None) => SidereonStatus::Ok,
                Err(err) => {
                    set_last_error(format!("sidereon_rinex_clock_bias_at_gps_seconds: {err:?}"));
                    SidereonStatus::InvalidArgument
                }
            }
        },
    )
}

/// Serialize a RINEX clock product back to text (not null-terminated).
/// Variable-length output contract. Delegates to
/// sidereon_core::rinex::clock::RinexClock::to_rinex_string.
///
/// Safety: clock is a live handle; out points to len writable bytes or NULL when
/// len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_to_text(
    clock: *const SidereonRinexClock,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_clock_to_text",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rinex_clock_to_text",
                out_written,
                out_required
            ));
            let clock = c_try!(require_ref(clock, "sidereon_rinex_clock_to_text", "clock"));
            let text = c_try!(clock.inner.to_rinex_string().map_err(|err| {
                set_last_error(format!("sidereon_rinex_clock_to_text: {err}"));
                SidereonStatus::InvalidArgument
            }));
            c_try!(copy_prefix_to_c(
                "sidereon_rinex_clock_to_text",
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

fn clock_point_to_c(point: &sidereon_core::rinex::clock::ClockPoint) -> SidereonClockPoint {
    let (representation, jd_whole, jd_fraction, nanos_high, nanos_low) = match point.epoch.repr {
        InstantRepr::JulianDate(jd) => (
            SidereonRinexClockInstantRepresentation::JulianDate as u32,
            jd.jd_whole,
            jd.fraction,
            0,
            0,
        ),
        InstantRepr::Nanos(nanos) => {
            let bits = nanos as u128;
            (
                SidereonRinexClockInstantRepresentation::Nanos as u32,
                0.0,
                0.0,
                (bits >> 64) as u64 as i64,
                bits as u64,
            )
        }
    };
    SidereonClockPoint {
        epoch: SidereonClockEpoch {
            scale: time_scale_to_c_code(point.epoch.scale),
            representation,
            jd_whole,
            jd_fraction,
            nanos_high,
            nanos_low,
        },
        bias_s: point.bias_s,
    }
}

// --- RINEX navigation serialize (sidereon_core::rinex_nav) -------------------

/// Serialize a parsed broadcast-ephemeris store back to RINEX navigation text.
/// The store's records are written via
/// sidereon_core::rinex_nav::encode_nav. Uses the variable-length output
/// contract: pass out=NULL/len=0 to size the buffer (out_required), then call
/// again with a buffer of at least out_required bytes.
///
/// Safety: eph is a live broadcast handle; out points to len bytes or NULL when
/// len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_encode_nav(
    eph: *const SidereonBroadcastEphemeris,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_rinex_encode_nav", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_rinex_encode_nav",
            out_written,
            out_required
        ));
        let eph = c_try!(require_ref(eph, "sidereon_rinex_encode_nav", "eph"));
        let text = sidereon_core::rinex::nav::encode_nav(eph.inner.records());
        c_try!(copy_prefix_to_c(
            "sidereon_rinex_encode_nav",
            "out",
            text.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

// === Round-2 RINEX QC, lint, and repair =====================================

pub const RINEX_QC_CODE_C_BYTES: usize = 16;

pub const RINEX_QC_FIELD_C_BYTES: usize = 65;

pub struct SidereonRinexLintReport {
    pub(crate) inner: sidereon_core::rinex::qc::LintReport,
}

pub struct SidereonRinexRepair {
    pub(crate) text: Vec<u8>,
    pub(crate) crinex_text: Option<Vec<u8>>,
    pub(crate) actions: Vec<sidereon_core::rinex::qc::RepairAction>,
    pub(crate) remaining: sidereon_core::rinex::qc::LintReport,
    pub(crate) decoded_from_crinex: bool,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonRinexQcSeverity {
    Fatal = 0,
    Error = 1,
    Warning = 2,
    Info = 3,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRinexLintSummary {
    pub finding_count: usize,
    pub fatal_count: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    pub is_clean: bool,
    pub decoded_from_crinex: bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRinexLintFinding {
    pub code: [c_char; RINEX_QC_CODE_C_BYTES],
    pub severity: u32,
    pub repairable: bool,
    pub has_epoch_index: bool,
    pub epoch_index: usize,
    pub has_satellite: bool,
    pub satellite: SidereonSatelliteToken,
    pub has_field: bool,
    pub field: [c_char; RINEX_QC_FIELD_C_BYTES],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRinexRepairOptions {
    pub has_file_stamp: bool,
    pub file_stamp_program: [c_char; RINEX_QC_FIELD_C_BYTES],
    pub file_stamp_run_by: [c_char; RINEX_QC_FIELD_C_BYTES],
    pub file_stamp_date: [c_char; RINEX_QC_FIELD_C_BYTES],
    pub set_interval: bool,
    pub set_time_of_last_obs: bool,
    pub set_obs_counts: bool,
    pub drop_empty_records: bool,
    pub sort_records: bool,
    pub drop_unsupported: bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRinexRepairAction {
    pub id: [c_char; RINEX_QC_CODE_C_BYTES],
    pub message: [c_char; RINEX_QC_FIELD_C_BYTES],
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_lint_obs(
    data: *const u8,
    len: usize,
    out_report: *mut *mut SidereonRinexLintReport,
) -> SidereonStatus {
    ffi_boundary("sidereon_rinex_lint_obs", SidereonStatus::Panic, || {
        let out_report = c_try!(require_out(
            out_report,
            "sidereon_rinex_lint_obs",
            "out_report"
        ));
        *out_report = ptr::null_mut();
        let text = c_try!(text_bytes_from_c("sidereon_rinex_lint_obs", data, len));
        let inner = sidereon_core::rinex::qc::lint_obs_text(text);
        write_boxed_handle(out_report, SidereonRinexLintReport { inner });
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_lint_nav(
    data: *const u8,
    len: usize,
    out_report: *mut *mut SidereonRinexLintReport,
) -> SidereonStatus {
    ffi_boundary("sidereon_rinex_lint_nav", SidereonStatus::Panic, || {
        let out_report = c_try!(require_out(
            out_report,
            "sidereon_rinex_lint_nav",
            "out_report"
        ));
        *out_report = ptr::null_mut();
        let text = c_try!(text_bytes_from_c("sidereon_rinex_lint_nav", data, len));
        let inner = sidereon_core::rinex::qc::lint_nav_text(text);
        write_boxed_handle(out_report, SidereonRinexLintReport { inner });
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_lint_summary(
    report: *const SidereonRinexLintReport,
    out_summary: *mut SidereonRinexLintSummary,
) -> SidereonStatus {
    ffi_boundary("sidereon_rinex_lint_summary", SidereonStatus::Panic, || {
        let out = c_try!(require_out(
            out_summary,
            "sidereon_rinex_lint_summary",
            "out_summary"
        ));
        let report = c_try!(require_ref(report, "sidereon_rinex_lint_summary", "report"));
        *out = rinex_lint_summary_to_c(&report.inner);
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_lint_findings(
    report: *const SidereonRinexLintReport,
    out: *mut SidereonRinexLintFinding,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_lint_findings",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rinex_lint_findings",
                out_written,
                out_required
            ));
            let report = c_try!(require_ref(
                report,
                "sidereon_rinex_lint_findings",
                "report"
            ));
            let values: Vec<_> = report
                .inner
                .findings
                .iter()
                .map(rinex_lint_finding_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_rinex_lint_findings",
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
pub unsafe extern "C" fn sidereon_rinex_lint_report_free(report: *mut SidereonRinexLintReport) {
    free_boxed(report);
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_repair_options_init(
    out_options: *mut SidereonRinexRepairOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_repair_options_init",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_options,
                "sidereon_rinex_repair_options_init",
                "out_options"
            ));
            let defaults = sidereon_core::rinex::qc::RepairOptions::default();
            *out = SidereonRinexRepairOptions {
                has_file_stamp: false,
                file_stamp_program: [0; RINEX_QC_FIELD_C_BYTES],
                file_stamp_run_by: [0; RINEX_QC_FIELD_C_BYTES],
                file_stamp_date: [0; RINEX_QC_FIELD_C_BYTES],
                set_interval: defaults.set_interval,
                set_time_of_last_obs: defaults.set_time_of_last_obs,
                set_obs_counts: defaults.set_obs_counts,
                drop_empty_records: defaults.drop_empty_records,
                sort_records: defaults.sort_records,
                drop_unsupported: defaults.drop_unsupported,
            };
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_repair_obs(
    data: *const u8,
    len: usize,
    options: *const SidereonRinexRepairOptions,
    out_repair: *mut *mut SidereonRinexRepair,
) -> SidereonStatus {
    ffi_boundary("sidereon_rinex_repair_obs", SidereonStatus::Panic, || {
        let out_repair = c_try!(require_out(
            out_repair,
            "sidereon_rinex_repair_obs",
            "out_repair"
        ));
        *out_repair = ptr::null_mut();
        let text = c_try!(text_bytes_from_c("sidereon_rinex_repair_obs", data, len));
        let options = c_try!(repair_options_from_c("sidereon_rinex_repair_obs", options));
        match sidereon_core::rinex::qc::repair_obs_text(text, &options) {
            Ok(repair) => {
                let crinex_text =
                    match sidereon_core::rinex::qc::repair_obs_to_crinex_string(&repair) {
                        Ok(text) => Some(text.into_bytes()),
                        Err(_) => None,
                    };
                let text = repair.repaired.to_rinex_string().into_bytes();
                write_boxed_handle(
                    out_repair,
                    SidereonRinexRepair {
                        text,
                        crinex_text,
                        actions: repair.actions,
                        remaining: repair.remaining,
                        decoded_from_crinex: repair.decoded_from_crinex,
                    },
                );
                SidereonStatus::Ok
            }
            Err(err) => {
                set_last_error(format!("sidereon_rinex_repair_obs: {err}"));
                SidereonStatus::InvalidArgument
            }
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_repair_nav(
    data: *const u8,
    len: usize,
    options: *const SidereonRinexRepairOptions,
    out_repair: *mut *mut SidereonRinexRepair,
) -> SidereonStatus {
    ffi_boundary("sidereon_rinex_repair_nav", SidereonStatus::Panic, || {
        let out_repair = c_try!(require_out(
            out_repair,
            "sidereon_rinex_repair_nav",
            "out_repair"
        ));
        *out_repair = ptr::null_mut();
        let text = c_try!(text_bytes_from_c("sidereon_rinex_repair_nav", data, len));
        let options = c_try!(repair_options_from_c("sidereon_rinex_repair_nav", options));
        match sidereon_core::rinex::qc::repair_nav_text(text, &options) {
            Ok(repair) => {
                let text = sidereon_core::rinex::nav::encode_nav(&repair.records).into_bytes();
                write_boxed_handle(
                    out_repair,
                    SidereonRinexRepair {
                        text,
                        crinex_text: None,
                        actions: repair.actions,
                        remaining: repair.remaining,
                        decoded_from_crinex: false,
                    },
                );
                SidereonStatus::Ok
            }
            Err(err) => {
                set_last_error(format!("sidereon_rinex_repair_nav: {err}"));
                SidereonStatus::InvalidArgument
            }
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_repair_text(
    repair: *const SidereonRinexRepair,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_rinex_repair_text", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_rinex_repair_text",
            out_written,
            out_required
        ));
        let repair = c_try!(require_ref(repair, "sidereon_rinex_repair_text", "repair"));
        c_try!(copy_prefix_to_c(
            "sidereon_rinex_repair_text",
            "out",
            &repair.text,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_repair_crinex_text(
    repair: *const SidereonRinexRepair,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_repair_crinex_text",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rinex_repair_crinex_text",
                out_written,
                out_required
            ));
            let repair = c_try!(require_ref(
                repair,
                "sidereon_rinex_repair_crinex_text",
                "repair"
            ));
            let Some(text) = repair.crinex_text.as_ref() else {
                set_last_error(
                    "sidereon_rinex_repair_crinex_text: no CRINEX output available".to_string(),
                );
                return SidereonStatus::InvalidArgument;
            };
            c_try!(copy_prefix_to_c(
                "sidereon_rinex_repair_crinex_text",
                "out",
                text,
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
pub unsafe extern "C" fn sidereon_rinex_repair_summary(
    repair: *const SidereonRinexRepair,
    out_summary: *mut SidereonRinexLintSummary,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_repair_summary",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_summary,
                "sidereon_rinex_repair_summary",
                "out_summary"
            ));
            let repair = c_try!(require_ref(
                repair,
                "sidereon_rinex_repair_summary",
                "repair"
            ));
            *out = rinex_lint_summary_to_c(&repair.remaining);
            out.decoded_from_crinex = repair.decoded_from_crinex;
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_repair_actions(
    repair: *const SidereonRinexRepair,
    out: *mut SidereonRinexRepairAction,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_repair_actions",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rinex_repair_actions",
                out_written,
                out_required
            ));
            let repair = c_try!(require_ref(
                repair,
                "sidereon_rinex_repair_actions",
                "repair"
            ));
            let values: Vec<_> = repair
                .actions
                .iter()
                .map(|action| SidereonRinexRepairAction {
                    id: fixed_c_chars::<RINEX_QC_CODE_C_BYTES>(action.id),
                    message: fixed_c_chars::<RINEX_QC_FIELD_C_BYTES>(&action.message),
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_rinex_repair_actions",
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
pub unsafe extern "C" fn sidereon_rinex_repair_free(repair: *mut SidereonRinexRepair) {
    free_boxed(repair);
}

/// Decode a CRINEX (Hatanaka-compressed) observation byte buffer into RINEX
/// observation text. The output is not null-terminated. Uses the variable-length
/// output contract documented at the top of the header: call once with out=NULL
/// to learn *out_required, then again with a buffer of that size. The decoded
/// text is byte-for-byte what crx2rnx produces.
///
/// Safety: data must point to len readable bytes; out must point to at least
/// out_len writable bytes or be NULL when out_len is 0; out_written and
/// out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_crinex_decode(
    data: *const u8,
    len: usize,
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_crinex_decode", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_crinex_decode",
            out_written,
            out_required
        ));
        let bytes = c_try!(require_slice(data, len, "sidereon_crinex_decode", "data"));
        let text = match str::from_utf8(bytes) {
            Ok(text) => text,
            Err(_) => {
                set_last_error("sidereon_crinex_decode: data is not valid UTF-8".to_string());
                return SidereonStatus::InvalidToken;
            }
        };
        let decoded = match crinex_decode(text) {
            Ok(decoded) => decoded,
            Err(err) => {
                set_last_error(format!("sidereon_crinex_decode: {err}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        c_try!(copy_prefix_to_c(
            "sidereon_crinex_decode",
            "out",
            decoded.as_bytes(),
            out,
            out_len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

// --- CRINEX encode (sidereon_core::crinex::encode_crinex) --------------------

/// Encode RINEX observation text into CRINEX (Hatanaka-compressed) text. The
/// output is not null-terminated and is byte-for-byte what rnx2crx produces. Uses
/// the variable-length output contract. Delegates to
/// sidereon_core::crinex::encode_crinex.
///
/// Safety: data points to len readable bytes; out points to at least out_len
/// writable bytes or NULL when out_len is 0; out_written and out_required point to
/// size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_crinex_encode(
    data: *const u8,
    len: usize,
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_crinex_encode", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_crinex_encode",
            out_written,
            out_required
        ));
        let bytes = c_try!(require_slice(data, len, "sidereon_crinex_encode", "data"));
        let text = match str::from_utf8(bytes) {
            Ok(text) => text,
            Err(_) => {
                set_last_error("sidereon_crinex_encode: data is not valid UTF-8".to_string());
                return SidereonStatus::InvalidToken;
            }
        };
        let encoded = match sidereon_core::rinex::crinex::encode_crinex(text) {
            Ok(encoded) => encoded,
            Err(err) => {
                set_last_error(format!("sidereon_crinex_encode: {err}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        c_try!(copy_prefix_to_c(
            "sidereon_crinex_encode",
            "out",
            encoded.as_bytes(),
            out,
            out_len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

fn rinex_obs_kind_to_c(kind: RinexObservationKind) -> u32 {
    match kind {
        RinexObservationKind::Pseudorange => SidereonRinexObsKind::Pseudorange as u32,
        RinexObservationKind::CarrierPhase => SidereonRinexObsKind::CarrierPhase as u32,
        RinexObservationKind::Doppler => SidereonRinexObsKind::Doppler as u32,
        RinexObservationKind::SignalStrength => SidereonRinexObsKind::SignalStrength as u32,
        RinexObservationKind::Unknown => SidereonRinexObsKind::Unknown as u32,
    }
}

fn rinex_obs_code_to_c(code: &str) -> [c_char; RINEX_OBS_CODE_C_BYTES] {
    fixed_c_chars::<RINEX_OBS_CODE_C_BYTES>(code)
}

fn empty_rinex_obs_header() -> SidereonRinexObsHeader {
    SidereonRinexObsHeader {
        version: 0.0,
        has_approx_position_m: false,
        approx_position_m: [0.0; 3],
        has_antenna_delta_hen_m: false,
        antenna_delta_hen_m: [0.0; 3],
        has_interval_s: false,
        interval_s: 0.0,
        has_time_of_first_obs: false,
        time_of_first_obs: SidereonCalendarEpoch {
            year: 0,
            month: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0.0,
        },
        time_of_first_obs_scale: SidereonTimeScale::Utc as u32,
        obs_code_count: 0,
        phase_shift_count: 0,
        scale_factor_count: 0,
        glonass_slot_count: 0,
        has_marker_name: false,
        marker_name: [0; RINEX_OBS_MARKER_C_BYTES],
    }
}

fn rinex_obs_error(fn_name: &str, err: CoreError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}

fn rinex_lint_summary_to_c(
    report: &sidereon_core::rinex::qc::LintReport,
) -> SidereonRinexLintSummary {
    use sidereon_core::rinex::qc::Severity;
    SidereonRinexLintSummary {
        finding_count: report.findings.len(),
        fatal_count: report.count(Severity::Fatal),
        error_count: report.count(Severity::Error),
        warning_count: report.count(Severity::Warning),
        info_count: report.count(Severity::Info),
        is_clean: report.is_clean(),
        decoded_from_crinex: report.decoded_from_crinex,
    }
}

fn rinex_lint_finding_to_c(
    finding: &sidereon_core::rinex::qc::Finding,
) -> SidereonRinexLintFinding {
    let at = finding.at();
    SidereonRinexLintFinding {
        code: fixed_c_chars::<RINEX_QC_CODE_C_BYTES>(finding.code()),
        severity: rinex_qc_severity_to_c(finding.severity()),
        repairable: finding.is_repairable(),
        has_epoch_index: at.epoch_index.is_some(),
        epoch_index: at.epoch_index.unwrap_or(0),
        has_satellite: at.satellite.is_some(),
        satellite: at
            .satellite
            .as_deref()
            .map(satellite_token_from_text)
            .unwrap_or_else(observation_qc_signal_empty_sat),
        has_field: at.field.is_some(),
        field: fixed_c_chars::<RINEX_QC_FIELD_C_BYTES>(at.field.unwrap_or("")),
    }
}

fn repair_options_from_c(
    fn_name: &str,
    options: *const SidereonRinexRepairOptions,
) -> Result<sidereon_core::rinex::qc::RepairOptions, SidereonStatus> {
    let Some(options) = (unsafe { options.as_ref() }) else {
        return Ok(sidereon_core::rinex::qc::RepairOptions::default());
    };
    let file_stamp = if options.has_file_stamp {
        Some(sidereon_core::rinex::observations::PgmRunByDate {
            program: fixed_c_array_to_string(
                fn_name,
                "file_stamp_program",
                &options.file_stamp_program,
            )?,
            run_by: fixed_c_array_to_string(
                fn_name,
                "file_stamp_run_by",
                &options.file_stamp_run_by,
            )?,
            date: fixed_c_array_to_string(fn_name, "file_stamp_date", &options.file_stamp_date)?,
        })
    } else {
        None
    };
    Ok(sidereon_core::rinex::qc::RepairOptions {
        file_stamp,
        set_interval: options.set_interval,
        set_time_of_last_obs: options.set_time_of_last_obs,
        set_obs_counts: options.set_obs_counts,
        drop_empty_records: options.drop_empty_records,
        sort_records: options.sort_records,
        drop_unsupported: options.drop_unsupported,
    })
}

fn rinex_qc_severity_to_c(severity: sidereon_core::rinex::qc::Severity) -> u32 {
    match severity {
        sidereon_core::rinex::qc::Severity::Fatal => SidereonRinexQcSeverity::Fatal as u32,
        sidereon_core::rinex::qc::Severity::Error => SidereonRinexQcSeverity::Error as u32,
        sidereon_core::rinex::qc::Severity::Warning => SidereonRinexQcSeverity::Warning as u32,
        sidereon_core::rinex::qc::Severity::Info => SidereonRinexQcSeverity::Info as u32,
    }
}

#[cfg(test)]
mod rinex_parity_tests {
    use super::*;
    use std::ffi::CString;

    const NAV_FIXTURE: &str =
        include_str!("../tests/fixtures/nav/ESBC00DNK_R_20201770000_01D_MN.rnx");
    const CLOCK_FIXTURE: &[u8] = include_bytes!("../tests/fixtures/clk/synthetic_rinex_clock.clk");
    const SBAS_HEX: &str = "0000000000000000000000000000000000000000000000000000000000";
    const SBAS_FRAME_HEX: &str = "0000000000000000000000000000000000000000000000000000000000000000";
    const GLONASS_NAV: &str = "     3.05           NAVIGATION DATA     M                   RINEX VERSION / TYPE\n     XXX                                                         END OF HEADER\nR01 2020 06 24 23 15 00 6.355904042721e-05 0.000000000000e+00 3.420000000000e+05\n     1.090894238281e+04 1.407806396484e+00-1.862645149231e-09 0.000000000000e+00\n    -2.885726074219e+03 2.795855522156e+00-0.000000000000e+00 1.000000000000e+00\n     2.288353955078e+04-3.169984817505e-01-2.793967723846e-09 0.000000000000e+00\n";

    fn token_text(token: SidereonSatelliteToken) -> String {
        let end = token
            .bytes
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(token.bytes.len());
        String::from_utf8(token.bytes[..end].iter().map(|byte| *byte as u8).collect())
            .expect("C token is UTF-8")
    }

    #[test]
    fn auxiliary_parsers_keep_distinctions_and_map_errors() {
        unsafe {
            let mut nav = ptr::null_mut();
            assert_eq!(
                sidereon_parse_rinex_nav_lenient(b"not nav".as_ptr(), 7, &mut nav),
                SidereonStatus::InvalidArgument
            );
            assert!(nav.is_null());

            let mut iono = empty_iono_corrections();
            assert_eq!(
                sidereon_parse_rinex_iono_corrections(
                    NAV_FIXTURE.as_ptr(),
                    NAV_FIXTURE.len(),
                    &mut iono
                ),
                SidereonStatus::Ok
            );
            assert!(iono.gps.present);
            assert!(iono.galileo.present);

            let mut leap = 0.0;
            let mut present = false;
            assert_eq!(
                sidereon_parse_rinex_leap_seconds(
                    NAV_FIXTURE.as_ptr(),
                    NAV_FIXTURE.len(),
                    &mut leap,
                    &mut present
                ),
                SidereonStatus::Ok
            );
            assert!(present);
            assert_eq!(leap, 18.0);

            let empty_header = b"     3.05           NAVIGATION DATA     MIXED               RINEX VERSION / TYPE\n                                                            END OF HEADER\n";
            assert_eq!(
                sidereon_parse_rinex_leap_seconds(
                    empty_header.as_ptr(),
                    empty_header.len(),
                    &mut leap,
                    &mut present
                ),
                SidereonStatus::Ok
            );
            assert!(!present);

            let ems = format!("120,26,7,1,0,0,1,1,{SBAS_FRAME_HEX}\n");
            let rtklib = format!("2360 259200 120 1 : {SBAS_HEX}\n");
            let mut ems_blocks = ptr::null_mut();
            let mut rtklib_blocks = ptr::null_mut();
            assert_eq!(
                sidereon_parse_sbas_ems_lines(ems.as_ptr(), ems.len(), &mut ems_blocks),
                SidereonStatus::Ok
            );
            assert_eq!(
                sidereon_parse_sbas_rtklib_lines(rtklib.as_ptr(), rtklib.len(), &mut rtklib_blocks),
                SidereonStatus::Ok
            );
            let mut count = 0;
            assert_eq!(
                sidereon_sbas_log_blocks_count(ems_blocks, &mut count),
                SidereonStatus::Ok
            );
            assert_eq!(count, 1);
            let mut block = empty_sbas_log_block();
            assert_eq!(
                sidereon_sbas_log_blocks_item(ems_blocks, 0, &mut block),
                SidereonStatus::Ok
            );
            assert_eq!(token_text(block.sat_id), "S20");
            assert_eq!(block.form, SidereonSbasWireForm::Framed250);
            assert_eq!(block.byte_count, 32);
            let mut rtklib_block = empty_sbas_log_block();
            assert_eq!(
                sidereon_sbas_log_blocks_item(rtklib_blocks, 0, &mut rtklib_block),
                SidereonStatus::Ok
            );
            assert_eq!(rtklib_block.form, SidereonSbasWireForm::Body226);
            sidereon_sbas_log_blocks_free(ems_blocks);
            sidereon_sbas_log_blocks_free(rtklib_blocks);

            let mut empty_blocks = ptr::null_mut();
            assert_eq!(
                sidereon_parse_sbas_ems_lines(b"not,enough".as_ptr(), 10, &mut empty_blocks),
                SidereonStatus::Ok
            );
            assert!(!empty_blocks.is_null());
            let mut empty_count = 1;
            sidereon_sbas_log_blocks_count(empty_blocks, &mut empty_count);
            assert_eq!(empty_count, 0);
            sidereon_sbas_log_blocks_free(empty_blocks);
        }
    }

    #[test]
    fn path_nav_loader_keeps_store_and_leap_seconds_on_one_source() {
        struct RemoveFile(std::path::PathBuf);

        impl Drop for RemoveFile {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
            }
        }

        let path = std::env::temp_dir().join(format!(
            "sidereon-c-rinex-nav-loader-{}.rnx",
            std::process::id()
        ));
        std::fs::write(&path, NAV_FIXTURE).expect("write NAV loader fixture");
        let _remove_file = RemoveFile(path.clone());
        let path = CString::new(path.to_str().expect("temporary path is UTF-8"))
            .expect("temporary path has no NUL");

        unsafe {
            let mut broadcast = ptr::null_mut();
            assert_eq!(
                sidereon_broadcast_ephemeris_load_nav(path.as_ptr(), &mut broadcast),
                SidereonStatus::Ok
            );
            assert!(!broadcast.is_null());
            let mut leap_seconds = 0.0;
            let mut present = false;
            assert_eq!(
                sidereon_broadcast_ephemeris_leap_seconds(
                    broadcast,
                    &mut leap_seconds,
                    &mut present
                ),
                SidereonStatus::Ok
            );
            assert!(present);
            assert_eq!(leap_seconds, 18.0);
            sidereon_broadcast_ephemeris_free(broadcast);

            let invalid_path = std::env::temp_dir().join(format!(
                "sidereon-c-rinex-nav-loader-invalid-{}.rnx",
                std::process::id()
            ));
            std::fs::write(&invalid_path, [0xff, 0xfe]).expect("write invalid UTF-8 fixture");
            let _remove_invalid_file = RemoveFile(invalid_path.clone());
            let invalid_path =
                CString::new(invalid_path.to_str().expect("temporary path is UTF-8"))
                    .expect("temporary path has no NUL");
            let sentinel = std::ptr::NonNull::<SidereonBroadcastEphemeris>::dangling().as_ptr();
            let mut broadcast = sentinel;
            assert_eq!(
                sidereon_broadcast_ephemeris_load_nav(invalid_path.as_ptr(), &mut broadcast),
                SidereonStatus::InvalidToken
            );
            assert!(broadcast.is_null());

            let mut message = [0 as c_char; 128];
            sidereon_last_error_message(message.as_mut_ptr(), message.len());
            let message = CStr::from_ptr(message.as_ptr())
                .to_str()
                .expect("last error is UTF-8");
            assert_eq!(
                message,
                "sidereon_broadcast_ephemeris_load_nav: source is not valid UTF-8"
            );
        }
    }

    #[test]
    fn raw_records_round_trip_and_rich_store_preserve_fields() {
        unsafe {
            let nav_with_glonass = format!("{NAV_FIXTURE}{GLONASS_NAV}");
            let mut records = ptr::null_mut();
            assert_eq!(
                sidereon_parse_rinex_nav_records(
                    NAV_FIXTURE.as_ptr(),
                    NAV_FIXTURE.len(),
                    &mut records
                ),
                SidereonStatus::Ok
            );
            let mut record_count = 0;
            assert_eq!(
                sidereon_rinex_nav_records_count(records, &mut record_count),
                SidereonStatus::Ok
            );
            assert!(record_count > 0);
            assert!(record_count > 1);
            let mut record = empty_broadcast_record();
            assert_eq!(
                sidereon_rinex_nav_records_item(records, 0, &mut record),
                SidereonStatus::Ok
            );
            assert!(record.elements.sqrt_a.is_finite());
            assert_ne!(record.sat_id.bytes[0], 0);

            let mut written = 0;
            let mut required = 0;
            assert_eq!(
                sidereon_encode_rinex_nav(
                    &record,
                    1,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, 0);
            assert!(required > 0);
            let mut encoded = vec![0_u8; required];
            assert_eq!(
                sidereon_encode_rinex_nav(
                    &record,
                    1,
                    encoded.as_mut_ptr(),
                    encoded.len(),
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, encoded.len());
            let mut reparsed = ptr::null_mut();
            assert_eq!(
                sidereon_parse_rinex_nav_records(encoded.as_ptr(), encoded.len(), &mut reparsed),
                SidereonStatus::Ok
            );
            let mut reparsed_count = 0;
            sidereon_rinex_nav_records_count(reparsed, &mut reparsed_count);
            assert_eq!(reparsed_count, 1);
            sidereon_rinex_nav_records_free(reparsed);
            sidereon_rinex_nav_records_free(records);

            let mut glonass_records = ptr::null_mut();
            assert_eq!(
                sidereon_parse_rinex_glonass_records(
                    GLONASS_NAV.as_ptr(),
                    GLONASS_NAV.len(),
                    &mut glonass_records
                ),
                SidereonStatus::Ok
            );
            let mut standalone_glonass_count = 0;
            sidereon_rinex_glonass_records_count(glonass_records, &mut standalone_glonass_count);
            assert_eq!(standalone_glonass_count, 1);
            let mut standalone_glonass = empty_glonass_record();
            sidereon_rinex_glonass_records_item(glonass_records, 0, &mut standalone_glonass);
            assert_eq!(token_text(standalone_glonass.sat_id), "R01");
            assert_eq!(standalone_glonass.freq_channel, 1);
            sidereon_rinex_glonass_records_free(glonass_records);

            let extended_glonass = GLONASS_NAV.replacen("R01 ", "R28 ", 1);
            let mut extended_records = ptr::null_mut();
            assert_eq!(
                sidereon_parse_rinex_glonass_records(
                    extended_glonass.as_ptr(),
                    extended_glonass.len(),
                    &mut extended_records
                ),
                SidereonStatus::Ok
            );
            let mut extended_record_count = usize::MAX;
            assert_eq!(
                sidereon_rinex_glonass_records_count(extended_records, &mut extended_record_count),
                SidereonStatus::Ok
            );
            assert_eq!(extended_record_count, 0);
            let mut skipped_count = usize::MAX;
            assert_eq!(
                sidereon_rinex_glonass_records_skipped_count(extended_records, &mut skipped_count),
                SidereonStatus::Ok
            );
            assert_eq!(skipped_count, 1);
            let mut skipped = SidereonSkippedGlonassRecord {
                satellite: satellite_token_from_text(""),
            };
            assert_eq!(
                sidereon_rinex_glonass_records_skipped_item(extended_records, 0, &mut skipped),
                SidereonStatus::Ok
            );
            assert_eq!(token_text(skipped.satellite), "R28");
            sidereon_rinex_glonass_records_free(extended_records);

            let mut broadcast = ptr::null_mut();
            assert_eq!(
                sidereon_broadcast_ephemeris_parse_nav(
                    nav_with_glonass.as_ptr(),
                    nav_with_glonass.len(),
                    &mut broadcast
                ),
                SidereonStatus::Ok
            );
            let mut full_count = 0;
            let mut glonass_count = 0;
            let mut channel_count = 0;
            sidereon_broadcast_ephemeris_record_count(broadcast, &mut full_count);
            sidereon_broadcast_ephemeris_glonass_record_count(broadcast, &mut glonass_count);
            sidereon_broadcast_ephemeris_glonass_frequency_channel_count(
                broadcast,
                &mut channel_count,
            );
            assert!(full_count > 0);
            assert!(record_count >= full_count);
            assert!(glonass_count > 0);
            assert_eq!(channel_count, glonass_count);
            let mut rich = empty_broadcast_record();
            let mut rich_written = 0;
            let mut rich_required = 0;
            assert_eq!(
                sidereon_broadcast_ephemeris_records_full(
                    broadcast,
                    &mut rich,
                    1,
                    &mut rich_written,
                    &mut rich_required
                ),
                SidereonStatus::InvalidArgument
            );
            assert_eq!(rich_required, full_count);
            let mut rich_records = vec![empty_broadcast_record(); full_count];
            assert_eq!(
                sidereon_broadcast_ephemeris_records_full(
                    broadcast,
                    rich_records.as_mut_ptr(),
                    rich_records.len(),
                    &mut rich_written,
                    &mut rich_required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(rich_written, full_count);
            assert_eq!(rich_records[0].sat_id.bytes, record.sat_id.bytes);
            assert!(rich_records[0].elements.sqrt_a.is_finite());

            let mut rich_glonass = vec![empty_glonass_record(); glonass_count];
            assert_eq!(
                sidereon_broadcast_ephemeris_glonass_records(
                    broadcast,
                    rich_glonass.as_mut_ptr(),
                    rich_glonass.len(),
                    &mut rich_written,
                    &mut rich_required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(rich_written, glonass_count);
            assert_eq!(token_text(rich_glonass[0].sat_id), "R01");
            assert_eq!(rich_glonass[0].freq_channel, 1);
            assert!(rich_glonass[0].pos_m[0].is_finite());

            let mut channels = vec![
                SidereonFrequencyChannel {
                    slot: 0,
                    channel: 0,
                };
                channel_count
            ];
            assert_eq!(
                sidereon_broadcast_ephemeris_glonass_frequency_channels(
                    broadcast,
                    channels.as_mut_ptr(),
                    channels.len(),
                    &mut rich_written,
                    &mut rich_required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(rich_written, channel_count);
            assert_eq!(channels[0].slot, 1);
            assert_eq!(channels[0].channel, 1);

            let mut iono = empty_iono_corrections();
            sidereon_broadcast_ephemeris_iono_corrections(broadcast, &mut iono);
            assert!(iono.gps.present);
            let mut rich_leap = 0.0;
            let mut rich_present = false;
            sidereon_broadcast_ephemeris_leap_seconds(broadcast, &mut rich_leap, &mut rich_present);
            assert!(rich_present);
            assert_eq!(rich_leap, 18.0);
            sidereon_broadcast_ephemeris_free(broadcast);
        }
    }

    #[test]
    fn lenient_diagnostics_and_lossy_clock_samples_are_owned() {
        unsafe {
            let source = NAV_FIXTURE
                .lines()
                .find(|line| {
                    let bytes = line.as_bytes();
                    bytes.len() > 1 && bytes[0].is_ascii_alphabetic() && bytes[1].is_ascii_digit()
                })
                .expect("fixture has a NAV record");
            let bad_source = source.replacen("2020", "XXXX", 1);
            let bad_nav = NAV_FIXTURE.replacen(source, &bad_source, 1);
            let mut parse = ptr::null_mut();
            assert_eq!(
                sidereon_parse_rinex_nav_lenient(bad_nav.as_ptr(), bad_nav.len(), &mut parse),
                SidereonStatus::Ok
            );
            let mut skipped_count = 0;
            sidereon_nav_parse_skipped_count(parse, &mut skipped_count);
            assert!(skipped_count > 0);
            let mut skipped = SidereonSkippedNavBlock {
                satellite: satellite_token_from_text(""),
                message: [0; 256],
            };
            sidereon_nav_parse_skipped(parse, 0, &mut skipped);
            assert_eq!(token_text(skipped.satellite), &source[..3]);
            let mut message_written = 0;
            let mut message_required = 0;
            assert_eq!(
                sidereon_nav_parse_skipped_message(
                    parse,
                    0,
                    ptr::null_mut(),
                    0,
                    &mut message_written,
                    &mut message_required
                ),
                SidereonStatus::Ok
            );
            assert!(message_required > 0);
            sidereon_nav_parse_free(parse);

            let mut clock = ptr::null_mut();
            assert_eq!(
                sidereon_rinex_clock_parse_lossy(
                    CLOCK_FIXTURE.as_ptr(),
                    CLOCK_FIXTURE.len(),
                    &mut clock
                ),
                SidereonStatus::Ok
            );
            let mut satellite_count = 0;
            let mut sample_count = 0;
            sidereon_rinex_clock_series_count(clock, &mut satellite_count);
            sidereon_rinex_clock_sample_count(clock, &mut sample_count);
            assert_eq!(satellite_count, 2);
            assert_eq!(sample_count, 5);
            let mut satellites = [SidereonSatelliteToken { bytes: [0; 17] }; 2];
            let mut sat_written = 0;
            let mut sat_required = 0;
            assert_eq!(
                sidereon_rinex_clock_satellites(
                    clock,
                    satellites.as_mut_ptr(),
                    satellites.len(),
                    &mut sat_written,
                    &mut sat_required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(sat_written, 2);
            assert_eq!(token_text(satellites[0]), "G05");
            let satellite = CString::new("G05").expect("satellite");
            let mut series = ptr::null_mut();
            assert_eq!(
                sidereon_rinex_clock_series_for(clock, satellite.as_ptr(), &mut series),
                SidereonStatus::Ok
            );
            let mut series_count = 0;
            sidereon_rinex_clock_series_sample_count(series, &mut series_count);
            assert_eq!(series_count, 3);
            let mut samples = vec![
                SidereonClockPoint {
                    epoch: SidereonClockEpoch {
                        scale: 0,
                        representation: 0,
                        jd_whole: 0.0,
                        jd_fraction: 0.0,
                        nanos_high: 0,
                        nanos_low: 0,
                    },
                    bias_s: 0.0,
                };
                3
            ];
            let mut sample_written = 0;
            let mut sample_required = 0;
            assert_eq!(
                sidereon_rinex_clock_series_samples(
                    series,
                    samples.as_mut_ptr(),
                    samples.len(),
                    &mut sample_written,
                    &mut sample_required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(samples[1].epoch.scale, SidereonTimeScale::Gpst as u32);
            assert_eq!(
                samples[1].epoch.representation,
                SidereonRinexClockInstantRepresentation::JulianDate as u32
            );
            assert_eq!(samples[0].epoch.jd_whole, 2461173.5);
            assert_eq!(samples[0].epoch.jd_fraction, 0.0);
            assert_eq!(samples[1].epoch.jd_whole, 2461173.5);
            assert_eq!(samples[1].epoch.jd_fraction, 30.0 / 86_400.0);
            assert_eq!(samples[1].bias_s.to_bits(), (-2.0000006e-4_f64).to_bits());
            sidereon_rinex_clock_series_free(series);
            sidereon_rinex_clock_free(clock);

            let malformed = b"AS G05  2026 05 13 00 00  bad-second  1   2.0e-04\n";
            let mut malformed_clock = ptr::null_mut();
            assert_eq!(
                sidereon_rinex_clock_parse_lossy(
                    malformed.as_ptr(),
                    malformed.len(),
                    &mut malformed_clock
                ),
                SidereonStatus::Ok
            );
            sidereon_rinex_clock_sample_count(malformed_clock, &mut sample_count);
            assert_eq!(sample_count, 0);
            sidereon_rinex_clock_free(malformed_clock);
        }
    }
}
