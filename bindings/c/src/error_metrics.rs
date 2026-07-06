use super::*;

/// Typed error detail for position-error metric functions.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonErrorMetricsErrorKind {
    /// No domain error occurred.
    None = 0,
    /// At least one numeric input was NaN or infinite.
    NonFinite = 1,
    /// The covariance was not positive semidefinite within tolerance.
    NotPositiveSemidefinite = 2,
    /// A probability value was outside the open interval (0, 1).
    InvalidProbability = 3,
    /// ECEF-to-ENU rotation failed.
    Rotation = 4,
}

/// Horizontal one-sigma error ellipse.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonErrorEllipse {
    /// Semi-major axis length, meters.
    pub semi_major_m: f64,
    /// Semi-minor axis length, meters.
    pub semi_minor_m: f64,
    /// Semi-major-axis orientation, radians from east toward north.
    pub orientation_rad: f64,
}

/// Circle or sphere radius containing a target probability mass.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPercentileRadius {
    /// Probability mass inside radius_m.
    pub probability: f64,
    /// Exact circle or sphere radius, meters.
    pub radius_m: f64,
    /// Approximate radius when the named approximation is applicable.
    pub approx_m: f64,
    /// Whether approx_m is valid for the covariance ratio.
    pub approx_valid: bool,
}

/// Standard position-error metrics from one covariance.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPositionErrorMetrics {
    /// Horizontal one-sigma covariance ellipse.
    pub ellipse: SidereonErrorEllipse,
    /// East standard deviation, meters.
    pub sigma_e_m: f64,
    /// North standard deviation, meters.
    pub sigma_n_m: f64,
    /// Up standard deviation, meters.
    pub sigma_u_m: f64,
    /// Horizontal 50 percent circular error probable.
    pub cep_m: SidereonPercentileRadius,
    /// Horizontal 95 percent radius.
    pub r95_m: SidereonPercentileRadius,
    /// Horizontal 99 percent radius.
    pub r99_m: SidereonPercentileRadius,
    /// Distance root mean square, meters.
    pub drms_m: f64,
    /// Two times distance root mean square, meters.
    pub two_drms_m: f64,
    /// Vertical 50 percent one-dimensional radius, meters.
    pub vep_m: f64,
    /// Three-dimensional 50 percent spherical error probable.
    pub sep_m: SidereonPercentileRadius,
    /// Mean radial spherical error, meters.
    pub mrse_m: f64,
}

/// Minimal kinematic solution input for position-error metrics.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonKinematicSolutionMetricsInput {
    /// Receiver ECEF position, meters.
    pub position_m: [f64; 3],
    /// Row-major ECEF position covariance, square meters.
    pub position_covariance_m2: [f64; 9],
}

/// Position covariance in ECEF and local ENU coordinates.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPositionCovariance {
    /// Row-major ECEF position covariance, square meters.
    pub ecef_m2: [f64; 9],
    /// Row-major local ENU position covariance, square meters.
    pub enu_m2: [f64; 9],
}

/// Compute standard metrics from an ENU covariance in square meters.
///
/// Safety: covariance_enu_m2 points to 9 row-major doubles; out_metrics and
/// out_error must point to writable structs.
#[no_mangle]
pub unsafe extern "C" fn sidereon_error_metrics_from_enu_covariance_m2(
    covariance_enu_m2: *const f64,
    out_metrics: *mut SidereonPositionErrorMetrics,
    out_error: *mut SidereonErrorMetricsErrorKind,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_error_metrics_from_enu_covariance_m2",
        SidereonStatus::Panic,
        || {
            let out = c_try!(init_error_metrics_out(
                "sidereon_error_metrics_from_enu_covariance_m2",
                out_metrics,
                out_error,
            ));
            let covariance = c_try!(read_mat3(
                "sidereon_error_metrics_from_enu_covariance_m2",
                "covariance_enu_m2",
                covariance_enu_m2,
            ));
            match sidereon_core::error_metrics::metrics_from_enu_covariance_m2(covariance) {
                Ok(metrics) => {
                    *out.0 = position_error_metrics_to_c(metrics);
                    SidereonStatus::Ok
                }
                Err(err) => map_error_metrics_error(
                    "sidereon_error_metrics_from_enu_covariance_m2",
                    err,
                    out.1,
                ),
            }
        },
    )
}

