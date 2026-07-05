use super::*;

/// A geodesic polygon fence on WGS84. Opaque to C. Create with
/// sidereon_geofence_create and release with sidereon_geofence_free.
pub struct SidereonGeofence {
    pub(crate) inner: CoreGeofence,
}

/// Geofence error category returned through out_error fields.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonGeofenceErrorKind {
    /// No geofence error occurred.
    None = 0,
    /// The fence had fewer than three distinct vertices.
    TooFewVertices = 1,
    /// A geofence input failed validation.
    InvalidInput = 2,
    /// A geodesic calculation failed.
    Geodesic = 3,
    /// A covariance rotation failed.
    Dop = 4,
    /// Uncertainty or percentile-radius validation failed.
    ErrorMetrics = 5,
}

/// Position uncertainty representation for geofence probability calls.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonGeofenceUncertaintyKind {
    /// Local east-north-up covariance in square meters.
    EnuCovarianceM2 = 0,
    /// ECEF covariance in square meters, rotated at the supplied position.
    EcefCovarianceM2 = 1,
    /// Circular error probable radius in meters.
    CepRadiusM = 2,
}

/// Position uncertainty for geofence probability calls.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonGeofenceUncertainty {
    /// Uncertainty kind as a SidereonGeofenceUncertaintyKind discriminant.
    pub kind: u32,
    /// Row-major 3x3 covariance in square meters for covariance kinds.
    pub covariance_m2: [f64; 9],
    /// Radius in meters for radius kinds.
    pub radius_m: f64,
}

/// Probability integration method.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonGeofenceProbabilityMethod {
    /// Boundary-normal Gaussian half-space approximation.
    BoundaryNormal = 0,
    /// Local planar quadrature over the fence.
    PlanarQuadrature = 1,
}

/// Probability integration options. Initialize with
/// sidereon_geofence_probability_options_init for engine defaults.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonGeofenceProbabilityOptions {
    /// Method as a SidereonGeofenceProbabilityMethod discriminant.
    pub method: u32,
}

/// Probability hysteresis thresholds. Initialize with
/// sidereon_geofence_hysteresis_init for engine defaults.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonGeofenceHysteresis {
    /// Inside probability required before emitting an entered event.
    pub enter_confidence: f64,
    /// Outside probability required before emitting a left event.
    pub leave_confidence: f64,
}

/// Position estimate with uncertainty for probabilistic crossing detection.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonGeofencePositionEstimate {
    /// Estimated WGS84 geodetic position.
    pub position: SidereonGeodetic,
    /// Position uncertainty associated with the estimate.
    pub uncertainty: SidereonGeofenceUncertainty,
}

/// Geofence crossing event kind.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonGeofenceCrossingKind {
    /// The sequence entered the fence.
    Entered = 0,
    /// The sequence left the fence.
    Left = 1,
}

/// A probabilistic geofence crossing event.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonGeofenceCrossingEvent {
    /// Index of the input sample that first satisfied the crossing condition.
    pub sample_index: usize,
    /// Event kind as a SidereonGeofenceCrossingKind discriminant.
    pub kind: u32,
    /// Inside probability at the event sample.
    pub inside_probability: f64,
}

/// Populate *out_options with geofence probability defaults.
///
/// Safety: out_options must point to a SidereonGeofenceProbabilityOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geofence_probability_options_init(
    out_options: *mut SidereonGeofenceProbabilityOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_geofence_probability_options_init",
        SidereonStatus::Panic,
        || {
            let out_options = c_try!(require_out(
                out_options,
                "sidereon_geofence_probability_options_init",
                "out_options"
            ));
            *out_options = SidereonGeofenceProbabilityOptions {
                method: SidereonGeofenceProbabilityMethod::BoundaryNormal as u32,
            };
            SidereonStatus::Ok
        },
    )
}

/// Populate *out_hysteresis with geofence hysteresis defaults.
///
/// Safety: out_hysteresis must point to a SidereonGeofenceHysteresis.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geofence_hysteresis_init(
    out_hysteresis: *mut SidereonGeofenceHysteresis,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_geofence_hysteresis_init",
        SidereonStatus::Panic,
        || {
            let out_hysteresis = c_try!(require_out(
                out_hysteresis,
                "sidereon_geofence_hysteresis_init",
                "out_hysteresis"
            ));
            let defaults = CoreGeofenceHysteresis::default();
            *out_hysteresis = SidereonGeofenceHysteresis {
                enter_confidence: defaults.enter_confidence,
                leave_confidence: defaults.leave_confidence,
            };
            SidereonStatus::Ok
        },
    )
}

