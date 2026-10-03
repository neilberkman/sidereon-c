use super::*;

/// One RAIM per-satellite inverse-variance weight for an FDE solve. Supplied as
/// an array on SidereonFdeOptions when weights_mode is BySatellite. A satellite
/// absent from the array defaults to unit weight, matching the engine RAIM
/// contract.
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
    /// Maximum number of exclusions. The default, 1, is RTKLIB demo5's single
    /// raim_fde exclusion; a larger budget repeats detection and a fresh
    /// leave-one-out search on the remaining set. Zero permits fault detection
    /// but no exclusion.
    pub max_exclusions: usize,
    /// The largest unweighted post-fit residual RMS, metres, an exclusion may
    /// leave; the default is RTKLIB demo5's initial rms of 100 m.
    pub max_exclusion_rms_m: f64,
    /// Which weights the detection statistic uses, a SidereonRaimWeightsMode
    /// value. The default, Solution, standardizes each residual by the
    /// pseudorange variance the solve weighted it by.
    pub weights_mode: u32,
    /// Pointer to weight_count per-satellite weights, used only when
    /// weights_mode is BySatellite. May be NULL when weight_count is 0.
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
/// excluded in exclusion order, the exclusion count and the accepted
/// solution's detection test. Opaque to C. Create with sidereon_fde_solve_spp
/// or sidereon_fde_solve_broadcast and release with sidereon_fde_solution_free.
pub struct SidereonFdeSolution {
    pub(crate) solution: ReceiverSolution,
    pub(crate) excluded: Vec<String>,
    pub(crate) iterations: usize,
    pub(crate) raim: RaimResult,
}

/// Why an FDE solve ended with a fault still detected
/// (sidereon_core::quality::FdeUnresolvedReason).
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonFdeUnresolvedReason {
    /// No unresolved FDE solve is recorded for this thread.
    None = 0,
    /// The exclusion budget (max_exclusions) was spent.
    ExclusionBudgetExhausted = 1,
    /// No leave-one-out re-solve was admissible: every candidate failed to
    /// solve, used fewer than five satellites, or left a residual RMS above
    /// max_exclusion_rms_m.
    NoAdmissibleExclusion = 2,
    /// A reason a later engine names that this binding has no code for yet.
    Unknown = 999,
}

/// The state the most recent FDE solve on this thread stopped in with a fault
/// still detected (SIDEREON_STATUS_SOLVE with an unresolved fault).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonFdeUnresolvedInfo {
    /// Why the loop stopped; None when no unresolved solve is recorded.
    pub reason: SidereonFdeUnresolvedReason,
    /// Satellites excluded before it stopped, copied by
    /// sidereon_last_fde_unresolved_excluded_sats.
    pub excluded_count: usize,
    /// The detection test of the last solution, with fault_detected true.
    pub raim: SidereonRaimResult,
}

thread_local! {
    static LAST_FDE_UNRESOLVED: std::cell::RefCell<Option<FdeUnresolved<ReceiverSolution>>> =
        const { std::cell::RefCell::new(None) };
}

fn fde_operation_boundary<T>(fn_name: &str, panic_value: T, body: impl FnOnce() -> T) -> T {
    LAST_FDE_UNRESOLVED.with(|slot| *slot.borrow_mut() = None);
    LAST_QUALITY_ERROR_KIND.with(|slot| slot.set(SidereonQualityErrorKind::None));
    ffi_boundary(fn_name, panic_value, body)
}

