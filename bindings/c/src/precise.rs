use super::*;

/// A sample-backed precise-ephemeris source, built from canonical
/// position/clock samples rather than parsed SP3 text. Opaque to C. Create with
/// sidereon_precise_ephemeris_samples_from_samples and release with
/// sidereon_precise_ephemeris_samples_free. Interpolates and predicts ranges
/// through the same substrate as a loaded SP3 product.
pub struct SidereonPreciseEphemerisSamples {
    pub(crate) inner: PreciseEphemerisSamples,
}

/// A build-once precise-ephemeris interpolant with cached per-satellite nodes.
/// Opaque to C. Create with sidereon_precise_ephemeris_interpolant_from_sp3,
/// sidereon_precise_ephemeris_interpolant_from_samples, or
/// sidereon_precise_ephemeris_interpolant_from_precise_ephemeris_samples and
/// release with sidereon_precise_ephemeris_interpolant_free.
pub struct SidereonPreciseEphemerisInterpolant {
    pub(crate) inner: PreciseEphemerisInterpolant,
}

// --- Batch forward-observable prediction ------------------------------------

/// One batch observable-prediction request: the satellite token, the static
/// receiver ECEF position (meters), and the receive epoch (seconds since J2000).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPredictRequest {
    /// Null-terminated satellite token (e.g. "G01").
    pub sat_id: *const c_char,
    /// Receiver ECEF position, meters.
    pub receiver_ecef_m: [f64; 3],
    /// Receive epoch, seconds since J2000.
    pub t_rx_j2000_s: f64,
}

// --- Precise-ephemeris samples + batch range prediction ---------------------

/// One canonical precise-ephemeris sample: a satellite's ECEF position (and
/// optional clock) at one epoch, in SI units. This is the serialization-
/// independent element behind an SP3 record; sidereon_sp3_precise_ephemeris_samples
/// extracts them and sidereon_precise_ephemeris_samples_from_samples rebuilds an
/// interpolatable source from them.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPreciseEphemerisSample {
    /// Satellite this sample describes, as a null-terminated token (e.g. G01).
    pub sat: SidereonSatelliteToken,
    /// Time scale the epoch is expressed in (a SidereonTimeScale code as
    /// uint32_t). Every sample in one source must share this scale.
    pub time_scale: u32,
    /// Sample epoch, seconds since J2000 in the sample's time scale.
    pub epoch_j2000_s: f64,
    /// Satellite ECEF position in the ITRF/IGS frame, meters.
    pub position_ecef_m: [f64; 3],
    /// Whether clock_s carries a satellite clock estimate.
    pub has_clock_s: bool,
    /// Satellite clock offset, seconds, when has_clock_s is true.
    pub clock_s: f64,
    /// Whether this epoch carries the SP3 E clock-event flag: true splits the
    /// clock interpolation arc here (a clock reset takes effect at this epoch).
    pub clock_event: bool,
}

/// One batch range-prediction request: the satellite token, the static receiver
/// ECEF position (meters), and the receive epoch (seconds since J2000).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRangePredictionRequest {
    /// Null-terminated satellite token (e.g. "G01").
    pub sat_id: *const c_char,
    /// Receiver ECEF position, meters.
    pub receiver_ecef_m: [f64; 3],
    /// Receive epoch, seconds since J2000.
    pub t_rx_j2000_s: f64,
}

