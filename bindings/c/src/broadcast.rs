use super::*;

/// Why the fallback produced a fix from broadcast ephemeris. Mirrors
/// sidereon_core::positioning::BroadcastReason.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonBroadcastReasonKind {
    /// The precise staleness selection declined outright; the carried selection
    /// status is the exact reason.
    PreciseUnavailable = 0,
    /// A stale (within-cap) precise product was selected but could not serve the
    /// requested epoch, so broadcast was used; the attempted product's staleness
    /// is carried.
    PreciseDegradedUnusable = 1,
}

/// A broadcast (navigation-message) ephemeris source for the supported real-time
/// / offline SPP mode. Opaque to C. Create with
/// sidereon_broadcast_ephemeris_parse_nav and release with
/// sidereon_broadcast_ephemeris_free.
pub struct SidereonBroadcastEphemeris {
    pub(crate) inner: BroadcastEphemeris,
}

/// Parse a RINEX navigation file into a broadcast ephemeris source, keeping the
/// records usable for single-frequency positioning (the engine's default
/// usability policy). On success writes a newly owned handle to *out_broadcast.
/// Release it with sidereon_broadcast_ephemeris_free.
///
/// Safety: data must point to len readable bytes; out_broadcast must point to
/// storage for a SidereonBroadcastEphemeris*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_broadcast_ephemeris_parse_nav(
    data: *const u8,
    len: usize,
    out_broadcast: *mut *mut SidereonBroadcastEphemeris,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_broadcast_ephemeris_parse_nav",
        SidereonStatus::Panic,
        || {
            let out_broadcast = c_try!(require_out(
                out_broadcast,
                "sidereon_broadcast_ephemeris_parse_nav",
                "out_broadcast"
            ));
            *out_broadcast = ptr::null_mut();
            let bytes = c_try!(require_slice(
                data,
                len,
                "sidereon_broadcast_ephemeris_parse_nav",
                "data"
            ));
            let text = match str::from_utf8(bytes) {
                Ok(text) => text,
                Err(_) => {
                    set_last_error(
                        "sidereon_broadcast_ephemeris_parse_nav: data is not valid UTF-8"
                            .to_string(),
                    );
                    return SidereonStatus::InvalidToken;
                }
            };
            let inner = match BroadcastEphemeris::from_nav(text) {
                Ok(inner) => inner,
                Err(err) => {
                    set_last_error(format!("sidereon_broadcast_ephemeris_parse_nav: {err}"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            write_boxed_handle(out_broadcast, SidereonBroadcastEphemeris { inner });
            SidereonStatus::Ok
        },
    )
}

/// Read and parse a RINEX navigation file from a UTF-8 filesystem path into a
/// broadcast ephemeris source. On success writes a newly owned handle to
/// *out_broadcast. Release it with sidereon_broadcast_ephemeris_free. Delegates
/// to sidereon::load_rinex_nav.
///
/// Safety: path must be a non-empty UTF-8 C string; out_broadcast must point to
/// storage for a SidereonBroadcastEphemeris*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_broadcast_ephemeris_load_nav(
    path: *const c_char,
    out_broadcast: *mut *mut SidereonBroadcastEphemeris,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_broadcast_ephemeris_load_nav",
        SidereonStatus::Panic,
        || {
            let out_broadcast = c_try!(require_out(
                out_broadcast,
                "sidereon_broadcast_ephemeris_load_nav",
                "out_broadcast"
            ));
            *out_broadcast = ptr::null_mut();
            let path = c_try!(parse_c_string(
                "sidereon_broadcast_ephemeris_load_nav",
                "path",
                path
            ));
            let inner = match sidereon::load_rinex_nav(&path) {
                Ok(inner) => inner,
                Err(err) => {
                    set_last_error(format!("sidereon_broadcast_ephemeris_load_nav: {err}"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            write_boxed_handle(out_broadcast, SidereonBroadcastEphemeris { inner });
            SidereonStatus::Ok
        },
    )
}

/// Release a broadcast ephemeris handle from
/// sidereon_broadcast_ephemeris_parse_nav. Passing NULL is a no-op.
///
/// Safety: broadcast must be NULL or a live handle from
/// sidereon_broadcast_ephemeris_parse_nav that has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_broadcast_ephemeris_free(
    broadcast: *mut SidereonBroadcastEphemeris,
) {
    ffi_boundary("sidereon_broadcast_ephemeris_free", (), || {
        free_boxed(broadcast);
    });
}

/// Predict one satellite's observables from a loaded broadcast (navigation
/// message) source. Delegates to sidereon_core::observables::predict. options
/// may be NULL for the engine defaults.
///
/// Safety: broadcast must be a live handle from
/// sidereon_broadcast_ephemeris_parse_nav; sat_id must be a null-terminated
/// token; receiver_ecef_m must point to three readable doubles; options must be
/// NULL or point to a SidereonObservablesOptions; out must point to a
/// SidereonPredictedObservables.
#[no_mangle]
pub unsafe extern "C" fn sidereon_broadcast_observables(
    broadcast: *const SidereonBroadcastEphemeris,
    sat_id: *const c_char,
    receiver_ecef_m: *const f64,
    t_rx_j2000_s: f64,
    options: *const SidereonObservablesOptions,
    out: *mut SidereonPredictedObservables,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_broadcast_observables",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_broadcast_observables", "out"));
            let broadcast = c_try!(require_ref(
                broadcast,
                "sidereon_broadcast_observables",
                "broadcast"
            ));
            let satellite = c_try!(parse_satellite_token(
                "sidereon_broadcast_observables",
                sat_id
            ));
            let receiver = c_try!(require_slice(
                receiver_ecef_m,
                3,
                "sidereon_broadcast_observables",
                "receiver_ecef_m"
            ));
            let receiver_ecef_m = [receiver[0], receiver[1], receiver[2]];
            let opts = c_try!(predict_options_from_c(
                "sidereon_broadcast_observables",
                options
            ));
            let obs = match observables_predict(
                &broadcast.inner,
                satellite,
                receiver_ecef_m,
                t_rx_j2000_s,
                opts,
            ) {
                Ok(obs) => obs,
                Err(err) => return map_observables_error("sidereon_broadcast_observables", err),
            };
            *out = predicted_observables_to_c(&obs);
            SidereonStatus::Ok
        },
    )
}

// --- Broadcast orbit/clock evaluation (sidereon_core::ephemeris / observables) -

/// Solve Kepler's equation for the eccentric anomaly (radians). Writes the
/// converged value to *out_eccentric_anomaly_rad and the iteration count to
/// *out_iterations. Delegates to sidereon_core::ephemeris::eccentric_anomaly.
///
/// Safety: out_eccentric_anomaly_rad points to a double; out_iterations points to
/// a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_broadcast_eccentric_anomaly(
    mean_anomaly_rad: f64,
    eccentricity: f64,
    out_eccentric_anomaly_rad: *mut f64,
    out_iterations: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_broadcast_eccentric_anomaly",
        SidereonStatus::Panic,
        || {
            let out_value = c_try!(require_out(
                out_eccentric_anomaly_rad,
                "sidereon_broadcast_eccentric_anomaly",
                "out_eccentric_anomaly_rad"
            ));
            *out_value = 0.0;
            let out_iterations = c_try!(require_out(
                out_iterations,
                "sidereon_broadcast_eccentric_anomaly",
                "out_iterations"
            ));
            *out_iterations = 0;
            match sidereon_core::ephemeris::eccentric_anomaly(mean_anomaly_rad, eccentricity) {
                Ok(ea) => {
                    *out_value = ea.value;
                    *out_iterations = ea.iterations;
                    SidereonStatus::Ok
                }
                Err(err) => {
                    set_last_error(format!("sidereon_broadcast_eccentric_anomaly: {err}"));
                    SidereonStatus::InvalidArgument
                }
            }
        },
    )
}

