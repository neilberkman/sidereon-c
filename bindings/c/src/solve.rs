use super::*;

/// Solve a static multi-epoch float RTK baseline. On success writes a newly
/// owned solution handle to *out_solution. Release it with
/// sidereon_rtk_float_solution_free.
///
/// Safety: config must point to a valid SidereonRtkFloatConfig and
/// out_solution must point to storage for a SidereonRtkFloatSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solve_rtk_float(
    config: *const SidereonRtkFloatConfig,
    out_solution: *mut *mut SidereonRtkFloatSolution,
) -> SidereonStatus {
    ffi_boundary("sidereon_solve_rtk_float", SidereonStatus::Panic, || {
        let out_solution = c_try!(require_out(
            out_solution,
            "sidereon_solve_rtk_float",
            "out_solution"
        ));
        *out_solution = ptr::null_mut();
        let config = c_try!(require_ref(config, "sidereon_solve_rtk_float", "config"));
        let epochs = c_try!(rtk_epochs_from_c(
            "sidereon_solve_rtk_float",
            config.epochs,
            config.epoch_count,
        ));
        let ambiguity_ids = c_try!(rtk_id_list_from_c(
            "sidereon_solve_rtk_float",
            config.ambiguity_ids,
            config.ambiguity_id_count,
            "ambiguity_ids",
        ));
        let model = c_try!(rtk_model_from_c("sidereon_solve_rtk_float", &config.model,));
        let receiver_antenna = c_try!(rtk_receiver_antenna_from_c(
            "sidereon_solve_rtk_float",
            config.receiver_antenna,
        ));
        let opts = rtk_float_options_from_c(&config.options);

        let inner = c_try!(guard(SidereonStatus::Solve, || {
            sidereon::solve_rtk_float(
                &epochs,
                config.base_ecef_m,
                &ambiguity_ids,
                config.initial_baseline_m,
                &model,
                opts,
                receiver_antenna.as_ref(),
            )
        }));
        write_boxed_handle(
            out_solution,
            SidereonRtkFloatSolution {
                inner,
                base_ecef_m: config.base_ecef_m,
            },
        );
        SidereonStatus::Ok
    })
}

/// Solve a static integer-fixed RTK baseline with residual validation. On
/// success writes a newly owned solution handle to *out_solution. Release it
/// with sidereon_rtk_fixed_solution_free.
///
/// Safety: config must point to a valid SidereonRtkFixedConfig and
/// out_solution must point to storage for a SidereonRtkFixedSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solve_rtk_fixed(
    config: *const SidereonRtkFixedConfig,
    out_solution: *mut *mut SidereonRtkFixedSolution,
) -> SidereonStatus {
    ffi_boundary("sidereon_solve_rtk_fixed", SidereonStatus::Panic, || {
        let out_solution = c_try!(require_out(
            out_solution,
            "sidereon_solve_rtk_fixed",
            "out_solution"
        ));
        *out_solution = ptr::null_mut();
        let config = c_try!(require_ref(config, "sidereon_solve_rtk_fixed", "config"));
        let epochs = c_try!(rtk_epochs_from_c(
            "sidereon_solve_rtk_fixed",
            config.epochs,
            config.epoch_count,
        ));
        let ambiguity_ids = c_try!(rtk_id_list_from_c(
            "sidereon_solve_rtk_fixed",
            config.ambiguity_ids,
            config.ambiguity_id_count,
            "ambiguity_ids",
        ));
        let ambiguity_satellites = c_try!(rtk_ambiguity_satellite_map_from_c(
            "sidereon_solve_rtk_fixed",
            config.ambiguity_satellites,
            config.ambiguity_satellite_count,
        ));
        let wavelengths_m = c_try!(rtk_f64_map_from_c(
            "sidereon_solve_rtk_fixed",
            config.wavelengths_m,
            config.wavelength_count,
            "wavelengths_m",
        ));
        let offsets_m = c_try!(rtk_f64_map_from_c(
            "sidereon_solve_rtk_fixed",
            config.offsets_m,
            config.offset_count,
            "offsets_m",
        ));
        let model = c_try!(rtk_model_from_c("sidereon_solve_rtk_fixed", &config.model,));
        let receiver_antenna = c_try!(rtk_receiver_antenna_from_c(
            "sidereon_solve_rtk_fixed",
            config.receiver_antenna,
        ));
        let float_only_systems = c_try!(rtk_float_only_systems_from_c(
            "sidereon_solve_rtk_fixed",
            config.float_only_systems,
            config.float_only_system_count,
        ));
        let ambiguities = AmbiguitySet {
            ids: &ambiguity_ids,
            satellites: &ambiguity_satellites,
            scale: AmbiguityScale {
                wavelengths_m: &wavelengths_m,
                offsets_m: &offsets_m,
            },
            float_only_systems: &float_only_systems,
        };
        let opts = ValidatedFixedSolveOpts {
            float: rtk_float_options_from_c(&config.float_options),
            fixed: rtk_fixed_options_from_c(&config.fixed_options),
            residual: rtk_residual_options_from_c(&config.residual_options),
        };

        let inner = c_try!(guard(SidereonStatus::Solve, || {
            sidereon::solve_rtk_fixed(
                &epochs,
                config.base_ecef_m,
                ambiguities,
                config.initial_baseline_m,
                &model,
                opts,
                receiver_antenna.as_ref(),
            )
        }));
        write_boxed_handle(
            out_solution,
            SidereonRtkFixedSolution {
                inner,
                base_ecef_m: config.base_ecef_m,
            },
        );
        SidereonStatus::Ok
    })
}

/// Solve a static multi-epoch float PPP arc. On success writes a newly owned
/// solution handle to *out_solution. Release it with
/// sidereon_ppp_float_solution_free.
///
/// Safety: sp3 and config must point to live handles/structs, and out_solution
/// must point to storage for a SidereonPppFloatSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solve_ppp_float(
    sp3: *const SidereonSp3,
    config: *const SidereonPppFloatConfig,
    out_solution: *mut *mut SidereonPppFloatSolution,
) -> SidereonStatus {
    ffi_boundary("sidereon_solve_ppp_float", SidereonStatus::Panic, || {
        let out_solution = c_try!(require_out(
            out_solution,
            "sidereon_solve_ppp_float",
            "out_solution"
        ));
        *out_solution = ptr::null_mut();
        let sp3 = c_try!(require_ref(sp3, "sidereon_solve_ppp_float", "sp3"));
        let config = c_try!(require_ref(config, "sidereon_solve_ppp_float", "config"));
        let epochs = c_try!(ppp_epochs_from_c(
            "sidereon_solve_ppp_float",
            config.epochs,
            config.epoch_count,
        ));
        let initial_state = c_try!(ppp_float_state_from_c(
            "sidereon_solve_ppp_float",
            &config.initial_state,
            epochs.len(),
        ));
        let solve_config = c_try!(ppp_float_config_from_c("sidereon_solve_ppp_float", config));
        let inner = c_try!(guard(SidereonStatus::Solve, || {
            sidereon::solve_ppp_float(&sp3.inner, &epochs, initial_state, solve_config)
        }));
        write_boxed_handle(out_solution, SidereonPppFloatSolution { inner });
        SidereonStatus::Ok
    })
}

/// Solve a static integer-fixed PPP arc from a PPP float solution. On success
/// writes a newly owned solution handle to *out_solution. Release it with
/// sidereon_ppp_fixed_solution_free.
///
/// Safety: sp3, float_solution, and config must point to live handles/structs,
/// and out_solution must point to storage for a SidereonPppFixedSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solve_ppp_fixed(
    sp3: *const SidereonSp3,
    float_solution: *const SidereonPppFloatSolution,
    config: *const SidereonPppFixedConfig,
    out_solution: *mut *mut SidereonPppFixedSolution,
) -> SidereonStatus {
    ffi_boundary("sidereon_solve_ppp_fixed", SidereonStatus::Panic, || {
        let out_solution = c_try!(require_out(
            out_solution,
            "sidereon_solve_ppp_fixed",
            "out_solution"
        ));
        *out_solution = ptr::null_mut();
        let sp3 = c_try!(require_ref(sp3, "sidereon_solve_ppp_fixed", "sp3"));
        let float_solution = c_try!(require_ref(
            float_solution,
            "sidereon_solve_ppp_fixed",
            "float_solution"
        ));
        let config = c_try!(require_ref(config, "sidereon_solve_ppp_fixed", "config"));
        let epochs = c_try!(ppp_epochs_from_c(
            "sidereon_solve_ppp_fixed",
            config.epochs,
            config.epoch_count,
        ));
        if epochs.len() != float_solution.inner.epoch_clocks_m.len() {
            set_last_error(
                "sidereon_solve_ppp_fixed: epoch_count must match the float solution".to_owned(),
            );
            return SidereonStatus::InvalidArgument;
        }
        let solve_config = c_try!(ppp_fixed_config_from_c("sidereon_solve_ppp_fixed", config));
        let inner = c_try!(guard(SidereonStatus::Solve, || {
            sidereon::solve_ppp_fixed(
                &sp3.inner,
                &epochs,
                float_solution.inner.clone(),
                solve_config,
            )
        }));
        write_boxed_handle(out_solution, SidereonPppFixedSolution { inner });
        SidereonStatus::Ok
    })
}

