use super::*;

/// A parsed IONEX vertical-TEC product. Create with sidereon_ionex_parse and
/// release with sidereon_ionex_free.
pub struct SidereonIonex {
    pub(crate) inner: Ionex,
}

/// Policy applied when an IONEX slant-delay request is outside product coverage.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonIonexCoveragePolicy {
    /// Return a typed error when the query is outside coverage.
    Strict = 0,
    /// Hold the nearest map or grid edge and return a held status.
    Hold = 1,
}

/// Successful IONEX slant-delay coverage status.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonIonexSlantDelayStatus {
    /// The query was inside product coverage.
    Valid = 0,
    /// The value was produced by the explicit hold policy.
    Held = 1,
}

/// IONEX coverage miss associated with a held slant-delay value.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonIonexCoverageErrorKind {
    /// No coverage error is associated with the value.
    None = 0,
    /// Query epoch precedes the first map epoch.
    EpochBeforeFirstMap = 1,
    /// Query epoch follows the last map epoch.
    EpochAfterLastMap = 2,
    /// Pierce-point latitude is outside the latitude nodes.
    LatitudeOutOfRange = 3,
    /// Pierce-point longitude is outside the longitude nodes.
    LongitudeOutOfRange = 4,
}

/// IONEX slant-delay value plus explicit coverage status.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonIonexSlantDelayEvaluation {
    /// Slant ionospheric group delay in meters.
    pub delay_m: f64,
    /// One of SidereonIonexSlantDelayStatus_*.
    pub status: u32,
    /// One of SidereonIonexCoverageErrorKind_*.
    pub coverage_error: u32,
}

/// Standalone GPS broadcast Klobuchar ionospheric group delay in the model's
/// native units (positive meters). This is the bit-exact (0-ULP) entry: it feeds
/// the kernel directly with no angle or time conversion.
///
/// `alpha`/`beta` are the eight broadcast coefficients (four each).
/// Latitude/longitude and azimuth/elevation are in degrees; `t_gps_s` is the GPS
/// second-of-day in [0, 86400). The L1 delay is scaled to `frequency_hz` by the
/// dispersive (f_l1 / f)^2 factor. Writes the delay to *out_delay_m.
///
/// Safety: alpha and beta must each point to four readable doubles; out_delay_m
/// must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_klobuchar_native(
    alpha: *const f64,
    beta: *const f64,
    lat_deg: f64,
    lon_deg: f64,
    az_deg: f64,
    el_deg: f64,
    t_gps_s: f64,
    frequency_hz: f64,
    out_delay_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_klobuchar_native", SidereonStatus::Panic, || {
        let out_delay_m = c_try!(require_out(
            out_delay_m,
            "sidereon_klobuchar_native",
            "out_delay_m"
        ));
        *out_delay_m = 0.0;
        let alpha = c_try!(require_slice(
            alpha,
            4,
            "sidereon_klobuchar_native",
            "alpha"
        ));
        let beta = c_try!(require_slice(beta, 4, "sidereon_klobuchar_native", "beta"));
        let params = KlobucharParams {
            alpha: [alpha[0], alpha[1], alpha[2], alpha[3]],
            beta: [beta[0], beta[1], beta[2], beta[3]],
        };
        let delay = match klobuchar_native(
            &params,
            lat_deg,
            lon_deg,
            az_deg,
            el_deg,
            t_gps_s,
            frequency_hz,
        ) {
            Ok(delay) => delay,
            Err(err) => return map_iono_error("sidereon_klobuchar_native", err),
        };
        *out_delay_m = delay;
        SidereonStatus::Ok
    })
}

/// Parse an IONEX vertical-TEC product from a byte buffer. On success writes a
/// newly owned handle to *out_ionex. Release it with sidereon_ionex_free.
///
/// Safety: data must point to len readable bytes; out_ionex must point to storage
/// for a SidereonIonex*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_parse(
    data: *const u8,
    len: usize,
    out_ionex: *mut *mut SidereonIonex,
) -> SidereonStatus {
    ffi_boundary("sidereon_ionex_parse", SidereonStatus::Panic, || {
        let out_ionex = c_try!(require_out(out_ionex, "sidereon_ionex_parse", "out_ionex"));
        *out_ionex = ptr::null_mut();
        let bytes = c_try!(require_slice(data, len, "sidereon_ionex_parse", "data"));
        let inner = match Ionex::parse(bytes) {
            Ok(ionex) => ionex,
            Err(err) => return map_iono_error("sidereon_ionex_parse", err),
        };
        write_boxed_handle(out_ionex, SidereonIonex { inner });
        SidereonStatus::Ok
    })
}

