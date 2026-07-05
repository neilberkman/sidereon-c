use super::*;

/// A multi-epoch static pseudorange position solution. Opaque to C. Create with
/// sidereon_solve_static_position_sp3 or sidereon_solve_static_position_broadcast
/// and release with sidereon_static_position_solution_free.
pub struct SidereonStaticPositionSolution {
    pub(crate) inner: CoreStaticSolution,
}

/// One static-position epoch, expressed with the existing SPP V2 input bundle.
#[repr(C)]
pub struct SidereonStaticPositionEpoch {
    /// Single-epoch SPP fields for this receive epoch.
    pub inputs: SidereonSppInputsV2,
    /// Optional positive measurement-weight multipliers aligned with
    /// inputs.base.observations.
    pub weights: *const f64,
    /// Number of entries in weights. Zero means no caller multipliers.
    pub weight_count: usize,
}

/// Static-position solver options. Initialize with
/// sidereon_static_position_options_init for engine defaults.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonStaticPositionOptions {
    /// Initial shared receiver ECEF position in meters.
    pub initial_position_m: [f64; 3],
    /// Whether to include the geodetic position in the result.
    pub with_geodetic: bool,
    /// Whether robust Huber/IRLS reweighting is enabled.
    pub robust_enabled: bool,
    /// Robust reweighting controls, used only when robust_enabled is true.
    pub robust: SidereonSppRobustConfig,
}

/// Static-position solve error category returned through out_error fields.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonStaticPositionErrorKind {
    /// No static-position error occurred.
    None = 0,
    /// No epochs were supplied.
    EmptyEpochs = 1,
    /// A public static solve input was malformed.
    InvalidInput = 2,
    /// A per-epoch SPP input was malformed.
    EpochInput = 3,
    /// The same satellite appeared twice in one epoch.
    DuplicateObservation = 4,
    /// An ionosphere-corrected epoch used a satellite without a carrier model.
    IonosphereUnsupported = 5,
    /// Too few accepted measurements remained for the stacked state.
    TooFewMeasurements = 6,
    /// A satellite lost ephemeris during the solve.
    EphemerisLost = 7,
    /// The stacked design was rank deficient.
    Singular = 8,
}

/// One solved epoch-local receiver clock.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonStaticPositionClockBias {
    /// Epoch index in the input slice.
    pub epoch_index: usize,
    /// GNSS system.
    pub system: SidereonGnssSystem,
    /// Receiver clock bias in seconds.
    pub clock_s: f64,
}

/// One post-fit residual from a static-position solve.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonStaticPositionResidual {
    /// Epoch index in the input slice.
    pub epoch_index: usize,
    /// Satellite token.
    pub sat_id: SidereonSatelliteToken,
    /// Unweighted observed-minus-computed pseudorange residual, meters.
    pub residual_m: f64,
    /// Base row weight before robust reweighting.
    pub base_weight: f64,
    /// Final row weight after robust reweighting.
    pub effective_weight: f64,
    /// Ratio effective_weight/base_weight.
    pub robust_weight_ratio: f64,
}

/// Static-position solve metadata.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonStaticPositionMetadata {
    /// Number of accepted trust-region iterations in the final inner solve.
    pub iterations: usize,
    /// Whether the final inner solve reached a convergence criterion.
    pub converged: bool,
    /// Final inner solver termination status.
    pub status: SidereonSppSolveStatus,
    /// Number of robust outer iterations performed.
    pub outer_iterations: usize,
    /// Whether final_robust_scale_m is present.
    pub has_final_robust_scale_m: bool,
    /// Final MAD robust scale in meters when present.
    pub final_robust_scale_m: f64,
    /// Number of measurements used by the final solve.
    pub used_measurements: usize,
    /// Number of fitted state parameters.
    pub n_parameters: usize,
    /// Degrees of freedom.
    pub redundancy: i64,
    /// Geometry observability and covariance-validation diagnostics.
    pub geometry_quality: SidereonGeometryQuality,
}

/// Status for a leave-one-out diagnostic solve.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonStaticPositionInfluenceStatus {
    /// The diagnostic solve completed.
    Solved = 0,
    /// The omitted data left too few measurements.
    TooFewMeasurements = 1,
    /// The omitted data left rank-deficient geometry.
    SingularGeometry = 2,
    /// Input validation failed for the diagnostic subset.
    InvalidInput = 3,
    /// Ephemeris was unavailable for the diagnostic subset.
    EphemerisUnavailable = 4,
    /// The diagnostic subset failed for another solve reason.
    SolveFailed = 5,
}

/// Leave-one-epoch-out diagnostic.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonStaticPositionEpochInfluence {
    /// Epoch index omitted from the diagnostic solve.
    pub epoch_index: usize,
    /// Number of measurements omitted.
    pub omitted_measurements: usize,
    /// Diagnostic status.
    pub status: SidereonStaticPositionInfluenceStatus,
    /// Whether position_delta_m and position_delta_norm_m are present.
    pub has_position_delta_m: bool,
    /// Difference diagnostic_position minus full_position, ECEF meters.
    pub position_delta_m: [f64; 3],
    /// Norm of position_delta_m, meters.
    pub position_delta_norm_m: f64,
    /// Whether residual_rms_m is present.
    pub has_residual_rms_m: bool,
    /// Diagnostic residual RMS, meters.
    pub residual_rms_m: f64,
    /// Minimum robust weight ratio among this epoch's full-solve rows.
    pub min_robust_weight_ratio: f64,
}