/// Run single-point positioning. On success writes a newly owned solution handle
/// to *out_solution. Release it with sidereon_spp_solution_free.
///
/// Safety: sp3 must be a live handle; inputs must point to a valid
/// SidereonSppInputs whose observations field points to observation_count valid
/// entries with null-terminated sat_id values whose terminator appears within
/// 16 bytes; out_solution must point to storage for a SidereonSppSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solve_spp(
    sp3: *const SidereonSp3,
    inputs: *const SidereonSppInputs,
    out_solution: *mut *mut SidereonSppSolution,
) -> SidereonStatus {
    ffi_boundary("sidereon_solve_spp", SidereonStatus::Panic, || {
        let out_solution = c_try!(require_out(
            out_solution,
            "sidereon_solve_spp",
            "out_solution"
        ));
        *out_solution = ptr::null_mut();
        let sp3 = c_try!(require_ref(sp3, "sidereon_solve_spp", "sp3"));
        let inputs = c_try!(require_ref(inputs, "sidereon_solve_spp", "inputs"));
        // The legacy entry point exposes no GLONASS channel array, so it solves
        // with an empty channel map: GPS/Galileo/BeiDou stay bit-identical and a
        // GLONASS observation with the ionosphere correction is rejected by the
        // engine. Use sidereon_solve_spp_v2 to supply channels.
        let solve_inputs = c_try!(build_spp_solve_inputs(
            "sidereon_solve_spp",
            inputs,
            None,
            None,
            BTreeMap::new(),
        ));

        let inner = c_try!(guard(SidereonStatus::Solve, || {
            sidereon::solve_spp(
                &sp3.inner,
                &solve_inputs,
                inputs.with_geodetic,
                SolvePolicy::default(),
            )
        }));
        write_boxed_handle(out_solution, SidereonSppSolution { inner });
        SidereonStatus::Ok
    })
}

/// Run single-point positioning with extended V2 controls. On success writes a
/// newly owned solution handle to *out_solution. Release it with
/// sidereon_spp_solution_free.
///
/// Safety: sp3 must be a live handle; inputs must point to a valid
/// SidereonSppInputsV2 whose base observations field points to
/// observation_count valid entries with null-terminated sat_id values whose
/// terminator appears within 16 bytes; out_solution must point to storage for a
/// SidereonSppSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solve_spp_v2(
    sp3: *const SidereonSp3,
    inputs: *const SidereonSppInputsV2,
    out_solution: *mut *mut SidereonSppSolution,
) -> SidereonStatus {
    ffi_boundary("sidereon_solve_spp_v2", SidereonStatus::Panic, || {
        let out_solution = c_try!(require_out(
            out_solution,
            "sidereon_solve_spp_v2",
            "out_solution"
        ));
        *out_solution = ptr::null_mut();
        let sp3 = c_try!(require_ref(sp3, "sidereon_solve_spp_v2", "sp3"));
        let inputs = c_try!(require_ref(inputs, "sidereon_solve_spp_v2", "inputs"));
        let glonass_channels = c_try!(glonass_channels_from_c("sidereon_solve_spp_v2", inputs));
        let solve_inputs = c_try!(build_spp_solve_inputs(
            "sidereon_solve_spp_v2",
            &inputs.base,
            beidou_klobuchar_from_c(inputs),
            robust_config_from_c(inputs),
            glonass_channels,
        ));
        let policy = c_try!(solve_policy_from_c("sidereon_solve_spp_v2", &inputs.policy));

        let inner = c_try!(guard(SidereonStatus::Solve, || {
            sidereon::solve_spp(&sp3.inner, &solve_inputs, inputs.base.with_geodetic, policy)
        }));
        write_boxed_handle(out_solution, SidereonSppSolution { inner });
        SidereonStatus::Ok
    })
}

/// Run single-point positioning from broadcast ephemeris ALONE: the supported
/// real-time / offline mode. On success writes a newly owned solution handle to
/// *out_solution (a SidereonSppSolution usable with the spp solution accessors
/// and released with sidereon_spp_solution_free). The legacy SidereonSppInputs
/// ABI is used, so this path supplies no GLONASS channels or BeiDou Klobuchar
/// coefficients, matching sidereon_solve_spp.
///
/// Safety: broadcast must be a live handle; inputs must point to a valid
/// SidereonSppInputs whose observations field points to observation_count valid
/// entries with bounded null-terminated sat_id values; out_solution must point to
/// storage for a SidereonSppSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solve_broadcast(
    broadcast: *const SidereonBroadcastEphemeris,
    inputs: *const SidereonSppInputs,
    out_solution: *mut *mut SidereonSppSolution,
) -> SidereonStatus {
    ffi_boundary("sidereon_solve_broadcast", SidereonStatus::Panic, || {
        let out_solution = c_try!(require_out(
            out_solution,
            "sidereon_solve_broadcast",
            "out_solution"
        ));
        *out_solution = ptr::null_mut();
        let broadcast = c_try!(require_ref(
            broadcast,
            "sidereon_solve_broadcast",
            "broadcast"
        ));
        let inputs = c_try!(require_ref(inputs, "sidereon_solve_broadcast", "inputs"));
        let solve_inputs = c_try!(build_spp_solve_inputs(
            "sidereon_solve_broadcast",
            inputs,
            None,
            None,
            BTreeMap::new(),
        ));
        let inner = match solve_broadcast(&broadcast.inner, &solve_inputs, inputs.with_geodetic) {
            Ok(solution) => solution,
            Err(err) => {
                set_last_error(format!("sidereon_solve_broadcast: {err}"));
                return SidereonStatus::Solve;
            }
        };
        write_boxed_handle(out_solution, SidereonSppSolution { inner });
        SidereonStatus::Ok
    })
}

/// Solve a receiver position, preferring precise SP3 products and falling back to
/// broadcast ephemeris, reporting which source was used and how stale it is. On
/// success writes a newly owned SidereonSourcedSolution to *out_solution (release
/// with sidereon_sourced_solution_free); query the provenance with the
/// sidereon_sourced_solution_* accessors and copy the receiver solution out with
/// sidereon_sourced_solution_solution.
///
/// Safety: precise must point to precise_count readable SidereonSp3 pointers (or
/// be NULL when precise_count is 0); broadcast and inputs must be live; out_solution
/// must point to storage for a SidereonSourcedSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solve_with_fallback(
    precise: *const *const SidereonSp3,
    precise_count: usize,
    broadcast: *const SidereonBroadcastEphemeris,
    inputs: *const SidereonSppInputs,
    policy: SidereonStalenessPolicy,
    out_solution: *mut *mut SidereonSourcedSolution,
) -> SidereonFallbackStatus {
    ffi_boundary(
        "sidereon_solve_with_fallback",
        SidereonFallbackStatus::Panic,
        || {
            let out_solution = fb_try!(require_out(
                out_solution,
                "sidereon_solve_with_fallback",
                "out_solution"
            ));
            *out_solution = ptr::null_mut();
            let broadcast = fb_try!(require_ref(
                broadcast,
                "sidereon_solve_with_fallback",
                "broadcast"
            ));
            let inputs = fb_try!(require_ref(
                inputs,
                "sidereon_solve_with_fallback",
                "inputs"
            ));
            let set = fb_try!(sp3_products_from_c(
                "sidereon_solve_with_fallback",
                precise,
                precise_count
            ));
            let solve_inputs = fb_try!(build_spp_solve_inputs(
                "sidereon_solve_with_fallback",
                inputs,
                None,
                None,
                BTreeMap::new(),
            ));
            let policy = StalenessPolicy {
                max_staleness_s: policy.max_staleness_s,
            };
            match solve_with_fallback(
                &set,
                &broadcast.inner,
                &solve_inputs,
                policy,
                inputs.with_geodetic,
            ) {
                Ok(sourced) => {
                    write_boxed_handle(
                        out_solution,
                        SidereonSourcedSolution {
                            solution: sourced.solution,
                            source: sourced.source,
                        },
                    );
                    SidereonFallbackStatus::Ok
                }
                Err(err) => {
                    set_last_error(format!("sidereon_solve_with_fallback: {err}"));
                    match err {
                        FallbackError::Precise(_) => SidereonFallbackStatus::PreciseSolve,
                        FallbackError::Broadcast(_) => SidereonFallbackStatus::BroadcastSolve,
                    }
                }
            }
        },
    )
}

