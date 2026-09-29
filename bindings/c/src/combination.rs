use super::*;

fn ionosphere_free_invalid_arg(
    operation: &str,
    error: sidereon_core::combinations::IonosphereFreeError,
) -> SidereonStatus {
    crate::engine_error::record_engine_error(
        crate::engine_error::SidereonEngineErrorFamily::IonosphereFree,
        operation,
        crate::engine_error::ionosphere_free_error_value(&error),
    );
    extra_invalid_arg(operation, error)
}

// --- Dual-frequency combinations (sidereon_core::combinations) ----------------

/// Ionospheric scaling factor gamma = (f1/f2)^2. Delegates to
/// sidereon_core::combinations::gamma.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_combination_gamma(
    f1_hz: f64,
    f2_hz: f64,
    out: *mut f64,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_combination_gamma",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_combination_gamma", "out"));
            *out = 0.0;
            match sidereon_core::combinations::gamma(f1_hz, f2_hz) {
                Ok(v) => {
                    *out = v;
                    SidereonStatus::Ok
                }
                Err(err) => ionosphere_free_invalid_arg("sidereon_combination_gamma", err),
            }
        },
    )
}

/// Ionosphere-free noise amplification factor. Delegates to
/// sidereon_core::combinations::noise_amplification.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_combination_noise_amplification(
    f1_hz: f64,
    f2_hz: f64,
    out: *mut f64,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_combination_noise_amplification",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_combination_noise_amplification",
                "out"
            ));
            *out = 0.0;
            match sidereon_core::combinations::noise_amplification(f1_hz, f2_hz) {
                Ok(v) => {
                    *out = v;
                    SidereonStatus::Ok
                }
                Err(err) => {
                    ionosphere_free_invalid_arg("sidereon_combination_noise_amplification", err)
                }
            }
        },
    )
}

/// Ionosphere-free pseudorange combination of two code observables in meters.
/// Delegates to sidereon_core::combinations::ionosphere_free.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_combination_ionosphere_free(
    obs1_m: f64,
    obs2_m: f64,
    f1_hz: f64,
    f2_hz: f64,
    out: *mut f64,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_combination_ionosphere_free",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_combination_ionosphere_free",
                "out"
            ));
            *out = 0.0;
            match sidereon_core::combinations::ionosphere_free(obs1_m, obs2_m, f1_hz, f2_hz) {
                Ok(v) => {
                    *out = v;
                    SidereonStatus::Ok
                }
                Err(err) => {
                    ionosphere_free_invalid_arg("sidereon_combination_ionosphere_free", err)
                }
            }
        },
    )
}

/// Ionosphere-free carrier-phase combination in meters. Delegates to
/// sidereon_core::combinations::ionosphere_free_phase_m.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_combination_ionosphere_free_phase_m(
    phase1_m: f64,
    phase2_m: f64,
    f1_hz: f64,
    f2_hz: f64,
    out: *mut f64,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_combination_ionosphere_free_phase_m",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_combination_ionosphere_free_phase_m",
                "out"
            ));
            *out = 0.0;
            match sidereon_core::combinations::ionosphere_free_phase_m(
                phase1_m, phase2_m, f1_hz, f2_hz,
            ) {
                Ok(v) => {
                    *out = v;
                    SidereonStatus::Ok
                }
                Err(err) => {
                    ionosphere_free_invalid_arg("sidereon_combination_ionosphere_free_phase_m", err)
                }
            }
        },
    )
}

// --- Measurement weighting / RAIM scalars (sidereon_core::quality) -----------

/// Pseudorange variance model.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonPseudorangeVarianceModel {
    /// Elevation-only weighting.
    Elevation = 0,
    /// Elevation plus C/N0 weighting.
    ElevationCn0 = 1,
}

/// Options for the pseudorange variance model, mirroring
/// sidereon_core::quality::PseudorangeVarianceOptions.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPseudorangeVarianceOptions {
    /// Zenith standard-deviation term, meters.
    pub a_m: f64,
    /// Elevation-scaled standard-deviation term, meters.
    pub b_m: f64,
    /// One of SidereonPseudorangeVarianceModel as uint32_t.
    pub model: u32,
    /// Whether cn0_dbhz is supplied (required for the ElevationCn0 model).
    pub has_cn0: bool,
    /// Carrier-to-noise density, dB-Hz, used when has_cn0 is true.
    pub cn0_dbhz: f64,
    /// C/N0 scaling term, meters squared.
    pub cn0_scale_m2: f64,
}

/// Fill *out_options with the engine default pseudorange variance options.
///
/// Safety: out_options must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_pseudorange_variance_options_init(
    out_options: *mut SidereonPseudorangeVarianceOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_pseudorange_variance_options_init",
        SidereonStatus::Panic,
        || {
            let out_options = c_try!(require_out(
                out_options,
                "sidereon_pseudorange_variance_options_init",
                "out_options"
            ));
            let d = sidereon_core::quality::PseudorangeVarianceOptions::default();
            *out_options = SidereonPseudorangeVarianceOptions {
                a_m: d.a_m,
                b_m: d.b_m,
                model: match d.model {
                    sidereon_core::quality::PseudorangeVarianceModel::Elevation => {
                        SidereonPseudorangeVarianceModel::Elevation as u32
                    }
                    sidereon_core::quality::PseudorangeVarianceModel::ElevationCn0 => {
                        SidereonPseudorangeVarianceModel::ElevationCn0 as u32
                    }
                },
                has_cn0: d.cn0_dbhz.is_some(),
                cn0_dbhz: d.cn0_dbhz.unwrap_or(0.0),
                cn0_scale_m2: d.cn0_scale_m2,
            };
            SidereonStatus::Ok
        },
    )
}

/// Pseudorange variance (meters squared) at an elevation under the weighting
/// model. Delegates to sidereon_core::quality::pseudorange_variance.
///
/// Safety: options must point to a SidereonPseudorangeVarianceOptions; out must
/// point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_pseudorange_variance(
    elevation_deg: f64,
    options: *const SidereonPseudorangeVarianceOptions,
    out: *mut f64,
) -> SidereonStatus {
    quality_operation_boundary(
        "sidereon_pseudorange_variance",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_pseudorange_variance", "out"));
            *out = 0.0;
            let options = c_try!(require_ref(
                options,
                "sidereon_pseudorange_variance",
                "options"
            ));
            let opts = c_try!(pseudorange_variance_options_from_c(
                "sidereon_pseudorange_variance",
                options
            ));
            match sidereon_core::quality::pseudorange_variance(elevation_deg, opts) {
                Ok(v) => {
                    *out = v;
                    SidereonStatus::Ok
                }
                Err(err) => map_quality_error("sidereon_pseudorange_variance", err),
            }
        },
    )
}