/// The geometry-only result of one range-prediction request: the transmit-time
/// geometry a range-only consumer needs, without Doppler or topocentric fields.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRangePrediction {
    /// Geometric range after optional Sagnac transport, meters.
    pub geometric_range_m: f64,
    /// Whether sat_clock_s is present.
    pub has_sat_clock_s: bool,
    /// Satellite clock offset at transmit time, seconds, when present.
    pub sat_clock_s: f64,
    /// Transmit time, seconds since J2000.
    pub transmit_time_j2000_s: f64,
    /// Sagnac-transported satellite ECEF position, meters.
    pub sat_pos_ecef_m: [f64; 3],
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonEphemerisSampleStatus {
    Valid = 0,
    Gap = 1,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonEphemerisSampleRow {
    pub sat_id: SidereonSatelliteToken,
    pub epoch_j2000_s: f64,
    pub status: SidereonEphemerisSampleStatus,
    pub has_position_ecef_m: bool,
    pub position_ecef_m: [f64; 3],
    pub has_clock_s: bool,
    pub clock_s: f64,
}

/// Build a sample-backed precise-ephemeris source from count canonical samples.
/// On success writes a newly owned handle to *out_handle; release it with
/// sidereon_precise_ephemeris_samples_free. Validation failures (no samples, a
/// single-sample satellite, non-monotonic epochs, mixed time scales, a
/// non-representable epoch, or a non-finite value) return
/// SIDEREON_STATUS_INVALID_ARGUMENT.
///
/// Safety: samples must point to count entries (each with a valid sat token) or
/// be NULL when count is 0; out_handle must point to storage for a
/// SidereonPreciseEphemerisSamples*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_samples_from_samples(
    samples: *const SidereonPreciseEphemerisSample,
    count: usize,
    out_handle: *mut *mut SidereonPreciseEphemerisSamples,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_precise_ephemeris_samples_from_samples",
        SidereonStatus::Panic,
        || {
            let out_handle = c_try!(require_out(
                out_handle,
                "sidereon_precise_ephemeris_samples_from_samples",
                "out_handle"
            ));
            *out_handle = ptr::null_mut();
            let raw = c_try!(require_slice(
                samples,
                count,
                "sidereon_precise_ephemeris_samples_from_samples",
                "samples"
            ));
            let mut parsed = Vec::with_capacity(raw.len());
            for sample in raw {
                parsed.push(c_try!(precise_sample_from_c(
                    "sidereon_precise_ephemeris_samples_from_samples",
                    sample
                )));
            }
            let inner = match PreciseEphemerisSamples::from_samples(parsed) {
                Ok(inner) => inner,
                Err(err) => {
                    return map_precise_samples_error(
                        "sidereon_precise_ephemeris_samples_from_samples",
                        err,
                    )
                }
            };
            write_boxed_handle(out_handle, SidereonPreciseEphemerisSamples { inner });
            SidereonStatus::Ok
        },
    )
}

/// Release a precise-ephemeris samples handle. Null is a no-op. A non-null handle
/// must come from sidereon_precise_ephemeris_samples_from_samples and must be
/// freed exactly once with this function.
///
/// Safety: samples must be NULL or a live handle from
/// sidereon_precise_ephemeris_samples_from_samples. Passing a handle after it has
/// already been freed is invalid.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_samples_free(
    samples: *mut SidereonPreciseEphemerisSamples,
) {
    ffi_boundary("sidereon_precise_ephemeris_samples_free", (), || {
        free_boxed(samples);
    });
}

/// Sample a sample-backed precise-ephemeris source over a regular grid.
///
/// Safety: samples must be a live handle; satellites points to satellite_count
/// null-terminated tokens; out points to len SidereonEphemerisSampleRow or NULL
/// when len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_samples_sample(
    samples: *const SidereonPreciseEphemerisSamples,
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
        "sidereon_precise_ephemeris_samples_sample",
        SidereonStatus::Panic,
        || {
            let samples = c_try!(require_ref(
                samples,
                "sidereon_precise_ephemeris_samples_sample",
                "samples"
            ));
            ephemeris_sample_common(
                "sidereon_precise_ephemeris_samples_sample",
                &samples.inner,
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

/// Predict geometric ranges for many (satellite, receiver, epoch) requests from a
/// sample-backed precise-ephemeris source in one call. Mirror of
/// sidereon_sp3_predict_ranges for the samples source; same per-request out
/// contract. Delegates to sidereon_core::observables::predict_ranges.
///
/// Safety: samples must be a live handle from
/// sidereon_precise_ephemeris_samples_from_samples; requests must point to count
/// entries (each with a valid sat_id); out must point to count writable entries
/// (or be NULL when count is 0); options must be NULL or point to a
/// SidereonObservablesOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_samples_predict_ranges(
    samples: *const SidereonPreciseEphemerisSamples,
    requests: *const SidereonRangePredictionRequest,
    count: usize,
    options: *const SidereonObservablesOptions,
    out: *mut SidereonRangePrediction,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_precise_ephemeris_samples_predict_ranges",
        SidereonStatus::Panic,
        || {
            let samples = c_try!(require_ref(
                samples,
                "sidereon_precise_ephemeris_samples_predict_ranges",
                "samples"
            ));
            predict_ranges_into(
                "sidereon_precise_ephemeris_samples_predict_ranges",
                &samples.inner,
                requests,
                count,
                options,
                out,
            )
        },
    )
}

// --- 0.13 batched observable states and cached interpolants -----------------

/// Per-element state category for a batched observable-state query.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonObservableStateElementStatus {
    /// Position and clock fields hold a usable state.
    Valid = 0,
    /// The source had no usable state for this satellite and epoch.
    Gap = 1,
    /// The scalar evaluator returned a non-gap error.
    Error = 2,
}

