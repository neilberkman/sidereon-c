use super::*;

// --- SSR decode accessors, correction store, and corrected broadcast source --

pub struct SidereonSsrCorrectionStore {
    pub(crate) inner: SsrCorrectionStore,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonRtcmSsrKind {
    Orbit = 0,
    Clock = 1,
    CombinedOrbitClock = 2,
    CodeBias = 3,
    PhaseBias = 4,
    Ura = 5,
    HighRateClock = 6,
    Vtec = 7,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSsrReferencePoint {
    AntennaPhaseCenter = 0,
    CenterOfMass = 1,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSsrMissingCorrectionAction {
    Decline = 0,
    FallBackToBroadcast = 1,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmSsrHeader {
    pub epoch_time_s: u32,
    pub update_interval: u8,
    pub multiple_message: bool,
    pub iod_ssr: u8,
    pub provider_id: u16,
    pub solution_id: u8,
    pub has_satellite_reference_datum: bool,
    pub satellite_reference_datum: bool,
    pub has_dispersive_bias_consistency: bool,
    pub dispersive_bias_consistency: bool,
    pub has_mw_consistency: bool,
    pub mw_consistency: bool,
    pub satellite_count: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmSsrInfo {
    pub message_number: u16,
    pub system: SidereonGnssSystem,
    pub kind: SidereonRtcmSsrKind,
    pub header: SidereonRtcmSsrHeader,
    pub orbit_count: usize,
    pub clock_count: usize,
    pub ura_count: usize,
    pub code_bias_count: usize,
    pub phase_bias_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmSsrOrbitRecord {
    pub satellite_id: u8,
    pub iode: u32,
    pub delta_radial: i32,
    pub delta_along: i32,
    pub delta_cross: i32,
    pub dot_delta_radial: i32,
    pub dot_delta_along: i32,
    pub dot_delta_cross: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmSsrClockRecord {
    pub satellite_id: u8,
    pub c0: i32,
    pub c1: i32,
    pub c2: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmSsrUraRecord {
    pub satellite_id: u8,
    pub ura_index: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSsrOrbitCorrection {
    pub source: u32,
    pub provider_id: u16,
    pub solution_id: u8,
    pub iode: u32,
    pub iod_ssr: u8,
    pub crs_regional: bool,
    pub reference_point: SidereonSsrReferencePoint,
    pub radial_m: f64,
    pub along_m: f64,
    pub cross_m: f64,
    pub radial_rate_m_s: f64,
    pub along_rate_m_s: f64,
    pub cross_rate_m_s: f64,
    pub ref_epoch_j2000_s: f64,
    pub update_interval_s: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSsrClockCorrection {
    pub source: u32,
    pub provider_id: u16,
    pub solution_id: u8,
    pub iod_ssr: u8,
    pub c0_m: f64,
    pub c1_m_s: f64,
    pub c2_m_s2: f64,
    pub ref_epoch_j2000_s: f64,
    pub update_interval_s: f64,
    pub has_high_rate: bool,
    pub high_rate_c0_m: f64,
    pub high_rate_ref_epoch_j2000_s: f64,
    pub high_rate_update_interval_s: f64,
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_store_new(
    reference_point: u32,
    out_store: *mut *mut SidereonSsrCorrectionStore,
) -> SidereonStatus {
    ffi_boundary("sidereon_ssr_store_new", SidereonStatus::Panic, || {
        let out = c_try!(require_out(
            out_store,
            "sidereon_ssr_store_new",
            "out_store"
        ));
        *out = ptr::null_mut();
        let reference_point = c_try!(ssr_reference_point_from_c(
            "sidereon_ssr_store_new",
            reference_point
        ));
        write_boxed_handle(
            out,
            SidereonSsrCorrectionStore {
                inner: SsrCorrectionStore::new().with_reference_point(reference_point),
            },
        );
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_store_from_rtcm(
    bytes: *const u8,
    len: usize,
    epoch: *const SidereonGnssWeekTow,
    out_store: *mut *mut SidereonSsrCorrectionStore,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ssr_store_from_rtcm",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_store,
                "sidereon_ssr_store_from_rtcm",
                "out_store"
            ));
            *out = ptr::null_mut();
            let bytes = c_try!(require_slice(
                bytes,
                len,
                "sidereon_ssr_store_from_rtcm",
                "bytes"
            ));
            let epoch = c_try!(require_ref(epoch, "sidereon_ssr_store_from_rtcm", "epoch"));
            let epoch = c_try!(gnss_week_tow_from_c("sidereon_ssr_store_from_rtcm", epoch));
            let inner = c_try!(guard(SidereonStatus::InvalidArgument, || {
                sidereon::ssr_store_from_rtcm(bytes, epoch)
            }));
            write_boxed_handle(out, SidereonSsrCorrectionStore { inner });
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_store_ingest_messages(
    store: *mut SidereonSsrCorrectionStore,
    messages: *const SidereonRtcmMessages,
    epoch: *const SidereonGnssWeekTow,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ssr_store_ingest_messages",
        SidereonStatus::Panic,
        || {
            let store = c_try!(require_out(
                store,
                "sidereon_ssr_store_ingest_messages",
                "store"
            ));
            let messages = c_try!(require_ref(
                messages,
                "sidereon_ssr_store_ingest_messages",
                "messages"
            ));
            let epoch = c_try!(require_ref(
                epoch,
                "sidereon_ssr_store_ingest_messages",
                "epoch"
            ));
            let epoch = c_try!(gnss_week_tow_from_c(
                "sidereon_ssr_store_ingest_messages",
                epoch
            ));
            for message in &messages.messages {
                if let Err(err) = store.inner.ingest(message, epoch) {
                    return map_ssr_error("sidereon_ssr_store_ingest_messages", err);
                }
            }
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_store_orbit(
    store: *const SidereonSsrCorrectionStore,
    sat_id: *const c_char,
    out_present: *mut bool,
    out_orbit: *mut SidereonSsrOrbitCorrection,
) -> SidereonStatus {
    ffi_boundary("sidereon_ssr_store_orbit", SidereonStatus::Panic, || {
        let out_present = c_try!(require_out(
            out_present,
            "sidereon_ssr_store_orbit",
            "out_present"
        ));
        *out_present = false;
        let out = c_try!(require_out(
            out_orbit,
            "sidereon_ssr_store_orbit",
            "out_orbit"
        ));
        *out = SidereonSsrOrbitCorrection {
            source: 0,
            provider_id: 0,
            solution_id: 0,
            iode: 0,
            iod_ssr: 0,
            crs_regional: false,
            reference_point: SidereonSsrReferencePoint::CenterOfMass,
            radial_m: 0.0,
            along_m: 0.0,
            cross_m: 0.0,
            radial_rate_m_s: 0.0,
            along_rate_m_s: 0.0,
            cross_rate_m_s: 0.0,
            ref_epoch_j2000_s: 0.0,
            update_interval_s: 0.0,
        };
        let store = c_try!(require_ref(store, "sidereon_ssr_store_orbit", "store"));
        let sat = c_try!(parse_satellite_token("sidereon_ssr_store_orbit", sat_id));
        if let Some(value) = store.inner.orbit(sat) {
            *out_present = true;
            *out = ssr_orbit_to_c(value);
        }
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_store_clock(
    store: *const SidereonSsrCorrectionStore,
    sat_id: *const c_char,
    out_present: *mut bool,
    out_clock: *mut SidereonSsrClockCorrection,
) -> SidereonStatus {
    ffi_boundary("sidereon_ssr_store_clock", SidereonStatus::Panic, || {
        let out_present = c_try!(require_out(
            out_present,
            "sidereon_ssr_store_clock",
            "out_present"
        ));
        *out_present = false;
        let out = c_try!(require_out(
            out_clock,
            "sidereon_ssr_store_clock",
            "out_clock"
        ));
        *out = SidereonSsrClockCorrection {
            source: 0,
            provider_id: 0,
            solution_id: 0,
            iod_ssr: 0,
            c0_m: 0.0,
            c1_m_s: 0.0,
            c2_m_s2: 0.0,
            ref_epoch_j2000_s: 0.0,
            update_interval_s: 0.0,
            has_high_rate: false,
            high_rate_c0_m: 0.0,
            high_rate_ref_epoch_j2000_s: 0.0,
            high_rate_update_interval_s: 0.0,
        };
        let store = c_try!(require_ref(store, "sidereon_ssr_store_clock", "store"));
        let sat = c_try!(parse_satellite_token("sidereon_ssr_store_clock", sat_id));
        if let Some(value) = store.inner.clock(sat) {
            *out_present = true;
            *out = ssr_clock_to_c(value);
        }
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_store_ura_index(
    store: *const SidereonSsrCorrectionStore,
    sat_id: *const c_char,
    out_present: *mut bool,
    out_ura_index: *mut u8,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ssr_store_ura_index",
        SidereonStatus::Panic,
        || {
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_ssr_store_ura_index",
                "out_present"
            ));
            *out_present = false;
            let out = c_try!(require_out(
                out_ura_index,
                "sidereon_ssr_store_ura_index",
                "out_ura_index"
            ));
            *out = 0;
            let store = c_try!(require_ref(store, "sidereon_ssr_store_ura_index", "store"));
            let sat = c_try!(parse_satellite_token(
                "sidereon_ssr_store_ura_index",
                sat_id
            ));
            if let Some(value) = store.inner.ura_index(sat) {
                *out_present = true;
                *out = value;
            }
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_store_code_bias_m(
    store: *const SidereonSsrCorrectionStore,
    sat_id: *const c_char,
    signal: u8,
    out_present: *mut bool,
    out_bias_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ssr_store_code_bias_m",
        SidereonStatus::Panic,
        || {
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_ssr_store_code_bias_m",
                "out_present"
            ));
            *out_present = false;
            let out = c_try!(require_out(
                out_bias_m,
                "sidereon_ssr_store_code_bias_m",
                "out_bias_m"
            ));
            *out = 0.0;
            let store = c_try!(require_ref(
                store,
                "sidereon_ssr_store_code_bias_m",
                "store"
            ));
            let sat = c_try!(parse_satellite_token(
                "sidereon_ssr_store_code_bias_m",
                sat_id
            ));
            if let Some(value) = store.inner.code_bias(sat, signal) {
                *out_present = true;
                *out = value;
            }
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_store_phase_bias_m(
    store: *const SidereonSsrCorrectionStore,
    sat_id: *const c_char,
    signal: u8,
    out_present: *mut bool,
    out_bias_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ssr_store_phase_bias_m",
        SidereonStatus::Panic,
        || {
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_ssr_store_phase_bias_m",
                "out_present"
            ));
            *out_present = false;
            let out = c_try!(require_out(
                out_bias_m,
                "sidereon_ssr_store_phase_bias_m",
                "out_bias_m"
            ));
            *out = 0.0;
            let store = c_try!(require_ref(
                store,
                "sidereon_ssr_store_phase_bias_m",
                "store"
            ));
            let sat = c_try!(parse_satellite_token(
                "sidereon_ssr_store_phase_bias_m",
                sat_id
            ));
            if let Some(value) = store.inner.phase_bias(sat, signal) {
                *out_present = true;
                *out = value;
            }
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_corrected_state(
    broadcast: *const SidereonBroadcastEphemeris,
    store: *const SidereonSsrCorrectionStore,
    sat_id: *const c_char,
    t_j2000_s: f64,
    staleness_s: f64,
    missing_action: u32,
    allow_regional_provider: bool,
    regional_provider_id: u16,
    out_present: *mut bool,
    out_position_ecef_m: *mut f64,
    out_clock_s: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ssr_corrected_state",
        SidereonStatus::Panic,
        || {
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_ssr_corrected_state",
                "out_present"
            ));
            *out_present = false;
            c_try!(require_out(
                out_position_ecef_m,
                "sidereon_ssr_corrected_state",
                "out_position_ecef_m"
            ));
            zero_f64_prefix(out_position_ecef_m, 3, 3);
            let out_clock = c_try!(require_out(
                out_clock_s,
                "sidereon_ssr_corrected_state",
                "out_clock_s"
            ));
            *out_clock = 0.0;
            let broadcast = c_try!(require_ref(
                broadcast,
                "sidereon_ssr_corrected_state",
                "broadcast"
            ));
            let store = c_try!(require_ref(store, "sidereon_ssr_corrected_state", "store"));
            let sat = c_try!(parse_satellite_token(
                "sidereon_ssr_corrected_state",
                sat_id
            ));
            let fallback = c_try!(ssr_fallback_from_c(
                "sidereon_ssr_corrected_state",
                missing_action,
                allow_regional_provider,
                regional_provider_id,
            ));
            let corrected = SsrCorrectedEphemeris::new(&broadcast.inner, &store.inner)
                .with_staleness(StalenessPolicy::seconds(staleness_s))
                .with_fallback(fallback);
            if let Some((position, clock)) = corrected.corrected_state(sat, t_j2000_s) {
                c_try!(copy_exact_f64s(
                    "sidereon_ssr_corrected_state",
                    "out_position_ecef_m",
                    out_position_ecef_m,
                    3,
                    &position
                ));
                *out_present = true;
                *out_clock = clock;
            }
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_solve_broadcast(
    broadcast: *const SidereonBroadcastEphemeris,
    store: *const SidereonSsrCorrectionStore,
    staleness_s: f64,
    missing_action: u32,
    allow_regional_provider: bool,
    regional_provider_id: u16,
    inputs: *const SidereonSppInputs,
    out_solution: *mut *mut SidereonSppSolution,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ssr_solve_broadcast",
        SidereonStatus::Panic,
        || {
            let out_solution = c_try!(require_out(
                out_solution,
                "sidereon_ssr_solve_broadcast",
                "out_solution"
            ));
            *out_solution = ptr::null_mut();
            let broadcast = c_try!(require_ref(
                broadcast,
                "sidereon_ssr_solve_broadcast",
                "broadcast"
            ));
            let store = c_try!(require_ref(store, "sidereon_ssr_solve_broadcast", "store"));
            let inputs = c_try!(require_ref(
                inputs,
                "sidereon_ssr_solve_broadcast",
                "inputs"
            ));
            let fallback = c_try!(ssr_fallback_from_c(
                "sidereon_ssr_solve_broadcast",
                missing_action,
                allow_regional_provider,
                regional_provider_id,
            ));
            let corrected = SsrCorrectedEphemeris::new(&broadcast.inner, &store.inner)
                .with_staleness(StalenessPolicy::seconds(staleness_s))
                .with_fallback(fallback);
            let solve_inputs = c_try!(build_spp_solve_inputs(
                "sidereon_ssr_solve_broadcast",
                inputs,
                None,
                None,
                BTreeMap::new(),
            ));
            let inner = c_try!(guard(SidereonStatus::Solve, || {
                sidereon::solve_spp(
                    &corrected,
                    &solve_inputs,
                    inputs.with_geodetic,
                    SolvePolicy::default(),
                )
            }));
            write_boxed_handle(out_solution, SidereonSppSolution { inner });
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_ephemeris_sample(
    broadcast: *const SidereonBroadcastEphemeris,
    store: *const SidereonSsrCorrectionStore,
    staleness_s: f64,
    missing_action: u32,
    allow_regional_provider: bool,
    regional_provider_id: u16,
    satellites: *const *const c_char,
    satellite_count: usize,
    start_j2000_s: f64,
    stop_j2000_s: f64,
    step_s: f64,
    out: *mut SidereonEphemerisSampleRow,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ssr_ephemeris_sample",
        SidereonStatus::Panic,
        || {
            let broadcast = c_try!(require_ref(
                broadcast,
                "sidereon_ssr_ephemeris_sample",
                "broadcast"
            ));
            let store = c_try!(require_ref(store, "sidereon_ssr_ephemeris_sample", "store"));
            let fallback = c_try!(ssr_fallback_from_c(
                "sidereon_ssr_ephemeris_sample",
                missing_action,
                allow_regional_provider,
                regional_provider_id,
            ));
            let corrected = SsrCorrectedEphemeris::new(&broadcast.inner, &store.inner)
                .with_staleness(StalenessPolicy::seconds(staleness_s))
                .with_fallback(fallback);
            ephemeris_sample_common(
                "sidereon_ssr_ephemeris_sample",
                &corrected,
                satellites,
                satellite_count,
                start_j2000_s,
                stop_j2000_s,
                step_s,
                out,
                len,
                out_written,
                out_required,
            )
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_store_free(store: *mut SidereonSsrCorrectionStore) {
    free_boxed(store);
}

// ============================================================================
// Newly merged core features: full NeQuick-G slant integration, the standalone
// range RAIM/FDE design, the RTK and PPP arc drivers, and RTCM 3 from-scratch
// message construction. Every function below marshals C input into the cited
// sidereon-core type, calls the engine entry point, and copies the result back.
// No modeling lives here.

fn ssr_reference_point_from_c(
    fn_name: &str,
    value: u32,
) -> Result<OrbitReferencePoint, SidereonStatus> {
    match value {
        v if v == SidereonSsrReferencePoint::AntennaPhaseCenter as u32 => {
            Ok(OrbitReferencePoint::AntennaPhaseCenter)
        }
        v if v == SidereonSsrReferencePoint::CenterOfMass as u32 => {
            Ok(OrbitReferencePoint::CenterOfMass)
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid SSR reference point"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn ssr_fallback_from_c(
    fn_name: &str,
    missing_action: u32,
    allow_regional_provider: bool,
    regional_provider_id: u16,
) -> Result<SsrFallbackPolicy, SidereonStatus> {
    let regional = if allow_regional_provider {
        let mut providers = BTreeSet::new();
        providers.insert(regional_provider_id);
        RegionalPolicy::AllowProviders(providers)
    } else {
        RegionalPolicy::DeclineRegional
    };
    Ok(SsrFallbackPolicy {
        on_missing_correction: ssr_missing_action_from_c(fn_name, missing_action)?,
        regional,
    })
}

fn ssr_orbit_to_c(value: &SsrOrbitCorrection) -> SidereonSsrOrbitCorrection {
    SidereonSsrOrbitCorrection {
        source: match value.solution.source {
            sidereon_core::ssr::SsrSource::RtcmSsr => 0,
            sidereon_core::ssr::SsrSource::GalileoHas => 1,
        },
        provider_id: value.solution.provider_id,
        solution_id: value.solution.solution_id,
        iode: value.iode,
        iod_ssr: value.iod_ssr,
        crs_regional: value.crs_regional,
        reference_point: ssr_reference_point_to_c(value.reference_point),
        radial_m: value.radial_m,
        along_m: value.along_m,
        cross_m: value.cross_m,
        radial_rate_m_s: value.radial_rate_m_s,
        along_rate_m_s: value.along_rate_m_s,
        cross_rate_m_s: value.cross_rate_m_s,
        ref_epoch_j2000_s: value.ref_epoch_j2000_s,
        update_interval_s: value.update_interval_s,
    }
}

fn ssr_clock_to_c(value: &SsrClockCorrection) -> SidereonSsrClockCorrection {
    SidereonSsrClockCorrection {
        source: match value.solution.source {
            sidereon_core::ssr::SsrSource::RtcmSsr => 0,
            sidereon_core::ssr::SsrSource::GalileoHas => 1,
        },
        provider_id: value.solution.provider_id,
        solution_id: value.solution.solution_id,
        iod_ssr: value.iod_ssr,
        c0_m: value.c0_m,
        c1_m_s: value.c1_m_s,
        c2_m_s2: value.c2_m_s2,
        ref_epoch_j2000_s: value.ref_epoch_j2000_s,
        update_interval_s: value.update_interval_s,
        has_high_rate: value.high_rate.is_some(),
        high_rate_c0_m: value.high_rate.map(|h| h.c0_m).unwrap_or(0.0),
        high_rate_ref_epoch_j2000_s: value.high_rate.map(|h| h.ref_epoch_j2000_s).unwrap_or(0.0),
        high_rate_update_interval_s: value.high_rate.map(|h| h.update_interval_s).unwrap_or(0.0),
    }
}

fn map_ssr_error(fn_name: &str, err: CoreError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        CoreError::InvalidInput(_) | CoreError::Parse(_) => SidereonStatus::InvalidArgument,
        _ => SidereonStatus::Solve,
    }
}

fn ssr_reference_point_to_c(value: OrbitReferencePoint) -> SidereonSsrReferencePoint {
    match value {
        OrbitReferencePoint::AntennaPhaseCenter => SidereonSsrReferencePoint::AntennaPhaseCenter,
        OrbitReferencePoint::CenterOfMass => SidereonSsrReferencePoint::CenterOfMass,
    }
}

fn ssr_missing_action_from_c(
    fn_name: &str,
    value: u32,
) -> Result<MissingCorrectionAction, SidereonStatus> {
    match value {
        v if v == SidereonSsrMissingCorrectionAction::Decline as u32 => {
            Ok(MissingCorrectionAction::Decline)
        }
        v if v == SidereonSsrMissingCorrectionAction::FallBackToBroadcast as u32 => {
            Ok(MissingCorrectionAction::FallBackToBroadcast)
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid SSR missing-correction action"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}