/// Leave-one-satellite-out diagnostic for one epoch.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonStaticPositionSatelliteInfluence {
    /// Epoch index containing the omitted satellite.
    pub epoch_index: usize,
    /// Omitted satellite token.
    pub sat_id: SidereonSatelliteToken,
    /// Diagnostic status.
    pub status: SidereonStaticPositionInfluenceStatus,
    /// Whether position_delta_m and position_delta_norm_m are present.
    pub has_position_delta_m: bool,
    /// Difference diagnostic_position minus full_position, ECEF meters.
    pub position_delta_m: [f64; 3],
    /// Norm of position_delta_m, meters.
    pub position_delta_norm_m: f64,
    /// Whether residual_rms_m is present.
    pub has_residual_rms_m: bool,
    /// Diagnostic residual RMS, meters.
    pub residual_rms_m: f64,
    /// Full-solve residual for this satellite, meters.
    pub residual_m: f64,
    /// Base row weight before robust reweighting.
    pub base_weight: f64,
    /// Final row weight after robust reweighting.
    pub effective_weight: f64,
    /// Ratio effective_weight/base_weight.
    pub robust_weight_ratio: f64,
}

/// Leave-one-satellite-out diagnostic across all epochs where a satellite appears.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonStaticPositionSatelliteBatchInfluence {
    /// Omitted satellite token.
    pub sat_id: SidereonSatelliteToken,
    /// Number of measurements omitted across the static batch.
    pub omitted_measurements: usize,
    /// Diagnostic status.
    pub status: SidereonStaticPositionInfluenceStatus,
    /// Whether position_delta_m and position_delta_norm_m are present.
    pub has_position_delta_m: bool,
    /// Difference diagnostic_position minus full_position, ECEF meters.
    pub position_delta_m: [f64; 3],
    /// Norm of position_delta_m, meters.
    pub position_delta_norm_m: f64,
    /// Whether residual_rms_m is present.
    pub has_residual_rms_m: bool,
    /// Diagnostic residual RMS, meters.
    pub residual_rms_m: f64,
    /// Minimum robust weight ratio among this satellite's full-solve rows.
    pub min_robust_weight_ratio: f64,
}

/// Populate *out_options with static-position engine defaults.
///
/// Safety: out_options must point to a SidereonStaticPositionOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_static_position_options_init(
    out_options: *mut SidereonStaticPositionOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_static_position_options_init",
        SidereonStatus::Panic,
        || {
            let out_options = c_try!(require_out(
                out_options,
                "sidereon_static_position_options_init",
                "out_options"
            ));
            let defaults = CoreStaticSolveOptions::default();
            *out_options = SidereonStaticPositionOptions {
                initial_position_m: defaults.initial_position_m,
                with_geodetic: defaults.with_geodetic,
                robust_enabled: false,
                robust: default_robust_config(),
            };
            SidereonStatus::Ok
        },
    )
}

/// Solve a static multi-epoch pseudorange position from a loaded SP3 source.
///
/// Safety: sp3 must be a live handle; epochs points to epoch_count epoch
/// structs or is NULL when epoch_count is 0; options may be NULL for defaults;
/// out_error and out_solution must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solve_static_position_sp3(
    sp3: *const SidereonSp3,
    epochs: *const SidereonStaticPositionEpoch,
    epoch_count: usize,
    options: *const SidereonStaticPositionOptions,
    out_error: *mut SidereonStaticPositionErrorKind,
    out_solution: *mut *mut SidereonStaticPositionSolution,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_solve_static_position_sp3",
        SidereonStatus::Panic,
        || {
            let sp3 = c_try!(require_ref(
                sp3,
                "sidereon_solve_static_position_sp3",
                "sp3"
            ));
            solve_static_position_common(
                "sidereon_solve_static_position_sp3",
                &sp3.inner,
                epochs,
                epoch_count,
                options,
                out_error,
                out_solution,
            )
        },
    )
}

/// Solve a static multi-epoch pseudorange position from broadcast ephemeris.
///
/// Safety: broadcast must be a live handle; epochs points to epoch_count epoch
/// structs or is NULL when epoch_count is 0; options may be NULL for defaults;
/// out_error and out_solution must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solve_static_position_broadcast(
    broadcast: *const SidereonBroadcastEphemeris,
    epochs: *const SidereonStaticPositionEpoch,
    epoch_count: usize,
    options: *const SidereonStaticPositionOptions,
    out_error: *mut SidereonStaticPositionErrorKind,
    out_solution: *mut *mut SidereonStaticPositionSolution,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_solve_static_position_broadcast",
        SidereonStatus::Panic,
        || {
            let broadcast = c_try!(require_ref(
                broadcast,
                "sidereon_solve_static_position_broadcast",
                "broadcast"
            ));
            solve_static_position_common(
                "sidereon_solve_static_position_broadcast",
                &broadcast.inner,
                epochs,
                epoch_count,
                options,
                out_error,
                out_solution,
            )
        },
    )
}

/// Copy the static receiver ECEF position [x_m, y_m, z_m] into out_xyz.
///
/// Safety: solution must be a live handle; out_xyz must point to len writable
/// doubles and len must be at least 3.
#[no_mangle]
pub unsafe extern "C" fn sidereon_static_position_solution_position(
    solution: *const SidereonStaticPositionSolution,
    out_xyz: *mut f64,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_static_position_solution_position",
        SidereonStatus::Panic,
        || {
            let solution = c_try!(require_ref(
                solution,
                "sidereon_static_position_solution_position",
                "solution"
            ));
            let p = solution.inner.position.as_array();
            c_try!(copy_exact_f64s(
                "sidereon_static_position_solution_position",
                "out_xyz",
                out_xyz,
                len,
                &p,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the optional geodetic receiver position and set *out_present.
///
/// Safety: solution must be a live handle; out_geodetic and out_present must
/// point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_static_position_solution_geodetic(
    solution: *const SidereonStaticPositionSolution,
    out_geodetic: *mut SidereonGeodetic,
    out_present: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_static_position_solution_geodetic",
        SidereonStatus::Panic,
        || {
            let out_geodetic = c_try!(require_out(
                out_geodetic,
                "sidereon_static_position_solution_geodetic",
                "out_geodetic"
            ));
            *out_geodetic = empty_geodetic();
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_static_position_solution_geodetic",
                "out_present"
            ));
            *out_present = false;
            let solution = c_try!(require_ref(
                solution,
                "sidereon_static_position_solution_geodetic",
                "solution"
            ));
            if let Some(geodetic) = solution.inner.geodetic {
                *out_geodetic = geodetic_to_c(&geodetic);
                *out_present = true;
            }
            SidereonStatus::Ok
        },
    )
}

