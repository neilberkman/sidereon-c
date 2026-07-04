use super::*;

// === Receiver velocity / clock drift ========================================

/// Observation value convention for a velocity solve.
#[repr(C)]
#[derive(Clone, Copy)]
pub enum SidereonVelocityObservable {
    /// Observation values are pseudorange rates in meters per second.
    RangeRate = 0,
    /// Observation values are Doppler shifts in hertz, converted with each
    /// observation's carrier_hz.
    Doppler = 1,
}

/// One satellite observation for the velocity solve.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonVelocityObservation {
    /// Null-terminated satellite token, for example G08.
    pub sat_id: *const c_char,
    /// Pseudorange rate in m/s, or Doppler in Hz, per the solve's observable.
    pub value: f64,
    /// Carrier frequency in hertz. Used only for Doppler observations.
    pub carrier_hz: f64,
    /// Satellite clock drift in seconds per second.
    pub sat_clock_drift_s_s: f64,
}

/// Options controlling the velocity solve. Initialize with
/// sidereon_velocity_options_init for engine defaults.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonVelocityOptions {
    /// Observation value convention. Stored as a uint32_t; use the
    /// SidereonVelocityObservable discriminants.
    pub observable: u32,
    /// Apply fixed-point light-time correction in the geometry substrate.
    pub light_time: bool,
    /// Apply Earth-rotation Sagnac correction in the geometry substrate.
    pub sagnac: bool,
}

/// A receiver velocity / clock-drift solution. Create with
/// sidereon_solve_velocity and release with sidereon_velocity_solution_free.
pub struct SidereonVelocitySolution {
    pub(crate) inner: VelocitySolution,
}

/// Populate *out_options with the engine's default velocity solve options
/// (range-rate observable, light-time and Sagnac corrections on).
///
/// Safety: out_options must point to a SidereonVelocityOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_velocity_options_init(
    out_options: *mut SidereonVelocityOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_velocity_options_init",
        SidereonStatus::Panic,
        || {
            let out_options = c_try!(require_out(
                out_options,
                "sidereon_velocity_options_init",
                "out_options"
            ));
            let defaults = VelocitySolveOptions::default();
            *out_options = SidereonVelocityOptions {
                observable: match defaults.observable {
                    VelocityObservable::RangeRate => SidereonVelocityObservable::RangeRate,
                    VelocityObservable::Doppler => SidereonVelocityObservable::Doppler,
                } as u32,
                light_time: defaults.light_time,
                sagnac: defaults.sagnac,
            };
            SidereonStatus::Ok
        },
    )
}

/// Solve receiver ECEF velocity and clock drift from one epoch of range-rate or
/// Doppler observations against an SP3 product. On success writes a newly owned
/// handle to *out_solution; release it with sidereon_velocity_solution_free. The
/// numbers are exactly what the engine produces.
///
/// `receiver_ecef_m` is the known receiver ECEF/ITRF position (three doubles).
/// `options` may be NULL for the engine defaults. At least four usable
/// satellites are required.
///
/// Safety: sp3 must be a live handle from sidereon_sp3_load or sidereon_sp3_merge;
/// observations must point to count entries (each with a valid sat_id);
/// receiver_ecef_m must point to three readable doubles; options must be NULL or
/// point to a SidereonVelocityOptions; out_solution must point to storage for a
/// SidereonVelocitySolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solve_velocity(
    sp3: *const SidereonSp3,
    observations: *const SidereonVelocityObservation,
    count: usize,
    receiver_ecef_m: *const f64,
    t_rx_j2000_s: f64,
    options: *const SidereonVelocityOptions,
    out_solution: *mut *mut SidereonVelocitySolution,
) -> SidereonStatus {
    ffi_boundary("sidereon_solve_velocity", SidereonStatus::Panic, || {
        let out_solution = c_try!(require_out(
            out_solution,
            "sidereon_solve_velocity",
            "out_solution"
        ));
        *out_solution = ptr::null_mut();
        let sp3 = c_try!(require_ref(sp3, "sidereon_solve_velocity", "sp3"));
        let raw = c_try!(require_slice(
            observations,
            count,
            "sidereon_solve_velocity",
            "observations"
        ));
        let receiver = c_try!(require_slice(
            receiver_ecef_m,
            3,
            "sidereon_solve_velocity",
            "receiver_ecef_m"
        ));
        let receiver_ecef_m = [receiver[0], receiver[1], receiver[2]];

        let core_options = if options.is_null() {
            VelocitySolveOptions::default()
        } else {
            let options = c_try!(require_ref(options, "sidereon_solve_velocity", "options"));
            VelocitySolveOptions {
                observable: c_try!(velocity_observable_from_c(
                    "sidereon_solve_velocity",
                    "options.observable",
                    options.observable
                )),
                light_time: options.light_time,
                sagnac: options.sagnac,
            }
        };

        let mut parsed = Vec::with_capacity(raw.len());
        for obs in raw {
            let satellite_id = c_try!(parse_satellite_token("sidereon_solve_velocity", obs.sat_id));
            parsed.push(VelocityObservation {
                satellite_id,
                value: obs.value,
                carrier_hz: obs.carrier_hz,
                sat_clock_drift_s_s: obs.sat_clock_drift_s_s,
            });
        }

        let inner = c_try!(guard(SidereonStatus::Solve, || {
            sidereon::solve_velocity(
                &sp3.inner,
                &parsed,
                receiver_ecef_m,
                t_rx_j2000_s,
                core_options,
            )
        }));
        write_boxed_handle(out_solution, SidereonVelocitySolution { inner });
        SidereonStatus::Ok
    })
}

