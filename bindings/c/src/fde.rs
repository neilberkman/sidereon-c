use super::*;

/// One RAIM per-satellite inverse-variance weight for an FDE solve. Supplied as
/// an array on SidereonFdeOptions when unit_weights is false. A satellite absent
/// from the array defaults to unit weight, matching the engine RAIM contract.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonFdeRaimWeight {
    /// Null-terminated satellite token, for example G08. The terminator must
    /// appear within 16 bytes.
    pub sat_id: *const c_char,
    /// Inverse-variance RAIM weight for this satellite; must be finite and
    /// positive.
    pub weight: f64,
}

/// Options for a fault-detection-and-exclusion solve. Initialize with
/// sidereon_fde_options_init, then override fields.
#[repr(C)]
pub struct SidereonFdeOptions {
    /// RAIM false-alarm probability, in the open interval (0, 1).
    pub p_fa: f64,
    /// Maximum number of exclusions to attempt. The Sidereon high-level API uses
    /// max(observation_count - 4, 0) as its default; choose a value for your
    /// geometry. Zero permits fault detection but no exclusion.
    pub max_iterations: usize,
    /// When true, RAIM uses unit weights and the weights array is ignored.
    pub unit_weights: bool,
    /// Pointer to weight_count per-satellite weights, used only when
    /// unit_weights is false. May be NULL when weight_count is 0.
    pub weights: *const SidereonFdeRaimWeight,
    /// Number of weight entries pointed to by weights.
    pub weight_count: usize,
    /// When true, override the distinct GNSS clock-system count RAIM uses for its
    /// degrees of freedom with n_systems; when false the engine counts the
    /// distinct systems among the used satellites.
    pub n_systems_enabled: bool,
    /// Distinct GNSS clock-system count override (must be >= 1), used only when
    /// n_systems_enabled is true.
    pub n_systems: i64,
    /// When false, the engine default validation gates apply to each
    /// per-iteration solve; when true, the validation field is applied.
    pub use_validation_options: bool,
    /// Per-iteration solution validation gates.
    pub validation: SidereonSppValidationOptions,
}

/// The result of an FDE solve: the surviving receiver solution, the satellites
/// excluded in exclusion order, and the exclusion count. Opaque to C. Create
/// with sidereon_fde_solve_spp or sidereon_fde_solve_broadcast and release with
/// sidereon_fde_solution_free.
pub struct SidereonFdeSolution {
    pub(crate) solution: ReceiverSolution,
    pub(crate) excluded: Vec<String>,
    pub(crate) iterations: usize,
}

/// Fill *out_options with the default FDE options: unit weights, the engine
/// default false-alarm probability, no exclusions, no system-count override, and
/// the engine default validation gates. Override fields before solving.
///
/// Safety: out_options must point to writable storage for a SidereonFdeOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fde_options_init(
    out_options: *mut SidereonFdeOptions,
) -> SidereonStatus {
    ffi_boundary("sidereon_fde_options_init", SidereonStatus::Panic, || {
        let out_options = c_try!(require_out(
            out_options,
            "sidereon_fde_options_init",
            "out_options"
        ));
        *out_options = default_fde_options();
        SidereonStatus::Ok
    })
}

/// Run fault detection and exclusion against an SP3 precise product. On success
/// writes a newly owned FDE solution handle to *out_solution (release with
/// sidereon_fde_solution_free). Uses the legacy SidereonSppInputs ABI, so this
/// path supplies no GLONASS channels or BeiDou Klobuchar coefficients, matching
/// sidereon_solve_spp.
///
/// Safety: sp3 must be a live handle; inputs must point to a valid
/// SidereonSppInputs whose observations field points to observation_count valid
/// entries with bounded null-terminated sat_id values; options must point to a
/// valid SidereonFdeOptions (with weights pointing to weight_count entries when
/// unit_weights is false); out_solution must point to storage for a
/// SidereonFdeSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fde_solve_spp(
    sp3: *const SidereonSp3,
    inputs: *const SidereonSppInputs,
    options: *const SidereonFdeOptions,
    out_solution: *mut *mut SidereonFdeSolution,
) -> SidereonStatus {
    ffi_boundary("sidereon_fde_solve_spp", SidereonStatus::Panic, || {
        let out_solution = c_try!(require_out(
            out_solution,
            "sidereon_fde_solve_spp",
            "out_solution"
        ));
        *out_solution = ptr::null_mut();
        let sp3 = c_try!(require_ref(sp3, "sidereon_fde_solve_spp", "sp3"));
        let inputs = c_try!(require_ref(inputs, "sidereon_fde_solve_spp", "inputs"));
        let options = c_try!(require_ref(options, "sidereon_fde_solve_spp", "options"));
        let solve_inputs = c_try!(build_spp_solve_inputs(
            "sidereon_fde_solve_spp",
            inputs,
            None,
            None,
            BTreeMap::new(),
        ));
        run_fde(
            "sidereon_fde_solve_spp",
            &sp3.inner,
            solve_inputs,
            inputs.with_geodetic,
            options,
            out_solution,
        )
    })
}

