use super::*;

// --- GNSS carrier frequencies (sidereon_core::frequencies) -------------------

/// A GNSS carrier band, mirroring sidereon_core::frequencies::CarrierBand.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonCarrierBand {
    /// GPS/QZSS L1.
    L1 = 0,
    /// GPS/QZSS L2.
    L2 = 1,
    /// GPS/QZSS/Galileo L5/E5a.
    L5 = 2,
    /// Galileo E1.
    E1 = 3,
    /// Galileo E5a.
    E5a = 4,
    /// Galileo E5b.
    E5b = 5,
    /// Galileo E5 (E5a+E5b).
    E5 = 6,
    /// Galileo E6.
    E6 = 7,
    /// BeiDou B1C.
    B1c = 8,
    /// BeiDou B1I.
    B1i = 9,
    /// BeiDou B2a.
    B2a = 10,
    /// BeiDou B2b.
    B2b = 11,
    /// BeiDou B2.
    B2 = 12,
    /// BeiDou B3I.
    B3i = 13,
    /// GLONASS G1.
    G1 = 14,
    /// GLONASS G2.
    G2 = 15,
}

/// Copy the core carrier-band label into out.
#[no_mangle]
pub unsafe extern "C" fn sidereon_carrier_band_label(
    band: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_carrier_band_label", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_carrier_band_label",
            out_written,
            out_required
        ));
        let band = c_try!(carrier_band_from_c(
            "sidereon_carrier_band_label",
            "band",
            band
        ));
        c_try!(copy_prefix_to_c(
            "sidereon_carrier_band_label",
            "out",
            band.as_str().as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Carrier frequency in Hz for a (system, band) pair, or
/// SIDEREON_STATUS_INVALID_ARGUMENT when the pair is not defined. Delegates to
/// sidereon_core::frequencies::frequency_hz. system is a SidereonGnssSystem code;
/// band is a SidereonCarrierBand code.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_frequency_hz(
    system: u32,
    band: u32,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_frequency_hz", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_frequency_hz", "out"));
        *out = 0.0;
        let system = c_try!(gnss_system_from_c_code(
            "sidereon_frequency_hz",
            "system",
            system
        ));
        let band = c_try!(carrier_band_from_c("sidereon_frequency_hz", "band", band));
        match sidereon_core::frequencies::frequency_hz(system, band) {
            Some(v) => {
                *out = v;
                SidereonStatus::Ok
            }
            None => {
                set_last_error("sidereon_frequency_hz: undefined system/band pair".to_string());
                SidereonStatus::InvalidArgument
            }
        }
    })
}

/// Carrier wavelength in meters for a (system, band) pair. Delegates to
/// sidereon_core::frequencies::wavelength_m.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_wavelength_m(
    system: u32,
    band: u32,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_wavelength_m", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_wavelength_m", "out"));
        *out = 0.0;
        let system = c_try!(gnss_system_from_c_code(
            "sidereon_wavelength_m",
            "system",
            system
        ));
        let band = c_try!(carrier_band_from_c("sidereon_wavelength_m", "band", band));
        match sidereon_core::frequencies::wavelength_m(system, band) {
            Some(v) => {
                *out = v;
                SidereonStatus::Ok
            }
            None => {
                set_last_error("sidereon_wavelength_m: undefined system/band pair".to_string());
                SidereonStatus::InvalidArgument
            }
        }
    })
}

/// Default single-point-positioning carrier frequency in Hz for a system.
/// Delegates to sidereon_core::frequencies::default_spp_frequency_hz.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_default_spp_frequency_hz(
    system: u32,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_default_spp_frequency_hz",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_default_spp_frequency_hz", "out"));
            *out = 0.0;
            let system = c_try!(gnss_system_from_c_code(
                "sidereon_default_spp_frequency_hz",
                "system",
                system
            ));
            match sidereon_core::frequencies::default_spp_frequency_hz(system) {
                Some(v) => {
                    *out = v;
                    SidereonStatus::Ok
                }
                None => {
                    set_last_error(
                        "sidereon_default_spp_frequency_hz: no default for system".to_string(),
                    );
                    SidereonStatus::InvalidArgument
                }
            }
        },
    )
}

// --- Carrier-phase combinations / cycle-slip (sidereon_core::carrier_phase) ---

/// Convert carrier phase in cycles to meters. Delegates to
/// sidereon_core::carrier_phase::phase_meters.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_carrier_phase_meters(
    phi_cycles: f64,
    f_hz: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_carrier_phase_meters",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_carrier_phase_meters", "out"));
            *out = 0.0;
            match sidereon_core::carrier_phase::phase_meters(phi_cycles, f_hz) {
                Ok(v) => {
                    *out = v;
                    SidereonStatus::Ok
                }
                Err(err) => extra_invalid_arg("sidereon_carrier_phase_meters", err),
            }
        },
    )
}

/// Geometry-free (L1-L2) carrier-phase combination in meters. Delegates to
/// sidereon_core::carrier_phase::geometry_free.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_carrier_geometry_free(
    l1_m: f64,
    l2_m: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_carrier_geometry_free",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_carrier_geometry_free", "out"));
            *out = 0.0;
            match sidereon_core::carrier_phase::geometry_free(l1_m, l2_m) {
                Ok(v) => {
                    *out = v;
                    SidereonStatus::Ok
                }
                Err(err) => extra_invalid_arg("sidereon_carrier_geometry_free", err),
            }
        },
    )
}

/// Wide-lane wavelength in meters for two frequencies. Delegates to
/// sidereon_core::carrier_phase::wide_lane_wavelength.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_carrier_wide_lane_wavelength(
    f1_hz: f64,
    f2_hz: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_carrier_wide_lane_wavelength",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_carrier_wide_lane_wavelength",
                "out"
            ));
            *out = 0.0;
            match sidereon_core::carrier_phase::wide_lane_wavelength(f1_hz, f2_hz) {
                Ok(v) => {
                    *out = v;
                    SidereonStatus::Ok
                }
                Err(err) => extra_invalid_arg("sidereon_carrier_wide_lane_wavelength", err),
            }
        },
    )
}

