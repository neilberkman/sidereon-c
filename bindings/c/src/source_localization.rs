use super::*;
use crate::engine_error::{
    dop_error_value, engine_error_operation_boundary, record_engine_error, trls_error_value,
    SidereonEngineErrorFamily,
};
use serde_json::{json, Value};

/// A receiver solution paired with the ephemeris-source provenance that produced
/// it. Opaque to C. Create with sidereon_solve_with_fallback and release with
/// sidereon_sourced_solution_free.
pub struct SidereonSourcedSolution {
    pub(crate) solution: ReceiverSolution,
    pub(crate) source: FixSource,
}

/// Source-localization solve mode.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSourceSolveMode {
    /// Absolute time of arrival. The solved state is position plus origin time.
    Toa = 0,
    /// Time difference of arrival against reference_sensor in the options.
    Tdoa = 1,
}

/// Source-localization trust-region loss selector.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSourceLoss {
    /// Ordinary least squares.
    Linear = 0,
    /// Soft-L1 robust loss.
    SoftL1 = 1,
    /// Huber robust loss.
    Huber = 2,
    /// Cauchy robust loss.
    Cauchy = 3,
    /// Arctangent robust loss.
    Arctan = 4,
}

/// A source-localization sensor position. position_m stores 2D or 3D Cartesian
/// coordinates in meters; dimension selects how many components are read.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSourceSensor {
    /// Coordinate dimension, 2 or 3.
    pub dimension: usize,
    /// Cartesian position in meters. Components beyond dimension are ignored.
    pub position_m: [f64; 3],
    /// Whether propagation_speed_m_s overrides the call-level speed.
    pub has_propagation_speed_m_s: bool,
    /// Per-sensor propagation speed in meters per second when present.
    pub propagation_speed_m_s: f64,
}

/// Options for sidereon_locate_source and sidereon_locate_source_with.
/// Initialize with sidereon_source_locate_options_init for the core defaults.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSourceLocateOptions {
    /// SidereonSourceSolveMode as a uint32_t.
    pub mode: u32,
    /// Reference sensor index when mode is SIDEREON_SOURCE_SOLVE_MODE_TDOA.
    pub reference_sensor: usize,
    /// Timing standard deviation in seconds for covariance and CRLB scaling.
    pub timing_sigma_s: f64,
    /// SidereonSourceLoss as a uint32_t.
    pub loss: u32,
    /// Residual scale in seconds for non-linear loss functions.
    pub f_scale_s: f64,
    /// Whether ftol is present.
    pub has_ftol: bool,
    /// Optional function tolerance.
    pub ftol: f64,
    /// Whether xtol is present.
    pub has_xtol: bool,
    /// Optional step tolerance.
    pub xtol: f64,
    /// Whether gtol is present.
    pub has_gtol: bool,
    /// Optional gradient tolerance.
    pub gtol: f64,
    /// Whether max_nfev is present.
    pub has_max_nfev: bool,
    /// Optional maximum residual evaluations.
    pub max_nfev: usize,
}

/// Closed-form initializer used to start source localization.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSourceInitialGuess {
    /// Coordinate dimension, 2 or 3.
    pub dimension: usize,
    /// Initial position in meters; components beyond dimension are zero.
    pub position_m: [f64; 3],
    /// Whether origin_time_s is present.
    pub has_origin_time_s: bool,
    /// Initial origin time in seconds when present.
    pub origin_time_s: f64,
    /// Root-mean-square residual of the seed in seconds.
    pub residual_rms_s: f64,
}

/// State covariance or CRLB for a source solve. Matrices are row-major.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSourceCovariance {
    /// Position coordinate dimension, 2 or 3.
    pub dimension: usize,
    /// State dimension. ToA is dimension + 1, TDOA is dimension.
    pub state_dimension: usize,
    /// Full state covariance, row-major, maximum 4 by 4.
    pub state: [f64; 16],
    /// Position covariance block in square meters, row-major, maximum 3 by 3.
    pub position_m2: [f64; 9],
    /// Whether origin_time_s2 is present.
    pub has_origin_time_s2: bool,
    /// Origin-time variance in square seconds when present.
    pub origin_time_s2: f64,
    /// Timing sigma in seconds used to scale the covariance.
    pub timing_sigma_s: f64,
}

/// One residual row associated with a sensor.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSourceResidual {
    /// Sensor index in the input array.
    pub sensor_index: usize,
    /// Whether reference_sensor_index is present.
    pub has_reference_sensor_index: bool,
    /// Reference sensor for TDOA residuals.
    pub reference_sensor_index: usize,
    /// Residual in seconds.
    pub residual_s: f64,
}

/// Per-sensor leave-one-out diagnostic.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSourceSensorInfluence {
    /// Sensor index in the input array.
    pub sensor_index: usize,
    /// ToA residual at the full solution in seconds.
    pub residual_s: f64,
    /// Whether leave_one_out_residual_s is present.
    pub has_leave_one_out_residual_s: bool,
    /// Held-out ToA residual in seconds.
    pub leave_one_out_residual_s: f64,
    /// Whether position_delta_m is present.
    pub has_position_delta_m: bool,
    /// Position change between full and leave-one-out solutions, meters.
    pub position_delta_m: f64,
    /// Whether origin_time_delta_s is present.
    pub has_origin_time_delta_s: bool,
    /// Origin-time change between full and leave-one-out solutions, seconds.
    pub origin_time_delta_s: f64,
    /// First-derivative loss weight for the full residual.
    pub loss_weight: f64,
    /// max(abs(residual_s), abs(leave_one_out_residual_s)) / timing_sigma_s.
    /// Robust downweighting is represented only by loss_weight.
    pub score: f64,
}

/// Source-localization solution summary.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSourceSolutionSummary {
    /// Coordinate dimension, 2 or 3.
    pub dimension: usize,
    /// Estimated source position in meters; components beyond dimension are zero.
    pub position_m: [f64; 3],
    /// Whether origin_time_s is present.
    pub has_origin_time_s: bool,
    /// Estimated origin time in seconds.
    pub origin_time_s: f64,
    /// Whether covariance is available.
    pub has_covariance: bool,
    /// Number of residual rows.
    pub residual_count: usize,
    /// Number of per-sensor influence rows.
    pub influence_count: usize,
    /// Geometry observability and covariance-validation diagnostics.
    pub geometry_quality: SidereonGeometryQuality,
    /// Closed-form seed used by the iterative solve.
    pub initial_guess: SidereonSourceInitialGuess,
    /// Trust-region termination status.
    pub status: i32,
    /// Residual evaluations used by the solver.
    pub nfev: usize,
    /// Jacobian evaluations used by the solver.
    pub njev: usize,
    /// Final least-squares cost.
    pub cost: f64,
    /// Infinity norm of the final gradient.
    pub optimality: f64,
}

/// CRLB and DOP for a proposed source geometry.
#[repr(C)]
pub struct SidereonSourceCrlb {
    /// Timing DOP values. Position DOP values multiply seconds into meters.
    pub dop: SidereonDop,
    /// State covariance scaled by the requested timing sigma.
    pub covariance: SidereonSourceCovariance,
}

