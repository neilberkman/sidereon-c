use super::*;
use crate::engine_error::{
    degrade_reason_name, engine_error_operation_boundary, record_engine_error,
    SidereonEngineErrorFamily,
};
use serde_json::{json, Value};

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
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SidereonCovarianceMatrix6 {
    pub values: [[f64; 6]; 6],
}

/// Results of validating a six-by-six covariance matrix.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonCovariance6Validation {
    /// Whether the validated matrix is symmetric.
    pub symmetric: bool,
    /// Whether the validated matrix is positive semidefinite.
    pub positive_semidefinite: bool,
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

/// Build a validated row-major six-by-six covariance from six diagonal
/// variances. Delegates to `sidereon_core::astro::covariance::Covariance6::from_diagonal`.
///
/// Safety: diagonal points to exactly `diagonal_len` readable doubles and must
/// contain six entries; out points to a writable SidereonCovarianceMatrix6.
#[no_mangle]
pub unsafe extern "C" fn sidereon_covariance6_from_diagonal(
    diagonal: *const f64,
    diagonal_len: usize,
    out: *mut SidereonCovarianceMatrix6,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_covariance6_from_diagonal",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_covariance6_from_diagonal",
                "out"
            ));
            *out = SidereonCovarianceMatrix6 {
                values: [[0.0; 6]; 6],
            };
            let diagonal = c_try!(require_slice(
                diagonal,
                diagonal_len,
                "sidereon_covariance6_from_diagonal",
                "diagonal"
            ));
            if diagonal_len != 6 {
                set_last_error(format!(
                    "sidereon_covariance6_from_diagonal: diagonal_len must be 6, got {diagonal_len}"
                ));
                return SidereonStatus::InvalidArgument;
            }
            let mut values = [0.0; 6];
            values.copy_from_slice(diagonal);
            match Covariance6::from_diagonal(values) {
                Ok(covariance) => {
                    *out = covariance_matrix_to_c(&covariance);
                    SidereonStatus::Ok
                }
                Err(err) => {
                    set_last_error(format!("sidereon_covariance6_from_diagonal: {err:?}"));
                    SidereonStatus::InvalidArgument
                }
            }
        },
    )
}

/// Validate a row-major six-by-six covariance matrix. The output is written
/// only after `Covariance6::try_from_matrix` accepts the matrix, so an invalid
/// matrix returns the engine error through the normal invalid-argument status.
/// Delegates to `sidereon_core::astro::covariance::Covariance6::try_from_matrix`.
///
/// Safety: covariance points to a SidereonCovarianceMatrix6 and out points to a
/// writable SidereonCovariance6Validation.
#[no_mangle]
pub unsafe extern "C" fn sidereon_covariance6_validate(
    covariance: *const SidereonCovarianceMatrix6,
    out: *mut SidereonCovariance6Validation,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_covariance6_validate",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_covariance6_validate", "out"));
            *out = SidereonCovariance6Validation {
                symmetric: false,
                positive_semidefinite: false,
            };
            let covariance = c_try!(require_ref(
                covariance,
                "sidereon_covariance6_validate",
                "covariance"
            ));
            match Covariance6::try_from_matrix(covariance.values) {
                Ok(covariance) => {
                    *out = SidereonCovariance6Validation {
                        symmetric: covariance.is_symmetric(),
                        positive_semidefinite: covariance.is_positive_semidefinite(),
                    };
                    SidereonStatus::Ok
                }
                Err(err) => {
                    set_last_error(format!("sidereon_covariance6_validate: {err:?}"));
                    SidereonStatus::InvalidArgument
                }
            }
        },
    )
}

/// Convert a validated six-by-six covariance from kilometres to metres.
/// Delegates to `sidereon_core::astro::covariance::covariance6_km_to_m`.
///
/// Safety: covariance points to a SidereonCovarianceMatrix6 and out points to a
/// writable SidereonCovarianceMatrix6.
#[no_mangle]
pub unsafe extern "C" fn sidereon_covariance6_km_to_m(
    covariance: *const SidereonCovarianceMatrix6,
    out: *mut SidereonCovarianceMatrix6,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_covariance6_km_to_m",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_covariance6_km_to_m", "out"));
            *out = SidereonCovarianceMatrix6 {
                values: [[0.0; 6]; 6],
            };
            let covariance = c_try!(require_ref(
                covariance,
                "sidereon_covariance6_km_to_m",
                "covariance"
            ));
            let covariance = c_try!(covariance_matrix_from_c(
                "sidereon_covariance6_km_to_m",
                covariance
            ));
            match sidereon_core::astro::covariance::covariance6_km_to_m(&covariance) {
                Ok(value) => {
                    *out = covariance_matrix_to_c(&value);
                    SidereonStatus::Ok
                }
                Err(err) => {
                    set_last_error(format!("sidereon_covariance6_km_to_m: {err:?}"));
                    SidereonStatus::InvalidArgument
                }
            }
        },
    )
}