/// Construct a WGS84 geodesic polygon fence from count vertices. Height is
/// ignored by containment and distance calculations.
///
/// Safety: vertices points to count SidereonGeodetic values or is NULL when
/// count is 0; out_error and out_fence must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geofence_create(
    vertices: *const SidereonGeodetic,
    count: usize,
    out_error: *mut SidereonGeofenceErrorKind,
    out_fence: *mut *mut SidereonGeofence,
) -> SidereonStatus {
    ffi_boundary("sidereon_geofence_create", SidereonStatus::Panic, || {
        c_try!(init_geofence_error(
            out_error,
            SidereonGeofenceErrorKind::None
        ));
        let out_fence = c_try!(geofence_validation(
            out_error,
            require_out(out_fence, "sidereon_geofence_create", "out_fence")
        ));
        *out_fence = ptr::null_mut();
        let raw = c_try!(geofence_validation(
            out_error,
            require_slice(vertices, count, "sidereon_geofence_create", "vertices")
        ));
        let mut parsed = Vec::with_capacity(raw.len());
        for vertex in raw {
            parsed.push(c_try!(geofence_validation(
                out_error,
                geodetic_to_wgs84("sidereon_geofence_create", "vertices", *vertex)
            )));
        }
        match CoreGeofence::new(parsed) {
            Ok(inner) => {
                write_boxed_handle(out_fence, SidereonGeofence { inner });
                SidereonStatus::Ok
            }
            Err(err) => map_geofence_error("sidereon_geofence_create", err, out_error),
        }
    })
}

/// Write whether position is inside fence to *out_contains.
///
/// Safety: fence must be a live handle; out_contains and out_error must point to
/// writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geofence_contains(
    fence: *const SidereonGeofence,
    position: SidereonGeodetic,
    out_error: *mut SidereonGeofenceErrorKind,
    out_contains: *mut bool,
) -> SidereonStatus {
    ffi_boundary("sidereon_geofence_contains", SidereonStatus::Panic, || {
        c_try!(init_geofence_error(
            out_error,
            SidereonGeofenceErrorKind::None
        ));
        let out_contains = c_try!(geofence_validation(
            out_error,
            require_out(out_contains, "sidereon_geofence_contains", "out_contains")
        ));
        *out_contains = false;
        let fence = c_try!(geofence_validation(
            out_error,
            require_ref(fence, "sidereon_geofence_contains", "fence")
        ));
        let position = c_try!(geofence_validation(
            out_error,
            geodetic_to_wgs84("sidereon_geofence_contains", "position", position)
        ));
        match geofence_containment(position, &fence.inner) {
            Ok(value) => {
                *out_contains = value;
                SidereonStatus::Ok
            }
            Err(err) => map_geofence_error("sidereon_geofence_contains", err, out_error),
        }
    })
}

/// Write signed distance to the fence boundary in meters to *out_distance_m.
/// Positive values are inside and negative values are outside.
///
/// Safety: fence must be a live handle; out_error and out_distance_m must point
/// to writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geofence_distance_to_boundary(
    fence: *const SidereonGeofence,
    position: SidereonGeodetic,
    out_error: *mut SidereonGeofenceErrorKind,
    out_distance_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_geofence_distance_to_boundary",
        SidereonStatus::Panic,
        || {
            c_try!(init_geofence_error(
                out_error,
                SidereonGeofenceErrorKind::None
            ));
            let out_distance_m = c_try!(geofence_validation(
                out_error,
                require_out(
                    out_distance_m,
                    "sidereon_geofence_distance_to_boundary",
                    "out_distance_m"
                )
            ));
            *out_distance_m = 0.0;
            let fence = c_try!(geofence_validation(
                out_error,
                require_ref(fence, "sidereon_geofence_distance_to_boundary", "fence")
            ));
            let position = c_try!(geofence_validation(
                out_error,
                geodetic_to_wgs84(
                    "sidereon_geofence_distance_to_boundary",
                    "position",
                    position
                )
            ));
            match geofence_distance_to_boundary(position, &fence.inner) {
                Ok(value) => {
                    *out_distance_m = value;
                    SidereonStatus::Ok
                }
                Err(err) => {
                    map_geofence_error("sidereon_geofence_distance_to_boundary", err, out_error)
                }
            }
        },
    )
}