/// Evaluate a broadcast ephemeris at a J2000 second for one satellite, writing
/// the ECEF position (meters) and, when present, the satellite clock offset
/// (seconds). Delegates to the engine ObservableEphemerisSource implementation on
/// the broadcast store
/// (sidereon_core::observables::ObservableEphemerisSource::observable_state_at_j2000_s).
///
/// Safety: eph is a live broadcast handle; satellite_id is a null-terminated
/// token; out_position_ecef_m points to 3 doubles; out_clock_s points to a
/// double; out_has_clock points to a bool.
#[no_mangle]
pub unsafe extern "C" fn sidereon_broadcast_observable_state(
    eph: *const SidereonBroadcastEphemeris,
    satellite_id: *const c_char,
    t_j2000_s: f64,
    out_position_ecef_m: *mut f64,
    out_clock_s: *mut f64,
    out_has_clock: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_broadcast_observable_state",
        SidereonStatus::Panic,
        || {
            let eph = c_try!(require_ref(
                eph,
                "sidereon_broadcast_observable_state",
                "eph"
            ));
            observable_state_common(
                "sidereon_broadcast_observable_state",
                &eph.inner,
                satellite_id,
                t_j2000_s,
                out_position_ecef_m,
                out_clock_s,
                out_has_clock,
            )
        },
    )
}

// --- Broadcast-vs-precise comparison (sidereon_core::broadcast_comparison) ----

/// One comparison epoch, mirroring
/// sidereon_core::broadcast_comparison::EpochInputs. The precise/plus/minus
/// fields are split Julian dates (whole + fraction) at which the precise SP3 is
/// evaluated for the central, look-ahead, and look-back samples.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonCompareEpoch {
    /// Broadcast evaluation epoch in J2000 seconds.
    pub broadcast_t_j2000_s: f64,
    /// Central precise epoch, whole Julian day.
    pub precise_jd_whole: f64,
    /// Central precise epoch, Julian-day fraction.
    pub precise_jd_fraction: f64,
    /// Look-ahead precise epoch, whole Julian day.
    pub precise_plus_jd_whole: f64,
    /// Look-ahead precise epoch, Julian-day fraction.
    pub precise_plus_jd_fraction: f64,
    /// Look-back precise epoch, whole Julian day.
    pub precise_minus_jd_whole: f64,
    /// Look-back precise epoch, Julian-day fraction.
    pub precise_minus_jd_fraction: f64,
}

/// SISRE comparison statistics, mirroring
/// sidereon_core::broadcast_comparison::CompareStats. Each optional metric is NaN
/// when undefined for the sample.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonCompareStats {
    /// Number of compared samples.
    pub count: usize,
    /// 3D orbit error RMS, meters (NaN if undefined).
    pub orbit_3d_rms_m: f64,
    /// 3D orbit error max, meters.
    pub orbit_3d_max_m: f64,
    /// Radial error RMS, meters.
    pub radial_rms_m: f64,
    /// Radial error max, meters.
    pub radial_max_m: f64,
    /// Along-track error RMS, meters.
    pub along_rms_m: f64,
    /// Along-track error max, meters.
    pub along_max_m: f64,
    /// Cross-track error RMS, meters.
    pub cross_rms_m: f64,
    /// Cross-track error max, meters.
    pub cross_max_m: f64,
    /// Clock error RMS, meters.
    pub clock_rms_m: f64,
    /// Clock error max, meters.
    pub clock_max_m: f64,
    /// Datum-removed clock error RMS, meters.
    pub clock_datum_removed_rms_m: f64,
    /// Datum-removed clock error max, meters.
    pub clock_datum_removed_max_m: f64,
}

/// A broadcast-vs-precise comparison report. Opaque to C. Create with
/// sidereon_broadcast_comparison_compare; release with
/// sidereon_broadcast_comparison_free.
pub struct SidereonBroadcastComparison {
    pub(crate) inner: sidereon_core::broadcast_comparison::CompareReport,
}

/// Compare a broadcast ephemeris against a precise SP3 product over a set of
/// satellites and epochs. On success writes a newly owned report handle.
/// Delegates to sidereon_core::broadcast_comparison::compare.
///
/// Safety: broadcast and precise are live handles; satellites points to
/// satellite_count null-terminated tokens; epochs points to epoch_count
/// SidereonCompareEpoch; out_report points to a SidereonBroadcastComparison*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_broadcast_comparison_compare(
    broadcast: *const SidereonBroadcastEphemeris,
    precise: *const SidereonSp3,
    satellites: *const *const c_char,
    satellite_count: usize,
    epochs: *const SidereonCompareEpoch,
    epoch_count: usize,
    velocity_half_s: f64,
    out_report: *mut *mut SidereonBroadcastComparison,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_broadcast_comparison_compare",
        SidereonStatus::Panic,
        || {
            let fn_name = "sidereon_broadcast_comparison_compare";
            let out_report = c_try!(require_out(out_report, fn_name, "out_report"));
            *out_report = ptr::null_mut();
            let broadcast = c_try!(require_ref(broadcast, fn_name, "broadcast"));
            let precise = c_try!(require_ref(precise, fn_name, "precise"));
            let sat_ptrs = c_try!(require_slice(
                satellites,
                satellite_count,
                fn_name,
                "satellites"
            ));
            let mut sats = Vec::with_capacity(satellite_count);
            for ptr in sat_ptrs {
                sats.push(c_try!(parse_satellite_token(fn_name, *ptr)));
            }
            let epoch_rows = c_try!(require_slice(epochs, epoch_count, fn_name, "epochs"));
            let mut epochs_vec = Vec::with_capacity(epoch_count);
            for e in epoch_rows {
                let precise = c_try!(sidereon_core::astro::time::JulianDateSplit::new(
                    e.precise_jd_whole,
                    e.precise_jd_fraction
                )
                .map_err(|err| extra_invalid_arg(fn_name, err)));
                let precise_plus = c_try!(sidereon_core::astro::time::JulianDateSplit::new(
                    e.precise_plus_jd_whole,
                    e.precise_plus_jd_fraction
                )
                .map_err(|err| extra_invalid_arg(fn_name, err)));
                let precise_minus = c_try!(sidereon_core::astro::time::JulianDateSplit::new(
                    e.precise_minus_jd_whole,
                    e.precise_minus_jd_fraction
                )
                .map_err(|err| extra_invalid_arg(fn_name, err)));
                epochs_vec.push(sidereon_core::broadcast_comparison::EpochInputs {
                    broadcast_t_j2000_s: e.broadcast_t_j2000_s,
                    precise,
                    precise_plus,
                    precise_minus,
                });
            }
            match sidereon_core::broadcast_comparison::compare(
                &broadcast.inner,
                &precise.inner,
                &sats,
                &epochs_vec,
                velocity_half_s,
            ) {
                Ok(report) => {
                    write_boxed_handle(out_report, SidereonBroadcastComparison { inner: report });
                    SidereonStatus::Ok
                }
                Err(err) => {
                    set_last_error(format!("{fn_name}: {err}"));
                    SidereonStatus::InvalidArgument
                }
            }
        },
    )
}

/// Write the overall (all-satellite) comparison statistics to *out_stats.
///
/// Safety: report is a live handle; out_stats points to a SidereonCompareStats.
#[no_mangle]
pub unsafe extern "C" fn sidereon_broadcast_comparison_overall(
    report: *const SidereonBroadcastComparison,
    out_stats: *mut SidereonCompareStats,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_broadcast_comparison_overall",
        SidereonStatus::Panic,
        || {
            let out_stats = c_try!(require_out(
                out_stats,
                "sidereon_broadcast_comparison_overall",
                "out_stats"
            ));
            let report = c_try!(require_ref(
                report,
                "sidereon_broadcast_comparison_overall",
                "report"
            ));
            *out_stats = compare_stats_to_c(&report.inner.overall);
            SidereonStatus::Ok
        },
    )
}

/// Write the number of per-satellite comparison rows to *out_count.
///
/// Safety: report is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_broadcast_comparison_satellite_count(
    report: *const SidereonBroadcastComparison,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_broadcast_comparison_satellite_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_broadcast_comparison_satellite_count",
                "out_count"
            ));
            *out_count = 0;
            let report = c_try!(require_ref(
                report,
                "sidereon_broadcast_comparison_satellite_count",
                "report"
            ));
            *out_count = report.inner.per_satellite.len();
            SidereonStatus::Ok
        },
    )
}