/// Rotate an ECEF covariance to ENU at receiver and compute standard metrics.
///
/// Safety: covariance_ecef_m2 points to 9 row-major doubles; out_metrics and
/// out_error must point to writable structs.
#[no_mangle]
pub unsafe extern "C" fn sidereon_error_metrics_from_ecef_covariance_m2(
    covariance_ecef_m2: *const f64,
    receiver: SidereonGeodetic,
    out_metrics: *mut SidereonPositionErrorMetrics,
    out_error: *mut SidereonErrorMetricsErrorKind,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_error_metrics_from_ecef_covariance_m2",
        SidereonStatus::Panic,
        || {
            let out = c_try!(init_error_metrics_out(
                "sidereon_error_metrics_from_ecef_covariance_m2",
                out_metrics,
                out_error,
            ));
            let covariance = c_try!(read_mat3(
                "sidereon_error_metrics_from_ecef_covariance_m2",
                "covariance_ecef_m2",
                covariance_ecef_m2,
            ));
            let receiver = c_try!(geodetic_to_wgs84(
                "sidereon_error_metrics_from_ecef_covariance_m2",
                "receiver",
                receiver,
            ));
            match sidereon_core::error_metrics::metrics_from_ecef_covariance_m2(
                covariance, receiver,
            ) {
                Ok(metrics) => {
                    *out.0 = position_error_metrics_to_c(metrics);
                    SidereonStatus::Ok
                }
                Err(err) => map_error_metrics_error(
                    "sidereon_error_metrics_from_ecef_covariance_m2",
                    err,
                    out.1,
                ),
            }
        },
    )
}

/// Compute standard metrics from a position covariance value.
///
/// Safety: covariance, out_metrics, and out_error must point to valid structs.
#[no_mangle]
pub unsafe extern "C" fn sidereon_error_metrics_from_position_covariance(
    covariance: *const SidereonPositionCovariance,
    out_metrics: *mut SidereonPositionErrorMetrics,
    out_error: *mut SidereonErrorMetricsErrorKind,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_error_metrics_from_position_covariance",
        SidereonStatus::Panic,
        || {
            let out = c_try!(init_error_metrics_out(
                "sidereon_error_metrics_from_position_covariance",
                out_metrics,
                out_error,
            ));
            let covariance = c_try!(require_ref(
                covariance,
                "sidereon_error_metrics_from_position_covariance",
                "covariance"
            ));
            let core_covariance = sidereon_core::geometry::PositionCovariance {
                ecef_m2: mat3_from_row_major(covariance.ecef_m2),
                enu_m2: mat3_from_row_major(covariance.enu_m2),
            };
            match sidereon_core::error_metrics::metrics_from_position_covariance(&core_covariance) {
                Ok(metrics) => {
                    *out.0 = position_error_metrics_to_c(metrics);
                    SidereonStatus::Ok
                }
                Err(err) => map_error_metrics_error(
                    "sidereon_error_metrics_from_position_covariance",
                    err,
                    out.1,
                ),
            }
        },
    )
}

/// Compute standard metrics from a kinematic PPP epoch solution shape.
///
/// Safety: solution, out_metrics, and out_error must point to valid structs.
#[no_mangle]
pub unsafe extern "C" fn sidereon_error_metrics_from_kinematic_solution(
    solution: *const SidereonKinematicSolutionMetricsInput,
    out_metrics: *mut SidereonPositionErrorMetrics,
    out_error: *mut SidereonErrorMetricsErrorKind,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_error_metrics_from_kinematic_solution",
        SidereonStatus::Panic,
        || {
            let out = c_try!(init_error_metrics_out(
                "sidereon_error_metrics_from_kinematic_solution",
                out_metrics,
                out_error,
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_error_metrics_from_kinematic_solution",
                "solution"
            ));
            let core_solution = sidereon_core::precise_positioning::KinematicEpochSolution {
                position_m: solution.position_m,
                clock_m: 0.0,
                ztd_residual_m: 0.0,
                ambiguities_m: BTreeMap::new(),
                position_covariance_m2: mat3_from_row_major(solution.position_covariance_m2),
                used_sats: Vec::new(),
                innovation_rms_m: 0.0,
                status: sidereon_core::precise_positioning::KinematicEpochStatus::Updated,
            };
            match sidereon_core::error_metrics::metrics_from_kinematic_solution(&core_solution) {
                Ok(metrics) => {
                    *out.0 = position_error_metrics_to_c(metrics);
                    SidereonStatus::Ok
                }
                Err(err) => map_error_metrics_error(
                    "sidereon_error_metrics_from_kinematic_solution",
                    err,
                    out.1,
                ),
            }
        },
    )
}