/// Write containment probability using engine default probability options.
///
/// Safety: fence must be a live handle; out_error and out_probability must point
/// to writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geofence_containment_probability(
    fence: *const SidereonGeofence,
    position: SidereonGeodetic,
    uncertainty: *const SidereonGeofenceUncertainty,
    out_error: *mut SidereonGeofenceErrorKind,
    out_probability: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_geofence_containment_probability",
        SidereonStatus::Panic,
        || {
            c_try!(init_geofence_error(
                out_error,
                SidereonGeofenceErrorKind::None
            ));
            let out_probability = c_try!(geofence_validation(
                out_error,
                require_out(
                    out_probability,
                    "sidereon_geofence_containment_probability",
                    "out_probability"
                )
            ));
            *out_probability = 0.0;
            let fence = c_try!(geofence_validation(
                out_error,
                require_ref(fence, "sidereon_geofence_containment_probability", "fence")
            ));
            let position = c_try!(geofence_validation(
                out_error,
                geodetic_to_wgs84(
                    "sidereon_geofence_containment_probability",
                    "position",
                    position
                )
            ));
            let uncertainty = c_try!(geofence_validation(
                out_error,
                geofence_uncertainty_from_c(
                    "sidereon_geofence_containment_probability",
                    uncertainty
                )
            ));
            match geofence_probability(position, uncertainty, &fence.inner) {
                Ok(value) => {
                    *out_probability = value;
                    SidereonStatus::Ok
                }
                Err(err) => {
                    map_geofence_error("sidereon_geofence_containment_probability", err, out_error)
                }
            }
        },
    )
}

/// Write containment probability using explicit probability options.
///
/// Safety: fence must be a live handle; uncertainty and options must point to
/// readable structs; out_error and out_probability must point to writable
/// storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geofence_containment_probability_with_options(
    fence: *const SidereonGeofence,
    position: SidereonGeodetic,
    uncertainty: *const SidereonGeofenceUncertainty,
    options: *const SidereonGeofenceProbabilityOptions,
    out_error: *mut SidereonGeofenceErrorKind,
    out_probability: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_geofence_containment_probability_with_options",
        SidereonStatus::Panic,
        || {
            c_try!(init_geofence_error(
                out_error,
                SidereonGeofenceErrorKind::None
            ));
            let out_probability = c_try!(geofence_validation(
                out_error,
                require_out(
                    out_probability,
                    "sidereon_geofence_containment_probability_with_options",
                    "out_probability"
                )
            ));
            *out_probability = 0.0;
            let fence = c_try!(geofence_validation(
                out_error,
                require_ref(
                    fence,
                    "sidereon_geofence_containment_probability_with_options",
                    "fence"
                )
            ));
            let position = c_try!(geofence_validation(
                out_error,
                geodetic_to_wgs84(
                    "sidereon_geofence_containment_probability_with_options",
                    "position",
                    position
                )
            ));
            let uncertainty = c_try!(geofence_validation(
                out_error,
                geofence_uncertainty_from_c(
                    "sidereon_geofence_containment_probability_with_options",
                    uncertainty
                )
            ));
            let options = c_try!(geofence_validation(
                out_error,
                geofence_probability_options_from_c(
                    "sidereon_geofence_containment_probability_with_options",
                    options
                )
            ));
            match geofence_probability_with_options(position, uncertainty, &fence.inner, options) {
                Ok(value) => {
                    *out_probability = value;
                    SidereonStatus::Ok
                }
                Err(err) => map_geofence_error(
                    "sidereon_geofence_containment_probability_with_options",
                    err,
                    out_error,
                ),
            }
        },
    )
}