/// Narrow-lane code combination in meters. Delegates to
/// sidereon_core::carrier_phase::narrow_lane_code.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_carrier_narrow_lane_code(
    p1_m: f64,
    p2_m: f64,
    f1_hz: f64,
    f2_hz: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_carrier_narrow_lane_code",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_carrier_narrow_lane_code", "out"));
            *out = 0.0;
            match sidereon_core::carrier_phase::narrow_lane_code(p1_m, p2_m, f1_hz, f2_hz) {
                Ok(v) => {
                    *out = v;
                    SidereonStatus::Ok
                }
                Err(err) => extra_invalid_arg("sidereon_carrier_narrow_lane_code", err),
            }
        },
    )
}

/// Melbourne-Wubbena combination in meters. Delegates to
/// sidereon_core::carrier_phase::melbourne_wubbena.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_carrier_melbourne_wubbena(
    phi1_cycles: f64,
    phi2_cycles: f64,
    p1_m: f64,
    p2_m: f64,
    f1_hz: f64,
    f2_hz: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_carrier_melbourne_wubbena",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_carrier_melbourne_wubbena",
                "out"
            ));
            *out = 0.0;
            match sidereon_core::carrier_phase::melbourne_wubbena(
                phi1_cycles,
                phi2_cycles,
                p1_m,
                p2_m,
                f1_hz,
                f2_hz,
            ) {
                Ok(v) => {
                    *out = v;
                    SidereonStatus::Ok
                }
                Err(err) => extra_invalid_arg("sidereon_carrier_melbourne_wubbena", err),
            }
        },
    )
}

/// Wide-lane ambiguity estimate in cycles. Delegates to
/// sidereon_core::carrier_phase::wide_lane_cycles.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_carrier_wide_lane_cycles(
    phi1_cycles: f64,
    phi2_cycles: f64,
    p1_m: f64,
    p2_m: f64,
    f1_hz: f64,
    f2_hz: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_carrier_wide_lane_cycles",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_carrier_wide_lane_cycles", "out"));
            *out = 0.0;
            match sidereon_core::carrier_phase::wide_lane_cycles(
                phi1_cycles,
                phi2_cycles,
                p1_m,
                p2_m,
                f1_hz,
                f2_hz,
            ) {
                Ok(v) => {
                    *out = v;
                    SidereonStatus::Ok
                }
                Err(err) => extra_invalid_arg("sidereon_carrier_wide_lane_cycles", err),
            }
        },
    )
}

/// Code-minus-carrier value in meters. Delegates to
/// sidereon_core::carrier_phase::code_minus_carrier.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_carrier_code_minus_carrier(
    p_m: f64,
    phi_cycles: f64,
    f_hz: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_carrier_code_minus_carrier",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_carrier_code_minus_carrier",
                "out"
            ));
            *out = 0.0;
            match sidereon_core::carrier_phase::code_minus_carrier(p_m, phi_cycles, f_hz) {
                Ok(v) => {
                    *out = v;
                    SidereonStatus::Ok
                }
                Err(err) => extra_invalid_arg("sidereon_carrier_code_minus_carrier", err),
            }
        },
    )
}

// --- GNSS signal scalars (sidereon_core::signal) -----------------------------

/// One C/A code chip (+1 or -1) for a GPS PRN at a chip index. Delegates to
/// sidereon_core::signal::ca_chip.
///
/// Safety: out must point to an int8_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_signal_ca_chip(
    prn: i64,
    index: i64,
    out: *mut i8,
) -> SidereonStatus {
    ffi_boundary("sidereon_signal_ca_chip", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_signal_ca_chip", "out"));
        *out = 0;
        match sidereon_core::signal::ca_chip(prn, index) {
            Ok(v) => {
                *out = v;
                SidereonStatus::Ok
            }
            Err(err) => extra_invalid_arg("sidereon_signal_ca_chip", err),
        }
    })
}

/// Coherent-integration power loss (fraction in 0..=1) from a frequency error.
/// Delegates to sidereon_core::signal::coherent_loss.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_signal_coherent_loss(
    freq_error_hz: f64,
    integration_time_s: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_signal_coherent_loss",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_signal_coherent_loss", "out"));
            *out = 0.0;
            match sidereon_core::signal::coherent_loss(freq_error_hz, integration_time_s) {
                Ok(v) => {
                    *out = v;
                    SidereonStatus::Ok
                }
                Err(err) => extra_invalid_arg("sidereon_signal_coherent_loss", err),
            }
        },
    )
}

/// Coherent-integration loss in dB. Delegates to
/// sidereon_core::signal::coherent_loss_db.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_signal_coherent_loss_db(
    freq_error_hz: f64,
    integration_time_s: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_signal_coherent_loss_db",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_signal_coherent_loss_db", "out"));
            *out = 0.0;
            match sidereon_core::signal::coherent_loss_db(freq_error_hz, integration_time_s) {
                Ok(v) => {
                    *out = v;
                    SidereonStatus::Ok
                }
                Err(err) => extra_invalid_arg("sidereon_signal_coherent_loss_db", err),
            }
        },
    )
}

/// Post-correlation SNR in dB for a C/N0 and integration time. Delegates to
/// sidereon_core::signal::snr_post_db.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_signal_snr_post_db(
    cn0_dbhz: f64,
    integration_time_s: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_signal_snr_post_db", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_signal_snr_post_db", "out"));
        *out = 0.0;
        match sidereon_core::signal::snr_post_db(cn0_dbhz, integration_time_s) {
            Ok(v) => {
                *out = v;
                SidereonStatus::Ok
            }
            Err(err) => extra_invalid_arg("sidereon_signal_snr_post_db", err),
        }
    })
}

// --- GNSS signal correlation / acquisition (sidereon_core::signal) -----------

/// One complex baseband sample, mirroring sidereon_core::signal::IqSample.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonIqSample {
    /// In-phase component.
    pub i: f64,
    /// Quadrature component.
    pub q: f64,
}

/// Options for sidereon_signal_replica, mirroring
/// sidereon_core::signal::ReplicaOptions.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonReplicaOptions {
    /// Sampling rate in hertz.
    pub sample_rate_hz: f64,
    /// Number of output samples.
    pub num_samples: usize,
    /// Initial C/A code phase in chips.
    pub code_phase_chips: f64,
    /// Code-rate Doppler in hertz.
    pub code_doppler_hz: f64,
}