/// Horizontal one-sigma ellipse from an ENU covariance in square meters.
///
/// Safety: covariance_enu_m2 points to 9 row-major doubles; out_ellipse and
/// out_error must point to writable structs.
#[no_mangle]
pub unsafe extern "C" fn sidereon_error_metrics_error_ellipse_from_enu_m2(
    covariance_enu_m2: *const f64,
    out_ellipse: *mut SidereonErrorEllipse,
    out_error: *mut SidereonErrorMetricsErrorKind,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_error_metrics_error_ellipse_from_enu_m2",
        SidereonStatus::Panic,
        || {
            let out = c_try!(init_error_ellipse_out(
                "sidereon_error_metrics_error_ellipse_from_enu_m2",
                out_ellipse,
                out_error,
            ));
            let covariance = c_try!(read_mat3(
                "sidereon_error_metrics_error_ellipse_from_enu_m2",
                "covariance_enu_m2",
                covariance_enu_m2,
            ));
            match sidereon_core::error_metrics::error_ellipse_from_enu_m2(covariance) {
                Ok(ellipse) => {
                    *out.0 = error_ellipse_to_c(ellipse);
                    SidereonStatus::Ok
                }
                Err(err) => map_error_metrics_error(
                    "sidereon_error_metrics_error_ellipse_from_enu_m2",
                    err,
                    out.1,
                ),
            }
        },
    )
}

/// Horizontal percentile circle radius from an ENU covariance.
///
/// Safety: covariance_enu_m2 points to 9 row-major doubles; out_radius and
/// out_error must point to writable structs.
#[no_mangle]
pub unsafe extern "C" fn sidereon_error_metrics_horizontal_radius_at(
    covariance_enu_m2: *const f64,
    probability: f64,
    out_radius: *mut SidereonPercentileRadius,
    out_error: *mut SidereonErrorMetricsErrorKind,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_error_metrics_horizontal_radius_at",
        SidereonStatus::Panic,
        || {
            let out = c_try!(init_percentile_radius_out(
                "sidereon_error_metrics_horizontal_radius_at",
                out_radius,
                out_error,
                probability,
            ));
            let covariance = c_try!(read_mat3(
                "sidereon_error_metrics_horizontal_radius_at",
                "covariance_enu_m2",
                covariance_enu_m2,
            ));
            match sidereon_core::error_metrics::horizontal_radius_at(covariance, probability) {
                Ok(radius) => {
                    *out.0 = percentile_radius_to_c(radius);
                    SidereonStatus::Ok
                }
                Err(err) => map_error_metrics_error(
                    "sidereon_error_metrics_horizontal_radius_at",
                    err,
                    out.1,
                ),
            }
        },
    )
}

/// Three-dimensional percentile sphere radius from an ENU covariance.
///
/// Safety: covariance_enu_m2 points to 9 row-major doubles; out_radius and
/// out_error must point to writable structs.
#[no_mangle]
pub unsafe extern "C" fn sidereon_error_metrics_spherical_radius_at(
    covariance_enu_m2: *const f64,
    probability: f64,
    out_radius: *mut SidereonPercentileRadius,
    out_error: *mut SidereonErrorMetricsErrorKind,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_error_metrics_spherical_radius_at",
        SidereonStatus::Panic,
        || {
            let out = c_try!(init_percentile_radius_out(
                "sidereon_error_metrics_spherical_radius_at",
                out_radius,
                out_error,
                probability,
            ));
            let covariance = c_try!(read_mat3(
                "sidereon_error_metrics_spherical_radius_at",
                "covariance_enu_m2",
                covariance_enu_m2,
            ));
            match sidereon_core::error_metrics::spherical_radius_at(covariance, probability) {
                Ok(radius) => {
                    *out.0 = percentile_radius_to_c(radius);
                    SidereonStatus::Ok
                }
                Err(err) => map_error_metrics_error(
                    "sidereon_error_metrics_spherical_radius_at",
                    err,
                    out.1,
                ),
            }
        },
    )
}