/// Read one per-satellite comparison row: its token (null-terminated) into
/// out_sat_id and its statistics into out_stats.
///
/// Safety: report is a live handle; out_sat_id points to sat_id_len bytes;
/// out_stats points to a SidereonCompareStats.
#[no_mangle]
pub unsafe extern "C" fn sidereon_broadcast_comparison_satellite(
    report: *const SidereonBroadcastComparison,
    index: usize,
    out_sat_id: *mut c_char,
    sat_id_len: usize,
    out_stats: *mut SidereonCompareStats,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_broadcast_comparison_satellite",
        SidereonStatus::Panic,
        || {
            let fn_name = "sidereon_broadcast_comparison_satellite";
            let out_stats = c_try!(require_out(out_stats, fn_name, "out_stats"));
            let report = c_try!(require_ref(report, fn_name, "report"));
            let (sat, stats) = match report.inner.per_satellite.get(index) {
                Some(row) => row,
                None => {
                    set_last_error(format!("{fn_name}: index out of range"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            c_try!(write_c_token(
                fn_name,
                out_sat_id,
                sat_id_len,
                &sat.to_string()
            ));
            *out_stats = compare_stats_to_c(stats);
            SidereonStatus::Ok
        },
    )
}

/// Release a broadcast-comparison report handle. Passing NULL is a no-op.
///
/// Safety: report must be a handle from sidereon_broadcast_comparison_compare or
/// NULL.
#[no_mangle]
pub unsafe extern "C" fn sidereon_broadcast_comparison_free(
    report: *mut SidereonBroadcastComparison,
) {
    free_boxed(report);
}

// ============================================================================
// Round 2 capability-parity additions: harder-to-marshal core capabilities.
// Each function below is a thin extern-C wrapper: it normalizes C input,
// marshals it into the cited sidereon-core type, calls the reference function,
// and copies the result back. No modeling logic lives here.

/// Evaluate the broadcast ECEF orbit at a seconds-of-week from Keplerian
/// elements. Delegates to
/// sidereon_core::ephemeris::satellite_position_ecef.
///
/// Safety: elements points to a SidereonKeplerianElements; consts to a
/// SidereonConstellationConstants; out to a SidereonOrbitState.
#[no_mangle]
pub unsafe extern "C" fn sidereon_broadcast_satellite_position_ecef(
    elements: *const SidereonKeplerianElements,
    consts: *const SidereonConstellationConstants,
    t_sow_s: f64,
    is_geo: bool,
    out: *mut SidereonOrbitState,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_broadcast_satellite_position_ecef",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_broadcast_satellite_position_ecef",
                "out"
            ));
            *out = SidereonOrbitState::ZERO;
            let elements = c_try!(require_ref(
                elements,
                "sidereon_broadcast_satellite_position_ecef",
                "elements"
            ))
            .to_core();
            let consts = c_try!(require_ref(
                consts,
                "sidereon_broadcast_satellite_position_ecef",
                "consts"
            ))
            .to_core();
            match sidereon_core::ephemeris::satellite_position_ecef(
                &elements, &consts, t_sow_s, is_geo,
            ) {
                Ok(o) => {
                    *out = SidereonOrbitState::from_core(&o);
                    SidereonStatus::Ok
                }
                Err(err) => extra_invalid_arg("sidereon_broadcast_satellite_position_ecef", err),
            }
        },
    )
}

/// Evaluate the broadcast satellite-clock offset at a seconds-of-week.
/// Delegates to sidereon_core::ephemeris::satellite_clock_offset_s.
///
/// Safety: clock points to a SidereonClockPolynomial; consts to a
/// SidereonConstellationConstants; elements to a SidereonKeplerianElements; out
/// to a SidereonClockOffset.
#[no_mangle]
pub unsafe extern "C" fn sidereon_broadcast_satellite_clock_offset_s(
    clock: *const SidereonClockPolynomial,
    consts: *const SidereonConstellationConstants,
    elements: *const SidereonKeplerianElements,
    sin_e: f64,
    t_sow_s: f64,
    tgd_s: f64,
    out: *mut SidereonClockOffset,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_broadcast_satellite_clock_offset_s",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_broadcast_satellite_clock_offset_s",
                "out"
            ));
            *out = SidereonClockOffset::ZERO;
            let clock = c_try!(require_ref(
                clock,
                "sidereon_broadcast_satellite_clock_offset_s",
                "clock"
            ))
            .to_core();
            let consts = c_try!(require_ref(
                consts,
                "sidereon_broadcast_satellite_clock_offset_s",
                "consts"
            ))
            .to_core();
            let elements = c_try!(require_ref(
                elements,
                "sidereon_broadcast_satellite_clock_offset_s",
                "elements"
            ))
            .to_core();
            match sidereon_core::ephemeris::satellite_clock_offset_s(
                &clock, &consts, &elements, sin_e, t_sow_s, tgd_s,
            ) {
                Ok(c) => {
                    *out = SidereonClockOffset::from_core(&c);
                    SidereonStatus::Ok
                }
                Err(err) => extra_invalid_arg("sidereon_broadcast_satellite_clock_offset_s", err),
            }
        },
    )
}

/// Evaluate the broadcast orbit and clock at the same instant. Delegates to
/// sidereon_core::ephemeris::satellite_state.
///
/// Safety: elements/clock/consts point to their respective structs; out to a
/// SidereonSatelliteState.
#[no_mangle]
pub unsafe extern "C" fn sidereon_broadcast_satellite_state(
    elements: *const SidereonKeplerianElements,
    clock: *const SidereonClockPolynomial,
    consts: *const SidereonConstellationConstants,
    t_sow_s: f64,
    tgd_s: f64,
    is_geo: bool,
    out: *mut SidereonSatelliteState,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_broadcast_satellite_state",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_broadcast_satellite_state",
                "out"
            ));
            *out = SidereonSatelliteState {
                orbit: SidereonOrbitState::ZERO,
                clock: SidereonClockOffset::ZERO,
            };
            let elements = c_try!(require_ref(
                elements,
                "sidereon_broadcast_satellite_state",
                "elements"
            ))
            .to_core();
            let clock = c_try!(require_ref(
                clock,
                "sidereon_broadcast_satellite_state",
                "clock"
            ))
            .to_core();
            let consts = c_try!(require_ref(
                consts,
                "sidereon_broadcast_satellite_state",
                "consts"
            ))
            .to_core();
            match sidereon_core::ephemeris::satellite_state(
                &elements, &clock, &consts, t_sow_s, tgd_s, is_geo,
            ) {
                Ok(s) => {
                    *out = SidereonSatelliteState::from_core(&s);
                    SidereonStatus::Ok
                }
                Err(err) => extra_invalid_arg("sidereon_broadcast_satellite_state", err),
            }
        },
    )
}

// --- GPS LNAV navigation message (sidereon_core::navigation::lnav) ------------

/// GPS LNAV clock and ephemeris parameters in engineering units, mirroring
/// sidereon_core::navigation::lnav::LnavParams. Integer-typed IS-GPS-200 fields
/// are int64_t; scaled fields are double. The codec is exact-power-of-two
/// arithmetic, so a given parameter set encodes to one exact bit pattern.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonLnavParams {
    /// GPS week number (10-bit).
    pub week_number: i64,
    /// L2 code flag.
    pub l2_code: i64,
    /// L2 P data flag (encode-only; not recovered by decode).
    pub l2_p_data_flag: i64,
    /// User range accuracy index.
    pub ura_index: i64,
    /// SV health.
    pub sv_health: i64,
    /// Issue of data, clock.
    pub iodc: i64,
    /// Group delay differential T_GD, seconds.
    pub tgd: f64,
    /// Clock data reference time t_oc, seconds.
    pub toc: i64,
    /// Clock bias a_f0, seconds.
    pub af0: f64,
    /// Clock drift a_f1, seconds/second.
    pub af1: f64,
    /// Clock drift rate a_f2, seconds/second^2.
    pub af2: f64,
    /// Issue of data, ephemeris.
    pub iode: i64,
    /// Orbit radius sine correction C_rs, meters.
    pub crs: f64,
    /// Mean motion difference delta_n, semicircles/second.
    pub delta_n: f64,
    /// Mean anomaly M_0, semicircles.
    pub m0: f64,
    /// Latitude cosine correction C_uc, radians.
    pub cuc: f64,
    /// Eccentricity (dimensionless).
    pub eccentricity: f64,
    /// Latitude sine correction C_us, radians.
    pub cus: f64,
    /// Square root of semi-major axis, sqrt(meters).
    pub sqrt_a: f64,
    /// Ephemeris reference time t_oe, seconds.
    pub toe: i64,
    /// Fit interval flag.
    pub fit_interval_flag: i64,
    /// Age of data offset, seconds.
    pub aodo: i64,
    /// Inclination cosine correction C_ic, radians.
    pub cic: f64,
    /// Longitude of ascending node Omega_0, semicircles.
    pub omega0: f64,
    /// Inclination sine correction C_is, radians.
    pub cis: f64,
    /// Inclination angle i_0, semicircles.
    pub i0: f64,
    /// Orbit radius cosine correction C_rc, meters.
    pub crc: f64,
    /// Argument of perigee omega, semicircles.
    pub omega: f64,
    /// Rate of right ascension Omega_dot, semicircles/second.
    pub omega_dot: f64,
    /// Rate of inclination IDOT, semicircles/second.
    pub idot: f64,
}