/// Options for sidereon_signal_correlate, mirroring
/// sidereon_core::signal::CorrelateOptions.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonCorrelateOptions {
    /// Sampling rate in hertz.
    pub sample_rate_hz: f64,
    /// Residual carrier Doppler to wipe off in hertz.
    pub doppler_hz: f64,
    /// Replica C/A code phase in chips.
    pub code_phase_chips: f64,
    /// Replica code-rate Doppler in hertz.
    pub code_doppler_hz: f64,
}

/// Coherent correlation result, mirroring
/// sidereon_core::signal::CorrelationResult.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonCorrelationResult {
    /// Real in-phase coherent sum.
    pub i: f64,
    /// Imaginary quadrature coherent sum.
    pub q: f64,
    /// Squared magnitude i*i + q*q.
    pub power: f64,
}

/// Options for sidereon_signal_acquire, mirroring
/// sidereon_core::signal::AcquisitionOptions.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonAcquisitionOptions {
    /// Sampling rate in hertz.
    pub sample_rate_hz: f64,
    /// Minimum Doppler bin in hertz.
    pub doppler_min_hz: f64,
    /// Maximum Doppler bin in hertz.
    pub doppler_max_hz: f64,
    /// Doppler bin step in hertz.
    pub doppler_step_hz: f64,
}

/// Acquisition result scalars, mirroring sidereon_core::signal::AcquisitionResult
/// (the searched Doppler-bin list is returned separately by
/// sidereon_signal_acquire).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonAcquisitionResult {
    /// Recovered code phase in chips.
    pub code_phase_chips: f64,
    /// Recovered Doppler bin in hertz.
    pub doppler_hz: f64,
    /// Peak-to-mean-off-peak metric.
    pub peak_metric: f64,
    /// Alias for peak_metric.
    pub metric: f64,
    /// Peak correlator power.
    pub peak_power: f64,
    /// Number of code-phase bins searched.
    pub grid_code_phase_bins: usize,
    /// Doppler step in hertz.
    pub grid_doppler_step_hz: f64,
    /// Samples per C/A chip at the configured sample rate.
    pub grid_samples_per_chip: f64,
    /// Number of Doppler bins searched (length of the separate bin output).
    pub grid_doppler_bin_count: usize,
}

/// The 1023 bipolar (+1/-1) GPS C/A chips for a PRN. Variable-length output
/// contract. Delegates to sidereon_core::signal::ca_code.
///
/// Safety: out points to len int8_t or NULL when len is 0; out_written and
/// out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_signal_ca_code(
    prn: i64,
    out: *mut i8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_signal_ca_code", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_signal_ca_code",
            out_written,
            out_required
        ));
        let code = match sidereon_core::signal::ca_code(prn) {
            Ok(c) => c,
            Err(err) => return extra_invalid_arg("sidereon_signal_ca_code", err),
        };
        c_try!(copy_prefix_to_c(
            "sidereon_signal_ca_code",
            "out",
            &code,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Build a sampled C/A-code replica. Variable-length output contract. Delegates
/// to sidereon_core::signal::replica.
///
/// Safety: options points to a SidereonReplicaOptions; out points to len int8_t
/// or NULL when len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_signal_replica(
    prn: i64,
    options: *const SidereonReplicaOptions,
    out: *mut i8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_signal_replica", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_signal_replica",
            out_written,
            out_required
        ));
        let options = c_try!(require_ref(options, "sidereon_signal_replica", "options"));
        let opts = sidereon_core::signal::ReplicaOptions {
            sample_rate_hz: options.sample_rate_hz,
            num_samples: options.num_samples,
            code_phase_chips: options.code_phase_chips,
            code_doppler_hz: options.code_doppler_hz,
        };
        let code = match sidereon_core::signal::replica(prn, opts) {
            Ok(c) => c,
            Err(err) => return extra_invalid_arg("sidereon_signal_replica", err),
        };
        c_try!(copy_prefix_to_c(
            "sidereon_signal_replica",
            "out",
            &code,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Coherently correlate a complex sample record against a PRN replica. Delegates
/// to sidereon_core::signal::correlate.
///
/// Safety: iq points to count SidereonIqSample; options to a
/// SidereonCorrelateOptions; out to a SidereonCorrelationResult.
#[no_mangle]
pub unsafe extern "C" fn sidereon_signal_correlate(
    iq: *const SidereonIqSample,
    count: usize,
    prn: i64,
    options: *const SidereonCorrelateOptions,
    out: *mut SidereonCorrelationResult,
) -> SidereonStatus {
    ffi_boundary("sidereon_signal_correlate", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_signal_correlate", "out"));
        *out = SidereonCorrelationResult {
            i: 0.0,
            q: 0.0,
            power: 0.0,
        };
        let options = c_try!(require_ref(options, "sidereon_signal_correlate", "options"));
        let samples = c_try!(iq_samples_from_c("sidereon_signal_correlate", iq, count));
        let opts = sidereon_core::signal::CorrelateOptions {
            sample_rate_hz: options.sample_rate_hz,
            doppler_hz: options.doppler_hz,
            code_phase_chips: options.code_phase_chips,
            code_doppler_hz: options.code_doppler_hz,
        };
        match sidereon_core::signal::correlate(&samples, prn, opts) {
            Ok(r) => {
                *out = SidereonCorrelationResult {
                    i: r.i,
                    q: r.q,
                    power: r.power,
                };
                SidereonStatus::Ok
            }
            Err(err) => extra_invalid_arg("sidereon_signal_correlate", err),
        }
    })
}

/// Coherent correlation against an explicit sampled code. Writes the in-phase
/// (out_i) and quadrature (out_q) coherent sums. Delegates to
/// sidereon_core::signal::correlate_against.
///
/// Safety: iq points to count SidereonIqSample; code points to code_len int8_t;
/// out_i and out_q point to a double each.
#[no_mangle]
pub unsafe extern "C" fn sidereon_signal_correlate_against(
    iq: *const SidereonIqSample,
    count: usize,
    code: *const i8,
    code_len: usize,
    fs: f64,
    doppler_hz: f64,
    out_i: *mut f64,
    out_q: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_signal_correlate_against",
        SidereonStatus::Panic,
        || {
            let out_i = c_try!(require_out(
                out_i,
                "sidereon_signal_correlate_against",
                "out_i"
            ));
            *out_i = 0.0;
            let out_q = c_try!(require_out(
                out_q,
                "sidereon_signal_correlate_against",
                "out_q"
            ));
            *out_q = 0.0;
            let samples = c_try!(iq_samples_from_c(
                "sidereon_signal_correlate_against",
                iq,
                count
            ));
            let code = c_try!(require_slice(
                code,
                code_len,
                "sidereon_signal_correlate_against",
                "code"
            ));
            match sidereon_core::signal::correlate_against(&samples, code, fs, doppler_hz) {
                Ok((i, q)) => {
                    *out_i = i;
                    *out_q = q;
                    SidereonStatus::Ok
                }
                Err(err) => extra_invalid_arg("sidereon_signal_correlate_against", err),
            }
        },
    )
}

