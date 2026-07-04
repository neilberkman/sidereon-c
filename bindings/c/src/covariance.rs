use super::*;

/// Write whether a 3x3 covariance matrix is symmetric. Delegates to
/// sidereon_core::astro::covariance::symmetric.
///
/// Safety: covariance points to 9 doubles; out points to a bool.
#[no_mangle]
pub unsafe extern "C" fn sidereon_covariance_is_symmetric(
    covariance: *const f64,
    out: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_covariance_is_symmetric",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_covariance_is_symmetric", "out"));
            *out = false;
            let covariance = c_try!(read_mat3(
                "sidereon_covariance_is_symmetric",
                "covariance",
                covariance
            ));
            *out = sidereon_core::astro::covariance::symmetric(&covariance);
            SidereonStatus::Ok
        },
    )
}

/// Write whether a 3x3 covariance matrix is positive semidefinite. Delegates to
/// sidereon_core::astro::covariance::positive_semidefinite.
///
/// Safety: covariance points to 9 doubles; out points to a bool.
#[no_mangle]
pub unsafe extern "C" fn sidereon_covariance_is_positive_semidefinite(
    covariance: *const f64,
    out: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_covariance_is_positive_semidefinite",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_covariance_is_positive_semidefinite",
                "out"
            ));
            *out = false;
            let covariance = c_try!(read_mat3(
                "sidereon_covariance_is_positive_semidefinite",
                "covariance",
                covariance
            ));
            *out = sidereon_core::astro::covariance::positive_semidefinite(&covariance);
            SidereonStatus::Ok
        },
    )
}

/// Fitted parameter covariance from a converged fit: `(J^T J)^-1` scaled by the
/// post-fit reduced chi-square `s_sq = 2 * cost / (m - n)`, the same quantity
/// `scipy.optimize.curve_fit` reports as `pcov`. Forms the covariance straight
/// from the row-major `m`-by-`n` design (Jacobian) matrix and the final cost via
/// the core `covariance_from_jacobian`, with the redundancy taken from the
/// Jacobian's own shape; requires positive redundancy `m > n`. Writes the
/// `n`-by-`n` covariance row-major into out. Same variable-length output contract
/// as sidereon_normal_covariance.
///
/// Safety: jacobian must point to m*n readable doubles (or be NULL when m*n is
/// 0); out must point to at least len writable doubles or be NULL when len is 0;
/// out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_covariance_from_jacobian(
    jacobian: *const f64,
    m: usize,
    n: usize,
    cost: f64,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_covariance_from_jacobian",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_covariance_from_jacobian",
                out_written,
                out_required
            ));
            let count = c_try!(m.checked_mul(n).ok_or_else(|| {
                set_last_error("sidereon_covariance_from_jacobian: m*n overflows".to_string());
                SidereonStatus::InvalidArgument
            }));
            let data = c_try!(require_slice(
                jacobian,
                count,
                "sidereon_covariance_from_jacobian",
                "jacobian"
            ));
            let jac = DMatrix::from_row_slice(m, n, data);
            let cov = match core_covariance_from_jacobian(&jac, cost) {
                Ok(cov) => cov,
                Err(err) => return map_lsq_error("sidereon_covariance_from_jacobian", err),
            };
            let row_major: Vec<f64> = cov
                .row_iter()
                .flat_map(|r| r.iter().copied().collect::<Vec<_>>())
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_covariance_from_jacobian",
                "out",
                &row_major,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

