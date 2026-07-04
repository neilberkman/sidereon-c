use super::*;

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

/// Options for sidereon_locate_source. Initialize with
/// sidereon_source_locate_options_init for the core defaults.
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
    /// Normalized diagnostic score. Larger means poorer fit.
    pub score: f64,
}

/// Geometry and redundancy diagnostics for a source solve.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSourceGeometryQuality {
    /// Number of residual rows used by the solve.
    pub residual_count: usize,
    /// Number of estimated state parameters.
    pub parameter_count: usize,
    /// Residual count minus parameter count, saturated at zero.
    pub redundancy: usize,
    /// Whether covariance was available from the normal matrix.
    pub covariance_available: bool,
    /// Whether the final normal matrix was rank deficient.
    pub rank_deficient: bool,
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
    /// Geometry and redundancy diagnostics.
    pub geometry_quality: SidereonSourceGeometryQuality,
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
/// On success writes a newly owned handle to *out_solution.
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
    ffi_boundary("sidereon_locate_source", SidereonStatus::Panic, || {
        let out_solution = c_try!(require_out(
            out_solution,
            "sidereon_locate_source",
            "out_solution"
        ));
        *out_solution = ptr::null_mut();
        let parsed_sensors = c_try!(source_sensors_from_c(
            "sidereon_locate_source",
            sensors,
            sensor_count
        ));
        let arrivals = c_try!(require_slice(
            arrival_times_s,
            sensor_count,
            "sidereon_locate_source",
            "arrival_times_s"
        ));
        let options = c_try!(source_options_from_c("sidereon_locate_source", options));
        match core_locate_source(&parsed_sensors, arrivals, propagation_speed_m_s, &options) {
            Ok(inner) => {
                write_boxed_handle(out_solution, SidereonSourceSolution { inner });
                SidereonStatus::Ok
            }
            Err(err) => map_source_localization_error("sidereon_locate_source", err),
        }
    })
}

/// Compute the closed-form source-localization initial guess.
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
    ffi_boundary(
        "sidereon_chan_ho_initial_guess",
        SidereonStatus::Panic,
        || {
            let out_guess = c_try!(require_out(
                out_guess,
                "sidereon_chan_ho_initial_guess",
                "out_guess"
            ));
            *out_guess = SidereonSourceInitialGuess {
                dimension: 0,
                position_m: [0.0; 3],
                has_origin_time_s: false,
                origin_time_s: 0.0,
                residual_rms_s: 0.0,
            };
            let parsed_sensors = c_try!(source_sensors_from_c(
                "sidereon_chan_ho_initial_guess",
                sensors,
                sensor_count
            ));
            let arrivals = c_try!(require_slice(
                arrival_times_s,
                sensor_count,
                "sidereon_chan_ho_initial_guess",
                "arrival_times_s"
            ));
            let mode = c_try!(source_solve_mode_from_c(
                "sidereon_chan_ho_initial_guess",
                mode,
                reference_sensor
            ));
            match core_chan_ho_initial_guess(&parsed_sensors, arrivals, propagation_speed_m_s, mode)
            {
                Ok(guess) => {
                    *out_guess = source_initial_guess_to_c(&guess);
                    SidereonStatus::Ok
                }
                Err(err) => map_source_localization_error("sidereon_chan_ho_initial_guess", err),
            }
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
    ffi_boundary("sidereon_source_dop", SidereonStatus::Panic, || {
        let out_dop = c_try!(require_out(out_dop, "sidereon_source_dop", "out_dop"));
        *out_dop = empty_dop();
        let parsed_sensors = c_try!(source_sensors_from_c(
            "sidereon_source_dop",
            sensors,
            sensor_count
        ));
        let source_position = c_try!(source_position_from_c(
            "sidereon_source_dop",
            source_position_m,
            source_dimension
        ));
        match core_source_dop(&parsed_sensors, &source_position, propagation_speed_m_s) {
            Ok(dop) => {
                *out_dop = dop_to_c(dop);
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
    ffi_boundary("sidereon_source_crlb", SidereonStatus::Panic, || {
        let out_crlb = c_try!(require_out(out_crlb, "sidereon_source_crlb", "out_crlb"));
        *out_crlb = SidereonSourceCrlb {
            dop: empty_dop(),
            covariance: source_covariance_empty(),
        };
        let parsed_sensors = c_try!(source_sensors_from_c(
            "sidereon_source_crlb",
            sensors,
            sensor_count
        ));
        let source_position = c_try!(source_position_from_c(
            "sidereon_source_crlb",
            source_position_m,
            source_dimension
        ));
        match core_source_crlb(
            &parsed_sensors,
            &source_position,
            propagation_speed_m_s,
            timing_sigma_s,
        ) {
            Ok(CoreSourceCrlb { dop, covariance }) => {
                *out_crlb = SidereonSourceCrlb {
                    dop: dop_to_c(dop),
                    covariance: source_covariance_to_c(&covariance),
                };
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
            let solution = c_try!(require_ref(
                solution,
                "sidereon_source_solution_summary",
                "solution"
            ));
            let out_summary = c_try!(require_out(
                out_summary,
                "sidereon_source_solution_summary",
                "out_summary"
            ));
            *out_summary = source_summary_to_c(&solution.inner);
            SidereonStatus::Ok
        },
    )
}

/// Copy the source solution covariance when available.
///
/// Safety: solution must be a live handle; out_covariance and out_available
/// must point to writable values.
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
            let solution = c_try!(require_ref(
                solution,
                "sidereon_source_solution_covariance",
                "solution"
            ));
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
/// Safety: solution must be NULL or a live handle from sidereon_locate_source.
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
        geometry_quality: source_geometry_quality_to_c(&solution.geometry_quality),
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
    Ok(CoreSourceLocateOptions {
        mode: source_solve_mode_from_c(fn_name, options.mode, options.reference_sensor)?,
        timing_sigma_s: options.timing_sigma_s,
        loss: source_loss_from_c(fn_name, "options.loss", options.loss)?,
        f_scale_s: options.f_scale_s,
        ftol: options.has_ftol.then_some(options.ftol),
        xtol: options.has_xtol.then_some(options.xtol),
        gtol: options.has_gtol.then_some(options.gtol),
        max_nfev: options.has_max_nfev.then_some(options.max_nfev),
    })
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

fn map_source_localization_error(
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

fn source_initial_guess_to_c(initial: &CoreSourceInitialGuess) -> SidereonSourceInitialGuess {
    SidereonSourceInitialGuess {
        dimension: initial.position_m.len().min(3),
        position_m: zero_vec3_from_slice(&initial.position_m),
        has_origin_time_s: initial.origin_time_s.is_some(),
        origin_time_s: initial.origin_time_s.unwrap_or(0.0),
        residual_rms_s: initial.residual_rms_s,
    }
}

fn source_geometry_quality_to_c(
    quality: &CoreSourceGeometryQuality,
) -> SidereonSourceGeometryQuality {
    SidereonSourceGeometryQuality {
        residual_count: quality.residual_count,
        parameter_count: quality.parameter_count,
        redundancy: quality.redundancy,
        covariance_available: quality.covariance_available,
        rank_deficient: quality.rank_deficient,
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