/// LNAV TLM/HOW options accompanying an encode, mirroring
/// sidereon_core::navigation::lnav::LnavOptions. All fields are integer-typed.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonLnavOptions {
    /// Time of week count (17-bit).
    pub tow: i64,
    /// Alert flag.
    pub alert: i64,
    /// Anti-spoof flag.
    pub anti_spoof: i64,
    /// Integrity status flag.
    pub integrity: i64,
    /// TLM message (14-bit).
    pub tlm_message: i64,
}

/// Decoded LNAV clock and ephemeris parameters, mirroring
/// sidereon_core::navigation::lnav::LnavDecoded. Integer fields are recovered
/// exactly; scaled fields are the transmitted integer times the IS-GPS-200 LSB.
/// (l2_p_data_flag is an encode-only field and is not recovered.)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonLnavDecoded {
    /// GPS week number.
    pub week_number: i64,
    /// L2 code flag.
    pub l2_code: i64,
    /// User range accuracy index.
    pub ura_index: i64,
    /// SV health.
    pub sv_health: i64,
    /// Issue of data, clock.
    pub iodc: i64,
    /// Group delay differential T_GD, seconds.
    pub tgd: f64,
    /// Clock data reference time t_oc, seconds.
    pub toc: i64,
    /// Clock bias a_f0, seconds.
    pub af0: f64,
    /// Clock drift a_f1, seconds/second.
    pub af1: f64,
    /// Clock drift rate a_f2, seconds/second^2.
    pub af2: f64,
    /// Issue of data, ephemeris.
    pub iode: i64,
    /// Orbit radius sine correction C_rs, meters.
    pub crs: f64,
    /// Mean motion difference delta_n, semicircles/second.
    pub delta_n: f64,
    /// Mean anomaly M_0, semicircles.
    pub m0: f64,
    /// Latitude cosine correction C_uc, radians.
    pub cuc: f64,
    /// Eccentricity (dimensionless).
    pub eccentricity: f64,
    /// Latitude sine correction C_us, radians.
    pub cus: f64,
    /// Square root of semi-major axis, sqrt(meters).
    pub sqrt_a: f64,
    /// Ephemeris reference time t_oe, seconds.
    pub toe: i64,
    /// Fit interval flag.
    pub fit_interval_flag: i64,
    /// Age of data offset, seconds.
    pub aodo: i64,
    /// Inclination cosine correction C_ic, radians.
    pub cic: f64,
    /// Longitude of ascending node Omega_0, semicircles.
    pub omega0: f64,
    /// Inclination sine correction C_is, radians.
    pub cis: f64,
    /// Inclination angle i_0, semicircles.
    pub i0: f64,
    /// Orbit radius cosine correction C_rc, meters.
    pub crc: f64,
    /// Argument of perigee omega, semicircles.
    pub omega: f64,
    /// Rate of right ascension Omega_dot, semicircles/second.
    pub omega_dot: f64,
    /// Rate of inclination IDOT, semicircles/second.
    pub idot: f64,
}

/// Bit length of a single LNAV subframe (IS-GPS-200 Section 20.3.2). Each encode
/// output buffer and each decode input must hold exactly this many bytes (one
/// 0/1 bit per byte, most significant first).
pub const SIDEREON_LNAV_SUBFRAME_LENGTH: usize = 300;

// Pin the exported constant to the core definition: a divergence is a build error.

/// Encode GPS LNAV subframes 1-3 from clock and ephemeris parameters. Writes the
/// three 300-bit subframes (one 0/1 bit per byte, most significant first) into
/// out_sf1/out_sf2/out_sf3, each of which must hold subframe_len ==
/// SIDEREON_LNAV_SUBFRAME_LENGTH bytes. Out-of-range parameters report
/// SIDEREON_STATUS_INVALID_ARGUMENT. Delegates to
/// sidereon_core::navigation::lnav::encode.
///
/// Safety: params and opts point to valid structs; out_sf1/out_sf2/out_sf3 each
/// point to subframe_len writable bytes.
#[no_mangle]
pub unsafe extern "C" fn sidereon_lnav_encode(
    params: *const SidereonLnavParams,
    opts: *const SidereonLnavOptions,
    out_sf1: *mut u8,
    out_sf2: *mut u8,
    out_sf3: *mut u8,
    subframe_len: usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_lnav_encode", SidereonStatus::Panic, || {
        let fn_name = "sidereon_lnav_encode";
        let params = c_try!(require_ref(params, fn_name, "params"));
        let opts = c_try!(require_ref(opts, fn_name, "opts"));
        if subframe_len < SIDEREON_LNAV_SUBFRAME_LENGTH {
            set_last_error(format!(
                "{fn_name}: subframe_len must be at least {SIDEREON_LNAV_SUBFRAME_LENGTH}"
            ));
            return SidereonStatus::InvalidArgument;
        }
        let out1 = c_try!(require_out(out_sf1, fn_name, "out_sf1"));
        let out2 = c_try!(require_out(out_sf2, fn_name, "out_sf2"));
        let out3 = c_try!(require_out(out_sf3, fn_name, "out_sf3"));
        let core_params = lnav_params_from_c(params);
        let core_opts = lnav_options_from_c(opts);
        let subframes = match sidereon_core::navigation::lnav::encode(&core_params, &core_opts) {
            Ok(subframes) => subframes,
            Err(err) => {
                set_last_error(format!("{fn_name}: {err:?}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        for (dst, sf) in [out1, out2, out3].into_iter().zip(subframes.iter()) {
            ptr::copy_nonoverlapping(sf.as_ptr(), dst, sf.len());
        }
        SidereonStatus::Ok
    })
}

/// Decode GPS LNAV subframes 1-3 back into engineering-unit parameters. Each of
/// sf1/sf2/sf3 must hold exactly SIDEREON_LNAV_SUBFRAME_LENGTH 0/1 bit bytes. A
/// parity failure reports SIDEREON_STATUS_INVALID_ARGUMENT. Delegates to
/// sidereon_core::navigation::lnav::decode.
///
/// Safety: sf1/sf2/sf3 each point to their *_len readable bytes; out points to a
/// SidereonLnavDecoded.
#[no_mangle]
pub unsafe extern "C" fn sidereon_lnav_decode(
    sf1: *const u8,
    sf1_len: usize,
    sf2: *const u8,
    sf2_len: usize,
    sf3: *const u8,
    sf3_len: usize,
    out: *mut SidereonLnavDecoded,
) -> SidereonStatus {
    ffi_boundary("sidereon_lnav_decode", SidereonStatus::Panic, || {
        let fn_name = "sidereon_lnav_decode";
        let out = c_try!(require_out(out, fn_name, "out"));
        let sf1 = c_try!(require_slice(sf1, sf1_len, fn_name, "sf1"));
        let sf2 = c_try!(require_slice(sf2, sf2_len, fn_name, "sf2"));
        let sf3 = c_try!(require_slice(sf3, sf3_len, fn_name, "sf3"));
        match sidereon_core::navigation::lnav::decode(sf1, sf2, sf3) {
            Ok(decoded) => {
                *out = lnav_decoded_to_c(&decoded);
                SidereonStatus::Ok
            }
            Err(err) => {
                set_last_error(format!("{fn_name}: {err:?}"));
                SidereonStatus::InvalidArgument
            }
        }
    })
}

// --- Broadcast-vs-precise comparison over a sampled window -------------------

/// A regularly sampled comparison window, mirroring
/// sidereon_core::broadcast_comparison::CompareWindow. The broadcast product is
/// queried on the continuous J2000-second axis; the precise product is queried by
/// split Julian date advanced from precise_start in lockstep.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonCompareWindow {
    /// Inclusive broadcast query window start, continuous seconds since J2000.
    pub broadcast_window_start_j2000_s: f64,
    /// Inclusive broadcast query window end, continuous seconds since J2000.
    pub broadcast_window_end_j2000_s: f64,
    /// Precise query for the window start, whole Julian day.
    pub precise_start_jd_whole: f64,
    /// Precise query for the window start, Julian-day fraction.
    pub precise_start_jd_fraction: f64,
    /// Sampling step between consecutive epochs, seconds.
    pub step_s: f64,
    /// Velocity finite-difference half step, seconds.
    pub velocity_half_s: f64,
}