/// Convert a validated six-by-six covariance from metres to kilometres.
/// Delegates to `sidereon_core::astro::covariance::covariance6_m_to_km`.
///
/// Safety: covariance points to a SidereonCovarianceMatrix6 and out points to a
/// writable SidereonCovarianceMatrix6.
#[no_mangle]
pub unsafe extern "C" fn sidereon_covariance6_m_to_km(
    covariance: *const SidereonCovarianceMatrix6,
    out: *mut SidereonCovarianceMatrix6,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_covariance6_m_to_km",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_covariance6_m_to_km", "out"));
            *out = SidereonCovarianceMatrix6 {
                values: [[0.0; 6]; 6],
            };
            let covariance = c_try!(require_ref(
                covariance,
                "sidereon_covariance6_m_to_km",
                "covariance"
            ));
            let covariance = c_try!(covariance_matrix_from_c(
                "sidereon_covariance6_m_to_km",
                covariance
            ));
            match sidereon_core::astro::covariance::covariance6_m_to_km(&covariance) {
                Ok(value) => {
                    *out = covariance_matrix_to_c(&value);
                    SidereonStatus::Ok
                }
                Err(err) => {
                    set_last_error(format!("sidereon_covariance6_m_to_km: {err:?}"));
                    SidereonStatus::InvalidArgument
                }
            }
        },
    )
}

/// Interpolate two same-frame six-by-six covariances with the core PSD-safe
/// interpolation policy. Delegates to
/// `sidereon_core::astro::covariance::interpolate_covariance_psd`.
///
/// Safety: a, b, and out point to SidereonCovarianceMatrix6 values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_covariance6_interpolate_psd(
    a: *const SidereonCovarianceMatrix6,
    b: *const SidereonCovarianceMatrix6,
    u: f64,
    out: *mut SidereonCovarianceMatrix6,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_covariance6_interpolate_psd",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_covariance6_interpolate_psd",
                "out"
            ));
            *out = SidereonCovarianceMatrix6 {
                values: [[0.0; 6]; 6],
            };
            let a = c_try!(require_ref(a, "sidereon_covariance6_interpolate_psd", "a"));
            let b = c_try!(require_ref(b, "sidereon_covariance6_interpolate_psd", "b"));
            let a = c_try!(covariance_matrix_from_c(
                "sidereon_covariance6_interpolate_psd",
                a
            ));
            let b = c_try!(covariance_matrix_from_c(
                "sidereon_covariance6_interpolate_psd",
                b
            ));
            match sidereon_core::astro::covariance::interpolate_covariance_psd(&a, &b, u) {
                Ok(value) => {
                    *out = covariance_matrix_to_c(&value);
                    SidereonStatus::Ok
                }
                Err(err) => {
                    set_last_error(format!("sidereon_covariance6_interpolate_psd: {err:?}"));
                    SidereonStatus::InvalidArgument
                }
            }
        },
    )
}

/// Transform a six-by-six ECI covariance to RTN axes at an ECI Cartesian
/// state. Delegates to `eci_to_rtn_covariance6` in sidereon-core.
///
/// Safety: covariance and state point to valid input structs; out points to a
/// writable SidereonCovarianceMatrix6.
#[no_mangle]
pub unsafe extern "C" fn sidereon_covariance6_eci_to_rtn(
    covariance: *const SidereonCovarianceMatrix6,
    state: *const SidereonCartesianState,
    out: *mut SidereonCovarianceMatrix6,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_covariance6_eci_to_rtn",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_covariance6_eci_to_rtn", "out"));
            *out = SidereonCovarianceMatrix6 {
                values: [[0.0; 6]; 6],
            };
            let covariance = c_try!(require_ref(
                covariance,
                "sidereon_covariance6_eci_to_rtn",
                "covariance"
            ));
            let state = c_try!(require_ref(
                state,
                "sidereon_covariance6_eci_to_rtn",
                "state"
            ));
            let covariance = c_try!(covariance_matrix_from_c(
                "sidereon_covariance6_eci_to_rtn",
                covariance
            ));
            match sidereon_core::astro::covariance::eci_to_rtn_covariance6(
                &covariance,
                &cartesian_state_from_c(state),
            ) {
                Ok(value) => {
                    *out = covariance_matrix_to_c(&value);
                    SidereonStatus::Ok
                }
                Err(err) => {
                    set_last_error(format!("sidereon_covariance6_eci_to_rtn: {err:?}"));
                    SidereonStatus::InvalidArgument
                }
            }
        },
    )
}

