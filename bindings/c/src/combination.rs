use super::*;

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
    ffi_boundary("sidereon_combination_gamma", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_combination_gamma", "out"));
        *out = 0.0;
        match sidereon_core::combinations::gamma(f1_hz, f2_hz) {
            Ok(v) => {
                *out = v;
                SidereonStatus::Ok
            }
            Err(err) => extra_invalid_arg("sidereon_combination_gamma", err),
        }
    })
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
    ffi_boundary(
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
                Err(err) => extra_invalid_arg("sidereon_combination_noise_amplification", err),
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
    ffi_boundary(
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
                Err(err) => extra_invalid_arg("sidereon_combination_ionosphere_free", err),
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
    ffi_boundary(
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
                Err(err) => extra_invalid_arg("sidereon_combination_ionosphere_free_phase_m", err),
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
    ffi_boundary(
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
                Err(err) => extra_invalid_arg("sidereon_pseudorange_variance", err),
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
    ffi_boundary("sidereon_smooth_code", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_smooth_code",
            out_written,
            out_required
        ));
        let (arc, opts) = c_try!(arc_from_c("sidereon_smooth_code", arc, count, options));
        let results = match sidereon_core::carrier_phase::smooth_code(&arc, opts, hatch_window_cap)
        {
            Ok(r) => r,
            Err(err) => return extra_invalid_arg("sidereon_smooth_code", err),
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
    ffi_boundary(
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
                Err(err) => return extra_invalid_arg("sidereon_smooth_iono_free_code", err),
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
    ffi_boundary(
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
                Err(err) => {
                    extra_invalid_arg("sidereon_combination_ionosphere_free_phase_cycles", err)
                }
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
    ffi_boundary(
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
                Err(err) => extra_invalid_arg(fname, err),
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
    ffi_boundary("sidereon_detect_cycle_slips", SidereonStatus::Panic, || {
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
            Err(err) => return extra_invalid_arg("sidereon_detect_cycle_slips", err),
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
        })
        .collect();
    let opts = sidereon_core::carrier_phase::CycleSlipOptions {
        gf_threshold_m: options.gf_threshold_m,
        mw_threshold_cycles: options.mw_threshold_cycles,
        min_arc_gap_s: options.min_arc_gap_s,
    };
    Ok((arc, opts))
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