/// Build a cached precise-ephemeris interpolant from a loaded SP3 handle. On
/// success writes a newly owned handle to *out_handle; release it with
/// sidereon_precise_ephemeris_interpolant_free.
///
/// Safety: sp3 must be a live handle; out_handle must point to storage for a
/// SidereonPreciseEphemerisInterpolant*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_interpolant_from_sp3(
    sp3: *const SidereonSp3,
    out_handle: *mut *mut SidereonPreciseEphemerisInterpolant,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_precise_ephemeris_interpolant_from_sp3",
        SidereonStatus::Panic,
        || {
            let out_handle = c_try!(require_out(
                out_handle,
                "sidereon_precise_ephemeris_interpolant_from_sp3",
                "out_handle"
            ));
            *out_handle = ptr::null_mut();
            let sp3 = c_try!(require_ref(
                sp3,
                "sidereon_precise_ephemeris_interpolant_from_sp3",
                "sp3"
            ));
            write_boxed_handle(
                out_handle,
                SidereonPreciseEphemerisInterpolant {
                    inner: PreciseEphemerisInterpolant::from_sp3(&sp3.inner),
                },
            );
            SidereonStatus::Ok
        },
    )
}

/// Build a cached precise-ephemeris interpolant from canonical samples. On
/// success writes a newly owned handle to *out_handle; release it with
/// sidereon_precise_ephemeris_interpolant_free.
///
/// Safety: samples must point to count entries or be NULL when count is 0;
/// out_handle must point to storage for a SidereonPreciseEphemerisInterpolant*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_interpolant_from_samples(
    samples: *const SidereonPreciseEphemerisSample,
    count: usize,
    out_handle: *mut *mut SidereonPreciseEphemerisInterpolant,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_precise_ephemeris_interpolant_from_samples",
        SidereonStatus::Panic,
        || {
            let out_handle = c_try!(require_out(
                out_handle,
                "sidereon_precise_ephemeris_interpolant_from_samples",
                "out_handle"
            ));
            *out_handle = ptr::null_mut();
            let raw = c_try!(require_slice(
                samples,
                count,
                "sidereon_precise_ephemeris_interpolant_from_samples",
                "samples"
            ));
            let mut parsed = Vec::with_capacity(raw.len());
            for sample in raw {
                parsed.push(c_try!(precise_sample_from_c(
                    "sidereon_precise_ephemeris_interpolant_from_samples",
                    sample
                )));
            }
            let inner = match PreciseEphemerisInterpolant::from_samples(parsed) {
                Ok(inner) => inner,
                Err(err) => {
                    return map_precise_interpolant_error(
                        "sidereon_precise_ephemeris_interpolant_from_samples",
                        err,
                    )
                }
            };
            write_boxed_handle(out_handle, SidereonPreciseEphemerisInterpolant { inner });
            SidereonStatus::Ok
        },
    )
}

/// Build a cached precise-ephemeris interpolant from an existing sample-backed
/// source handle. On success writes a newly owned handle to *out_handle; release
/// it with sidereon_precise_ephemeris_interpolant_free.
///
/// Safety: samples must be a live handle; out_handle must point to storage for a
/// SidereonPreciseEphemerisInterpolant*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_interpolant_from_precise_ephemeris_samples(
    samples: *const SidereonPreciseEphemerisSamples,
    out_handle: *mut *mut SidereonPreciseEphemerisInterpolant,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_precise_ephemeris_interpolant_from_precise_ephemeris_samples",
        SidereonStatus::Panic,
        || {
            let out_handle = c_try!(require_out(
                out_handle,
                "sidereon_precise_ephemeris_interpolant_from_precise_ephemeris_samples",
                "out_handle"
            ));
            *out_handle = ptr::null_mut();
            let samples = c_try!(require_ref(
                samples,
                "sidereon_precise_ephemeris_interpolant_from_precise_ephemeris_samples",
                "samples"
            ));
            write_boxed_handle(
                out_handle,
                SidereonPreciseEphemerisInterpolant {
                    inner: PreciseEphemerisInterpolant::from_precise_ephemeris_samples(
                        &samples.inner,
                    ),
                },
            );
            SidereonStatus::Ok
        },
    )
}