/// Source-localization solution handle. Opaque to C. Create with
/// sidereon_locate_source, read with sidereon_source_solution_* accessors, and
/// release with sidereon_source_solution_free.
pub struct SidereonSourceSolution {
    pub(crate) inner: CoreSourceSolution,
}

/// Initialize source-localization options to the core defaults: ToA mode, timing
/// sigma 1 second, linear loss, f_scale 1 second, and no explicit tolerances.
///
/// Safety: out_options must point to a SidereonSourceLocateOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_source_locate_options_init(
    out_options: *mut SidereonSourceLocateOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_source_locate_options_init",
        SidereonStatus::Panic,
        || {
            let out_options = c_try!(require_out(
                out_options,
                "sidereon_source_locate_options_init",
                "out_options"
            ));
            *out_options = SidereonSourceLocateOptions {
                mode: SidereonSourceSolveMode::Toa as u32,
                reference_sensor: 0,
                timing_sigma_s: 1.0,
                loss: SidereonSourceLoss::Linear as u32,
                f_scale_s: 1.0,
                has_ftol: false,
                ftol: 0.0,
                has_xtol: false,
                xtol: 0.0,
                has_gtol: false,
                gtol: 0.0,
                has_max_nfev: false,
                max_nfev: 0,
            };
            SidereonStatus::Ok
        },
    )
}

/// Locate a source from sensor arrival times. Sensor positions are caller-owned
/// 2D or 3D Cartesian coordinates in meters. Arrival times are seconds, and
/// propagation_speed_m_s is meters per second. options may be NULL for defaults.
/// This legacy entry point always computes per-sensor influence diagnostics and
/// is equivalent to sidereon_locate_source_with with include_influence true. On
/// success writes a newly owned handle to *out_solution.
///
/// Safety: sensors and arrival_times_s point to sensor_count entries or NULL
/// when sensor_count is 0; options is NULL or points to options; out_solution
/// points to storage for a SidereonSourceSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_locate_source(
    sensors: *const SidereonSourceSensor,
    sensor_count: usize,
    arrival_times_s: *const f64,
    propagation_speed_m_s: f64,
    options: *const SidereonSourceLocateOptions,
    out_solution: *mut *mut SidereonSourceSolution,
) -> SidereonStatus {
    engine_error_operation_boundary("sidereon_locate_source", SidereonStatus::Panic, || {
        let out_solution = c_try!(require_out(
            out_solution,
            "sidereon_locate_source",
            "out_solution"
        ));
        let inputs = (|| {
            let parsed_sensors =
                source_sensors_from_c("sidereon_locate_source", sensors, sensor_count)?;
            let arrivals = require_slice(
                arrival_times_s,
                sensor_count,
                "sidereon_locate_source",
                "arrival_times_s",
            )?
            .to_vec();
            let options = source_options_from_c("sidereon_locate_source", options)?;
            Ok::<_, SidereonStatus>((parsed_sensors, arrivals, options))
        })();
        *out_solution = ptr::null_mut();
        let (parsed_sensors, arrivals, options) = c_try!(inputs);
        match core_locate_source(&parsed_sensors, &arrivals, propagation_speed_m_s, &options) {
            Ok(inner) => {
                write_boxed_handle(out_solution, SidereonSourceSolution { inner });
                SidereonStatus::Ok
            }
            Err(err) => map_source_localization_error("sidereon_locate_source", err),
        }
    })
}

/// Locate a source from sensor arrival times, optionally omitting influence
/// diagnostics. This has the same contract as sidereon_locate_source. When
/// include_influence is false, the per-sensor leave-one-out solves are skipped
/// and the solution's influence list is empty; all other output is bit-identical
/// to sidereon_locate_source.
///
/// Safety: sensors and arrival_times_s point to sensor_count entries or NULL
/// when sensor_count is 0; options is NULL or points to options; out_solution
/// points to storage for a SidereonSourceSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_locate_source_with(
    sensors: *const SidereonSourceSensor,
    sensor_count: usize,
    arrival_times_s: *const f64,
    propagation_speed_m_s: f64,
    options: *const SidereonSourceLocateOptions,
    include_influence: bool,
    out_solution: *mut *mut SidereonSourceSolution,
) -> SidereonStatus {
    engine_error_operation_boundary("sidereon_locate_source_with", SidereonStatus::Panic, || {
        let out_solution = c_try!(require_out(
            out_solution,
            "sidereon_locate_source_with",
            "out_solution"
        ));
        let inputs = (|| {
            let parsed_sensors =
                source_sensors_from_c("sidereon_locate_source_with", sensors, sensor_count)?;
            let arrivals = require_slice(
                arrival_times_s,
                sensor_count,
                "sidereon_locate_source_with",
                "arrival_times_s",
            )?
            .to_vec();
            let options = source_options_from_c("sidereon_locate_source_with", options)?;
            Ok::<_, SidereonStatus>((parsed_sensors, arrivals, options))
        })();
        *out_solution = ptr::null_mut();
        let (parsed_sensors, arrivals, options) = c_try!(inputs);
        let mut config = CoreSourceLocateConfig::default();
        config.options = options;
        config.include_influence = include_influence;
        match core_locate_source_with(&parsed_sensors, &arrivals, propagation_speed_m_s, &config) {
            Ok(inner) => {
                write_boxed_handle(out_solution, SidereonSourceSolution { inner });
                SidereonStatus::Ok
            }
            Err(err) => map_source_localization_error("sidereon_locate_source_with", err),
        }
    })
}

struct SourceInitialGuessCall {
    sensors: *const SidereonSourceSensor,
    sensor_count: usize,
    arrival_times_s: *const f64,
    propagation_speed_m_s: f64,
    mode: u32,
    reference_sensor: usize,
    out_guess: *mut SidereonSourceInitialGuess,
}

unsafe fn source_initial_guess_impl(
    fn_name: &'static str,
    call: SourceInitialGuessCall,
) -> SidereonStatus {
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let out_guess = c_try!(require_out(call.out_guess, fn_name, "out_guess"));
        let inputs = (|| {
            let parsed_sensors = source_sensors_from_c(fn_name, call.sensors, call.sensor_count)?;
            let arrivals = require_slice(
                call.arrival_times_s,
                call.sensor_count,
                fn_name,
                "arrival_times_s",
            )?
            .to_vec();
            let mode = source_solve_mode_from_c(fn_name, call.mode, call.reference_sensor)?;
            Ok::<_, SidereonStatus>((parsed_sensors, arrivals, mode))
        })();
        *out_guess = SidereonSourceInitialGuess {
            dimension: 0,
            position_m: [0.0; 3],
            has_origin_time_s: false,
            origin_time_s: 0.0,
            residual_rms_s: 0.0,
        };
        let (parsed_sensors, arrivals, mode) = c_try!(inputs);
        match core_closed_form_initial_guess(
            &parsed_sensors,
            &arrivals,
            call.propagation_speed_m_s,
            mode,
        ) {
            Ok(guess) => {
                *out_guess = source_initial_guess_to_c(&guess);
                SidereonStatus::Ok
            }
            Err(err) => map_source_localization_error(fn_name, err),
        }
    })
}