/// Compare a broadcast ephemeris against a precise SP3 product over a sampled
/// window. On success writes a newly owned report handle (read with the
/// sidereon_broadcast_comparison_* accessors, released with
/// sidereon_broadcast_comparison_free). Delegates to
/// sidereon_core::broadcast_comparison::compare_window.
///
/// Safety: broadcast and precise are live handles; satellites points to
/// satellite_count null-terminated tokens; window points to a
/// SidereonCompareWindow; out_report points to a SidereonBroadcastComparison*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_broadcast_comparison_compare_window(
    broadcast: *const SidereonBroadcastEphemeris,
    precise: *const SidereonSp3,
    satellites: *const *const c_char,
    satellite_count: usize,
    window: *const SidereonCompareWindow,
    out_report: *mut *mut SidereonBroadcastComparison,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_broadcast_comparison_compare_window",
        SidereonStatus::Panic,
        || {
            let fn_name = "sidereon_broadcast_comparison_compare_window";
            let out_report = c_try!(require_out(out_report, fn_name, "out_report"));
            *out_report = ptr::null_mut();
            let broadcast = c_try!(require_ref(broadcast, fn_name, "broadcast"));
            let precise = c_try!(require_ref(precise, fn_name, "precise"));
            let window = c_try!(require_ref(window, fn_name, "window"));
            let sat_ptrs = c_try!(require_slice(
                satellites,
                satellite_count,
                fn_name,
                "satellites"
            ));
            let mut sats = Vec::with_capacity(satellite_count);
            for ptr in sat_ptrs {
                sats.push(c_try!(parse_satellite_token(fn_name, *ptr)));
            }
            let precise_start = c_try!(sidereon_core::astro::time::JulianDateSplit::new(
                window.precise_start_jd_whole,
                window.precise_start_jd_fraction
            )
            .map_err(|err| extra_invalid_arg(fn_name, err)));
            let core_window = sidereon_core::broadcast_comparison::CompareWindow {
                broadcast_window_j2000_s: (
                    window.broadcast_window_start_j2000_s,
                    window.broadcast_window_end_j2000_s,
                ),
                precise_start,
                step_s: window.step_s,
                velocity_half_s: window.velocity_half_s,
            };
            match sidereon_core::broadcast_comparison::compare_window(
                &broadcast.inner,
                &precise.inner,
                &sats,
                &core_window,
            ) {
                Ok(report) => {
                    write_boxed_handle(out_report, SidereonBroadcastComparison { inner: report });
                    SidereonStatus::Ok
                }
                Err(err) => {
                    set_last_error(format!("{fn_name}: {err}"));
                    SidereonStatus::InvalidArgument
                }
            }
        },
    )
}

/// Predict observables for many `(satellite, receiver, epoch)` requests from a
/// loaded broadcast (navigation message) source in one call. Mirror of
/// sidereon_sp3_observables_batch for the broadcast source; same per-request
/// out/out_ok contract. Delegates to the core serial `predict_batch`.
///
/// Safety: broadcast must be a live handle; requests must point to count entries
/// (each with a valid sat_id); out and out_ok must each point to count writable
/// entries (or be NULL when count is 0); options must be NULL or point to a
/// SidereonObservablesOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_broadcast_observables_batch(
    broadcast: *const SidereonBroadcastEphemeris,
    requests: *const SidereonPredictRequest,
    count: usize,
    options: *const SidereonObservablesOptions,
    out: *mut SidereonPredictedObservables,
    out_ok: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_broadcast_observables_batch",
        SidereonStatus::Panic,
        || {
            let broadcast = c_try!(require_ref(
                broadcast,
                "sidereon_broadcast_observables_batch",
                "broadcast"
            ));
            let raw = c_try!(require_slice(
                requests,
                count,
                "sidereon_broadcast_observables_batch",
                "requests"
            ));
            // Guard the caller's output arrays (non-null when count > 0, no
            // element overflow) before writing them element-by-element below.
            c_try!(require_slice(
                out as *const SidereonPredictedObservables,
                count,
                "sidereon_broadcast_observables_batch",
                "out"
            ));
            c_try!(require_slice(
                out_ok as *const bool,
                count,
                "sidereon_broadcast_observables_batch",
                "out_ok"
            ));
            let opts = c_try!(predict_options_from_c(
                "sidereon_broadcast_observables_batch",
                options
            ));
            let parsed = c_try!(predict_requests_from_c(
                "sidereon_broadcast_observables_batch",
                raw
            ));
            let results = core_predict_batch(&broadcast.inner, &parsed, opts);
            write_predict_batch_results(&results, out, out_ok);
            SidereonStatus::Ok
        },
    )
}

/// Sample a loaded broadcast-navigation source over a regular grid.
///
/// Safety: broadcast must be a live handle; satellites points to satellite_count
/// null-terminated tokens; out points to len SidereonEphemerisSampleRow or NULL
/// when len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_broadcast_ephemeris_sample(
    broadcast: *const SidereonBroadcastEphemeris,
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
        "sidereon_broadcast_ephemeris_sample",
        SidereonStatus::Panic,
        || {
            let broadcast = c_try!(require_ref(
                broadcast,
                "sidereon_broadcast_ephemeris_sample",
                "broadcast"
            ));
            ephemeris_sample_common(
                "sidereon_broadcast_ephemeris_sample",
                &broadcast.inner,
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

/// Evaluate many broadcast-ephemeris observable states with per-satellite
/// epochs. The output arrays follow
/// sidereon_sp3_observable_states_at_j2000_s.
///
/// Safety: broadcast is a live handle; all array pointers follow the SP3 batch
/// state contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_broadcast_observable_states_at_j2000_s(
    broadcast: *const SidereonBroadcastEphemeris,
    satellites: *const *const c_char,
    epochs_j2000_s: *const f64,
    count: usize,
    out_positions_ecef_m: *mut f64,
    out_clocks_s: *mut f64,
    out_has_clocks_s: *mut bool,
    out_element_statuses: *mut SidereonObservableStateElementStatus,
    out_result_statuses: *mut SidereonStatus,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_broadcast_observable_states_at_j2000_s",
        SidereonStatus::Panic,
        || {
            let broadcast = c_try!(require_ref(
                broadcast,
                "sidereon_broadcast_observable_states_at_j2000_s",
                "broadcast"
            ));
            observable_states_at_j2000_s_common(
                "sidereon_broadcast_observable_states_at_j2000_s",
                &broadcast.inner,
                satellites,
                epochs_j2000_s,
                count,
                out_positions_ecef_m,
                out_clocks_s,
                out_has_clocks_s,
                out_element_statuses,
                out_result_statuses,
            )
        },
    )
}