/// Solve a moving-baseline RTK arc: each epoch carries its own base ECEF
/// position. On success writes a newly owned solution handle to *out_solution.
/// Release it with sidereon_moving_baseline_solution_free. Delegates to
/// sidereon_core::rtk_filter::moving_baseline::solve_moving_baseline (the
/// single-epoch solver is reachable with epoch_count = 1).
///
/// Safety: config points to a valid SidereonMovingBaselineConfig and
/// out_solution points to a SidereonMovingBaselineSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solve_moving_baseline(
    config: *const SidereonMovingBaselineConfig,
    out_solution: *mut *mut SidereonMovingBaselineSolution,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_solve_moving_baseline",
        SidereonStatus::Panic,
        || {
            let out_solution = c_try!(require_out(
                out_solution,
                "sidereon_solve_moving_baseline",
                "out_solution"
            ));
            *out_solution = ptr::null_mut();
            let config = c_try!(require_ref(
                config,
                "sidereon_solve_moving_baseline",
                "config"
            ));
            let epochs = c_try!(run_moving_baseline(config));
            write_boxed_handle(out_solution, SidereonMovingBaselineSolution { epochs });
            SidereonStatus::Ok
        },
    )
}

/// Solve a static multi-epoch float PPP arc from raw epochs, auto-initializing
/// the float state from the SPP seed (no caller initial FloatState needed). The
/// epochs and float settings come from `float_config` (its `initial_state` is
/// ignored). On success writes a newly owned solution handle to *out_solution;
/// release it with sidereon_ppp_float_solution_free. Delegates to
/// sidereon_core::precise_positioning::solve_ppp_auto_init_float.
///
/// Safety: sp3, float_config, and options must point to live handles/structs;
/// out_solution must point to storage for a SidereonPppFloatSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solve_ppp_auto_init_float(
    sp3: *const SidereonSp3,
    float_config: *const SidereonPppFloatConfig,
    options: *const SidereonPppAutoInitOptions,
    out_solution: *mut *mut SidereonPppFloatSolution,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_solve_ppp_auto_init_float",
        SidereonStatus::Panic,
        || {
            let out_solution = c_try!(require_out(
                out_solution,
                "sidereon_solve_ppp_auto_init_float",
                "out_solution"
            ));
            *out_solution = ptr::null_mut();
            let sp3 = c_try!(require_ref(
                sp3,
                "sidereon_solve_ppp_auto_init_float",
                "sp3"
            ));
            let float_config = c_try!(require_ref(
                float_config,
                "sidereon_solve_ppp_auto_init_float",
                "float_config"
            ));
            let options = c_try!(require_ref(
                options,
                "sidereon_solve_ppp_auto_init_float",
                "options"
            ));
            let epochs = c_try!(ppp_epochs_from_c(
                "sidereon_solve_ppp_auto_init_float",
                float_config.epochs,
                float_config.epoch_count,
            ));
            let solve_config = c_try!(ppp_float_config_from_c(
                "sidereon_solve_ppp_auto_init_float",
                float_config
            ));
            let auto_options = ppp_auto_init_options_from_c(options);
            let inner =
                match solve_ppp_auto_init_float(&sp3.inner, &epochs, auto_options, solve_config) {
                    Ok(inner) => inner,
                    Err(err) => {
                        set_last_error(format!("sidereon_solve_ppp_auto_init_float: {err}"));
                        return SidereonStatus::Solve;
                    }
                };
            write_boxed_handle(out_solution, SidereonPppFloatSolution { inner });
            SidereonStatus::Ok
        },
    )
}

/// Solve a static integer-fixed PPP arc from raw epochs: the SPP auto-init seed,
/// the float solve, then the LAMBDA integer fix and ambiguity-conditioned
/// re-solve. The epochs and float settings come from `float_config`; the
/// ambiguity and fixed settings from `fixed_config` (both configs' epoch lists
/// must describe the same arc, and the auto-init reads `float_config`). On
/// success writes a newly owned solution handle to *out_solution; release it with
/// sidereon_ppp_fixed_solution_free. Delegates to
/// sidereon_core::precise_positioning::solve_ppp_auto_init_fixed.
///
/// Safety: sp3, float_config, fixed_config, and options must point to live
/// handles/structs; out_solution must point to storage for a
/// SidereonPppFixedSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solve_ppp_auto_init_fixed(
    sp3: *const SidereonSp3,
    float_config: *const SidereonPppFloatConfig,
    fixed_config: *const SidereonPppFixedConfig,
    options: *const SidereonPppAutoInitOptions,
    out_solution: *mut *mut SidereonPppFixedSolution,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_solve_ppp_auto_init_fixed",
        SidereonStatus::Panic,
        || {
            let out_solution = c_try!(require_out(
                out_solution,
                "sidereon_solve_ppp_auto_init_fixed",
                "out_solution"
            ));
            *out_solution = ptr::null_mut();
            let sp3 = c_try!(require_ref(
                sp3,
                "sidereon_solve_ppp_auto_init_fixed",
                "sp3"
            ));
            let float_config = c_try!(require_ref(
                float_config,
                "sidereon_solve_ppp_auto_init_fixed",
                "float_config"
            ));
            let fixed_config = c_try!(require_ref(
                fixed_config,
                "sidereon_solve_ppp_auto_init_fixed",
                "fixed_config"
            ));
            let options = c_try!(require_ref(
                options,
                "sidereon_solve_ppp_auto_init_fixed",
                "options"
            ));
            let epochs = c_try!(ppp_epochs_from_c(
                "sidereon_solve_ppp_auto_init_fixed",
                float_config.epochs,
                float_config.epoch_count,
            ));
            let float_solve_config = c_try!(ppp_float_config_from_c(
                "sidereon_solve_ppp_auto_init_fixed",
                float_config
            ));
            let fixed_solve_config = c_try!(ppp_fixed_config_from_c(
                "sidereon_solve_ppp_auto_init_fixed",
                fixed_config
            ));
            let auto_options = ppp_auto_init_options_from_c(options);
            let inner = match solve_ppp_auto_init_fixed(
                &sp3.inner,
                &epochs,
                auto_options,
                float_solve_config,
                fixed_solve_config,
            ) {
                Ok(inner) => inner,
                Err(err) => {
                    set_last_error(format!("sidereon_solve_ppp_auto_init_fixed: {err}"));
                    return SidereonStatus::Solve;
                }
            };
            write_boxed_handle(out_solution, SidereonPppFixedSolution { inner });
            SidereonStatus::Ok
        },
    )
}

/// Solve a sequential RTK baseline arc from raw rover+base epochs. On success
/// writes a newly owned solution handle to *out_solution. Release it with
/// sidereon_rtk_arc_solution_free. Delegates to
/// sidereon_core::rtk_filter::solve_rtk_arc.
///
/// Safety: epochs points to epoch_count SidereonRtkArcEpoch (or NULL when 0);
/// config points to a SidereonRtkArcConfig; out_solution to a
/// SidereonRtkArcSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solve_rtk_arc(
    epochs: *const SidereonRtkArcEpoch,
    epoch_count: usize,
    config: *const SidereonRtkArcConfig,
    out_solution: *mut *mut SidereonRtkArcSolution,
) -> SidereonStatus {
    ffi_boundary("sidereon_solve_rtk_arc", SidereonStatus::Panic, || {
        let out_solution = c_try!(require_out(
            out_solution,
            "sidereon_solve_rtk_arc",
            "out_solution"
        ));
        *out_solution = ptr::null_mut();
        let config = c_try!(require_ref(config, "sidereon_solve_rtk_arc", "config"));
        let core_config = c_try!(rtk_arc_config_from_c("sidereon_solve_rtk_arc", config));
        let core_epochs = c_try!(rtk_arc_epochs_from_c(
            "sidereon_solve_rtk_arc",
            epochs,
            epoch_count,
            core_config.preprocessing.cycle_slip.is_some(),
        ));
        match solve_rtk_arc(&core_epochs, &core_config) {
            Ok(inner) => {
                write_boxed_handle(out_solution, SidereonRtkArcSolution { inner });
                SidereonStatus::Ok
            }
            Err(err) => map_rtk_arc_error("sidereon_solve_rtk_arc", &err),
        }
    })
}

