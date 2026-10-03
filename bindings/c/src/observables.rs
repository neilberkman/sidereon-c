use super::*;

// ===========================================================================

/// Predictor options. Initialize with sidereon_observables_options_init for the
/// engine defaults (L1 carrier, light-time and Sagnac corrections on).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonObservablesOptions {
    /// Carrier frequency in hertz used for the Doppler conversion.
    pub carrier_hz: f64,
    /// Apply fixed-point light-time correction in the geometry substrate.
    pub light_time: bool,
    /// Apply Earth-rotation Sagnac correction in the geometry substrate.
    pub sagnac: bool,
}

/// One satellite's predicted observables at one epoch.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPredictedObservables {
    /// Geometric range, meters.
    pub geometric_range_m: f64,
    /// Range rate (positive receding), meters per second.
    pub range_rate_m_s: f64,
    /// Doppler shift at options.carrier_hz, hertz.
    pub doppler_hz: f64,
    /// Whether sat_clock_s is present.
    pub has_sat_clock_s: bool,
    /// Satellite clock offset, seconds, when present.
    pub sat_clock_s: f64,
    /// Topocentric elevation, degrees.
    pub elevation_deg: f64,
    /// Topocentric azimuth, degrees in [0, 360).
    pub azimuth_deg: f64,
    /// Transmit-time offset from the receive epoch, microseconds.
    pub transmit_offset_us: i64,
    /// Transmit time, seconds since J2000.
    pub transmit_time_j2000_s: f64,
    /// ECEF line-of-sight unit vector (receiver toward satellite).
    pub los_unit: [f64; 3],
    /// Satellite ECEF position, meters.
    pub sat_pos_ecef_m: [f64; 3],
    /// Satellite ECEF velocity, meters per second.
    pub sat_velocity_m_s: [f64; 3],
}

/// Per-satellite status for an emission-epoch state and media batch row.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonEmissionMediaStatus {
    /// The row contains state, clock, ionosphere, and troposphere outputs.
    Valid = 0,
    /// The ephemeris product has no usable state for this satellite and epoch.
    Gap = 1,
    /// The row had a state, but its elevation was below the requested cutoff.
    BelowElevationCutoff = 2,
    /// The scalar evaluator returned a non-gap error.
    Error = 3,
}

/// Options for one-call emission-epoch state and media correction batches.
/// Initialize with sidereon_emission_media_options_init for engine defaults.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonEmissionMediaOptions {
    /// Carrier frequency used for ionospheric group delay, hertz.
    pub carrier_hz: f64,
    /// Whether min_elevation_rad is applied.
    pub min_elevation_enabled: bool,
    /// Optional minimum topocentric elevation, radians.
    pub min_elevation_rad: f64,
    /// Whether troposphere correction is enabled.
    pub troposphere_enabled: bool,
    /// Surface meteorology for troposphere correction.
    pub met: SidereonMet,
    /// Optional IONEX handle for ionosphere correction. NULL disables IONEX.
    pub ionex: *const SidereonIonex,
    /// Whether ionex_policy applies to the IONEX correction. False evaluates
    /// the IONEX product under the engine default policy, the one
    /// sidereon_ionex_slant_policy_default returns, so a zero-filled options
    /// struct keeps that default.
    pub ionex_policy_enabled: bool,
    /// Coverage, missing-node and mapping policy for the IONEX correction,
    /// when ionex_policy_enabled is true. An unrecognized tag fails the call
    /// with SIDEREON_STATUS_INVALID_ARGUMENT.
    pub ionex_policy: SidereonIonexSlantPolicy,
}

/// Populate *out_options with the engine's default predictor options (L1
/// carrier, light-time and Sagnac corrections on).
///
/// Safety: out_options must point to a SidereonObservablesOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_observables_options_init(
    out_options: *mut SidereonObservablesOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_observables_options_init",
        SidereonStatus::Panic,
        || {
            let out_options = c_try!(require_out(
                out_options,
                "sidereon_observables_options_init",
                "out_options"
            ));
            let defaults = PredictOptions::default();
            *out_options = SidereonObservablesOptions {
                carrier_hz: defaults.carrier_hz,
                light_time: defaults.light_time,
                sagnac: defaults.sagnac,
            };
            SidereonStatus::Ok
        },
    )
}