/// Compute probabilistic crossing events using engine default probability
/// options. Output uses the variable-length contract documented in the header.
///
/// Safety: fence must be a live handle; samples points to count estimates or is
/// NULL when count is 0; out must point to len events or be NULL when len is 0;
/// out_written, out_required, and out_error must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geofence_crossing_probability(
    fence: *const SidereonGeofence,
    samples: *const SidereonGeofencePositionEstimate,
    count: usize,
    hysteresis: *const SidereonGeofenceHysteresis,
    out_error: *mut SidereonGeofenceErrorKind,
    out: *mut SidereonGeofenceCrossingEvent,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_geofence_crossing_probability",
        SidereonStatus::Panic,
        || {
            geofence_crossing_probability_common(
                "sidereon_geofence_crossing_probability",
                fence,
                samples,
                count,
                hysteresis,
                ptr::null(),
                out_error,
                out,
                len,
                out_written,
                out_required,
            )
        },
    )
}

/// Compute probabilistic crossing events using explicit probability options.
/// Output uses the variable-length contract documented in the header.
///
/// Safety: fence must be a live handle; samples points to count estimates or is
/// NULL when count is 0; hysteresis and options point to readable structs; out
/// must point to len events or be NULL when len is 0; count pointers and
/// out_error must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geofence_crossing_probability_with_options(
    fence: *const SidereonGeofence,
    samples: *const SidereonGeofencePositionEstimate,
    count: usize,
    hysteresis: *const SidereonGeofenceHysteresis,
    options: *const SidereonGeofenceProbabilityOptions,
    out_error: *mut SidereonGeofenceErrorKind,
    out: *mut SidereonGeofenceCrossingEvent,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_geofence_crossing_probability_with_options",
        SidereonStatus::Panic,
        || {
            geofence_crossing_probability_common(
                "sidereon_geofence_crossing_probability_with_options",
                fence,
                samples,
                count,
                hysteresis,
                options,
                out_error,
                out,
                len,
                out_written,
                out_required,
            )
        },
    )
}

/// Release a geofence handle. Passing NULL is a no-op.
///
/// Safety: fence must be NULL or a live handle from sidereon_geofence_create
/// that has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geofence_free(fence: *mut SidereonGeofence) {
    ffi_boundary("sidereon_geofence_free", (), || {
        free_boxed(fence);
    });
}

unsafe fn init_geofence_error(
    out_error: *mut SidereonGeofenceErrorKind,
    value: SidereonGeofenceErrorKind,
) -> Result<(), SidereonStatus> {
    let out_error = require_out(out_error, "sidereon_geofence", "out_error")?;
    *out_error = value;
    Ok(())
}

unsafe fn geofence_validation<T>(
    out_error: *mut SidereonGeofenceErrorKind,
    result: Result<T, SidereonStatus>,
) -> Result<T, SidereonStatus> {
    result.inspect_err(|_| {
        let _ = init_geofence_error(out_error, SidereonGeofenceErrorKind::InvalidInput);
    })
}

fn geofence_error_kind(err: &CoreGeofenceError) -> SidereonGeofenceErrorKind {
    match err {
        CoreGeofenceError::TooFewVertices => SidereonGeofenceErrorKind::TooFewVertices,
        CoreGeofenceError::InvalidInput { .. } => SidereonGeofenceErrorKind::InvalidInput,
        CoreGeofenceError::Geodesic(_) => SidereonGeofenceErrorKind::Geodesic,
        CoreGeofenceError::Dop(_) => SidereonGeofenceErrorKind::Dop,
        CoreGeofenceError::ErrorMetrics(_) => SidereonGeofenceErrorKind::ErrorMetrics,
    }
}

unsafe fn map_geofence_error(
    fn_name: &str,
    err: CoreGeofenceError,
    out_error: *mut SidereonGeofenceErrorKind,
) -> SidereonStatus {
    let kind = geofence_error_kind(&err);
    let _ = init_geofence_error(out_error, kind);
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}

fn geofence_covariance_from_row_major(values: [f64; 9]) -> [[f64; 3]; 3] {
    [
        [values[0], values[1], values[2]],
        [values[3], values[4], values[5]],
        [values[6], values[7], values[8]],
    ]
}