// --- Carrier-phase Hatch smoothing (sidereon_core::carrier_phase) ------------

/// One epoch of single-satellite dual-frequency observables for arc smoothing.
/// Optional fields use NaN (for the f64 fields) or the has_* flags (for LLI) to
/// signal absence, mirroring sidereon_core::carrier_phase::ArcEpoch.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonArcEpoch {
    /// Band-1 carrier phase in cycles, or NaN when absent.
    pub phi1_cycles: f64,
    /// Band-2 carrier phase in cycles, or NaN when absent.
    pub phi2_cycles: f64,
    /// Band-1 pseudorange in meters, or NaN when absent.
    pub p1_m: f64,
    /// Band-2 pseudorange in meters, or NaN when absent.
    pub p2_m: f64,
    /// Whether lli1 carries a value.
    pub has_lli1: bool,
    /// Band-1 loss-of-lock indicator when has_lli1 is true.
    pub lli1: i64,
    /// Whether lli2 carries a value.
    pub has_lli2: bool,
    /// Band-2 loss-of-lock indicator when has_lli2 is true.
    pub lli2: i64,
    /// Band-1 carrier frequency in Hz, or NaN when absent.
    pub f1_hz: f64,
    /// Band-2 carrier frequency in Hz, or NaN when absent.
    pub f2_hz: f64,
    /// Elapsed seconds since the previous epoch, or NaN when absent.
    pub gap_time_s: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonArcEpochV2 {
    pub legacy: SidereonArcEpoch,
    pub has_gap_epoch: bool,
    pub gap_epoch: *const SidereonExactEpoch,
}

/// Cycle-slip thresholds, mirroring
/// sidereon_core::carrier_phase::CycleSlipOptions.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonCycleSlipOptions {
    /// Geometry-free jump threshold in meters.
    pub gf_threshold_m: f64,
    /// Melbourne-Wubbena jump threshold in cycles.
    pub mw_threshold_cycles: f64,
    /// Minimum arc gap in seconds before forcing a reset.
    pub min_arc_gap_s: f64,
}

/// One smoothed-code result, mirroring
/// sidereon_core::carrier_phase::SmoothCodeResult. p_smooth_m is NaN when the
/// epoch produced no smoothed value.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSmoothCodeResult {
    /// Hatch-smoothed pseudorange in meters, or NaN when unavailable.
    pub p_smooth_m: f64,
    /// Current smoothing window length.
    pub window: usize,
    /// Whether the smoother reset at this epoch.
    pub reset: bool,
}

/// One ionosphere-free smoothed-code result, mirroring
/// sidereon_core::carrier_phase::IonoFreeSmoothResult. Unavailable values are
/// NaN.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonIonoFreeSmoothResult {
    /// Smoothed ionosphere-free pseudorange in meters, or NaN.
    pub p_smooth_m: f64,
    /// Raw ionosphere-free pseudorange in meters, or NaN.
    pub p_if_m: f64,
    /// Ionosphere-free carrier phase in meters, or NaN.
    pub l_if_m: f64,
    /// Current smoothing window length.
    pub window: usize,
    /// Whether the smoother reset at this epoch.
    pub reset: bool,
}

/// Fill *out_options with the engine default cycle-slip thresholds. Override
/// before smoothing.
///
/// Safety: out_options must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_cycle_slip_options_init(
    out_options: *mut SidereonCycleSlipOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_cycle_slip_options_init",
        SidereonStatus::Panic,
        || {
            let out_options = c_try!(require_out(
                out_options,
                "sidereon_cycle_slip_options_init",
                "out_options"
            ));
            let d = sidereon_core::carrier_phase::CycleSlipOptions::default();
            *out_options = SidereonCycleSlipOptions {
                gf_threshold_m: d.gf_threshold_m,
                mw_threshold_cycles: d.mw_threshold_cycles,
                min_arc_gap_s: d.min_arc_gap_s,
            };
            SidereonStatus::Ok
        },
    )
}

