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

// --- RINEX clock (sidereon_core::rinex::clock) -------------------------------

/// A parsed RINEX clock product. Opaque to C. Create with
/// sidereon_rinex_clock_parse; release with sidereon_rinex_clock_free.
pub struct SidereonRinexClock {
    pub(crate) inner: sidereon_core::rinex::clock::RinexClock,
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