/// Compute the closed-form source-localization initial guess.
///
/// Safety: sensors and arrival_times_s point to sensor_count entries; out_guess
/// must point to a SidereonSourceInitialGuess.
#[no_mangle]
pub unsafe extern "C" fn sidereon_closed_form_initial_guess(
    sensors: *const SidereonSourceSensor,
    sensor_count: usize,
    arrival_times_s: *const f64,
    propagation_speed_m_s: f64,
    mode: u32,
    reference_sensor: usize,
    out_guess: *mut SidereonSourceInitialGuess,
) -> SidereonStatus {
    source_initial_guess_impl(
        "sidereon_closed_form_initial_guess",
        SourceInitialGuessCall {
            sensors,
            sensor_count,
            arrival_times_s,
            propagation_speed_m_s,
            mode,
            reference_sensor,
            out_guess,
        },
    )
}

/// Deprecated: use sidereon_closed_form_initial_guess. Computes the same
/// closed-form source-localization initial guess.
///
/// Safety: sensors and arrival_times_s point to sensor_count entries; out_guess
/// must point to a SidereonSourceInitialGuess.
#[no_mangle]
pub unsafe extern "C" fn sidereon_chan_ho_initial_guess(
    sensors: *const SidereonSourceSensor,
    sensor_count: usize,
    arrival_times_s: *const f64,
    propagation_speed_m_s: f64,
    mode: u32,
    reference_sensor: usize,
    out_guess: *mut SidereonSourceInitialGuess,
) -> SidereonStatus {
    source_initial_guess_impl(
        "sidereon_chan_ho_initial_guess",
        SourceInitialGuessCall {
            sensors,
            sensor_count,
            arrival_times_s,
            propagation_speed_m_s,
            mode,
            reference_sensor,
            out_guess,
        },
    )
}

/// Compute timing DOP for a proposed source location. DOP position values
/// multiply timing sigma in seconds to produce meters.
///
/// Safety: sensors points to sensor_count entries; source_position_m points to
/// source_dimension doubles; out_dop points to a SidereonDop.
#[no_mangle]
pub unsafe extern "C" fn sidereon_source_dop(
    sensors: *const SidereonSourceSensor,
    sensor_count: usize,
    source_position_m: *const f64,
    source_dimension: usize,
    propagation_speed_m_s: f64,
    out_dop: *mut SidereonDop,
) -> SidereonStatus {
    engine_error_operation_boundary("sidereon_source_dop", SidereonStatus::Panic, || {
        let out_dop = c_try!(require_uninit_out(
            out_dop,
            "sidereon_source_dop",
            "out_dop"
        ));
        let inputs = (|| {
            let parsed_sensors =
                source_sensors_from_c("sidereon_source_dop", sensors, sensor_count)?;
            let source_position =
                source_position_from_c("sidereon_source_dop", source_position_m, source_dimension)?;
            Ok::<_, SidereonStatus>((parsed_sensors, source_position))
        })();
        out_dop.write(empty_dop());
        let (parsed_sensors, source_position) = c_try!(inputs);
        match core_source_dop(&parsed_sensors, &source_position, propagation_speed_m_s) {
            Ok(dop) => {
                out_dop.write(dop_to_c(dop));
                SidereonStatus::Ok
            }
            Err(err) => map_source_localization_error("sidereon_source_dop", err),
        }
    })
}

/// Compute a timing CRLB and DOP for a proposed source location.
///
/// Safety: sensors points to sensor_count entries; source_position_m points to
/// source_dimension doubles; out_crlb points to a SidereonSourceCrlb.
#[no_mangle]
pub unsafe extern "C" fn sidereon_source_crlb(
    sensors: *const SidereonSourceSensor,
    sensor_count: usize,
    source_position_m: *const f64,
    source_dimension: usize,
    propagation_speed_m_s: f64,
    timing_sigma_s: f64,
    out_crlb: *mut SidereonSourceCrlb,
) -> SidereonStatus {
    engine_error_operation_boundary("sidereon_source_crlb", SidereonStatus::Panic, || {
        let out_crlb = c_try!(require_uninit_out(
            out_crlb,
            "sidereon_source_crlb",
            "out_crlb"
        ));
        let inputs = (|| {
            let parsed_sensors =
                source_sensors_from_c("sidereon_source_crlb", sensors, sensor_count)?;
            let source_position = source_position_from_c(
                "sidereon_source_crlb",
                source_position_m,
                source_dimension,
            )?;
            Ok::<_, SidereonStatus>((parsed_sensors, source_position))
        })();
        out_crlb.write(SidereonSourceCrlb {
            dop: empty_dop(),
            covariance: source_covariance_empty(),
        });
        let (parsed_sensors, source_position) = c_try!(inputs);
        match core_source_crlb(
            &parsed_sensors,
            &source_position,
            propagation_speed_m_s,
            timing_sigma_s,
        ) {
            Ok(CoreSourceCrlb { dop, covariance }) => {
                out_crlb.write(SidereonSourceCrlb {
                    dop: dop_to_c(dop),
                    covariance: source_covariance_to_c(&covariance),
                });
                SidereonStatus::Ok
            }
            Err(err) => map_source_localization_error("sidereon_source_crlb", err),
        }
    })
}

/// Copy a source solution summary into *out_summary.
///
/// Safety: solution must be a live handle; out_summary points to a summary.
#[no_mangle]
pub unsafe extern "C" fn sidereon_source_solution_summary(
    solution: *const SidereonSourceSolution,
    out_summary: *mut SidereonSourceSolutionSummary,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_source_solution_summary",
        SidereonStatus::Panic,
        || {
            let out_summary = c_try!(require_out(
                out_summary,
                "sidereon_source_solution_summary",
                "out_summary"
            ));
            c_try!(reject_output_overlaps_handle(
                "sidereon_source_solution_summary",
                out_summary,
                "out_summary",
                solution,
                "solution"
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_source_solution_summary",
                "solution"
            ));
            *out_summary = source_summary_to_c(&solution.inner);
            SidereonStatus::Ok
        },
    )
}

/// Copy the source solution covariance when available.
///
/// Safety: solution must be a live handle; out_covariance and out_available
/// must point to writable values. The two output ranges must not overlap.
#[no_mangle]
pub unsafe extern "C" fn sidereon_source_solution_covariance(
    solution: *const SidereonSourceSolution,
    out_covariance: *mut SidereonSourceCovariance,
    out_available: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_source_solution_covariance",
        SidereonStatus::Panic,
        || {
            if !out_covariance.is_null() && !out_available.is_null() {
                let covariance_range = c_try!(super::checked_output_range(
                    "sidereon_source_solution_covariance",
                    out_covariance,
                    1,
                    "out_covariance"
                ));
                let available_range = c_try!(super::checked_output_range(
                    "sidereon_source_solution_covariance",
                    out_available,
                    1,
                    "out_available"
                ));
                c_try!(super::reject_overlapping_outputs(
                    "sidereon_source_solution_covariance",
                    covariance_range,
                    available_range,
                    "out_covariance",
                    "out_available"
                ));
            }
            let out_covariance = c_try!(require_out(
                out_covariance,
                "sidereon_source_solution_covariance",
                "out_covariance"
            ));
            let out_available = c_try!(require_out(
                out_available,
                "sidereon_source_solution_covariance",
                "out_available"
            ));
            c_try!(reject_output_overlaps_handle(
                "sidereon_source_solution_covariance",
                out_covariance,
                "out_covariance",
                solution,
                "solution"
            ));
            c_try!(reject_output_overlaps_handle(
                "sidereon_source_solution_covariance",
                out_available,
                "out_available",
                solution,
                "solution"
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_source_solution_covariance",
                "solution"
            ));
            *out_available = false;
            *out_covariance = source_covariance_empty();
            if let Some(covariance) = solution.inner.covariance.as_ref() {
                *out_covariance = source_covariance_to_c(covariance);
                *out_available = true;
            }
            SidereonStatus::Ok
        },
    )
}