/// Release a cached precise-ephemeris interpolant. Null is a no-op.
///
/// Safety: interpolant must be NULL or a live handle from an interpolant
/// creation function.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_interpolant_free(
    interpolant: *mut SidereonPreciseEphemerisInterpolant,
) {
    ffi_boundary("sidereon_precise_ephemeris_interpolant_free", (), || {
        free_boxed(interpolant);
    });
}

/// Evaluate many sample-backed precise-ephemeris observable states with
/// per-satellite epochs. The output arrays follow
/// sidereon_sp3_observable_states_at_j2000_s.
///
/// Safety: samples is a live handle; all array pointers follow the SP3 batch
/// state contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_samples_observable_states_at_j2000_s(
    samples: *const SidereonPreciseEphemerisSamples,
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
        "sidereon_precise_ephemeris_samples_observable_states_at_j2000_s",
        SidereonStatus::Panic,
        || {
            let samples = c_try!(require_ref(
                samples,
                "sidereon_precise_ephemeris_samples_observable_states_at_j2000_s",
                "samples"
            ));
            observable_states_at_j2000_s_common(
                "sidereon_precise_ephemeris_samples_observable_states_at_j2000_s",
                &samples.inner,
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

/// Evaluate many sample-backed precise-ephemeris observable states at one shared
/// epoch.
///
/// Safety: same output-array contract as
/// sidereon_sp3_observable_states_at_j2000_s.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_samples_observable_states_at_shared_j2000_s(
    samples: *const SidereonPreciseEphemerisSamples,
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
        "sidereon_precise_ephemeris_samples_observable_states_at_shared_j2000_s",
        SidereonStatus::Panic,
        || {
            let samples = c_try!(require_ref(
                samples,
                "sidereon_precise_ephemeris_samples_observable_states_at_shared_j2000_s",
                "samples"
            ));
            observable_states_at_shared_j2000_s_common(
                "sidereon_precise_ephemeris_samples_observable_states_at_shared_j2000_s",
                &samples.inner,
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

/// Evaluate many cached precise-interpolant observable states with
/// per-satellite epochs. The output arrays follow
/// sidereon_sp3_observable_states_at_j2000_s.
///
/// Safety: interpolant is a live handle; all array pointers follow the SP3 batch
/// state contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_interpolant_observable_states_at_j2000_s(
    interpolant: *const SidereonPreciseEphemerisInterpolant,
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
        "sidereon_precise_ephemeris_interpolant_observable_states_at_j2000_s",
        SidereonStatus::Panic,
        || {
            let interpolant = c_try!(require_ref(
                interpolant,
                "sidereon_precise_ephemeris_interpolant_observable_states_at_j2000_s",
                "interpolant"
            ));
            observable_states_at_j2000_s_common(
                "sidereon_precise_ephemeris_interpolant_observable_states_at_j2000_s",
                &interpolant.inner,
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

/// Evaluate many cached precise-interpolant observable states at one shared
/// epoch.
///
/// Safety: same output-array contract as
/// sidereon_sp3_observable_states_at_j2000_s.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_interpolant_observable_states_at_shared_j2000_s(
    interpolant: *const SidereonPreciseEphemerisInterpolant,
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
        "sidereon_precise_ephemeris_interpolant_observable_states_at_shared_j2000_s",
        SidereonStatus::Panic,
        || {
            let interpolant = c_try!(require_ref(
                interpolant,
                "sidereon_precise_ephemeris_interpolant_observable_states_at_shared_j2000_s",
                "interpolant"
            ));
            observable_states_at_shared_j2000_s_common(
                "sidereon_precise_ephemeris_interpolant_observable_states_at_shared_j2000_s",
                &interpolant.inner,
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

fn map_precise_samples_error(fn_name: &str, err: PreciseSamplesError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}

unsafe fn precise_sample_from_c(
    fn_name: &str,
    sample: &SidereonPreciseEphemerisSample,
) -> Result<PreciseEphemerisSample, SidereonStatus> {
    let sat = parse_satellite_token(fn_name, sample.sat.bytes.as_ptr())?;
    let scale = time_scale_from_c_code(fn_name, "sample.time_scale", sample.time_scale)?;
    let epoch = instant_from_j2000_seconds(fn_name, "sample", scale, sample.epoch_j2000_s)?;
    let clock_s = if sample.has_clock_s {
        Some(sample.clock_s)
    } else {
        None
    };
    Ok(PreciseEphemerisSample {
        sat,
        epoch,
        position_ecef_m: sample.position_ecef_m,
        clock_s,
        clock_event: sample.clock_event,
    })
}

fn map_precise_interpolant_error(fn_name: &str, err: PreciseInterpolantError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}
