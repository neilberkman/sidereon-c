use super::*;
use crate::covariance::map_propagation_error;
use crate::engine_error::engine_error_operation_boundary;
use sidereon_core::astro::propagator::{
    propagate_variational, VariationalEpoch, VariationalEquations, VariationalState,
};

/// Numerical integrator and controls for state and variational propagation.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SidereonIntegratorOptions {
    /// Integrator as SidereonPropagationIntegrator.
    pub integrator: u32,
    /// Absolute error tolerance.
    pub abs_tol: f64,
    /// Relative error tolerance.
    pub rel_tol: f64,
    /// Initial step in seconds.
    pub initial_step_s: f64,
    /// Minimum step in seconds.
    pub min_step_s: f64,
    /// Maximum step in seconds.
    pub max_step_s: f64,
    /// Maximum integration steps.
    pub max_steps: u32,
}

impl SidereonIntegratorOptions {
    pub(crate) fn from_core(integrator: u32, options: &IntegratorOptions) -> Self {
        Self {
            integrator,
            abs_tol: options.abs_tol,
            rel_tol: options.rel_tol,
            initial_step_s: options.initial_step,
            min_step_s: options.min_step,
            max_step_s: options.max_step,
            max_steps: options.max_steps,
        }
    }

    pub(crate) fn to_core(self) -> IntegratorOptions {
        let mut options = IntegratorOptions::default();
        options.abs_tol = self.abs_tol;
        options.rel_tol = self.rel_tol;
        options.initial_step = self.initial_step_s;
        options.min_step = self.min_step_s;
        options.max_step = self.max_step_s;
        options.max_steps = self.max_steps;
        options.dense_output = false;
        options
    }
}

/// Default numerical integration options with Dormand–Prince 5(4).
#[no_mangle]
pub extern "C" fn sidereon_integrator_options_default() -> SidereonIntegratorOptions {
    SidereonIntegratorOptions::from_core(
        SidereonPropagationIntegrator::Dp54 as u32,
        &IntegratorOptions::default(),
    )
}

/// Force evaluation epoch and integration time. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SidereonVariationalEpoch {
    /// Absolute TDB seconds past J2000 for force evaluation.
    pub tdb_j2000_s: f64,
    /// Elapsed TT seconds from the initial state.
    pub elapsed_tt_s: f64,
}

impl From<SidereonVariationalEpoch> for VariationalEpoch {
    fn from(value: SidereonVariationalEpoch) -> Self {
        Self {
            tdb_j2000_s: value.tdb_j2000_s,
            elapsed_tt_s: value.elapsed_tt_s,
        }
    }
}

impl From<VariationalEpoch> for SidereonVariationalEpoch {
    fn from(value: VariationalEpoch) -> Self {
        Self {
            tdb_j2000_s: value.tdb_j2000_s,
            elapsed_tt_s: value.elapsed_tt_s,
        }
    }
}

/// Compute the elapsed TT seconds for the requested absolute TDB epoch.
#[no_mangle]
pub extern "C" fn sidereon_variational_epoch_from_tdb_seconds(
    initial_tdb_j2000_s: f64,
    tdb_j2000_s: f64,
) -> SidereonVariationalEpoch {
    VariationalEpoch::from_tdb_seconds(initial_tdb_j2000_s, tdb_j2000_s).into()
}

/// Force model, its parameter values, and the ordered sensitivity columns.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonVariationalEquations {
    /// Live force model handle.
    pub force: *const SidereonForceModel,
    /// Replacement parameter values in force parameter order.
    pub parameters: *const f64,
    /// Number of replacement parameters.
    pub parameter_count: usize,
    /// Indices of the parameters whose sensitivities are requested.
    pub sensitivity_parameters: *const usize,
    /// Number of sensitivity columns.
    pub sensitivity_count: usize,
}

/// Owned variational trajectory. Release with sidereon_variational_trajectory_free.
pub struct SidereonVariationalTrajectory {
    states: Vec<VariationalState>,
}

/// State and state-transition matrix at one variational output epoch.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonVariationalState {
    /// Cartesian state.
    pub state: SidereonCartesianState,
    /// State-transition matrix in row-major order.
    pub state_transition: [f64; 36],
    /// Number of columns in the six-row parameter-sensitivity matrix.
    pub sensitivity_columns: usize,
}