unsafe fn geofence_uncertainty_from_c(
    fn_name: &str,
    uncertainty: *const SidereonGeofenceUncertainty,
) -> Result<CoreGeofenceUncertainty, SidereonStatus> {
    let uncertainty = require_ref(uncertainty, fn_name, "uncertainty")?;
    match uncertainty.kind {
        value if value == SidereonGeofenceUncertaintyKind::EnuCovarianceM2 as u32 => {
            Ok(CoreGeofenceUncertainty::EnuCovarianceM2(
                geofence_covariance_from_row_major(uncertainty.covariance_m2),
            ))
        }
        value if value == SidereonGeofenceUncertaintyKind::EcefCovarianceM2 as u32 => {
            Ok(CoreGeofenceUncertainty::EcefCovarianceM2(
                geofence_covariance_from_row_major(uncertainty.covariance_m2),
            ))
        }
        value if value == SidereonGeofenceUncertaintyKind::CepRadiusM as u32 => {
            Ok(CoreGeofenceUncertainty::CepRadiusM(uncertainty.radius_m))
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid uncertainty kind"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

unsafe fn geofence_probability_options_from_c(
    fn_name: &str,
    options: *const SidereonGeofenceProbabilityOptions,
) -> Result<CoreGeofenceOptions, SidereonStatus> {
    let options = require_ref(options, fn_name, "options")?;
    let method = match options.method {
        value if value == SidereonGeofenceProbabilityMethod::BoundaryNormal as u32 => {
            CoreGeofenceMethod::BoundaryNormal
        }
        value if value == SidereonGeofenceProbabilityMethod::PlanarQuadrature as u32 => {
            CoreGeofenceMethod::PlanarQuadrature
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid probability method"));
            return Err(SidereonStatus::InvalidArgument);
        }
    };
    Ok(CoreGeofenceOptions { method })
}

unsafe fn geofence_hysteresis_from_c(
    fn_name: &str,
    hysteresis: *const SidereonGeofenceHysteresis,
) -> Result<CoreGeofenceHysteresis, SidereonStatus> {
    let hysteresis = require_ref(hysteresis, fn_name, "hysteresis")?;
    CoreGeofenceHysteresis::new(hysteresis.enter_confidence, hysteresis.leave_confidence).map_err(
        |err| {
            set_last_error(format!("{fn_name}: {err}"));
            SidereonStatus::InvalidArgument
        },
    )
}

unsafe fn geofence_estimates_from_c(
    fn_name: &str,
    samples: *const SidereonGeofencePositionEstimate,
    count: usize,
) -> Result<Vec<CoreGeofencePositionEstimate>, SidereonStatus> {
    let raw = require_slice(samples, count, fn_name, "samples")?;
    let mut parsed = Vec::with_capacity(raw.len());
    for sample in raw {
        parsed.push(CoreGeofencePositionEstimate {
            position: geodetic_to_wgs84(fn_name, "sample.position", sample.position)?,
            uncertainty: geofence_uncertainty_from_c(fn_name, &sample.uncertainty)?,
        });
    }
    Ok(parsed)
}

fn geofence_crossing_event_to_c(event: &CoreGeofenceEvent) -> SidereonGeofenceCrossingEvent {
    SidereonGeofenceCrossingEvent {
        sample_index: event.sample_index,
        kind: match event.kind {
            CoreGeofenceCrossingKind::Entered => SidereonGeofenceCrossingKind::Entered,
            CoreGeofenceCrossingKind::Left => SidereonGeofenceCrossingKind::Left,
        } as u32,
        inside_probability: event.inside_probability,
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn geofence_crossing_probability_common(
    fn_name: &str,
    fence: *const SidereonGeofence,
    samples: *const SidereonGeofencePositionEstimate,
    count: usize,
    hysteresis: *const SidereonGeofenceHysteresis,
    options: *const SidereonGeofenceProbabilityOptions,
    out_error: *mut SidereonGeofenceErrorKind,
    out: *mut SidereonGeofenceCrossingEvent,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    c_try!(init_geofence_error(
        out_error,
        SidereonGeofenceErrorKind::None
    ));
    c_try!(geofence_validation(
        out_error,
        init_copy_counts(fn_name, out_written, out_required)
    ));
    let fence = c_try!(geofence_validation(
        out_error,
        require_ref(fence, fn_name, "fence")
    ));
    let samples = c_try!(geofence_validation(
        out_error,
        geofence_estimates_from_c(fn_name, samples, count)
    ));
    let hysteresis = c_try!(geofence_validation(
        out_error,
        geofence_hysteresis_from_c(fn_name, hysteresis)
    ));
    let result = if options.is_null() {
        geofence_crossing_probability(&samples, &fence.inner, hysteresis)
    } else {
        let options = c_try!(geofence_validation(
            out_error,
            geofence_probability_options_from_c(fn_name, options)
        ));
        geofence_crossing_probability_with_options(&samples, &fence.inner, hysteresis, options)
    };
    let events = match result {
        Ok(events) => events,
        Err(err) => return map_geofence_error(fn_name, err, out_error),
    };
    let mapped: Vec<SidereonGeofenceCrossingEvent> =
        events.iter().map(geofence_crossing_event_to_c).collect();
    c_try!(copy_prefix_to_c(
        fn_name,
        "out",
        &mapped,
        out,
        len,
        out_written,
        out_required,
    ));
    SidereonStatus::Ok
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(lat_deg: f64, lon_deg: f64) -> SidereonGeodetic {
        SidereonGeodetic {
            lat_rad: lat_deg.to_radians(),
            lon_rad: lon_deg.to_radians(),
            height_m: 0.0,
        }
    }

    #[test]
    fn geofence_probability_matches_core_reference() {
        let vertices = [
            point(37.0, -122.0),
            point(37.0, -121.99),
            point(37.01, -121.99),
            point(37.01, -122.0),
        ];
        let core_vertices = vertices
            .iter()
            .map(|vertex| geodetic_to_wgs84("test", "vertex", *vertex).expect("valid vertex"))
            .collect::<Vec<_>>();
        let core_fence = CoreGeofence::new(core_vertices).expect("core fence");
        let query = point(37.005, -121.995);
        let query_core = geodetic_to_wgs84("test", "query", query).expect("valid query");
        let uncertainty = SidereonGeofenceUncertainty {
            kind: SidereonGeofenceUncertaintyKind::EnuCovarianceM2 as u32,
            covariance_m2: [400.0, 0.0, 0.0, 0.0, 400.0, 0.0, 0.0, 0.0, 0.0],
            radius_m: 0.0,
        };
        let core_uncertainty = CoreGeofenceUncertainty::EnuCovarianceM2([
            [400.0, 0.0, 0.0],
            [0.0, 400.0, 0.0],
            [0.0, 0.0, 0.0],
        ]);
        let expected_distance =
            geofence_distance_to_boundary(query_core, &core_fence).expect("core distance");
        let expected_probability = geofence_probability(query_core, core_uncertainty, &core_fence)
            .expect("core probability");

        let mut error = SidereonGeofenceErrorKind::None;
        let mut fence = ptr::null_mut();
        let status = unsafe {
            sidereon_geofence_create(vertices.as_ptr(), vertices.len(), &mut error, &mut fence)
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(error, SidereonGeofenceErrorKind::None);

        let mut contains = false;
        let status = unsafe { sidereon_geofence_contains(fence, query, &mut error, &mut contains) };
        assert_eq!(status, SidereonStatus::Ok);
        assert!(contains);

        let mut distance = 0.0;
        let status = unsafe {
            sidereon_geofence_distance_to_boundary(fence, query, &mut error, &mut distance)
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert!((distance - expected_distance).abs() < 1.0e-9);

        let mut probability = 0.0;
        let status = unsafe {
            sidereon_geofence_containment_probability(
                fence,
                query,
                &uncertainty,
                &mut error,
                &mut probability,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert!((probability - expected_probability).abs() < 1.0e-15);

        let invalid_uncertainty = SidereonGeofenceUncertainty {
            kind: 999,
            covariance_m2: uncertainty.covariance_m2,
            radius_m: 0.0,
        };
        error = SidereonGeofenceErrorKind::None;
        let status = unsafe {
            sidereon_geofence_containment_probability(
                fence,
                query,
                &invalid_uncertainty,
                &mut error,
                &mut probability,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);
        assert_eq!(error, SidereonGeofenceErrorKind::InvalidInput);

        let invalid_options = SidereonGeofenceProbabilityOptions { method: 999 };
        error = SidereonGeofenceErrorKind::None;
        let status = unsafe {
            sidereon_geofence_containment_probability_with_options(
                fence,
                query,
                &uncertainty,
                &invalid_options,
                &mut error,
                &mut probability,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);
        assert_eq!(error, SidereonGeofenceErrorKind::InvalidInput);

        unsafe { sidereon_geofence_free(fence) };
    }
}