/// Acquire a PRN by 2D code-phase/Doppler search. The scalar result is written
/// to out_result; the searched Doppler bins (hertz) follow the variable-length
/// output contract via out_doppler_hz. Delegates to
/// sidereon_core::signal::acquire.
///
/// Safety: samples points to count SidereonIqSample; options to a
/// SidereonAcquisitionOptions; out_result to a SidereonAcquisitionResult;
/// out_doppler_hz to len doubles or NULL when len is 0; out_written and
/// out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_signal_acquire(
    samples: *const SidereonIqSample,
    count: usize,
    prn: i64,
    options: *const SidereonAcquisitionOptions,
    out_result: *mut SidereonAcquisitionResult,
    out_doppler_hz: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_signal_acquire", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_signal_acquire",
            out_written,
            out_required
        ));
        let out_result = c_try!(require_out(
            out_result,
            "sidereon_signal_acquire",
            "out_result"
        ));
        *out_result = SidereonAcquisitionResult {
            code_phase_chips: 0.0,
            doppler_hz: 0.0,
            peak_metric: 0.0,
            metric: 0.0,
            peak_power: 0.0,
            grid_code_phase_bins: 0,
            grid_doppler_step_hz: 0.0,
            grid_samples_per_chip: 0.0,
            grid_doppler_bin_count: 0,
        };
        let options = c_try!(require_ref(options, "sidereon_signal_acquire", "options"));
        let iq = c_try!(iq_samples_from_c("sidereon_signal_acquire", samples, count));
        let opts = sidereon_core::signal::AcquisitionOptions {
            sample_rate_hz: options.sample_rate_hz,
            doppler_min_hz: options.doppler_min_hz,
            doppler_max_hz: options.doppler_max_hz,
            doppler_step_hz: options.doppler_step_hz,
        };
        let result = match sidereon_core::signal::acquire(&iq, prn, opts) {
            Ok(r) => r,
            Err(err) => return extra_invalid_arg("sidereon_signal_acquire", err),
        };
        *out_result = SidereonAcquisitionResult {
            code_phase_chips: result.code_phase_chips,
            doppler_hz: result.doppler_hz,
            peak_metric: result.peak_metric,
            metric: result.metric,
            peak_power: result.peak_power,
            grid_code_phase_bins: result.grid.code_phase_bins,
            grid_doppler_step_hz: result.grid.doppler_step_hz,
            grid_samples_per_chip: result.grid.samples_per_chip,
            grid_doppler_bin_count: result.grid.doppler_hz.len(),
        };
        c_try!(copy_prefix_to_c(
            "sidereon_signal_acquire",
            "out_doppler_hz",
            &result.grid.doppler_hz,
            out_doppler_hz,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Single-lag circular correlation between two equal-length codes. Delegates to
/// sidereon_core::signal::correlation_at.
///
/// Safety: code_a and code_b point to count int8_t each; out points to an
/// int32_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_signal_correlation_at(
    code_a: *const i8,
    code_b: *const i8,
    count: usize,
    lag: i64,
    out: *mut i32,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_signal_correlation_at",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_signal_correlation_at", "out"));
            *out = 0;
            let code_a = c_try!(require_slice(
                code_a,
                count,
                "sidereon_signal_correlation_at",
                "code_a"
            ));
            let code_b = c_try!(require_slice(
                code_b,
                count,
                "sidereon_signal_correlation_at",
                "code_b"
            ));
            match sidereon_core::signal::correlation_at(code_a, code_b, lag) {
                Ok(v) => {
                    *out = v;
                    SidereonStatus::Ok
                }
                Err(err) => extra_invalid_arg("sidereon_signal_correlation_at", err),
            }
        },
    )
}

