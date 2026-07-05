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
    ffi_boundary(
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
    ffi_boundary(
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
    let sats = c_try!(satellites_from_c_tokens(fn_name, satellites, count));
    let epochs = c_try!(require_slice(
        emission_epochs_j2000_s,
        count,
        fn_name,
        "emission_epochs_j2000_s"
    ));
    let receiver = c_try!(read_vec3(fn_name, "receiver_ecef_m", receiver_ecef_m));
    let options = c_try!(emission_media_options_from_c(fn_name, options));
    let batch =
        match observables_emission_media_batch_at_j2000_s(source, &sats, epochs, receiver, options)
        {
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
        Some(ObservableIonosphereCorrection::Ionex(&ionex.inner))
    };
    Ok(EmissionMediaBatchOptions {
        carrier_hz: options.carrier_hz,
        media: ObservableMediaOptions {
            troposphere,
            ionosphere,
        },
        min_elevation_rad: options
            .min_elevation_enabled
            .then_some(options.min_elevation_rad),
    })
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
        out_result_statuses
            .add(idx)
            .write(emission_media_result_status(
                status,
                &batch.element_errors[idx],
            ));
    }
    SidereonStatus::Ok
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn fixture_sp3() -> Sp3 {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/sp3/GRG0MGXFIN_20201760000_01D_15M_ORB.SP3");
        let bytes = fs::read(path).expect("read SP3 fixture");
        Sp3::parse(&bytes).expect("parse SP3")
    }

    fn assert_close(got: f64, want: f64, tol: f64) {
        assert!(
            (got - want).abs() <= tol,
            "got {got:e}, want {want:e}, tol {tol:e}"
        );
    }

    #[test]
    fn emission_media_batch_matches_core_reference() {
        let sp3 = fixture_sp3();
        let receiver = [6_378_137.0, 0.0, 0.0];
        let epoch = 646_272_000.0;
        let met = Met::new(1013.25, 288.15, 0.5).expect("valid met");
        let options = EmissionMediaBatchOptions {
            carrier_hz: sidereon_core::constants::F_L1_HZ,
            media: ObservableMediaOptions {
                troposphere: Some(ObservableTroposphereCorrection {
                    met,
                    mapping: MappingModel::Niell,
                }),
                ionosphere: None,
            },
            min_elevation_rad: None,
        };

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
        let epochs = [epoch, epoch + 300.0, epoch + 600.0, epoch + 10_000_000.0];
        let expected = observables_emission_media_batch_at_j2000_s(
            &sp3,
            &satellites,
            &epochs,
            receiver,
            options,
        )
        .expect("core emission batch");

        let sp3_handle = SidereonSp3 { inner: sp3 };
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
                &sp3_handle,
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
                    assert_close(positions[idx * 3 + axis], position[axis], 1.0e-9);
                }
            }
            assert_eq!(has_clocks[idx], expected.clocks_s[idx].is_some());
            if let Some(clock) = expected.clocks_s[idx] {
                assert_close(clocks[idx], clock, 1.0e-15);
            }
            assert_eq!(
                has_iono[idx],
                expected.ionosphere_slant_delays_m[idx].is_some()
            );
            assert_eq!(has_tropo[idx], expected.troposphere_delays_m[idx].is_some());
            if let Some(delay) = expected.troposphere_delays_m[idx] {
                assert_close(tropo[idx], delay, 1.0e-12);
            }
        }
    }
}