/// Solve one static float+fixed RTK baseline over a raw rover+base arc. On
/// success writes a newly owned solution handle to *out_solution. Release it
/// with sidereon_rtk_static_arc_solution_free. Delegates to
/// sidereon_core::rtk_filter::solve_static_rtk_arc.
///
/// Safety: epochs points to epoch_count SidereonRtkArcEpoch (or NULL when 0);
/// config points to a SidereonRtkStaticArcConfig; out_solution to a
/// SidereonRtkStaticArcSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solve_static_rtk_arc(
    epochs: *const SidereonRtkArcEpoch,
    epoch_count: usize,
    config: *const SidereonRtkStaticArcConfig,
    out_solution: *mut *mut SidereonRtkStaticArcSolution,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_solve_static_rtk_arc",
        SidereonStatus::Panic,
        || {
            let out_solution = c_try!(require_out(
                out_solution,
                "sidereon_solve_static_rtk_arc",
                "out_solution"
            ));
            *out_solution = ptr::null_mut();
            let config = c_try!(require_ref(
                config,
                "sidereon_solve_static_rtk_arc",
                "config"
            ));
            let core_config = c_try!(rtk_static_arc_config_from_c(
                "sidereon_solve_static_rtk_arc",
                config
            ));
            let core_epochs = c_try!(rtk_arc_epochs_from_c(
                "sidereon_solve_static_rtk_arc",
                epochs,
                epoch_count,
                core_config.arc.preprocessing.cycle_slip.is_some(),
            ));
            match solve_static_rtk_arc(&core_epochs, &core_config) {
                Ok(inner) => {
                    write_boxed_handle(out_solution, SidereonRtkStaticArcSolution { inner });
                    SidereonStatus::Ok
                }
                Err(err) => map_rtk_static_arc_error("sidereon_solve_static_rtk_arc", &err),
            }
        },
    )
}

/// Solve a batch of independent SPP epochs serially over a shared ephemeris. On
/// success writes a newly owned batch handle to *out_batch. Delegates to
/// sidereon_core::spp::solve_spp_batch_serial.
///
/// Safety: sp3 is a live handle; inputs points to input_count SidereonSppInputsV2
/// (or NULL when 0); policy points to a SidereonSppSolvePolicy; out_batch points
/// to storage for a SidereonSppBatch*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solve_spp_batch_serial(
    sp3: *const SidereonSp3,
    inputs: *const SidereonSppInputsV2,
    input_count: usize,
    with_geodetic: bool,
    policy: *const SidereonSppSolvePolicy,
    out_batch: *mut *mut SidereonSppBatch,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_solve_spp_batch_serial",
        SidereonStatus::Panic,
        || {
            solve_spp_batch_impl(
                "sidereon_solve_spp_batch_serial",
                sp3,
                inputs,
                input_count,
                with_geodetic,
                policy,
                out_batch,
                false,
            )
        },
    )
}

/// Solve a batch of independent SPP epochs in parallel over a shared ephemeris.
/// Element i is byte-for-byte identical to the serial element i. On success
/// writes a newly owned batch handle to *out_batch. Delegates to
/// sidereon_core::spp::solve_spp_batch_parallel.
///
/// Safety: sp3 is a live handle; inputs points to input_count SidereonSppInputsV2
/// (or NULL when 0); policy points to a SidereonSppSolvePolicy; out_batch points
/// to storage for a SidereonSppBatch*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solve_spp_batch_parallel(
    sp3: *const SidereonSp3,
    inputs: *const SidereonSppInputsV2,
    input_count: usize,
    with_geodetic: bool,
    policy: *const SidereonSppSolvePolicy,
    out_batch: *mut *mut SidereonSppBatch,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_solve_spp_batch_parallel",
        SidereonStatus::Panic,
        || {
            solve_spp_batch_impl(
                "sidereon_solve_spp_batch_parallel",
                sp3,
                inputs,
                input_count,
                with_geodetic,
                policy,
                out_batch,
                true,
            )
        },
    )
}

/// Solve a data-driven least-squares problem, transferring a solution handle to
/// `*out_solution`. Delegates to the core `solve_data_problem` (native backend)
/// or `solve_data_problem_with` (host-LAPACK backend). The trust-region loop
/// runs entirely in the core; read the result with the
/// sidereon_trls_solution_* accessors and release it with
/// sidereon_trls_solution_free.
///
/// Safety: problem must point to a SidereonDataProblem whose data pointers are
/// valid for their stated lengths; out_solution must point to storage for a
/// SidereonTrlsSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solve_data_problem(
    problem: *const SidereonDataProblem,
    out_solution: *mut *mut SidereonTrlsSolution,
) -> SidereonStatus {
    ffi_boundary("sidereon_solve_data_problem", SidereonStatus::Panic, || {
        let out_solution = c_try!(require_out(
            out_solution,
            "sidereon_solve_data_problem",
            "out_solution"
        ));
        *out_solution = ptr::null_mut();
        let backend_raw = c_try!(require_ref(
            problem,
            "sidereon_solve_data_problem",
            "problem"
        ))
        .backend;
        let backend = c_try!(trls_backend_from_c(
            "sidereon_solve_data_problem",
            "backend",
            backend_raw
        ));
        let parsed = c_try!(data_problem_from_c("sidereon_solve_data_problem", problem));
        let result = match backend {
            SidereonTrlsBackend::Native => core_solve_data_problem(&parsed),
            SidereonTrlsBackend::HostLapack => {
                core_solve_data_problem_with(&parsed, &LapackSvd::from_env())
            }
        };
        let inner = match result {
            Ok(inner) => inner,
            Err(err) => return map_trls_error("sidereon_solve_data_problem", err),
        };
        write_boxed_handle(out_solution, SidereonTrlsSolution { inner });
        SidereonStatus::Ok
    })
}

/// Solve a data-driven problem and every leave-one-out re-solve (RAIM / FDE),
/// transferring a report handle to *out_report. Delegates to the core
/// `solve_data_problem_drop_one` (native) or `_with` (host-LAPACK). The report
/// carries the base solve, one re-solve per masked residual row, and the cost
/// delta for each masked row.
///
/// Safety: problem must point to a SidereonDataProblem whose data pointers are
/// valid for their stated lengths; out_report must point to storage for a
/// SidereonTrlsDropOne*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solve_data_problem_drop_one(
    problem: *const SidereonDataProblem,
    out_report: *mut *mut SidereonTrlsDropOne,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_solve_data_problem_drop_one",
        SidereonStatus::Panic,
        || {
            let out_report = c_try!(require_out(
                out_report,
                "sidereon_solve_data_problem_drop_one",
                "out_report"
            ));
            *out_report = ptr::null_mut();
            let backend_raw = c_try!(require_ref(
                problem,
                "sidereon_solve_data_problem_drop_one",
                "problem"
            ))
            .backend;
            let backend = c_try!(trls_backend_from_c(
                "sidereon_solve_data_problem_drop_one",
                "backend",
                backend_raw
            ));
            let parsed = c_try!(data_problem_from_c(
                "sidereon_solve_data_problem_drop_one",
                problem
            ));
            let result = match backend {
                SidereonTrlsBackend::Native => core_solve_drop_one(&parsed),
                SidereonTrlsBackend::HostLapack => {
                    core_solve_drop_one_with(&parsed, &LapackSvd::from_env())
                }
            };
            let inner = match result {
                Ok(inner) => inner,
                Err(err) => return map_trls_error("sidereon_solve_data_problem_drop_one", err),
            };
            write_boxed_handle(out_report, SidereonTrlsDropOne { inner });
            SidereonStatus::Ok
        },
    )
}

unsafe fn ppp_epochs_from_c(
    fn_name: &str,
    epochs: *const SidereonPppEpoch,
    epoch_count: usize,
) -> Result<Vec<PppFloatEpoch>, SidereonStatus> {
    let raw_epochs = require_slice(epochs, epoch_count, fn_name, "epochs")?;
    validate_element_count::<PppFloatEpoch>(fn_name, "epoch_count", raw_epochs.len())?;
    let mut out = Vec::with_capacity(raw_epochs.len());
    for (epoch_idx, epoch) in raw_epochs.iter().enumerate() {
        let observations = require_slice(
            epoch.observations,
            epoch.observation_count,
            fn_name,
            &format!("epochs[{epoch_idx}].observations"),
        )?;
        validate_element_count::<PppFloatObservation>(
            fn_name,
            &format!("epochs[{epoch_idx}].observation_count"),
            observations.len(),
        )?;
        let mut core_observations = Vec::with_capacity(observations.len());
        for row in observations {
            core_observations.push(ppp_observation_from_c(fn_name, row)?);
        }
        out.push(PppFloatEpoch {
            epoch: CivilDateTime {
                year: epoch.civil.year,
                month: epoch.civil.month,
                day: epoch.civil.day,
                hour: epoch.civil.hour,
                minute: epoch.civil.minute,
                second: epoch.civil.second,
            },
            jd_whole: epoch.jd_whole,
            jd_fraction: epoch.jd_fraction,
            t_rx_j2000_s: epoch.t_rx_j2000_s,
            observations: core_observations,
        });
    }
    Ok(out)
}