/// Circular cross-correlation over all lags between two equal-length codes.
/// Variable-length output contract. Delegates to
/// sidereon_core::signal::cross_correlation.
///
/// Safety: code_a and code_b point to count int8_t each; out points to len
/// int32_t or NULL when len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_signal_cross_correlation(
    code_a: *const i8,
    code_b: *const i8,
    count: usize,
    out: *mut i32,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_signal_cross_correlation",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_signal_cross_correlation",
                out_written,
                out_required
            ));
            let code_a = c_try!(require_slice(
                code_a,
                count,
                "sidereon_signal_cross_correlation",
                "code_a"
            ));
            let code_b = c_try!(require_slice(
                code_b,
                count,
                "sidereon_signal_cross_correlation",
                "code_b"
            ));
            let corr = match sidereon_core::signal::cross_correlation(code_a, code_b) {
                Ok(c) => c,
                Err(err) => return extra_invalid_arg("sidereon_signal_cross_correlation", err),
            };
            c_try!(copy_prefix_to_c(
                "sidereon_signal_cross_correlation",
                "out",
                &corr,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Circular autocorrelation over all lags of a code (infallible).
/// Variable-length output contract. Delegates to
/// sidereon_core::signal::autocorrelation.
///
/// Safety: code points to count int8_t; out points to len int32_t or NULL when
/// len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_signal_autocorrelation(
    code: *const i8,
    count: usize,
    out: *mut i32,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_signal_autocorrelation",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_signal_autocorrelation",
                out_written,
                out_required
            ));
            let code = c_try!(require_slice(
                code,
                count,
                "sidereon_signal_autocorrelation",
                "code"
            ));
            let corr = sidereon_core::signal::autocorrelation(code);
            c_try!(copy_prefix_to_c(
                "sidereon_signal_autocorrelation",
                "out",
                &corr,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

// --- Signal analysis (sidereon_core::signal::analysis) ----------------------

/// Navigation modulation family for signal-analysis metrics.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSignalAnalysisModulationKind {
    /// BPSK(n), with code rate n * 1.023 MHz.
    Bpsk = 0,
    /// Sine-phased BOC(m,n).
    BocSine = 1,
    /// Cosine-phased BOC(m,n).
    BocCosine = 2,
    /// MBOC(6,1,1/11), the weighted BOC(1,1) and BOC(6,1) spectrum.
    Mboc611Over11 = 3,
    /// GPS L1C pilot TMBOC(6,1,4/33) spectrum.
    Tmboc614Over33 = 4,
    /// Galileo E1 CBOC(6,1,1/11) with positive high-rate component.
    Cboc611Over11Plus = 5,
    /// Galileo E1 CBOC(6,1,1/11) with negative high-rate component.
    Cboc611Over11Minus = 6,
}

/// Navigation modulation descriptor for signal-analysis metrics.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSignalAnalysisModulation {
    /// One of SidereonSignalAnalysisModulationKind_*.
    pub kind: u32,
    /// BPSK order n for BPSK(n); ignored by BOC and fixed composite variants.
    pub order: f64,
    /// BOC subcarrier multiplier m for BOC(m,n); ignored by fixed composite variants.
    pub m: f64,
    /// BOC code-rate multiplier n for BOC(m,n); ignored by fixed composite variants.
    pub n: f64,
}

/// Interference term for effective C/N0 degradation.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSignalAnalysisInterference {
    /// Interference modulation.
    pub modulation: SidereonSignalAnalysisModulation,
    /// Interference received power divided by desired-signal received power.
    pub power_ratio_to_carrier: f64,
}

/// Spectral separation coefficient result.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSignalAnalysisSpectralSeparation {
    /// Linear SSC value in hertz.
    pub hz: f64,
    /// SSC value in decibel-hertz.
    pub db_hz: f64,
}

/// Effective C/N0 degradation result.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSignalAnalysisCn0Degradation {
    /// Effective carrier-to-noise-density ratio in hertz.
    pub effective_cn0_hz: f64,
    /// Effective carrier-to-noise-density ratio in decibel-hertz.
    pub effective_cn0_db_hz: f64,
    /// Loss from the input C/N0 to the effective C/N0, in decibels.
    pub degradation_db: f64,
}

/// DLL thermal-noise processing selector.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSignalAnalysisDllProcessing {
    /// Coherent early-minus-late processing.
    Coherent = 0,
    /// Non-coherent early-minus-late power processing.
    NonCoherent = 1,
}

/// Inputs for DLL tracking thermal-noise metrics.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSignalAnalysisDllTrackingOptions {
    /// Carrier-to-noise-density ratio in decibel-hertz.
    pub cn0_db_hz: f64,
    /// One-sided DLL loop bandwidth in hertz.
    pub loop_bandwidth_hz: f64,
    /// Predetection coherent integration time in seconds.
    pub integration_time_s: f64,
    /// Early-late correlator spacing in code chips.
    pub correlator_spacing_chips: f64,
    /// Two-sided receiver bandwidth in hertz.
    pub receiver_bandwidth_hz: f64,
}

/// DLL thermal-noise jitter result.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSignalAnalysisDllJitter {
    /// One-sigma delay jitter in seconds.
    pub seconds: f64,
    /// One-sigma delay jitter in code chips.
    pub chips: f64,
    /// One-sigma range jitter in meters.
    pub meters: f64,
    /// Non-coherent squaring-loss multiplier.
    pub squaring_loss: f64,
}

/// Inputs for one-path specular multipath envelope metrics.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSignalAnalysisMultipathOptions {
    /// Reflected-path amplitude divided by direct-path amplitude.
    pub multipath_to_direct_ratio: f64,
    /// Early-late correlator spacing in code chips.
    pub correlator_spacing_chips: f64,
    /// Two-sided receiver bandwidth in hertz.
    pub receiver_bandwidth_hz: f64,
}

/// Multipath envelope value at one reflected-path delay.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSignalAnalysisMultipathEnvelopePoint {
    /// Reflected-path delay in code chips.
    pub delay_chips: f64,
    /// Reflected-path delay in seconds.
    pub delay_s: f64,
    /// In-phase tracking error in code chips.
    pub in_phase_chips: f64,
    /// In-phase tracking error in seconds.
    pub in_phase_s: f64,
    /// In-phase tracking error in meters.
    pub in_phase_m: f64,
    /// Anti-phase tracking error in code chips.
    pub anti_phase_chips: f64,
    /// Anti-phase tracking error in seconds.
    pub anti_phase_s: f64,
    /// Anti-phase tracking error in meters.
    pub anti_phase_m: f64,
    /// Running average of the absolute envelope in code chips.
    pub running_average_chips: f64,
    /// Running average of the absolute envelope in seconds.
    pub running_average_s: f64,
    /// Running average of the absolute envelope in meters.
    pub running_average_m: f64,
}

/// Evaluate a normalized modulation PSD at an offset frequency.
///
/// Safety: modulation and out must point to readable and writable objects.
#[no_mangle]
pub unsafe extern "C" fn sidereon_signal_analysis_psd(
    modulation: *const SidereonSignalAnalysisModulation,
    offset_hz: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_signal_analysis_psd",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_signal_analysis_psd", "out"));
            *out = 0.0;
            let modulation = c_try!(signal_analysis_modulation_from_c(
                "sidereon_signal_analysis_psd",
                modulation
            ));
            match modulation.psd_hz(offset_hz) {
                Ok(value) => {
                    *out = value;
                    SidereonStatus::Ok
                }
                Err(err) => extra_invalid_arg("sidereon_signal_analysis_psd", err),
            }
        },
    )
}

