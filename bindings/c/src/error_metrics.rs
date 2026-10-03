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
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_error_metrics_from_enu_covariance_m2",
        SidereonStatus::Panic,
        || {
            c_try!(validate_error_metrics_out(
                "sidereon_error_metrics_from_enu_covariance_m2",
                out_metrics,
                "out_metrics",
                out_error,
            ));
            let covariance_result = read_mat3(
                "sidereon_error_metrics_from_enu_covariance_m2",
                "covariance_enu_m2",
                covariance_enu_m2,
            );
            let out = c_try!(init_error_metrics_out(
                "sidereon_error_metrics_from_enu_covariance_m2",
                out_metrics,
                out_error,
            ));
            let covariance = c_try!(covariance_result);
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
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_error_metrics_from_ecef_covariance_m2",
        SidereonStatus::Panic,
        || {
            c_try!(validate_error_metrics_out(
                "sidereon_error_metrics_from_ecef_covariance_m2",
                out_metrics,
                "out_metrics",
                out_error,
            ));
            let covariance_result = read_mat3(
                "sidereon_error_metrics_from_ecef_covariance_m2",
                "covariance_ecef_m2",
                covariance_ecef_m2,
            );
            let out = c_try!(init_error_metrics_out(
                "sidereon_error_metrics_from_ecef_covariance_m2",
                out_metrics,
                out_error,
            ));
            let covariance = c_try!(covariance_result);
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
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_error_metrics_from_position_covariance",
        SidereonStatus::Panic,
        || {
            c_try!(validate_error_metrics_out(
                "sidereon_error_metrics_from_position_covariance",
                out_metrics,
                "out_metrics",
                out_error,
            ));
            let covariance_result = require_ref(
                covariance,
                "sidereon_error_metrics_from_position_covariance",
                "covariance",
            )
            .copied();
            let out = c_try!(init_error_metrics_out(
                "sidereon_error_metrics_from_position_covariance",
                out_metrics,
                out_error,
            ));
            let covariance = c_try!(covariance_result);
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
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_error_metrics_from_kinematic_solution",
        SidereonStatus::Panic,
        || {
            c_try!(validate_error_metrics_out(
                "sidereon_error_metrics_from_kinematic_solution",
                out_metrics,
                "out_metrics",
                out_error,
            ));
            let solution_result = require_ref(
                solution,
                "sidereon_error_metrics_from_kinematic_solution",
                "solution",
            )
            .copied();
            let out = c_try!(init_error_metrics_out(
                "sidereon_error_metrics_from_kinematic_solution",
                out_metrics,
                out_error,
            ));
            let solution = c_try!(solution_result);
            let core_solution = sidereon_core::precise_positioning::KinematicEpochSolution {
                position_m: solution.position_m,
                clock_m: 0.0,
                ztd_residual_m: 0.0,
                ambiguities_m: BTreeMap::new(),
                position_covariance_m2: mat3_from_row_major(solution.position_covariance_m2),
                used_sats: Vec::new(),
                innovation_rms_m: 0.0,
                status: sidereon_core::precise_positioning::KinematicEpochStatus::Updated,
                // The metrics read only the position and its covariance.
                ssr_bias_exclusions: Vec::new(),
                unplaced_observations: Vec::new(),
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
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_error_metrics_error_ellipse_from_enu_m2",
        SidereonStatus::Panic,
        || {
            c_try!(validate_error_metrics_out(
                "sidereon_error_metrics_error_ellipse_from_enu_m2",
                out_ellipse,
                "out_ellipse",
                out_error,
            ));
            let covariance_result = read_mat3(
                "sidereon_error_metrics_error_ellipse_from_enu_m2",
                "covariance_enu_m2",
                covariance_enu_m2,
            );
            let out = c_try!(init_error_ellipse_out(
                "sidereon_error_metrics_error_ellipse_from_enu_m2",
                out_ellipse,
                out_error,
            ));
            let covariance = c_try!(covariance_result);
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
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_error_metrics_horizontal_radius_at",
        SidereonStatus::Panic,
        || {
            c_try!(validate_error_metrics_out(
                "sidereon_error_metrics_horizontal_radius_at",
                out_radius,
                "out_radius",
                out_error,
            ));
            let covariance_result = read_mat3(
                "sidereon_error_metrics_horizontal_radius_at",
                "covariance_enu_m2",
                covariance_enu_m2,
            );
            let out = c_try!(init_percentile_radius_out(
                "sidereon_error_metrics_horizontal_radius_at",
                out_radius,
                out_error,
                probability,
            ));
            let covariance = c_try!(covariance_result);
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
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_error_metrics_spherical_radius_at",
        SidereonStatus::Panic,
        || {
            c_try!(validate_error_metrics_out(
                "sidereon_error_metrics_spherical_radius_at",
                out_radius,
                "out_radius",
                out_error,
            ));
            let covariance_result = read_mat3(
                "sidereon_error_metrics_spherical_radius_at",
                "covariance_enu_m2",
                covariance_enu_m2,
            );
            let out = c_try!(init_percentile_radius_out(
                "sidereon_error_metrics_spherical_radius_at",
                out_radius,
                out_error,
                probability,
            ));
            let covariance = c_try!(covariance_result);
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
    crate::engine_error::engine_error_operation_boundary(
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

unsafe fn validate_error_metrics_out<T: Copy>(
    fn_name: &str,
    output: *mut T,
    output_name: &str,
    out_error: *mut SidereonErrorMetricsErrorKind,
) -> Result<(), SidereonStatus> {
    let output = require_out(output, fn_name, output_name)?;
    let out_error = require_out(out_error, fn_name, "out_error")?;
    reject_output_pair_alias(fn_name, output, output_name, out_error, "out_error")
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
    validate_error_metrics_out(fn_name, out_metrics, "out_metrics", out_error)?;
    let out_metrics = require_out(out_metrics, fn_name, "out_metrics")?;
    *out_metrics = empty_position_error_metrics();
    let out_error = require_out(out_error, fn_name, "out_error")?;
    *out_error = SidereonErrorMetricsErrorKind::None;
    Ok((&mut *out_metrics, &mut *out_error))
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
    validate_error_metrics_out(fn_name, out_ellipse, "out_ellipse", out_error)?;
    let out_ellipse = require_out(out_ellipse, fn_name, "out_ellipse")?;
    *out_ellipse = SidereonErrorEllipse {
        semi_major_m: 0.0,
        semi_minor_m: 0.0,
        orientation_rad: 0.0,
    };
    let out_error = require_out(out_error, fn_name, "out_error")?;
    *out_error = SidereonErrorMetricsErrorKind::None;
    Ok((&mut *out_ellipse, &mut *out_error))
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
    validate_error_metrics_out(fn_name, out_radius, "out_radius", out_error)?;
    let out_radius = require_out(out_radius, fn_name, "out_radius")?;
    *out_radius = empty_percentile_radius(probability);
    let out_error = require_out(out_error, fn_name, "out_error")?;
    *out_error = SidereonErrorMetricsErrorKind::None;
    Ok((&mut *out_radius, &mut *out_error))
}

unsafe fn init_vertical_radius_out<'a>(
    fn_name: &str,
    out_radius_m: *mut f64,
    out_error: *mut SidereonErrorMetricsErrorKind,
) -> Result<(&'a mut f64, &'a mut SidereonErrorMetricsErrorKind), SidereonStatus> {
    validate_error_metrics_out(fn_name, out_radius_m, "out_radius_m", out_error)?;
    let out_radius_m = require_out(out_radius_m, fn_name, "out_radius_m")?;
    *out_radius_m = 0.0;
    let out_error = require_out(out_error, fn_name, "out_error")?;
    *out_error = SidereonErrorMetricsErrorKind::None;
    Ok((&mut *out_radius_m, &mut *out_error))
}

unsafe fn reject_output_pair_alias<T, U>(
    fn_name: &str,
    first: *mut T,
    first_name: &str,
    second: *mut U,
    second_name: &str,
) -> Result<(), SidereonStatus> {
    let first = checked_output_range(fn_name, first, 1, first_name)?;
    let second = checked_output_range(fn_name, second, 1, second_name)?;
    reject_overlapping_outputs(fn_name, first, second, first_name, second_name)
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
    crate::engine_error::record_engine_error(
        crate::engine_error::SidereonEngineErrorFamily::ErrorMetrics,
        fn_name,
        crate::engine_error::error_metrics_error_value(&err),
    );
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

#[cfg(test)]
mod engine_error_tests {
    use super::*;
    use crate::engine_error::{snapshot_engine_error_for_test, SidereonEngineErrorFamily};

    #[repr(C)]
    union PositionCovarianceMetricsStorage {
        covariance: SidereonPositionCovariance,
        metrics: SidereonPositionErrorMetrics,
    }

    #[repr(C)]
    union CovarianceErrorStorage {
        covariance: [f64; 9],
        error: SidereonErrorMetricsErrorKind,
    }

    fn kinematic_input(position_m: [f64; 3]) -> SidereonKinematicSolutionMetricsInput {
        SidereonKinematicSolutionMetricsInput {
            position_m,
            position_covariance_m2: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        }
    }

    unsafe fn assert_rotation_refusal(
        input: &SidereonKinematicSolutionMetricsInput,
        metrics: *mut SidereonPositionErrorMetrics,
        out_error: *mut SidereonErrorMetricsErrorKind,
    ) -> String {
        assert_eq!(
            sidereon_error_metrics_from_kinematic_solution(input, metrics, out_error),
            SidereonStatus::InvalidArgument
        );
        assert_eq!(*out_error, SidereonErrorMetricsErrorKind::Rotation);
        let (info, payload) =
            snapshot_engine_error_for_test().expect("typed error-metrics refusal");
        assert_eq!(info.family, SidereonEngineErrorFamily::ErrorMetrics);
        assert_eq!(info.payload_len, payload.len());
        let value: serde_json::Value = serde_json::from_str(&payload).expect("valid JSON");
        assert_eq!(
            value,
            serde_json::json!({
                "schema_version":1,
                "family":"error_metrics",
                "operation":"sidereon_error_metrics_from_kinematic_solution",
                "error":{
                    "kind":"rotation",
                    "fields":{"cause":{
                        "kind":"invalid_input",
                        "fields":{"field":"position_m","reason":"geodetic conversion failed"}
                    }}
                }
            })
        );
        let mut message = vec![0 as std::os::raw::c_char; 256];
        unsafe { crate::sidereon_last_error_message(message.as_mut_ptr(), message.len()) };
        assert_eq!(
            unsafe { std::ffi::CStr::from_ptr(message.as_ptr()) }
                .to_str()
                .expect("legacy UTF-8"),
            "sidereon_error_metrics_from_kinematic_solution: Rotation(InvalidInput { field: \"position_m\", reason: \"geodetic conversion failed\" })"
        );
        payload
    }

    #[test]
    fn kinematic_rotation_error_retains_complete_owned_payload_and_resets() {
        let invalid = kinematic_input([f64::NAN, 0.0, 0.0]);
        let valid = kinematic_input([6_378_137.0, 0.0, 0.0]);
        let mut metrics = empty_position_error_metrics();
        let mut out_error = SidereonErrorMetricsErrorKind::None;
        let expected = unsafe { assert_rotation_refusal(&invalid, &mut metrics, &mut out_error) };

        let mut written = 0;
        let mut required = 0;
        assert_eq!(
            unsafe {
                crate::sidereon_last_engine_error_payload(
                    std::ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(required, expected.len());
        let mut canary = [0xA5_u8; 8];
        assert_eq!(
            unsafe {
                crate::sidereon_last_engine_error_payload(
                    canary.as_mut_ptr(),
                    canary.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert_eq!(canary, [0xA5; 8]);
        assert_eq!(required, expected.len());

        let mut payload_bytes = vec![0; required];
        assert_eq!(
            unsafe {
                crate::sidereon_last_engine_error_payload(
                    payload_bytes.as_mut_ptr(),
                    payload_bytes.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(written, expected.len());
        assert_eq!(String::from_utf8(payload_bytes).unwrap(), expected);
        let mut info = crate::engine_error::SidereonEngineErrorInfo {
            family: SidereonEngineErrorFamily::None,
            payload_len: 0,
        };
        assert_eq!(
            unsafe { crate::sidereon_last_engine_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.family, SidereonEngineErrorFamily::ErrorMetrics);
        assert_eq!(snapshot_engine_error_for_test().unwrap().1, expected);

        assert_eq!(
            unsafe {
                assert_rotation_refusal(&invalid, &mut metrics, &mut out_error);
                sidereon_error_metrics_from_kinematic_solution(&valid, &mut metrics, &mut out_error)
            },
            SidereonStatus::Ok
        );
        assert_eq!(out_error, SidereonErrorMetricsErrorKind::None);
        assert_eq!(metrics.sigma_e_m, 1.0);
        assert_eq!(metrics.sigma_n_m, 1.0);
        assert_eq!(metrics.sigma_u_m, 1.0);
        assert_eq!(metrics.drms_m, std::f64::consts::SQRT_2);
        assert_eq!(metrics.two_drms_m, 2.0 * std::f64::consts::SQRT_2);
        assert_eq!(metrics.mrse_m, 1.732_050_807_568_877_2);
        assert!(snapshot_engine_error_for_test().is_none());

        unsafe { assert_rotation_refusal(&invalid, &mut metrics, &mut out_error) };
        assert_eq!(
            unsafe {
                sidereon_error_metrics_from_kinematic_solution(
                    &invalid,
                    std::ptr::null_mut(),
                    &mut out_error,
                )
            },
            SidereonStatus::NullPointer
        );
        assert!(snapshot_engine_error_for_test().is_none());
    }

    #[test]
    fn position_covariance_input_can_overlap_metrics_output() {
        let covariance = SidereonPositionCovariance {
            ecef_m2: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
            enu_m2: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        };
        let mut expected = empty_position_error_metrics();
        let mut expected_error = SidereonErrorMetricsErrorKind::NonFinite;
        assert_eq!(
            unsafe {
                sidereon_error_metrics_from_position_covariance(
                    &covariance,
                    &mut expected,
                    &mut expected_error,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(expected_error, SidereonErrorMetricsErrorKind::None);

        let mut storage = PositionCovarianceMetricsStorage { covariance };
        let storage_ptr = &mut storage as *mut PositionCovarianceMetricsStorage;
        let covariance_ptr = unsafe { std::ptr::addr_of!((*storage_ptr).covariance) };
        let metrics_ptr = unsafe { std::ptr::addr_of_mut!((*storage_ptr).metrics) };
        let mut actual_error = SidereonErrorMetricsErrorKind::NonFinite;
        assert_eq!(
            unsafe {
                sidereon_error_metrics_from_position_covariance(
                    covariance_ptr,
                    metrics_ptr,
                    &mut actual_error,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(actual_error, SidereonErrorMetricsErrorKind::None);
        let actual = unsafe { metrics_ptr.read() };
        assert_eq!(actual.sigma_e_m, expected.sigma_e_m);
        assert_eq!(actual.sigma_n_m, expected.sigma_n_m);
        assert_eq!(actual.sigma_u_m, expected.sigma_u_m);
        assert_eq!(actual.drms_m, expected.drms_m);
    }

    #[test]
    fn enu_covariance_input_can_overlap_error_output() {
        let covariance = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        let mut expected = empty_position_error_metrics();
        let mut expected_error = SidereonErrorMetricsErrorKind::NonFinite;
        assert_eq!(
            unsafe {
                sidereon_error_metrics_from_enu_covariance_m2(
                    covariance.as_ptr(),
                    &mut expected,
                    &mut expected_error,
                )
            },
            SidereonStatus::Ok
        );

        let mut storage = CovarianceErrorStorage { covariance };
        let storage_ptr = &mut storage as *mut CovarianceErrorStorage;
        let covariance_ptr = unsafe { std::ptr::addr_of!((*storage_ptr).covariance).cast::<f64>() };
        let error_ptr = unsafe { std::ptr::addr_of_mut!((*storage_ptr).error) };
        let mut actual = empty_position_error_metrics();
        assert_eq!(
            unsafe {
                sidereon_error_metrics_from_enu_covariance_m2(
                    covariance_ptr,
                    &mut actual,
                    error_ptr,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(
            unsafe { error_ptr.read() },
            SidereonErrorMetricsErrorKind::None
        );
        assert_eq!(actual.sigma_e_m, expected.sigma_e_m);
        assert_eq!(actual.sigma_n_m, expected.sigma_n_m);
        assert_eq!(actual.sigma_u_m, expected.sigma_u_m);
        assert_eq!(actual.drms_m, expected.drms_m);
    }

    #[test]
    fn output_pair_overlap_is_refused_before_reset_and_null_error_preserves_first_output() {
        let covariance = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        let mut metrics = empty_position_error_metrics();
        metrics.sigma_e_m = 42.0;
        let metrics_ptr = &mut metrics as *mut SidereonPositionErrorMetrics;
        let overlapping_error = metrics_ptr.cast::<SidereonErrorMetricsErrorKind>();
        assert_eq!(
            unsafe {
                sidereon_error_metrics_from_enu_covariance_m2(
                    covariance.as_ptr(),
                    metrics_ptr,
                    overlapping_error,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert_eq!(metrics.sigma_e_m, 42.0);

        assert_eq!(
            unsafe {
                sidereon_error_metrics_from_enu_covariance_m2(
                    covariance.as_ptr(),
                    metrics_ptr,
                    std::ptr::null_mut(),
                )
            },
            SidereonStatus::NullPointer
        );
        // Output-pair validation now precedes resetting either output.
        assert_eq!(metrics.sigma_e_m, 42.0);
    }

    #[test]
    fn every_error_metrics_producer_clears_freshly_seeded_detail() {
        let invalid = kinematic_input([f64::NAN, 0.0, 0.0]);
        let valid = kinematic_input([6_378_137.0, 0.0, 0.0]);
        let matrix = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        let covariance = SidereonPositionCovariance {
            ecef_m2: matrix,
            enu_m2: matrix,
        };
        let receiver = SidereonGeodetic {
            lat_rad: 0.1,
            lon_rad: 0.2,
            height_m: 0.0,
        };
        let mut metrics = empty_position_error_metrics();
        let mut ellipse = error_ellipse_to_c(sidereon_core::error_metrics::ErrorEllipse {
            semi_major_m: 0.0,
            semi_minor_m: 0.0,
            orientation_rad: 0.0,
        });
        let mut radius = empty_percentile_radius(0.95);
        let mut vertical_radius = 0.0;
        let mut out_error = SidereonErrorMetricsErrorKind::None;

        macro_rules! assert_success {
            ($call:expr) => {{
                unsafe { assert_rotation_refusal(&invalid, &mut metrics, &mut out_error) };
                assert_eq!($call, SidereonStatus::Ok);
                assert!(snapshot_engine_error_for_test().is_none());
            }};
        }
        macro_rules! assert_null {
            ($call:expr) => {{
                unsafe { assert_rotation_refusal(&invalid, &mut metrics, &mut out_error) };
                assert_eq!($call, SidereonStatus::NullPointer);
                assert!(snapshot_engine_error_for_test().is_none());
            }};
        }

        assert_success!(unsafe {
            sidereon_error_metrics_from_enu_covariance_m2(
                matrix.as_ptr(),
                &mut metrics,
                &mut out_error,
            )
        });
        assert_success!(unsafe {
            sidereon_error_metrics_from_ecef_covariance_m2(
                matrix.as_ptr(),
                receiver,
                &mut metrics,
                &mut out_error,
            )
        });
        assert_success!(unsafe {
            sidereon_error_metrics_from_position_covariance(
                &covariance,
                &mut metrics,
                &mut out_error,
            )
        });
        assert_success!(unsafe {
            sidereon_error_metrics_from_kinematic_solution(&valid, &mut metrics, &mut out_error)
        });
        assert_success!(unsafe {
            sidereon_error_metrics_error_ellipse_from_enu_m2(
                matrix.as_ptr(),
                &mut ellipse,
                &mut out_error,
            )
        });
        assert_success!(unsafe {
            sidereon_error_metrics_horizontal_radius_at(
                matrix.as_ptr(),
                0.95,
                &mut radius,
                &mut out_error,
            )
        });
        assert_success!(unsafe {
            sidereon_error_metrics_spherical_radius_at(
                matrix.as_ptr(),
                0.95,
                &mut radius,
                &mut out_error,
            )
        });
        assert_success!(unsafe {
            sidereon_error_metrics_vertical_radius_at(
                1.0,
                0.95,
                &mut vertical_radius,
                &mut out_error,
            )
        });

        assert_null!(unsafe {
            sidereon_error_metrics_from_enu_covariance_m2(
                matrix.as_ptr(),
                std::ptr::null_mut(),
                &mut out_error,
            )
        });
        assert_null!(unsafe {
            sidereon_error_metrics_from_ecef_covariance_m2(
                matrix.as_ptr(),
                receiver,
                std::ptr::null_mut(),
                &mut out_error,
            )
        });
        assert_null!(unsafe {
            sidereon_error_metrics_from_position_covariance(
                &covariance,
                std::ptr::null_mut(),
                &mut out_error,
            )
        });
        assert_null!(unsafe {
            sidereon_error_metrics_from_kinematic_solution(
                &valid,
                std::ptr::null_mut(),
                &mut out_error,
            )
        });
        assert_null!(unsafe {
            sidereon_error_metrics_error_ellipse_from_enu_m2(
                matrix.as_ptr(),
                std::ptr::null_mut(),
                &mut out_error,
            )
        });
        assert_null!(unsafe {
            sidereon_error_metrics_horizontal_radius_at(
                matrix.as_ptr(),
                0.95,
                std::ptr::null_mut(),
                &mut out_error,
            )
        });
        assert_null!(unsafe {
            sidereon_error_metrics_spherical_radius_at(
                matrix.as_ptr(),
                0.95,
                std::ptr::null_mut(),
                &mut out_error,
            )
        });
        assert_null!(unsafe {
            sidereon_error_metrics_vertical_radius_at(
                1.0,
                0.95,
                std::ptr::null_mut(),
                &mut out_error,
            )
        });
    }
}