/// Vertical one-dimensional percentile radius from an up variance.
///
/// Safety: out_radius_m and out_error must point to writable values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_error_metrics_vertical_radius_at(
    sigma_u_m2: f64,
    probability: f64,
    out_radius_m: *mut f64,
    out_error: *mut SidereonErrorMetricsErrorKind,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_error_metrics_vertical_radius_at",
        SidereonStatus::Panic,
        || {
            let out = c_try!(init_vertical_radius_out(
                "sidereon_error_metrics_vertical_radius_at",
                out_radius_m,
                out_error,
            ));
            match sidereon_core::error_metrics::vertical_radius_at(sigma_u_m2, probability) {
                Ok(radius) => {
                    *out.0 = radius;
                    SidereonStatus::Ok
                }
                Err(err) => {
                    map_error_metrics_error("sidereon_error_metrics_vertical_radius_at", err, out.1)
                }
            }
        },
    )
}

unsafe fn init_error_metrics_out<'a>(
    fn_name: &str,
    out_metrics: *mut SidereonPositionErrorMetrics,
    out_error: *mut SidereonErrorMetricsErrorKind,
) -> Result<
    (
        &'a mut SidereonPositionErrorMetrics,
        &'a mut SidereonErrorMetricsErrorKind,
    ),
    SidereonStatus,
> {
    let out_metrics = require_out(out_metrics, fn_name, "out_metrics")?;
    *out_metrics = empty_position_error_metrics();
    let out_error = require_out(out_error, fn_name, "out_error")?;
    *out_error = SidereonErrorMetricsErrorKind::None;
    Ok((out_metrics, out_error))
}

unsafe fn init_error_ellipse_out<'a>(
    fn_name: &str,
    out_ellipse: *mut SidereonErrorEllipse,
    out_error: *mut SidereonErrorMetricsErrorKind,
) -> Result<
    (
        &'a mut SidereonErrorEllipse,
        &'a mut SidereonErrorMetricsErrorKind,
    ),
    SidereonStatus,
> {
    let out_ellipse = require_out(out_ellipse, fn_name, "out_ellipse")?;
    *out_ellipse = SidereonErrorEllipse {
        semi_major_m: 0.0,
        semi_minor_m: 0.0,
        orientation_rad: 0.0,
    };
    let out_error = require_out(out_error, fn_name, "out_error")?;
    *out_error = SidereonErrorMetricsErrorKind::None;
    Ok((out_ellipse, out_error))
}

unsafe fn init_percentile_radius_out<'a>(
    fn_name: &str,
    out_radius: *mut SidereonPercentileRadius,
    out_error: *mut SidereonErrorMetricsErrorKind,
    probability: f64,
) -> Result<
    (
        &'a mut SidereonPercentileRadius,
        &'a mut SidereonErrorMetricsErrorKind,
    ),
    SidereonStatus,
> {
    let out_radius = require_out(out_radius, fn_name, "out_radius")?;
    *out_radius = empty_percentile_radius(probability);
    let out_error = require_out(out_error, fn_name, "out_error")?;
    *out_error = SidereonErrorMetricsErrorKind::None;
    Ok((out_radius, out_error))
}

unsafe fn init_vertical_radius_out<'a>(
    fn_name: &str,
    out_radius_m: *mut f64,
    out_error: *mut SidereonErrorMetricsErrorKind,
) -> Result<(&'a mut f64, &'a mut SidereonErrorMetricsErrorKind), SidereonStatus> {
    let out_radius_m = require_out(out_radius_m, fn_name, "out_radius_m")?;
    *out_radius_m = 0.0;
    let out_error = require_out(out_error, fn_name, "out_error")?;
    *out_error = SidereonErrorMetricsErrorKind::None;
    Ok((out_radius_m, out_error))
}