/// Compute the fraction of modulation power inside a two-sided bandwidth.
///
/// Safety: modulation and out must point to readable and writable objects.
#[no_mangle]
pub unsafe extern "C" fn sidereon_signal_analysis_fraction_power(
    modulation: *const SidereonSignalAnalysisModulation,
    receiver_bandwidth_hz: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_signal_analysis_fraction_power",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_signal_analysis_fraction_power",
                "out"
            ));
            *out = 0.0;
            let modulation = c_try!(signal_analysis_modulation_from_c(
                "sidereon_signal_analysis_fraction_power",
                modulation
            ));
            match sidereon_core::signal::analysis::fraction_power_in_band(
                &modulation,
                receiver_bandwidth_hz,
            ) {
                Ok(value) => {
                    *out = value;
                    SidereonStatus::Ok
                }
                Err(err) => extra_invalid_arg("sidereon_signal_analysis_fraction_power", err),
            }
        },
    )
}

/// Compute the RMS, or Gabor, bandwidth over a two-sided receiver bandwidth.
///
/// Safety: modulation and out must point to readable and writable objects.
#[no_mangle]
pub unsafe extern "C" fn sidereon_signal_analysis_rms_bandwidth_hz(
    modulation: *const SidereonSignalAnalysisModulation,
    receiver_bandwidth_hz: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_signal_analysis_rms_bandwidth_hz",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_signal_analysis_rms_bandwidth_hz",
                "out"
            ));
            *out = 0.0;
            let modulation = c_try!(signal_analysis_modulation_from_c(
                "sidereon_signal_analysis_rms_bandwidth_hz",
                modulation
            ));
            match sidereon_core::signal::analysis::rms_bandwidth_hz(
                &modulation,
                receiver_bandwidth_hz,
            ) {
                Ok(value) => {
                    *out = value;
                    SidereonStatus::Ok
                }
                Err(err) => extra_invalid_arg("sidereon_signal_analysis_rms_bandwidth_hz", err),
            }
        },
    )
}

/// Compute the spectral separation coefficient between two modulations.
///
/// Safety: desired, interference, and out must point to readable and writable
/// objects.
#[no_mangle]
pub unsafe extern "C" fn sidereon_signal_analysis_spectral_separation(
    desired: *const SidereonSignalAnalysisModulation,
    interference: *const SidereonSignalAnalysisModulation,
    receiver_bandwidth_hz: f64,
    out: *mut SidereonSignalAnalysisSpectralSeparation,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_signal_analysis_spectral_separation",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_signal_analysis_spectral_separation",
                "out"
            ));
            *out = SidereonSignalAnalysisSpectralSeparation {
                hz: 0.0,
                db_hz: 0.0,
            };
            let desired = c_try!(signal_analysis_modulation_from_c(
                "sidereon_signal_analysis_spectral_separation",
                desired
            ));
            let interference = c_try!(signal_analysis_modulation_from_c(
                "sidereon_signal_analysis_spectral_separation",
                interference
            ));
            let hz = match sidereon_core::signal::analysis::spectral_separation_coefficient_hz(
                &desired,
                &interference,
                receiver_bandwidth_hz,
            ) {
                Ok(value) => value,
                Err(err) => {
                    return extra_invalid_arg("sidereon_signal_analysis_spectral_separation", err)
                }
            };
            let db_hz = match sidereon_core::signal::analysis::spectral_separation_coefficient_db_hz(
                &desired,
                &interference,
                receiver_bandwidth_hz,
            ) {
                Ok(value) => value,
                Err(err) => {
                    return extra_invalid_arg("sidereon_signal_analysis_spectral_separation", err)
                }
            };
            *out = SidereonSignalAnalysisSpectralSeparation { hz, db_hz };
            SidereonStatus::Ok
        },
    )
}

/// Compute effective C/N0 degradation from finite-band interference terms.
///
/// Safety: desired and out must point to readable and writable objects;
/// interferences must point to interference_count rows or NULL when the count is
/// zero.
#[no_mangle]
pub unsafe extern "C" fn sidereon_signal_analysis_effective_cn0_degradation(
    desired: *const SidereonSignalAnalysisModulation,
    cn0_db_hz: f64,
    receiver_bandwidth_hz: f64,
    interferences: *const SidereonSignalAnalysisInterference,
    interference_count: usize,
    out: *mut SidereonSignalAnalysisCn0Degradation,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_signal_analysis_effective_cn0_degradation",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_signal_analysis_effective_cn0_degradation",
                "out"
            ));
            *out = SidereonSignalAnalysisCn0Degradation {
                effective_cn0_hz: 0.0,
                effective_cn0_db_hz: 0.0,
                degradation_db: 0.0,
            };
            let desired = c_try!(signal_analysis_modulation_from_c(
                "sidereon_signal_analysis_effective_cn0_degradation",
                desired
            ));
            let interferences = c_try!(signal_analysis_interferences_from_c(
                "sidereon_signal_analysis_effective_cn0_degradation",
                interferences,
                interference_count
            ));
            match sidereon_core::signal::analysis::effective_cn0_degradation(
                &desired,
                cn0_db_hz,
                receiver_bandwidth_hz,
                &interferences,
            ) {
                Ok(value) => {
                    *out = SidereonSignalAnalysisCn0Degradation {
                        effective_cn0_hz: value.effective_cn0_hz,
                        effective_cn0_db_hz: value.effective_cn0_db_hz,
                        degradation_db: value.degradation_db,
                    };
                    SidereonStatus::Ok
                }
                Err(err) => {
                    extra_invalid_arg("sidereon_signal_analysis_effective_cn0_degradation", err)
                }
            }
        },
    )
}