/// Hatch-smooth single-frequency code over an arc. One result is produced per
/// input epoch (parallel arrays). Variable-length output contract. Delegates to
/// sidereon_core::carrier_phase::smooth_code.
///
/// Safety: arc points to count SidereonArcEpoch; options points to a
/// SidereonCycleSlipOptions; out points to len SidereonSmoothCodeResult or NULL
/// when len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_smooth_code(
    arc: *const SidereonArcEpoch,
    count: usize,
    options: *const SidereonCycleSlipOptions,
    hatch_window_cap: usize,
    out: *mut SidereonSmoothCodeResult,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_smooth_code",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_smooth_code",
                out_written,
                out_required
            ));
            let (arc, opts) = c_try!(arc_from_c("sidereon_smooth_code", arc, count, options));
            let results =
                match sidereon_core::carrier_phase::smooth_code(&arc, opts, hatch_window_cap) {
                    Ok(r) => r,
                    Err(err) => {
                        return crate::signal::record_carrier_phase_error(
                            "sidereon_smooth_code",
                            err,
                        )
                    }
                };
            let mapped: Vec<SidereonSmoothCodeResult> = results
                .iter()
                .map(|r| SidereonSmoothCodeResult {
                    p_smooth_m: none_to_nan(r.p_smooth_m),
                    window: r.window,
                    reset: r.reset,
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_smooth_code",
                "out",
                &mapped,
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
pub unsafe extern "C" fn sidereon_smooth_code_v2(
    arc: *const SidereonArcEpochV2,
    count: usize,
    options: *const SidereonCycleSlipOptions,
    hatch_window_cap: usize,
    out: *mut SidereonSmoothCodeResult,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_smooth_code_v2";
    crate::engine_error::engine_error_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let (arc, opts) = c_try!(arc_v2_from_c(FN_NAME, arc, count, options));
        let results = match sidereon_core::carrier_phase::smooth_code(&arc, opts, hatch_window_cap)
        {
            Ok(results) => results,
            Err(error) => return crate::signal::record_carrier_phase_error(FN_NAME, error),
        };
        let mapped: Vec<_> = results
            .iter()
            .map(|result| SidereonSmoothCodeResult {
                p_smooth_m: none_to_nan(result.p_smooth_m),
                window: result.window,
                reset: result.reset,
            })
            .collect();
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            &mapped,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Hatch-smooth ionosphere-free code over a dual-frequency arc. One result per
/// input epoch. Variable-length output contract. Delegates to
/// sidereon_core::carrier_phase::smooth_iono_free_code.
///
/// Safety: arc points to count SidereonArcEpoch; options points to a
/// SidereonCycleSlipOptions; out points to len SidereonIonoFreeSmoothResult or
/// NULL when len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_smooth_iono_free_code(
    arc: *const SidereonArcEpoch,
    count: usize,
    options: *const SidereonCycleSlipOptions,
    hatch_window_cap: usize,
    out: *mut SidereonIonoFreeSmoothResult,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_smooth_iono_free_code",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_smooth_iono_free_code",
                out_written,
                out_required
            ));
            let (arc, opts) = c_try!(arc_from_c(
                "sidereon_smooth_iono_free_code",
                arc,
                count,
                options
            ));
            let results = match sidereon_core::carrier_phase::smooth_iono_free_code(
                &arc,
                opts,
                hatch_window_cap,
            ) {
                Ok(r) => r,
                Err(err) => {
                    return crate::signal::record_carrier_phase_error(
                        "sidereon_smooth_iono_free_code",
                        err,
                    )
                }
            };
            let mapped: Vec<SidereonIonoFreeSmoothResult> = results
                .iter()
                .map(|r| SidereonIonoFreeSmoothResult {
                    p_smooth_m: none_to_nan(r.p_smooth_m),
                    p_if_m: none_to_nan(r.p_if_m),
                    l_if_m: none_to_nan(r.l_if_m),
                    window: r.window,
                    reset: r.reset,
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_smooth_iono_free_code",
                "out",
                &mapped,
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
pub unsafe extern "C" fn sidereon_smooth_iono_free_code_v2(
    arc: *const SidereonArcEpochV2,
    count: usize,
    options: *const SidereonCycleSlipOptions,
    hatch_window_cap: usize,
    out: *mut SidereonIonoFreeSmoothResult,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_smooth_iono_free_code_v2";
    crate::engine_error::engine_error_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let (arc, opts) = c_try!(arc_v2_from_c(FN_NAME, arc, count, options));
        let results =
            match sidereon_core::carrier_phase::smooth_iono_free_code(&arc, opts, hatch_window_cap)
            {
                Ok(results) => results,
                Err(error) => return crate::signal::record_carrier_phase_error(FN_NAME, error),
            };
        let mapped: Vec<_> = results
            .iter()
            .map(|result| SidereonIonoFreeSmoothResult {
                p_smooth_m: none_to_nan(result.p_smooth_m),
                p_if_m: none_to_nan(result.p_if_m),
                l_if_m: none_to_nan(result.l_if_m),
                window: result.window,
                reset: result.reset,
            })
            .collect();
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            &mapped,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

// --- Ionosphere-free phase from cycles (sidereon_core::combinations) ---------

/// Ionosphere-free carrier-phase combination (meters) from cycle-valued phase
/// inputs and the two carrier frequencies (Hz). Delegates to
/// sidereon_core::combinations::ionosphere_free_phase_cycles.
///
/// Safety: out points to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_combination_ionosphere_free_phase_cycles(
    phi1_cycles: f64,
    phi2_cycles: f64,
    f1_hz: f64,
    f2_hz: f64,
    out: *mut f64,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_combination_ionosphere_free_phase_cycles",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_combination_ionosphere_free_phase_cycles",
                "out"
            ));
            *out = 0.0;
            match sidereon_core::combinations::ionosphere_free_phase_cycles(
                phi1_cycles,
                phi2_cycles,
                f1_hz,
                f2_hz,
            ) {
                Ok(v) => {
                    *out = v;
                    SidereonStatus::Ok
                }
                Err(err) => ionosphere_free_invalid_arg(
                    "sidereon_combination_ionosphere_free_phase_cycles",
                    err,
                ),
            }
        },
    )
}

// --- Cycle-slip detection (sidereon_core::carrier_phase) ---------------------

/// Cycle-slip classification for one input epoch, mirroring
/// sidereon_core::carrier_phase::SlipResult. reason_mask is a bitwise OR of the
/// SIDEREON_SLIP_REASON_* flags.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSlipResult {
    /// Whether any slip reason was flagged.
    pub slip: bool,
    /// Bitmask of slip reasons (SIDEREON_SLIP_REASON_*).
    pub reason_mask: u32,
    /// Geometry-free phase, meters, or NaN when not computable.
    pub gf_m: f64,
    /// Melbourne-Wubbena combination, meters, or NaN when not computable.
    pub mw_m: f64,
    /// Whether the epoch was skipped (a frequency was unavailable).
    pub skipped: bool,
}

/// Slip reason: loss-of-lock indicator set.
pub const SIDEREON_SLIP_REASON_LLI: u32 = 1;

/// Slip reason: data gap exceeded the threshold.
pub const SIDEREON_SLIP_REASON_DATA_GAP: u32 = 2;

/// Slip reason: geometry-free phase step exceeded the threshold.
pub const SIDEREON_SLIP_REASON_GEOMETRY_FREE: u32 = 4;

/// Slip reason: Melbourne-Wubbena step exceeded the threshold.
pub const SIDEREON_SLIP_REASON_MELBOURNE_WUBBENA: u32 = 8;

/// A satellite-tokened pseudorange observation for one carrier band.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPseudorangeObservation {
    /// Null-terminated satellite token, for example G01.
    pub sat_id: *const c_char,
    /// Pseudorange, meters.
    pub pseudorange_m: f64,
}