/// Copy the receiver ECEF velocity (meters per second) into out_xyz (three
/// doubles).
///
/// Safety: solution must be a live handle from sidereon_solve_velocity; out_xyz
/// must point to len writable doubles (len must be at least 3).
#[no_mangle]
pub unsafe extern "C" fn sidereon_velocity_solution_velocity(
    solution: *const SidereonVelocitySolution,
    out_xyz: *mut f64,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_velocity_solution_velocity",
        SidereonStatus::Panic,
        || {
            c_try!(require_out(
                out_xyz,
                "sidereon_velocity_solution_velocity",
                "out_xyz"
            ));
            zero_f64_prefix(out_xyz, len, 3);
            let solution = c_try!(require_ref(
                solution,
                "sidereon_velocity_solution_velocity",
                "solution"
            ));
            c_try!(copy_exact_f64s(
                "sidereon_velocity_solution_velocity",
                "out_xyz",
                out_xyz,
                len,
                &solution.inner.velocity_m_s
            ));
            SidereonStatus::Ok
        },
    )
}

/// Write the receiver speed (meters per second) to *out_speed.
///
/// Safety: solution must be a live handle from sidereon_solve_velocity; out_speed
/// must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_velocity_solution_speed(
    solution: *const SidereonVelocitySolution,
    out_speed: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_velocity_solution_speed",
        SidereonStatus::Panic,
        || {
            let out_speed = c_try!(require_out(
                out_speed,
                "sidereon_velocity_solution_speed",
                "out_speed"
            ));
            *out_speed = 0.0;
            let solution = c_try!(require_ref(
                solution,
                "sidereon_velocity_solution_speed",
                "solution"
            ));
            *out_speed = solution.inner.speed_m_s;
            SidereonStatus::Ok
        },
    )
}

/// Write the receiver clock drift (seconds per second) to *out_drift.
///
/// Safety: solution must be a live handle from sidereon_solve_velocity; out_drift
/// must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_velocity_solution_clock_drift(
    solution: *const SidereonVelocitySolution,
    out_drift: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_velocity_solution_clock_drift",
        SidereonStatus::Panic,
        || {
            let out_drift = c_try!(require_out(
                out_drift,
                "sidereon_velocity_solution_clock_drift",
                "out_drift"
            ));
            *out_drift = 0.0;
            let solution = c_try!(require_ref(
                solution,
                "sidereon_velocity_solution_clock_drift",
                "solution"
            ));
            *out_drift = solution.inner.clock_drift_s_s;
            SidereonStatus::Ok
        },
    )
}

/// Write the number of satellites that contributed rows to *out_count.
///
/// Safety: solution must be a live handle from sidereon_solve_velocity; out_count
/// must point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_velocity_solution_used_sat_count(
    solution: *const SidereonVelocitySolution,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_velocity_solution_used_sat_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_velocity_solution_used_sat_count",
                "out_count"
            ));
            *out_count = 0;
            let solution = c_try!(require_ref(
                solution,
                "sidereon_velocity_solution_used_sat_count",
                "solution"
            ));
            *out_count = solution.inner.used_sats.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy the used-satellite tokens, in solve order. Uses the variable-length
/// output contract documented at the top of the header.
///
/// Safety: solution must be a live handle from sidereon_solve_velocity; out (when
/// non-NULL) must point to len writable tokens; out_written and out_required must
/// point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_velocity_solution_used_sat_ids(
    solution: *const SidereonVelocitySolution,
    out: *mut SidereonSatelliteToken,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_velocity_solution_used_sat_ids",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_velocity_solution_used_sat_ids",
                out_written,
                out_required
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_velocity_solution_used_sat_ids",
                "solution"
            ));
            let values: Vec<SidereonSatelliteToken> = solution
                .inner
                .used_sats
                .iter()
                .copied()
                .map(satellite_token)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_velocity_solution_used_sat_ids",
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