/// Run fault detection and exclusion against a broadcast (navigation-message)
/// ephemeris. On success writes a newly owned FDE solution handle to
/// *out_solution (release with sidereon_fde_solution_free). Uses the legacy
/// SidereonSppInputs ABI, matching sidereon_solve_broadcast.
///
/// Safety: broadcast must be a live handle; inputs must point to a valid
/// SidereonSppInputs whose observations field points to observation_count valid
/// entries with bounded null-terminated sat_id values; options must point to a
/// valid SidereonFdeOptions; out_solution must point to storage for a
/// SidereonFdeSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fde_solve_broadcast(
    broadcast: *const SidereonBroadcastEphemeris,
    inputs: *const SidereonSppInputs,
    options: *const SidereonFdeOptions,
    out_solution: *mut *mut SidereonFdeSolution,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fde_solve_broadcast",
        SidereonStatus::Panic,
        || {
            let out_solution = c_try!(require_out(
                out_solution,
                "sidereon_fde_solve_broadcast",
                "out_solution"
            ));
            *out_solution = ptr::null_mut();
            let broadcast = c_try!(require_ref(
                broadcast,
                "sidereon_fde_solve_broadcast",
                "broadcast"
            ));
            let inputs = c_try!(require_ref(
                inputs,
                "sidereon_fde_solve_broadcast",
                "inputs"
            ));
            let options = c_try!(require_ref(
                options,
                "sidereon_fde_solve_broadcast",
                "options"
            ));
            let solve_inputs = c_try!(build_spp_solve_inputs(
                "sidereon_fde_solve_broadcast",
                inputs,
                None,
                None,
                BTreeMap::new(),
            ));
            run_fde(
                "sidereon_fde_solve_broadcast",
                &broadcast.inner,
                solve_inputs,
                inputs.with_geodetic,
                options,
                out_solution,
            )
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_robust_fde_solve_spp(
    sp3: *const SidereonSp3,
    inputs: *const SidereonSppInputs,
    robust: *const SidereonSppRobustConfig,
    options: *const SidereonFdeOptions,
    out_solution: *mut *mut SidereonFdeSolution,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_robust_fde_solve_spp",
        SidereonStatus::Panic,
        || {
            let out_solution = c_try!(require_out(
                out_solution,
                "sidereon_robust_fde_solve_spp",
                "out_solution"
            ));
            *out_solution = ptr::null_mut();
            let sp3 = c_try!(require_ref(sp3, "sidereon_robust_fde_solve_spp", "sp3"));
            let inputs = c_try!(require_ref(
                inputs,
                "sidereon_robust_fde_solve_spp",
                "inputs"
            ));
            let robust = c_try!(require_ref(
                robust,
                "sidereon_robust_fde_solve_spp",
                "robust"
            ));
            let options = c_try!(require_ref(
                options,
                "sidereon_robust_fde_solve_spp",
                "options"
            ));
            let solve_inputs = c_try!(build_spp_solve_inputs(
                "sidereon_robust_fde_solve_spp",
                inputs,
                None,
                None,
                BTreeMap::new(),
            ));
            run_robust_fde(
                "sidereon_robust_fde_solve_spp",
                &sp3.inner,
                solve_inputs,
                inputs.with_geodetic,
                robust_config_value_from_c(robust),
                options,
                out_solution,
            )
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_robust_fde_solve_broadcast(
    broadcast: *const SidereonBroadcastEphemeris,
    inputs: *const SidereonSppInputs,
    robust: *const SidereonSppRobustConfig,
    options: *const SidereonFdeOptions,
    out_solution: *mut *mut SidereonFdeSolution,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_robust_fde_solve_broadcast",
        SidereonStatus::Panic,
        || {
            let out_solution = c_try!(require_out(
                out_solution,
                "sidereon_robust_fde_solve_broadcast",
                "out_solution"
            ));
            *out_solution = ptr::null_mut();
            let broadcast = c_try!(require_ref(
                broadcast,
                "sidereon_robust_fde_solve_broadcast",
                "broadcast"
            ));
            let inputs = c_try!(require_ref(
                inputs,
                "sidereon_robust_fde_solve_broadcast",
                "inputs"
            ));
            let robust = c_try!(require_ref(
                robust,
                "sidereon_robust_fde_solve_broadcast",
                "robust"
            ));
            let options = c_try!(require_ref(
                options,
                "sidereon_robust_fde_solve_broadcast",
                "options"
            ));
            let solve_inputs = c_try!(build_spp_solve_inputs(
                "sidereon_robust_fde_solve_broadcast",
                inputs,
                None,
                None,
                BTreeMap::new(),
            ));
            run_robust_fde(
                "sidereon_robust_fde_solve_broadcast",
                &broadcast.inner,
                solve_inputs,
                inputs.with_geodetic,
                robust_config_value_from_c(robust),
                options,
                out_solution,
            )
        },
    )
}

/// Copy the surviving receiver solution out of an FDE solution into a newly owned
/// SidereonSppSolution, so the full spp solution accessors apply. Release the new
/// handle with sidereon_spp_solution_free.
///
/// Safety: sol must be a live handle; out_solution must point to storage for a
/// SidereonSppSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fde_solution_solution(
    sol: *const SidereonFdeSolution,
    out_solution: *mut *mut SidereonSppSolution,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fde_solution_solution",
        SidereonStatus::Panic,
        || {
            let out_solution = c_try!(require_out(
                out_solution,
                "sidereon_fde_solution_solution",
                "out_solution"
            ));
            *out_solution = ptr::null_mut();
            let sol = c_try!(require_ref(sol, "sidereon_fde_solution_solution", "sol"));
            write_boxed_handle(
                out_solution,
                SidereonSppSolution {
                    inner: sol.solution.clone(),
                },
            );
            SidereonStatus::Ok
        },
    )
}