/// IONEX vertical-TEC-grid slant ionospheric group delay (positive meters),
/// writing the result to *out_delay_m. Receiver geodetic latitude/longitude and
/// the satellite azimuth/elevation are in degrees; the pierce point rides on the
/// IONEX shell so no receiver height enters. `epoch_j2000_s` is integer seconds
/// since J2000, landing exactly on the product's epoch axis. `frequency_hz` is
/// the carrier the dispersive delay is reported on. Requests outside the product
/// time or grid coverage hold to the nearest product sample, matching the
/// binding's legacy scalar behavior. The numbers are exactly what the engine
/// produces under that coverage policy.
///
/// Safety: ionex must be a live handle from sidereon_ionex_parse; out_delay_m
/// must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_slant_delay(
    ionex: *const SidereonIonex,
    lat_deg: f64,
    lon_deg: f64,
    azimuth_deg: f64,
    elevation_deg: f64,
    epoch_j2000_s: i64,
    frequency_hz: f64,
    out_delay_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_ionex_slant_delay", SidereonStatus::Panic, || {
        let out_delay_m = c_try!(require_out(
            out_delay_m,
            "sidereon_ionex_slant_delay",
            "out_delay_m"
        ));
        *out_delay_m = 0.0;
        let request = IonexSlantDelayCRequest {
            lat_deg,
            lon_deg,
            azimuth_deg,
            elevation_deg,
            epoch_j2000_s,
            frequency_hz,
        };
        let evaluation = c_try!(ionex_slant_delay_eval_from_c(
            "sidereon_ionex_slant_delay",
            ionex,
            request,
            IonexCoveragePolicy::Hold,
        ));
        *out_delay_m = evaluation.delay_m;
        SidereonStatus::Ok
    })
}

/// IONEX vertical-TEC-grid slant ionospheric group delay with explicit coverage
/// policy and status. Receiver geodetic latitude/longitude and satellite
/// azimuth/elevation are in degrees; `epoch_j2000_s` is integer seconds since
/// J2000; `frequency_hz` is the carrier the dispersive delay is reported on.
///
/// Safety: ionex must be a live handle from sidereon_ionex_parse; out must
/// point to a SidereonIonexSlantDelayEvaluation.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_slant_delay_with_policy(
    ionex: *const SidereonIonex,
    lat_deg: f64,
    lon_deg: f64,
    azimuth_deg: f64,
    elevation_deg: f64,
    epoch_j2000_s: i64,
    frequency_hz: f64,
    policy: u32,
    out: *mut SidereonIonexSlantDelayEvaluation,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_slant_delay_with_policy",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_ionex_slant_delay_with_policy",
                "out"
            ));
            *out = zero_ionex_slant_delay_evaluation();
            let policy = c_try!(ionex_coverage_policy_from_c(
                "sidereon_ionex_slant_delay_with_policy",
                policy
            ));
            let request = IonexSlantDelayCRequest {
                lat_deg,
                lon_deg,
                azimuth_deg,
                elevation_deg,
                epoch_j2000_s,
                frequency_hz,
            };
            let evaluation = c_try!(ionex_slant_delay_eval_from_c(
                "sidereon_ionex_slant_delay_with_policy",
                ionex,
                request,
                policy,
            ));
            *out = ionex_slant_delay_evaluation_to_c(evaluation);
            SidereonStatus::Ok
        },
    )
}

/// Write the number of TEC map epochs in the product to *out_count.
///
/// Safety: ionex must be a live handle from sidereon_ionex_parse; out_count must
/// point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_epoch_count(
    ionex: *const SidereonIonex,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_ionex_epoch_count", SidereonStatus::Panic, || {
        let out_count = c_try!(require_out(
            out_count,
            "sidereon_ionex_epoch_count",
            "out_count"
        ));
        *out_count = 0;
        let ionex = c_try!(require_ref(ionex, "sidereon_ionex_epoch_count", "ionex"));
        *out_count = ionex.inner.map_epochs_s().len();
        SidereonStatus::Ok
    })
}

/// Write the IONEX EXPONENT header field to *out_exponent (the TEC scale is
/// 10^EXPONENT).
///
/// Safety: ionex must be a live handle from sidereon_ionex_parse; out_exponent
/// must point to an int32_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_exponent(
    ionex: *const SidereonIonex,
    out_exponent: *mut i32,
) -> SidereonStatus {
    ffi_boundary("sidereon_ionex_exponent", SidereonStatus::Panic, || {
        let out_exponent = c_try!(require_out(
            out_exponent,
            "sidereon_ionex_exponent",
            "out_exponent"
        ));
        *out_exponent = 0;
        let ionex = c_try!(require_ref(ionex, "sidereon_ionex_exponent", "ionex"));
        *out_exponent = ionex.inner.exponent();
        SidereonStatus::Ok
    })
}

