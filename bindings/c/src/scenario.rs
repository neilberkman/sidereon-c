use super::*;

/// Synthetic scenario simulation output. Create with
/// sidereon_scenario_simulate_json and release with
/// sidereon_scenario_simulation_free.
pub struct SidereonScenarioSimulation {
    pub(crate) inner: sidereon_core::scenario::SyntheticObservationSet,
    pub(crate) json: Vec<u8>,
}

/// Summary of a deterministic scenario simulation.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonScenarioSummary {
    /// Scenario schema version used for the output.
    pub schema_version: u32,
    /// Seed used by deterministic streams.
    pub seed: u64,
    /// Number of receiver truth rows.
    pub receiver_truth_count: usize,
    /// Number of synthetic observation rows.
    pub observation_count: usize,
    /// Number of epoch offset entries.
    pub epoch_offset_count: usize,
    /// Deterministic FNV-1a fingerprint over output bits.
    pub determinism_fingerprint: u64,
    /// Number of bytes in the JSON serialization of the output set.
    pub json_len: usize,
}

/// Receiver truth row from a synthetic scenario.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonScenarioReceiverTruth {
    /// Receive epoch, seconds since J2000.
    pub t_rx_j2000_s: f64,
    /// Receiver ECEF position in meters.
    pub position_ecef_m: [f64; 3],
    /// Receiver ECEF velocity in meters per second.
    pub velocity_ecef_m_s: [f64; 3],
    /// Receiver clock contribution in meters.
    pub clock_m: f64,
    /// Receiver clock range-rate contribution in meters per second.
    pub clock_rate_m_s: f64,
}

/// Synthetic observable row.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonScenarioObservation {
    /// Epoch index for this observation.
    pub epoch_index: usize,
    /// Satellite token for this observation.
    pub sat_id: SidereonSatelliteToken,
    /// Null-terminated RINEX code observable label.
    pub code_observable: [c_char; RINEX_OBS_CODE_C_BYTES],
    /// Null-terminated RINEX phase observable label.
    pub phase_observable: [c_char; RINEX_OBS_CODE_C_BYTES],
    /// Null-terminated RINEX Doppler observable label.
    pub doppler_observable: [c_char; RINEX_OBS_CODE_C_BYTES],
    /// Carrier frequency in hertz.
    pub carrier_hz: f64,
    /// Synthetic code pseudorange in meters.
    pub pseudorange_m: f64,
    /// Synthetic carrier phase in cycles.
    pub carrier_phase_cycles: f64,
    /// Synthetic Doppler shift in hertz.
    pub doppler_hz: f64,
}

/// Ground-truth term ledger row for one synthetic observation.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonScenarioTerms {
    /// Geometric range in meters.
    pub geometric_range_m: f64,
    /// Nominal ephemeris satellite-clock contribution in meters.
    pub satellite_clock_m: f64,
    /// Receiver-clock contribution in meters.
    pub receiver_clock_m: f64,
    /// Injected satellite-clock contribution in meters.
    pub satellite_clock_error_m: f64,
    /// Ionospheric code delay in meters.
    pub ionosphere_m: f64,
    /// Tropospheric delay in meters.
    pub troposphere_m: f64,
    /// Thermal code noise in meters.
    pub thermal_noise_m: f64,
    /// Specular code multipath in meters.
    pub multipath_m: f64,
    /// Core quantization contribution in meters.
    pub quantization_m: f64,
    /// Carrier geometric range contribution in cycles.
    pub carrier_phase_geometric_cycles: f64,
    /// Carrier receiver-clock contribution in cycles.
    pub carrier_phase_receiver_clock_cycles: f64,
    /// Carrier nominal satellite-clock contribution in cycles.
    pub carrier_phase_satellite_clock_cycles: f64,
    /// Carrier injected satellite-clock contribution in cycles.
    pub carrier_phase_satellite_clock_error_cycles: f64,
    /// Carrier ionosphere contribution in cycles.
    pub carrier_phase_ionosphere_cycles: f64,
    /// Carrier troposphere contribution in cycles.
    pub carrier_phase_troposphere_cycles: f64,
    /// Carrier thermal-noise contribution in cycles.
    pub carrier_phase_thermal_noise_cycles: f64,
    /// Constant carrier-phase ambiguity contribution in cycles.
    pub carrier_phase_bias_cycles: f64,
    /// Core carrier quantization contribution in cycles.
    pub carrier_phase_quantization_cycles: f64,
    /// Doppler contribution from satellite line-of-sight motion in hertz.
    pub doppler_satellite_motion_hz: f64,
    /// Doppler contribution from receiver line-of-sight motion in hertz.
    pub doppler_receiver_motion_hz: f64,
    /// Doppler contribution from nominal ephemeris satellite-clock rate in hertz.
    pub doppler_satellite_clock_hz: f64,
    /// Doppler contribution from receiver-clock rate in hertz.
    pub doppler_receiver_clock_hz: f64,
    /// Doppler contribution from injected satellite-clock rate in hertz.
    pub doppler_satellite_clock_error_hz: f64,
    /// Doppler thermal-noise contribution in hertz.
    pub doppler_thermal_noise_hz: f64,
    /// Core Doppler quantization contribution in hertz.
    pub doppler_quantization_hz: f64,
}