/// Integrate the state, state-transition matrix and requested sensitivities.
///
/// Safety: epochs points to epoch_count records. The equations' force handle
/// is live and its arrays have the declared lengths. context is NULL or
/// references live handles. out is disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_propagate_variational(
    initial: SidereonCartesianState,
    epochs: *const SidereonVariationalEpoch,
    epoch_count: usize,
    equations: SidereonVariationalEquations,
    options: SidereonIntegratorOptions,
    context: *const SidereonPropagationContext,
    out: *mut *mut SidereonVariationalTrajectory,
) -> SidereonStatus {
    const OP: &str = "sidereon_propagate_variational";
    engine_error_operation_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        c_try!(reject_output_overlaps_handle(
            OP,
            out,
            "out",
            equations.force,
            "force"
        ));
        out.write(ptr::null_mut());
        let force = c_try!(require_ref(equations.force, OP, "force"));
        let parameters = c_try!(require_slice(
            equations.parameters,
            equations.parameter_count,
            OP,
            "parameters"
        ));
        let sensitivity_parameters = c_try!(require_slice(
            equations.sensitivity_parameters,
            equations.sensitivity_count,
            OP,
            "sensitivity_parameters",
        ));
        let equations = VariationalEquations {
            force: force.inner.as_ref(),
            parameters,
            sensitivity_parameters,
        };
        let epochs = c_try!(require_slice(epochs, epoch_count, OP, "epochs"));
        let epochs: Vec<_> = epochs.iter().copied().map(VariationalEpoch::from).collect();
        let integrator = c_try!(propagation_integrator_from_c(OP, options.integrator));
        let context =
            crate::force_models::force_context_from_c(context, PropagationContext::default());
        let states = c_try!(propagate_variational(
            cartesian_state_from_c(&initial),
            &epochs,
            &equations,
            integrator,
            &options.to_core(),
            &context,
        )
        .map_err(|err| map_propagation_error(OP, err)));
        out.write(Box::into_raw(Box::new(SidereonVariationalTrajectory {
            states,
        })));
        SidereonStatus::Ok
    })
}

/// Release a variational trajectory.
///
/// Safety: trajectory is NULL or a live owned trajectory.
#[no_mangle]
pub unsafe extern "C" fn sidereon_variational_trajectory_free(
    trajectory: *mut SidereonVariationalTrajectory,
) {
    ffi_boundary("sidereon_variational_trajectory_free", (), || {
        free_boxed(trajectory)
    });
}

fn variational_state_at<'a>(
    operation: &str,
    trajectory: &'a SidereonVariationalTrajectory,
    index: usize,
) -> Result<&'a VariationalState, SidereonStatus> {
    trajectory.states.get(index).ok_or_else(|| {
        extra_invalid_arg(operation, format_args!("state index {index} out of range"))
    })
}

/// Copy the state and state-transition matrix at one trajectory index.
///
/// Safety: trajectory is live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_variational_trajectory_state(
    trajectory: *const SidereonVariationalTrajectory,
    index: usize,
    out: *mut SidereonVariationalState,
) -> SidereonStatus {
    const OP: &str = "sidereon_variational_trajectory_state";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        c_try!(reject_output_overlaps_handle(
            OP,
            out,
            "out",
            trajectory,
            "trajectory"
        ));
        let trajectory = c_try!(require_ref(trajectory, OP, "trajectory"));
        let value = c_try!(variational_state_at(OP, trajectory, index));
        out.write(SidereonVariationalState {
            state: cartesian_state_to_c(&value.state),
            state_transition: std::array::from_fn(|i| value.state_transition[(i / 6, i % 6)]),
            sensitivity_columns: value.sensitivity.ncols(),
        });
        SidereonStatus::Ok
    })
}

/// Copy the six-row sensitivity matrix in row-major order.
/// NULL and zero capacity query the full number of doubles.
///
/// Safety: trajectory is live. All outputs are writable and disjoint from
/// each other and the handle. out_matrix has capacity doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_variational_trajectory_sensitivity(
    trajectory: *const SidereonVariationalTrajectory,
    index: usize,
    out_matrix: *mut f64,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_variational_trajectory_sensitivity";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        c_try!(init_handle_copy_counts(
            OP,
            out_matrix,
            capacity,
            out_written,
            out_required,
            trajectory,
            "trajectory"
        ));
        let trajectory = c_try!(require_ref(trajectory, OP, "trajectory"));
        let value = c_try!(variational_state_at(OP, trajectory, index));
        let columns = value.sensitivity.ncols();
        let matrix: Vec<_> = (0..value.sensitivity.len())
            .map(|i| value.sensitivity[(i / columns, i % columns)])
            .collect();
        c_try!(copy_prefix_to_c(
            OP,
            "out_matrix",
            &matrix,
            out_matrix,
            capacity,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Count the states in a variational trajectory.
///
/// Safety: trajectory is live and out points to disjoint writable count storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_variational_trajectory_count(
    trajectory: *const SidereonVariationalTrajectory,
    out: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_variational_trajectory_count";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        c_try!(reject_output_overlaps_handle(
            OP,
            out,
            "out",
            trajectory,
            "trajectory"
        ));
        let trajectory = c_try!(require_ref(trajectory, OP, "trajectory"));
        out.write(trajectory.states.len());
        SidereonStatus::Ok
    })
}