unsafe fn ppp_float_state_from_c(
    fn_name: &str,
    state: &SidereonPppFloatState,
    epoch_count: usize,
) -> Result<PppFloatStateInner, SidereonStatus> {
    let clocks = require_slice(
        state.clocks_m,
        state.clock_count,
        fn_name,
        "initial_state.clocks_m",
    )?;
    validate_element_count::<f64>(fn_name, "initial_state.clock_count", clocks.len())?;
    if clocks.len() != epoch_count {
        set_last_error(format!(
            "{fn_name}: initial_state.clock_count must equal epoch_count"
        ));
        return Err(SidereonStatus::InvalidArgument);
    }
    let ambiguities_m = ppp_f64_map_from_c(
        fn_name,
        state.ambiguities_m,
        state.ambiguity_count,
        "initial_state.ambiguities_m",
    )?;
    Ok(PppFloatStateInner {
        position_m: state.position_m,
        clocks_m: clocks.to_vec(),
        ambiguities_m,
        ztd_m: state.ztd_m,
        tropo_gradient_north_m: state.tropo_gradient_north_m,
        tropo_gradient_east_m: state.tropo_gradient_east_m,
        residual_ionosphere_m: BTreeMap::new(),
    })
}

unsafe fn ppp_float_config_from_c(
    fn_name: &str,
    config: &SidereonPppFloatConfig,
) -> Result<PppFloatSolveConfigInner, SidereonStatus> {
    Ok(PppFloatSolveConfigInner {
        weights: ppp_weights_from_c(&config.weights),
        tropo: ppp_tropo_from_c(fn_name, &config.tropo)?,
        corrections: ppp_range_corrections_from_c(fn_name, &config.corrections)?,
        opts: ppp_float_options_from_c(&config.options),
        residual_screen: config.residual_screen,
        elevation_cutoff_deg: config
            .has_elevation_cutoff_deg
            .then_some(config.elevation_cutoff_deg),
        estimate_residual_ionosphere: false,
    })
}

unsafe fn ppp_fixed_config_from_c(
    fn_name: &str,
    config: &SidereonPppFixedConfig,
) -> Result<PppFixedSolveConfigInner, SidereonStatus> {
    let wavelengths_m = ppp_f64_map_from_c(
        fn_name,
        config.ambiguity.wavelengths_m,
        config.ambiguity.wavelength_count,
        "ambiguity.wavelengths_m",
    )?;
    let offsets_m = ppp_f64_map_from_c(
        fn_name,
        config.ambiguity.offsets_m,
        config.ambiguity.offset_count,
        "ambiguity.offsets_m",
    )?;
    Ok(PppFixedSolveConfigInner {
        weights: ppp_weights_from_c(&config.weights),
        tropo: ppp_tropo_from_c(fn_name, &config.tropo)?,
        corrections: ppp_range_corrections_from_c(fn_name, &config.corrections)?,
        opts: ppp_float_options_from_c(&config.options),
        ambiguity: PppFixedAmbiguityOptionsInner {
            wavelengths_m,
            offsets_m,
            ratio_threshold: config.ambiguity.ratio_threshold,
        },
        elevation_cutoff_deg: config
            .has_elevation_cutoff_deg
            .then_some(config.elevation_cutoff_deg),
        estimate_residual_ionosphere: false,
    })
}

#[allow(clippy::type_complexity)]
unsafe fn run_moving_baseline(
    config: &SidereonMovingBaselineConfig,
) -> Result<Vec<MovingBaselineEpochSolution>, SidereonStatus> {
    let fn_name = "sidereon_solve_moving_baseline";
    let raw_epochs = require_slice(config.epochs, config.epoch_count, fn_name, "epochs")?;
    validate_element_count::<MovingBaselineEpoch>(fn_name, "epoch_count", raw_epochs.len())?;

    // Owned per-epoch storage, built fully before any MovingBaselineEpoch borrows
    // into it. The borrows below are valid for the duration of the solve.
    let mut bases: Vec<[f64; 3]> = Vec::with_capacity(raw_epochs.len());
    let mut obs_epochs: Vec<RtkEpoch> = Vec::with_capacity(raw_epochs.len());
    let mut ids_per: Vec<Vec<String>> = Vec::with_capacity(raw_epochs.len());
    let mut sats_per: Vec<BTreeMap<String, String>> = Vec::with_capacity(raw_epochs.len());
    let mut wavelengths_per: Vec<BTreeMap<String, f64>> = Vec::with_capacity(raw_epochs.len());
    let mut offsets_per: Vec<BTreeMap<String, f64>> = Vec::with_capacity(raw_epochs.len());
    let mut float_only_per: Vec<Vec<String>> = Vec::with_capacity(raw_epochs.len());

    for epoch in raw_epochs {
        bases.push(epoch.base_position_m);
        let mut single = rtk_epochs_from_c(fn_name, &epoch.epoch, 1)?;
        let observation = single.pop().ok_or_else(|| {
            set_last_error(format!("{fn_name}: epoch produced no observation"));
            SidereonStatus::InvalidArgument
        })?;
        obs_epochs.push(observation);
        ids_per.push(rtk_id_list_from_c(
            fn_name,
            epoch.ambiguity_ids,
            epoch.ambiguity_id_count,
            "ambiguity_ids",
        )?);
        sats_per.push(rtk_ambiguity_satellite_map_from_c(
            fn_name,
            epoch.ambiguity_satellites,
            epoch.ambiguity_satellite_count,
        )?);
        wavelengths_per.push(rtk_f64_map_from_c(
            fn_name,
            epoch.wavelengths_m,
            epoch.wavelength_count,
            "wavelengths_m",
        )?);
        offsets_per.push(rtk_f64_map_from_c(
            fn_name,
            epoch.offsets_m,
            epoch.offset_count,
            "offsets_m",
        )?);
        float_only_per.push(rtk_float_only_systems_from_c(
            fn_name,
            epoch.float_only_systems,
            epoch.float_only_system_count,
        )?);
    }

    let mb_epochs: Vec<MovingBaselineEpoch> = (0..raw_epochs.len())
        .map(|idx| MovingBaselineEpoch {
            base_position_m: bases[idx],
            epoch: &obs_epochs[idx],
            ambiguities: AmbiguitySet {
                ids: &ids_per[idx],
                satellites: &sats_per[idx],
                scale: AmbiguityScale {
                    wavelengths_m: &wavelengths_per[idx],
                    offsets_m: &offsets_per[idx],
                },
                float_only_systems: &float_only_per[idx],
            },
        })
        .collect();

    let model = rtk_model_from_c(fn_name, &config.model)?;
    let receiver_antenna = rtk_receiver_antenna_from_c(fn_name, config.receiver_antenna)?;
    let opts = MovingBaselineOpts {
        model,
        float: rtk_float_options_from_c(&config.float_options),
        fixed: rtk_fixed_options_from_c(&config.fixed_options),
        initial_baseline_m: config.initial_baseline_m,
        warm_start: config.warm_start,
    };

    solve_moving_baseline(&mb_epochs, opts, receiver_antenna.as_ref()).map_err(|err| {
        set_last_error(format!("{fn_name}: {err}"));
        SidereonStatus::Solve
    })
}

fn ppp_auto_init_options_from_c(options: &SidereonPppAutoInitOptions) -> PppAutoInitOptions {
    PppAutoInitOptions {
        initial_guess: options.has_initial_guess.then_some(PppInitialGuess {
            position_m: options.initial_guess_position_m,
            clock_m: options.initial_guess_clock_m,
        }),
        spp_initial_guess: options.spp_initial_guess,
        spp_troposphere: options.spp_troposphere,
        spp_met: SurfaceMet {
            pressure_hpa: options.spp_pressure_hpa,
            temperature_k: options.spp_temperature_k,
            relative_humidity: options.spp_relative_humidity,
        },
    }
}

fn map_rtk_arc_error(
    fn_name: &str,
    err: &sidereon_core::rtk_filter::RtkArcError,
) -> SidereonStatus {
    use sidereon_core::rtk_filter::RtkArcError;
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        RtkArcError::EmptyEpochs
        | RtkArcError::TooFewSatellites { .. }
        | RtkArcError::InvalidEpochTime { .. }
        | RtkArcError::MissingPosition { .. } => SidereonStatus::InvalidArgument,
        _ => SidereonStatus::Solve,
    }
}