/// Copy the observable-state missing-position sentinel into out. The sentinel is
/// three NaN components and is also written for every failed batch element.
///
/// Safety: out_position_ecef_m must point to at least len doubles; len must be
/// at least 3.
#[no_mangle]
pub unsafe extern "C" fn sidereon_observable_state_missing_position_ecef_m(
    out_position_ecef_m: *mut f64,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_observable_state_missing_position_ecef_m",
        SidereonStatus::Panic,
        || {
            c_try!(copy_exact_f64s(
                "sidereon_observable_state_missing_position_ecef_m",
                "out_position_ecef_m",
                out_position_ecef_m,
                len,
                &OBSERVABLE_STATE_MISSING_POSITION_ECEF_M,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Populate *out_options with emission media batch defaults.
///
/// Safety: out_options must point to a SidereonEmissionMediaOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_emission_media_options_init(
    out_options: *mut SidereonEmissionMediaOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_emission_media_options_init",
        SidereonStatus::Panic,
        || {
            let out_options = c_try!(require_out(
                out_options,
                "sidereon_emission_media_options_init",
                "out_options"
            ));
            let met = SurfaceMet::default();
            *out_options = SidereonEmissionMediaOptions {
                carrier_hz: PredictOptions::default().carrier_hz,
                min_elevation_enabled: false,
                min_elevation_rad: 0.0,
                troposphere_enabled: false,
                met: SidereonMet {
                    pressure_hpa: met.pressure_hpa,
                    temperature_k: met.temperature_k,
                    relative_humidity: met.relative_humidity,
                },
                ionex: ptr::null(),
                ionex_policy_enabled: false,
                ionex_policy: sidereon_ionex_slant_policy_default(),
            };
            SidereonStatus::Ok
        },
    )
}

/// Evaluate emission-epoch states, clocks, and media delays from a loaded SP3
/// product in one call. Output arrays are index-aligned with satellites.
///
/// Safety: sp3 must be a live handle; satellites and emission_epochs_j2000_s
/// point to count entries; receiver_ecef_m points to three doubles; options may
/// be NULL for defaults. Position output points to count*3 doubles; all other
/// output arrays point to count entries.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_emission_media_batch_at_j2000_s(
    sp3: *const SidereonSp3,
    satellites: *const *const c_char,
    emission_epochs_j2000_s: *const f64,
    count: usize,
    receiver_ecef_m: *const f64,
    options: *const SidereonEmissionMediaOptions,
    out_positions_ecef_m: *mut f64,
    out_has_positions: *mut bool,
    out_clocks_s: *mut f64,
    out_has_clocks_s: *mut bool,
    out_ionosphere_slant_delays_m: *mut f64,
    out_has_ionosphere_slant_delays_m: *mut bool,
    out_troposphere_delays_m: *mut f64,
    out_has_troposphere_delays_m: *mut bool,
    out_statuses: *mut SidereonEmissionMediaStatus,
    out_result_statuses: *mut SidereonStatus,
) -> SidereonStatus {
    crate::engine_error::observables_error_operation_boundary(
        "sidereon_sp3_emission_media_batch_at_j2000_s",
        SidereonStatus::Panic,
        || {
            let sp3 = c_try!(require_ref(
                sp3,
                "sidereon_sp3_emission_media_batch_at_j2000_s",
                "sp3"
            ));
            emission_media_batch_common(
                "sidereon_sp3_emission_media_batch_at_j2000_s",
                &sp3.inner,
                satellites,
                emission_epochs_j2000_s,
                count,
                receiver_ecef_m,
                options,
                out_positions_ecef_m,
                out_has_positions,
                out_clocks_s,
                out_has_clocks_s,
                out_ionosphere_slant_delays_m,
                out_has_ionosphere_slant_delays_m,
                out_troposphere_delays_m,
                out_has_troposphere_delays_m,
                out_statuses,
                out_result_statuses,
            )
        },
    )
}

/// Evaluate emission-epoch states, clocks, and media delays from broadcast
/// ephemeris in one call. Output arrays are index-aligned with satellites.
///
/// Safety: broadcast must be a live handle; satellites and
/// emission_epochs_j2000_s point to count entries; receiver_ecef_m points to
/// three doubles; options may be NULL for defaults. Position output points to
/// count*3 doubles; all other output arrays point to count entries.
#[no_mangle]
pub unsafe extern "C" fn sidereon_broadcast_emission_media_batch_at_j2000_s(
    broadcast: *const SidereonBroadcastEphemeris,
    satellites: *const *const c_char,
    emission_epochs_j2000_s: *const f64,
    count: usize,
    receiver_ecef_m: *const f64,
    options: *const SidereonEmissionMediaOptions,
    out_positions_ecef_m: *mut f64,
    out_has_positions: *mut bool,
    out_clocks_s: *mut f64,
    out_has_clocks_s: *mut bool,
    out_ionosphere_slant_delays_m: *mut f64,
    out_has_ionosphere_slant_delays_m: *mut bool,
    out_troposphere_delays_m: *mut f64,
    out_has_troposphere_delays_m: *mut bool,
    out_statuses: *mut SidereonEmissionMediaStatus,
    out_result_statuses: *mut SidereonStatus,
) -> SidereonStatus {
    crate::engine_error::observables_error_operation_boundary(
        "sidereon_broadcast_emission_media_batch_at_j2000_s",
        SidereonStatus::Panic,
        || {
            let broadcast = c_try!(require_ref(
                broadcast,
                "sidereon_broadcast_emission_media_batch_at_j2000_s",
                "broadcast"
            ));
            emission_media_batch_common(
                "sidereon_broadcast_emission_media_batch_at_j2000_s",
                &broadcast.inner,
                satellites,
                emission_epochs_j2000_s,
                count,
                receiver_ecef_m,
                options,
                out_positions_ecef_m,
                out_has_positions,
                out_clocks_s,
                out_has_clocks_s,
                out_ionosphere_slant_delays_m,
                out_has_ionosphere_slant_delays_m,
                out_troposphere_delays_m,
                out_has_troposphere_delays_m,
                out_statuses,
                out_result_statuses,
            )
        },
    )
}

#[allow(clippy::too_many_arguments)]
unsafe fn emission_media_batch_common(
    fn_name: &str,
    source: &dyn ObservableEphemerisSource,
    satellites: *const *const c_char,
    emission_epochs_j2000_s: *const f64,
    count: usize,
    receiver_ecef_m: *const f64,
    options: *const SidereonEmissionMediaOptions,
    out_positions_ecef_m: *mut f64,
    out_has_positions: *mut bool,
    out_clocks_s: *mut f64,
    out_has_clocks_s: *mut bool,
    out_ionosphere_slant_delays_m: *mut f64,
    out_has_ionosphere_slant_delays_m: *mut bool,
    out_troposphere_delays_m: *mut f64,
    out_has_troposphere_delays_m: *mut bool,
    out_statuses: *mut SidereonEmissionMediaStatus,
    out_result_statuses: *mut SidereonStatus,
) -> SidereonStatus {
    // Snapshot and validate every caller-owned input before initializing
    // outputs. Defer any input error until after output initialization to keep
    // the established reset-on-error behavior while supporting in-place use.
    let inputs_result = (|| {
        let sats = satellites_from_c_tokens(fn_name, satellites, count)?;
        let epochs = require_slice(
            emission_epochs_j2000_s,
            count,
            fn_name,
            "emission_epochs_j2000_s",
        )?
        .to_vec();
        let receiver = read_vec3(fn_name, "receiver_ecef_m", receiver_ecef_m)?;
        let options = emission_media_options_from_c(fn_name, options)?;
        Ok((sats, epochs, receiver, options))
    })();
    c_try!(initialize_emission_media_outputs(
        fn_name,
        count,
        out_positions_ecef_m,
        out_has_positions,
        out_clocks_s,
        out_has_clocks_s,
        out_ionosphere_slant_delays_m,
        out_has_ionosphere_slant_delays_m,
        out_troposphere_delays_m,
        out_has_troposphere_delays_m,
        out_statuses,
        out_result_statuses,
    ));
    let (sats, epochs, receiver, options) = c_try!(inputs_result);
    let batch = match observables_emission_media_batch_at_j2000_s(
        source, &sats, &epochs, receiver, options,
    ) {
        Ok(batch) => batch,
        Err(err) => return map_observables_error(fn_name, err),
    };
    write_emission_media_batch(
        fn_name,
        &batch,
        count,
        out_positions_ecef_m,
        out_has_positions,
        out_clocks_s,
        out_has_clocks_s,
        out_ionosphere_slant_delays_m,
        out_has_ionosphere_slant_delays_m,
        out_troposphere_delays_m,
        out_has_troposphere_delays_m,
        out_statuses,
        out_result_statuses,
    )
}

unsafe fn emission_media_options_from_c<'a>(
    fn_name: &str,
    options: *const SidereonEmissionMediaOptions,
) -> Result<EmissionMediaBatchOptions<'a>, SidereonStatus> {
    let options = match options.as_ref() {
        Some(options) => *options,
        None => {
            return Ok(EmissionMediaBatchOptions::default());
        }
    };
    let troposphere = if options.troposphere_enabled {
        Some(ObservableTroposphereCorrection {
            met: emission_met_from_c(fn_name, &options.met)?,
            mapping: MappingModel::Niell,
        })
    } else {
        None
    };
    let ionosphere = if options.ionex.is_null() {
        None
    } else {
        let ionex = require_ref(options.ionex, fn_name, "options.ionex")?;
        if options.ionex_policy_enabled {
            let policy = super::ionex::ionex_slant_policy_from_c(fn_name, options.ionex_policy)?;
            Some(ObservableIonosphereCorrection::IonexWithPolicy(
                &ionex.inner,
                policy,
            ))
        } else {
            Some(ObservableIonosphereCorrection::Ionex(&ionex.inner))
        }
    };
    let mut media = ObservableMediaOptions::default();
    media.troposphere = troposphere;
    media.ionosphere = ionosphere;

    let mut o = EmissionMediaBatchOptions::default();
    o.carrier_hz = options.carrier_hz;
    o.media = media;
    o.min_elevation_rad = options
        .min_elevation_enabled
        .then_some(options.min_elevation_rad);
    Ok(o)
}