/// Simulate a deterministic synthetic GNSS scenario from JSON text.
///
/// Safety: data must point to len readable UTF-8 bytes; out_simulation must
/// point to storage for a SidereonScenarioSimulation*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_scenario_simulate_json(
    data: *const u8,
    len: usize,
    out_simulation: *mut *mut SidereonScenarioSimulation,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_scenario_simulate_json",
        SidereonStatus::Panic,
        || {
            let out_simulation = c_try!(require_out(
                out_simulation,
                "sidereon_scenario_simulate_json",
                "out_simulation"
            ));
            *out_simulation = ptr::null_mut();
            let text = c_try!(ndm_text_from_utf8(
                data,
                len,
                "sidereon_scenario_simulate_json"
            ));
            let scenario: sidereon_core::scenario::Scenario = match serde_json::from_str(text) {
                Ok(scenario) => scenario,
                Err(err) => {
                    set_last_error(format!("sidereon_scenario_simulate_json: {err}"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            let inner = match sidereon_core::scenario::simulate_scenario(&scenario) {
                Ok(set) => set,
                Err(err) => return map_scenario_error("sidereon_scenario_simulate_json", err),
            };
            let json = match serde_json::to_vec(&inner) {
                Ok(json) => json,
                Err(err) => {
                    set_last_error(format!("sidereon_scenario_simulate_json: {err}"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            write_boxed_handle(out_simulation, SidereonScenarioSimulation { inner, json });
            SidereonStatus::Ok
        },
    )
}

/// Simulate a deterministic synthetic GNSS scenario from JSON text and a parsed
/// IONEX product declared by the scenario's ionosphere model.
///
/// Safety: data must point to len readable UTF-8 bytes; ionex must be a live
/// handle from sidereon_ionex_parse; out_simulation must point to storage for a
/// SidereonScenarioSimulation*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_scenario_simulate_json_with_ionex(
    data: *const u8,
    len: usize,
    ionex: *const SidereonIonex,
    out_simulation: *mut *mut SidereonScenarioSimulation,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_scenario_simulate_json_with_ionex",
        SidereonStatus::Panic,
        || {
            c_try!(init_scenario_output(
                "sidereon_scenario_simulate_json_with_ionex",
                out_simulation,
            ));
            let scenario = c_try!(scenario_from_json(
                "sidereon_scenario_simulate_json_with_ionex",
                data,
                len,
            ));
            let ionex = c_try!(require_ref(
                ionex,
                "sidereon_scenario_simulate_json_with_ionex",
                "ionex"
            ));
            let ionex_identity = c_try!(scenario_ionex_identity(
                "sidereon_scenario_simulate_json_with_ionex",
                &scenario,
            ));
            let declared_ionex =
                sidereon_core::scenario::DeclaredIonexSource::new(&ionex.inner, &ionex_identity);
            let media = sidereon_core::scenario::ScenarioMediaSources {
                ionex: Some(declared_ionex),
            };
            let inner =
                match sidereon_core::scenario::simulate_scenario_with_media(&scenario, &media) {
                    Ok(set) => set,
                    Err(err) => {
                        return map_scenario_error(
                            "sidereon_scenario_simulate_json_with_ionex",
                            err,
                        )
                    }
                };
            c_try!(write_scenario_simulation(
                "sidereon_scenario_simulate_json_with_ionex",
                out_simulation,
                inner,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Simulate a deterministic external-product scenario from JSON text and a
/// parsed SP3 product declared by the scenario constellation.
///
/// Safety: data must point to len readable UTF-8 bytes; sp3 must be a live
/// handle from sidereon_sp3_load or sidereon_sp3_merge; out_simulation must
/// point to storage for a SidereonScenarioSimulation*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_scenario_simulate_json_with_sp3(
    data: *const u8,
    len: usize,
    sp3: *const SidereonSp3,
    out_simulation: *mut *mut SidereonScenarioSimulation,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_scenario_simulate_json_with_sp3",
        SidereonStatus::Panic,
        || {
            c_try!(init_scenario_output(
                "sidereon_scenario_simulate_json_with_sp3",
                out_simulation,
            ));
            let scenario = c_try!(scenario_from_json(
                "sidereon_scenario_simulate_json_with_sp3",
                data,
                len,
            ));
            let sp3 = c_try!(require_ref(
                sp3,
                "sidereon_scenario_simulate_json_with_sp3",
                "sp3"
            ));
            let inner = c_try!(simulate_scenario_with_external_source(
                "sidereon_scenario_simulate_json_with_sp3",
                &scenario,
                sidereon_core::scenario::ScenarioExternalProductKind::Sp3,
                &sp3.inner,
                None,
            ));
            c_try!(write_scenario_simulation(
                "sidereon_scenario_simulate_json_with_sp3",
                out_simulation,
                inner,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Simulate a deterministic external-product scenario from JSON text, a parsed
/// SP3 product, and a parsed IONEX product declared by the scenario.
///
/// Safety: data must point to len readable UTF-8 bytes; sp3 and ionex must be
/// live handles; out_simulation must point to storage for a
/// SidereonScenarioSimulation*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_scenario_simulate_json_with_sp3_and_ionex(
    data: *const u8,
    len: usize,
    sp3: *const SidereonSp3,
    ionex: *const SidereonIonex,
    out_simulation: *mut *mut SidereonScenarioSimulation,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_scenario_simulate_json_with_sp3_and_ionex",
        SidereonStatus::Panic,
        || {
            c_try!(init_scenario_output(
                "sidereon_scenario_simulate_json_with_sp3_and_ionex",
                out_simulation,
            ));
            let scenario = c_try!(scenario_from_json(
                "sidereon_scenario_simulate_json_with_sp3_and_ionex",
                data,
                len,
            ));
            let sp3 = c_try!(require_ref(
                sp3,
                "sidereon_scenario_simulate_json_with_sp3_and_ionex",
                "sp3"
            ));
            let ionex = c_try!(require_ref(
                ionex,
                "sidereon_scenario_simulate_json_with_sp3_and_ionex",
                "ionex"
            ));
            let ionex_identity = c_try!(scenario_ionex_identity(
                "sidereon_scenario_simulate_json_with_sp3_and_ionex",
                &scenario,
            ));
            let declared_ionex =
                sidereon_core::scenario::DeclaredIonexSource::new(&ionex.inner, &ionex_identity);
            let media = sidereon_core::scenario::ScenarioMediaSources {
                ionex: Some(declared_ionex),
            };
            let inner = c_try!(simulate_scenario_with_external_source(
                "sidereon_scenario_simulate_json_with_sp3_and_ionex",
                &scenario,
                sidereon_core::scenario::ScenarioExternalProductKind::Sp3,
                &sp3.inner,
                Some(&media),
            ));
            c_try!(write_scenario_simulation(
                "sidereon_scenario_simulate_json_with_sp3_and_ionex",
                out_simulation,
                inner,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Simulate a deterministic external-product scenario from JSON text and a
/// parsed broadcast ephemeris declared by the scenario constellation.
///
/// Safety: data must point to len readable UTF-8 bytes; broadcast must be a live
/// handle from sidereon_broadcast_ephemeris_parse_nav; out_simulation must point
/// to storage for a SidereonScenarioSimulation*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_scenario_simulate_json_with_broadcast(
    data: *const u8,
    len: usize,
    broadcast: *const SidereonBroadcastEphemeris,
    out_simulation: *mut *mut SidereonScenarioSimulation,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_scenario_simulate_json_with_broadcast",
        SidereonStatus::Panic,
        || {
            c_try!(init_scenario_output(
                "sidereon_scenario_simulate_json_with_broadcast",
                out_simulation,
            ));
            let scenario = c_try!(scenario_from_json(
                "sidereon_scenario_simulate_json_with_broadcast",
                data,
                len,
            ));
            let broadcast = c_try!(require_ref(
                broadcast,
                "sidereon_scenario_simulate_json_with_broadcast",
                "broadcast"
            ));
            let inner = c_try!(simulate_scenario_with_external_source(
                "sidereon_scenario_simulate_json_with_broadcast",
                &scenario,
                sidereon_core::scenario::ScenarioExternalProductKind::Broadcast,
                &broadcast.inner,
                None,
            ));
            c_try!(write_scenario_simulation(
                "sidereon_scenario_simulate_json_with_broadcast",
                out_simulation,
                inner,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Simulate a deterministic external-product scenario from JSON text, parsed
/// broadcast ephemeris, and a parsed IONEX product declared by the scenario.
///
/// Safety: data must point to len readable UTF-8 bytes; broadcast and ionex must
/// be live handles; out_simulation must point to storage for a
/// SidereonScenarioSimulation*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_scenario_simulate_json_with_broadcast_and_ionex(
    data: *const u8,
    len: usize,
    broadcast: *const SidereonBroadcastEphemeris,
    ionex: *const SidereonIonex,
    out_simulation: *mut *mut SidereonScenarioSimulation,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_scenario_simulate_json_with_broadcast_and_ionex",
        SidereonStatus::Panic,
        || {
            c_try!(init_scenario_output(
                "sidereon_scenario_simulate_json_with_broadcast_and_ionex",
                out_simulation,
            ));
            let scenario = c_try!(scenario_from_json(
                "sidereon_scenario_simulate_json_with_broadcast_and_ionex",
                data,
                len,
            ));
            let broadcast = c_try!(require_ref(
                broadcast,
                "sidereon_scenario_simulate_json_with_broadcast_and_ionex",
                "broadcast"
            ));
            let ionex = c_try!(require_ref(
                ionex,
                "sidereon_scenario_simulate_json_with_broadcast_and_ionex",
                "ionex"
            ));
            let ionex_identity = c_try!(scenario_ionex_identity(
                "sidereon_scenario_simulate_json_with_broadcast_and_ionex",
                &scenario,
            ));
            let declared_ionex =
                sidereon_core::scenario::DeclaredIonexSource::new(&ionex.inner, &ionex_identity);
            let media = sidereon_core::scenario::ScenarioMediaSources {
                ionex: Some(declared_ionex),
            };
            let inner = c_try!(simulate_scenario_with_external_source(
                "sidereon_scenario_simulate_json_with_broadcast_and_ionex",
                &scenario,
                sidereon_core::scenario::ScenarioExternalProductKind::Broadcast,
                &broadcast.inner,
                Some(&media),
            ));
            c_try!(write_scenario_simulation(
                "sidereon_scenario_simulate_json_with_broadcast_and_ionex",
                out_simulation,
                inner,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Release a scenario simulation handle from sidereon_scenario_simulate_json.
/// Passing NULL is a no-op.
///
/// Safety: simulation must be NULL or a live SidereonScenarioSimulation handle
/// that has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_scenario_simulation_free(
    simulation: *mut SidereonScenarioSimulation,
) {
    ffi_boundary("sidereon_scenario_simulation_free", (), || {
        free_boxed(simulation);
    });
}

/// Copy a scenario simulation summary.
///
/// Safety: simulation must be a live handle and out must point to a
/// SidereonScenarioSummary.
#[no_mangle]
pub unsafe extern "C" fn sidereon_scenario_simulation_summary(
    simulation: *const SidereonScenarioSimulation,
    out: *mut SidereonScenarioSummary,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_scenario_simulation_summary",
        SidereonStatus::Panic,
        || {
            let simulation = c_try!(require_ref(
                simulation,
                "sidereon_scenario_simulation_summary",
                "simulation"
            ));
            let out = c_try!(require_out(
                out,
                "sidereon_scenario_simulation_summary",
                "out"
            ));
            *out = SidereonScenarioSummary {
                schema_version: simulation.inner.schema_version,
                seed: simulation.inner.seed,
                receiver_truth_count: simulation.inner.receiver_truth.len(),
                observation_count: simulation.inner.observation_count(),
                epoch_offset_count: simulation.inner.observations.epoch_offsets.len(),
                determinism_fingerprint: simulation.inner.determinism_fingerprint(),
                json_len: simulation.json.len(),
            };
            SidereonStatus::Ok
        },
    )
}

/// Copy the JSON serialization of a scenario simulation output set.
///
/// Safety: simulation must be a live handle; out must point to len bytes or
/// NULL when len is 0; out_written and out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_scenario_simulation_json(
    simulation: *const SidereonScenarioSimulation,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_scenario_simulation_json",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_scenario_simulation_json",
                out_written,
                out_required
            ));
            let simulation = c_try!(require_ref(
                simulation,
                "sidereon_scenario_simulation_json",
                "simulation"
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_scenario_simulation_json",
                "out",
                &simulation.json,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy epoch offsets from a scenario simulation.
///
/// Safety: simulation must be a live handle; out must point to len size_t
/// entries or NULL when len is 0; out_written and out_required must point to
/// size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_scenario_epoch_offsets(
    simulation: *const SidereonScenarioSimulation,
    out: *mut usize,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_scenario_epoch_offsets",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_scenario_epoch_offsets",
                out_written,
                out_required
            ));
            let simulation = c_try!(require_ref(
                simulation,
                "sidereon_scenario_epoch_offsets",
                "simulation"
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_scenario_epoch_offsets",
                "out",
                &simulation.inner.observations.epoch_offsets,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy receiver truth rows from a scenario simulation.
///
/// Safety: simulation must be a live handle; out must point to len rows or NULL
/// when len is 0; out_written and out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_scenario_receiver_truth(
    simulation: *const SidereonScenarioSimulation,
    out: *mut SidereonScenarioReceiverTruth,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_scenario_receiver_truth",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_scenario_receiver_truth",
                out_written,
                out_required
            ));
            let simulation = c_try!(require_ref(
                simulation,
                "sidereon_scenario_receiver_truth",
                "simulation"
            ));
            let rows: Vec<_> = simulation
                .inner
                .receiver_truth
                .iter()
                .map(receiver_truth_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_scenario_receiver_truth",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy synthetic observation rows from a scenario simulation.
///
/// Safety: simulation must be a live handle; out must point to len rows or NULL
/// when len is 0; out_written and out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_scenario_observations(
    simulation: *const SidereonScenarioSimulation,
    out: *mut SidereonScenarioObservation,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_scenario_observations",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_scenario_observations",
                out_written,
                out_required
            ));
            let simulation = c_try!(require_ref(
                simulation,
                "sidereon_scenario_observations",
                "simulation"
            ));
            let rows = scenario_observations_to_c(&simulation.inner);
            c_try!(copy_prefix_to_c(
                "sidereon_scenario_observations",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy ground-truth term ledger rows from a scenario simulation.
///
/// Safety: simulation must be a live handle; out must point to len rows or NULL
/// when len is 0; out_written and out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_scenario_terms(
    simulation: *const SidereonScenarioSimulation,
    out: *mut SidereonScenarioTerms,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_scenario_terms", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_scenario_terms",
            out_written,
            out_required
        ));
        let simulation = c_try!(require_ref(
            simulation,
            "sidereon_scenario_terms",
            "simulation"
        ));
        let rows = scenario_terms_to_c(&simulation.inner.truth_terms);
        c_try!(copy_prefix_to_c(
            "sidereon_scenario_terms",
            "out",
            &rows,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

fn receiver_truth_to_c(
    truth: &sidereon_core::scenario::SyntheticReceiverTruth,
) -> SidereonScenarioReceiverTruth {
    SidereonScenarioReceiverTruth {
        t_rx_j2000_s: truth.t_rx_j2000_s,
        position_ecef_m: truth.position_ecef_m,
        velocity_ecef_m_s: truth.velocity_ecef_m_s,
        clock_m: truth.clock_m,
        clock_rate_m_s: truth.clock_rate_m_s,
    }
}

fn scenario_observations_to_c(
    set: &sidereon_core::scenario::SyntheticObservationSet,
) -> Vec<SidereonScenarioObservation> {
    let observations = &set.observations;
    let mut rows = Vec::with_capacity(set.observation_count());
    for idx in 0..set.observation_count() {
        rows.push(SidereonScenarioObservation {
            epoch_index: observations.epoch_index[idx],
            sat_id: satellite_token(observations.satellite_id[idx]),
            code_observable: fixed_c_chars(&observations.code_observable[idx]),
            phase_observable: fixed_c_chars(&observations.phase_observable[idx]),
            doppler_observable: fixed_c_chars(&observations.doppler_observable[idx]),
            carrier_hz: observations.carrier_hz[idx],
            pseudorange_m: observations.pseudorange_m[idx],
            carrier_phase_cycles: observations.carrier_phase_cycles[idx],
            doppler_hz: observations.doppler_hz[idx],
        });
    }
    rows
}

fn scenario_terms_to_c(
    terms: &sidereon_core::scenario::SyntheticTermArrays,
) -> Vec<SidereonScenarioTerms> {
    let count = terms.geometric_range_m.len();
    let mut rows = Vec::with_capacity(count);
    for idx in 0..count {
        rows.push(SidereonScenarioTerms {
            geometric_range_m: terms.geometric_range_m[idx],
            satellite_clock_m: terms.satellite_clock_m[idx],
            receiver_clock_m: terms.receiver_clock_m[idx],
            satellite_clock_error_m: terms.satellite_clock_error_m[idx],
            ionosphere_m: terms.ionosphere_m[idx],
            troposphere_m: terms.troposphere_m[idx],
            thermal_noise_m: terms.thermal_noise_m[idx],
            multipath_m: terms.multipath_m[idx],
            quantization_m: terms.quantization_m[idx],
            carrier_phase_geometric_cycles: terms.carrier_phase_geometric_cycles[idx],
            carrier_phase_receiver_clock_cycles: terms.carrier_phase_receiver_clock_cycles[idx],
            carrier_phase_satellite_clock_cycles: terms.carrier_phase_satellite_clock_cycles[idx],
            carrier_phase_satellite_clock_error_cycles: terms
                .carrier_phase_satellite_clock_error_cycles[idx],
            carrier_phase_ionosphere_cycles: terms.carrier_phase_ionosphere_cycles[idx],
            carrier_phase_troposphere_cycles: terms.carrier_phase_troposphere_cycles[idx],
            carrier_phase_thermal_noise_cycles: terms.carrier_phase_thermal_noise_cycles[idx],
            carrier_phase_bias_cycles: terms.carrier_phase_bias_cycles[idx],
            carrier_phase_quantization_cycles: terms.carrier_phase_quantization_cycles[idx],
            doppler_satellite_motion_hz: terms.doppler_satellite_motion_hz[idx],
            doppler_receiver_motion_hz: terms.doppler_receiver_motion_hz[idx],
            doppler_satellite_clock_hz: terms.doppler_satellite_clock_hz[idx],
            doppler_receiver_clock_hz: terms.doppler_receiver_clock_hz[idx],
            doppler_satellite_clock_error_hz: terms.doppler_satellite_clock_error_hz[idx],
            doppler_thermal_noise_hz: terms.doppler_thermal_noise_hz[idx],
            doppler_quantization_hz: terms.doppler_quantization_hz[idx],
        });
    }
    rows
}

unsafe fn init_scenario_output(
    fn_name: &str,
    out_simulation: *mut *mut SidereonScenarioSimulation,
) -> Result<(), SidereonStatus> {
    let out_simulation = require_out(out_simulation, fn_name, "out_simulation")?;
    *out_simulation = ptr::null_mut();
    Ok(())
}

unsafe fn scenario_from_json(
    fn_name: &str,
    data: *const u8,
    len: usize,
) -> Result<sidereon_core::scenario::Scenario, SidereonStatus> {
    let text = ndm_text_from_utf8(data, len, fn_name)?;
    match serde_json::from_str(text) {
        Ok(scenario) => Ok(scenario),
        Err(err) => {
            set_last_error(format!("{fn_name}: {err}"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn write_scenario_simulation(
    fn_name: &str,
    out_simulation: *mut *mut SidereonScenarioSimulation,
    inner: sidereon_core::scenario::SyntheticObservationSet,
) -> Result<(), SidereonStatus> {
    let out_simulation = unsafe { require_out(out_simulation, fn_name, "out_simulation")? };
    let json = match serde_json::to_vec(&inner) {
        Ok(json) => json,
        Err(err) => {
            set_last_error(format!("{fn_name}: {err}"));
            return Err(SidereonStatus::InvalidArgument);
        }
    };
    write_boxed_handle(out_simulation, SidereonScenarioSimulation { inner, json });
    Ok(())
}

fn scenario_constellation_identity(
    fn_name: &str,
    scenario: &sidereon_core::scenario::Scenario,
    expected_kind: sidereon_core::scenario::ScenarioExternalProductKind,
) -> Result<sidereon_core::scenario::ScenarioExternalProduct, SidereonStatus> {
    match &scenario.constellation {
        sidereon_core::scenario::ScenarioConstellation::ExternalProducts { source, .. }
            if source.kind == expected_kind =>
        {
            Ok(source.clone())
        }
        sidereon_core::scenario::ScenarioConstellation::ExternalProducts { source, .. } => {
            set_last_error(format!(
                "{fn_name}: scenario constellation source kind is {:?}, expected {:?}",
                source.kind, expected_kind
            ));
            Err(SidereonStatus::InvalidArgument)
        }
        sidereon_core::scenario::ScenarioConstellation::SyntheticKeplerian { .. } => {
            set_last_error(format!(
                "{fn_name}: scenario constellation must use external products"
            ));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn scenario_ionex_identity(
    fn_name: &str,
    scenario: &sidereon_core::scenario::Scenario,
) -> Result<sidereon_core::scenario::ScenarioExternalProduct, SidereonStatus> {
    match &scenario.error_budget.ionosphere {
        sidereon_core::scenario::ScenarioIonosphereModel::SuppliedIonex { source } => {
            Ok(source.clone())
        }
        _ => {
            set_last_error(format!(
                "{fn_name}: scenario ionosphere model must use supplied_ionex"
            ));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn simulate_scenario_with_external_source<E>(
    fn_name: &str,
    scenario: &sidereon_core::scenario::Scenario,
    expected_kind: sidereon_core::scenario::ScenarioExternalProductKind,
    source: &E,
    media: Option<&sidereon_core::scenario::ScenarioMediaSources<'_>>,
) -> Result<sidereon_core::scenario::SyntheticObservationSet, SidereonStatus>
where
    E: sidereon_core::ephemeris::EphemerisSource
        + sidereon_core::observables::ObservableEphemerisSource,
{
    let identity = scenario_constellation_identity(fn_name, scenario, expected_kind)?;
    let declared = sidereon_core::scenario::DeclaredScenarioSource::new(source, identity);
    let result = match media {
        Some(media) => sidereon_core::scenario::simulate_scenario_with_source_and_media(
            scenario, &declared, media,
        ),
        None => sidereon_core::scenario::simulate_scenario_with_source(scenario, &declared),
    };
    match result {
        Ok(set) => Ok(set),
        Err(err) => Err(map_scenario_error(fn_name, err)),
    }
}

fn map_scenario_error(
    fn_name: &str,
    err: sidereon_core::scenario::ScenarioError,
) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        sidereon_core::scenario::ScenarioError::NoEphemeris { .. }
        | sidereon_core::scenario::ScenarioError::Observable(_) => SidereonStatus::Solve,
        sidereon_core::scenario::ScenarioError::InvalidInput { .. }
        | sidereon_core::scenario::ScenarioError::ExternalSourceRequired
        | sidereon_core::scenario::ScenarioError::ExternalSourceMismatch { .. }
        | sidereon_core::scenario::ScenarioError::ExternalIonosphereRequired
        | sidereon_core::scenario::ScenarioError::Ionosphere(_)
        | sidereon_core::scenario::ScenarioError::Frame(_) => SidereonStatus::InvalidArgument,
    }
}