/// Copy static-position solve metadata.
///
/// Safety: solution must be a live handle; out_metadata must point to writable
/// storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_static_position_solution_metadata(
    solution: *const SidereonStaticPositionSolution,
    out_metadata: *mut SidereonStaticPositionMetadata,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_static_position_solution_metadata",
        SidereonStatus::Panic,
        || {
            let out_metadata = c_try!(require_out(
                out_metadata,
                "sidereon_static_position_solution_metadata",
                "out_metadata"
            ));
            *out_metadata = empty_static_position_metadata();
            let solution = c_try!(require_ref(
                solution,
                "sidereon_static_position_solution_metadata",
                "solution"
            ));
            *out_metadata =
                static_metadata_to_c(&solution.inner.metadata, &solution.inner.geometry_quality);
            SidereonStatus::Ok
        },
    )
}

/// Copy the 3x3 ECEF position covariance in row-major order.
///
/// Safety: solution must be a live handle; out_m2 must point to len writable
/// doubles and len must be at least 9.
#[no_mangle]
pub unsafe extern "C" fn sidereon_static_position_solution_position_covariance_ecef_m2(
    solution: *const SidereonStaticPositionSolution,
    out_m2: *mut f64,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_static_position_solution_position_covariance_ecef_m2",
        SidereonStatus::Panic,
        || {
            let solution = c_try!(require_ref(
                solution,
                "sidereon_static_position_solution_position_covariance_ecef_m2",
                "solution"
            ));
            let values = flatten_mat3(solution.inner.covariance.position_ecef_m2);
            c_try!(copy_exact_f64s(
                "sidereon_static_position_solution_position_covariance_ecef_m2",
                "out_m2",
                out_m2,
                len,
                &values,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the 3x3 ENU position covariance in row-major order.
///
/// Safety: solution must be a live handle; out_m2 must point to len writable
/// doubles and len must be at least 9.
#[no_mangle]
pub unsafe extern "C" fn sidereon_static_position_solution_position_covariance_enu_m2(
    solution: *const SidereonStaticPositionSolution,
    out_m2: *mut f64,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_static_position_solution_position_covariance_enu_m2",
        SidereonStatus::Panic,
        || {
            let solution = c_try!(require_ref(
                solution,
                "sidereon_static_position_solution_position_covariance_enu_m2",
                "solution"
            ));
            let values = flatten_mat3(solution.inner.covariance.position_enu_m2);
            c_try!(copy_exact_f64s(
                "sidereon_static_position_solution_position_covariance_enu_m2",
                "out_m2",
                out_m2,
                len,
                &values,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the full static state covariance in row-major order. Output uses the
/// variable-length contract documented in the header.
///
/// Safety: solution must be a live handle; out must point to len doubles or be
/// NULL when len is 0; out_written and out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_static_position_solution_state_covariance_m2(
    solution: *const SidereonStaticPositionSolution,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_static_position_solution_state_covariance_m2",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_static_position_solution_state_covariance_m2",
                out_written,
                out_required
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_static_position_solution_state_covariance_m2",
                "solution"
            ));
            let values: Vec<f64> = solution
                .inner
                .covariance
                .state_m2
                .iter()
                .flat_map(|row| row.iter().copied())
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_static_position_solution_state_covariance_m2",
                "out",
                &values,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy epoch-local receiver clocks. Output uses the variable-length contract.
///
/// Safety: solution must be a live handle; out must point to len entries or be
/// NULL when len is 0; out_written and out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_static_position_solution_clock_biases(
    solution: *const SidereonStaticPositionSolution,
    out: *mut SidereonStaticPositionClockBias,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_static_position_solution_clock_biases",
        SidereonStatus::Panic,
        || {
            let solution = c_try!(require_ref(
                solution,
                "sidereon_static_position_solution_clock_biases",
                "solution"
            ));
            let values: Vec<SidereonStaticPositionClockBias> = solution
                .inner
                .per_epoch_clock
                .iter()
                .map(static_clock_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_static_position_solution_clock_biases",
                "out",
                &values,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy post-fit residual rows. Output uses the variable-length contract.
///
/// Safety: solution must be a live handle; out must point to len entries or be
/// NULL when len is 0; out_written and out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_static_position_solution_residuals(
    solution: *const SidereonStaticPositionSolution,
    out: *mut SidereonStaticPositionResidual,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_static_position_solution_residuals",
        SidereonStatus::Panic,
        || {
            let solution = c_try!(require_ref(
                solution,
                "sidereon_static_position_solution_residuals",
                "solution"
            ));
            let values: Vec<SidereonStaticPositionResidual> = solution
                .inner
                .residuals_m
                .iter()
                .map(static_residual_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_static_position_solution_residuals",
                "out",
                &values,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy rejected satellites for one input epoch. Output uses the
/// variable-length contract.
///
/// Safety: solution must be a live handle; out must point to len entries or be
/// NULL when len is 0; out_written and out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_static_position_solution_rejected_sats(
    solution: *const SidereonStaticPositionSolution,
    epoch_index: usize,
    out: *mut SidereonSppRejectedSat,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_static_position_solution_rejected_sats",
        SidereonStatus::Panic,
        || {
            let solution = c_try!(require_ref(
                solution,
                "sidereon_static_position_solution_rejected_sats",
                "solution"
            ));
            let Some(rows) = solution.inner.rejected_sats.get(epoch_index) else {
                set_last_error(format!(
                    "sidereon_static_position_solution_rejected_sats: epoch_index {epoch_index} out of range"
                ));
                return SidereonStatus::InvalidArgument;
            };
            let values: Vec<SidereonSppRejectedSat> = rows
                .iter()
                .map(|rejected| SidereonSppRejectedSat {
                    sat_id: satellite_token(rejected.satellite_id),
                    reason: spp_rejection_reason_to_c(rejected.reason),
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_static_position_solution_rejected_sats",
                "out",
                &values,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy leave-one-epoch-out diagnostics. Output uses the variable-length contract.
///
/// Safety: solution must be a live handle; out must point to len entries or be
/// NULL when len is 0; out_written and out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_static_position_solution_epoch_influence(
    solution: *const SidereonStaticPositionSolution,
    out: *mut SidereonStaticPositionEpochInfluence,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_static_position_solution_epoch_influence",
        SidereonStatus::Panic,
        || {
            let solution = c_try!(require_ref(
                solution,
                "sidereon_static_position_solution_epoch_influence",
                "solution"
            ));
            let values: Vec<SidereonStaticPositionEpochInfluence> = solution
                .inner
                .per_epoch_influence
                .iter()
                .map(epoch_influence_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_static_position_solution_epoch_influence",
                "out",
                &values,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy leave-one-satellite-out diagnostics by epoch. Output uses the
/// variable-length contract.
///
/// Safety: solution must be a live handle; out must point to len entries or be
/// NULL when len is 0; out_written and out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_static_position_solution_satellite_influence(
    solution: *const SidereonStaticPositionSolution,
    out: *mut SidereonStaticPositionSatelliteInfluence,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_static_position_solution_satellite_influence",
        SidereonStatus::Panic,
        || {
            let solution = c_try!(require_ref(
                solution,
                "sidereon_static_position_solution_satellite_influence",
                "solution"
            ));
            let values: Vec<SidereonStaticPositionSatelliteInfluence> = solution
                .inner
                .per_satellite_influence
                .iter()
                .map(satellite_influence_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_static_position_solution_satellite_influence",
                "out",
                &values,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy leave-one-satellite-out diagnostics grouped across epochs. Output uses
/// the variable-length contract.
///
/// Safety: solution must be a live handle; out must point to len entries or be
/// NULL when len is 0; out_written and out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_static_position_solution_satellite_batch_influence(
    solution: *const SidereonStaticPositionSolution,
    out: *mut SidereonStaticPositionSatelliteBatchInfluence,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_static_position_solution_satellite_batch_influence",
        SidereonStatus::Panic,
        || {
            let solution = c_try!(require_ref(
                solution,
                "sidereon_static_position_solution_satellite_batch_influence",
                "solution"
            ));
            let values: Vec<SidereonStaticPositionSatelliteBatchInfluence> = solution
                .inner
                .per_satellite_batch_influence
                .iter()
                .map(satellite_batch_influence_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_static_position_solution_satellite_batch_influence",
                "out",
                &values,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Release a static-position solution handle. Passing NULL is a no-op.
///
/// Safety: solution must be NULL or a live handle from a static-position solve
/// that has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_static_position_solution_free(
    solution: *mut SidereonStaticPositionSolution,
) {
    ffi_boundary("sidereon_static_position_solution_free", (), || {
        free_boxed(solution);
    });
}

#[allow(clippy::too_many_arguments)]
unsafe fn solve_static_position_common(
    fn_name: &str,
    source: &dyn EphemerisSource,
    epochs: *const SidereonStaticPositionEpoch,
    epoch_count: usize,
    options: *const SidereonStaticPositionOptions,
    out_error: *mut SidereonStaticPositionErrorKind,
    out_solution: *mut *mut SidereonStaticPositionSolution,
) -> SidereonStatus {
    c_try!(init_static_position_error(
        out_error,
        SidereonStaticPositionErrorKind::None
    ));
    let out_solution = c_try!(static_position_validation(
        out_error,
        require_out(out_solution, fn_name, "out_solution")
    ));
    *out_solution = ptr::null_mut();
    let epochs = c_try!(static_position_validation(
        out_error,
        static_epochs_from_c(fn_name, epochs, epoch_count)
    ));
    let options = c_try!(static_position_validation(
        out_error,
        static_options_from_c(fn_name, options)
    ));
    match core_solve_static(source, &epochs, options) {
        Ok(inner) => {
            write_boxed_handle(out_solution, SidereonStaticPositionSolution { inner });
            SidereonStatus::Ok
        }
        Err(err) => map_static_position_error(fn_name, err, out_error),
    }
}

unsafe fn init_static_position_error(
    out_error: *mut SidereonStaticPositionErrorKind,
    value: SidereonStaticPositionErrorKind,
) -> Result<(), SidereonStatus> {
    let out_error = require_out(out_error, "sidereon_static_position", "out_error")?;
    *out_error = value;
    Ok(())
}

unsafe fn static_position_validation<T>(
    out_error: *mut SidereonStaticPositionErrorKind,
    result: Result<T, SidereonStatus>,
) -> Result<T, SidereonStatus> {
    result.inspect_err(|_| {
        let _ =
            init_static_position_error(out_error, SidereonStaticPositionErrorKind::InvalidInput);
    })
}

unsafe fn static_epochs_from_c(
    fn_name: &str,
    epochs: *const SidereonStaticPositionEpoch,
    epoch_count: usize,
) -> Result<Vec<CoreStaticEpoch>, SidereonStatus> {
    let raw = require_slice(epochs, epoch_count, fn_name, "epochs")?;
    let mut out = Vec::with_capacity(raw.len());
    for (idx, epoch) in raw.iter().enumerate() {
        let glonass_channels = glonass_channels_from_c(fn_name, &epoch.inputs)?;
        let solve_inputs = build_spp_solve_inputs(
            fn_name,
            &epoch.inputs.base,
            beidou_klobuchar_from_c(&epoch.inputs),
            None,
            glonass_channels,
        )?;
        let weights = if epoch.weight_count == 0 {
            None
        } else {
            let weights = require_slice(epoch.weights, epoch.weight_count, fn_name, "weights")?;
            if weights.len() != epoch.inputs.base.observation_count {
                set_last_error(format!(
                    "{fn_name}: epochs[{idx}].weight_count must match observation_count"
                ));
                return Err(SidereonStatus::InvalidArgument);
            }
            Some(weights.to_vec())
        };
        let mut core_epoch = CoreStaticEpoch::from_solve_inputs(solve_inputs);
        core_epoch.weights = weights;
        out.push(core_epoch);
    }
    Ok(out)
}

unsafe fn static_options_from_c(
    fn_name: &str,
    options: *const SidereonStaticPositionOptions,
) -> Result<CoreStaticSolveOptions, SidereonStatus> {
    let options = match options.as_ref() {
        Some(options) => *options,
        None => {
            let defaults = CoreStaticSolveOptions::default();
            return Ok(defaults);
        }
    };
    if options.robust_enabled && options.robust.max_outer == 0 {
        set_last_error(format!("{fn_name}: robust.max_outer must be positive"));
        return Err(SidereonStatus::InvalidArgument);
    }
    Ok(CoreStaticSolveOptions {
        initial_position_m: options.initial_position_m,
        with_geodetic: options.with_geodetic,
        robust: options.robust_enabled.then_some(RobustConfig {
            huber_k: options.robust.huber_k,
            scale_floor_m: options.robust.scale_floor_m,
            max_outer: options.robust.max_outer,
            outer_tol_m: options.robust.outer_tol_m,
        }),
    })
}

fn static_position_error_kind(err: &CoreStaticSolveError) -> SidereonStaticPositionErrorKind {
    match err {
        CoreStaticSolveError::EmptyEpochs => SidereonStaticPositionErrorKind::EmptyEpochs,
        CoreStaticSolveError::InvalidInput { .. } => SidereonStaticPositionErrorKind::InvalidInput,
        CoreStaticSolveError::EpochInput { .. } => SidereonStaticPositionErrorKind::EpochInput,
        CoreStaticSolveError::DuplicateObservation { .. } => {
            SidereonStaticPositionErrorKind::DuplicateObservation
        }
        CoreStaticSolveError::IonosphereUnsupported { .. } => {
            SidereonStaticPositionErrorKind::IonosphereUnsupported
        }
        CoreStaticSolveError::TooFewMeasurements { .. } => {
            SidereonStaticPositionErrorKind::TooFewMeasurements
        }
        CoreStaticSolveError::EphemerisLost { .. } => {
            SidereonStaticPositionErrorKind::EphemerisLost
        }
        CoreStaticSolveError::Singular(_) => SidereonStaticPositionErrorKind::Singular,
    }
}

unsafe fn map_static_position_error(
    fn_name: &str,
    err: CoreStaticSolveError,
    out_error: *mut SidereonStaticPositionErrorKind,
) -> SidereonStatus {
    let kind = static_position_error_kind(&err);
    let _ = init_static_position_error(out_error, kind);
    set_last_error(format!("{fn_name}: {err}"));
    match kind {
        SidereonStaticPositionErrorKind::InvalidInput
        | SidereonStaticPositionErrorKind::EpochInput
        | SidereonStaticPositionErrorKind::DuplicateObservation
        | SidereonStaticPositionErrorKind::IonosphereUnsupported
        | SidereonStaticPositionErrorKind::EmptyEpochs => SidereonStatus::InvalidArgument,
        _ => SidereonStatus::Solve,
    }
}

fn flatten_mat3(matrix: [[f64; 3]; 3]) -> [f64; 9] {
    [
        matrix[0][0],
        matrix[0][1],
        matrix[0][2],
        matrix[1][0],
        matrix[1][1],
        matrix[1][2],
        matrix[2][0],
        matrix[2][1],
        matrix[2][2],
    ]
}

fn static_solve_status_to_c(status: Status) -> SidereonSppSolveStatus {
    match status {
        Status::GradientTolerance => SidereonSppSolveStatus::GradientTolerance,
        Status::CostTolerance => SidereonSppSolveStatus::CostTolerance,
        Status::StepTolerance => SidereonSppSolveStatus::StepTolerance,
        Status::MaxEvaluations => SidereonSppSolveStatus::MaxEvaluations,
    }
}

fn empty_static_position_metadata() -> SidereonStaticPositionMetadata {
    SidereonStaticPositionMetadata {
        iterations: 0,
        converged: false,
        status: SidereonSppSolveStatus::MaxEvaluations,
        outer_iterations: 0,
        has_final_robust_scale_m: false,
        final_robust_scale_m: 0.0,
        used_measurements: 0,
        n_parameters: 0,
        redundancy: 0,
        geometry_quality: empty_geometry_quality(),
    }
}

fn static_metadata_to_c(
    metadata: &CoreStaticSolutionMetadata,
    geometry_quality: &CoreGeometryQuality,
) -> SidereonStaticPositionMetadata {
    SidereonStaticPositionMetadata {
        iterations: metadata.iterations,
        converged: metadata.converged,
        status: static_solve_status_to_c(metadata.status),
        outer_iterations: metadata.outer_iterations,
        has_final_robust_scale_m: metadata.final_robust_scale_m.is_some(),
        final_robust_scale_m: metadata.final_robust_scale_m.unwrap_or(0.0),
        used_measurements: metadata.used_measurements,
        n_parameters: metadata.n_parameters,
        redundancy: metadata.redundancy as i64,
        geometry_quality: geometry_quality_to_c(geometry_quality),
    }
}

fn static_clock_to_c(clock: &CoreStaticClockBias) -> SidereonStaticPositionClockBias {
    SidereonStaticPositionClockBias {
        epoch_index: clock.epoch_index,
        system: gnss_system_to_c(clock.system),
        clock_s: clock.clock_s,
    }
}

fn static_residual_to_c(residual: &CoreStaticResidual) -> SidereonStaticPositionResidual {
    SidereonStaticPositionResidual {
        epoch_index: residual.epoch_index,
        sat_id: satellite_token(residual.satellite_id),
        residual_m: residual.residual_m,
        base_weight: residual.base_weight,
        effective_weight: residual.effective_weight,
        robust_weight_ratio: residual.robust_weight_ratio,
    }
}

fn spp_rejection_reason_to_c(reason: CoreSppRejectionReason) -> SidereonSppRejectionReason {
    match reason {
        CoreSppRejectionReason::NoEphemeris => SidereonSppRejectionReason::NoEphemeris,
        CoreSppRejectionReason::LowElevation => SidereonSppRejectionReason::LowElevation,
        CoreSppRejectionReason::SbasWithdrawn => SidereonSppRejectionReason::SbasWithdrawn,
        CoreSppRejectionReason::SbasIonoUncovered => SidereonSppRejectionReason::SbasIonoUncovered,
    }
}

fn influence_status_to_c(
    status: CoreStaticInfluenceStatus,
) -> SidereonStaticPositionInfluenceStatus {
    match status {
        CoreStaticInfluenceStatus::Solved => SidereonStaticPositionInfluenceStatus::Solved,
        CoreStaticInfluenceStatus::TooFewMeasurements => {
            SidereonStaticPositionInfluenceStatus::TooFewMeasurements
        }
        CoreStaticInfluenceStatus::SingularGeometry => {
            SidereonStaticPositionInfluenceStatus::SingularGeometry
        }
        CoreStaticInfluenceStatus::InvalidInput => {
            SidereonStaticPositionInfluenceStatus::InvalidInput
        }
        CoreStaticInfluenceStatus::EphemerisUnavailable => {
            SidereonStaticPositionInfluenceStatus::EphemerisUnavailable
        }
        CoreStaticInfluenceStatus::SolveFailed => {
            SidereonStaticPositionInfluenceStatus::SolveFailed
        }
    }
}

fn optional_delta(delta: Option<[f64; 3]>) -> (bool, [f64; 3]) {
    match delta {
        Some(delta) => (true, delta),
        None => (false, [0.0; 3]),
    }
}

fn epoch_influence_to_c(
    influence: &CoreStaticEpochInfluence,
) -> SidereonStaticPositionEpochInfluence {
    let (has_delta, delta) = optional_delta(influence.position_delta_m);
    SidereonStaticPositionEpochInfluence {
        epoch_index: influence.epoch_index,
        omitted_measurements: influence.omitted_measurements,
        status: influence_status_to_c(influence.status),
        has_position_delta_m: has_delta,
        position_delta_m: delta,
        position_delta_norm_m: influence.position_delta_norm_m.unwrap_or(0.0),
        has_residual_rms_m: influence.residual_rms_m.is_some(),
        residual_rms_m: influence.residual_rms_m.unwrap_or(0.0),
        min_robust_weight_ratio: influence.min_robust_weight_ratio,
    }
}

fn satellite_influence_to_c(
    influence: &CoreStaticSatelliteInfluence,
) -> SidereonStaticPositionSatelliteInfluence {
    let (has_delta, delta) = optional_delta(influence.position_delta_m);
    SidereonStaticPositionSatelliteInfluence {
        epoch_index: influence.epoch_index,
        sat_id: satellite_token(influence.satellite_id),
        status: influence_status_to_c(influence.status),
        has_position_delta_m: has_delta,
        position_delta_m: delta,
        position_delta_norm_m: influence.position_delta_norm_m.unwrap_or(0.0),
        has_residual_rms_m: influence.residual_rms_m.is_some(),
        residual_rms_m: influence.residual_rms_m.unwrap_or(0.0),
        residual_m: influence.residual_m,
        base_weight: influence.base_weight,
        effective_weight: influence.effective_weight,
        robust_weight_ratio: influence.robust_weight_ratio,
    }
}

fn satellite_batch_influence_to_c(
    influence: &CoreStaticSatelliteBatchInfluence,
) -> SidereonStaticPositionSatelliteBatchInfluence {
    let (has_delta, delta) = optional_delta(influence.position_delta_m);
    SidereonStaticPositionSatelliteBatchInfluence {
        sat_id: satellite_token(influence.satellite_id),
        omitted_measurements: influence.omitted_measurements,
        status: influence_status_to_c(influence.status),
        has_position_delta_m: has_delta,
        position_delta_m: delta,
        position_delta_norm_m: influence.position_delta_norm_m.unwrap_or(0.0),
        has_residual_rms_m: influence.residual_rms_m.is_some(),
        residual_rms_m: influence.residual_rms_m.unwrap_or(0.0),
        min_robust_weight_ratio: influence.min_robust_weight_ratio,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::MaybeUninit;

    #[derive(Debug, Clone)]
    struct FixedEphemeris {
        positions: BTreeMap<GnssSatelliteId, [f64; 3]>,
    }

    impl EphemerisSource for FixedEphemeris {
        fn position_clock_at_j2000_s(
            &self,
            sat: GnssSatelliteId,
            _t_j2000_s: f64,
        ) -> Option<([f64; 3], f64)> {
            self.positions
                .get(&sat)
                .copied()
                .map(|position| (position, 0.0))
        }
    }

    const TRUTH: [f64; 3] = [6_378_137.0, 0.0, 0.0];
    const RANGE_M: f64 = 20_200_000.0;

    fn gps(prn: u8) -> GnssSatelliteId {
        GnssSatelliteId::new(GnssSystem::Gps, prn).expect("valid GPS id")
    }

    fn zero_klobuchar() -> KlobucharCoeffs {
        KlobucharCoeffs {
            alpha: [0.0; 4],
            beta: [0.0; 4],
        }
    }

    fn sat_position(azimuth_deg: f64, elevation_deg: f64) -> [f64; 3] {
        let los = line_of_sight_from_az_el_deg(
            azimuth_deg,
            elevation_deg,
            Wgs84Geodetic {
                lat_rad: 0.0,
                lon_rad: 0.0,
                height_m: 0.0,
            },
        )
        .expect("valid line of sight");
        [
            TRUTH[0] + RANGE_M * los.e_x,
            TRUTH[1] + RANGE_M * los.e_y,
            TRUTH[2] + RANGE_M * los.e_z,
        ]
    }

    fn epoch_angles(epoch_index: usize) -> Vec<(f64, f64)> {
        match epoch_index {
            0 => vec![
                (0.0, 58.0),
                (60.0, 47.0),
                (130.0, 42.0),
                (210.0, 53.0),
                (300.0, 38.0),
                (20.0, 32.0),
            ],
            _ => vec![
                (25.0, 55.0),
                (90.0, 44.0),
                (155.0, 49.0),
                (240.0, 36.0),
                (315.0, 46.0),
                (350.0, 31.0),
            ],
        }
    }

    fn make_store(epoch_count: usize) -> FixedEphemeris {
        let mut positions = BTreeMap::new();
        for epoch_index in 0..epoch_count {
            for (sat_index, (azimuth_deg, elevation_deg)) in
                epoch_angles(epoch_index).into_iter().enumerate()
            {
                let sat = gps((epoch_index * 10 + sat_index + 1) as u8);
                positions.insert(sat, sat_position(azimuth_deg, elevation_deg));
            }
        }
        FixedEphemeris { positions }
    }

    fn geometric_range(position: [f64; 3]) -> f64 {
        ((position[0] - TRUTH[0]).powi(2)
            + (position[1] - TRUTH[1]).powi(2)
            + (position[2] - TRUTH[2]).powi(2))
        .sqrt()
    }

    fn make_epoch(eph: &FixedEphemeris, epoch_index: usize, clock_m: f64) -> CoreStaticEpoch {
        let measurements = epoch_angles(epoch_index)
            .into_iter()
            .enumerate()
            .map(|(sat_index, _)| {
                let sat = gps((epoch_index * 10 + sat_index + 1) as u8);
                let position = eph.positions[&sat];
                Observation {
                    satellite_id: sat,
                    pseudorange_m: geometric_range(position) + clock_m,
                }
            })
            .collect();
        CoreStaticEpoch {
            measurements,
            weights: None,
            t_rx_j2000_s: 1000.0 + epoch_index as f64 * 30.0,
            t_rx_second_of_day_s: 12_000.0,
            day_of_year: 120.0 + epoch_index as f64,
            clock_initial_m: 0.0,
            corrections: Corrections::NONE,
            klobuchar: zero_klobuchar(),
            beidou_klobuchar: None,
            galileo_nequick: None,
            sbas_iono: None,
            glonass_channels: BTreeMap::new(),
            met: SurfaceMet::default(),
        }
    }

    fn options() -> CoreStaticSolveOptions {
        CoreStaticSolveOptions {
            initial_position_m: [TRUTH[0] + 120.0, TRUTH[1] - 80.0, TRUTH[2] + 50.0],
            with_geodetic: true,
            robust: None,
        }
    }

    fn c_options(options: CoreStaticSolveOptions) -> SidereonStaticPositionOptions {
        SidereonStaticPositionOptions {
            initial_position_m: options.initial_position_m,
            with_geodetic: options.with_geodetic,
            robust_enabled: false,
            robust: default_robust_config(),
        }
    }

    fn c_epoch_inputs(
        core_epochs: &[CoreStaticEpoch],
    ) -> (
        Vec<Vec<CString>>,
        Vec<Vec<SidereonObservation>>,
        Vec<SidereonStaticPositionEpoch>,
    ) {
        let mut tokens = Vec::new();
        let mut observations = Vec::new();
        for epoch in core_epochs {
            let epoch_tokens = epoch
                .measurements
                .iter()
                .map(|obs| CString::new(obs.satellite_id.to_string()).expect("sat token"))
                .collect::<Vec<_>>();
            let epoch_observations = epoch
                .measurements
                .iter()
                .zip(&epoch_tokens)
                .map(|(obs, token)| SidereonObservation {
                    sat_id: token.as_ptr(),
                    pseudorange_m: obs.pseudorange_m,
                })
                .collect::<Vec<_>>();
            tokens.push(epoch_tokens);
            observations.push(epoch_observations);
        }

        let mut c_epochs = Vec::new();
        for (idx, epoch) in core_epochs.iter().enumerate() {
            let mut inputs = MaybeUninit::<SidereonSppInputsV2>::uninit();
            let status = unsafe { sidereon_spp_inputs_v2_init(inputs.as_mut_ptr()) };
            assert_eq!(status, SidereonStatus::Ok);
            let mut inputs = unsafe { inputs.assume_init() };
            inputs.base = SidereonSppInputs {
                observations: observations[idx].as_ptr(),
                observation_count: observations[idx].len(),
                t_rx_j2000_s: epoch.t_rx_j2000_s,
                t_rx_second_of_day_s: epoch.t_rx_second_of_day_s,
                day_of_year: epoch.day_of_year,
                initial_guess: [
                    TRUTH[0] + 120.0,
                    TRUTH[1] - 80.0,
                    TRUTH[2] + 50.0,
                    epoch.clock_initial_m,
                ],
                ionosphere: false,
                troposphere: false,
                klobuchar_alpha: [0.0; 4],
                klobuchar_beta: [0.0; 4],
                pressure_hpa: SurfaceMet::default().pressure_hpa,
                temperature_k: SurfaceMet::default().temperature_k,
                relative_humidity: SurfaceMet::default().relative_humidity,
                with_geodetic: true,
            };
            c_epochs.push(SidereonStaticPositionEpoch {
                inputs,
                weights: ptr::null(),
                weight_count: 0,
            });
        }
        (tokens, observations, c_epochs)
    }

    fn assert_close(got: f64, want: f64, tol: f64) {
        assert!(
            (got - want).abs() <= tol,
            "got {got:e}, want {want:e}, tol {tol:e}"
        );
    }

    fn flatten_state(matrix: &[Vec<f64>]) -> Vec<f64> {
        matrix.iter().flat_map(|row| row.iter().copied()).collect()
    }

    #[test]
    fn static_position_solution_matches_core_reference() {
        let eph = make_store(2);
        let core_epochs = vec![make_epoch(&eph, 0, 12.0), make_epoch(&eph, 1, 16.0)];
        let core_options = options();
        let expected =
            core_solve_static(&eph, &core_epochs, core_options).expect("core static solve");

        let (_tokens, _observations, c_epochs) = c_epoch_inputs(&core_epochs);
        let c_options = c_options(core_options);
        let mut error = SidereonStaticPositionErrorKind::None;
        let mut solution = ptr::null_mut();
        let status = unsafe {
            solve_static_position_common(
                "test_static_position",
                &eph,
                c_epochs.as_ptr(),
                c_epochs.len(),
                &c_options,
                &mut error,
                &mut solution,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(error, SidereonStaticPositionErrorKind::None);
        assert!(!solution.is_null());

        let mut position = [0.0; 3];
        let status = unsafe {
            sidereon_static_position_solution_position(solution, position.as_mut_ptr(), 3)
        };
        assert_eq!(status, SidereonStatus::Ok);
        for (got, want) in position.iter().zip(expected.position.as_array()) {
            assert_close(*got, want, 1.0e-8);
        }

        let mut geodetic = empty_geodetic();
        let mut present = false;
        let status = unsafe {
            sidereon_static_position_solution_geodetic(solution, &mut geodetic, &mut present)
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert!(present);
        let expected_geodetic = expected.geodetic.expect("geodetic");
        assert_close(geodetic.lat_rad, expected_geodetic.lat_rad, 1.0e-14);
        assert_close(geodetic.lon_rad, expected_geodetic.lon_rad, 1.0e-14);
        assert_close(geodetic.height_m, expected_geodetic.height_m, 1.0e-8);

        let mut metadata = empty_static_position_metadata();
        let status = unsafe { sidereon_static_position_solution_metadata(solution, &mut metadata) };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(metadata.iterations, expected.metadata.iterations);
        assert_eq!(metadata.converged, expected.metadata.converged);
        assert_eq!(
            metadata.status,
            static_solve_status_to_c(expected.metadata.status)
        );
        assert_eq!(
            metadata.used_measurements,
            expected.metadata.used_measurements
        );
        assert_eq!(metadata.n_parameters, expected.metadata.n_parameters);
        assert_eq!(metadata.redundancy, expected.metadata.redundancy as i64);

        let mut ecef = [0.0; 9];
        let status = unsafe {
            sidereon_static_position_solution_position_covariance_ecef_m2(
                solution,
                ecef.as_mut_ptr(),
                9,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        for (got, want) in ecef
            .iter()
            .zip(flatten_mat3(expected.covariance.position_ecef_m2))
        {
            assert_close(*got, want, 1.0e-12);
        }

        let expected_state = flatten_state(&expected.covariance.state_m2);
        let mut written = 0usize;
        let mut required = 0usize;
        let status = unsafe {
            sidereon_static_position_solution_state_covariance_m2(
                solution,
                ptr::null_mut(),
                0,
                &mut written,
                &mut required,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(written, 0);
        assert_eq!(required, expected_state.len());
        let mut state = vec![0.0; required];
        let status = unsafe {
            sidereon_static_position_solution_state_covariance_m2(
                solution,
                state.as_mut_ptr(),
                state.len(),
                &mut written,
                &mut required,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(written, expected_state.len());
        for (got, want) in state.iter().zip(expected_state) {
            assert_close(*got, want, 1.0e-10);
        }

        let mut clock_required = 0usize;
        let status = unsafe {
            sidereon_static_position_solution_clock_biases(
                solution,
                ptr::null_mut(),
                0,
                &mut written,
                &mut clock_required,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(clock_required, expected.per_epoch_clock.len());
        let mut clocks = vec![
            SidereonStaticPositionClockBias {
                epoch_index: 0,
                system: SidereonGnssSystem::Gps,
                clock_s: 0.0,
            };
            clock_required
        ];
        let status = unsafe {
            sidereon_static_position_solution_clock_biases(
                solution,
                clocks.as_mut_ptr(),
                clocks.len(),
                &mut written,
                &mut clock_required,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        for (got, want) in clocks.iter().zip(&expected.per_epoch_clock) {
            assert_eq!(got.epoch_index, want.epoch_index);
            assert_eq!(got.system, gnss_system_to_c(want.system));
            assert_close(got.clock_s, want.clock_s, 1.0e-15);
        }

        let mut residual_required = 0usize;
        let status = unsafe {
            sidereon_static_position_solution_residuals(
                solution,
                ptr::null_mut(),
                0,
                &mut written,
                &mut residual_required,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(residual_required, expected.residuals_m.len());

        let mut influence_required = 0usize;
        let status = unsafe {
            sidereon_static_position_solution_epoch_influence(
                solution,
                ptr::null_mut(),
                0,
                &mut written,
                &mut influence_required,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(influence_required, expected.per_epoch_influence.len());

        unsafe { sidereon_static_position_solution_free(solution) };
    }

    #[test]
    fn static_position_pre_core_validation_sets_typed_error() {
        let eph = make_store(1);
        let core_epochs = vec![make_epoch(&eph, 0, 12.0)];
        let (_tokens, _observations, mut c_epochs) = c_epoch_inputs(&core_epochs);

        let mut invalid_options = c_options(options());
        invalid_options.robust_enabled = true;
        invalid_options.robust.max_outer = 0;
        let mut error = SidereonStaticPositionErrorKind::None;
        let mut solution = ptr::null_mut();
        let status = unsafe {
            solve_static_position_common(
                "test_static_position_invalid_options",
                &eph,
                c_epochs.as_ptr(),
                c_epochs.len(),
                &invalid_options,
                &mut error,
                &mut solution,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);
        assert_eq!(error, SidereonStaticPositionErrorKind::InvalidInput);
        assert!(solution.is_null());

        let weight = 1.0;
        c_epochs[0].weights = &weight;
        c_epochs[0].weight_count = 1;
        let valid_options = c_options(options());
        error = SidereonStaticPositionErrorKind::None;
        let status = unsafe {
            solve_static_position_common(
                "test_static_position_bad_weights",
                &eph,
                c_epochs.as_ptr(),
                c_epochs.len(),
                &valid_options,
                &mut error,
                &mut solution,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);
        assert_eq!(error, SidereonStaticPositionErrorKind::InvalidInput);
        assert!(solution.is_null());
    }
}