// === Round-2 covariance propagation and transport ===========================

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonCovarianceMatrix6 {
    pub values: [[f64; 6]; 6],
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonCovarianceFrame {
    Inertial = 0,
    Rtn = 1,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonProcessNoiseKind {
    None = 0,
    RtnAccelerationPsd = 1,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonProcessNoise {
    pub kind: u32,
    pub q_radial_km2_s3: f64,
    pub q_transverse_km2_s3: f64,
    pub q_normal_km2_s3: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonCovariancePropagationOptions {
    pub input_frame: u32,
    pub output_frame: u32,
    pub process_noise: SidereonProcessNoise,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonCovarianceTransportSegment {
    pub stm: SidereonCovarianceMatrix6,
    pub dt_seconds: f64,
    pub q_rotation_state: SidereonCartesianState,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonCovarianceNode {
    pub state: SidereonCartesianState,
    pub covariance: SidereonCovarianceMatrix6,
    pub frame: u32,
}

pub struct SidereonCovarianceEphemeris {
    pub(crate) inner: sidereon_core::astro::propagator::CovarianceEphemeris,
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_covariance_transport(
    covariance0: *const SidereonCovarianceMatrix6,
    segments: *const SidereonCovarianceTransportSegment,
    segment_count: usize,
    process_noise: SidereonProcessNoise,
    out: *mut SidereonCovarianceMatrix6,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_covariance_transport",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_covariance_transport",
                out_written,
                out_required
            ));
            let covariance0 = c_try!(require_ref(
                covariance0,
                "sidereon_covariance_transport",
                "covariance0"
            ));
            let covariance0 = c_try!(covariance_matrix_from_c(
                "sidereon_covariance_transport",
                covariance0
            ));
            let raw_segments = c_try!(require_slice(
                segments,
                segment_count,
                "sidereon_covariance_transport",
                "segments"
            ));
            let segments: Vec<_> = raw_segments
                .iter()
                .map(
                    |segment| sidereon_core::astro::propagator::CovarianceSegment {
                        stm: segment.stm.values,
                        dt_seconds: segment.dt_seconds,
                        q_rotation_state: cartesian_state_from_c(&segment.q_rotation_state),
                    },
                )
                .collect();
            let process_noise = c_try!(process_noise_from_c(
                "sidereon_covariance_transport",
                process_noise
            ));
            let covariances = match sidereon_core::astro::propagator::transport_covariance(
                covariance0,
                &segments,
                process_noise,
            ) {
                Ok(values) => values,
                Err(err) => {
                    return map_covariance_propagation_error("sidereon_covariance_transport", err)
                }
            };
            let values: Vec<_> = covariances.iter().map(covariance_matrix_to_c).collect();
            c_try!(copy_prefix_to_c(
                "sidereon_covariance_transport",
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

#[no_mangle]
pub unsafe extern "C" fn sidereon_covariance_ephemeris_count(
    ephemeris: *const SidereonCovarianceEphemeris,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_covariance_ephemeris_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_covariance_ephemeris_count",
                "out_count"
            ));
            *out_count = 0;
            let ephemeris = c_try!(require_ref(
                ephemeris,
                "sidereon_covariance_ephemeris_count",
                "ephemeris"
            ));
            *out_count = ephemeris.inner.len();
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_covariance_ephemeris_nodes(
    ephemeris: *const SidereonCovarianceEphemeris,
    out: *mut SidereonCovarianceNode,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_covariance_ephemeris_nodes",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_covariance_ephemeris_nodes",
                out_written,
                out_required
            ));
            let ephemeris = c_try!(require_ref(
                ephemeris,
                "sidereon_covariance_ephemeris_nodes",
                "ephemeris"
            ));
            let nodes: Vec<_> = ephemeris
                .inner
                .nodes()
                .iter()
                .map(covariance_node_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_covariance_ephemeris_nodes",
                "out",
                &nodes,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_covariance_ephemeris_covariance_at(
    ephemeris: *const SidereonCovarianceEphemeris,
    epoch_s: f64,
    out_covariance: *mut SidereonCovarianceMatrix6,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_covariance_ephemeris_covariance_at",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_covariance,
                "sidereon_covariance_ephemeris_covariance_at",
                "out_covariance"
            ));
            *out = SidereonCovarianceMatrix6 {
                values: [[0.0; 6]; 6],
            };
            let ephemeris = c_try!(require_ref(
                ephemeris,
                "sidereon_covariance_ephemeris_covariance_at",
                "ephemeris"
            ));
            match ephemeris.inner.covariance_at(epoch_s) {
                Ok(value) => {
                    *out = covariance_matrix_to_c(&value);
                    SidereonStatus::Ok
                }
                Err(err) => map_covariance_propagation_error(
                    "sidereon_covariance_ephemeris_covariance_at",
                    err,
                ),
            }
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_covariance_ephemeris_free(
    ephemeris: *mut SidereonCovarianceEphemeris,
) {
    free_boxed(ephemeris);
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_propagate_covariance(
    config: *const SidereonStatePropagationConfig,
    covariance0: *const SidereonCovarianceMatrix6,
    epochs_s: *const f64,
    epoch_count: usize,
    options: SidereonCovariancePropagationOptions,
    out_ephemeris: *mut *mut SidereonCovarianceEphemeris,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_propagate_covariance",
        SidereonStatus::Panic,
        || {
            let out_ephemeris = c_try!(require_out(
                out_ephemeris,
                "sidereon_propagate_covariance",
                "out_ephemeris"
            ));
            *out_ephemeris = ptr::null_mut();
            let config = c_try!(require_ref(
                config,
                "sidereon_propagate_covariance",
                "config"
            ));
            let covariance0 = c_try!(require_ref(
                covariance0,
                "sidereon_propagate_covariance",
                "covariance0"
            ));
            let covariance0 = c_try!(covariance_matrix_from_c(
                "sidereon_propagate_covariance",
                covariance0
            ));
            let epochs = c_try!(times_from_c(
                "sidereon_propagate_covariance",
                epochs_s,
                epoch_count
            ));
            let propagator = c_try!(state_propagator_from_c(
                "sidereon_propagate_covariance",
                config
            ));
            let input_frame = c_try!(covariance_frame_from_c(
                "sidereon_propagate_covariance",
                options.input_frame
            ));
            let output_frame = c_try!(covariance_frame_from_c(
                "sidereon_propagate_covariance",
                options.output_frame
            ));
            let process_noise = c_try!(process_noise_from_c(
                "sidereon_propagate_covariance",
                options.process_noise
            ));
            let result = match propagator.propagate_covariance(
                sidereon_core::astro::propagator::LabeledCovariance6 {
                    covariance: covariance0,
                    frame: input_frame,
                },
                epochs,
                &sidereon_core::astro::propagator::CovariancePropagationOptions {
                    process_noise,
                    output_frame,
                },
            ) {
                Ok(result) => result,
                Err(err) => {
                    return map_covariance_propagation_error("sidereon_propagate_covariance", err)
                }
            };
            write_boxed_handle(out_ephemeris, SidereonCovarianceEphemeris { inner: result });
            SidereonStatus::Ok
        },
    )
}

fn covariance_matrix_from_c(
    fn_name: &str,
    value: &SidereonCovarianceMatrix6,
) -> Result<Covariance6, SidereonStatus> {
    Covariance6::try_from_matrix(value.values).map_err(|err| {
        set_last_error(format!("{fn_name}: covariance is invalid: {err:?}"));
        SidereonStatus::InvalidArgument
    })
}

fn covariance_frame_from_c(
    fn_name: &str,
    value: u32,
) -> Result<sidereon_core::astro::propagator::CovarianceFrame, SidereonStatus> {
    match value {
        x if x == SidereonCovarianceFrame::Inertial as u32 => {
            Ok(sidereon_core::astro::propagator::CovarianceFrame::Inertial)
        }
        x if x == SidereonCovarianceFrame::Rtn as u32 => {
            Ok(sidereon_core::astro::propagator::CovarianceFrame::Rtn)
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid covariance frame {value}"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn process_noise_from_c(
    fn_name: &str,
    value: SidereonProcessNoise,
) -> Result<sidereon_core::astro::propagator::ProcessNoise, SidereonStatus> {
    match value.kind {
        x if x == SidereonProcessNoiseKind::None as u32 => {
            Ok(sidereon_core::astro::propagator::ProcessNoise::None)
        }
        x if x == SidereonProcessNoiseKind::RtnAccelerationPsd as u32 => Ok(
            sidereon_core::astro::propagator::ProcessNoise::RtnAccelerationPsd {
                q_radial_km2_s3: value.q_radial_km2_s3,
                q_transverse_km2_s3: value.q_transverse_km2_s3,
                q_normal_km2_s3: value.q_normal_km2_s3,
            },
        ),
        _ => {
            set_last_error(format!(
                "{fn_name}: invalid process noise kind {}",
                value.kind
            ));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn map_covariance_propagation_error(fn_name: &str, err: PropagationError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        PropagationError::InvalidInput(_) => SidereonStatus::InvalidArgument,
        _ => SidereonStatus::Solve,
    }
}

fn covariance_node_to_c(
    node: &sidereon_core::astro::propagator::CovarianceNode,
) -> SidereonCovarianceNode {
    let frame = match node.frame {
        sidereon_core::astro::propagator::CovarianceFrame::Inertial => {
            SidereonCovarianceFrame::Inertial as u32
        }
        sidereon_core::astro::propagator::CovarianceFrame::Rtn => {
            SidereonCovarianceFrame::Rtn as u32
        }
    };
    SidereonCovarianceNode {
        state: cartesian_state_to_c(&node.state),
        covariance: covariance_matrix_to_c(&node.covariance),
        frame,
    }
}

fn covariance_matrix_to_c(value: &Covariance6) -> SidereonCovarianceMatrix6 {
    SidereonCovarianceMatrix6 {
        values: *value.as_matrix(),
    }
}