/// Transform a six-by-six RTN covariance to ECI axes at an ECI Cartesian
/// state. Delegates to `rtn_to_eci_covariance6` in sidereon-core.
///
/// Safety: covariance and state point to valid input structs; out points to a
/// writable SidereonCovarianceMatrix6.
#[no_mangle]
pub unsafe extern "C" fn sidereon_covariance6_rtn_to_eci(
    covariance: *const SidereonCovarianceMatrix6,
    state: *const SidereonCartesianState,
    out: *mut SidereonCovarianceMatrix6,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_covariance6_rtn_to_eci",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_covariance6_rtn_to_eci", "out"));
            *out = SidereonCovarianceMatrix6 {
                values: [[0.0; 6]; 6],
            };
            let covariance = c_try!(require_ref(
                covariance,
                "sidereon_covariance6_rtn_to_eci",
                "covariance"
            ));
            let state = c_try!(require_ref(
                state,
                "sidereon_covariance6_rtn_to_eci",
                "state"
            ));
            let covariance = c_try!(covariance_matrix_from_c(
                "sidereon_covariance6_rtn_to_eci",
                covariance
            ));
            match sidereon_core::astro::covariance::rtn_to_eci_covariance6(
                &covariance,
                &cartesian_state_from_c(state),
            ) {
                Ok(value) => {
                    *out = covariance_matrix_to_c(&value);
                    SidereonStatus::Ok
                }
                Err(err) => {
                    set_last_error(format!("sidereon_covariance6_rtn_to_eci: {err:?}"));
                    SidereonStatus::InvalidArgument
                }
            }
        },
    )
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
    const FN: &str = "sidereon_covariance_transport";
    engine_error_operation_boundary(FN, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN, out_written, out_required));
        let covariance0 = c_try!(require_ref(covariance0, FN, "covariance0"));
        let covariance0 = c_try!(covariance_matrix_from_c(FN, covariance0));
        let raw_segments = c_try!(require_slice(segments, segment_count, FN, "segments"));
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
        let process_noise = c_try!(process_noise_from_c(FN, process_noise));
        let covariances = match sidereon_core::astro::propagator::transport_covariance(
            covariance0,
            &segments,
            process_noise,
        ) {
            Ok(values) => values,
            Err(err) => {
                return map_propagation_error(FN, err);
            }
        };
        let values: Vec<_> = covariances.iter().map(covariance_matrix_to_c).collect();
        c_try!(copy_prefix_to_c(
            FN,
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
    const FN: &str = "sidereon_covariance_ephemeris_covariance_at";
    engine_error_operation_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_covariance, FN, "out_covariance"));
        *out = SidereonCovarianceMatrix6 {
            values: [[0.0; 6]; 6],
        };
        let ephemeris = c_try!(require_ref(ephemeris, FN, "ephemeris"));
        match ephemeris.inner.covariance_at(epoch_s) {
            Ok(value) => {
                *out = covariance_matrix_to_c(&value);
                SidereonStatus::Ok
            }
            Err(err) => map_propagation_error(FN, err),
        }
    })
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
    const FN: &str = "sidereon_propagate_covariance";
    engine_error_operation_boundary(FN, SidereonStatus::Panic, || {
        let out_ephemeris = c_try!(require_out(out_ephemeris, FN, "out_ephemeris"));
        *out_ephemeris = ptr::null_mut();
        propagate_covariance_with_tide_system_impl(
            FN,
            config,
            CoreTideSystem::TideFree,
            covariance0,
            epochs_s,
            epoch_count,
            options,
            out_ephemeris,
        )
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_propagate_covariance_with_tide_system(
    config: *const SidereonStatePropagationConfig,
    gravity_tide_system: u32,
    covariance0: *const SidereonCovarianceMatrix6,
    epochs_s: *const f64,
    epoch_count: usize,
    options: SidereonCovariancePropagationOptions,
    out_ephemeris: *mut *mut SidereonCovarianceEphemeris,
) -> SidereonStatus {
    const FN: &str = "sidereon_propagate_covariance_with_tide_system";
    engine_error_operation_boundary(FN, SidereonStatus::Panic, || {
        let out_ephemeris = c_try!(require_out(out_ephemeris, FN, "out_ephemeris"));
        *out_ephemeris = ptr::null_mut();
        let tide_system = c_try!(gravity_tide_system_from_c(FN, gravity_tide_system));
        propagate_covariance_with_tide_system_impl(
            FN,
            config,
            tide_system,
            covariance0,
            epochs_s,
            epoch_count,
            options,
            out_ephemeris,
        )
    })
}

#[allow(clippy::too_many_arguments)]
unsafe fn propagate_covariance_with_tide_system_impl(
    fn_name: &str,
    config: *const SidereonStatePropagationConfig,
    tide_system: CoreTideSystem,
    covariance0: *const SidereonCovarianceMatrix6,
    epochs_s: *const f64,
    epoch_count: usize,
    options: SidereonCovariancePropagationOptions,
    out_ephemeris: *mut *mut SidereonCovarianceEphemeris,
) -> SidereonStatus {
    let config = c_try!(require_ref(config, fn_name, "config"));
    let covariance0 = c_try!(require_ref(covariance0, fn_name, "covariance0"));
    let covariance0 = c_try!(covariance_matrix_from_c(fn_name, covariance0));
    let epochs = c_try!(times_from_c(fn_name, epochs_s, epoch_count));
    let propagator = c_try!(state_propagator_from_c_with_tide_system(
        fn_name,
        config,
        tide_system
    ));
    let input_frame = c_try!(covariance_frame_from_c(fn_name, options.input_frame));
    let output_frame = c_try!(covariance_frame_from_c(fn_name, options.output_frame));
    let process_noise = c_try!(process_noise_from_c(fn_name, options.process_noise));
    let mut prop_opts = sidereon_core::astro::propagator::CovariancePropagationOptions::default();
    prop_opts.process_noise = process_noise;
    prop_opts.output_frame = output_frame;
    let result = match propagator.propagate_covariance(
        sidereon_core::astro::propagator::LabeledCovariance6 {
            covariance: covariance0,
            frame: input_frame,
        },
        epochs,
        &prop_opts,
    ) {
        Ok(result) => result,
        Err(err) => return map_propagation_error(fn_name, err),
    };
    write_boxed_handle(out_ephemeris, SidereonCovarianceEphemeris { inner: result });
    SidereonStatus::Ok
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

pub fn propagation_node(kind: &str, fields: Value) -> Value {
    json!({
        "kind": kind,
        "fields": fields,
    })
}

pub(crate) fn covariance_propagation_error_value(error: &PropagationError) -> Value {
    use PropagationError as E;
    match error {
        E::InvalidInput(message) => propagation_node(
            "invalid_input",
            json!({
                "message": message,
                "reason": message,
            }),
        ),
        E::NumericalFailure(message) => propagation_node(
            "numerical_failure",
            json!({
                "message": message,
                "reason": message,
            }),
        ),
        E::MaxStepsExceeded => propagation_node("max_steps_exceeded", json!({})),
        E::EventFailure(message) => propagation_node(
            "event_failure",
            json!({
                "message": message,
                "reason": message,
            }),
        ),
        E::ForceModelFailure(message) => propagation_node(
            "force_model_failure",
            json!({
                "message": message,
                "reason": message,
            }),
        ),
        E::Ut1OutsideCoverage(reason) => propagation_node(
            "ut1_outside_coverage",
            json!({
                "reason": degrade_reason_name(*reason),
            }),
        ),
    }
}

pub fn map_propagation_error(fn_name: &str, err: PropagationError) -> SidereonStatus {
    record_engine_error(
        SidereonEngineErrorFamily::Propagation,
        fn_name,
        covariance_propagation_error_value(&err),
    );
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        PropagationError::InvalidInput(_) => SidereonStatus::InvalidArgument,
        PropagationError::Ut1OutsideCoverage(_) => SidereonStatus::Ut1OutsideCoverage,
        PropagationError::NumericalFailure(_)
        | PropagationError::MaxStepsExceeded
        | PropagationError::EventFailure(_)
        | PropagationError::ForceModelFailure(_) => SidereonStatus::Solve,
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

#[cfg(test)]
mod tests {
    use super::*;
    use sidereon_core::astro::covariance as core_covariance;

    fn diagonal(values: [f64; 6]) -> SidereonCovarianceMatrix6 {
        let mut matrix = SidereonCovarianceMatrix6 {
            values: [[0.0; 6]; 6],
        };
        for (idx, value) in values.into_iter().enumerate() {
            matrix.values[idx][idx] = value;
        }
        matrix
    }

    fn core(matrix: &SidereonCovarianceMatrix6) -> Covariance6 {
        Covariance6::try_from_matrix(matrix.values).expect("core accepts the matrix")
    }

    fn assert_same_bits(got: &SidereonCovarianceMatrix6, expected: &Covariance6) {
        let expected = covariance_matrix_to_c(expected);
        for row in 0..6 {
            for column in 0..6 {
                assert_eq!(
                    got.values[row][column].to_bits(),
                    expected.values[row][column].to_bits(),
                    "[{row}][{column}]"
                );
            }
        }
    }

    #[test]
    fn covariance6_fixed_routes_cover_validation_units_and_interpolation() {
        let diagonal_values = [1.0, 4.0, 9.0, 16.0, 25.0, 36.0];
        let mut covariance = SidereonCovarianceMatrix6 {
            values: [[0.0; 6]; 6],
        };
        assert_eq!(
            unsafe {
                sidereon_covariance6_from_diagonal(
                    diagonal_values.as_ptr(),
                    diagonal_values.len(),
                    &mut covariance,
                )
            },
            SidereonStatus::Ok
        );
        assert_same_bits(
            &covariance,
            &Covariance6::from_diagonal(diagonal_values).expect("core diagonal"),
        );

        let mut validation = SidereonCovariance6Validation {
            symmetric: false,
            positive_semidefinite: false,
        };
        assert_eq!(
            unsafe { sidereon_covariance6_validate(&covariance, &mut validation) },
            SidereonStatus::Ok
        );
        assert_eq!(validation.symmetric, core(&covariance).is_symmetric());
        assert_eq!(
            validation.positive_semidefinite,
            core(&covariance).is_positive_semidefinite()
        );

        // sidereon-core refuses a negative variance on both routes.
        let invalid_diagonal = [-1.0, 4.0, 9.0, 16.0, 25.0, 36.0];
        assert!(Covariance6::from_diagonal(invalid_diagonal).is_err());
        assert!(Covariance6::try_from_matrix(diagonal(invalid_diagonal).values).is_err());
        assert_eq!(
            unsafe {
                sidereon_covariance6_from_diagonal(
                    invalid_diagonal.as_ptr(),
                    invalid_diagonal.len(),
                    &mut covariance,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert_eq!(
            unsafe { sidereon_covariance6_validate(&diagonal(invalid_diagonal), &mut validation) },
            SidereonStatus::InvalidArgument
        );

        let a = diagonal(diagonal_values);
        let b = diagonal([4.0, 9.0, 16.0, 25.0, 36.0, 49.0]);
        for u in [0.0, 1.0, 0.5] {
            let mut interpolated = SidereonCovarianceMatrix6 {
                values: [[0.0; 6]; 6],
            };
            assert_eq!(
                unsafe { sidereon_covariance6_interpolate_psd(&a, &b, u, &mut interpolated) },
                SidereonStatus::Ok
            );
            assert_same_bits(
                &interpolated,
                &core_covariance::interpolate_covariance_psd(&core(&a), &core(&b), u)
                    .expect("core interpolation"),
            );
        }
    }

    #[test]
    fn covariance6_fixed_routes_scale_exactly_and_round_trip_rtn() {
        let covariance = diagonal([1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let mut metres = SidereonCovarianceMatrix6 {
            values: [[0.0; 6]; 6],
        };
        assert_eq!(
            unsafe { sidereon_covariance6_km_to_m(&covariance, &mut metres) },
            SidereonStatus::Ok
        );
        assert_same_bits(
            &metres,
            &core_covariance::covariance6_km_to_m(&core(&covariance)).expect("core km to m"),
        );

        let mut kilometres = SidereonCovarianceMatrix6 {
            values: [[0.0; 6]; 6],
        };
        assert_eq!(
            unsafe { sidereon_covariance6_m_to_km(&metres, &mut kilometres) },
            SidereonStatus::Ok
        );
        assert_same_bits(
            &kilometres,
            &core_covariance::covariance6_m_to_km(&core(&metres)).expect("core m to km"),
        );
        // The unit scaling round trips exactly.
        assert_eq!(kilometres, covariance);

        let state = SidereonCartesianState {
            epoch_s: 123.0,
            position_km: [7000.0, 1000.0, 2000.0],
            velocity_km_s: [-1.0, 7.2, 2.0],
        };
        let mut rtn = SidereonCovarianceMatrix6 {
            values: [[0.0; 6]; 6],
        };
        assert_eq!(
            unsafe { sidereon_covariance6_eci_to_rtn(&covariance, &state, &mut rtn) },
            SidereonStatus::Ok
        );
        assert_same_bits(
            &rtn,
            &core_covariance::eci_to_rtn_covariance6(
                &core(&covariance),
                &cartesian_state_from_c(&state),
            )
            .expect("core ECI to RTN"),
        );
        let mut round_trip = SidereonCovarianceMatrix6 {
            values: [[0.0; 6]; 6],
        };
        assert_eq!(
            unsafe { sidereon_covariance6_rtn_to_eci(&rtn, &state, &mut round_trip) },
            SidereonStatus::Ok
        );
        assert_same_bits(
            &round_trip,
            &core_covariance::rtn_to_eci_covariance6(&core(&rtn), &cartesian_state_from_c(&state))
                .expect("core RTN to ECI"),
        );
        for row in 0..6 {
            for column in 0..6 {
                assert!(
                    (round_trip.values[row][column] - covariance.values[row][column]).abs()
                        < 1.0e-12
                );
            }
        }

        // sidereon-core refuses a state at the origin, which has no RTN axes.
        let invalid_state = SidereonCartesianState {
            position_km: [0.0; 3],
            ..state
        };
        assert!(core_covariance::eci_to_rtn_covariance6(
            &core(&covariance),
            &cartesian_state_from_c(&invalid_state)
        )
        .is_err());
        assert_eq!(
            unsafe { sidereon_covariance6_eci_to_rtn(&covariance, &invalid_state, &mut rtn) },
            SidereonStatus::InvalidArgument
        );
    }

    #[test]
    fn table_driven_propagation_error_mapping() {
        use crate::engine_error::{
            clear_engine_error, sidereon_last_engine_error_info, SidereonEngineErrorInfo,
        };
        use sidereon_core::astro::time::DegradeReason;

        clear_engine_error();

        let cases: Vec<(PropagationError, &'static str, SidereonStatus)> = vec![
            (
                PropagationError::InvalidInput("step size must be positive".to_string()),
                "invalid_input",
                SidereonStatus::InvalidArgument,
            ),
            (
                PropagationError::NumericalFailure("integrator step diverged".to_string()),
                "numerical_failure",
                SidereonStatus::Solve,
            ),
            (
                PropagationError::MaxStepsExceeded,
                "max_steps_exceeded",
                SidereonStatus::Solve,
            ),
            (
                PropagationError::EventFailure("altitude root crossing missed".to_string()),
                "event_failure",
                SidereonStatus::Solve,
            ),
            (
                PropagationError::ForceModelFailure(
                    "nrlmsise-00 density out of domain".to_string(),
                ),
                "force_model_failure",
                SidereonStatus::Solve,
            ),
            (
                PropagationError::Ut1OutsideCoverage(DegradeReason::AfterCoverage),
                "ut1_outside_coverage",
                SidereonStatus::Ut1OutsideCoverage,
            ),
            (
                PropagationError::Ut1OutsideCoverage(DegradeReason::BeforeCoverage),
                "ut1_outside_coverage",
                SidereonStatus::Ut1OutsideCoverage,
            ),
        ];

        for (err, expected_kind, expected_status) in cases {
            let val = covariance_propagation_error_value(&err);
            assert_eq!(val["kind"], json!(expected_kind));
            match &err {
                PropagationError::InvalidInput(msg)
                | PropagationError::NumericalFailure(msg)
                | PropagationError::EventFailure(msg)
                | PropagationError::ForceModelFailure(msg) => {
                    assert_eq!(val["fields"]["message"], json!(msg));
                    assert_eq!(val["fields"]["reason"], json!(msg));
                }
                PropagationError::MaxStepsExceeded => {
                    assert_eq!(val["fields"], json!({}));
                }
                PropagationError::Ut1OutsideCoverage(reason) => {
                    assert_eq!(
                        val["fields"]["reason"],
                        json!(match reason {
                            DegradeReason::BeforeCoverage => "before_coverage",
                            DegradeReason::AfterCoverage => "after_coverage",
                        })
                    );
                }
            }

            let status = map_propagation_error("test_op", err);
            assert_eq!(status, expected_status);

            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                unsafe { sidereon_last_engine_error_info(&mut info) },
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Propagation);
            assert!(info.payload_len > 0);
            clear_engine_error();
        }
    }

    fn assert_retained_error(expected_family: SidereonEngineErrorFamily, expected_payload: &[u8]) {
        let mut info = SidereonEngineErrorInfo {
            family: SidereonEngineErrorFamily::None,
            payload_len: 0,
        };
        assert_eq!(
            unsafe { sidereon_last_engine_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.family, expected_family);
        assert_eq!(info.payload_len, expected_payload.len());
        let mut buf = vec![0u8; expected_payload.len()];
        let mut written = 0usize;
        let mut required = 0usize;
        assert_eq!(
            unsafe {
                sidereon_last_engine_error_payload(
                    buf.as_mut_ptr(),
                    buf.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(written, expected_payload.len());
        assert_eq!(required, expected_payload.len());
        assert_eq!(buf, expected_payload);
    }

    #[test]
    fn covariance_propagation_real_public_refusal_and_valid_control() {
        use crate::engine_error::{
            clear_engine_error, sidereon_last_engine_error_info,
            sidereon_last_engine_error_payload, SidereonEngineErrorInfo,
        };
        use std::ptr;

        clear_engine_error();

        let cov0 = diagonal([1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let stm = diagonal([1.0; 6]);
        let q_state = SidereonCartesianState {
            epoch_s: 0.0,
            position_km: [7000.0, 0.0, 0.0],
            velocity_km_s: [0.0, 7.5, 0.0],
        };
        let segment = SidereonCovarianceTransportSegment {
            stm,
            dt_seconds: 1.0,
            q_rotation_state: q_state,
        };
        let process_noise = SidereonProcessNoise {
            kind: SidereonProcessNoiseKind::None as u32,
            q_radial_km2_s3: 0.0,
            q_transverse_km2_s3: 0.0,
            q_normal_km2_s3: 0.0,
        };

        // 1. Valid control: sidereon_covariance_transport
        let mut out_covs = [SidereonCovarianceMatrix6 {
            values: [[0.0; 6]; 6],
        }; 2];
        let mut written = 0usize;
        let mut required = 0usize;
        let status = unsafe {
            sidereon_covariance_transport(
                &cov0,
                &segment,
                1,
                process_noise,
                out_covs.as_mut_ptr(),
                out_covs.len(),
                &mut written,
                &mut required,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(written, 2);
        assert_eq!(required, 2);

        let mut info = SidereonEngineErrorInfo {
            family: SidereonEngineErrorFamily::Propagation,
            payload_len: 999,
        };
        assert_eq!(
            unsafe { sidereon_last_engine_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(info.payload_len, 0);

        // 2. Real bounded public refusal: sidereon_propagate_covariance with direction reversal
        let prop_config = SidereonStatePropagationConfig {
            epoch_s: 0.0,
            position_km: [7000.0, 0.0, 0.0],
            velocity_km_s: [0.0, 7.5, 0.0],
            force_model: SidereonPropagationForceModel::TwoBody as u32,
            integrator: SidereonPropagationIntegrator::Dp54 as u32,
            abs_tol: 1e-8,
            rel_tol: 1e-8,
            initial_step_s: 60.0,
            min_step_s: 0.1,
            max_step_s: 120.0,
            max_steps: 1000,
            mu_km3_s2_enabled: false,
            mu_km3_s2: MU_EARTH,
            has_drag: false,
            drag: drag_parameters_to_c(
                DragParameters::from_bc_factor_m2_kg(
                    0.01,
                    SpaceWeather::default(),
                    DragForce::DEFAULT_REENTRY_ALTITUDE_KM,
                )
                .unwrap(),
            ),
            force_components: SidereonForceModelComponents {
                has_two_body: true,
                two_body_mu_km3_s2_enabled: false,
                two_body_mu_km3_s2: MU_EARTH,
                has_zonal: false,
                zonal_max_degree: 2,
                has_spherical_harmonic: false,
                spherical_harmonic_max_degree: 2,
                spherical_harmonic_max_order: 2,
                has_solid_earth_tide: false,
                has_solid_earth_pole_tide: false,
                has_third_body: false,
                third_body_sun: false,
                third_body_moon: false,
                has_solar_radiation_pressure: false,
                solar_radiation_pressure: SidereonSolarRadiationPressure {
                    cr: 1.0,
                    area_to_mass_m2_kg: 0.01,
                },
                has_relativity: false,
            },
        };
        let bad_epochs = [100.0, 50.0];
        let prop_options = SidereonCovariancePropagationOptions {
            input_frame: SidereonCovarianceFrame::Inertial as u32,
            output_frame: SidereonCovarianceFrame::Inertial as u32,
            process_noise,
        };
        let mut ephem: *mut SidereonCovarianceEphemeris = ptr::null_mut();
        let status = unsafe {
            sidereon_propagate_covariance(
                &prop_config,
                &cov0,
                bad_epochs.as_ptr(),
                bad_epochs.len(),
                prop_options,
                &mut ephem,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);
        assert!(ephem.is_null());

        assert_eq!(
            unsafe { sidereon_last_engine_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.family, SidereonEngineErrorFamily::Propagation);
        assert!(info.payload_len > 0);
        let expected_len = info.payload_len;

        // Two-pass payload retrieval: Pass 1 query length
        let mut payload_written = 999usize;
        let mut payload_required = 0usize;
        assert_eq!(
            unsafe {
                sidereon_last_engine_error_payload(
                    ptr::null_mut(),
                    0,
                    &mut payload_written,
                    &mut payload_required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(payload_written, 0);
        assert_eq!(payload_required, expected_len);

        // Short-buffer check: InvalidArgument, 0 written, required set, no copy
        let mut short_buf = vec![0u8; expected_len - 1];
        assert_eq!(
            unsafe {
                sidereon_last_engine_error_payload(
                    short_buf.as_mut_ptr(),
                    short_buf.len(),
                    &mut payload_written,
                    &mut payload_required,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert_eq!(payload_written, 0);
        assert_eq!(payload_required, expected_len);

        // Pass 2: full exact buffer retrieval
        let mut payload_buf = vec![0u8; expected_len];
        assert_eq!(
            unsafe {
                sidereon_last_engine_error_payload(
                    payload_buf.as_mut_ptr(),
                    payload_buf.len(),
                    &mut payload_written,
                    &mut payload_required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(payload_written, expected_len);
        assert_eq!(payload_required, expected_len);

        let parsed: serde_json::Value =
            serde_json::from_slice(&payload_buf).expect("valid JSON payload");
        assert_eq!(parsed["schema_version"], 1);
        assert_eq!(parsed["family"], "propagation");
        assert_eq!(parsed["operation"], "sidereon_propagate_covariance");
        assert_eq!(parsed["error"]["kind"], "invalid_input");
        assert_eq!(
            parsed["error"]["fields"]["message"],
            "epochs_tdb_seconds direction reversal"
        );
        assert_eq!(
            parsed["error"]["fields"]["reason"],
            "epochs_tdb_seconds direction reversal"
        );

        // 3. Handle readers retain the error
        let good_epochs = [60.0];
        let status = unsafe {
            sidereon_propagate_covariance(
                &prop_config,
                &cov0,
                good_epochs.as_ptr(),
                good_epochs.len(),
                prop_options,
                &mut ephem,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert!(!ephem.is_null());
        assert_eq!(
            unsafe { sidereon_last_engine_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(info.payload_len, 0);

        // Seed refusal before early-null reset
        let mut dummy_ephem = ptr::null_mut();
        let status = unsafe {
            sidereon_propagate_covariance(
                &prop_config,
                &cov0,
                bad_epochs.as_ptr(),
                bad_epochs.len(),
                prop_options,
                &mut dummy_ephem,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);
        assert!(dummy_ephem.is_null());
        assert_eq!(
            unsafe { sidereon_last_engine_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.family, SidereonEngineErrorFamily::Propagation);
        assert!(info.payload_len > 0);

        // Early-null reset
        let status = unsafe {
            sidereon_propagate_covariance(
                &prop_config,
                &cov0,
                bad_epochs.as_ptr(),
                bad_epochs.len(),
                prop_options,
                ptr::null_mut(),
            )
        };
        assert_eq!(status, SidereonStatus::NullPointer);
        // Early-null clears the slot
        assert_eq!(
            unsafe { sidereon_last_engine_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(info.payload_len, 0);

        // Put an error in the slot
        let _ = unsafe {
            sidereon_propagate_covariance(
                &prop_config,
                &cov0,
                bad_epochs.as_ptr(),
                bad_epochs.len(),
                prop_options,
                &mut dummy_ephem,
            )
        };
        assert_eq!(
            unsafe { sidereon_last_engine_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.family, SidereonEngineErrorFamily::Propagation);

        // Calling ephemeris readers retains the error
        let mut count = 0usize;
        assert_eq!(
            unsafe { sidereon_covariance_ephemeris_count(ephem, &mut count) },
            SidereonStatus::Ok
        );
        assert_eq!(count, 1);
        assert_retained_error(SidereonEngineErrorFamily::Propagation, &payload_buf);

        let mut node = SidereonCovarianceNode {
            state: q_state,
            covariance: cov0,
            frame: 0,
        };
        let mut node_written = 0usize;
        let mut node_required = 0usize;
        assert_eq!(
            unsafe {
                sidereon_covariance_ephemeris_nodes(
                    ephem,
                    &mut node,
                    1,
                    &mut node_written,
                    &mut node_required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(node_written, 1);
        assert_retained_error(SidereonEngineErrorFamily::Propagation, &payload_buf);

        // Freeing ephemeris retains the error
        unsafe { sidereon_covariance_ephemeris_free(ephem) };
        assert_retained_error(SidereonEngineErrorFamily::Propagation, &payload_buf);

        clear_engine_error();
    }
}