/// One ionosphere-free band-pair override, mirroring the
/// (system_letter, band1_name, band2_name) tuples accepted by
/// sidereon_core::combinations::ionosphere_free_pseudoranges.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonIonoFreeOverride {
    /// RINEX/IGS constellation letter as a single byte, for example 'G'.
    pub system: c_char,
    /// Null-terminated band-1 name.
    pub band1: *const c_char,
    /// Null-terminated band-2 name.
    pub band2: *const c_char,
}

/// One combined ionosphere-free pseudorange.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonIonoFreeCombined {
    /// Null-terminated satellite token.
    pub sat_id: [c_char; 17],
    /// Ionosphere-free pseudorange, meters.
    pub pseudorange_m: f64,
}

/// One dropped-satellite reason.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonIonoFreeDropped {
    /// Null-terminated satellite token.
    pub sat_id: [c_char; 17],
    /// One of the SIDEREON_PSEUDORANGE_DROP_* reasons.
    pub reason: u32,
}

/// Drop reason: present in band 2 only.
pub const SIDEREON_PSEUDORANGE_DROP_MISSING_BAND1: u32 = 0;

/// Drop reason: present in band 1 only.
pub const SIDEREON_PSEUDORANGE_DROP_MISSING_BAND2: u32 = 1;

/// Drop reason: the satellite appeared more than once in at least one band.
pub const SIDEREON_PSEUDORANGE_DROP_DUPLICATE_OBSERVATION: u32 = 2;

/// Drop reason: the constellation or requested band pair is unsupported.
pub const SIDEREON_PSEUDORANGE_DROP_UNKNOWN_SYSTEM: u32 = 3;

/// The result of an ionosphere-free paired-pseudorange combination. Opaque to C.
/// Create with sidereon_combination_ionosphere_free_pseudoranges and release with
/// sidereon_iono_free_pseudoranges_free.
pub struct SidereonIonoFreePseudoranges {
    pub(crate) combined: Vec<(String, f64)>,
    pub(crate) dropped: Vec<(String, PseudorangeDropReason)>,
}

/// Combine two satellite-keyed pseudorange bands into ionosphere-free ranges. On
/// success writes a newly owned result handle; read it with
/// sidereon_iono_free_pseudoranges_combined /
/// sidereon_iono_free_pseudoranges_dropped and release it with
/// sidereon_iono_free_pseudoranges_free. Delegates to
/// sidereon_core::combinations::ionosphere_free_pseudoranges.
///
/// Safety: band1/band2 point to band1_count/band2_count
/// SidereonPseudorangeObservation; overrides point to override_count
/// SidereonIonoFreeOverride (or NULL when 0); out points to handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_combination_ionosphere_free_pseudoranges(
    band1: *const SidereonPseudorangeObservation,
    band1_count: usize,
    band2: *const SidereonPseudorangeObservation,
    band2_count: usize,
    overrides: *const SidereonIonoFreeOverride,
    override_count: usize,
    out: *mut *mut SidereonIonoFreePseudoranges,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_combination_ionosphere_free_pseudoranges",
        SidereonStatus::Panic,
        || {
            let fname = "sidereon_combination_ionosphere_free_pseudoranges";
            let out = c_try!(require_out(out, fname, "out"));
            *out = ptr::null_mut();
            let band1 = c_try!(pseudorange_band_from_c(fname, "band1", band1, band1_count));
            let band2 = c_try!(pseudorange_band_from_c(fname, "band2", band2, band2_count));
            let override_rows =
                c_try!(require_slice(overrides, override_count, fname, "overrides"));
            let mut overrides_vec: Vec<(char, String, String)> = Vec::with_capacity(override_count);
            for row in override_rows {
                let system = (row.system as u8) as char;
                let band1_name = c_try!(parse_bounded_c_string(
                    fname,
                    "override.band1",
                    row.band1,
                    16
                ));
                let band2_name = c_try!(parse_bounded_c_string(
                    fname,
                    "override.band2",
                    row.band2,
                    16
                ));
                overrides_vec.push((system, band1_name, band2_name));
            }
            match sidereon_core::combinations::ionosphere_free_pseudoranges(
                &band1,
                &band2,
                &overrides_vec,
            ) {
                Ok((combined, dropped)) => {
                    write_boxed_handle(out, SidereonIonoFreePseudoranges { combined, dropped });
                    SidereonStatus::Ok
                }
                Err(err) => ionosphere_free_invalid_arg(fname, err),
            }
        },
    )
}