/// Copy the latitude node axis (degrees, descending north-to-south). Uses the
/// variable-length output contract documented at the top of the header.
///
/// Safety: ionex must be a live handle from sidereon_ionex_parse; out (when
/// non-NULL) must point to len writable doubles; out_written and out_required
/// must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_lat_nodes_deg(
    ionex: *const SidereonIonex,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_lat_nodes_deg",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ionex_lat_nodes_deg",
                out_written,
                out_required
            ));
            let ionex = c_try!(require_ref(ionex, "sidereon_ionex_lat_nodes_deg", "ionex"));
            c_try!(copy_prefix_to_c(
                "sidereon_ionex_lat_nodes_deg",
                "out",
                ionex.inner.lat_nodes_deg(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the longitude node axis (degrees, ascending west-to-east). Uses the
/// variable-length output contract documented at the top of the header.
///
/// Safety: ionex must be a live handle from sidereon_ionex_parse; out (when
/// non-NULL) must point to len writable doubles; out_written and out_required
/// must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_lon_nodes_deg(
    ionex: *const SidereonIonex,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_lon_nodes_deg",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ionex_lon_nodes_deg",
                out_written,
                out_required
            ));
            let ionex = c_try!(require_ref(ionex, "sidereon_ionex_lon_nodes_deg", "ionex"));
            c_try!(copy_prefix_to_c(
                "sidereon_ionex_lon_nodes_deg",
                "out",
                ionex.inner.lon_nodes_deg(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the TEC map epoch axis as seconds since J2000 (ascending). Uses the
/// variable-length output contract documented at the top of the header.
///
/// Safety: ionex must be a live handle from sidereon_ionex_parse; out (when
/// non-NULL) must point to len writable int64_t; out_written and out_required
/// must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_map_epochs_j2000_s(
    ionex: *const SidereonIonex,
    out: *mut i64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_map_epochs_j2000_s",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ionex_map_epochs_j2000_s",
                out_written,
                out_required
            ));
            let ionex = c_try!(require_ref(
                ionex,
                "sidereon_ionex_map_epochs_j2000_s",
                "ionex"
            ));
            let epochs = ionex.inner.map_epochs_s();
            c_try!(copy_prefix_to_c(
                "sidereon_ionex_map_epochs_j2000_s",
                "out",
                &epochs,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Serialize an IONEX product back to IONEX text. The output is not
/// null-terminated. Uses the variable-length output contract documented at the
/// top of the header: call once with out=NULL to learn *out_required, then again
/// with a buffer of that size. Round-trips with sidereon_ionex_parse.
///
/// Safety: ionex must be a live handle from sidereon_ionex_parse; out must point
/// to at least len writable bytes or be NULL when len is 0; out_written and
/// out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_to_ionex_text(
    ionex: *const SidereonIonex,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_to_ionex_text",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ionex_to_ionex_text",
                out_written,
                out_required
            ));
            let ionex = c_try!(require_ref(ionex, "sidereon_ionex_to_ionex_text", "ionex"));
            let text = ionex.inner.to_ionex_string();
            c_try!(copy_prefix_to_c(
                "sidereon_ionex_to_ionex_text",
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

/// Release an IONEX product handle from sidereon_ionex_parse. Passing NULL is a
/// no-op.
///
/// Safety: ionex must be NULL or a live handle from sidereon_ionex_parse that has
/// not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_free(ionex: *mut SidereonIonex) {
    ffi_boundary("sidereon_ionex_free", (), || {
        free_boxed(ionex);
    });
}

// --- IONEX sample IR --------------------------------------------------------

/// One IONEX vertical-TEC node sample. Angles are degrees, VTEC and RMS are
/// TECU, and the epoch is seconds since J2000 in `time_scale`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTecSample {
    /// Epoch time scale as SidereonTimeScale.
    pub time_scale: u32,
    /// Node epoch, seconds since J2000 in `time_scale`.
    pub epoch_j2000_s: f64,
    /// Latitude node, degrees.
    pub lat_deg: f64,
    /// Longitude node, degrees.
    pub lon_deg: f64,
    /// Vertical TEC, TECU.
    pub vtec_tecu: f64,
    /// Whether rms_tecu carries an RMS value.
    pub has_rms_tecu: bool,
    /// RMS value, TECU, when has_rms_tecu is true.
    pub rms_tecu: f64,
}

/// Whole-grid IONEX vertical-TEC samples for sidereon_ionex_from_tec_grid_samples.
/// Arrays are caller-owned. `tec_maps_tecu` is flattened in
/// `[map][lat][lon]` order with
/// `map_epoch_count * lat_node_count * lon_node_count` values. RMS maps use the
/// same order when `has_rms_maps` is true.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTecGridSamples {
    /// Epoch time scale as SidereonTimeScale.
    pub time_scale: u32,
    /// Map epochs, seconds since J2000 in `time_scale`.
    pub map_epochs_j2000_s: *const f64,
    /// Number of map epochs.
    pub map_epoch_count: usize,
    /// Latitude nodes, degrees, descending.
    pub lat_nodes_deg: *const f64,
    /// Number of latitude nodes.
    pub lat_node_count: usize,
    /// Longitude nodes, degrees, ascending.
    pub lon_nodes_deg: *const f64,
    /// Number of longitude nodes.
    pub lon_node_count: usize,
    /// Signed latitude step, degrees.
    pub dlat_deg: f64,
    /// Signed longitude step, degrees.
    pub dlon_deg: f64,
    /// Single-layer shell height, kilometers.
    pub shell_height_km: f64,
    /// Mean earth radius used by the IONEX geometry, kilometers.
    pub base_radius_km: f64,
    /// IONEX EXPONENT header value.
    pub exponent: i32,
    /// Flattened VTEC maps, TECU.
    pub tec_maps_tecu: *const f64,
    /// Number of flattened VTEC values.
    pub tec_map_value_count: usize,
    /// Whether RMS maps are present.
    pub has_rms_maps: bool,
    /// Flattened RMS maps, TECU, when has_rms_maps is true.
    pub rms_maps_tecu: *const f64,
    /// Number of flattened RMS values.
    pub rms_map_value_count: usize,
}

/// Dimensions and scalar metadata extracted from an IONEX vertical-TEC sample
/// grid. Heights are kilometers and angular fields are degrees.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTecGridSamplesInfo {
    /// Number of map epochs.
    pub map_epoch_count: usize,
    /// Number of latitude nodes.
    pub lat_node_count: usize,
    /// Number of longitude nodes.
    pub lon_node_count: usize,
    /// Signed latitude step, degrees.
    pub dlat_deg: f64,
    /// Signed longitude step, degrees.
    pub dlon_deg: f64,
    /// Single-layer shell height, kilometers.
    pub shell_height_km: f64,
    /// Mean earth radius used by the IONEX geometry, kilometers.
    pub base_radius_km: f64,
    /// IONEX EXPONENT header value.
    pub exponent: i32,
    /// Whether RMS maps are present.
    pub has_rms_maps: bool,
    /// Flattened VTEC value count.
    pub tec_map_value_count: usize,
    /// Flattened RMS value count.
    pub rms_map_value_count: usize,
}