/// Write the number of exclusions performed to *out_iterations.
///
/// Safety: sol must be a live handle; out_iterations must point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fde_solution_iterations(
    sol: *const SidereonFdeSolution,
    out_iterations: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fde_solution_iterations",
        SidereonStatus::Panic,
        || {
            let out_iterations = c_try!(require_out(
                out_iterations,
                "sidereon_fde_solution_iterations",
                "out_iterations"
            ));
            *out_iterations = 0;
            let sol = c_try!(require_ref(sol, "sidereon_fde_solution_iterations", "sol"));
            *out_iterations = sol.iterations;
            SidereonStatus::Ok
        },
    )
}

/// Copy the excluded satellite tokens in exclusion order. Uses the variable-length
/// output contract documented at the top of the header.
///
/// Safety: sol must be a live handle; out must point to at least len writable
/// entries or be NULL when len is 0; out_written and out_required must point to
/// size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fde_solution_excluded_sats(
    sol: *const SidereonFdeSolution,
    out: *mut SidereonSatelliteToken,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fde_solution_excluded_sats",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_fde_solution_excluded_sats",
                out_written,
                out_required
            ));
            let sol = c_try!(require_ref(
                sol,
                "sidereon_fde_solution_excluded_sats",
                "sol"
            ));
            let values: Vec<SidereonSatelliteToken> = sol
                .excluded
                .iter()
                .map(|token| satellite_token_from_text(token))
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_fde_solution_excluded_sats",
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

/// Release an FDE solution handle from sidereon_fde_solve_spp or
/// sidereon_fde_solve_broadcast. Passing NULL is a no-op.
///
/// Safety: sol must be NULL or a live handle from sidereon_fde_solve_spp or
/// sidereon_fde_solve_broadcast that has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fde_solution_free(sol: *mut SidereonFdeSolution) {
    ffi_boundary("sidereon_fde_solution_free", (), || {
        free_boxed(sol);
    });
}

// === CCSDS OEM and OPM navigation data messages ============================
//
// Forgiving readers and round-trippable writers for the CCSDS Orbit Ephemeris
// Message (OEM) and Orbit Parameter Message (OPM), in both KVN and XML
// encodings. Each message parses into an opaque handle; the handle serializes
// back to either encoding. This mirrors the SP3 handle idiom: parse to a handle,
// serialize from the handle, release with the matching _free.

fn robust_config_value_from_c(config: &SidereonSppRobustConfig) -> RobustConfig {
    RobustConfig {
        huber_k: config.huber_k,
        scale_floor_m: config.scale_floor_m,
        max_outer: config.max_outer,
        outer_tol_m: config.outer_tol_m,
    }
}

fn default_fde_options() -> SidereonFdeOptions {
    SidereonFdeOptions {
        p_fa: RaimOptions::default().p_fa,
        max_iterations: 0,
        unit_weights: true,
        weights: ptr::null(),
        weight_count: 0,
        n_systems_enabled: false,
        n_systems: 0,
        use_validation_options: false,
        validation: default_validation_options(),
    }
}