/// Fill *out_options with the default FDE options: sidereon-core's
/// FdeOptions::default() (the solve's own variances as RAIM weights, the
/// engine false-alarm probability, RTKLIB demo5's single exclusion and 100 m
/// exclusion RMS cap), no system-count override, and the engine default
/// validation gates. Override fields before solving.
///
/// Safety: out_options must point to writable storage for a SidereonFdeOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fde_options_init(
    out_options: *mut SidereonFdeOptions,
) -> SidereonStatus {
    ffi_boundary("sidereon_fde_options_init", SidereonStatus::Panic, || {
        let out_options = c_try!(require_uninit_out(
            out_options,
            "sidereon_fde_options_init",
            "out_options"
        ));
        out_options.write(default_fde_options());
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
/// weights_mode is BySatellite); out_solution must point to storage for a
/// SidereonFdeSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fde_solve_spp(
    sp3: *const SidereonSp3,
    inputs: *const SidereonSppInputs,
    options: *const SidereonFdeOptions,
    out_solution: *mut *mut SidereonFdeSolution,
) -> SidereonStatus {
    fde_operation_boundary("sidereon_fde_solve_spp", SidereonStatus::Panic, || {
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
    fde_operation_boundary(
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
    fde_operation_boundary(
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
    fde_operation_boundary(
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

/// Write the detection test of the accepted solution: testable is false when
/// the accepted set has no redundancy left to test.
///
/// Safety: sol must be a live handle; out must point to a SidereonRaimResult.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fde_solution_raim(
    sol: *const SidereonFdeSolution,
    out: *mut SidereonRaimResult,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_fde_solution_raim";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, FN_NAME, "out"));
        *out = crate::raim::empty_raim_result();
        let sol = c_try!(require_ref(sol, FN_NAME, "sol"));
        *out = crate::raim::raim_result_to_c(&sol.raim, &sol.solution.residuals_m);
        SidereonStatus::Ok
    })
}

/// Copy the weighted residuals of the accepted solution's detection test,
/// ordered by satellite token. Uses the variable-length output contract.
///
/// Safety: sol must be a live handle; out must point to len writable
/// SidereonRaimNormalizedResidual values or be NULL when len is 0; out_written
/// and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fde_solution_raim_normalized_residuals(
    sol: *const SidereonFdeSolution,
    out: *mut SidereonRaimNormalizedResidual,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_fde_solution_raim_normalized_residuals";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let sol = c_try!(require_ref(sol, FN_NAME, "sol"));
        let rows = crate::raim::raim_normalized_residuals_to_c(&sol.raim);
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            &rows,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Write the state the most recent FDE solve on this thread stopped in with a
/// fault still detected. Every FDE solve clears it first, so reason is None
/// unless the last one returned SIDEREON_STATUS_SOLVE for an unresolved fault.
///
/// Safety: out_info must point to a SidereonFdeUnresolvedInfo.
#[no_mangle]
pub unsafe extern "C" fn sidereon_last_fde_unresolved(
    out_info: *mut SidereonFdeUnresolvedInfo,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_last_fde_unresolved";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_info, FN_NAME, "out_info"));
        *out = SidereonFdeUnresolvedInfo {
            reason: SidereonFdeUnresolvedReason::None,
            excluded_count: 0,
            raim: crate::raim::empty_raim_result(),
        };
        LAST_FDE_UNRESOLVED.with(|slot| {
            if let Some(unresolved) = slot.borrow().as_ref() {
                *out = SidereonFdeUnresolvedInfo {
                    reason: fde_unresolved_reason_to_c(unresolved.reason),
                    excluded_count: unresolved.excluded.len(),
                    raim: crate::raim::raim_result_to_c(
                        &unresolved.raim,
                        &unresolved.solution.residuals_m,
                    ),
                };
            }
        });
        SidereonStatus::Ok
    })
}

/// Copy the last solution of the most recent unresolved FDE solve on this
/// thread into a newly owned SidereonSppSolution (release it with
/// sidereon_spp_solution_free); *out_solution is NULL when none is recorded.
///
/// Safety: out_solution must point to storage for a SidereonSppSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_last_fde_unresolved_solution(
    out_solution: *mut *mut SidereonSppSolution,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_last_fde_unresolved_solution";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_solution, FN_NAME, "out_solution"));
        *out = ptr::null_mut();
        let solution = LAST_FDE_UNRESOLVED.with(|slot| {
            slot.borrow()
                .as_ref()
                .map(|unresolved| unresolved.solution.clone())
        });
        if let Some(inner) = solution {
            write_boxed_handle(out, SidereonSppSolution { inner });
        }
        SidereonStatus::Ok
    })
}

/// Copy the satellites the most recent unresolved FDE solve on this thread
/// excluded, in exclusion order. Uses the variable-length output contract.
///
/// Safety: out must point to len writable SidereonSatelliteToken values or be
/// NULL when len is 0; out_written and out_required must point to size_t
/// values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_last_fde_unresolved_excluded_sats(
    out: *mut SidereonSatelliteToken,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_last_fde_unresolved_excluded_sats";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let values: Vec<SidereonSatelliteToken> = LAST_FDE_UNRESOLVED.with(|slot| {
            slot.borrow()
                .as_ref()
                .map(|unresolved| {
                    unresolved
                        .excluded
                        .iter()
                        .map(|token| satellite_token_from_text(token))
                        .collect()
                })
                .unwrap_or_default()
        });
        c_try!(copy_prefix_to_c(
            FN_NAME,
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

/// Copy the stored weighted residuals from the unresolved FDE's detection
/// test, ordered by satellite token. The result is retained on the current
/// thread until the next FDE solve producer begins.
///
/// Safety: out must point to len writable SidereonRaimNormalizedResidual
/// values or be NULL when len is 0; out_written and out_required must point to
/// size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_last_fde_unresolved_raim_normalized_residuals(
    out: *mut SidereonRaimNormalizedResidual,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_last_fde_unresolved_raim_normalized_residuals";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let rows = LAST_FDE_UNRESOLVED.with(|slot| {
            slot.borrow()
                .as_ref()
                .map(|unresolved| crate::raim::raim_normalized_residuals_to_c(&unresolved.raim))
                .unwrap_or_default()
        });
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            &rows,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
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
    let mut o = RobustConfig::default();
    o.huber_k = config.huber_k;
    o.scale_floor_m = config.scale_floor_m;
    o.max_outer = config.max_outer;
    o.outer_tol_m = config.outer_tol_m;
    o
}