/// Build an IONEX product from whole-grid TEC samples. On success writes a new
/// handle to *out_ionex; release it with sidereon_ionex_free.
///
/// Safety: samples must point to a SidereonTecGridSamples whose arrays match its
/// counts; out_ionex must point to storage for a SidereonIonex*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_from_tec_grid_samples(
    samples: *const SidereonTecGridSamples,
    out_ionex: *mut *mut SidereonIonex,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_from_tec_grid_samples",
        SidereonStatus::Panic,
        || {
            let out_ionex = c_try!(require_out(
                out_ionex,
                "sidereon_ionex_from_tec_grid_samples",
                "out_ionex"
            ));
            *out_ionex = ptr::null_mut();
            let samples = c_try!(require_ref(
                samples,
                "sidereon_ionex_from_tec_grid_samples",
                "samples"
            ));
            let samples = c_try!(tec_grid_samples_from_c(
                "sidereon_ionex_from_tec_grid_samples",
                samples
            ));
            match Ionex::from_samples(samples) {
                Ok(inner) => {
                    write_boxed_handle(out_ionex, SidereonIonex { inner });
                    SidereonStatus::Ok
                }
                Err(err) => map_tec_samples_error("sidereon_ionex_from_tec_grid_samples", err),
            }
        },
    )
}

/// Build an IONEX product from one sample per grid node. Angles are degrees,
/// VTEC/RMS are TECU, shell/base radii are kilometers, and epochs are seconds
/// since J2000 in each sample's time scale.
///
/// Safety: samples points to count SidereonTecSample entries, or NULL when count
/// is zero; out_ionex must point to storage for a SidereonIonex*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_from_tec_samples(
    samples: *const SidereonTecSample,
    count: usize,
    shell_height_km: f64,
    base_radius_km: f64,
    exponent: i32,
    out_ionex: *mut *mut SidereonIonex,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_from_tec_samples",
        SidereonStatus::Panic,
        || {
            let out_ionex = c_try!(require_out(
                out_ionex,
                "sidereon_ionex_from_tec_samples",
                "out_ionex"
            ));
            *out_ionex = ptr::null_mut();
            let raw = c_try!(require_slice(
                samples,
                count,
                "sidereon_ionex_from_tec_samples",
                "samples"
            ));
            let mut parsed = Vec::with_capacity(raw.len());
            for sample in raw {
                parsed.push(c_try!(tec_sample_from_c(
                    "sidereon_ionex_from_tec_samples",
                    sample
                )));
            }
            match Ionex::from_node_samples(parsed, shell_height_km, base_radius_km, exponent) {
                Ok(inner) => {
                    write_boxed_handle(out_ionex, SidereonIonex { inner });
                    SidereonStatus::Ok
                }
                Err(err) => map_tec_samples_error("sidereon_ionex_from_tec_samples", err),
            }
        },
    )
}

/// Read IONEX TEC-grid sample dimensions and scalar metadata. Heights are
/// kilometers and angular fields are degrees.
///
/// Safety: ionex must be a live handle; out_info must point to a
/// SidereonTecGridSamplesInfo.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_tec_grid_samples_info(
    ionex: *const SidereonIonex,
    out_info: *mut SidereonTecGridSamplesInfo,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_tec_grid_samples_info",
        SidereonStatus::Panic,
        || {
            let out_info = c_try!(require_out(
                out_info,
                "sidereon_ionex_tec_grid_samples_info",
                "out_info"
            ));
            *out_info = SidereonTecGridSamplesInfo {
                map_epoch_count: 0,
                lat_node_count: 0,
                lon_node_count: 0,
                dlat_deg: 0.0,
                dlon_deg: 0.0,
                shell_height_km: 0.0,
                base_radius_km: 0.0,
                exponent: 0,
                has_rms_maps: false,
                tec_map_value_count: 0,
                rms_map_value_count: 0,
            };
            let ionex = c_try!(require_ref(
                ionex,
                "sidereon_ionex_tec_grid_samples_info",
                "ionex"
            ));
            let samples = ionex.inner.tec_grid_samples();
            *out_info = tec_grid_samples_info(&samples);
            SidereonStatus::Ok
        },
    )
}