fn emission_met_from_c(fn_name: &str, met: &SidereonMet) -> Result<Met, SidereonStatus> {
    Met::new(met.pressure_hpa, met.temperature_k, met.relative_humidity).map_err(|err| {
        set_last_error(format!("{fn_name}: {err}"));
        SidereonStatus::InvalidArgument
    })
}

#[allow(clippy::too_many_arguments)]
unsafe fn initialize_emission_media_outputs(
    fn_name: &str,
    count: usize,
    out_positions_ecef_m: *mut f64,
    out_has_positions: *mut bool,
    out_clocks_s: *mut f64,
    out_has_clocks_s: *mut bool,
    out_ionosphere_slant_delays_m: *mut f64,
    out_has_ionosphere_slant_delays_m: *mut bool,
    out_troposphere_delays_m: *mut f64,
    out_has_troposphere_delays_m: *mut bool,
    out_statuses: *mut SidereonEmissionMediaStatus,
    out_result_statuses: *mut SidereonStatus,
) -> Result<(), SidereonStatus> {
    let position_values = checked_position_output_count(fn_name, count)?;
    require_out_array(
        out_positions_ecef_m,
        position_values,
        fn_name,
        "out_positions_ecef_m",
    )?;
    require_out_array(out_has_positions, count, fn_name, "out_has_positions")?;
    require_out_array(out_clocks_s, count, fn_name, "out_clocks_s")?;
    require_out_array(out_has_clocks_s, count, fn_name, "out_has_clocks_s")?;
    require_out_array(
        out_ionosphere_slant_delays_m,
        count,
        fn_name,
        "out_ionosphere_slant_delays_m",
    )?;
    require_out_array(
        out_has_ionosphere_slant_delays_m,
        count,
        fn_name,
        "out_has_ionosphere_slant_delays_m",
    )?;
    require_out_array(
        out_troposphere_delays_m,
        count,
        fn_name,
        "out_troposphere_delays_m",
    )?;
    require_out_array(
        out_has_troposphere_delays_m,
        count,
        fn_name,
        "out_has_troposphere_delays_m",
    )?;
    require_out_array(out_statuses, count, fn_name, "out_statuses")?;
    require_out_array(out_result_statuses, count, fn_name, "out_result_statuses")?;

    if count != 0 {
        let outputs = [
            Some((
                checked_output_range(
                    fn_name,
                    out_positions_ecef_m,
                    position_values,
                    "out_positions_ecef_m",
                )?,
                "out_positions_ecef_m",
            )),
            Some((
                checked_output_range(fn_name, out_has_positions, count, "out_has_positions")?,
                "out_has_positions",
            )),
            Some((
                checked_output_range(fn_name, out_clocks_s, count, "out_clocks_s")?,
                "out_clocks_s",
            )),
            Some((
                checked_output_range(fn_name, out_has_clocks_s, count, "out_has_clocks_s")?,
                "out_has_clocks_s",
            )),
            Some((
                checked_output_range(
                    fn_name,
                    out_ionosphere_slant_delays_m,
                    count,
                    "out_ionosphere_slant_delays_m",
                )?,
                "out_ionosphere_slant_delays_m",
            )),
            Some((
                checked_output_range(
                    fn_name,
                    out_has_ionosphere_slant_delays_m,
                    count,
                    "out_has_ionosphere_slant_delays_m",
                )?,
                "out_has_ionosphere_slant_delays_m",
            )),
            Some((
                checked_output_range(
                    fn_name,
                    out_troposphere_delays_m,
                    count,
                    "out_troposphere_delays_m",
                )?,
                "out_troposphere_delays_m",
            )),
            Some((
                checked_output_range(
                    fn_name,
                    out_has_troposphere_delays_m,
                    count,
                    "out_has_troposphere_delays_m",
                )?,
                "out_has_troposphere_delays_m",
            )),
            Some((
                checked_output_range(fn_name, out_statuses, count, "out_statuses")?,
                "out_statuses",
            )),
            Some((
                checked_output_range(fn_name, out_result_statuses, count, "out_result_statuses")?,
                "out_result_statuses",
            )),
        ];
        reject_overlapping_optional_outputs(fn_name, &outputs)?;
    }

    for idx in 0..count {
        let base = idx * 3;
        for axis in 0..3 {
            out_positions_ecef_m.add(base + axis).write(f64::NAN);
        }
        out_has_positions.add(idx).write(false);
        out_clocks_s.add(idx).write(0.0);
        out_has_clocks_s.add(idx).write(false);
        out_ionosphere_slant_delays_m.add(idx).write(0.0);
        out_has_ionosphere_slant_delays_m.add(idx).write(false);
        out_troposphere_delays_m.add(idx).write(0.0);
        out_has_troposphere_delays_m.add(idx).write(false);
        out_statuses
            .add(idx)
            .write(SidereonEmissionMediaStatus::Error);
        out_result_statuses
            .add(idx)
            .write(SidereonStatus::InvalidArgument);
    }
    Ok(())
}