unsafe fn rtk_arc_epochs_from_c(
    fn_name: &str,
    epochs: *const SidereonRtkArcEpoch,
    epoch_count: usize,
    read_lli: bool,
) -> Result<Vec<RtkArcEpoch>, SidereonStatus> {
    let raw_epochs = require_slice(epochs, epoch_count, fn_name, "epochs")?;
    validate_element_count::<RtkArcEpoch>(fn_name, "epoch_count", raw_epochs.len())?;
    let mut out = Vec::with_capacity(raw_epochs.len());
    for (idx, epoch) in raw_epochs.iter().enumerate() {
        let base = rtk_arc_observations_from_c(
            fn_name,
            epoch.base,
            epoch.base_count,
            &format!("epochs[{idx}].base"),
            read_lli,
        )?;
        let rover = rtk_arc_observations_from_c(
            fn_name,
            epoch.rover,
            epoch.rover_count,
            &format!("epochs[{idx}].rover"),
            read_lli,
        )?;
        let satellite_positions_m = rtk_arc_positions_from_c(
            fn_name,
            epoch.satellite_positions,
            epoch.satellite_position_count,
            &format!("epochs[{idx}].satellite_positions"),
        )?;
        let base_satellite_positions_m = rtk_arc_positions_from_c(
            fn_name,
            epoch.base_satellite_positions,
            epoch.base_satellite_position_count,
            &format!("epochs[{idx}].base_satellite_positions"),
        )?;
        let rover_satellite_positions_m = rtk_arc_positions_from_c(
            fn_name,
            epoch.rover_satellite_positions,
            epoch.rover_satellite_position_count,
            &format!("epochs[{idx}].rover_satellite_positions"),
        )?;
        out.push(RtkArcEpoch {
            base,
            rover,
            satellite_positions_m,
            base_satellite_positions_m,
            rover_satellite_positions_m,
            velocity_mps: epoch.has_velocity_mps.then_some(epoch.velocity_mps),
            prediction_time_s: epoch.has_prediction_time.then_some(epoch.prediction_time_s),
        });
    }
    Ok(out)
}

fn rtk_static_arc_config_from_c(
    fn_name: &str,
    config: &SidereonRtkStaticArcConfig,
) -> Result<RtkStaticArcConfig, SidereonStatus> {
    let arc = unsafe { rtk_arc_config_from_c(fn_name, &config.arc)? };
    Ok(RtkStaticArcConfig {
        arc,
        opts: ValidatedFixedSolveOpts {
            float: rtk_float_options_from_c(&config.float_options),
            fixed: rtk_fixed_options_from_c(&config.fixed_options),
            residual: rtk_residual_options_from_c(&config.residual_options),
        },
    })
}

fn map_rtk_static_arc_error(fn_name: &str, err: &RtkStaticArcError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        RtkStaticArcError::Arc(RtkArcError::EmptyEpochs)
        | RtkStaticArcError::Arc(RtkArcError::TooFewSatellites { .. })
        | RtkStaticArcError::Arc(RtkArcError::InvalidEpochTime { .. })
        | RtkStaticArcError::Arc(RtkArcError::MissingPosition { .. }) => {
            SidereonStatus::InvalidArgument
        }
        _ => SidereonStatus::Solve,
    }
}

/// Shared implementation of the serial and parallel batch SPP solves. Builds one
/// SolveInputs per V2 input row (its own corrections, robust config, BeiDou
/// Klobuchar, and GLONASS channels) and applies the batch-level `with_geodetic`
/// and `policy` uniformly, matching the core batch signature.
#[allow(clippy::too_many_arguments)]
unsafe fn solve_spp_batch_impl(
    fn_name: &str,
    sp3: *const SidereonSp3,
    inputs: *const SidereonSppInputsV2,
    input_count: usize,
    with_geodetic: bool,
    policy: *const SidereonSppSolvePolicy,
    out_batch: *mut *mut SidereonSppBatch,
    parallel: bool,
) -> SidereonStatus {
    let out_batch = c_try!(require_out(out_batch, fn_name, "out_batch"));
    *out_batch = ptr::null_mut();
    let sp3 = c_try!(require_ref(sp3, fn_name, "sp3"));
    let policy = c_try!(require_ref(policy, fn_name, "policy"));
    let core_policy = c_try!(solve_policy_from_c(fn_name, policy));
    let rows = c_try!(require_slice(inputs, input_count, fn_name, "inputs"));
    let mut epochs = Vec::with_capacity(rows.len());
    for row in rows {
        let glonass = c_try!(glonass_channels_from_c(fn_name, row));
        let solve_inputs = c_try!(build_spp_solve_inputs(
            fn_name,
            &row.base,
            beidou_klobuchar_from_c(row),
            robust_config_from_c(row),
            glonass,
        ));
        epochs.push(solve_inputs);
    }
    let results = if parallel {
        sidereon::solve_spp_batch(&sp3.inner, &epochs, with_geodetic, core_policy)
    } else {
        sidereon::solve_spp_batch_serial(&sp3.inner, &epochs, with_geodetic, core_policy)
    };
    let inner = results
        .into_iter()
        .map(|r| r.map_err(|err| err.to_string()))
        .collect();
    write_boxed_handle(out_batch, SidereonSppBatch { inner });
    SidereonStatus::Ok
}