/// Copy TEC-grid map epochs as seconds since J2000. Uses the variable-length
/// output contract.
///
/// Safety: ionex must be a live handle; out points to len writable doubles or
/// NULL when len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_tec_grid_samples_epochs_j2000_s(
    ionex: *const SidereonIonex,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_tec_grid_samples_epochs_j2000_s",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ionex_tec_grid_samples_epochs_j2000_s",
                out_written,
                out_required
            ));
            let ionex = c_try!(require_ref(
                ionex,
                "sidereon_ionex_tec_grid_samples_epochs_j2000_s",
                "ionex"
            ));
            let samples = ionex.inner.tec_grid_samples();
            let epochs: Vec<f64> = samples
                .map_epochs
                .iter()
                .map(|epoch| instant_to_j2000_seconds(epoch).unwrap_or(f64::NAN))
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_ionex_tec_grid_samples_epochs_j2000_s",
                "out",
                &epochs,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy flattened IONEX VTEC maps in `[map][lat][lon]` order, TECU. Uses the
/// variable-length output contract.
///
/// Safety: ionex must be a live handle; out points to len writable doubles or
/// NULL when len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_tec_grid_samples_tec_maps_tecu(
    ionex: *const SidereonIonex,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_tec_grid_samples_tec_maps_tecu",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ionex_tec_grid_samples_tec_maps_tecu",
                out_written,
                out_required
            ));
            let ionex = c_try!(require_ref(
                ionex,
                "sidereon_ionex_tec_grid_samples_tec_maps_tecu",
                "ionex"
            ));
            let samples = ionex.inner.tec_grid_samples();
            let flat = flatten_tec_maps(&samples.tec_maps);
            c_try!(copy_prefix_to_c(
                "sidereon_ionex_tec_grid_samples_tec_maps_tecu",
                "out",
                &flat,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy flattened IONEX RMS maps in `[map][lat][lon]` order, TECU. If no RMS
/// maps are present, the required length is zero.
///
/// Safety: ionex must be a live handle; out points to len writable doubles or
/// NULL when len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_tec_grid_samples_rms_maps_tecu(
    ionex: *const SidereonIonex,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_tec_grid_samples_rms_maps_tecu",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ionex_tec_grid_samples_rms_maps_tecu",
                out_written,
                out_required
            ));
            let ionex = c_try!(require_ref(
                ionex,
                "sidereon_ionex_tec_grid_samples_rms_maps_tecu",
                "ionex"
            ));
            let samples = ionex.inner.tec_grid_samples();
            let flat = flatten_tec_maps(&samples.rms_maps);
            c_try!(copy_prefix_to_c(
                "sidereon_ionex_tec_grid_samples_rms_maps_tecu",
                "out",
                &flat,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy one IONEX TEC sample per grid node. Uses the variable-length output
/// contract. Angles are degrees and VTEC/RMS values are TECU.
///
/// Safety: ionex must be a live handle; out points to len SidereonTecSample
/// entries or NULL when len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_tec_samples(
    ionex: *const SidereonIonex,
    out: *mut SidereonTecSample,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_ionex_tec_samples", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_ionex_tec_samples",
            out_written,
            out_required
        ));
        let ionex = c_try!(require_ref(ionex, "sidereon_ionex_tec_samples", "ionex"));
        let values: Vec<SidereonTecSample> = ionex
            .inner
            .tec_samples()
            .iter()
            .map(tec_sample_to_c)
            .collect();
        c_try!(copy_prefix_to_c(
            "sidereon_ionex_tec_samples",
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

// === CRINEX (Hatanaka) decode + RINEX 3 observation read ===================
//
// This is the first slice of the RINEX surface for C: CRINEX decode and the
// RINEX-3 observation reader. RINEX navigation and RINEX clock are not yet
// exposed here.

// ============================================================================

// --- Galileo NeQuick-G ionosphere (sidereon_core::atmosphere::ionosphere) ----

/// Galileo coefficient-driven single-frequency ionospheric group delay in the
/// model's native units (positive meters). Delegates to
/// sidereon_core::atmosphere::ionosphere::galileo_nequick_g_native.
///
/// `ai0`/`ai1`/`ai2` are the three broadcast NeQuick-G coefficients.
/// Latitude/longitude/elevation are in degrees; `t_gal_s` is the Galileo-system
/// second of day and `day_of_year` is the fractional day of year. The slant TEC
/// is mapped to meters on `frequency_hz`. Writes the delay to *out_delay_m.
///
/// Safety: out_delay_m must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_galileo_nequick_g_native(
    ai0: f64,
    ai1: f64,
    ai2: f64,
    lat_deg: f64,
    lon_deg: f64,
    el_deg: f64,
    t_gal_s: f64,
    day_of_year: f64,
    frequency_hz: f64,
    out_delay_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_galileo_nequick_g_native",
        SidereonStatus::Panic,
        || {
            let out_delay_m = c_try!(require_out(
                out_delay_m,
                "sidereon_galileo_nequick_g_native",
                "out_delay_m"
            ));
            *out_delay_m = 0.0;
            let coeffs = GalileoNequickCoeffs { ai0, ai1, ai2 };
            let eval = GalileoNequickEval {
                lat_deg,
                lon_deg,
                el_deg,
                t_gal_s,
                day_of_year,
                frequency_hz,
            };
            match galileo_nequick_g_native(&coeffs, eval) {
                Ok(delay) => {
                    *out_delay_m = delay;
                    SidereonStatus::Ok
                }
                Err(err) => map_iono_error("sidereon_galileo_nequick_g_native", err),
            }
        },
    )
}