/// Copy solution residual rows using the variable-length output contract.
///
/// Safety: solution must be a live handle; out points to len residual rows or is
/// NULL when len is 0; out_written and out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_source_solution_residuals(
    solution: *const SidereonSourceSolution,
    out: *mut SidereonSourceResidual,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_source_solution_residuals",
        SidereonStatus::Panic,
        || {
            c_try!(reject_output_overlaps_handle(
                "sidereon_source_solution_residuals",
                out_written,
                "out_written",
                solution,
                "solution"
            ));
            c_try!(reject_output_overlaps_handle(
                "sidereon_source_solution_residuals",
                out_required,
                "out_required",
                solution,
                "solution"
            ));
            c_try!(reject_outputs_overlapping_handle(
                "sidereon_source_solution_residuals",
                &[
                    (out.cast(), size_of::<SidereonSourceResidual>(), len, "out"),
                    (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                    (out_required.cast(), size_of::<usize>(), 1, "out_required")
                ],
                solution,
                "solution"
            ));
            c_try!(init_copy_counts(
                "sidereon_source_solution_residuals",
                out_written,
                out_required
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_source_solution_residuals",
                "solution"
            ));
            let rows: Vec<SidereonSourceResidual> = solution
                .inner
                .residuals
                .iter()
                .map(source_residual_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_source_solution_residuals",
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

/// Copy per-sensor influence diagnostics using the variable-length output
/// contract.
///
/// Safety: solution must be a live handle; out points to len influence rows or
/// is NULL when len is 0; out_written and out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_source_solution_influences(
    solution: *const SidereonSourceSolution,
    out: *mut SidereonSourceSensorInfluence,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_source_solution_influences",
        SidereonStatus::Panic,
        || {
            c_try!(reject_output_overlaps_handle(
                "sidereon_source_solution_influences",
                out_written,
                "out_written",
                solution,
                "solution"
            ));
            c_try!(reject_output_overlaps_handle(
                "sidereon_source_solution_influences",
                out_required,
                "out_required",
                solution,
                "solution"
            ));
            c_try!(reject_outputs_overlapping_handle(
                "sidereon_source_solution_influences",
                &[
                    (
                        out.cast(),
                        size_of::<SidereonSourceSensorInfluence>(),
                        len,
                        "out"
                    ),
                    (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                    (out_required.cast(), size_of::<usize>(), 1, "out_required")
                ],
                solution,
                "solution"
            ));
            c_try!(init_copy_counts(
                "sidereon_source_solution_influences",
                out_written,
                out_required
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_source_solution_influences",
                "solution"
            ));
            let rows: Vec<SidereonSourceSensorInfluence> = solution
                .inner
                .per_sensor_influence
                .iter()
                .map(source_influence_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_source_solution_influences",
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

/// Release a source-localization solution handle. Null is a no-op.
///
/// Safety: solution must be NULL or a live handle from sidereon_locate_source or
/// sidereon_locate_source_with.
#[no_mangle]
pub unsafe extern "C" fn sidereon_source_solution_free(solution: *mut SidereonSourceSolution) {
    ffi_boundary("sidereon_source_solution_free", (), || {
        free_boxed(solution);
    });
}

fn source_covariance_empty() -> SidereonSourceCovariance {
    SidereonSourceCovariance {
        dimension: 0,
        state_dimension: 0,
        state: [0.0; 16],
        position_m2: [0.0; 9],
        has_origin_time_s2: false,
        origin_time_s2: 0.0,
        timing_sigma_s: 0.0,
    }
}

fn source_covariance_to_c(covariance: &CoreSourceCovariance) -> SidereonSourceCovariance {
    let dimension = covariance.position_m2.len().min(3);
    let state_dimension = covariance.state.len().min(4);
    let mut state = [0.0; 16];
    for row in 0..state_dimension {
        for col in 0..covariance.state[row].len().min(4) {
            state[row * 4 + col] = covariance.state[row][col];
        }
    }
    let mut position_m2 = [0.0; 9];
    for row in 0..dimension {
        for col in 0..covariance.position_m2[row].len().min(3) {
            position_m2[row * 3 + col] = covariance.position_m2[row][col];
        }
    }
    SidereonSourceCovariance {
        dimension,
        state_dimension,
        state,
        position_m2,
        has_origin_time_s2: covariance.origin_time_s2.is_some(),
        origin_time_s2: covariance.origin_time_s2.unwrap_or(0.0),
        timing_sigma_s: covariance.timing_sigma_s,
    }
}

fn source_residual_to_c(residual: &CoreSourceResidual) -> SidereonSourceResidual {
    SidereonSourceResidual {
        sensor_index: residual.sensor_index,
        has_reference_sensor_index: residual.reference_sensor_index.is_some(),
        reference_sensor_index: residual.reference_sensor_index.unwrap_or(0),
        residual_s: residual.residual_s,
    }
}

fn source_influence_to_c(influence: &CoreSourceSensorInfluence) -> SidereonSourceSensorInfluence {
    SidereonSourceSensorInfluence {
        sensor_index: influence.sensor_index,
        residual_s: influence.residual_s,
        has_leave_one_out_residual_s: influence.leave_one_out_residual_s.is_some(),
        leave_one_out_residual_s: influence.leave_one_out_residual_s.unwrap_or(0.0),
        has_position_delta_m: influence.position_delta_m.is_some(),
        position_delta_m: influence.position_delta_m.unwrap_or(0.0),
        has_origin_time_delta_s: influence.origin_time_delta_s.is_some(),
        origin_time_delta_s: influence.origin_time_delta_s.unwrap_or(0.0),
        loss_weight: influence.loss_weight,
        score: influence.score,
    }
}

fn source_summary_to_c(solution: &CoreSourceSolution) -> SidereonSourceSolutionSummary {
    SidereonSourceSolutionSummary {
        dimension: solution.position_m.len().min(3),
        position_m: zero_vec3_from_slice(&solution.position_m),
        has_origin_time_s: solution.origin_time_s.is_some(),
        origin_time_s: solution.origin_time_s.unwrap_or(0.0),
        has_covariance: solution.covariance.is_some(),
        residual_count: solution.residuals.len(),
        influence_count: solution.per_sensor_influence.len(),
        geometry_quality: geometry_quality_to_c(&solution.geometry_quality),
        initial_guess: source_initial_guess_to_c(&solution.initial_guess),
        status: solution.status,
        nfev: solution.nfev,
        njev: solution.njev,
        cost: solution.cost,
        optimality: solution.optimality,
    }
}

fn source_options_from_c(
    fn_name: &str,
    options: *const SidereonSourceLocateOptions,
) -> Result<CoreSourceLocateOptions, SidereonStatus> {
    if options.is_null() {
        return Ok(CoreSourceLocateOptions::default());
    }
    let options = unsafe { require_ref(options, fn_name, "options") }?;
    let mut o = CoreSourceLocateOptions::default();
    o.mode = source_solve_mode_from_c(fn_name, options.mode, options.reference_sensor)?;
    o.timing_sigma_s = options.timing_sigma_s;
    o.loss = source_loss_from_c(fn_name, "options.loss", options.loss)?;
    o.f_scale_s = options.f_scale_s;
    o.ftol = options.has_ftol.then_some(options.ftol);
    o.xtol = options.has_xtol.then_some(options.xtol);
    o.gtol = options.has_gtol.then_some(options.gtol);
    o.max_nfev = options.has_max_nfev.then_some(options.max_nfev);
    Ok(o)
}

unsafe fn source_sensors_from_c(
    fn_name: &str,
    sensors: *const SidereonSourceSensor,
    sensor_count: usize,
) -> Result<Vec<CoreSourceSensor>, SidereonStatus> {
    let raw = require_slice(sensors, sensor_count, fn_name, "sensors")?;
    let mut parsed = Vec::with_capacity(raw.len());
    for sensor in raw {
        if !(2..=3).contains(&sensor.dimension) {
            set_last_error(format!("{fn_name}: sensor.dimension must be 2 or 3"));
            return Err(SidereonStatus::InvalidArgument);
        }
        let position_m = sensor.position_m[..sensor.dimension].to_vec();
        if sensor.has_propagation_speed_m_s {
            parsed.push(CoreSourceSensor::with_speed(
                position_m,
                sensor.propagation_speed_m_s,
            ));
        } else {
            parsed.push(CoreSourceSensor::new(position_m));
        }
    }
    Ok(parsed)
}

unsafe fn source_position_from_c(
    fn_name: &str,
    source_position_m: *const f64,
    dimension: usize,
) -> Result<Vec<f64>, SidereonStatus> {
    if !(2..=3).contains(&dimension) {
        set_last_error(format!("{fn_name}: source_dimension must be 2 or 3"));
        return Err(SidereonStatus::InvalidArgument);
    }
    Ok(require_slice(source_position_m, dimension, fn_name, "source_position_m")?.to_vec())
}

fn source_localization_node(kind: &str, fields: Value) -> Value {
    json!({"kind": kind, "fields": fields})
}

pub(crate) fn source_localization_error_value(error: &CoreSourceLocalizationError) -> Value {
    use CoreSourceLocalizationError as E;
    match error {
        E::InvalidInput { field, reason } => {
            source_localization_node("invalid_input", json!({"field": field, "reason": reason}))
        }
        E::TooFewSensors { sensors, needed } => source_localization_node(
            "too_few_sensors",
            json!({"sensors": sensors, "needed": needed}),
        ),
        E::InitializerSingular => source_localization_node("initializer_singular", json!({})),
        E::Geometry(err) => {
            source_localization_node("geometry", json!({"cause": dop_error_value(err)}))
        }
        E::Solver(err) => {
            source_localization_node("solver", json!({"cause": trls_error_value(err)}))
        }
        E::DidNotConverge { status } => {
            source_localization_node("did_not_converge", json!({"status": status}))
        }
    }
}

fn map_source_localization_error_retaining(
    fn_name: &str,
    err: CoreSourceLocalizationError,
) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        CoreSourceLocalizationError::InvalidInput { .. }
        | CoreSourceLocalizationError::TooFewSensors { .. } => SidereonStatus::InvalidArgument,
        CoreSourceLocalizationError::Geometry(DopError::InvalidInput { .. }) => {
            SidereonStatus::InvalidArgument
        }
        CoreSourceLocalizationError::InitializerSingular
        | CoreSourceLocalizationError::Geometry(_)
        | CoreSourceLocalizationError::Solver(_)
        | CoreSourceLocalizationError::DidNotConverge { .. } => SidereonStatus::Solve,
    }
}

fn map_source_localization_error(
    fn_name: &str,
    err: CoreSourceLocalizationError,
) -> SidereonStatus {
    record_engine_error(
        SidereonEngineErrorFamily::SourceLocalization,
        fn_name,
        source_localization_error_value(&err),
    );
    map_source_localization_error_retaining(fn_name, err)
}

fn source_initial_guess_to_c(initial: &CoreSourceInitialGuess) -> SidereonSourceInitialGuess {
    SidereonSourceInitialGuess {
        dimension: initial.position_m.len().min(3),
        position_m: zero_vec3_from_slice(&initial.position_m),
        has_origin_time_s: initial.origin_time_s.is_some(),
        origin_time_s: initial.origin_time_s.unwrap_or(0.0),
        residual_rms_s: initial.residual_rms_s,
    }
}

fn source_loss_from_c(
    fn_name: &str,
    arg_name: &str,
    loss: u32,
) -> Result<SourceLossInner, SidereonStatus> {
    match loss {
        value if value == SidereonSourceLoss::Linear as u32 => Ok(SourceLossInner::Linear),
        value if value == SidereonSourceLoss::SoftL1 as u32 => Ok(SourceLossInner::SoftL1),
        value if value == SidereonSourceLoss::Huber as u32 => Ok(SourceLossInner::Huber),
        value if value == SidereonSourceLoss::Cauchy as u32 => Ok(SourceLossInner::Cauchy),
        value if value == SidereonSourceLoss::Arctan as u32 => Ok(SourceLossInner::Arctan),
        _ => {
            set_last_error(format!("{fn_name}: invalid {arg_name} source loss"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn source_solve_mode_from_c(
    fn_name: &str,
    mode: u32,
    reference_sensor: usize,
) -> Result<CoreSourceSolveMode, SidereonStatus> {
    match mode {
        value if value == SidereonSourceSolveMode::Toa as u32 => Ok(CoreSourceSolveMode::Toa),
        value if value == SidereonSourceSolveMode::Tdoa as u32 => {
            Ok(CoreSourceSolveMode::Tdoa { reference_sensor })
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid source solve mode"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_error::{
        clear_engine_error, sidereon_last_engine_error_info, sidereon_last_engine_error_payload,
        SidereonEngineErrorFamily, SidereonEngineErrorInfo,
    };
    use std::mem::MaybeUninit;

    fn arrivals(
        sensors: &[SidereonSourceSensor],
        source_m: &[f64],
        origin_time_s: f64,
        propagation_speed_m_s: f64,
    ) -> Vec<f64> {
        sensors
            .iter()
            .map(|sensor| {
                let distance_m = sensor.position_m[..sensor.dimension]
                    .iter()
                    .zip(source_m)
                    .map(|(&sensor_coordinate, &source_coordinate)| {
                        (source_coordinate - sensor_coordinate).powi(2)
                    })
                    .sum::<f64>()
                    .sqrt();
                origin_time_s + distance_m / propagation_speed_m_s
            })
            .collect()
    }

    fn default_options() -> SidereonSourceLocateOptions {
        let mut options = MaybeUninit::uninit();
        assert_eq!(
            unsafe { sidereon_source_locate_options_init(options.as_mut_ptr()) },
            SidereonStatus::Ok
        );
        unsafe { options.assume_init() }
    }

    fn solution_summary(solution: *const SidereonSourceSolution) -> SidereonSourceSolutionSummary {
        let mut summary = MaybeUninit::uninit();
        assert_eq!(
            unsafe { sidereon_source_solution_summary(solution, summary.as_mut_ptr()) },
            SidereonStatus::Ok
        );
        unsafe { summary.assume_init() }
    }

    fn assert_position_and_origin_bits_equal(
        actual: &SidereonSourceSolutionSummary,
        expected: &SidereonSourceSolutionSummary,
    ) {
        assert_eq!(actual.dimension, expected.dimension);
        for (actual, expected) in actual.position_m.iter().zip(expected.position_m) {
            assert_eq!(actual.to_bits(), expected.to_bits());
        }
        assert_eq!(actual.has_origin_time_s, expected.has_origin_time_s);
        assert_eq!(
            actual.origin_time_s.to_bits(),
            expected.origin_time_s.to_bits()
        );
    }

    #[test]
    fn locate_source_with_controls_influence_without_changing_solution_bits() {
        let sensors = [
            SidereonSourceSensor {
                dimension: 3,
                position_m: [0.0, 0.0, 0.0],
                has_propagation_speed_m_s: false,
                propagation_speed_m_s: 0.0,
            },
            SidereonSourceSensor {
                dimension: 3,
                position_m: [1200.0, 0.0, 0.0],
                has_propagation_speed_m_s: false,
                propagation_speed_m_s: 0.0,
            },
            SidereonSourceSensor {
                dimension: 3,
                position_m: [0.0, 900.0, 0.0],
                has_propagation_speed_m_s: false,
                propagation_speed_m_s: 0.0,
            },
            SidereonSourceSensor {
                dimension: 3,
                position_m: [0.0, 0.0, 700.0],
                has_propagation_speed_m_s: false,
                propagation_speed_m_s: 0.0,
            },
            SidereonSourceSensor {
                dimension: 3,
                position_m: [1100.0, 800.0, 600.0],
                has_propagation_speed_m_s: false,
                propagation_speed_m_s: 0.0,
            },
        ];
        let speed = 343.0;
        let mut times = arrivals(&sensors, &[320.0, 260.0, 180.0], 12.5, speed);
        for (time, noise) in times
            .iter_mut()
            .zip([0.00031, -0.00022, 0.00017, -0.00008, 0.00041])
        {
            *time += noise;
        }
        let mut options = default_options();
        options.timing_sigma_s = 0.001;

        let mut legacy_solution = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_locate_source(
                    sensors.as_ptr(),
                    sensors.len(),
                    times.as_ptr(),
                    speed,
                    &options,
                    &mut legacy_solution,
                )
            },
            SidereonStatus::Ok
        );
        let mut explicit_solution = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_locate_source_with(
                    sensors.as_ptr(),
                    sensors.len(),
                    times.as_ptr(),
                    speed,
                    &options,
                    true,
                    &mut explicit_solution,
                )
            },
            SidereonStatus::Ok
        );
        let mut lean_solution = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_locate_source_with(
                    sensors.as_ptr(),
                    sensors.len(),
                    times.as_ptr(),
                    speed,
                    &options,
                    false,
                    &mut lean_solution,
                )
            },
            SidereonStatus::Ok
        );

        // sidereon-core's own solves of the same inputs, with and without
        // the influence diagnostics.
        let core_sensors =
            unsafe { source_sensors_from_c("test", sensors.as_ptr(), sensors.len()) }
                .expect("sensors");
        let core_solve = |include_influence: bool| {
            let mut config = CoreSourceLocateConfig::default();
            config.options = source_options_from_c("test", &options).expect("options");
            config.include_influence = include_influence;
            core_locate_source_with(&core_sensors, &times, speed, &config).expect("core solve")
        };
        let core_with = core_solve(true);
        let core_lean = core_solve(false);
        let legacy_summary = solution_summary(legacy_solution);
        let explicit_summary = solution_summary(explicit_solution);
        let lean_summary = solution_summary(lean_solution);
        assert_eq!(
            legacy_summary.influence_count,
            core_with.per_sensor_influence.len()
        );
        assert_eq!(
            explicit_summary.influence_count,
            core_with.per_sensor_influence.len()
        );
        assert_eq!(
            lean_summary.influence_count,
            core_lean.per_sensor_influence.len()
        );
        assert_position_and_origin_bits_equal(&explicit_summary, &legacy_summary);
        assert_position_and_origin_bits_equal(&lean_summary, &legacy_summary);

        let mut written = usize::MAX;
        let mut required = usize::MAX;
        assert_eq!(
            unsafe {
                sidereon_source_solution_influences(
                    lean_solution,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(
            (written, required),
            (0, core_lean.per_sensor_influence.len())
        );

        unsafe {
            sidereon_source_solution_free(lean_solution);
            sidereon_source_solution_free(explicit_solution);
            sidereon_source_solution_free(legacy_solution);
        }
    }

    #[test]
    fn initializer_exports_agree_on_clean_2d_toa_seed() {
        let sensors = [
            SidereonSourceSensor {
                dimension: 2,
                position_m: [0.0, 0.0, 0.0],
                has_propagation_speed_m_s: false,
                propagation_speed_m_s: 0.0,
            },
            SidereonSourceSensor {
                dimension: 2,
                position_m: [700.0, 0.0, 0.0],
                has_propagation_speed_m_s: false,
                propagation_speed_m_s: 0.0,
            },
            SidereonSourceSensor {
                dimension: 2,
                position_m: [0.0, 600.0, 0.0],
                has_propagation_speed_m_s: false,
                propagation_speed_m_s: 0.0,
            },
            SidereonSourceSensor {
                dimension: 2,
                position_m: [650.0, 550.0, 0.0],
                has_propagation_speed_m_s: false,
                propagation_speed_m_s: 0.0,
            },
        ];
        let times = arrivals(&sensors, &[210.0, 170.0], 2.75, 343.0);
        let mut closed_form = MaybeUninit::uninit();
        assert_eq!(
            unsafe {
                sidereon_closed_form_initial_guess(
                    sensors.as_ptr(),
                    sensors.len(),
                    times.as_ptr(),
                    343.0,
                    SidereonSourceSolveMode::Toa as u32,
                    0,
                    closed_form.as_mut_ptr(),
                )
            },
            SidereonStatus::Ok
        );
        let closed_form = unsafe { closed_form.assume_init() };
        let mut deprecated = MaybeUninit::uninit();
        assert_eq!(
            unsafe {
                sidereon_chan_ho_initial_guess(
                    sensors.as_ptr(),
                    sensors.len(),
                    times.as_ptr(),
                    343.0,
                    SidereonSourceSolveMode::Toa as u32,
                    0,
                    deprecated.as_mut_ptr(),
                )
            },
            SidereonStatus::Ok
        );
        let deprecated = unsafe { deprecated.assume_init() };
        // sidereon-core's own closed-form seed of the same arrivals.
        let core_sensors =
            unsafe { source_sensors_from_c("test", sensors.as_ptr(), sensors.len()) }
                .expect("sensors");
        let core_mode =
            source_solve_mode_from_c("test", SidereonSourceSolveMode::Toa as u32, 0).expect("mode");
        let expected = source_initial_guess_to_c(
            &core_closed_form_initial_guess(&core_sensors, &times, 343.0, core_mode)
                .expect("core closed form"),
        );
        assert_eq!(closed_form.dimension, expected.dimension);
        assert_eq!(closed_form.has_origin_time_s, expected.has_origin_time_s);
        for (got, want) in closed_form.position_m.iter().zip(expected.position_m) {
            assert_eq!(got.to_bits(), want.to_bits());
        }
        assert_eq!(
            closed_form.origin_time_s.to_bits(),
            expected.origin_time_s.to_bits()
        );
        assert_eq!(
            closed_form.residual_rms_s.to_bits(),
            expected.residual_rms_s.to_bits()
        );

        // The seed recovers the source and origin time the arrivals were
        // formed from.
        assert!((closed_form.position_m[0] - 210.0).abs() < 1.0e-8);
        assert!((closed_form.position_m[1] - 170.0).abs() < 1.0e-8);
        assert!((closed_form.origin_time_s - 2.75).abs() < 1.0e-10);
        assert_eq!(closed_form.dimension, deprecated.dimension);
        assert_eq!(closed_form.has_origin_time_s, deprecated.has_origin_time_s);
        for (closed_form, deprecated) in closed_form.position_m.iter().zip(deprecated.position_m) {
            assert_eq!(closed_form.to_bits(), deprecated.to_bits());
        }
        assert_eq!(
            closed_form.origin_time_s.to_bits(),
            deprecated.origin_time_s.to_bits()
        );
        assert_eq!(
            closed_form.residual_rms_s.to_bits(),
            deprecated.residual_rms_s.to_bits()
        );
    }

    #[test]
    fn table_driven_source_localization_error_mapping() {
        use sidereon_core::dop::DopError;
        use trust_region_least_squares::trf::{BackendError, TrfError};

        let cases: Vec<(CoreSourceLocalizationError, &'static str)> = vec![
            (
                CoreSourceLocalizationError::InvalidInput {
                    field: "propagation_speed_m_s",
                    reason: "must be positive and finite",
                },
                "invalid_input",
            ),
            (
                CoreSourceLocalizationError::TooFewSensors {
                    sensors: 2,
                    needed: 4,
                },
                "too_few_sensors",
            ),
            (
                CoreSourceLocalizationError::InitializerSingular,
                "initializer_singular",
            ),
            (
                CoreSourceLocalizationError::Geometry(DopError::InvalidInput {
                    field: "source_position_m",
                    reason: "coordinates must be finite",
                }),
                "geometry",
            ),
            (
                CoreSourceLocalizationError::Geometry(DopError::TooFewSatellites),
                "geometry",
            ),
            (
                CoreSourceLocalizationError::Geometry(DopError::Singular),
                "geometry",
            ),
            (
                CoreSourceLocalizationError::Solver(TrfError::EmptyResidual),
                "solver",
            ),
            (
                CoreSourceLocalizationError::Solver(TrfError::InsufficientRows { m: 1, n: 3 }),
                "solver",
            ),
            (
                CoreSourceLocalizationError::Solver(TrfError::Backend(BackendError::Failed(
                    "backend divergence".into(),
                ))),
                "solver",
            ),
            (
                CoreSourceLocalizationError::DidNotConverge { status: -1 },
                "did_not_converge",
            ),
        ];

        for (err, expected_kind) in cases {
            let val = source_localization_error_value(&err);
            assert_eq!(val["kind"], expected_kind);
            match &err {
                CoreSourceLocalizationError::InvalidInput { field, reason } => {
                    assert_eq!(val["fields"]["field"], *field);
                    assert_eq!(val["fields"]["reason"], *reason);
                }
                CoreSourceLocalizationError::TooFewSensors { sensors, needed } => {
                    assert_eq!(val["fields"]["sensors"], *sensors);
                    assert_eq!(val["fields"]["needed"], *needed);
                }
                CoreSourceLocalizationError::InitializerSingular => {
                    assert_eq!(val["fields"], json!({}));
                }
                CoreSourceLocalizationError::Geometry(dop_err) => match dop_err {
                    DopError::InvalidInput { field, reason } => {
                        assert_eq!(val["fields"]["cause"]["kind"], "invalid_input");
                        assert_eq!(val["fields"]["cause"]["fields"]["field"], *field);
                        assert_eq!(val["fields"]["cause"]["fields"]["reason"], *reason);
                    }
                    DopError::TooFewSatellites => {
                        assert_eq!(val["fields"]["cause"]["kind"], "too_few_satellites");
                    }
                    DopError::Singular => {
                        assert_eq!(val["fields"]["cause"]["kind"], "singular");
                    }
                },
                CoreSourceLocalizationError::Solver(trf_err) => match trf_err {
                    TrfError::EmptyResidual => {
                        assert_eq!(val["fields"]["cause"]["kind"], "empty_residual");
                    }
                    TrfError::InsufficientRows { m, n } => {
                        assert_eq!(val["fields"]["cause"]["kind"], "insufficient_rows");
                        assert_eq!(val["fields"]["cause"]["fields"]["m"], *m);
                        assert_eq!(val["fields"]["cause"]["fields"]["n"], *n);
                    }
                    TrfError::Backend(_) => {
                        assert_eq!(val["fields"]["cause"]["kind"], "backend");
                    }
                    _ => {}
                },
                CoreSourceLocalizationError::DidNotConverge { status } => {
                    assert_eq!(val["fields"]["status"], *status);
                }
            }
        }
    }

    #[test]
    fn source_localization_public_producer_control_and_refusals() {
        clear_engine_error();

        let sensors = [
            SidereonSourceSensor {
                dimension: 3,
                position_m: [0.0, 0.0, 0.0],
                has_propagation_speed_m_s: false,
                propagation_speed_m_s: 0.0,
            },
            SidereonSourceSensor {
                dimension: 3,
                position_m: [1200.0, 0.0, 0.0],
                has_propagation_speed_m_s: false,
                propagation_speed_m_s: 0.0,
            },
            SidereonSourceSensor {
                dimension: 3,
                position_m: [0.0, 900.0, 0.0],
                has_propagation_speed_m_s: false,
                propagation_speed_m_s: 0.0,
            },
            SidereonSourceSensor {
                dimension: 3,
                position_m: [0.0, 0.0, 700.0],
                has_propagation_speed_m_s: false,
                propagation_speed_m_s: 0.0,
            },
            SidereonSourceSensor {
                dimension: 3,
                position_m: [1100.0, 800.0, 600.0],
                has_propagation_speed_m_s: false,
                propagation_speed_m_s: 0.0,
            },
        ];
        let speed = 343.0;
        let times = arrivals(&sensors, &[320.0, 260.0, 180.0], 12.5, speed);
        let options = default_options();

        // 1. Valid producer control
        let mut solution = ptr::null_mut();
        unsafe {
            assert_eq!(
                sidereon_locate_source_with(
                    sensors.as_ptr(),
                    sensors.len(),
                    times.as_ptr(),
                    speed,
                    &options,
                    false,
                    &mut solution,
                ),
                SidereonStatus::Ok
            );
            assert!(!solution.is_null());

            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::SourceLocalization,
                payload_len: 123,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::None);
            assert_eq!(info.payload_len, 0);

            sidereon_source_solution_free(solution);
        }

        // 2. Real public refusal: too few sensors (2 sensors for 3D ToA requires at least 4)
        unsafe {
            let mut bad_solution = ptr::null_mut();
            assert_eq!(
                sidereon_locate_source_with(
                    sensors.as_ptr(),
                    2,
                    times.as_ptr(),
                    speed,
                    &options,
                    false,
                    &mut bad_solution,
                ),
                SidereonStatus::InvalidArgument
            );
            assert!(bad_solution.is_null());

            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::SourceLocalization);
            assert!(info.payload_len > 0);

            let mut written = 0;
            let mut required = 0;
            let mut buf = vec![0u8; info.payload_len];
            assert_eq!(
                sidereon_last_engine_error_payload(
                    buf.as_mut_ptr(),
                    buf.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, info.payload_len);
            let payload: Value = serde_json::from_slice(&buf).expect("valid JSON payload");
            assert_eq!(payload["schema_version"], 1);
            assert_eq!(payload["family"], "source_localization");
            assert_eq!(payload["operation"], "sidereon_locate_source_with");
            assert_eq!(payload["error"]["kind"], "too_few_sensors");
            assert_eq!(payload["error"]["fields"]["sensors"], 2);
            assert_eq!(payload["error"]["fields"]["needed"], 4);
        }
    }

    #[test]
    fn source_localization_producer_early_clearing_and_retention() {
        clear_engine_error();

        let sensors = [
            SidereonSourceSensor {
                dimension: 3,
                position_m: [0.0, 0.0, 0.0],
                has_propagation_speed_m_s: false,
                propagation_speed_m_s: 0.0,
            },
            SidereonSourceSensor {
                dimension: 3,
                position_m: [1200.0, 0.0, 0.0],
                has_propagation_speed_m_s: false,
                propagation_speed_m_s: 0.0,
            },
            SidereonSourceSensor {
                dimension: 3,
                position_m: [0.0, 900.0, 0.0],
                has_propagation_speed_m_s: false,
                propagation_speed_m_s: 0.0,
            },
            SidereonSourceSensor {
                dimension: 3,
                position_m: [0.0, 0.0, 700.0],
                has_propagation_speed_m_s: false,
                propagation_speed_m_s: 0.0,
            },
            SidereonSourceSensor {
                dimension: 3,
                position_m: [1100.0, 800.0, 600.0],
                has_propagation_speed_m_s: false,
                propagation_speed_m_s: 0.0,
            },
        ];
        let speed = 343.0;
        let times = arrivals(&sensors, &[320.0, 260.0, 180.0], 12.5, speed);
        let options = default_options();

        unsafe {
            // Seed retained engine error with actual refusal
            let mut bad_solution = ptr::null_mut();
            assert_eq!(
                sidereon_locate_source_with(
                    sensors.as_ptr(),
                    2,
                    times.as_ptr(),
                    speed,
                    &options,
                    false,
                    &mut bad_solution,
                ),
                SidereonStatus::InvalidArgument
            );

            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::SourceLocalization);
            let expected_len = info.payload_len;
            assert!(expected_len > 0);

            // Free retains
            sidereon_source_solution_free(ptr::null_mut());
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::SourceLocalization);
            assert_eq!(info.payload_len, expected_len);

            // Pass 1: query length retains
            let mut written = 999;
            let mut required = 0;
            assert_eq!(
                sidereon_last_engine_error_payload(ptr::null_mut(), 0, &mut written, &mut required),
                SidereonStatus::Ok
            );
            assert_eq!(written, 0);
            assert_eq!(required, expected_len);

            // Short buffer query returns InvalidArgument, writes 0, retains
            let mut short_buf = vec![0u8; expected_len - 1];
            assert_eq!(
                sidereon_last_engine_error_payload(
                    short_buf.as_mut_ptr(),
                    short_buf.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::InvalidArgument
            );
            assert_eq!(written, 0);
            assert_eq!(required, expected_len);

            // Pass 2: Exact buffer query succeeds and retains
            let mut buf = vec![0u8; expected_len];
            assert_eq!(
                sidereon_last_engine_error_payload(
                    buf.as_mut_ptr(),
                    buf.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, expected_len);
            assert_eq!(required, expected_len);

            // Retained after read
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::SourceLocalization);

            // Create a valid solution handle to test live handle retention
            let mut valid_solution = ptr::null_mut();
            assert_eq!(
                sidereon_locate_source_with(
                    sensors.as_ptr(),
                    sensors.len(),
                    times.as_ptr(),
                    speed,
                    &options,
                    true,
                    &mut valid_solution,
                ),
                SidereonStatus::Ok
            );
            assert!(!valid_solution.is_null());

            // Successful producer reset verified
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::None);
            assert_eq!(info.payload_len, 0);

            // Now seed a new real refusal while valid_solution is live
            assert_eq!(
                sidereon_locate_source_with(
                    sensors.as_ptr(),
                    2,
                    times.as_ptr(),
                    speed,
                    &options,
                    false,
                    &mut bad_solution,
                ),
                SidereonStatus::InvalidArgument
            );
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::SourceLocalization);
            let active_len = info.payload_len;

            // Capture payload before calling getters
            let mut payload_before = vec![0u8; active_len];
            assert_eq!(
                sidereon_last_engine_error_payload(
                    payload_before.as_mut_ptr(),
                    payload_before.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );

            // Call inspectors on live handle: summary, covariance, residuals, influences
            let mut summary = MaybeUninit::uninit();
            assert_eq!(
                sidereon_source_solution_summary(valid_solution, summary.as_mut_ptr()),
                SidereonStatus::Ok
            );

            let mut cov = MaybeUninit::uninit();
            let mut avail = false;
            assert_eq!(
                sidereon_source_solution_covariance(valid_solution, cov.as_mut_ptr(), &mut avail),
                SidereonStatus::Ok
            );
            assert!(avail);

            let mut res_written = 0;
            let mut res_required = 0;
            assert_eq!(
                sidereon_source_solution_residuals(
                    valid_solution,
                    ptr::null_mut(),
                    0,
                    &mut res_written,
                    &mut res_required,
                ),
                SidereonStatus::Ok
            );

            let mut inf_written = 0;
            let mut inf_required = 0;
            assert_eq!(
                sidereon_source_solution_influences(
                    valid_solution,
                    ptr::null_mut(),
                    0,
                    &mut inf_written,
                    &mut inf_required,
                ),
                SidereonStatus::Ok
            );

            // Verify payload is retained after getters
            let mut payload_after = vec![0u8; active_len];
            assert_eq!(
                sidereon_last_engine_error_payload(
                    payload_after.as_mut_ptr(),
                    payload_after.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(payload_before, payload_after);

            // Free valid solution and verify retention
            sidereon_source_solution_free(valid_solution);
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::SourceLocalization);
            assert_eq!(info.payload_len, active_len);

            // Producer early null reset
            assert_eq!(
                sidereon_locate_source_with(
                    sensors.as_ptr(),
                    sensors.len(),
                    times.as_ptr(),
                    speed,
                    &options,
                    false,
                    ptr::null_mut(),
                ),
                SidereonStatus::NullPointer
            );
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::None);
            assert_eq!(info.payload_len, 0);
        }
    }
}