fn trls_backend_from_c(
    fn_name: &str,
    arg_name: &str,
    backend: u32,
) -> Result<SidereonTrlsBackend, SidereonStatus> {
    match backend {
        value if value == SidereonTrlsBackend::Native as u32 => Ok(SidereonTrlsBackend::Native),
        value if value == SidereonTrlsBackend::HostLapack as u32 => {
            Ok(SidereonTrlsBackend::HostLapack)
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid {arg_name} TRLS backend"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

/// Map a TRLS solver error to a status code. Shape/configuration errors report
/// SIDEREON_STATUS_INVALID_ARGUMENT; runtime numeric failures (non-finite
/// initial residual, SVD backend failure) report SIDEREON_STATUS_SOLVE.
fn map_trls_error(fn_name: &str, err: TrfError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        TrfError::NonFiniteInitialResidual | TrfError::InvalidSvdOutput(_) | TrfError::Svd(_) => {
            SidereonStatus::Solve
        }
        _ => SidereonStatus::InvalidArgument,
    }
}

unsafe fn data_problem_from_c(
    fn_name: &str,
    problem: *const SidereonDataProblem,
) -> Result<DataProblem, SidereonStatus> {
    let problem = require_ref(problem, fn_name, "problem")?;

    let kind = match trls_kind_from_c(fn_name, "kind", problem.kind)? {
        SidereonTrlsKind::Linear => {
            let a = require_slice(problem.a, problem.a_len, fn_name, "a")?;
            let b = require_slice(problem.b, problem.b_len, fn_name, "b")?;
            BuiltinResidual::Linear {
                a: a.to_vec(),
                b: b.to_vec(),
                m: problem.m,
                n: problem.n,
            }
        }
        SidereonTrlsKind::Polynomial => {
            let t = require_slice(problem.t, problem.t_len, fn_name, "t")?;
            let y = require_slice(problem.y, problem.y_len, fn_name, "y")?;
            BuiltinResidual::Polynomial {
                degree: problem.degree,
                t: t.to_vec(),
                y: y.to_vec(),
            }
        }
        SidereonTrlsKind::Exponential => {
            let t = require_slice(problem.t, problem.t_len, fn_name, "t")?;
            let y = require_slice(problem.y, problem.y_len, fn_name, "y")?;
            BuiltinResidual::Exponential {
                t: t.to_vec(),
                y: y.to_vec(),
            }
        }
    };

    let x0 = require_slice(problem.x0, problem.x0_len, fn_name, "x0")?.to_vec();
    let x_scale = match trls_xscale_from_c(fn_name, "x_scale_mode", problem.x_scale_mode)? {
        SidereonTrlsXScale::Unit => XScale::Unit,
        SidereonTrlsXScale::Jac => XScale::Jac,
        SidereonTrlsXScale::Values => {
            let values = require_slice(
                problem.x_scale_values,
                problem.x_scale_values_len,
                fn_name,
                "x_scale_values",
            )?;
            XScale::Values(values.to_vec())
        }
    };
    let max_nfev = if problem.max_nfev < 0 {
        None
    } else {
        Some(problem.max_nfev as usize)
    };

    Ok(DataProblem {
        kind,
        x0,
        loss: trls_loss_from_c(fn_name, "loss", problem.loss)?,
        f_scale: problem.f_scale,
        x_scale,
        max_nfev,
        ftol: problem.ftol,
        xtol: problem.xtol,
        gtol: problem.gtol,
    })
}

pub(crate) fn solve_policy_from_c(
    fn_name: &str,
    policy: &SidereonSppSolvePolicy,
) -> Result<SolvePolicy, SidereonStatus> {
    let validation = validation_options_from_c(policy.use_validation_options, &policy.validation);
    let coarse_search_seeds = if policy.coarse_search_enabled {
        validate_element_count::<[f64; 4]>(
            fn_name,
            "policy.coarse_search_seeds",
            policy.coarse_search_seeds,
        )?;
        Some(policy.coarse_search_seeds)
    } else {
        None
    };
    Ok(SolvePolicy {
        validation,
        coarse_search_seeds,
    })
}

unsafe fn rtk_epochs_from_c(
    fn_name: &str,
    epochs: *const SidereonRtkEpoch,
    epoch_count: usize,
) -> Result<Vec<RtkEpoch>, SidereonStatus> {
    let raw_epochs = require_slice(epochs, epoch_count, fn_name, "epochs")?;
    validate_element_count::<RtkEpoch>(fn_name, "epoch_count", raw_epochs.len())?;
    let mut out = Vec::with_capacity(raw_epochs.len());
    for (epoch_idx, epoch) in raw_epochs.iter().enumerate() {
        let references = require_slice(
            epoch.references,
            epoch.reference_count,
            fn_name,
            &format!("epochs[{epoch_idx}].references"),
        )?;
        let nonref = require_slice(
            epoch.nonref,
            epoch.nonref_count,
            fn_name,
            &format!("epochs[{epoch_idx}].nonref"),
        )?;
        validate_element_count::<SatMeas>(
            fn_name,
            &format!("epochs[{epoch_idx}].reference_count"),
            references.len(),
        )?;
        validate_element_count::<SatMeas>(
            fn_name,
            &format!("epochs[{epoch_idx}].nonref_count"),
            nonref.len(),
        )?;
        let mut core_references = Vec::with_capacity(references.len());
        for row in references {
            core_references.push(rtk_sat_measurement_from_c(fn_name, row)?);
        }
        let mut core_nonref = Vec::with_capacity(nonref.len());
        for row in nonref {
            core_nonref.push(rtk_sat_measurement_from_c(fn_name, row)?);
        }
        out.push(RtkEpoch {
            references: core_references,
            nonref: core_nonref,
            velocity_mps: epoch.has_velocity_mps.then_some(epoch.velocity_mps),
            dt_s: epoch.dt_s,
        });
    }
    Ok(out)
}

unsafe fn rtk_id_list_from_c(
    fn_name: &str,
    values: *const *const c_char,
    count: usize,
    arg_name: &str,
) -> Result<Vec<String>, SidereonStatus> {
    let raw_values = require_slice(values, count, fn_name, arg_name)?;
    validate_element_count::<String>(fn_name, arg_name, raw_values.len())?;
    raw_values
        .iter()
        .copied()
        .enumerate()
        .map(|(idx, value)| {
            parse_bounded_c_string(
                fn_name,
                &format!("{arg_name}[{idx}]"),
                value,
                MAX_RTK_ID_BYTES,
            )
        })
        .collect()
}

unsafe fn rtk_ambiguity_satellite_map_from_c(
    fn_name: &str,
    values: *const SidereonRtkAmbiguitySatellite,
    count: usize,
) -> Result<BTreeMap<String, String>, SidereonStatus> {
    let raw_values = require_slice(values, count, fn_name, "ambiguity_satellites")?;
    validate_element_count::<SidereonRtkAmbiguitySatellite>(
        fn_name,
        "ambiguity_satellite_count",
        raw_values.len(),
    )?;
    let mut out = BTreeMap::new();
    for (idx, value) in raw_values.iter().enumerate() {
        let id = parse_bounded_c_string(
            fn_name,
            &format!("ambiguity_satellites[{idx}].id"),
            value.id,
            MAX_RTK_ID_BYTES,
        )?;
        let sat = parse_satellite_token(fn_name, value.sat_id)?.to_string();
        insert_unique_string_key(fn_name, "ambiguity_satellites", idx, &mut out, id, sat)?;
    }
    Ok(out)
}

fn rtk_float_options_from_c(options: &SidereonRtkFloatOptions) -> FloatSolveOpts {
    FloatSolveOpts {
        position_tol_m: options.position_tol_m,
        ambiguity_tol_m: options.ambiguity_tol_m,
        max_iterations: options.max_iterations,
    }
}

fn rtk_fixed_options_from_c(options: &SidereonRtkFixedOptions) -> FixedSolveOpts {
    FixedSolveOpts {
        position_tol_m: options.position_tol_m,
        ambiguity_tol_m: options.ambiguity_tol_m,
        max_iterations: options.max_iterations,
        ratio_threshold: options.ratio_threshold,
        partial_ambiguity_resolution: options.partial_ambiguity_resolution,
        partial_min_ambiguities: options.partial_min_ambiguities,
    }
}

fn rtk_residual_options_from_c(
    options: &SidereonRtkResidualValidationOptions,
) -> ResidualValidationOpts {
    ResidualValidationOpts {
        threshold_sigma: options
            .threshold_sigma_enabled
            .then_some(options.threshold_sigma),
        max_exclusions: options.max_exclusions,
    }
}

unsafe fn ppp_observation_from_c(
    fn_name: &str,
    row: &SidereonPppObservation,
) -> Result<PppFloatObservation, SidereonStatus> {
    let sat = parse_satellite_token(fn_name, row.sat_id)?;
    let satellite_id = sat.to_string();
    let ambiguity_id =
        parse_bounded_c_string(fn_name, "ambiguity_id", row.ambiguity_id, MAX_PPP_ID_BYTES)?;
    Ok(PppFloatObservation {
        sat,
        satellite_id,
        ambiguity_id,
        code_m: row.code_m,
        phase_m: row.phase_m,
        freq1_hz: row.freq1_hz,
        freq2_hz: row.freq2_hz,
        glonass_channel: None,
    })
}

unsafe fn ppp_f64_map_from_c(
    fn_name: &str,
    values: *const SidereonPppFloatMapEntry,
    count: usize,
    arg_name: &str,
) -> Result<BTreeMap<String, f64>, SidereonStatus> {
    let raw_values = require_slice(values, count, fn_name, arg_name)?;
    validate_element_count::<SidereonPppFloatMapEntry>(fn_name, arg_name, raw_values.len())?;
    let mut out = BTreeMap::new();
    for (idx, value) in raw_values.iter().enumerate() {
        let id = parse_bounded_c_string(
            fn_name,
            &format!("{arg_name}[{idx}].id"),
            value.id,
            MAX_PPP_ID_BYTES,
        )?;
        insert_unique_string_key(fn_name, arg_name, idx, &mut out, id, value.value)?;
    }
    Ok(out)
}

fn ppp_weights_from_c(weights: &SidereonPppMeasurementWeights) -> PppMeasurementWeightsInner {
    PppMeasurementWeightsInner {
        code: weights.code,
        phase: weights.phase,
        elevation_weighting: weights.elevation_weighting,
    }
}

fn ppp_tropo_from_c(
    fn_name: &str,
    tropo: &SidereonPppTroposphereOptions,
) -> Result<PppTroposphereOptionsInner, SidereonStatus> {
    if tropo.enabled {
        // The core validates the surface meteorology and rejects non-physical
        // values; map that rejection to a status code rather than unwrapping.
        let met = Met::new(
            tropo.pressure_hpa,
            tropo.temperature_k,
            tropo.relative_humidity,
        )
        .map_err(|err| {
            set_last_error(format!("{fn_name}: {err}"));
            SidereonStatus::InvalidArgument
        })?;
        let mapping = ppp_tropo_mapping_from_c(fn_name, tropo)?;
        Ok(PppTroposphereOptionsInner {
            enabled: true,
            estimate_ztd: tropo.estimate_ztd,
            estimate_tropo_gradients: tropo.estimate_tropo_gradients,
            met,
            mapping,
        })
    } else {
        Ok(PppTroposphereOptionsInner::disabled())
    }
}

unsafe fn ppp_range_corrections_from_c(
    fn_name: &str,
    corrections: &SidereonPppRangeCorrections,
) -> Result<RangeCorrections, SidereonStatus> {
    reject_unsupported_ppp_correction(
        fn_name,
        corrections.solid_earth_tide,
        "solid_earth_tide",
        "C PPP does not yet accept precomputed tide correction tables",
    )?;
    reject_unsupported_ppp_correction(
        fn_name,
        corrections.phase_windup,
        "phase_windup",
        "C PPP does not yet accept precomputed windup correction tables",
    )?;
    reject_unsupported_ppp_correction(
        fn_name,
        corrections.satellite_antenna,
        "satellite_antenna",
        "C PPP does not yet accept satellite ANTEX correction tables",
    )?;
    Ok(RangeCorrections {
        receiver_antenna: ppp_receiver_antenna_from_c(fn_name, corrections.receiver_antenna)?,
        sat_clock_relativity: corrections.sat_clock_relativity,
        satellite_clock: ppp_satellite_clock_from_c(
            fn_name,
            corrections.satellite_clock_records,
            corrections.satellite_clock_record_count,
        )?,
        ppp: Default::default(),
    })
}

fn ppp_float_options_from_c(options: &SidereonPppFloatOptions) -> PppFloatSolveOptions {
    PppFloatSolveOptions {
        max_iterations: options.max_iterations,
        position_tolerance_m: options.position_tolerance_m,
        clock_tolerance_m: options.clock_tolerance_m,
        ambiguity_tolerance_m: options.ambiguity_tolerance_m,
        ztd_tolerance_m: options.ztd_tolerance_m,
    }
}

unsafe fn rtk_arc_observations_from_c(
    fn_name: &str,
    ptr: *const SidereonRtkArcObservation,
    count: usize,
    arg_name: &str,
    read_lli: bool,
) -> Result<Vec<RtkArcObservation>, SidereonStatus> {
    let raw = require_slice(ptr, count, fn_name, arg_name)?;
    validate_element_count::<RtkArcObservation>(fn_name, arg_name, raw.len())?;
    let mut out = Vec::with_capacity(raw.len());
    for (idx, obs) in raw.iter().enumerate() {
        let satellite_id = parse_satellite_token(fn_name, obs.sat_id)?.to_string();
        let ambiguity_id = parse_bounded_c_string(
            fn_name,
            &format!("{arg_name}[{idx}].ambiguity_id"),
            obs.ambiguity_id,
            MAX_RTK_ID_BYTES,
        )?;
        out.push(RtkArcObservation {
            satellite_id,
            ambiguity_id,
            code_m: obs.code_m,
            phase_m: obs.phase_m,
            lli: (read_lli && obs.has_lli).then_some(obs.lli),
        });
    }
    Ok(out)
}

unsafe fn rtk_arc_config_from_c(
    fn_name: &str,
    config: &SidereonRtkArcConfig,
) -> Result<RtkArcConfig, SidereonStatus> {
    let reference = rtk_arc_reference_from_c(fn_name, config)?;
    let model = rtk_model_from_c(fn_name, &config.model)?;
    let wavelengths_m = rtk_f64_map_from_c(
        fn_name,
        config.wavelengths_m,
        config.wavelength_count,
        "wavelengths_m",
    )?;
    let offsets_m =
        rtk_f64_map_from_c(fn_name, config.offsets_m, config.offset_count, "offsets_m")?;
    let update_opts = rtk_arc_update_opts_from_c(fn_name, &config.update_options)?;
    let preprocessing = rtk_arc_preprocessing_from_c(fn_name, &config.preprocessing)?;
    Ok(RtkArcConfig {
        base_m: config.base_m,
        reference,
        model,
        baseline_prior_sigma_m: config.baseline_prior_sigma_m,
        ambiguity_prior_sigma_m: config.ambiguity_prior_sigma_m,
        initial_baseline_m: config.initial_baseline_m,
        wavelengths_m,
        offsets_m,
        update_opts,
        preprocessing,
    })
}

fn trls_loss_from_c(fn_name: &str, arg_name: &str, loss: u32) -> Result<TrlsLoss, SidereonStatus> {
    match loss {
        value if value == SidereonTrlsLoss::Linear as u32 => Ok(TrlsLoss::Linear),
        value if value == SidereonTrlsLoss::SoftL1 as u32 => Ok(TrlsLoss::SoftL1),
        value if value == SidereonTrlsLoss::Huber as u32 => Ok(TrlsLoss::Huber),
        value if value == SidereonTrlsLoss::Cauchy as u32 => Ok(TrlsLoss::Cauchy),
        value if value == SidereonTrlsLoss::Arctan as u32 => Ok(TrlsLoss::Arctan),
        _ => {
            set_last_error(format!("{fn_name}: invalid {arg_name} TRLS loss"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn trls_xscale_from_c(
    fn_name: &str,
    arg_name: &str,
    mode: u32,
) -> Result<SidereonTrlsXScale, SidereonStatus> {
    match mode {
        value if value == SidereonTrlsXScale::Unit as u32 => Ok(SidereonTrlsXScale::Unit),
        value if value == SidereonTrlsXScale::Jac as u32 => Ok(SidereonTrlsXScale::Jac),
        value if value == SidereonTrlsXScale::Values as u32 => Ok(SidereonTrlsXScale::Values),
        _ => {
            set_last_error(format!("{fn_name}: invalid {arg_name} TRLS x_scale mode"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c_weights() -> SidereonPppMeasurementWeights {
        SidereonPppMeasurementWeights {
            code: 1.0,
            phase: 100.0,
            elevation_weighting: false,
        }
    }

    fn c_tropo(estimate_tropo_gradients: bool) -> SidereonPppTroposphereOptions {
        SidereonPppTroposphereOptions {
            enabled: true,
            estimate_ztd: true,
            estimate_tropo_gradients,
            pressure_hpa: 1013.25,
            temperature_k: 288.15,
            relative_humidity: 0.5,
            mapping: SidereonPppTropoMapping::Niell as u32,
            vmf_sample_count: 0,
            vmf_samples: [SidereonPppVmfSiteSample {
                mjd: 0.0,
                ah: 0.0,
                aw: 0.0,
            }; SIDEREON_PPP_VMF_SITE_MAX_SAMPLES],
        }
    }

    fn c_options() -> SidereonPppFloatOptions {
        SidereonPppFloatOptions {
            max_iterations: 8,
            position_tolerance_m: 1.0e-4,
            clock_tolerance_m: 1.0e-4,
            ambiguity_tolerance_m: 1.0e-4,
            ztd_tolerance_m: 1.0e-4,
        }
    }

    fn c_corrections() -> SidereonPppRangeCorrections {
        SidereonPppRangeCorrections {
            receiver_antenna: ptr::null(),
            sat_clock_relativity: false,
            satellite_clock_records: ptr::null(),
            satellite_clock_record_count: 0,
            solid_earth_tide: false,
            phase_windup: false,
            satellite_antenna: false,
        }
    }

    #[test]
    fn ppp_float_state_marshals_tropo_gradient_initial_state() {
        let clocks = [0.1, 0.2];
        let state = SidereonPppFloatState {
            position_m: [1.0, 2.0, 3.0],
            clocks_m: clocks.as_ptr(),
            clock_count: clocks.len(),
            ambiguities_m: ptr::null(),
            ambiguity_count: 0,
            ztd_m: 0.3,
            tropo_gradient_north_m: 0.04,
            tropo_gradient_east_m: -0.05,
        };

        let core_state = unsafe { ppp_float_state_from_c("test_ppp_state", &state, clocks.len()) }
            .expect("state");

        assert_eq!(core_state.tropo_gradient_north_m, 0.04);
        assert_eq!(core_state.tropo_gradient_east_m, -0.05);
        assert!(core_state.residual_ionosphere_m.is_empty());
    }

    #[test]
    fn ppp_configs_marshal_elevation_cutoff_and_tropo_gradient_toggle() {
        let float_config = SidereonPppFloatConfig {
            epochs: ptr::null(),
            epoch_count: 0,
            initial_state: SidereonPppFloatState {
                position_m: [0.0; 3],
                clocks_m: ptr::null(),
                clock_count: 0,
                ambiguities_m: ptr::null(),
                ambiguity_count: 0,
                ztd_m: 0.0,
                tropo_gradient_north_m: 0.0,
                tropo_gradient_east_m: 0.0,
            },
            weights: c_weights(),
            tropo: c_tropo(true),
            corrections: c_corrections(),
            options: c_options(),
            has_elevation_cutoff_deg: true,
            elevation_cutoff_deg: 12.5,
            residual_screen: true,
        };
        let core_float = unsafe { ppp_float_config_from_c("test_ppp_float_config", &float_config) }
            .expect("float config");
        assert_eq!(core_float.elevation_cutoff_deg, Some(12.5));
        assert!(core_float.tropo.estimate_tropo_gradients);
        assert!(core_float.residual_screen);
        assert!(!core_float.estimate_residual_ionosphere);

        let fixed_config = SidereonPppFixedConfig {
            epochs: ptr::null(),
            epoch_count: 0,
            weights: c_weights(),
            tropo: c_tropo(true),
            corrections: c_corrections(),
            options: c_options(),
            has_elevation_cutoff_deg: true,
            elevation_cutoff_deg: 15.0,
            ambiguity: SidereonPppFixedAmbiguityOptions {
                wavelengths_m: ptr::null(),
                wavelength_count: 0,
                offsets_m: ptr::null(),
                offset_count: 0,
                ratio_threshold: 3.0,
            },
        };
        let core_fixed = unsafe { ppp_fixed_config_from_c("test_ppp_fixed_config", &fixed_config) }
            .expect("fixed config");
        assert_eq!(core_fixed.elevation_cutoff_deg, Some(15.0));
        assert!(core_fixed.tropo.estimate_tropo_gradients);
        assert!(!core_fixed.estimate_residual_ionosphere);
    }
}