// ============================================================================

// --- NeQuick-G full slant integration (sidereon_core::atmosphere::ionosphere) -

/// Receiver/satellite ray geometry and epoch for a full NeQuick-G evaluation,
/// mirroring sidereon_core::atmosphere::ionosphere::NequickGRayEval. Geodetic
/// longitudes and latitudes are in degrees, heights in metres above the
/// reference sphere; `month` is 1..=12 and `utc_hours` is in [0, 24].
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonNequickGRay {
    /// Month of the year, 1..=12.
    pub month: u8,
    /// UTC time of day in hours, [0, 24].
    pub utc_hours: f64,
    /// Receiver geodetic longitude, degrees.
    pub station_lon_deg: f64,
    /// Receiver geodetic latitude, degrees.
    pub station_lat_deg: f64,
    /// Receiver height above the reference sphere, metres.
    pub station_height_m: f64,
    /// Satellite geodetic longitude, degrees.
    pub satellite_lon_deg: f64,
    /// Satellite geodetic latitude, degrees.
    pub satellite_lat_deg: f64,
    /// Satellite height above the reference sphere, metres.
    pub satellite_height_m: f64,
}

/// Full NeQuick-G slant total electron content along the ray, in TECU. Delegates
/// to sidereon_core::atmosphere::ionosphere::nequick_g_stec_tecu.
///
/// `ai0`/`ai1`/`ai2` are the three Galileo broadcast effective-ionisation
/// coefficients. Writes the slant TEC to *out_stec_tecu.
///
/// Safety: ray must point to a SidereonNequickGRay; out_stec_tecu to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nequick_g_stec_tecu(
    ai0: f64,
    ai1: f64,
    ai2: f64,
    ray: *const SidereonNequickGRay,
    out_stec_tecu: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_nequick_g_stec_tecu",
        SidereonStatus::Panic,
        || {
            let out_stec_tecu = c_try!(require_out(
                out_stec_tecu,
                "sidereon_nequick_g_stec_tecu",
                "out_stec_tecu"
            ));
            *out_stec_tecu = 0.0;
            let ray = c_try!(require_ref(ray, "sidereon_nequick_g_stec_tecu", "ray"));
            let coeffs = GalileoNequickCoeffs { ai0, ai1, ai2 };
            match nequick_g_stec_tecu(&coeffs, &nequick_g_ray_from_c(ray)) {
                Ok(stec) => {
                    *out_stec_tecu = stec;
                    SidereonStatus::Ok
                }
                Err(err) => map_iono_error("sidereon_nequick_g_stec_tecu", err),
            }
        },
    )
}

/// Full NeQuick-G slant ionospheric group delay (positive metres) on
/// `frequency_hz`. Delegates to
/// sidereon_core::atmosphere::ionosphere::nequick_g_delay_m.
///
/// Safety: ray must point to a SidereonNequickGRay; out_delay_m to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nequick_g_delay_m(
    ai0: f64,
    ai1: f64,
    ai2: f64,
    ray: *const SidereonNequickGRay,
    frequency_hz: f64,
    out_delay_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_nequick_g_delay_m", SidereonStatus::Panic, || {
        let out_delay_m = c_try!(require_out(
            out_delay_m,
            "sidereon_nequick_g_delay_m",
            "out_delay_m"
        ));
        *out_delay_m = 0.0;
        let ray = c_try!(require_ref(ray, "sidereon_nequick_g_delay_m", "ray"));
        let coeffs = GalileoNequickCoeffs { ai0, ai1, ai2 };
        match nequick_g_delay_m(&coeffs, &nequick_g_ray_from_c(ray), frequency_hz) {
            Ok(delay) => {
                *out_delay_m = delay;
                SidereonStatus::Ok
            }
            Err(err) => map_iono_error("sidereon_nequick_g_delay_m", err),
        }
    })
}

fn zero_ionex_slant_delay_evaluation() -> SidereonIonexSlantDelayEvaluation {
    SidereonIonexSlantDelayEvaluation {
        delay_m: 0.0,
        status: SidereonIonexSlantDelayStatus::Valid as u32,
        coverage_error: SidereonIonexCoverageErrorKind::None as u32,
    }
}