fn default_fde_options() -> SidereonFdeOptions {
    let defaults = FdeOptions::default();
    SidereonFdeOptions {
        p_fa: defaults.raim.p_fa,
        max_exclusions: defaults.max_exclusions,
        max_exclusion_rms_m: defaults.max_exclusion_rms_m,
        weights_mode: SidereonRaimWeightsMode::Solution as u32,
        weights: ptr::null(),
        weight_count: 0,
        n_systems_enabled: false,
        n_systems: 0,
        use_validation_options: false,
        validation: default_validation_options(),
    }
}

/// The engine FDE options the C record states.
unsafe fn fde_spp_options_from_c(
    fn_name: &str,
    options: &SidereonFdeOptions,
) -> Result<FdeSppOptions, SidereonStatus> {
    let raim = raim_options_from_fde_c(fn_name, options)?;
    let validation = validation_options_from_c(options.use_validation_options, &options.validation);
    let mut fde = FdeOptions::new(raim, options.max_exclusions);
    fde.max_exclusion_rms_m = options.max_exclusion_rms_m;
    Ok(FdeSppOptions::new(fde, validation))
}

/// Write an FDE outcome into a newly owned handle, or map its failure. A fault
/// the loop could not resolve keeps its last solution, exclusions and detection
/// test for sidereon_last_fde_unresolved.
unsafe fn fde_outcome_to_c(
    fn_name: &str,
    outcome: Result<FdeResult<ReceiverSolution>, FdeError<ReceiverSolution, FdeSppError>>,
    out_solution: *mut *mut SidereonFdeSolution,
) -> SidereonStatus {
    match outcome {
        Ok(found) => {
            write_boxed_handle(
                out_solution,
                SidereonFdeSolution {
                    solution: found.solution,
                    excluded: found.excluded,
                    iterations: found.iterations,
                    raim: found.raim,
                },
            );
            SidereonStatus::Ok
        }
        Err(FdeError::FaultUnresolved(unresolved)) => {
            set_last_error(format!(
                "{fn_name}: RAIM fault unresolved ({:?}) after excluding {:?}, test statistic {}",
                unresolved.reason, unresolved.excluded, unresolved.raim.test_statistic
            ));
            LAST_FDE_UNRESOLVED.with(|slot| *slot.borrow_mut() = Some(*unresolved));
            SidereonStatus::Solve
        }
        Err(FdeError::Solve(FdeSppError::Spp(error))) => {
            set_last_error(format!("{fn_name}: {error}"));
            if ut1_refusal(&error) {
                SidereonStatus::Ut1OutsideCoverage
            } else {
                SidereonStatus::Solve
            }
        }
        Err(FdeError::Solve(FdeSppError::Validation(error))) => {
            set_last_error(format!("{fn_name}: solution validation failed: {error:?}"));
            SidereonStatus::Solve
        }
        Err(FdeError::Raim(error)) => {
            record_quality_error_kind(error);
            set_last_error(format!("{fn_name}: RAIM options invalid: {error:?}"));
            SidereonStatus::InvalidArgument
        }
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
    let fde_options = match fde_spp_options_from_c(fn_name, options) {
        Ok(options) => options,
        Err(status) => return status,
    };
    fde_outcome_to_c(
        fn_name,
        fde_spp(eph, &inputs, with_geodetic, &fde_options),
        out_solution,
    )
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
    let fde_options = match fde_spp_options_from_c(fn_name, options) {
        Ok(options) => options,
        Err(status) => return status,
    };
    fde_outcome_to_c(
        fn_name,
        spp_robust_fde_driver(eph, &inputs, with_geodetic, robust, &fde_options),
        out_solution,
    )
}

/// Build the engine [`RaimOptions`] from the C FDE options.
unsafe fn raim_options_from_fde_c(
    fn_name: &str,
    options: &SidereonFdeOptions,
) -> Result<RaimOptions, SidereonStatus> {
    let weights = crate::raim::raim_weights_from_c(
        fn_name,
        options.weights_mode,
        options.weights,
        options.weight_count,
    )?;
    let n_systems = options
        .n_systems_enabled
        .then_some(options.n_systems as isize);
    let mut o = RaimOptions::default();
    o.p_fa = options.p_fa;
    o.weights = weights;
    o.n_systems = n_systems;
    Ok(o)
}

fn fde_unresolved_reason_to_c(reason: FdeUnresolvedReason) -> SidereonFdeUnresolvedReason {
    match reason {
        FdeUnresolvedReason::ExclusionBudgetExhausted => {
            SidereonFdeUnresolvedReason::ExclusionBudgetExhausted
        }
        FdeUnresolvedReason::NoAdmissibleExclusion => {
            SidereonFdeUnresolvedReason::NoAdmissibleExclusion
        }
        // `FdeUnresolvedReason` is non-exhaustive; the message names the
        // variant a later engine adds.
        _ => SidereonFdeUnresolvedReason::Unknown,
    }
}