fn empty_position_error_metrics() -> SidereonPositionErrorMetrics {
    SidereonPositionErrorMetrics {
        ellipse: SidereonErrorEllipse {
            semi_major_m: 0.0,
            semi_minor_m: 0.0,
            orientation_rad: 0.0,
        },
        sigma_e_m: 0.0,
        sigma_n_m: 0.0,
        sigma_u_m: 0.0,
        cep_m: empty_percentile_radius(0.5),
        r95_m: empty_percentile_radius(0.95),
        r99_m: empty_percentile_radius(0.99),
        drms_m: 0.0,
        two_drms_m: 0.0,
        vep_m: 0.0,
        sep_m: empty_percentile_radius(0.5),
        mrse_m: 0.0,
    }
}

fn empty_percentile_radius(probability: f64) -> SidereonPercentileRadius {
    SidereonPercentileRadius {
        probability,
        radius_m: 0.0,
        approx_m: 0.0,
        approx_valid: false,
    }
}

fn position_error_metrics_to_c(
    metrics: sidereon_core::error_metrics::PositionErrorMetrics,
) -> SidereonPositionErrorMetrics {
    SidereonPositionErrorMetrics {
        ellipse: error_ellipse_to_c(metrics.ellipse),
        sigma_e_m: metrics.sigma_e_m,
        sigma_n_m: metrics.sigma_n_m,
        sigma_u_m: metrics.sigma_u_m,
        cep_m: percentile_radius_to_c(metrics.cep_m),
        r95_m: percentile_radius_to_c(metrics.r95_m),
        r99_m: percentile_radius_to_c(metrics.r99_m),
        drms_m: metrics.drms_m,
        two_drms_m: metrics.two_drms_m,
        vep_m: metrics.vep_m,
        sep_m: percentile_radius_to_c(metrics.sep_m),
        mrse_m: metrics.mrse_m,
    }
}

fn error_ellipse_to_c(ellipse: sidereon_core::error_metrics::ErrorEllipse) -> SidereonErrorEllipse {
    SidereonErrorEllipse {
        semi_major_m: ellipse.semi_major_m,
        semi_minor_m: ellipse.semi_minor_m,
        orientation_rad: ellipse.orientation_rad,
    }
}

fn percentile_radius_to_c(
    radius: sidereon_core::error_metrics::PercentileRadius,
) -> SidereonPercentileRadius {
    SidereonPercentileRadius {
        probability: radius.probability,
        radius_m: radius.radius_m,
        approx_m: radius.approx_m,
        approx_valid: radius.approx_valid,
    }
}

fn mat3_from_row_major(values: [f64; 9]) -> [[f64; 3]; 3] {
    [
        [values[0], values[1], values[2]],
        [values[3], values[4], values[5]],
        [values[6], values[7], values[8]],
    ]
}

fn map_error_metrics_error(
    fn_name: &str,
    err: sidereon_core::error_metrics::ErrorMetricsError,
    out_error: &mut SidereonErrorMetricsErrorKind,
) -> SidereonStatus {
    *out_error = match err {
        sidereon_core::error_metrics::ErrorMetricsError::NonFinite => {
            SidereonErrorMetricsErrorKind::NonFinite
        }
        sidereon_core::error_metrics::ErrorMetricsError::NotPositiveSemidefinite => {
            SidereonErrorMetricsErrorKind::NotPositiveSemidefinite
        }
        sidereon_core::error_metrics::ErrorMetricsError::InvalidProbability => {
            SidereonErrorMetricsErrorKind::InvalidProbability
        }
        sidereon_core::error_metrics::ErrorMetricsError::Rotation(_) => {
            SidereonErrorMetricsErrorKind::Rotation
        }
    };
    set_last_error(format!("{fn_name}: {err:?}"));
    SidereonStatus::InvalidArgument
}