/// Compute early-late DLL thermal-noise jitter for a modulation.
///
/// Safety: modulation, options, and out must point to readable and writable
/// objects.
#[no_mangle]
pub unsafe extern "C" fn sidereon_signal_analysis_dll_jitter(
    modulation: *const SidereonSignalAnalysisModulation,
    options: *const SidereonSignalAnalysisDllTrackingOptions,
    processing: u32,
    out: *mut SidereonSignalAnalysisDllJitter,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_signal_analysis_dll_jitter",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_signal_analysis_dll_jitter",
                "out"
            ));
            *out = zero_signal_jitter();
            let modulation = c_try!(signal_analysis_modulation_from_c(
                "sidereon_signal_analysis_dll_jitter",
                modulation
            ));
            let options = c_try!(signal_analysis_dll_options_from_c(
                "sidereon_signal_analysis_dll_jitter",
                options
            ));
            let processing = c_try!(signal_analysis_processing_from_c(
                "sidereon_signal_analysis_dll_jitter",
                processing
            ));
            match sidereon_core::signal::analysis::dll_thermal_noise_jitter(
                &modulation,
                options,
                processing,
            ) {
                Ok(value) => {
                    *out = signal_jitter_to_c(value);
                    SidereonStatus::Ok
                }
                Err(err) => extra_invalid_arg("sidereon_signal_analysis_dll_jitter", err),
            }
        },
    )
}

/// Compute the lower bound for code-delay tracking jitter.
///
/// Safety: modulation, options, and out must point to readable and writable
/// objects.
#[no_mangle]
pub unsafe extern "C" fn sidereon_signal_analysis_dll_lower_bound(
    modulation: *const SidereonSignalAnalysisModulation,
    options: *const SidereonSignalAnalysisDllTrackingOptions,
    out: *mut SidereonSignalAnalysisDllJitter,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_signal_analysis_dll_lower_bound",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_signal_analysis_dll_lower_bound",
                "out"
            ));
            *out = zero_signal_jitter();
            let modulation = c_try!(signal_analysis_modulation_from_c(
                "sidereon_signal_analysis_dll_lower_bound",
                modulation
            ));
            let options = c_try!(signal_analysis_dll_options_from_c(
                "sidereon_signal_analysis_dll_lower_bound",
                options
            ));
            match sidereon_core::signal::analysis::dll_lower_bound(&modulation, options) {
                Ok(value) => {
                    *out = signal_jitter_to_c(value);
                    SidereonStatus::Ok
                }
                Err(err) => extra_invalid_arg("sidereon_signal_analysis_dll_lower_bound", err),
            }
        },
    )
}