/// Copy the combined ionosphere-free pseudoranges. Variable-length output
/// contract.
///
/// Safety: result is a live handle; out points to len SidereonIonoFreeCombined or
/// NULL when len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_iono_free_pseudoranges_combined(
    result: *const SidereonIonoFreePseudoranges,
    out: *mut SidereonIonoFreeCombined,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_iono_free_pseudoranges_combined",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_iono_free_pseudoranges_combined",
                out_written,
                out_required
            ));
            let result = c_try!(require_ref(
                result,
                "sidereon_iono_free_pseudoranges_combined",
                "result"
            ));
            let mapped: Vec<SidereonIonoFreeCombined> = result
                .combined
                .iter()
                .map(|(sat, value)| {
                    let mut sat_id = [0 as c_char; 17];
                    write_token_str_buf(&mut sat_id, sat);
                    SidereonIonoFreeCombined {
                        sat_id,
                        pseudorange_m: *value,
                    }
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_iono_free_pseudoranges_combined",
                "out",
                &mapped,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the dropped-satellite reasons. Variable-length output contract.
///
/// Safety: result is a live handle; out points to len SidereonIonoFreeDropped or
/// NULL when len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_iono_free_pseudoranges_dropped(
    result: *const SidereonIonoFreePseudoranges,
    out: *mut SidereonIonoFreeDropped,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_iono_free_pseudoranges_dropped",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_iono_free_pseudoranges_dropped",
                out_written,
                out_required
            ));
            let result = c_try!(require_ref(
                result,
                "sidereon_iono_free_pseudoranges_dropped",
                "result"
            ));
            let mapped: Vec<SidereonIonoFreeDropped> = result
                .dropped
                .iter()
                .map(|(sat, reason)| {
                    let mut sat_id = [0 as c_char; 17];
                    write_token_str_buf(&mut sat_id, sat);
                    SidereonIonoFreeDropped {
                        sat_id,
                        reason: pseudorange_drop_reason_code(*reason),
                    }
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_iono_free_pseudoranges_dropped",
                "out",
                &mapped,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Release an ionosphere-free paired-pseudorange result handle.
///
/// Safety: result must be a handle from
/// sidereon_combination_ionosphere_free_pseudoranges or NULL.
#[no_mangle]
pub unsafe extern "C" fn sidereon_iono_free_pseudoranges_free(
    result: *mut SidereonIonoFreePseudoranges,
) {
    free_boxed(result);
}

// ============================================================================
// Capability-parity round: NeQuick, rv<->COE, observation geometry, geoid,
// civil-instant construction, moving-baseline RTK, and RTCM 3 decode/encode.
// Every function here marshals C input into the engine type, calls the cited
// sidereon-core entry point, and copies the result back. No modeling lives here.

/// Classify cycle slips over a single-satellite carrier-phase arc. One result is
/// produced per input epoch. Variable-length output contract. Delegates to
/// sidereon_core::carrier_phase::detect_cycle_slips.
///
/// Safety: arc points to count SidereonArcEpoch; options to a
/// SidereonCycleSlipOptions; out to len SidereonSlipResult or NULL when len is 0;
/// out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_detect_cycle_slips(
    arc: *const SidereonArcEpoch,
    count: usize,
    options: *const SidereonCycleSlipOptions,
    out: *mut SidereonSlipResult,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_detect_cycle_slips",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_detect_cycle_slips",
                out_written,
                out_required
            ));
            let (arc, opts) = c_try!(arc_from_c(
                "sidereon_detect_cycle_slips",
                arc,
                count,
                options
            ));
            let results = match sidereon_core::carrier_phase::detect_cycle_slips(&arc, opts) {
                Ok(r) => r,
                Err(err) => {
                    return crate::signal::record_carrier_phase_error(
                        "sidereon_detect_cycle_slips",
                        err,
                    )
                }
            };
            let mapped: Vec<SidereonSlipResult> = results
                .iter()
                .map(|r| SidereonSlipResult {
                    slip: r.slip,
                    reason_mask: slip_reason_mask(&r.reasons),
                    gf_m: none_to_nan(r.gf_m),
                    mw_m: none_to_nan(r.mw_m),
                    skipped: r.skipped,
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_detect_cycle_slips",
                "out",
                &mapped,
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
pub unsafe extern "C" fn sidereon_detect_cycle_slips_v2(
    arc: *const SidereonArcEpochV2,
    count: usize,
    options: *const SidereonCycleSlipOptions,
    out: *mut SidereonSlipResult,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_detect_cycle_slips_v2";
    crate::engine_error::engine_error_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let (arc, opts) = c_try!(arc_v2_from_c(FN_NAME, arc, count, options));
        let results = match sidereon_core::carrier_phase::detect_cycle_slips(&arc, opts) {
            Ok(results) => results,
            Err(error) => return crate::signal::record_carrier_phase_error(FN_NAME, error),
        };
        let mapped: Vec<_> = results
            .iter()
            .map(|result| SidereonSlipResult {
                slip: result.slip,
                reason_mask: slip_reason_mask(&result.reasons),
                gf_m: none_to_nan(result.gf_m),
                mw_m: none_to_nan(result.mw_m),
                skipped: result.skipped,
            })
            .collect();
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            &mapped,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

unsafe fn arc_from_c(
    fn_name: &str,
    arc: *const SidereonArcEpoch,
    count: usize,
    options: *const SidereonCycleSlipOptions,
) -> Result<
    (
        Vec<sidereon_core::carrier_phase::ArcEpoch>,
        sidereon_core::carrier_phase::CycleSlipOptions,
    ),
    SidereonStatus,
> {
    let options = require_ref(options, fn_name, "options")?;
    let rows = require_slice(arc, count, fn_name, "arc")?;
    let arc: Vec<sidereon_core::carrier_phase::ArcEpoch> = rows
        .iter()
        .map(|e| sidereon_core::carrier_phase::ArcEpoch {
            phi1_cycles: nan_to_none(e.phi1_cycles),
            phi2_cycles: nan_to_none(e.phi2_cycles),
            p1_m: nan_to_none(e.p1_m),
            p2_m: nan_to_none(e.p2_m),
            lli1: e.has_lli1.then_some(e.lli1),
            lli2: e.has_lli2.then_some(e.lli2),
            f1_hz: nan_to_none(e.f1_hz),
            f2_hz: nan_to_none(e.f2_hz),
            gap_time_s: nan_to_none(e.gap_time_s),
            gap_epoch: None,
        })
        .collect();
    let mut opts = sidereon_core::carrier_phase::CycleSlipOptions::default();
    opts.gf_threshold_m = options.gf_threshold_m;
    opts.mw_threshold_cycles = options.mw_threshold_cycles;
    opts.min_arc_gap_s = options.min_arc_gap_s;
    Ok((arc, opts))
}

unsafe fn arc_v2_from_c(
    fn_name: &str,
    arc: *const SidereonArcEpochV2,
    count: usize,
    options: *const SidereonCycleSlipOptions,
) -> Result<
    (
        Vec<sidereon_core::carrier_phase::ArcEpoch>,
        sidereon_core::carrier_phase::CycleSlipOptions,
    ),
    SidereonStatus,
> {
    let options = require_ref(options, fn_name, "options")?;
    let rows = require_slice(arc, count, fn_name, "arc")?;
    validate_element_count::<SidereonArcEpochV2>(fn_name, "count", rows.len())?;
    let mut converted = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        let gap_epoch = if row.has_gap_epoch {
            Some(require_ref(row.gap_epoch, fn_name, &format!("arc[{index}].gap_epoch"))?.inner)
        } else {
            None
        };
        converted.push(sidereon_core::carrier_phase::ArcEpoch {
            phi1_cycles: nan_to_none(row.legacy.phi1_cycles),
            phi2_cycles: nan_to_none(row.legacy.phi2_cycles),
            p1_m: nan_to_none(row.legacy.p1_m),
            p2_m: nan_to_none(row.legacy.p2_m),
            lli1: row.legacy.has_lli1.then_some(row.legacy.lli1),
            lli2: row.legacy.has_lli2.then_some(row.legacy.lli2),
            f1_hz: nan_to_none(row.legacy.f1_hz),
            f2_hz: nan_to_none(row.legacy.f2_hz),
            gap_time_s: nan_to_none(row.legacy.gap_time_s),
            gap_epoch,
        });
    }
    let mut core_options = sidereon_core::carrier_phase::CycleSlipOptions::default();
    core_options.gf_threshold_m = options.gf_threshold_m;
    core_options.mw_threshold_cycles = options.mw_threshold_cycles;
    core_options.min_arc_gap_s = options.min_arc_gap_s;
    Ok((converted, core_options))
}

fn slip_reason_mask(reasons: &[sidereon_core::carrier_phase::SlipReason]) -> u32 {
    use sidereon_core::carrier_phase::SlipReason;
    let mut mask = 0u32;
    for reason in reasons {
        mask |= match reason {
            SlipReason::Lli => SIDEREON_SLIP_REASON_LLI,
            SlipReason::DataGap => SIDEREON_SLIP_REASON_DATA_GAP,
            SlipReason::GeometryFree => SIDEREON_SLIP_REASON_GEOMETRY_FREE,
            SlipReason::MelbourneWubbena => SIDEREON_SLIP_REASON_MELBOURNE_WUBBENA,
        };
    }
    mask
}

fn pseudorange_drop_reason_code(reason: PseudorangeDropReason) -> u32 {
    match reason {
        PseudorangeDropReason::MissingBand1 => SIDEREON_PSEUDORANGE_DROP_MISSING_BAND1,
        PseudorangeDropReason::MissingBand2 => SIDEREON_PSEUDORANGE_DROP_MISSING_BAND2,
        PseudorangeDropReason::DuplicateObservation => {
            SIDEREON_PSEUDORANGE_DROP_DUPLICATE_OBSERVATION
        }
        PseudorangeDropReason::UnknownSystem => SIDEREON_PSEUDORANGE_DROP_UNKNOWN_SYSTEM,
    }
}

// Write a token String into a fixed 17-byte null-terminated C buffer.

fn write_token_str_buf(buf: &mut [c_char; 17], token: &str) {
    let bytes = token.as_bytes();
    let n = bytes.len().min(16);
    for slot in buf.iter_mut() {
        *slot = 0;
    }
    for (slot, b) in buf.iter_mut().zip(bytes.iter().take(n)) {
        *slot = *b as c_char;
    }
    buf[n] = 0;
}

unsafe fn pseudorange_band_from_c(
    fn_name: &str,
    arg_name: &str,
    band: *const SidereonPseudorangeObservation,
    count: usize,
) -> Result<Vec<(String, f64)>, SidereonStatus> {
    let rows = require_slice(band, count, fn_name, arg_name)?;
    let mut out = Vec::with_capacity(count);
    for row in rows {
        let sat = parse_satellite_token(fn_name, row.sat_id)?;
        out.push((sat.to_string(), row.pseudorange_m));
    }
    Ok(out)
}

fn nan_to_none(value: f64) -> Option<f64> {
    if value.is_nan() {
        None
    } else {
        Some(value)
    }
}

#[cfg(test)]
mod ionosphere_free_engine_error_tests {
    use super::*;
    use crate::engine_error::{snapshot_engine_error_for_test, SidereonEngineErrorFamily};
    use std::ffi::{CStr, CString};

    fn assert_producer_contract(
        operation: &str,
        kind: &str,
        fields: serde_json::Value,
        legacy: &str,
        refusal: impl Fn() -> SidereonStatus,
        success: impl FnOnce() -> SidereonStatus,
        early_null: impl FnOnce() -> SidereonStatus,
    ) {
        assert_eq!(refusal(), SidereonStatus::InvalidArgument);
        let (info, payload) = snapshot_engine_error_for_test().expect("typed combination refusal");
        assert_eq!(info.family, SidereonEngineErrorFamily::IonosphereFree);
        assert_eq!(info.payload_len, payload.len());
        let value: serde_json::Value = serde_json::from_str(&payload).expect("valid JSON");
        assert_eq!(
            value,
            serde_json::json!({
                "schema_version": 1,
                "family": "ionosphere_free",
                "operation": operation,
                "error": {"kind": kind, "fields": fields}
            })
        );

        let mut message = vec![0 as std::os::raw::c_char; 256];
        let needed =
            unsafe { crate::sidereon_last_error_message(message.as_mut_ptr(), message.len()) };
        assert!(needed > 0);
        let message = unsafe { CStr::from_ptr(message.as_ptr()) }
            .to_str()
            .expect("legacy UTF-8");
        assert_eq!(message, legacy);

        assert_eq!(success(), SidereonStatus::Ok);
        assert!(snapshot_engine_error_for_test().is_none());
        assert_eq!(refusal(), SidereonStatus::InvalidArgument);
        assert!(snapshot_engine_error_for_test().is_some());
        assert_eq!(early_null(), SidereonStatus::NullPointer);
        assert!(snapshot_engine_error_for_test().is_none());
    }

    #[test]
    fn every_ionosphere_free_producer_records_and_clears_typed_errors() {
        assert_producer_contract(
            "sidereon_combination_gamma",
            "equal_frequencies",
            serde_json::json!({}),
            "sidereon_combination_gamma: equal carrier frequencies",
            || unsafe { sidereon_combination_gamma(1.0, 1.0, &mut 0.0) },
            || {
                let mut out = 0.0;
                unsafe { sidereon_combination_gamma(1575.42e6, 1227.60e6, &mut out) }
            },
            || unsafe { sidereon_combination_gamma(1575.42e6, 1227.60e6, ptr::null_mut()) },
        );
        assert_producer_contract(
            "sidereon_combination_noise_amplification",
            "invalid_frequency",
            serde_json::json!({}),
            "sidereon_combination_noise_amplification: carrier frequencies must be positive",
            || unsafe { sidereon_combination_noise_amplification(0.0, 1.0, &mut 0.0) },
            || {
                let mut out = 0.0;
                unsafe { sidereon_combination_noise_amplification(1575.42e6, 1227.60e6, &mut out) }
            },
            || unsafe {
                sidereon_combination_noise_amplification(1575.42e6, 1227.60e6, ptr::null_mut())
            },
        );
        assert_producer_contract(
            "sidereon_combination_ionosphere_free",
            "invalid_observation",
            serde_json::json!({}),
            "sidereon_combination_ionosphere_free: observations must be finite",
            || unsafe {
                sidereon_combination_ionosphere_free(f64::NAN, 2.0, 1575.42e6, 1227.60e6, &mut 0.0)
            },
            || {
                let mut out = 0.0;
                unsafe {
                    sidereon_combination_ionosphere_free(
                        23.0e6, 23.1e6, 1575.42e6, 1227.60e6, &mut out,
                    )
                }
            },
            || unsafe {
                sidereon_combination_ionosphere_free(
                    23.0e6,
                    23.1e6,
                    1575.42e6,
                    1227.60e6,
                    ptr::null_mut(),
                )
            },
        );
        assert_producer_contract(
            "sidereon_combination_ionosphere_free_phase_m",
            "invalid_observation",
            serde_json::json!({}),
            "sidereon_combination_ionosphere_free_phase_m: observations must be finite",
            || unsafe {
                sidereon_combination_ionosphere_free_phase_m(
                    1.0,
                    f64::NAN,
                    1575.42e6,
                    1227.60e6,
                    &mut 0.0,
                )
            },
            || {
                let mut out = 0.0;
                unsafe {
                    sidereon_combination_ionosphere_free_phase_m(
                        1.0, 2.0, 1575.42e6, 1227.60e6, &mut out,
                    )
                }
            },
            || unsafe {
                sidereon_combination_ionosphere_free_phase_m(
                    1.0,
                    2.0,
                    1575.42e6,
                    1227.60e6,
                    ptr::null_mut(),
                )
            },
        );
        assert_producer_contract(
            "sidereon_combination_ionosphere_free_phase_cycles",
            "invalid_observation",
            serde_json::json!({}),
            "sidereon_combination_ionosphere_free_phase_cycles: observations must be finite",
            || unsafe {
                sidereon_combination_ionosphere_free_phase_cycles(
                    f64::NAN,
                    2.0,
                    1575.42e6,
                    1227.60e6,
                    &mut 0.0,
                )
            },
            || {
                let mut out = 0.0;
                unsafe {
                    sidereon_combination_ionosphere_free_phase_cycles(
                        1.0, 2.0, 1575.42e6, 1227.60e6, &mut out,
                    )
                }
            },
            || unsafe {
                sidereon_combination_ionosphere_free_phase_cycles(
                    1.0,
                    2.0,
                    1575.42e6,
                    1227.60e6,
                    ptr::null_mut(),
                )
            },
        );

        let sat = CString::new("G01").expect("sat token");
        let invalid_band1 = [SidereonPseudorangeObservation {
            sat_id: sat.as_ptr(),
            pseudorange_m: f64::NAN,
        }];
        let valid_band1 = [SidereonPseudorangeObservation {
            sat_id: sat.as_ptr(),
            pseudorange_m: 23.0e6,
        }];
        let band2 = [SidereonPseudorangeObservation {
            sat_id: sat.as_ptr(),
            pseudorange_m: 23.1e6,
        }];
        assert_producer_contract(
            "sidereon_combination_ionosphere_free_pseudoranges",
            "invalid_observation",
            serde_json::json!({}),
            "sidereon_combination_ionosphere_free_pseudoranges: observations must be finite",
            || unsafe {
                sidereon_combination_ionosphere_free_pseudoranges(
                    invalid_band1.as_ptr(),
                    invalid_band1.len(),
                    band2.as_ptr(),
                    band2.len(),
                    ptr::null(),
                    0,
                    &mut ptr::null_mut(),
                )
            },
            || {
                let mut result = ptr::null_mut();
                let status = unsafe {
                    sidereon_combination_ionosphere_free_pseudoranges(
                        valid_band1.as_ptr(),
                        valid_band1.len(),
                        band2.as_ptr(),
                        band2.len(),
                        ptr::null(),
                        0,
                        &mut result,
                    )
                };
                if !result.is_null() {
                    unsafe { sidereon_iono_free_pseudoranges_free(result) };
                }
                status
            },
            || unsafe {
                sidereon_combination_ionosphere_free_pseudoranges(
                    valid_band1.as_ptr(),
                    valid_band1.len(),
                    band2.as_ptr(),
                    band2.len(),
                    ptr::null(),
                    0,
                    ptr::null_mut(),
                )
            },
        );
    }
}

#[cfg(test)]
mod carrier_phase_engine_error_tests {
    use super::*;
    use crate::engine_error::{snapshot_engine_error_for_test, SidereonEngineErrorFamily};

    fn assert_producer(
        operation: &str,
        legacy: &str,
        refusal: impl Fn() -> SidereonStatus,
        success: impl Fn() -> SidereonStatus,
        early_null: impl Fn() -> SidereonStatus,
    ) {
        assert_eq!(refusal(), SidereonStatus::InvalidArgument);
        let (info, payload) = snapshot_engine_error_for_test().expect("typed carrier refusal");
        assert_eq!(info.family, SidereonEngineErrorFamily::CarrierPhase);
        assert_eq!(info.payload_len, payload.len());
        let value: serde_json::Value = serde_json::from_str(&payload).expect("valid JSON");
        assert_eq!(
            value,
            serde_json::json!({
                "schema_version":1,
                "family":"carrier_phase",
                "operation":operation,
                "error":{"kind":"invalid_threshold","fields":{}}
            })
        );
        let mut message = vec![0 as std::os::raw::c_char; 128];
        unsafe { crate::sidereon_last_error_message(message.as_mut_ptr(), message.len()) };
        let message = unsafe { std::ffi::CStr::from_ptr(message.as_ptr()) }
            .to_str()
            .expect("legacy UTF-8");
        assert_eq!(message, legacy);
        assert_eq!(success(), SidereonStatus::Ok);
        assert!(snapshot_engine_error_for_test().is_none());
        assert_eq!(refusal(), SidereonStatus::InvalidArgument);
        assert!(snapshot_engine_error_for_test().is_some());
        assert_eq!(early_null(), SidereonStatus::NullPointer);
        assert!(snapshot_engine_error_for_test().is_none());
    }

    #[test]
    fn all_six_arc_carrier_producers_record_and_reset_typed_errors() {
        let bad = SidereonCycleSlipOptions {
            gf_threshold_m: f64::NAN,
            mw_threshold_cycles: 4.0,
            min_arc_gap_s: 120.0,
        };
        let good = SidereonCycleSlipOptions {
            gf_threshold_m: 0.05,
            mw_threshold_cycles: 4.0,
            min_arc_gap_s: 120.0,
        };
        let bad_ptr = &bad as *const _;
        let good_ptr = &good as *const _;
        let mut written = 0usize;
        let mut required = 0usize;
        let written_ptr = &mut written as *mut usize;
        let required_ptr = &mut required as *mut usize;
        let smooth: *mut SidereonSmoothCodeResult = std::ptr::null_mut();
        let iono_free: *mut SidereonIonoFreeSmoothResult = std::ptr::null_mut();
        let slips: *mut SidereonSlipResult = std::ptr::null_mut();

        assert_producer(
            "sidereon_smooth_code",
            "sidereon_smooth_code: carrier thresholds must be finite and sane",
            || unsafe {
                sidereon_smooth_code(
                    std::ptr::null(),
                    0,
                    bad_ptr,
                    1,
                    smooth,
                    0,
                    &mut *written_ptr,
                    &mut *required_ptr,
                )
            },
            || unsafe {
                sidereon_smooth_code(
                    std::ptr::null(),
                    0,
                    good_ptr,
                    1,
                    smooth,
                    0,
                    &mut *written_ptr,
                    &mut *required_ptr,
                )
            },
            || unsafe {
                sidereon_smooth_code(
                    std::ptr::null(),
                    0,
                    good_ptr,
                    1,
                    smooth,
                    0,
                    std::ptr::null_mut(),
                    &mut *required_ptr,
                )
            },
        );
        assert_producer(
            "sidereon_smooth_code_v2",
            "sidereon_smooth_code_v2: carrier thresholds must be finite and sane",
            || unsafe {
                sidereon_smooth_code_v2(
                    std::ptr::null(),
                    0,
                    bad_ptr,
                    1,
                    smooth,
                    0,
                    &mut *written_ptr,
                    &mut *required_ptr,
                )
            },
            || unsafe {
                sidereon_smooth_code_v2(
                    std::ptr::null(),
                    0,
                    good_ptr,
                    1,
                    smooth,
                    0,
                    &mut *written_ptr,
                    &mut *required_ptr,
                )
            },
            || unsafe {
                sidereon_smooth_code_v2(
                    std::ptr::null(),
                    0,
                    good_ptr,
                    1,
                    smooth,
                    0,
                    std::ptr::null_mut(),
                    &mut *required_ptr,
                )
            },
        );
        assert_producer(
            "sidereon_smooth_iono_free_code",
            "sidereon_smooth_iono_free_code: carrier thresholds must be finite and sane",
            || unsafe {
                sidereon_smooth_iono_free_code(
                    std::ptr::null(),
                    0,
                    bad_ptr,
                    1,
                    iono_free,
                    0,
                    &mut *written_ptr,
                    &mut *required_ptr,
                )
            },
            || unsafe {
                sidereon_smooth_iono_free_code(
                    std::ptr::null(),
                    0,
                    good_ptr,
                    1,
                    iono_free,
                    0,
                    &mut *written_ptr,
                    &mut *required_ptr,
                )
            },
            || unsafe {
                sidereon_smooth_iono_free_code(
                    std::ptr::null(),
                    0,
                    good_ptr,
                    1,
                    iono_free,
                    0,
                    std::ptr::null_mut(),
                    &mut *required_ptr,
                )
            },
        );
        assert_producer(
            "sidereon_smooth_iono_free_code_v2",
            "sidereon_smooth_iono_free_code_v2: carrier thresholds must be finite and sane",
            || unsafe {
                sidereon_smooth_iono_free_code_v2(
                    std::ptr::null(),
                    0,
                    bad_ptr,
                    1,
                    iono_free,
                    0,
                    &mut *written_ptr,
                    &mut *required_ptr,
                )
            },
            || unsafe {
                sidereon_smooth_iono_free_code_v2(
                    std::ptr::null(),
                    0,
                    good_ptr,
                    1,
                    iono_free,
                    0,
                    &mut *written_ptr,
                    &mut *required_ptr,
                )
            },
            || unsafe {
                sidereon_smooth_iono_free_code_v2(
                    std::ptr::null(),
                    0,
                    good_ptr,
                    1,
                    iono_free,
                    0,
                    std::ptr::null_mut(),
                    &mut *required_ptr,
                )
            },
        );
        assert_producer(
            "sidereon_detect_cycle_slips",
            "sidereon_detect_cycle_slips: carrier thresholds must be finite and sane",
            || unsafe {
                sidereon_detect_cycle_slips(
                    std::ptr::null(),
                    0,
                    bad_ptr,
                    slips,
                    0,
                    &mut *written_ptr,
                    &mut *required_ptr,
                )
            },
            || unsafe {
                sidereon_detect_cycle_slips(
                    std::ptr::null(),
                    0,
                    good_ptr,
                    slips,
                    0,
                    &mut *written_ptr,
                    &mut *required_ptr,
                )
            },
            || unsafe {
                sidereon_detect_cycle_slips(
                    std::ptr::null(),
                    0,
                    good_ptr,
                    slips,
                    0,
                    std::ptr::null_mut(),
                    &mut *required_ptr,
                )
            },
        );
        assert_producer(
            "sidereon_detect_cycle_slips_v2",
            "sidereon_detect_cycle_slips_v2: carrier thresholds must be finite and sane",
            || unsafe {
                sidereon_detect_cycle_slips_v2(
                    std::ptr::null(),
                    0,
                    bad_ptr,
                    slips,
                    0,
                    &mut *written_ptr,
                    &mut *required_ptr,
                )
            },
            || unsafe {
                sidereon_detect_cycle_slips_v2(
                    std::ptr::null(),
                    0,
                    good_ptr,
                    slips,
                    0,
                    &mut *written_ptr,
                    &mut *required_ptr,
                )
            },
            || unsafe {
                sidereon_detect_cycle_slips_v2(
                    std::ptr::null(),
                    0,
                    good_ptr,
                    slips,
                    0,
                    std::ptr::null_mut(),
                    &mut *required_ptr,
                )
            },
        );
    }
}