fn emission_media_status_to_c(status: EmissionMediaStatus) -> SidereonEmissionMediaStatus {
    match status {
        EmissionMediaStatus::Valid => SidereonEmissionMediaStatus::Valid,
        EmissionMediaStatus::Gap => SidereonEmissionMediaStatus::Gap,
        EmissionMediaStatus::BelowElevationCutoff => {
            SidereonEmissionMediaStatus::BelowElevationCutoff
        }
        EmissionMediaStatus::Error => SidereonEmissionMediaStatus::Error,
    }
}

fn emission_media_result_status(
    status: EmissionMediaStatus,
    error: &Option<ObservablesError>,
) -> SidereonStatus {
    match status {
        EmissionMediaStatus::Valid | EmissionMediaStatus::BelowElevationCutoff => {
            SidereonStatus::Ok
        }
        EmissionMediaStatus::Gap => SidereonStatus::Solve,
        EmissionMediaStatus::Error => error
            .as_ref()
            .map(observable_error_status)
            .unwrap_or(SidereonStatus::Solve),
    }
}

fn observable_error_status(error: &ObservablesError) -> SidereonStatus {
    match error {
        ObservablesError::Ephemeris(CoreError::Ut1OutsideCoverage(_)) => {
            SidereonStatus::Ut1OutsideCoverage
        }
        ObservablesError::InvalidInput { .. }
        | ObservablesError::Media(_)
        | ObservablesError::Ephemeris(CoreError::InvalidInput(_)) => {
            SidereonStatus::InvalidArgument
        }
        ObservablesError::NoEphemeris | ObservablesError::Ephemeris(_) => SidereonStatus::Solve,
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn write_emission_media_batch(
    fn_name: &str,
    batch: &EmissionMediaBatch,
    count: usize,
    out_positions_ecef_m: *mut f64,
    out_has_positions: *mut bool,
    out_clocks_s: *mut f64,
    out_has_clocks_s: *mut bool,
    out_ionosphere_slant_delays_m: *mut f64,
    out_has_ionosphere_slant_delays_m: *mut bool,
    out_troposphere_delays_m: *mut f64,
    out_has_troposphere_delays_m: *mut bool,
    out_statuses: *mut SidereonEmissionMediaStatus,
    out_result_statuses: *mut SidereonStatus,
) -> SidereonStatus {
    if batch.len() != count {
        set_last_error(format!(
            "{fn_name}: core returned {} emission rows for {count} inputs",
            batch.len()
        ));
        return SidereonStatus::Solve;
    }
    for idx in 0..count {
        if let Some(position) = batch.positions_ecef_m[idx] {
            let base = idx * 3;
            for (axis, value) in position.iter().enumerate() {
                out_positions_ecef_m.add(base + axis).write(*value);
            }
            out_has_positions.add(idx).write(true);
        }
        if let Some(clock_s) = batch.clocks_s[idx] {
            out_clocks_s.add(idx).write(clock_s);
            out_has_clocks_s.add(idx).write(true);
        }
        if let Some(delay_m) = batch.ionosphere_slant_delays_m[idx] {
            out_ionosphere_slant_delays_m.add(idx).write(delay_m);
            out_has_ionosphere_slant_delays_m.add(idx).write(true);
        }
        if let Some(delay_m) = batch.troposphere_delays_m[idx] {
            out_troposphere_delays_m.add(idx).write(delay_m);
            out_has_troposphere_delays_m.add(idx).write(true);
        }
        let status = batch.statuses[idx];
        out_statuses
            .add(idx)
            .write(emission_media_status_to_c(status));
        let result_status = emission_media_result_status(status, &batch.element_errors[idx]);
        if let Some(error) = &batch.element_errors[idx] {
            crate::engine_error::record_observable_row_error(fn_name, idx, result_status, error);
        }
        out_result_statuses.add(idx).write(result_status);
    }
    SidereonStatus::Ok
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::mem::MaybeUninit;
    use std::path::PathBuf;

    fn fixture_sp3() -> Sp3 {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/sp3/GRG0MGXFIN_20201760000_01D_15M_ORB.SP3");
        let bytes = fs::read(path).expect("read SP3 fixture");
        Sp3::parse(&bytes).expect("parse SP3")
    }

    fn assert_same_bits(got: f64, want: f64) {
        assert_eq!(got.to_bits(), want.to_bits(), "got {got:e}, want {want:e}");
    }

    fn last_error_message() -> String {
        let needed = unsafe { sidereon_last_error_message(ptr::null_mut(), 0) };
        let mut buffer = vec![0 as c_char; needed + 1];
        unsafe {
            sidereon_last_error_message(buffer.as_mut_ptr(), buffer.len());
            CStr::from_ptr(buffer.as_ptr())
                .to_str()
                .expect("valid error message")
                .to_owned()
        }
    }

    #[test]
    fn emission_media_batch_matches_core_reference() {
        let sp3 = fixture_sp3();
        let receiver = [6_378_137.0, 0.0, 0.0];
        let epoch = 646_272_000.0;
        let met = Met::new(1013.25, 288.15, 0.5).expect("valid met");
        let mut media = ObservableMediaOptions::default();
        media.troposphere = Some(ObservableTroposphereCorrection {
            met,
            mapping: MappingModel::Niell,
        });
        media.ionosphere = None;

        let mut options = EmissionMediaBatchOptions::default();
        options.carrier_hz = sidereon_core::constants::F_L1_HZ;
        options.media = media;
        options.min_elevation_rad = None;

        let mut satellites = Vec::new();
        for sat in sp3
            .satellites()
            .iter()
            .copied()
            .filter(|sat| sat.system == GnssSystem::Gps)
        {
            let batch = observables_emission_media_batch_at_j2000_s(
                &sp3,
                &[sat],
                &[epoch],
                receiver,
                options,
            )
            .expect("core emission probe");
            if batch.statuses[0] == EmissionMediaStatus::Valid
                && batch.positions_ecef_m[0].is_some()
                && batch.troposphere_delays_m[0].is_some()
            {
                satellites.push(sat);
            }
            if satellites.len() == 3 {
                break;
            }
        }
        assert_eq!(satellites.len(), 3);
        let gap_sat = satellites[0];
        satellites.push(gap_sat);
        satellites.push(GnssSatelliteId::new(GnssSystem::Gps, 99).expect("valid G99 token"));
        let epochs = [
            epoch,
            epoch + 300.0,
            epoch + 600.0,
            epoch + 10_000_000.0,
            epoch,
        ];
        let expected = observables_emission_media_batch_at_j2000_s(
            &sp3,
            &satellites,
            &epochs,
            receiver,
            options,
        )
        .expect("core emission batch");

        let sp3_handle = Box::into_raw(Box::new(SidereonSp3 { inner: sp3 }));
        let sat_tokens = satellites
            .iter()
            .map(|sat| CString::new(sat.to_string()).expect("sat token"))
            .collect::<Vec<_>>();
        let sat_ptrs = sat_tokens
            .iter()
            .map(|token| token.as_ptr())
            .collect::<Vec<_>>();
        let c_options = SidereonEmissionMediaOptions {
            carrier_hz: sidereon_core::constants::F_L1_HZ,
            min_elevation_enabled: false,
            min_elevation_rad: 0.0,
            troposphere_enabled: true,
            met: SidereonMet {
                pressure_hpa: met.pressure_hpa,
                temperature_k: met.temperature_k,
                relative_humidity: met.relative_humidity,
            },
            ionex: ptr::null(),
            ionex_policy_enabled: false,
            ionex_policy: sidereon_ionex_slant_policy_default(),
        };
        let count = satellites.len();
        let mut positions = vec![0.0; count * 3];
        let mut has_positions = vec![false; count];
        let mut clocks = vec![0.0; count];
        let mut has_clocks = vec![false; count];
        let mut iono = vec![0.0; count];
        let mut has_iono = vec![false; count];
        let mut tropo = vec![0.0; count];
        let mut has_tropo = vec![false; count];
        let mut statuses = vec![SidereonEmissionMediaStatus::Error; count];
        let mut result_statuses = vec![SidereonStatus::Panic; count];

        let status = unsafe {
            sidereon_sp3_emission_media_batch_at_j2000_s(
                sp3_handle,
                sat_ptrs.as_ptr(),
                epochs.as_ptr(),
                count,
                receiver.as_ptr(),
                &c_options,
                positions.as_mut_ptr(),
                has_positions.as_mut_ptr(),
                clocks.as_mut_ptr(),
                has_clocks.as_mut_ptr(),
                iono.as_mut_ptr(),
                has_iono.as_mut_ptr(),
                tropo.as_mut_ptr(),
                has_tropo.as_mut_ptr(),
                statuses.as_mut_ptr(),
                result_statuses.as_mut_ptr(),
            )
        };
        assert_eq!(status, SidereonStatus::Ok);

        for idx in 0..count {
            assert_eq!(
                statuses[idx],
                emission_media_status_to_c(expected.statuses[idx])
            );
            assert_eq!(
                result_statuses[idx],
                emission_media_result_status(expected.statuses[idx], &expected.element_errors[idx])
            );
            assert_eq!(has_positions[idx], expected.positions_ecef_m[idx].is_some());
            if let Some(position) = expected.positions_ecef_m[idx] {
                for axis in 0..3 {
                    assert_same_bits(positions[idx * 3 + axis], position[axis]);
                }
            }
            assert_eq!(has_clocks[idx], expected.clocks_s[idx].is_some());
            if let Some(clock) = expected.clocks_s[idx] {
                assert_same_bits(clocks[idx], clock);
            }
            assert_eq!(
                has_iono[idx],
                expected.ionosphere_slant_delays_m[idx].is_some()
            );
            assert_eq!(has_tropo[idx], expected.troposphere_delays_m[idx].is_some());
            if let Some(delay) = expected.troposphere_delays_m[idx] {
                assert_same_bits(tropo[idx], delay);
            }
        }

        let mut owned_row_errors = ptr::null_mut();
        assert_eq!(
            unsafe { sidereon_observable_row_errors_snapshot(&mut owned_row_errors) },
            SidereonStatus::Ok
        );
        assert!(!owned_row_errors.is_null());
        let mut row_error_count = usize::MAX;
        assert_eq!(
            unsafe { sidereon_observable_row_errors_count(owned_row_errors, &mut row_error_count) },
            SidereonStatus::Ok
        );
        let expected_row_errors = expected
            .element_errors
            .iter()
            .filter(|error| error.is_some())
            .count();
        assert_eq!(row_error_count, expected_row_errors);
        let unknown_row_index = expected
            .element_errors
            .iter()
            .position(|error| {
                matches!(
                    error,
                    Some(ObservablesError::Ephemeris(CoreError::UnknownSatellite(sat)))
                        if sat.to_string() == "G99"
                )
            })
            .expect("G99 has a core unknown-satellite error");
        let owned_index = expected
            .element_errors
            .iter()
            .take(unknown_row_index)
            .filter(|error| error.is_some())
            .count();
        let mut row_info = MaybeUninit::<SidereonObservableRowErrorInfo>::uninit();
        assert_eq!(
            unsafe {
                sidereon_observable_row_errors_info(
                    owned_row_errors,
                    owned_index,
                    row_info.as_mut_ptr(),
                )
            },
            SidereonStatus::Ok
        );
        let row_info = unsafe { row_info.assume_init() };
        assert_eq!(row_info.row_index, unknown_row_index);
        assert_eq!(row_info.status, SidereonStatus::Solve);

        let mut required = 0usize;
        let mut written = 0usize;
        assert_eq!(
            unsafe {
                sidereon_observable_row_errors_payload(
                    owned_row_errors,
                    owned_index,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(written, 0);
        assert_eq!(required, row_info.payload_len);
        let mut canary = vec![0xA5; required - 1];
        assert_eq!(
            unsafe {
                sidereon_observable_row_errors_payload(
                    owned_row_errors,
                    owned_index,
                    canary.as_mut_ptr(),
                    canary.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert!(canary.iter().all(|byte| *byte == 0xA5));
        let mut payload = vec![0u8; required];
        assert_eq!(
            unsafe {
                sidereon_observable_row_errors_payload(
                    owned_row_errors,
                    owned_index,
                    payload.as_mut_ptr(),
                    payload.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(written, required);
        let payload: serde_json::Value = serde_json::from_slice(&payload).expect("JSON payload");
        assert_eq!(payload["schema_version"], 1);
        assert_eq!(payload["family"], "observables");
        assert_eq!(
            payload["operation"],
            "sidereon_sp3_emission_media_batch_at_j2000_s"
        );
        assert_eq!(payload["error"]["kind"], "ephemeris");
        assert_eq!(
            payload["error"]["fields"]["cause"]["kind"],
            "unknown_satellite"
        );
        assert_eq!(
            payload["error"]["fields"]["cause"]["fields"]["satellite_id"],
            "G99"
        );
        let expected_payload = payload.clone();

        assert_eq!(
            unsafe {
                sidereon_sp3_emission_media_batch_at_j2000_s(
                    sp3_handle,
                    sat_ptrs.as_ptr(),
                    epochs.as_ptr(),
                    count,
                    receiver.as_ptr(),
                    &c_options,
                    ptr::null_mut(),
                    has_positions.as_mut_ptr(),
                    clocks.as_mut_ptr(),
                    has_clocks.as_mut_ptr(),
                    iono.as_mut_ptr(),
                    has_iono.as_mut_ptr(),
                    tropo.as_mut_ptr(),
                    has_tropo.as_mut_ptr(),
                    statuses.as_mut_ptr(),
                    result_statuses.as_mut_ptr(),
                )
            },
            SidereonStatus::NullPointer
        );
        let mut cleared_engine_info = MaybeUninit::<SidereonEngineErrorInfo>::uninit();
        assert_eq!(
            unsafe { sidereon_last_engine_error_info(cleared_engine_info.as_mut_ptr()) },
            SidereonStatus::Ok
        );
        assert_eq!(
            unsafe { cleared_engine_info.assume_init() }.family,
            SidereonEngineErrorFamily::None
        );
        let mut current_row_errors = ptr::null_mut();
        assert_eq!(
            unsafe { sidereon_observable_row_errors_snapshot(&mut current_row_errors) },
            SidereonStatus::Ok
        );
        let mut current_count = usize::MAX;
        assert_eq!(
            unsafe { sidereon_observable_row_errors_count(current_row_errors, &mut current_count) },
            SidereonStatus::Ok
        );
        assert_eq!(current_count, 0);
        unsafe { sidereon_observable_row_errors_free(current_row_errors) };

        let mut scalar_out = MaybeUninit::<SidereonPredictedObservables>::uninit();
        let g01 = CString::new(satellites[0].to_string()).expect("G01 token");
        assert_eq!(
            unsafe {
                sidereon_sp3_observables(
                    sp3_handle,
                    g01.as_ptr(),
                    receiver.as_ptr(),
                    epoch,
                    ptr::null(),
                    scalar_out.as_mut_ptr(),
                )
            },
            SidereonStatus::Ok
        );
        let mut stale_row_error_count = usize::MAX;
        assert_eq!(
            unsafe {
                sidereon_observable_row_errors_count(owned_row_errors, &mut stale_row_error_count)
            },
            SidereonStatus::Ok
        );
        assert_eq!(stale_row_error_count, expected_row_errors);
        current_row_errors = ptr::null_mut();
        assert_eq!(
            unsafe { sidereon_observable_row_errors_snapshot(&mut current_row_errors) },
            SidereonStatus::Ok
        );
        current_count = usize::MAX;
        assert_eq!(
            unsafe { sidereon_observable_row_errors_count(current_row_errors, &mut current_count) },
            SidereonStatus::Ok
        );
        assert_eq!(current_count, 0);
        unsafe { sidereon_observable_row_errors_free(current_row_errors) };
        unsafe { sidereon_sp3_free(sp3_handle) };

        // The owned snapshot remains readable after a subsequent success and
        // after freeing the source that produced the batch.
        let mut retained_info = MaybeUninit::<SidereonObservableRowErrorInfo>::uninit();
        assert_eq!(
            unsafe {
                sidereon_observable_row_errors_info(
                    owned_row_errors,
                    owned_index,
                    retained_info.as_mut_ptr(),
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(
            unsafe { retained_info.assume_init() }.row_index,
            unknown_row_index
        );
        let mut retained_bytes = vec![0u8; required];
        assert_eq!(
            unsafe {
                sidereon_observable_row_errors_payload(
                    owned_row_errors,
                    owned_index,
                    retained_bytes.as_mut_ptr(),
                    retained_bytes.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        let retained_payload: serde_json::Value =
            serde_json::from_slice(&retained_bytes).expect("retained JSON payload");
        assert_eq!(retained_payload, expected_payload);

        let g99 = CString::new("G99").expect("G99 token");
        let mut error_out = MaybeUninit::<SidereonPredictedObservables>::uninit();
        let fresh_sp3 = Box::into_raw(Box::new(SidereonSp3 {
            inner: fixture_sp3(),
        }));

        let mut sample_written = 0usize;
        let mut sample_required = 0usize;
        assert_eq!(
            unsafe {
                sidereon_sp3_ephemeris_sample(
                    fresh_sp3,
                    sat_ptrs.as_ptr(),
                    1,
                    epoch,
                    epoch,
                    0.0,
                    ptr::null_mut(),
                    0,
                    &mut sample_written,
                    &mut sample_required,
                )
            },
            SidereonStatus::InvalidArgument
        );
        let mut engine_info = MaybeUninit::<SidereonEngineErrorInfo>::uninit();
        assert_eq!(
            unsafe { sidereon_last_engine_error_info(engine_info.as_mut_ptr()) },
            SidereonStatus::Ok
        );
        assert_eq!(
            unsafe { engine_info.assume_init() }.family,
            SidereonEngineErrorFamily::Observables
        );
        assert_eq!(
            last_error_message(),
            "sidereon_sp3_ephemeris_sample: invalid observable input step_s: not positive"
        );
        let mut payload_written = 0usize;
        let mut payload_required = 0usize;
        assert_eq!(
            unsafe {
                sidereon_last_engine_error_payload(
                    ptr::null_mut(),
                    0,
                    &mut payload_written,
                    &mut payload_required,
                )
            },
            SidereonStatus::Ok
        );
        let mut sample_error_payload = vec![0u8; payload_required];
        assert_eq!(
            unsafe {
                sidereon_last_engine_error_payload(
                    sample_error_payload.as_mut_ptr(),
                    sample_error_payload.len(),
                    &mut payload_written,
                    &mut payload_required,
                )
            },
            SidereonStatus::Ok
        );
        let sample_error_payload: serde_json::Value =
            serde_json::from_slice(&sample_error_payload).expect("sample error JSON");
        assert_eq!(
            sample_error_payload,
            serde_json::json!({
                "schema_version": 1,
                "family": "observables",
                "operation": "sidereon_sp3_ephemeris_sample",
                "error": {
                    "kind": "invalid_input",
                    "fields": {"field": "step_s", "kind": "not_positive"}
                }
            })
        );

        let mut success_out = MaybeUninit::<SidereonPredictedObservables>::uninit();
        assert_eq!(
            unsafe {
                sidereon_sp3_observables(
                    fresh_sp3,
                    g01.as_ptr(),
                    receiver.as_ptr(),
                    epoch,
                    ptr::null(),
                    success_out.as_mut_ptr(),
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(
            unsafe { sidereon_last_engine_error_info(engine_info.as_mut_ptr()) },
            SidereonStatus::Ok
        );
        assert_eq!(
            unsafe { engine_info.assume_init() }.family,
            SidereonEngineErrorFamily::None
        );

        assert_eq!(
            unsafe {
                sidereon_sp3_observables(
                    fresh_sp3,
                    g99.as_ptr(),
                    receiver.as_ptr(),
                    epoch,
                    ptr::null(),
                    error_out.as_mut_ptr(),
                )
            },
            SidereonStatus::Solve
        );
        assert_eq!(
            last_error_message(),
            "sidereon_sp3_observables: unknown satellite: G99"
        );
        assert_eq!(
            unsafe { sidereon_last_engine_error_info(engine_info.as_mut_ptr()) },
            SidereonStatus::Ok
        );
        assert_eq!(
            unsafe { engine_info.assume_init() }.family,
            SidereonEngineErrorFamily::Observables
        );
        assert_eq!(
            unsafe {
                sidereon_sp3_observables(
                    fresh_sp3,
                    g01.as_ptr(),
                    receiver.as_ptr(),
                    epoch,
                    ptr::null(),
                    ptr::null_mut(),
                )
            },
            SidereonStatus::NullPointer
        );
        assert_eq!(
            unsafe { sidereon_last_engine_error_info(engine_info.as_mut_ptr()) },
            SidereonStatus::Ok
        );
        assert_eq!(
            unsafe { engine_info.assume_init() }.family,
            SidereonEngineErrorFamily::None
        );
        unsafe {
            sidereon_sp3_free(fresh_sp3);
            sidereon_observable_row_errors_free(owned_row_errors);
        }
    }

    #[test]
    fn observable_state_batch_retains_owned_g99_row_error() {
        let sp3 = fixture_sp3();
        let g01 = GnssSatelliteId::new(GnssSystem::Gps, 1).expect("valid G01");
        let g99 = GnssSatelliteId::new(GnssSystem::Gps, 99).expect("valid G99");
        let epoch = 646_272_000.0;
        let expected_state = sp3
            .observable_state_at_j2000_s(g01, epoch)
            .expect("fixture contains G01 at the selected epoch");
        let sp3_handle = Box::into_raw(Box::new(SidereonSp3 { inner: sp3 }));
        let tokens = [
            CString::new(g01.to_string()).unwrap(),
            CString::new(g99.to_string()).unwrap(),
        ];
        let satellite_ptrs = [tokens[0].as_ptr(), tokens[1].as_ptr()];
        let epochs = [epoch, epoch];
        let mut positions = [0.0; 6];
        let mut clocks = [0.0; 2];
        let mut has_clocks = [false; 2];
        let mut element_statuses = [SidereonObservableStateElementStatus::Error; 2];
        let mut result_statuses = [SidereonStatus::Panic; 2];

        assert_eq!(
            unsafe {
                sidereon_sp3_observable_states_at_j2000_s(
                    sp3_handle,
                    satellite_ptrs.as_ptr(),
                    epochs.as_ptr(),
                    2,
                    positions.as_mut_ptr(),
                    clocks.as_mut_ptr(),
                    has_clocks.as_mut_ptr(),
                    element_statuses.as_mut_ptr(),
                    result_statuses.as_mut_ptr(),
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(
            element_statuses[0],
            SidereonObservableStateElementStatus::Valid
        );
        assert_eq!(result_statuses[0], SidereonStatus::Ok);
        for (axis, position) in positions[..3].iter().enumerate() {
            assert_same_bits(*position, expected_state.position_ecef_m[axis]);
        }
        assert_eq!(has_clocks[0], expected_state.clock_s.is_some());
        if let Some(clock) = expected_state.clock_s {
            assert_same_bits(clocks[0], clock);
        }
        assert_eq!(
            element_statuses[1],
            SidereonObservableStateElementStatus::Gap
        );
        assert_eq!(result_statuses[1], SidereonStatus::Solve);
        assert!((3..6).all(|index| positions[index].is_nan()));
        assert!(!has_clocks[1]);

        let mut row_errors = ptr::null_mut();
        assert_eq!(
            unsafe { sidereon_observable_row_errors_snapshot(&mut row_errors) },
            SidereonStatus::Ok
        );
        let mut count = usize::MAX;
        assert_eq!(
            unsafe { sidereon_observable_row_errors_count(row_errors, &mut count) },
            SidereonStatus::Ok
        );
        assert_eq!(count, 1);
        let mut info = MaybeUninit::<SidereonObservableRowErrorInfo>::uninit();
        assert_eq!(
            unsafe { sidereon_observable_row_errors_info(row_errors, 0, info.as_mut_ptr()) },
            SidereonStatus::Ok
        );
        let info = unsafe { info.assume_init() };
        assert_eq!(info.row_index, 1);
        assert_eq!(info.status, SidereonStatus::Solve);

        let mut written = 0usize;
        let mut required = 0usize;
        assert_eq!(
            unsafe {
                sidereon_observable_row_errors_payload(
                    row_errors,
                    0,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(written, 0);
        assert_eq!(required, info.payload_len);
        let mut payload = vec![0u8; required];
        assert_eq!(
            unsafe {
                sidereon_observable_row_errors_payload(
                    row_errors,
                    0,
                    payload.as_mut_ptr(),
                    payload.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        let payload: serde_json::Value = serde_json::from_slice(&payload).expect("row JSON");
        assert_eq!(
            payload,
            serde_json::json!({
                "schema_version": 1,
                "family": "observables",
                "operation": "sidereon_sp3_observable_states_at_j2000_s",
                "error": {
                    "kind": "ephemeris",
                    "fields": {
                        "cause": {
                            "kind": "unknown_satellite",
                            "fields": {"satellite_id": "G99"}
                        }
                    }
                }
            })
        );

        let mut invalid_info = SidereonObservableRowErrorInfo {
            row_index: 77,
            status: SidereonStatus::Panic,
            payload_len: 99,
        };
        assert_eq!(
            unsafe { sidereon_observable_row_errors_info(row_errors, count, &mut invalid_info) },
            SidereonStatus::InvalidArgument
        );
        assert_eq!(invalid_info.row_index, 77);
        assert_eq!(invalid_info.status, SidereonStatus::Panic);
        assert_eq!(invalid_info.payload_len, 99);
        let mut canary = [0xA5u8; 8];
        written = usize::MAX;
        required = usize::MAX;
        assert_eq!(
            unsafe {
                sidereon_observable_row_errors_payload(
                    row_errors,
                    count,
                    canary.as_mut_ptr(),
                    canary.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert_eq!(canary, [0xA5; 8]);
        assert_eq!(written, 0);
        assert_eq!(required, 0);
        let mut count_after_invalid_queries = usize::MAX;
        assert_eq!(
            unsafe {
                sidereon_observable_row_errors_count(row_errors, &mut count_after_invalid_queries)
            },
            SidereonStatus::Ok
        );
        assert_eq!(count_after_invalid_queries, 1);

        unsafe {
            sidereon_sp3_free(sp3_handle);
            sidereon_observable_row_errors_free(ptr::null_mut());
            sidereon_observable_row_errors_free(row_errors);
        }
    }

    #[test]
    fn emission_media_options_carry_the_ionex_slant_policy() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/ionex/synthetic_2map_7x7.20i");
        let bytes = fs::read(path).expect("read IONEX fixture");
        let ionex = SidereonIonex {
            inner: Ionex::parse(&bytes).expect("parse IONEX"),
        };
        let mut c_options = unsafe {
            let mut options = std::mem::MaybeUninit::<SidereonEmissionMediaOptions>::uninit();
            assert_eq!(
                sidereon_emission_media_options_init(options.as_mut_ptr()),
                SidereonStatus::Ok
            );
            options.assume_init()
        };
        // The defaults leave the policy off, so the engine default applies.
        assert!(!c_options.ionex_policy_enabled);
        assert_eq!(
            c_options.ionex_policy,
            sidereon_ionex_slant_policy_default()
        );
        c_options.ionex = &ionex;

        let options = unsafe { emission_media_options_from_c("test", &c_options) }
            .expect("default IONEX options");
        assert!(matches!(
            options.media.ionosphere,
            Some(ObservableIonosphereCorrection::Ionex(_))
        ));

        c_options.ionex_policy_enabled = true;
        c_options.ionex_policy = sidereon_ionex_slant_policy_init(
            SidereonIonexCoveragePolicy::Hold as u32,
            SidereonIonexMissingNodePolicy::Renormalize as u32,
            SidereonIonexMappingPolicy::Declared as u32,
        );
        let options = unsafe { emission_media_options_from_c("test", &c_options) }
            .expect("explicit IONEX policy");
        match options.media.ionosphere {
            Some(ObservableIonosphereCorrection::IonexWithPolicy(_, policy)) => {
                assert_eq!(policy.coverage, IonexCoveragePolicy::Hold);
                assert_eq!(policy.missing_nodes, IonexMissingNodePolicy::Renormalize);
                assert_eq!(policy.mapping, IonexMappingPolicy::Declared);
            }
            _ => panic!("the enabled policy selects the policy-aware correction"),
        }

        // A tag the binding does not name fails the options, not a row.
        c_options.ionex_policy.mapping = 9;
        let refused = unsafe { emission_media_options_from_c("test", &c_options) };
        assert!(matches!(refused, Err(SidereonStatus::InvalidArgument)));
    }
}