fn ionex_coverage_policy_from_c(
    fn_name: &str,
    policy: u32,
) -> Result<IonexCoveragePolicy, SidereonStatus> {
    match policy {
        value if value == SidereonIonexCoveragePolicy::Strict as u32 => {
            Ok(IonexCoveragePolicy::Strict)
        }
        value if value == SidereonIonexCoveragePolicy::Hold as u32 => Ok(IonexCoveragePolicy::Hold),
        _ => {
            set_last_error(format!("{fn_name}: invalid IONEX coverage policy"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

#[derive(Clone, Copy)]
struct IonexSlantDelayCRequest {
    lat_deg: f64,
    lon_deg: f64,
    azimuth_deg: f64,
    elevation_deg: f64,
    epoch_j2000_s: i64,
    frequency_hz: f64,
}

unsafe fn ionex_slant_delay_eval_from_c(
    fn_name: &str,
    ionex: *const SidereonIonex,
    request: IonexSlantDelayCRequest,
    policy: IonexCoveragePolicy,
) -> Result<IonexSlantDelayEvaluation, SidereonStatus> {
    let ionex = require_ref(ionex, fn_name, "ionex")?;
    let receiver = geodetic_to_wgs84(
        fn_name,
        "receiver",
        SidereonGeodetic {
            lat_rad: request.lat_deg * IONO_DEG_TO_RAD,
            lon_rad: request.lon_deg * IONO_DEG_TO_RAD,
            height_m: 0.0,
        },
    )?;
    match ionex_slant_delay_with_policy(
        &ionex.inner,
        receiver,
        request.elevation_deg * IONO_DEG_TO_RAD,
        request.azimuth_deg * IONO_DEG_TO_RAD,
        request.epoch_j2000_s,
        request.frequency_hz,
        policy,
    ) {
        Ok(evaluation) => Ok(evaluation),
        Err(err) => Err(map_iono_error(fn_name, err)),
    }
}

fn ionex_slant_delay_evaluation_to_c(
    evaluation: IonexSlantDelayEvaluation,
) -> SidereonIonexSlantDelayEvaluation {
    let (status, coverage_error) = match evaluation.status {
        IonexSlantDelayStatus::Valid => (
            SidereonIonexSlantDelayStatus::Valid as u32,
            SidereonIonexCoverageErrorKind::None as u32,
        ),
        IonexSlantDelayStatus::Held(error) => (
            SidereonIonexSlantDelayStatus::Held as u32,
            ionex_coverage_error_to_c(error),
        ),
    };
    SidereonIonexSlantDelayEvaluation {
        delay_m: evaluation.delay_m,
        status,
        coverage_error,
    }
}

fn ionex_coverage_error_to_c(error: IonexCoverageError) -> u32 {
    match error {
        IonexCoverageError::EpochBeforeFirstMap => {
            SidereonIonexCoverageErrorKind::EpochBeforeFirstMap as u32
        }
        IonexCoverageError::EpochAfterLastMap => {
            SidereonIonexCoverageErrorKind::EpochAfterLastMap as u32
        }
        IonexCoverageError::LatitudeOutOfRange => {
            SidereonIonexCoverageErrorKind::LatitudeOutOfRange as u32
        }
        IonexCoverageError::LongitudeOutOfRange => {
            SidereonIonexCoverageErrorKind::LongitudeOutOfRange as u32
        }
    }
}

/// Map a core ionosphere error to a status code: malformed inputs report
/// SIDEREON_STATUS_INVALID_ARGUMENT, everything else SIDEREON_STATUS_SOLVE.
fn map_iono_error(fn_name: &str, err: CoreError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        CoreError::InvalidInput(_) => SidereonStatus::InvalidArgument,
        _ => SidereonStatus::Solve,
    }
}

fn map_tec_samples_error(fn_name: &str, err: TecSamplesError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}

fn flatten_tec_maps(maps: &[Vec<Vec<f64>>]) -> Vec<f64> {
    maps.iter()
        .flat_map(|map| map.iter().flat_map(|row| row.iter().copied()))
        .collect()
}

unsafe fn tec_grid_samples_from_c(
    fn_name: &str,
    samples: &SidereonTecGridSamples,
) -> Result<CoreTecGridSamples, SidereonStatus> {
    let scale = time_scale_from_c_code(fn_name, "samples.time_scale", samples.time_scale)?;
    let map_epochs_s = require_slice(
        samples.map_epochs_j2000_s,
        samples.map_epoch_count,
        fn_name,
        "samples.map_epochs_j2000_s",
    )?;
    let mut map_epochs = Vec::with_capacity(map_epochs_s.len());
    for (idx, &epoch_s) in map_epochs_s.iter().enumerate() {
        map_epochs.push(instant_from_j2000_seconds(
            fn_name,
            &format!("samples.map_epochs_j2000_s[{idx}]"),
            scale,
            epoch_s,
        )?);
    }
    let lat_nodes = require_slice(
        samples.lat_nodes_deg,
        samples.lat_node_count,
        fn_name,
        "samples.lat_nodes_deg",
    )?;
    let lon_nodes = require_slice(
        samples.lon_nodes_deg,
        samples.lon_node_count,
        fn_name,
        "samples.lon_nodes_deg",
    )?;
    let expected = checked_tec_grid_value_count(
        fn_name,
        samples.map_epoch_count,
        samples.lat_node_count,
        samples.lon_node_count,
    )?;
    if samples.tec_map_value_count != expected {
        set_last_error(format!(
            "{fn_name}: samples.tec_map_value_count must be {expected}"
        ));
        return Err(SidereonStatus::InvalidArgument);
    }
    let tec_flat = require_slice(
        samples.tec_maps_tecu,
        samples.tec_map_value_count,
        fn_name,
        "samples.tec_maps_tecu",
    )?;
    let rms_maps = if samples.has_rms_maps {
        if samples.rms_map_value_count != expected {
            set_last_error(format!(
                "{fn_name}: samples.rms_map_value_count must be {expected}"
            ));
            return Err(SidereonStatus::InvalidArgument);
        }
        let rms_flat = require_slice(
            samples.rms_maps_tecu,
            samples.rms_map_value_count,
            fn_name,
            "samples.rms_maps_tecu",
        )?;
        nested_tec_maps(
            rms_flat,
            samples.map_epoch_count,
            samples.lat_node_count,
            samples.lon_node_count,
        )
    } else {
        Vec::new()
    };
    Ok(CoreTecGridSamples {
        map_epochs,
        lat_nodes_deg: lat_nodes.to_vec(),
        lon_nodes_deg: lon_nodes.to_vec(),
        dlat_deg: samples.dlat_deg,
        dlon_deg: samples.dlon_deg,
        shell_height_km: samples.shell_height_km,
        base_radius_km: samples.base_radius_km,
        exponent: samples.exponent,
        tec_maps: nested_tec_maps(
            tec_flat,
            samples.map_epoch_count,
            samples.lat_node_count,
            samples.lon_node_count,
        ),
        rms_maps,
    })
}

fn tec_sample_to_c(sample: &CoreTecSample) -> SidereonTecSample {
    SidereonTecSample {
        time_scale: time_scale_to_c_code(sample.epoch.scale),
        epoch_j2000_s: instant_to_j2000_seconds(&sample.epoch).unwrap_or(f64::NAN),
        lat_deg: sample.lat_deg,
        lon_deg: sample.lon_deg,
        vtec_tecu: sample.vtec_tecu,
        has_rms_tecu: sample.rms_tecu.is_some(),
        rms_tecu: sample.rms_tecu.unwrap_or(0.0),
    }
}

unsafe fn tec_sample_from_c(
    fn_name: &str,
    sample: &SidereonTecSample,
) -> Result<CoreTecSample, SidereonStatus> {
    let scale = time_scale_from_c_code(fn_name, "sample.time_scale", sample.time_scale)?;
    let epoch = instant_from_j2000_seconds(fn_name, "sample", scale, sample.epoch_j2000_s)?;
    Ok(CoreTecSample {
        epoch,
        lat_deg: sample.lat_deg,
        lon_deg: sample.lon_deg,
        vtec_tecu: sample.vtec_tecu,
        rms_tecu: sample.has_rms_tecu.then_some(sample.rms_tecu),
    })
}

fn tec_grid_samples_info(samples: &CoreTecGridSamples) -> SidereonTecGridSamplesInfo {
    let tec_map_value_count = samples
        .map_epochs
        .len()
        .saturating_mul(samples.lat_nodes_deg.len())
        .saturating_mul(samples.lon_nodes_deg.len());
    SidereonTecGridSamplesInfo {
        map_epoch_count: samples.map_epochs.len(),
        lat_node_count: samples.lat_nodes_deg.len(),
        lon_node_count: samples.lon_nodes_deg.len(),
        dlat_deg: samples.dlat_deg,
        dlon_deg: samples.dlon_deg,
        shell_height_km: samples.shell_height_km,
        base_radius_km: samples.base_radius_km,
        exponent: samples.exponent,
        has_rms_maps: !samples.rms_maps.is_empty(),
        tec_map_value_count,
        rms_map_value_count: if samples.rms_maps.is_empty() {
            0
        } else {
            tec_map_value_count
        },
    }
}

fn nequick_g_ray_from_c(ray: &SidereonNequickGRay) -> NequickGRayEval {
    NequickGRayEval {
        month: ray.month,
        utc_hours: ray.utc_hours,
        station_lon_deg: ray.station_lon_deg,
        station_lat_deg: ray.station_lat_deg,
        station_height_m: ray.station_height_m,
        satellite_lon_deg: ray.satellite_lon_deg,
        satellite_lat_deg: ray.satellite_lat_deg,
        satellite_height_m: ray.satellite_height_m,
    }
}

fn checked_tec_grid_value_count(
    fn_name: &str,
    map_count: usize,
    lat_count: usize,
    lon_count: usize,
) -> Result<usize, SidereonStatus> {
    let map_lat = map_count.checked_mul(lat_count).ok_or_else(|| {
        set_last_error(format!("{fn_name}: IONEX grid dimensions overflow"));
        SidereonStatus::InvalidArgument
    })?;
    let total = map_lat.checked_mul(lon_count).ok_or_else(|| {
        set_last_error(format!("{fn_name}: IONEX grid dimensions overflow"));
        SidereonStatus::InvalidArgument
    })?;
    validate_element_count::<f64>(fn_name, "IONEX grid values", total)?;
    Ok(total)
}

fn nested_tec_maps(
    flat: &[f64],
    map_count: usize,
    lat_count: usize,
    lon_count: usize,
) -> Vec<Vec<Vec<f64>>> {
    let mut maps = Vec::with_capacity(map_count);
    for map_idx in 0..map_count {
        let mut rows = Vec::with_capacity(lat_count);
        for lat_idx in 0..lat_count {
            let offset = (map_idx * lat_count + lat_idx) * lon_count;
            rows.push(flat[offset..offset + lon_count].to_vec());
        }
        maps.push(rows);
    }
    maps
}