/// Drive [`fde_spp`] over any ephemeris source and write the surviving solution
/// into a newly owned handle. The detect/exclude/re-solve loop, the per-iteration
/// engine SPP solve, and the [`validate_receiver_solution`] gate are all the core
/// driver's own; this function only marshals the options in and the result out.
unsafe fn run_fde(
    fn_name: &str,
    eph: &dyn EphemerisSource,
    inputs: SolveInputs,
    with_geodetic: bool,
    options: &SidereonFdeOptions,
    out_solution: *mut *mut SidereonFdeSolution,
) -> SidereonStatus {
    let out_solution = match require_out(out_solution, fn_name, "out_solution") {
        Ok(out) => out,
        Err(status) => return status,
    };
    *out_solution = ptr::null_mut();

    let raim = match raim_options_from_fde_c(fn_name, options) {
        Ok(raim) => raim,
        Err(status) => return status,
    };
    let validation = validation_options_from_c(options.use_validation_options, &options.validation);
    let fde_options = FdeSppOptions {
        fde: FdeOptions {
            raim,
            max_iterations: options.max_iterations,
        },
        validation,
    };

    match fde_spp(eph, &inputs, with_geodetic, &fde_options) {
        Ok(found) => {
            write_boxed_handle(
                out_solution,
                SidereonFdeSolution {
                    solution: found.solution,
                    excluded: found.excluded,
                    iterations: found.iterations,
                },
            );
            SidereonStatus::Ok
        }
        Err(FdeError::FaultUnresolved(statistic)) => {
            set_last_error(format!(
                "{fn_name}: RAIM fault unresolved, test statistic {statistic}"
            ));
            SidereonStatus::Solve
        }
        Err(FdeError::Solve(FdeSppError::Spp(error))) => {
            set_last_error(format!("{fn_name}: {error}"));
            SidereonStatus::Solve
        }
        Err(FdeError::Solve(FdeSppError::Validation(error))) => {
            set_last_error(format!("{fn_name}: solution validation failed: {error:?}"));
            SidereonStatus::Solve
        }
        Err(FdeError::Raim(error)) => {
            set_last_error(format!("{fn_name}: RAIM options invalid: {error:?}"));
            SidereonStatus::InvalidArgument
        }
    }
}

unsafe fn run_robust_fde(
    fn_name: &str,
    eph: &dyn EphemerisSource,
    inputs: SolveInputs,
    with_geodetic: bool,
    robust: RobustConfig,
    options: &SidereonFdeOptions,
    out_solution: *mut *mut SidereonFdeSolution,
) -> SidereonStatus {
    let out_solution = match require_out(out_solution, fn_name, "out_solution") {
        Ok(out) => out,
        Err(status) => return status,
    };
    *out_solution = ptr::null_mut();

    let raim = match raim_options_from_fde_c(fn_name, options) {
        Ok(raim) => raim,
        Err(status) => return status,
    };
    let validation = validation_options_from_c(options.use_validation_options, &options.validation);
    let fde_options = FdeSppOptions {
        fde: FdeOptions {
            raim,
            max_iterations: options.max_iterations,
        },
        validation,
    };

    match spp_robust_fde_driver(eph, &inputs, with_geodetic, robust, &fde_options) {
        Ok(found) => {
            write_boxed_handle(
                out_solution,
                SidereonFdeSolution {
                    solution: found.solution,
                    excluded: found.excluded,
                    iterations: found.iterations,
                },
            );
            SidereonStatus::Ok
        }
        Err(FdeError::FaultUnresolved(statistic)) => {
            set_last_error(format!(
                "{fn_name}: RAIM fault unresolved, test statistic {statistic}"
            ));
            SidereonStatus::Solve
        }
        Err(FdeError::Solve(FdeSppError::Spp(error))) => {
            set_last_error(format!("{fn_name}: {error}"));
            SidereonStatus::Solve
        }
        Err(FdeError::Solve(FdeSppError::Validation(error))) => {
            set_last_error(format!("{fn_name}: solution validation failed: {error:?}"));
            SidereonStatus::Solve
        }
        Err(FdeError::Raim(error)) => {
            set_last_error(format!("{fn_name}: RAIM options invalid: {error:?}"));
            SidereonStatus::InvalidArgument
        }
    }
}

/// Build the engine [`RaimOptions`] from the C FDE options. With unit weights the
/// weights array is not read; otherwise each entry's token is canonicalized via
/// the same parser the SPP observations use, so the weight keys match the
/// solution's used-satellite tokens.
unsafe fn raim_options_from_fde_c(
    fn_name: &str,
    options: &SidereonFdeOptions,
) -> Result<RaimOptions, SidereonStatus> {
    let weights = if options.unit_weights {
        RaimWeights::Unit
    } else {
        let rows = require_slice(options.weights, options.weight_count, fn_name, "weights")?;
        let mut map = BTreeMap::new();
        for row in rows {
            let satellite_id = parse_satellite_token(fn_name, row.sat_id)?;
            map.insert(satellite_id.to_string(), row.weight);
        }
        RaimWeights::BySatellite(map)
    };
    let n_systems = options
        .n_systems_enabled
        .then_some(options.n_systems as isize);
    Ok(RaimOptions {
        p_fa: options.p_fa,
        weights,
        n_systems,
    })
}