/// Copy the post-fit range-rate residuals (meters per second), in the same order
/// as sidereon_velocity_solution_used_sat_ids. Uses the variable-length output
/// contract documented at the top of the header.
///
/// Safety: solution must be a live handle from sidereon_solve_velocity; out (when
/// non-NULL) must point to len writable doubles; out_written and out_required
/// must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_velocity_solution_residuals(
    solution: *const SidereonVelocitySolution,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_velocity_solution_residuals",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_velocity_solution_residuals",
                out_written,
                out_required
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_velocity_solution_residuals",
                "solution"
            ));
            let values: Vec<f64> = solution
                .inner
                .residuals_m_s
                .iter()
                .map(|(_, residual)| *residual)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_velocity_solution_residuals",
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

/// Release a velocity solution handle from sidereon_solve_velocity. Passing NULL
/// is a no-op.
///
/// Safety: solution must be NULL or a live handle from sidereon_solve_velocity
/// that has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_velocity_solution_free(solution: *mut SidereonVelocitySolution) {
    ffi_boundary("sidereon_velocity_solution_free", (), || {
        free_boxed(solution);
    });
}

/// Solve receiver ECEF velocity and clock drift from one epoch of range-rate or
/// Doppler observations against a broadcast source. Mirror of
/// sidereon_solve_velocity for the broadcast (navigation message) source; it
/// produces the same SidereonVelocitySolution handle, so the
/// sidereon_velocity_solution_* accessors and sidereon_velocity_solution_free
/// apply unchanged. Delegates to sidereon::solve_velocity.
///
/// Safety: broadcast must be a live handle from
/// sidereon_broadcast_ephemeris_parse_nav; observations must point to count
/// entries (each with a valid sat_id); receiver_ecef_m must point to three
/// readable doubles; options must be NULL or point to a SidereonVelocityOptions;
/// out_solution must point to storage for a SidereonVelocitySolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solve_velocity_broadcast(
    broadcast: *const SidereonBroadcastEphemeris,
    observations: *const SidereonVelocityObservation,
    count: usize,
    receiver_ecef_m: *const f64,
    t_rx_j2000_s: f64,
    options: *const SidereonVelocityOptions,
    out_solution: *mut *mut SidereonVelocitySolution,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_solve_velocity_broadcast",
        SidereonStatus::Panic,
        || {
            let out_solution = c_try!(require_out(
                out_solution,
                "sidereon_solve_velocity_broadcast",
                "out_solution"
            ));
            *out_solution = ptr::null_mut();
            let broadcast = c_try!(require_ref(
                broadcast,
                "sidereon_solve_velocity_broadcast",
                "broadcast"
            ));
            let raw = c_try!(require_slice(
                observations,
                count,
                "sidereon_solve_velocity_broadcast",
                "observations"
            ));
            let receiver = c_try!(require_slice(
                receiver_ecef_m,
                3,
                "sidereon_solve_velocity_broadcast",
                "receiver_ecef_m"
            ));
            let receiver_ecef_m = [receiver[0], receiver[1], receiver[2]];

            let core_options = if options.is_null() {
                VelocitySolveOptions::default()
            } else {
                let options = c_try!(require_ref(
                    options,
                    "sidereon_solve_velocity_broadcast",
                    "options"
                ));
                VelocitySolveOptions {
                    observable: c_try!(velocity_observable_from_c(
                        "sidereon_solve_velocity_broadcast",
                        "options.observable",
                        options.observable
                    )),
                    light_time: options.light_time,
                    sagnac: options.sagnac,
                }
            };

            let mut parsed = Vec::with_capacity(raw.len());
            for obs in raw {
                let satellite_id = c_try!(parse_satellite_token(
                    "sidereon_solve_velocity_broadcast",
                    obs.sat_id
                ));
                parsed.push(VelocityObservation {
                    satellite_id,
                    value: obs.value,
                    carrier_hz: obs.carrier_hz,
                    sat_clock_drift_s_s: obs.sat_clock_drift_s_s,
                });
            }

            let inner = c_try!(guard(SidereonStatus::Solve, || {
                sidereon::solve_velocity(
                    &broadcast.inner,
                    &parsed,
                    receiver_ecef_m,
                    t_rx_j2000_s,
                    core_options,
                )
            }));
            write_boxed_handle(out_solution, SidereonVelocitySolution { inner });
            SidereonStatus::Ok
        },
    )
}

// ===========================================================================
// Reduced-orbit (compact mean-element) fit / evaluation / drift. Delegates to
// sidereon_core::orbit::{fit_with_model, position, position_velocity, drift}.