/// Evaluate many broadcast-ephemeris observable states at one shared epoch.
///
/// Safety: same output-array contract as
/// sidereon_sp3_observable_states_at_j2000_s.
#[no_mangle]
pub unsafe extern "C" fn sidereon_broadcast_observable_states_at_shared_j2000_s(
    broadcast: *const SidereonBroadcastEphemeris,
    satellites: *const *const c_char,
    satellite_count: usize,
    epoch_j2000_s: f64,
    out_positions_ecef_m: *mut f64,
    out_clocks_s: *mut f64,
    out_has_clocks_s: *mut bool,
    out_element_statuses: *mut SidereonObservableStateElementStatus,
    out_result_statuses: *mut SidereonStatus,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_broadcast_observable_states_at_shared_j2000_s",
        SidereonStatus::Panic,
        || {
            let broadcast = c_try!(require_ref(
                broadcast,
                "sidereon_broadcast_observable_states_at_shared_j2000_s",
                "broadcast"
            ));
            observable_states_at_shared_j2000_s_common(
                "sidereon_broadcast_observable_states_at_shared_j2000_s",
                &broadcast.inner,
                satellites,
                satellite_count,
                epoch_j2000_s,
                out_positions_ecef_m,
                out_clocks_s,
                out_has_clocks_s,
                out_element_statuses,
                out_result_statuses,
            )
        },
    )
}