/// Compute one-path early-late multipath envelopes on a delay grid.
///
/// Safety: modulation and options must point to readable objects; delay_chips
/// must point to delay_count doubles or NULL when count is zero; out must point
/// to len rows or NULL when len is 0; out_written and out_required must point to
/// size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_signal_analysis_multipath_envelope(
    modulation: *const SidereonSignalAnalysisModulation,
    options: *const SidereonSignalAnalysisMultipathOptions,
    delay_chips: *const f64,
    delay_count: usize,
    out: *mut SidereonSignalAnalysisMultipathEnvelopePoint,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_signal_analysis_multipath_envelope",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_signal_analysis_multipath_envelope",
                out_written,
                out_required
            ));
            let modulation = c_try!(signal_analysis_modulation_from_c(
                "sidereon_signal_analysis_multipath_envelope",
                modulation
            ));
            let options = c_try!(signal_analysis_multipath_options_from_c(
                "sidereon_signal_analysis_multipath_envelope",
                options
            ));
            let delays = c_try!(require_slice(
                delay_chips,
                delay_count,
                "sidereon_signal_analysis_multipath_envelope",
                "delay_chips"
            ));
            let points = match sidereon_core::signal::analysis::multipath_error_envelope(
                &modulation,
                options,
                delays,
            ) {
                Ok(points) => points,
                Err(err) => {
                    return extra_invalid_arg("sidereon_signal_analysis_multipath_envelope", err)
                }
            };
            let rows: Vec<_> = points.into_iter().map(multipath_point_to_c).collect();
            c_try!(copy_prefix_to_c(
                "sidereon_signal_analysis_multipath_envelope",
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

fn carrier_band_from_c(
    fn_name: &str,
    arg_name: &str,
    band: u32,
) -> Result<sidereon_core::frequencies::CarrierBand, SidereonStatus> {
    use sidereon_core::frequencies::CarrierBand as B;
    let mapped = match band {
        v if v == SidereonCarrierBand::L1 as u32 => B::L1,
        v if v == SidereonCarrierBand::L2 as u32 => B::L2,
        v if v == SidereonCarrierBand::L5 as u32 => B::L5,
        v if v == SidereonCarrierBand::E1 as u32 => B::E1,
        v if v == SidereonCarrierBand::E5a as u32 => B::E5a,
        v if v == SidereonCarrierBand::E5b as u32 => B::E5b,
        v if v == SidereonCarrierBand::E5 as u32 => B::E5,
        v if v == SidereonCarrierBand::E6 as u32 => B::E6,
        v if v == SidereonCarrierBand::B1c as u32 => B::B1c,
        v if v == SidereonCarrierBand::B1i as u32 => B::B1i,
        v if v == SidereonCarrierBand::B2a as u32 => B::B2a,
        v if v == SidereonCarrierBand::B2b as u32 => B::B2b,
        v if v == SidereonCarrierBand::B2 as u32 => B::B2,
        v if v == SidereonCarrierBand::B3i as u32 => B::B3i,
        v if v == SidereonCarrierBand::G1 as u32 => B::G1,
        v if v == SidereonCarrierBand::G2 as u32 => B::G2,
        _ => {
            set_last_error(format!("{fn_name}: invalid {arg_name} carrier band"));
            return Err(SidereonStatus::InvalidArgument);
        }
    };
    Ok(mapped)
}

unsafe fn iq_samples_from_c(
    fn_name: &str,
    iq: *const SidereonIqSample,
    count: usize,
) -> Result<Vec<sidereon_core::signal::IqSample>, SidereonStatus> {
    let rows = require_slice(iq, count, fn_name, "iq")?;
    Ok(rows
        .iter()
        .map(|s| sidereon_core::signal::IqSample::new(s.i, s.q))
        .collect())
}

unsafe fn signal_analysis_modulation_from_c(
    fn_name: &str,
    modulation: *const SidereonSignalAnalysisModulation,
) -> Result<sidereon_core::signal::analysis::SignalModulation, SidereonStatus> {
    let raw = require_ref(modulation, fn_name, "modulation")?;
    match raw.kind {
        value if value == SidereonSignalAnalysisModulationKind::Bpsk as u32 => {
            sidereon_core::signal::analysis::SignalModulation::bpsk(raw.order).map_err(|err| {
                set_last_error(format!("{fn_name}: {err}"));
                SidereonStatus::InvalidArgument
            })
        }
        value if value == SidereonSignalAnalysisModulationKind::BocSine as u32 => {
            sidereon_core::signal::analysis::SignalModulation::boc_sine(raw.m, raw.n).map_err(
                |err| {
                    set_last_error(format!("{fn_name}: {err}"));
                    SidereonStatus::InvalidArgument
                },
            )
        }
        value if value == SidereonSignalAnalysisModulationKind::BocCosine as u32 => {
            sidereon_core::signal::analysis::SignalModulation::boc_cosine(raw.m, raw.n).map_err(
                |err| {
                    set_last_error(format!("{fn_name}: {err}"));
                    SidereonStatus::InvalidArgument
                },
            )
        }
        value if value == SidereonSignalAnalysisModulationKind::Mboc611Over11 as u32 => {
            Ok(sidereon_core::signal::analysis::SignalModulation::mboc_6_1_1_over_11())
        }
        value if value == SidereonSignalAnalysisModulationKind::Tmboc614Over33 as u32 => {
            Ok(sidereon_core::signal::analysis::SignalModulation::tmboc_6_1_4_over_33())
        }
        value if value == SidereonSignalAnalysisModulationKind::Cboc611Over11Plus as u32 => Ok(
            sidereon_core::signal::analysis::SignalModulation::cboc_6_1_1_over_11(
                sidereon_core::signal::analysis::CbocSign::Plus,
            ),
        ),
        value if value == SidereonSignalAnalysisModulationKind::Cboc611Over11Minus as u32 => Ok(
            sidereon_core::signal::analysis::SignalModulation::cboc_6_1_1_over_11(
                sidereon_core::signal::analysis::CbocSign::Minus,
            ),
        ),
        _ => {
            set_last_error(format!(
                "{fn_name}: invalid signal-analysis modulation kind"
            ));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

unsafe fn signal_analysis_interferences_from_c(
    fn_name: &str,
    interferences: *const SidereonSignalAnalysisInterference,
    count: usize,
) -> Result<Vec<sidereon_core::signal::analysis::InterferenceTerm>, SidereonStatus> {
    let rows = require_slice(interferences, count, fn_name, "interferences")?;
    rows.iter()
        .map(|row| {
            let modulation = signal_analysis_modulation_from_c(fn_name, &row.modulation)?;
            Ok(sidereon_core::signal::analysis::InterferenceTerm::new(
                modulation,
                row.power_ratio_to_carrier,
            ))
        })
        .collect()
}

unsafe fn signal_analysis_dll_options_from_c(
    fn_name: &str,
    options: *const SidereonSignalAnalysisDllTrackingOptions,
) -> Result<sidereon_core::signal::analysis::DllTrackingOptions, SidereonStatus> {
    let options = require_ref(options, fn_name, "options")?;
    Ok(sidereon_core::signal::analysis::DllTrackingOptions {
        cn0_db_hz: options.cn0_db_hz,
        loop_bandwidth_hz: options.loop_bandwidth_hz,
        integration_time_s: options.integration_time_s,
        correlator_spacing_chips: options.correlator_spacing_chips,
        receiver_bandwidth_hz: options.receiver_bandwidth_hz,
    })
}

fn signal_analysis_processing_from_c(
    fn_name: &str,
    processing: u32,
) -> Result<sidereon_core::signal::analysis::DllProcessing, SidereonStatus> {
    match processing {
        value if value == SidereonSignalAnalysisDllProcessing::Coherent as u32 => {
            Ok(sidereon_core::signal::analysis::DllProcessing::Coherent)
        }
        value if value == SidereonSignalAnalysisDllProcessing::NonCoherent as u32 => {
            Ok(sidereon_core::signal::analysis::DllProcessing::NonCoherent)
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid DLL processing selector"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

unsafe fn signal_analysis_multipath_options_from_c(
    fn_name: &str,
    options: *const SidereonSignalAnalysisMultipathOptions,
) -> Result<sidereon_core::signal::analysis::MultipathOptions, SidereonStatus> {
    let options = require_ref(options, fn_name, "options")?;
    Ok(sidereon_core::signal::analysis::MultipathOptions {
        multipath_to_direct_ratio: options.multipath_to_direct_ratio,
        correlator_spacing_chips: options.correlator_spacing_chips,
        receiver_bandwidth_hz: options.receiver_bandwidth_hz,
    })
}

fn signal_jitter_to_c(
    value: sidereon_core::signal::analysis::DllJitter,
) -> SidereonSignalAnalysisDllJitter {
    SidereonSignalAnalysisDllJitter {
        seconds: value.seconds,
        chips: value.chips,
        meters: value.meters,
        squaring_loss: value.squaring_loss,
    }
}

fn zero_signal_jitter() -> SidereonSignalAnalysisDllJitter {
    SidereonSignalAnalysisDllJitter {
        seconds: 0.0,
        chips: 0.0,
        meters: 0.0,
        squaring_loss: 0.0,
    }
}

fn multipath_point_to_c(
    point: sidereon_core::signal::analysis::MultipathEnvelopePoint,
) -> SidereonSignalAnalysisMultipathEnvelopePoint {
    SidereonSignalAnalysisMultipathEnvelopePoint {
        delay_chips: point.delay_chips,
        delay_s: point.delay_s,
        in_phase_chips: point.in_phase_chips,
        in_phase_s: point.in_phase_s,
        in_phase_m: point.in_phase_m,
        anti_phase_chips: point.anti_phase_chips,
        anti_phase_s: point.anti_phase_s,
        anti_phase_m: point.anti_phase_m,
        running_average_chips: point.running_average_chips,
        running_average_s: point.running_average_s,
        running_average_m: point.running_average_m,
    }
}
