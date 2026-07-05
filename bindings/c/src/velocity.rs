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

/// Copy the unit-variance 4x4 state covariance in row-major order. The state is
/// [vx, vy, vz, clock_drift], where velocity is ECEF meters per second and clock
/// drift is seconds per second.
///
/// Safety: solution must be a live handle from sidereon_solve_velocity or a
/// related velocity solve; out_m2 must point to len writable doubles and len
/// must be at least 16.
#[no_mangle]
pub unsafe extern "C" fn sidereon_velocity_solution_state_covariance(
    solution: *const SidereonVelocitySolution,
    out_m2: *mut f64,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_velocity_solution_state_covariance",
        SidereonStatus::Panic,
        || {
            let solution = c_try!(require_ref(
                solution,
                "sidereon_velocity_solution_state_covariance",
                "solution"
            ));
            let values = flatten_velocity_covariance(solution.inner.state_covariance);
            c_try!(copy_exact_f64s(
                "sidereon_velocity_solution_state_covariance",
                "out_m2",
                out_m2,
                len,
                &values,
            ));
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

fn flatten_velocity_covariance(matrix: [[f64; 4]; 4]) -> [f64; 16] {
    [
        matrix[0][0],
        matrix[0][1],
        matrix[0][2],
        matrix[0][3],
        matrix[1][0],
        matrix[1][1],
        matrix[1][2],
        matrix[1][3],
        matrix[2][0],
        matrix[2][1],
        matrix[2][2],
        matrix[2][3],
        matrix[3][0],
        matrix[3][1],
        matrix[3][2],
        matrix[3][3],
    ]
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    const T_RX_J2000_S: f64 = 646_272_000.0;
    const RECEIVER: [f64; 3] = [4_500_000.0, 500_000.0, 4_500_000.0];
    const V_TRUE: [f64; 3] = [12.0, -7.0, 3.0];
    const DRIFT_TRUE: f64 = 1.0e-9;

    fn fixture_sp3() -> Sp3 {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/sp3/GRG0MGXFIN_20201760000_01D_15M_ORB.SP3");
        let bytes = fs::read(path).expect("read SP3 fixture");
        Sp3::parse(&bytes).expect("parse SP3")
    }

    fn visible_gps(sp3: &Sp3) -> Vec<GnssSatelliteId> {
        let planning = PredictOptions {
            light_time: false,
            ..PredictOptions::default()
        };
        sp3.satellites()
            .iter()
            .copied()
            .filter(|sat| sat.system == GnssSystem::Gps)
            .filter(|sat| {
                observables_predict(sp3, *sat, RECEIVER, T_RX_J2000_S, planning)
                    .map(|obs| obs.elevation_deg >= 5.0)
                    .unwrap_or(false)
            })
            .collect()
    }

    fn synth_range_rate(sp3: &Sp3, sat: GnssSatelliteId) -> f64 {
        let obs = observables_predict(sp3, sat, RECEIVER, T_RX_J2000_S, PredictOptions::default())
            .expect("predict synthetic observation");
        let receiver_projection =
            obs.los_unit[0] * V_TRUE[0] + obs.los_unit[1] * V_TRUE[1] + obs.los_unit[2] * V_TRUE[2];
        obs.range_rate_m_s - receiver_projection + sidereon_core::constants::C_M_S * DRIFT_TRUE
    }

    fn assert_close(got: f64, want: f64, tol: f64) {
        assert!(
            (got - want).abs() <= tol,
            "got {got:e}, want {want:e}, tol {tol:e}"
        );
    }

    #[test]
    fn doppler_velocity_covariance_matches_core_reference() {
        let sp3 = fixture_sp3();
        let sats = visible_gps(&sp3);
        assert!(sats.len() >= 4);
        let core_observations = sats
            .iter()
            .copied()
            .map(|sat| {
                let range_rate = synth_range_rate(&sp3, sat);
                VelocityObservation {
                    satellite_id: sat,
                    value: sidereon_core::velocity::range_rate_to_doppler(
                        range_rate,
                        sidereon_core::constants::F_L1_HZ,
                    )
                    .expect("range-rate to Doppler"),
                    carrier_hz: sidereon_core::constants::F_L1_HZ,
                    sat_clock_drift_s_s: 0.0,
                }
            })
            .collect::<Vec<_>>();
        let expected = sidereon_core::velocity::solve(
            &sp3,
            &core_observations,
            RECEIVER,
            T_RX_J2000_S,
            VelocitySolveOptions {
                observable: VelocityObservable::Doppler,
                ..VelocitySolveOptions::default()
            },
        )
        .expect("core velocity solve");

        let sat_tokens = core_observations
            .iter()
            .map(|obs| CString::new(obs.satellite_id.to_string()).expect("sat token"))
            .collect::<Vec<_>>();
        let c_observations = core_observations
            .iter()
            .zip(&sat_tokens)
            .map(|(obs, token)| SidereonVelocityObservation {
                sat_id: token.as_ptr(),
                value: obs.value,
                carrier_hz: obs.carrier_hz,
                sat_clock_drift_s_s: obs.sat_clock_drift_s_s,
            })
            .collect::<Vec<_>>();
        let sp3_handle = SidereonSp3 { inner: sp3 };
        let options = SidereonVelocityOptions {
            observable: SidereonVelocityObservable::Doppler as u32,
            light_time: true,
            sagnac: true,
        };
        let mut solution = ptr::null_mut();
        let status = unsafe {
            sidereon_solve_velocity(
                &sp3_handle,
                c_observations.as_ptr(),
                c_observations.len(),
                RECEIVER.as_ptr(),
                T_RX_J2000_S,
                &options,
                &mut solution,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert!(!solution.is_null());

        let mut velocity = [0.0; 3];
        let status =
            unsafe { sidereon_velocity_solution_velocity(solution, velocity.as_mut_ptr(), 3) };
        assert_eq!(status, SidereonStatus::Ok);
        for (got, want) in velocity.iter().zip(expected.velocity_m_s) {
            assert_close(*got, want, 1.0e-9);
        }

        let mut drift = 0.0;
        let status = unsafe { sidereon_velocity_solution_clock_drift(solution, &mut drift) };
        assert_eq!(status, SidereonStatus::Ok);
        assert_close(drift, expected.clock_drift_s_s, 1.0e-18);

        let mut covariance = [0.0; 16];
        let status = unsafe {
            sidereon_velocity_solution_state_covariance(solution, covariance.as_mut_ptr(), 16)
        };
        assert_eq!(status, SidereonStatus::Ok);
        let expected_covariance = flatten_velocity_covariance(expected.state_covariance);
        for (got, want) in covariance.iter().zip(expected_covariance) {
            assert_close(*got, want, 1.0e-12);
        }

        unsafe { sidereon_velocity_solution_free(solution) };
    }
}

// ===========================================================================
// Reduced-orbit (compact mean-element) fit / evaluation / drift. Delegates to
// sidereon_core::orbit::{fit_with_model, position, position_velocity, drift}.