// === Round-2 CNAV and RINEX-4 broadcast accessors ===========================

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonNavMessage {
    GpsLnav = 0,
    GpsCnav = 1,
    GpsCnav2 = 2,
    QzssCnav = 3,
    QzssCnav2 = 4,
    GalileoInav = 5,
    GalileoFnav = 6,
    BeidouD1 = 7,
    BeidouD2 = 8,
    QzssLnav = 9,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonNavMessagePreference {
    PreferLegacy = 0,
    PreferModern = 1,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonBroadcastGroupDelayTerm {
    GpsTgd = 0,
    GalileoBgdE5aE1 = 1,
    GalileoBgdE5bE1 = 2,
    BeidouTgd1 = 3,
    BeidouTgd2 = 4,
    CnavIscL1Ca = 5,
    CnavIscL2C = 6,
    CnavIscL5I5 = 7,
    CnavIscL5Q5 = 8,
    CnavIscL1Cd = 9,
    CnavIscL1Cp = 10,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonCnavSignal {
    L1Ca = 0,
    L2C = 1,
    L5I5 = 2,
    L5Q5 = 3,
    L1Cp = 4,
    L1Cd = 5,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonCnavParameters {
    pub present: bool,
    pub adot_m_s: f64,
    pub delta_n0_dot_rad_s2: f64,
    pub top_week: u32,
    pub top_tow_s: f64,
    pub ura_ed_index: i8,
    pub ura_ned0_index: i8,
    pub ura_ned1_index: u8,
    pub ura_ned2_index: u8,
    pub transmission_time_sow: f64,
    pub has_flags: bool,
    pub flags: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonBroadcastRecordInfo {
    pub sat_id: SidereonSatelliteToken,
    pub message: u32,
    pub issue: u32,
    pub issue_message: u32,
    pub week: u32,
    pub toe_week: u32,
    pub toe_tow_s: f64,
    pub toc_week: u32,
    pub toc_tow_s: f64,
    pub sv_health: f64,
    pub sv_accuracy_m: f64,
    pub has_fit_interval_s: bool,
    pub fit_interval_s: f64,
    pub default_group_delay_s: f64,
    pub cnav: SidereonCnavParameters,
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_broadcast_ephemeris_record_count(
    broadcast: *const SidereonBroadcastEphemeris,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_broadcast_ephemeris_record_count",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_count,
                "sidereon_broadcast_ephemeris_record_count",
                "out_count"
            ));
            *out = 0;
            let broadcast = c_try!(require_ref(
                broadcast,
                "sidereon_broadcast_ephemeris_record_count",
                "broadcast"
            ));
            *out = broadcast.inner.records().len();
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_broadcast_ephemeris_records(
    broadcast: *const SidereonBroadcastEphemeris,
    out: *mut SidereonBroadcastRecordInfo,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_broadcast_ephemeris_records",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_broadcast_ephemeris_records",
                out_written,
                out_required
            ));
            let broadcast = c_try!(require_ref(
                broadcast,
                "sidereon_broadcast_ephemeris_records",
                "broadcast"
            ));
            let values: Vec<_> = broadcast
                .inner
                .records()
                .iter()
                .map(broadcast_record_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_broadcast_ephemeris_records",
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
pub unsafe extern "C" fn sidereon_broadcast_ephemeris_set_nav_message_preference(
    broadcast: *mut SidereonBroadcastEphemeris,
    preference: u32,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_broadcast_ephemeris_set_nav_message_preference",
        SidereonStatus::Panic,
        || {
            let broadcast = c_try!(require_out(
                broadcast,
                "sidereon_broadcast_ephemeris_set_nav_message_preference",
                "broadcast"
            ));
            let preference = match preference {
                x if x == SidereonNavMessagePreference::PreferLegacy as u32 => {
                    sidereon_core::rinex::nav::NavMessagePreference::PreferLegacy
                }
                x if x == SidereonNavMessagePreference::PreferModern as u32 => {
                    sidereon_core::rinex::nav::NavMessagePreference::PreferModern
                }
                _ => {
                    set_last_error(format!(
                        "sidereon_broadcast_ephemeris_set_nav_message_preference: invalid preference {preference}"
                    ));
                    return SidereonStatus::InvalidArgument;
                }
            };
            broadcast.inner.set_message_preference(preference);
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_broadcast_ephemeris_nav_message_preference(
    broadcast: *const SidereonBroadcastEphemeris,
    out_preference: *mut u32,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_broadcast_ephemeris_nav_message_preference",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_preference,
                "sidereon_broadcast_ephemeris_nav_message_preference",
                "out_preference"
            ));
            *out = 0;
            let broadcast = c_try!(require_ref(
                broadcast,
                "sidereon_broadcast_ephemeris_nav_message_preference",
                "broadcast"
            ));
            *out = match broadcast.inner.message_preference() {
                sidereon_core::rinex::nav::NavMessagePreference::PreferLegacy => {
                    SidereonNavMessagePreference::PreferLegacy as u32
                }
                sidereon_core::rinex::nav::NavMessagePreference::PreferModern => {
                    SidereonNavMessagePreference::PreferModern as u32
                }
            };
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_broadcast_ephemeris_record_group_delay(
    broadcast: *const SidereonBroadcastEphemeris,
    index: usize,
    term: u32,
    out_value_s: *mut f64,
    out_present: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_broadcast_ephemeris_record_group_delay",
        SidereonStatus::Panic,
        || {
            let out_value = c_try!(require_out(
                out_value_s,
                "sidereon_broadcast_ephemeris_record_group_delay",
                "out_value_s"
            ));
            *out_value = 0.0;
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_broadcast_ephemeris_record_group_delay",
                "out_present"
            ));
            *out_present = false;
            let broadcast = c_try!(require_ref(
                broadcast,
                "sidereon_broadcast_ephemeris_record_group_delay",
                "broadcast"
            ));
            let term = c_try!(group_delay_term_from_c(
                "sidereon_broadcast_ephemeris_record_group_delay",
                term
            ));
            let Some(record) = broadcast.inner.records().get(index) else {
                set_last_error(format!(
                    "sidereon_broadcast_ephemeris_record_group_delay: index {index} out of range"
                ));
                return SidereonStatus::InvalidArgument;
            };
            if let Some(value) = record.group_delays.get(term) {
                *out_value = value;
                *out_present = true;
            }
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_broadcast_ephemeris_record_cnav_correction(
    broadcast: *const SidereonBroadcastEphemeris,
    index: usize,
    signal: u32,
    out_value_s: *mut f64,
    out_present: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_broadcast_ephemeris_record_cnav_correction",
        SidereonStatus::Panic,
        || {
            let out_value = c_try!(require_out(
                out_value_s,
                "sidereon_broadcast_ephemeris_record_cnav_correction",
                "out_value_s"
            ));
            *out_value = 0.0;
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_broadcast_ephemeris_record_cnav_correction",
                "out_present"
            ));
            *out_present = false;
            let broadcast = c_try!(require_ref(
                broadcast,
                "sidereon_broadcast_ephemeris_record_cnav_correction",
                "broadcast"
            ));
            let signal = c_try!(cnav_signal_from_c(
                "sidereon_broadcast_ephemeris_record_cnav_correction",
                signal
            ));
            let Some(record) = broadcast.inner.records().get(index) else {
                set_last_error(format!(
                    "sidereon_broadcast_ephemeris_record_cnav_correction: index {index} out of range"
                ));
                return SidereonStatus::InvalidArgument;
            };
            if let Some(value) = record
                .group_delays
                .cnav_single_frequency_correction_s(signal)
            {
                *out_value = value;
                *out_present = true;
            }
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_broadcast_ephemeris_select_by_issue(
    broadcast: *const SidereonBroadcastEphemeris,
    sat_id: *const c_char,
    issue: u32,
    message: u32,
    epoch_j2000_s: f64,
    out_record: *mut SidereonBroadcastRecordInfo,
    out_present: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_broadcast_ephemeris_select_by_issue",
        SidereonStatus::Panic,
        || {
            let out_record = c_try!(require_out(
                out_record,
                "sidereon_broadcast_ephemeris_select_by_issue",
                "out_record"
            ));
            *out_record = empty_broadcast_record_info();
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_broadcast_ephemeris_select_by_issue",
                "out_present"
            ));
            *out_present = false;
            let broadcast = c_try!(require_ref(
                broadcast,
                "sidereon_broadcast_ephemeris_select_by_issue",
                "broadcast"
            ));
            let sat = c_try!(parse_satellite_token(
                "sidereon_broadcast_ephemeris_select_by_issue",
                sat_id
            ));
            let message = c_try!(nav_message_from_c(
                "sidereon_broadcast_ephemeris_select_by_issue",
                message
            ));
            let issue = sidereon_core::ephemeris::BroadcastIssue { issue, message };
            if let Some(record) =
                broadcast
                    .inner
                    .select_by_issue_at(sat, issue, message, epoch_j2000_s)
            {
                *out_record = broadcast_record_to_c(record);
                *out_present = true;
            }
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_cnav_ura_nominal_m(
    index: i8,
    out_value_m: *mut f64,
    out_present: *mut bool,
) -> SidereonStatus {
    ffi_boundary("sidereon_cnav_ura_nominal_m", SidereonStatus::Panic, || {
        let out_value = c_try!(require_out(
            out_value_m,
            "sidereon_cnav_ura_nominal_m",
            "out_value_m"
        ));
        *out_value = 0.0;
        let out_present = c_try!(require_out(
            out_present,
            "sidereon_cnav_ura_nominal_m",
            "out_present"
        ));
        *out_present = false;
        if let Some(value) = sidereon_core::rinex::nav::cnav_ura_nominal_m(index) {
            *out_value = value;
            *out_present = true;
        }
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_cnav_ura_ned_m(
    params: *const SidereonCnavParameters,
    query_week: u32,
    query_tow_s: f64,
    out_value_m: *mut f64,
    out_present: *mut bool,
) -> SidereonStatus {
    ffi_boundary("sidereon_cnav_ura_ned_m", SidereonStatus::Panic, || {
        let out_value = c_try!(require_out(
            out_value_m,
            "sidereon_cnav_ura_ned_m",
            "out_value_m"
        ));
        *out_value = 0.0;
        let out_present = c_try!(require_out(
            out_present,
            "sidereon_cnav_ura_ned_m",
            "out_present"
        ));
        *out_present = false;
        let params = c_try!(require_ref(params, "sidereon_cnav_ura_ned_m", "params"));
        if !params.present {
            set_last_error("sidereon_cnav_ura_ned_m: params.present is false".to_string());
            return SidereonStatus::InvalidArgument;
        }
        let params = sidereon_core::rinex::nav::CnavParameters {
            adot_m_s: params.adot_m_s,
            delta_n0_dot_rad_s2: params.delta_n0_dot_rad_s2,
            top: GnssWeekTow {
                system: TimeScale::Gpst,
                week: params.top_week,
                tow_s: params.top_tow_s,
            },
            ura_ed_index: params.ura_ed_index,
            ura_ned0_index: params.ura_ned0_index,
            ura_ned1_index: params.ura_ned1_index,
            ura_ned2_index: params.ura_ned2_index,
            transmission_time_sow: params.transmission_time_sow,
            flags: params.has_flags.then_some(params.flags),
        };
        if let Some(value) = sidereon_core::rinex::nav::cnav_ura_ned_m(
            &params,
            GnssWeekTow {
                system: TimeScale::Gpst,
                week: query_week,
                tow_s: query_tow_s,
            },
        ) {
            *out_value = value;
            *out_present = true;
        }
        SidereonStatus::Ok
    })
}

fn compare_stats_to_c(
    s: &sidereon_core::broadcast_comparison::CompareStats,
) -> SidereonCompareStats {
    SidereonCompareStats {
        count: s.count,
        orbit_3d_rms_m: none_to_nan(s.orbit_3d_rms_m),
        orbit_3d_max_m: none_to_nan(s.orbit_3d_max_m),
        radial_rms_m: none_to_nan(s.radial_rms_m),
        radial_max_m: none_to_nan(s.radial_max_m),
        along_rms_m: none_to_nan(s.along_rms_m),
        along_max_m: none_to_nan(s.along_max_m),
        cross_rms_m: none_to_nan(s.cross_rms_m),
        cross_max_m: none_to_nan(s.cross_max_m),
        clock_rms_m: none_to_nan(s.clock_rms_m),
        clock_max_m: none_to_nan(s.clock_max_m),
        clock_datum_removed_rms_m: none_to_nan(s.clock_datum_removed_rms_m),
        clock_datum_removed_max_m: none_to_nan(s.clock_datum_removed_max_m),
    }
}

fn lnav_params_from_c(params: &SidereonLnavParams) -> sidereon_core::navigation::lnav::LnavParams {
    use sidereon_core::navigation::lnav::LnavNumber::{Float, Int};
    sidereon_core::navigation::lnav::LnavParams {
        week_number: Int(params.week_number),
        l2_code: Int(params.l2_code),
        l2_p_data_flag: Int(params.l2_p_data_flag),
        ura_index: Int(params.ura_index),
        sv_health: Int(params.sv_health),
        iodc: Int(params.iodc),
        tgd: Float(params.tgd),
        toc: Int(params.toc),
        af0: Float(params.af0),
        af1: Float(params.af1),
        af2: Float(params.af2),
        iode: Int(params.iode),
        crs: Float(params.crs),
        delta_n: Float(params.delta_n),
        m0: Float(params.m0),
        cuc: Float(params.cuc),
        eccentricity: Float(params.eccentricity),
        cus: Float(params.cus),
        sqrt_a: Float(params.sqrt_a),
        toe: Int(params.toe),
        fit_interval_flag: Int(params.fit_interval_flag),
        aodo: Int(params.aodo),
        cic: Float(params.cic),
        omega0: Float(params.omega0),
        cis: Float(params.cis),
        i0: Float(params.i0),
        crc: Float(params.crc),
        omega: Float(params.omega),
        omega_dot: Float(params.omega_dot),
        idot: Float(params.idot),
    }
}

fn lnav_options_from_c(opts: &SidereonLnavOptions) -> sidereon_core::navigation::lnav::LnavOptions {
    use sidereon_core::navigation::lnav::LnavNumber::Int;
    sidereon_core::navigation::lnav::LnavOptions {
        tow: Int(opts.tow),
        alert: Int(opts.alert),
        anti_spoof: Int(opts.anti_spoof),
        integrity: Int(opts.integrity),
        tlm_message: Int(opts.tlm_message),
    }
}

fn lnav_decoded_to_c(d: &sidereon_core::navigation::lnav::LnavDecoded) -> SidereonLnavDecoded {
    SidereonLnavDecoded {
        week_number: d.week_number,
        l2_code: d.l2_code,
        ura_index: d.ura_index,
        sv_health: d.sv_health,
        iodc: d.iodc,
        tgd: d.tgd,
        toc: d.toc,
        af0: d.af0,
        af1: d.af1,
        af2: d.af2,
        iode: d.iode,
        crs: d.crs,
        delta_n: d.delta_n,
        m0: d.m0,
        cuc: d.cuc,
        eccentricity: d.eccentricity,
        cus: d.cus,
        sqrt_a: d.sqrt_a,
        toe: d.toe,
        fit_interval_flag: d.fit_interval_flag,
        aodo: d.aodo,
        cic: d.cic,
        omega0: d.omega0,
        cis: d.cis,
        i0: d.i0,
        crc: d.crc,
        omega: d.omega,
        omega_dot: d.omega_dot,
        idot: d.idot,
    }
}

fn empty_broadcast_record_info() -> SidereonBroadcastRecordInfo {
    SidereonBroadcastRecordInfo {
        sat_id: SidereonSatelliteToken {
            bytes: [0; SATELLITE_TOKEN_C_BYTES],
        },
        message: 0,
        issue: 0,
        issue_message: 0,
        week: 0,
        toe_week: 0,
        toe_tow_s: 0.0,
        toc_week: 0,
        toc_tow_s: 0.0,
        sv_health: 0.0,
        sv_accuracy_m: 0.0,
        has_fit_interval_s: false,
        fit_interval_s: 0.0,
        default_group_delay_s: 0.0,
        cnav: empty_cnav_parameters(),
    }
}

fn nav_message_from_c(
    fn_name: &str,
    message: u32,
) -> Result<sidereon_core::rinex::nav::NavMessage, SidereonStatus> {
    use sidereon_core::rinex::nav::NavMessage as M;
    match message {
        x if x == SidereonNavMessage::GpsLnav as u32 => Ok(M::GpsLnav),
        x if x == SidereonNavMessage::GpsCnav as u32 => Ok(M::GpsCnav),
        x if x == SidereonNavMessage::GpsCnav2 as u32 => Ok(M::GpsCnav2),
        x if x == SidereonNavMessage::QzssCnav as u32 => Ok(M::QzssCnav),
        x if x == SidereonNavMessage::QzssCnav2 as u32 => Ok(M::QzssCnav2),
        x if x == SidereonNavMessage::QzssLnav as u32 => Ok(M::QzssLnav),
        x if x == SidereonNavMessage::GalileoInav as u32 => Ok(M::GalileoInav),
        x if x == SidereonNavMessage::GalileoFnav as u32 => Ok(M::GalileoFnav),
        x if x == SidereonNavMessage::BeidouD1 as u32 => Ok(M::BeidouD1),
        x if x == SidereonNavMessage::BeidouD2 as u32 => Ok(M::BeidouD2),
        _ => {
            set_last_error(format!("{fn_name}: invalid nav message {message}"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn group_delay_term_from_c(
    fn_name: &str,
    term: u32,
) -> Result<sidereon_core::rinex::nav::BroadcastGroupDelayTerm, SidereonStatus> {
    use sidereon_core::rinex::nav::BroadcastGroupDelayTerm as T;
    match term {
        x if x == SidereonBroadcastGroupDelayTerm::GpsTgd as u32 => Ok(T::GpsTgd),
        x if x == SidereonBroadcastGroupDelayTerm::GalileoBgdE5aE1 as u32 => Ok(T::GalileoBgdE5aE1),
        x if x == SidereonBroadcastGroupDelayTerm::GalileoBgdE5bE1 as u32 => Ok(T::GalileoBgdE5bE1),
        x if x == SidereonBroadcastGroupDelayTerm::BeidouTgd1 as u32 => Ok(T::BeidouTgd1),
        x if x == SidereonBroadcastGroupDelayTerm::BeidouTgd2 as u32 => Ok(T::BeidouTgd2),
        x if x == SidereonBroadcastGroupDelayTerm::CnavIscL1Ca as u32 => Ok(T::CnavIscL1Ca),
        x if x == SidereonBroadcastGroupDelayTerm::CnavIscL2C as u32 => Ok(T::CnavIscL2C),
        x if x == SidereonBroadcastGroupDelayTerm::CnavIscL5I5 as u32 => Ok(T::CnavIscL5I5),
        x if x == SidereonBroadcastGroupDelayTerm::CnavIscL5Q5 as u32 => Ok(T::CnavIscL5Q5),
        x if x == SidereonBroadcastGroupDelayTerm::CnavIscL1Cd as u32 => Ok(T::CnavIscL1Cd),
        x if x == SidereonBroadcastGroupDelayTerm::CnavIscL1Cp as u32 => Ok(T::CnavIscL1Cp),
        _ => {
            set_last_error(format!("{fn_name}: invalid group-delay term {term}"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn cnav_signal_from_c(
    fn_name: &str,
    signal: u32,
) -> Result<sidereon_core::rinex::nav::CnavSignal, SidereonStatus> {
    use sidereon_core::rinex::nav::CnavSignal as S;
    match signal {
        x if x == SidereonCnavSignal::L1Ca as u32 => Ok(S::L1Ca),
        x if x == SidereonCnavSignal::L2C as u32 => Ok(S::L2C),
        x if x == SidereonCnavSignal::L5I5 as u32 => Ok(S::L5I5),
        x if x == SidereonCnavSignal::L5Q5 as u32 => Ok(S::L5Q5),
        x if x == SidereonCnavSignal::L1Cp as u32 => Ok(S::L1Cp),
        x if x == SidereonCnavSignal::L1Cd as u32 => Ok(S::L1Cd),
        _ => {
            set_last_error(format!("{fn_name}: invalid CNAV signal {signal}"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn broadcast_record_to_c(
    record: &sidereon_core::rinex::nav::BroadcastRecord,
) -> SidereonBroadcastRecordInfo {
    SidereonBroadcastRecordInfo {
        sat_id: satellite_token(record.satellite_id),
        message: nav_message_to_c(record.message),
        issue: record.issue_of_data.issue,
        issue_message: nav_message_to_c(record.issue_of_data.message),
        week: record.week,
        toe_week: record.toe.week,
        toe_tow_s: record.toe.tow_s,
        toc_week: record.toc.week,
        toc_tow_s: record.toc.tow_s,
        sv_health: record.sv_health,
        sv_accuracy_m: record.sv_accuracy_m,
        has_fit_interval_s: record.fit_interval_s.is_some(),
        fit_interval_s: record.fit_interval_s.unwrap_or(0.0),
        default_group_delay_s: record.broadcast_clock_group_delay_s(),
        cnav: cnav_parameters_to_c(record.cnav),
    }
}

fn nav_message_to_c(message: sidereon_core::rinex::nav::NavMessage) -> u32 {
    use sidereon_core::rinex::nav::NavMessage as M;
    match message {
        M::GpsLnav => SidereonNavMessage::GpsLnav as u32,
        M::GpsCnav => SidereonNavMessage::GpsCnav as u32,
        M::GpsCnav2 => SidereonNavMessage::GpsCnav2 as u32,
        M::QzssCnav => SidereonNavMessage::QzssCnav as u32,
        M::QzssCnav2 => SidereonNavMessage::QzssCnav2 as u32,
        M::QzssLnav => SidereonNavMessage::QzssLnav as u32,
        M::GalileoInav => SidereonNavMessage::GalileoInav as u32,
        M::GalileoFnav => SidereonNavMessage::GalileoFnav as u32,
        M::BeidouD1 => SidereonNavMessage::BeidouD1 as u32,
        M::BeidouD2 => SidereonNavMessage::BeidouD2 as u32,
    }
}

fn cnav_parameters_to_c(
    cnav: Option<sidereon_core::rinex::nav::CnavParameters>,
) -> SidereonCnavParameters {
    if let Some(cnav) = cnav {
        SidereonCnavParameters {
            present: true,
            adot_m_s: cnav.adot_m_s,
            delta_n0_dot_rad_s2: cnav.delta_n0_dot_rad_s2,
            top_week: cnav.top.week,
            top_tow_s: cnav.top.tow_s,
            ura_ed_index: cnav.ura_ed_index,
            ura_ned0_index: cnav.ura_ned0_index,
            ura_ned1_index: cnav.ura_ned1_index,
            ura_ned2_index: cnav.ura_ned2_index,
            transmission_time_sow: cnav.transmission_time_sow,
            has_flags: cnav.flags.is_some(),
            flags: cnav.flags.unwrap_or(0),
        }
    } else {
        empty_cnav_parameters()
    }
}
