use super::*;
use crate::engine_error::{
    engine_error_operation_boundary, record_engine_error, SidereonEngineErrorFamily,
};

fn error_node(kind: &str, fields: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "kind": kind,
        "fields": fields,
    })
}

pub(crate) fn conjunction_error_value(
    error: &sidereon_core::astro::conjunction::ConjunctionError,
) -> serde_json::Value {
    use sidereon_core::astro::conjunction::ConjunctionError as E;
    match error {
        E::NonFinite { field } => error_node("non_finite", serde_json::json!({ "field": field })),
        E::NotPositive { field } => {
            error_node("not_positive", serde_json::json!({ "field": field }))
        }
        E::UndefinedFrame => error_node("undefined_frame", serde_json::json!({})),
    }
}

pub(crate) fn event_finder_error_value(
    error: &sidereon_core::astro::events::EventFinderError,
) -> serde_json::Value {
    use sidereon_core::astro::events::EventFinderError as E;
    match error {
        E::InvalidInput { field, reason } => error_node(
            "invalid_input",
            serde_json::json!({
                "field": field,
                "reason": reason,
            }),
        ),
        E::Ut1OutsideCoverage(reason) => error_node(
            "ut1_outside_coverage",
            serde_json::json!({
                "reason": crate::engine_error::degrade_reason_name(*reason),
            }),
        ),
    }
}

pub(crate) fn tca_error_value(error: &core_tca::TcaError) -> serde_json::Value {
    use core_tca::TcaError as E;
    match error {
        E::InvalidInput { field, reason } => error_node(
            "invalid_input",
            serde_json::json!({
                "field": field,
                "reason": reason,
            }),
        ),
        E::Init { object, source } => error_node(
            "init",
            serde_json::json!({
                "object": match object {
                    core_tca::TcaObject::Primary => "primary",
                    core_tca::TcaObject::Secondary => "secondary",
                },
                "cause": crate::tle::sgp4_error_value(source),
            }),
        ),
        E::Propagate { object, source } => error_node(
            "propagate",
            serde_json::json!({
                "object": match object {
                    core_tca::TcaObject::Primary => "primary",
                    core_tca::TcaObject::Secondary => "secondary",
                },
                "cause": crate::tle::sgp4_error_value(source),
            }),
        ),
        E::CovariancePropagation { object, reason } => error_node(
            "covariance_propagation",
            serde_json::json!({
                "object": match object {
                    core_tca::TcaObject::Primary => "primary",
                    core_tca::TcaObject::Secondary => "secondary",
                },
                "reason": reason,
            }),
        ),
        E::EventFinder(finder) => error_node(
            "event_finder",
            serde_json::json!({
                "cause": event_finder_error_value(finder),
            }),
        ),
        E::Conjunction(conj) => error_node(
            "conjunction",
            serde_json::json!({
                "cause": conjunction_error_value(conj),
            }),
        ),
    }
}

fn map_conjunction_error(
    fn_name: &str,
    err: sidereon_core::astro::conjunction::ConjunctionError,
) -> SidereonStatus {
    record_engine_error(
        SidereonEngineErrorFamily::Conjunction,
        fn_name,
        conjunction_error_value(&err),
    );
    extra_invalid_arg(fn_name, err)
}

// --- Conjunction / collision probability (sidereon_core::astro::conjunction) --

/// Collision-probability computation method.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonPcMethod {
    /// Foster equal-area (2D) Pc.
    FosterEqualArea = 0,
    /// Foster numerical (2D) Pc.
    FosterNumerical = 1,
    /// Alfano 2005 Pc.
    Alfano2005 = 2,
}

/// One object's state for a conjunction, mirroring
/// sidereon_core::astro::conjunction::ConjunctionState.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonConjunctionState {
    /// ECI position, km.
    pub position_km: [f64; 3],
    /// ECI velocity, km/s.
    pub velocity_km_s: [f64; 3],
    /// 3x3 position covariance, km^2, row-major.
    pub covariance_km2: [[f64; 3]; 3],
}

/// Encounter (B-plane) frame, mirroring
/// sidereon_core::astro::conjunction::EncounterFrame.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonEncounterFrame {
    /// Frame x-axis unit vector.
    pub x_hat: [f64; 3],
    /// Frame y-axis unit vector.
    pub y_hat: [f64; 3],
    /// Frame z-axis unit vector.
    pub z_hat: [f64; 3],
    /// Relative position, km.
    pub relative_position_km: [f64; 3],
    /// Relative velocity, km/s.
    pub relative_velocity_km_s: [f64; 3],
    /// Miss distance, km.
    pub miss_km: f64,
    /// Relative speed, km/s.
    pub relative_speed_km_s: f64,
}

/// Collision probability result, mirroring
/// sidereon_core::astro::conjunction::CollisionPc.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonCollisionPc {
    /// Probability of collision.
    pub pc: f64,
    /// Miss distance, km.
    pub miss_km: f64,
    /// Relative speed, km/s.
    pub relative_speed_km_s: f64,
    /// Encounter-plane sigma along x, km.
    pub sigma_x_km: f64,
    /// Encounter-plane sigma along z, km.
    pub sigma_z_km: f64,
}

/// Build the encounter (B-plane) frame from two objects' position/velocity.
/// Delegates to sidereon_core::astro::conjunction::encounter_frame.
///
/// Safety: each pointer must point to 3 doubles; out to a SidereonEncounterFrame.
#[no_mangle]
pub unsafe extern "C" fn sidereon_encounter_frame(
    r1_km: *const f64,
    v1_km_s: *const f64,
    r2_km: *const f64,
    v2_km_s: *const f64,
    out: *mut SidereonEncounterFrame,
) -> SidereonStatus {
    engine_error_operation_boundary("sidereon_encounter_frame", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_encounter_frame", "out"));
        *out = SidereonEncounterFrame {
            x_hat: [0.0; 3],
            y_hat: [0.0; 3],
            z_hat: [0.0; 3],
            relative_position_km: [0.0; 3],
            relative_velocity_km_s: [0.0; 3],
            miss_km: 0.0,
            relative_speed_km_s: 0.0,
        };
        let r1 = c_try!(read_vec3("sidereon_encounter_frame", "r1_km", r1_km));
        let v1 = c_try!(read_vec3("sidereon_encounter_frame", "v1_km_s", v1_km_s));
        let r2 = c_try!(read_vec3("sidereon_encounter_frame", "r2_km", r2_km));
        let v2 = c_try!(read_vec3("sidereon_encounter_frame", "v2_km_s", v2_km_s));
        match sidereon_core::astro::conjunction::encounter_frame(r1, v1, r2, v2) {
            Ok(f) => {
                *out = SidereonEncounterFrame {
                    x_hat: f.x_hat,
                    y_hat: f.y_hat,
                    z_hat: f.z_hat,
                    relative_position_km: f.relative_position_km,
                    relative_velocity_km_s: f.relative_velocity_km_s,
                    miss_km: f.miss_km,
                    relative_speed_km_s: f.relative_speed_km_s,
                };
                SidereonStatus::Ok
            }
            Err(err) => map_conjunction_error("sidereon_encounter_frame", err),
        }
    })
}

/// Two-object collision probability for a hard-body radius (km) and method.
/// Delegates to sidereon_core::astro::conjunction::collision_probability.
///
/// Safety: object1 and object2 point to SidereonConjunctionState; out to a
/// SidereonCollisionPc.
#[no_mangle]
pub unsafe extern "C" fn sidereon_collision_probability(
    object1: *const SidereonConjunctionState,
    object2: *const SidereonConjunctionState,
    hard_body_radius_km: f64,
    method: u32,
    out: *mut SidereonCollisionPc,
) -> SidereonStatus {
    engine_error_operation_boundary(
        "sidereon_collision_probability",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_collision_probability", "out"));
            *out = SidereonCollisionPc {
                pc: 0.0,
                miss_km: 0.0,
                relative_speed_km_s: 0.0,
                sigma_x_km: 0.0,
                sigma_z_km: 0.0,
            };
            let o1 = c_try!(require_ref(
                object1,
                "sidereon_collision_probability",
                "object1"
            ));
            let o2 = c_try!(require_ref(
                object2,
                "sidereon_collision_probability",
                "object2"
            ));
            let method = match method {
                v if v == SidereonPcMethod::FosterEqualArea as u32 => {
                    sidereon_core::astro::conjunction::PcMethod::FosterEqualArea
                }
                v if v == SidereonPcMethod::FosterNumerical as u32 => {
                    sidereon_core::astro::conjunction::PcMethod::FosterNumerical
                }
                v if v == SidereonPcMethod::Alfano2005 as u32 => {
                    sidereon_core::astro::conjunction::PcMethod::Alfano2005
                }
                _ => {
                    set_last_error("sidereon_collision_probability: invalid method".to_string());
                    return SidereonStatus::InvalidArgument;
                }
            };
            let s1 = sidereon_core::astro::conjunction::ConjunctionState {
                position_km: o1.position_km,
                velocity_km_s: o1.velocity_km_s,
                covariance_km2: o1.covariance_km2,
            };
            let s2 = sidereon_core::astro::conjunction::ConjunctionState {
                position_km: o2.position_km,
                velocity_km_s: o2.velocity_km_s,
                covariance_km2: o2.covariance_km2,
            };
            match sidereon_core::astro::conjunction::collision_probability(
                &s1,
                &s2,
                hard_body_radius_km,
                method,
            ) {
                Ok(pc) => {
                    *out = SidereonCollisionPc {
                        pc: pc.pc,
                        miss_km: pc.miss_km,
                        relative_speed_km_s: pc.relative_speed_km_s,
                        sigma_x_km: pc.sigma_x_km,
                        sigma_z_km: pc.sigma_z_km,
                    };
                    SidereonStatus::Ok
                }
                Err(err) => map_conjunction_error("sidereon_collision_probability", err),
            }
        },
    )
}

// --- Encounter-plane covariance projection (sidereon_core::astro::conjunction)

/// Project a 3x3 ECI position covariance (km^2, row-major in cov_km2) into the 2D
/// encounter plane, writing the 2x2 result row-major into out (4 doubles).
/// Delegates to
/// sidereon_core::astro::conjunction::encounter_plane_covariance.
///
/// Safety: frame points to a SidereonEncounterFrame; cov_km2 points to 9
/// doubles; out points to 4 doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_encounter_plane_covariance(
    frame: *const SidereonEncounterFrame,
    cov_km2: *const f64,
    out: *mut f64,
) -> SidereonStatus {
    engine_error_operation_boundary(
        "sidereon_encounter_plane_covariance",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_encounter_plane_covariance",
                "out"
            ));
            for idx in 0..4 {
                *out.add(idx) = 0.0;
            }
            let frame = c_try!(require_ref(
                frame,
                "sidereon_encounter_plane_covariance",
                "frame"
            ));
            let cov = c_try!(read_mat3(
                "sidereon_encounter_plane_covariance",
                "cov_km2",
                cov_km2
            ));
            let core_frame = sidereon_core::astro::conjunction::EncounterFrame {
                x_hat: frame.x_hat,
                y_hat: frame.y_hat,
                z_hat: frame.z_hat,
                relative_position_km: frame.relative_position_km,
                relative_velocity_km_s: frame.relative_velocity_km_s,
                miss_km: frame.miss_km,
                relative_speed_km_s: frame.relative_speed_km_s,
            };
            match sidereon_core::astro::conjunction::encounter_plane_covariance(&core_frame, &cov) {
                Ok(m) => {
                    *out.add(0) = m[0][0];
                    *out.add(1) = m[0][1];
                    *out.add(2) = m[1][0];
                    *out.add(3) = m[1][1];
                    SidereonStatus::Ok
                }
                Err(err) => map_conjunction_error("sidereon_encounter_plane_covariance", err),
            }
        },
    )
}

/// TCA coarse-search options, mirroring
/// sidereon_core::astro::tca::TcaFinderOptions.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTcaFinderOptions {
    /// Coarse sampling step used to bracket local range minima, seconds.
    pub coarse_step_seconds: f64,
    /// Time tolerance to which each local minimum is refined, seconds.
    pub time_tolerance_seconds: f64,
}

/// One local time-of-closest-approach candidate, mirroring
/// sidereon_core::astro::tca::TcaCandidate.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTcaCandidate {
    /// Refined absolute TCA, Julian date whole part.
    pub tca_time_jd_whole: f64,
    /// Refined absolute TCA, Julian date fractional part.
    pub tca_time_jd_fraction: f64,
    /// Refined seconds since the search window start.
    pub tca_seconds_since_window_start: f64,
    /// Miss distance, km.
    pub miss_distance_km: f64,
    /// Primary minus secondary TEME position, km.
    pub relative_position_km: [f64; 3],
    /// Primary minus secondary TEME velocity, km/s.
    pub relative_velocity_km_s: [f64; 3],
}

/// Collision-probability options for a TCA candidate, mirroring
/// sidereon_core::astro::tca::TcaPcOptions. When use_default_covariance is true
/// the fallback per-object position covariances are used and the matrix fields
/// are ignored.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTcaPcOptions {
    /// Hard-body radius, km.
    pub hard_body_radius_km: f64,
    /// One of SidereonPcMethod as uint32_t.
    pub method: u32,
    /// Use the fallback TCA position covariances.
    pub use_default_covariance: bool,
    /// Primary GCRS position covariance, km^2 (row-major).
    pub primary_covariance_km2: [[f64; 3]; 3],
    /// Secondary GCRS position covariance, km^2 (row-major).
    pub secondary_covariance_km2: [[f64; 3]; 3],
}

/// A TCA candidate paired with its collision probability, mirroring
/// sidereon_core::astro::tca::TcaConjunction.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTcaConjunction {
    /// Refined TCA candidate.
    pub candidate: SidereonTcaCandidate,
    /// Collision probability at the TCA.
    pub collision_probability: SidereonCollisionPc,
}

/// One threshold-screening hit, mirroring
/// sidereon_core::astro::tca::TcaScreeningHit.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTcaScreeningHit {
    /// Index of the secondary in the caller-supplied catalog.
    pub secondary_index: usize,
    /// Refined TCA candidate at or below the threshold.
    pub candidate: SidereonTcaCandidate,
}

/// One threshold-screening hit with Pc, mirroring
/// sidereon_core::astro::tca::TcaScreeningConjunctionHit.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTcaScreeningConjunctionHit {
    /// Index of the secondary in the caller-supplied catalog.
    pub secondary_index: usize,
    /// TCA and Pc for this threshold breach.
    pub conjunction: SidereonTcaConjunction,
}

/// Propagated-covariance Pc options for a TCA candidate, mirroring
/// sidereon_core::astro::tca::TcaPropagatedCovariancePcOptions. The 6x6 initial
/// state covariances are row-major and validated by the engine.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTcaPropagatedCovariancePcOptions {
    /// Hard-body radius, km.
    pub hard_body_radius_km: f64,
    /// One of SidereonPcMethod as uint32_t.
    pub method: u32,
    /// Primary initial 6x6 state covariance at its TLE epoch (row-major).
    pub primary_covariance0: [[f64; 6]; 6],
    /// Secondary initial 6x6 state covariance at its TLE epoch (row-major).
    pub secondary_covariance0: [[f64; 6]; 6],
    /// One of SidereonPropagationForceModel as uint32_t (covariance transport).
    pub force_model: u32,
    /// One of SidereonPropagationIntegrator as uint32_t (covariance transport).
    pub integrator: u32,
    /// Absolute tolerance.
    pub abs_tol: f64,
    /// Relative tolerance.
    pub rel_tol: f64,
    /// Initial integration step, seconds.
    pub initial_step_s: f64,
    /// Minimum integration step, seconds.
    pub min_step_s: f64,
    /// Maximum integration step, seconds.
    pub max_step_s: f64,
    /// Maximum integration steps.
    pub max_steps: u32,
    /// Whether mu_km3_s2 overrides the engine default Earth value.
    pub mu_km3_s2_enabled: bool,
    /// Gravitational parameter in km^3/s^2 when enabled.
    pub mu_km3_s2: f64,
}

/// One borrowed TLE pair for a TCA secondary catalog.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTcaTlePair {
    /// Null-terminated TLE line 1.
    pub line1: *const c_char,
    /// Null-terminated TLE line 2.
    pub line2: *const c_char,
}

/// Fill *out_options with the engine default TCA finder options.
///
/// Safety: out_options must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tca_finder_options_init(
    out_options: *mut SidereonTcaFinderOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_tca_finder_options_init",
        SidereonStatus::Panic,
        || {
            let out_options = c_try!(require_out(
                out_options,
                "sidereon_tca_finder_options_init",
                "out_options"
            ));
            let d = core_tca::TcaFinderOptions::default();
            *out_options = SidereonTcaFinderOptions {
                coarse_step_seconds: d.coarse_step_seconds,
                time_tolerance_seconds: d.time_tolerance_seconds,
            };
            SidereonStatus::Ok
        },
    )
}

/// Find local TCA candidates between two satellites given by TLE strings, over a
/// search window expressed as split Julian dates. Variable-length output
/// contract. Delegates to
/// sidereon_core::astro::tca::find_tca_candidates_from_tles.
///
/// Safety: the four line pointers are null-terminated TLE lines; options points
/// to a SidereonTcaFinderOptions; out points to len SidereonTcaCandidate or NULL
/// when len is 0; out_written and out_required point to size_t.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn sidereon_find_tca_candidates_from_tles(
    primary_line1: *const c_char,
    primary_line2: *const c_char,
    secondary_line1: *const c_char,
    secondary_line2: *const c_char,
    window_start_jd_whole: f64,
    window_start_jd_fraction: f64,
    window_end_jd_whole: f64,
    window_end_jd_fraction: f64,
    options: *const SidereonTcaFinderOptions,
    out: *mut SidereonTcaCandidate,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    engine_error_operation_boundary(
        "sidereon_find_tca_candidates_from_tles",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_find_tca_candidates_from_tles",
                out_written,
                out_required
            ));
            let p1 = c_try!(tle_line_from_c(
                "sidereon_find_tca_candidates_from_tles",
                "primary_line1",
                primary_line1
            ));
            let p2 = c_try!(tle_line_from_c(
                "sidereon_find_tca_candidates_from_tles",
                "primary_line2",
                primary_line2
            ));
            let s1 = c_try!(tle_line_from_c(
                "sidereon_find_tca_candidates_from_tles",
                "secondary_line1",
                secondary_line1
            ));
            let s2 = c_try!(tle_line_from_c(
                "sidereon_find_tca_candidates_from_tles",
                "secondary_line2",
                secondary_line2
            ));
            let options = c_try!(require_ref(
                options,
                "sidereon_find_tca_candidates_from_tles",
                "options"
            ));
            let candidates = match core_tca::find_tca_candidates_from_tles(
                &p1,
                &p2,
                &s1,
                &s2,
                TcaJulianDate(window_start_jd_whole, window_start_jd_fraction),
                TcaJulianDate(window_end_jd_whole, window_end_jd_fraction),
                tca_finder_options_from_c(options),
            ) {
                Ok(c) => c,
                Err(err) => return map_tca_error("sidereon_find_tca_candidates_from_tles", err),
            };
            let mapped: Vec<SidereonTcaCandidate> = candidates
                .iter()
                .map(SidereonTcaCandidate::from_core)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_find_tca_candidates_from_tles",
                "out",
                &mapped,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Compute Pc for an already-refined TCA candidate. Delegates to
/// sidereon_core::astro::tca::tca_collision_probability.
///
/// Safety: candidate points to a SidereonTcaCandidate; options to a
/// SidereonTcaPcOptions; out to a SidereonTcaConjunction.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tca_collision_probability(
    candidate: *const SidereonTcaCandidate,
    options: *const SidereonTcaPcOptions,
    out: *mut SidereonTcaConjunction,
) -> SidereonStatus {
    engine_error_operation_boundary(
        "sidereon_tca_collision_probability",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_tca_collision_probability",
                "out"
            ));
            *out = SidereonTcaConjunction {
                candidate: SidereonTcaCandidate::ZERO,
                collision_probability: SidereonCollisionPc {
                    pc: 0.0,
                    miss_km: 0.0,
                    relative_speed_km_s: 0.0,
                    sigma_x_km: 0.0,
                    sigma_z_km: 0.0,
                },
            };
            let candidate = c_try!(require_ref(
                candidate,
                "sidereon_tca_collision_probability",
                "candidate"
            ))
            .to_core();
            let options = c_try!(require_ref(
                options,
                "sidereon_tca_collision_probability",
                "options"
            ));
            let pc_options = c_try!(tca_pc_options_from_c(
                "sidereon_tca_collision_probability",
                options
            ));
            match core_tca::tca_collision_probability(candidate, pc_options) {
                Ok(c) => {
                    *out = SidereonTcaConjunction::from_core(&c);
                    SidereonStatus::Ok
                }
                Err(err) => map_tca_error("sidereon_tca_collision_probability", err),
            }
        },
    )
}

/// Find TCA candidates between two TLE strings and compute Pc at each TCA.
/// Variable-length output contract. Delegates to
/// sidereon_core::astro::tca::find_tca_conjunctions_from_tles.
///
/// Safety: the four line pointers are null-terminated TLE lines; tca_options
/// points to a SidereonTcaFinderOptions; pc_options to a SidereonTcaPcOptions;
/// out points to len SidereonTcaConjunction or NULL when len is 0; out_written
/// and out_required point to size_t.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn sidereon_find_tca_conjunctions_from_tles(
    primary_line1: *const c_char,
    primary_line2: *const c_char,
    secondary_line1: *const c_char,
    secondary_line2: *const c_char,
    window_start_jd_whole: f64,
    window_start_jd_fraction: f64,
    window_end_jd_whole: f64,
    window_end_jd_fraction: f64,
    tca_options: *const SidereonTcaFinderOptions,
    pc_options: *const SidereonTcaPcOptions,
    out: *mut SidereonTcaConjunction,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    engine_error_operation_boundary(
        "sidereon_find_tca_conjunctions_from_tles",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_find_tca_conjunctions_from_tles",
                out_written,
                out_required
            ));
            let p1 = c_try!(tle_line_from_c(
                "sidereon_find_tca_conjunctions_from_tles",
                "primary_line1",
                primary_line1
            ));
            let p2 = c_try!(tle_line_from_c(
                "sidereon_find_tca_conjunctions_from_tles",
                "primary_line2",
                primary_line2
            ));
            let s1 = c_try!(tle_line_from_c(
                "sidereon_find_tca_conjunctions_from_tles",
                "secondary_line1",
                secondary_line1
            ));
            let s2 = c_try!(tle_line_from_c(
                "sidereon_find_tca_conjunctions_from_tles",
                "secondary_line2",
                secondary_line2
            ));
            let tca_options = c_try!(require_ref(
                tca_options,
                "sidereon_find_tca_conjunctions_from_tles",
                "tca_options"
            ));
            let pc_options = c_try!(require_ref(
                pc_options,
                "sidereon_find_tca_conjunctions_from_tles",
                "pc_options"
            ));
            let pc = c_try!(tca_pc_options_from_c(
                "sidereon_find_tca_conjunctions_from_tles",
                pc_options
            ));
            let conjunctions = match core_tca::find_tca_conjunctions_from_tles(
                core_tca::TcaTle::new(&p1, &p2),
                core_tca::TcaTle::new(&s1, &s2),
                TcaJulianDate(window_start_jd_whole, window_start_jd_fraction),
                TcaJulianDate(window_end_jd_whole, window_end_jd_fraction),
                tca_finder_options_from_c(tca_options),
                pc,
            ) {
                Ok(c) => c,
                Err(err) => return map_tca_error("sidereon_find_tca_conjunctions_from_tles", err),
            };
            let mapped: Vec<SidereonTcaConjunction> = conjunctions
                .iter()
                .map(SidereonTcaConjunction::from_core)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_find_tca_conjunctions_from_tles",
                "out",
                &mapped,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Find TCA candidates between two TLE strings and compute Pc after propagating
/// each object's initial state covariance to the candidate TCA. Variable-length
/// output contract. Delegates to
/// sidereon_core::astro::tca::find_tca_conjunctions_with_propagated_covariance_from_tles.
///
/// Safety: the four line pointers are null-terminated TLE lines; tca_options
/// points to a SidereonTcaFinderOptions; pc_options to a
/// SidereonTcaPropagatedCovariancePcOptions; out points to len
/// SidereonTcaConjunction or NULL when len is 0; out_written and out_required
/// point to size_t.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn sidereon_find_tca_conjunctions_with_propagated_covariance_from_tles(
    primary_line1: *const c_char,
    primary_line2: *const c_char,
    secondary_line1: *const c_char,
    secondary_line2: *const c_char,
    window_start_jd_whole: f64,
    window_start_jd_fraction: f64,
    window_end_jd_whole: f64,
    window_end_jd_fraction: f64,
    tca_options: *const SidereonTcaFinderOptions,
    pc_options: *const SidereonTcaPropagatedCovariancePcOptions,
    out: *mut SidereonTcaConjunction,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    engine_error_operation_boundary(
        "sidereon_find_tca_conjunctions_with_propagated_covariance_from_tles",
        SidereonStatus::Panic,
        || {
            let fname = "sidereon_find_tca_conjunctions_with_propagated_covariance_from_tles";
            c_try!(init_copy_counts(fname, out_written, out_required));
            let p1 = c_try!(tle_line_from_c(fname, "primary_line1", primary_line1));
            let p2 = c_try!(tle_line_from_c(fname, "primary_line2", primary_line2));
            let s1 = c_try!(tle_line_from_c(fname, "secondary_line1", secondary_line1));
            let s2 = c_try!(tle_line_from_c(fname, "secondary_line2", secondary_line2));
            let tca_options = c_try!(require_ref(tca_options, fname, "tca_options"));
            let pc_options = c_try!(require_ref(pc_options, fname, "pc_options"));
            let method = c_try!(pc_method_from_c(fname, pc_options.method));
            let force_model = c_try!(force_model_kind_from_c(
                fname,
                pc_options.force_model,
                pc_options.mu_km3_s2_enabled,
                pc_options.mu_km3_s2,
            ));
            let integrator = c_try!(propagation_integrator_from_c(fname, pc_options.integrator));
            let primary_covariance0 = c_try!(Covariance6::try_from_matrix(
                pc_options.primary_covariance0
            )
            .map_err(|err| { extra_invalid_arg(fname, format!("primary_covariance0: {err:?}")) }));
            let secondary_covariance0 = c_try!(Covariance6::try_from_matrix(
                pc_options.secondary_covariance0
            )
            .map_err(|err| {
                extra_invalid_arg(fname, format!("secondary_covariance0: {err:?}"))
            }));
            let mut integrator_options = IntegratorOptions::default();
            integrator_options.abs_tol = pc_options.abs_tol;
            integrator_options.rel_tol = pc_options.rel_tol;
            integrator_options.initial_step = pc_options.initial_step_s;
            integrator_options.min_step = pc_options.min_step_s;
            integrator_options.max_step = pc_options.max_step_s;
            integrator_options.max_steps = pc_options.max_steps;
            integrator_options.dense_output = false;

            let mut prop_options = core_tca::TcaPropagatedCovariancePcOptions::new(
                pc_options.hard_body_radius_km,
                method,
                primary_covariance0,
                secondary_covariance0,
            );
            prop_options.force_model = force_model;
            prop_options.integrator = integrator;
            prop_options.integrator_options = integrator_options;
            prop_options.process_noise = sidereon_core::astro::propagator::ProcessNoise::None;
            let conjunctions =
                match core_tca::find_tca_conjunctions_with_propagated_covariance_from_tles(
                    core_tca::TcaTle::new(&p1, &p2),
                    core_tca::TcaTle::new(&s1, &s2),
                    TcaJulianDate(window_start_jd_whole, window_start_jd_fraction),
                    TcaJulianDate(window_end_jd_whole, window_end_jd_fraction),
                    tca_finder_options_from_c(tca_options),
                    prop_options,
                ) {
                    Ok(c) => c,
                    Err(err) => return map_tca_error(fname, err),
                };
            let mapped: Vec<SidereonTcaConjunction> = conjunctions
                .iter()
                .map(SidereonTcaConjunction::from_core)
                .collect();
            c_try!(copy_prefix_to_c(
                fname,
                "out",
                &mapped,
                out,
                len,
                out_written,
                out_required
            ));
            SidereonStatus::Ok
        },
    )
}

// Read a secondary TLE catalog into owned line pairs, returning them so the
// borrowed TcaTle views built from them stay valid for the screen call.

/// Serially screen a primary TLE against a secondary TLE catalog for threshold
/// TCAs. Variable-length output contract. Delegates to
/// sidereon_core::astro::tca::screen_tca_candidates_from_tle_catalog_serial.
///
/// Safety: the two primary line pointers are null-terminated TLE lines;
/// secondaries points to secondary_count SidereonTcaTlePair; options to a
/// SidereonTcaFinderOptions; out points to len SidereonTcaScreeningHit or NULL
/// when len is 0; out_written and out_required point to size_t.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn sidereon_screen_tca_candidates_from_tle_catalog(
    primary_line1: *const c_char,
    primary_line2: *const c_char,
    secondaries: *const SidereonTcaTlePair,
    secondary_count: usize,
    window_start_jd_whole: f64,
    window_start_jd_fraction: f64,
    window_end_jd_whole: f64,
    window_end_jd_fraction: f64,
    miss_distance_threshold_km: f64,
    options: *const SidereonTcaFinderOptions,
    out: *mut SidereonTcaScreeningHit,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    engine_error_operation_boundary(
        "sidereon_screen_tca_candidates_from_tle_catalog",
        SidereonStatus::Panic,
        || {
            let fname = "sidereon_screen_tca_candidates_from_tle_catalog";
            c_try!(init_copy_counts(fname, out_written, out_required));
            let p1 = c_try!(tle_line_from_c(fname, "primary_line1", primary_line1));
            let p2 = c_try!(tle_line_from_c(fname, "primary_line2", primary_line2));
            let options = c_try!(require_ref(options, fname, "options"));
            let secondary_lines = c_try!(tca_secondary_lines_from_c(
                fname,
                secondaries,
                secondary_count
            ));
            let secondary_tles: Vec<core_tca::TcaTle> = secondary_lines
                .iter()
                .map(|(l1, l2)| core_tca::TcaTle::new(l1.as_str(), l2.as_str()))
                .collect();
            let window = core_tca::TcaWindow::new(
                TcaJulianDate(window_start_jd_whole, window_start_jd_fraction),
                TcaJulianDate(window_end_jd_whole, window_end_jd_fraction),
            );
            let hits = match core_tca::screen_tca_candidates_from_tle_catalog_serial(
                core_tca::TcaTle::new(&p1, &p2),
                &secondary_tles,
                window,
                miss_distance_threshold_km,
                tca_finder_options_from_c(options),
            ) {
                Ok(h) => h,
                Err(err) => return map_tca_error(fname, err),
            };
            let mapped: Vec<SidereonTcaScreeningHit> = hits
                .iter()
                .map(|h| SidereonTcaScreeningHit {
                    secondary_index: h.secondary_index,
                    candidate: SidereonTcaCandidate::from_core(&h.candidate),
                })
                .collect();
            c_try!(copy_prefix_to_c(
                fname,
                "out",
                &mapped,
                out,
                len,
                out_written,
                out_required
            ));
            SidereonStatus::Ok
        },
    )
}

/// Serially screen a primary TLE against a secondary TLE catalog and compute Pc
/// for each threshold breach. Variable-length output contract. Delegates to
/// sidereon_core::astro::tca::screen_tca_conjunctions_from_tle_catalog_serial.
///
/// Safety: the two primary line pointers are null-terminated TLE lines;
/// secondaries points to secondary_count SidereonTcaTlePair; tca_options to a
/// SidereonTcaFinderOptions; pc_options to a SidereonTcaPcOptions; out points to
/// len SidereonTcaScreeningConjunctionHit or NULL when len is 0; out_written and
/// out_required point to size_t.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn sidereon_screen_tca_conjunctions_from_tle_catalog(
    primary_line1: *const c_char,
    primary_line2: *const c_char,
    secondaries: *const SidereonTcaTlePair,
    secondary_count: usize,
    window_start_jd_whole: f64,
    window_start_jd_fraction: f64,
    window_end_jd_whole: f64,
    window_end_jd_fraction: f64,
    miss_distance_threshold_km: f64,
    tca_options: *const SidereonTcaFinderOptions,
    pc_options: *const SidereonTcaPcOptions,
    out: *mut SidereonTcaScreeningConjunctionHit,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    engine_error_operation_boundary(
        "sidereon_screen_tca_conjunctions_from_tle_catalog",
        SidereonStatus::Panic,
        || {
            let fname = "sidereon_screen_tca_conjunctions_from_tle_catalog";
            c_try!(init_copy_counts(fname, out_written, out_required));
            let p1 = c_try!(tle_line_from_c(fname, "primary_line1", primary_line1));
            let p2 = c_try!(tle_line_from_c(fname, "primary_line2", primary_line2));
            let tca_options = c_try!(require_ref(tca_options, fname, "tca_options"));
            let pc_options = c_try!(require_ref(pc_options, fname, "pc_options"));
            let pc = c_try!(tca_pc_options_from_c(fname, pc_options));
            let secondary_lines = c_try!(tca_secondary_lines_from_c(
                fname,
                secondaries,
                secondary_count
            ));
            let secondary_tles: Vec<core_tca::TcaTle> = secondary_lines
                .iter()
                .map(|(l1, l2)| core_tca::TcaTle::new(l1.as_str(), l2.as_str()))
                .collect();
            let window = core_tca::TcaWindow::new(
                TcaJulianDate(window_start_jd_whole, window_start_jd_fraction),
                TcaJulianDate(window_end_jd_whole, window_end_jd_fraction),
            );
            let hits = match core_tca::screen_tca_conjunctions_from_tle_catalog_serial(
                core_tca::TcaTle::new(&p1, &p2),
                &secondary_tles,
                window,
                miss_distance_threshold_km,
                tca_finder_options_from_c(tca_options),
                pc,
            ) {
                Ok(h) => h,
                Err(err) => return map_tca_error(fname, err),
            };
            let mapped: Vec<SidereonTcaScreeningConjunctionHit> = hits
                .iter()
                .map(|h| SidereonTcaScreeningConjunctionHit {
                    secondary_index: h.secondary_index,
                    conjunction: SidereonTcaConjunction::from_core(&h.conjunction),
                })
                .collect();
            c_try!(copy_prefix_to_c(
                fname,
                "out",
                &mapped,
                out,
                len,
                out_written,
                out_required
            ));
            SidereonStatus::Ok
        },
    )
}

/// Compose the concrete [`ForceModelKind`] for the non-propagator paths (the TCA
/// propagated-covariance API) that take the same force-model selector and
/// gravitational-parameter override but feed a [`ForceModelKind`] directly rather
/// than running [`propagate_states`]. The composition (filling the canonical
/// Earth reference radius and J2 for the J2 variant, defaulting the gravitational
/// parameter) is the core driver's via [`PropagationConfig::force_model_kind`], so
/// no force-model constant policy lives in the binding.
fn force_model_kind_from_c(
    fn_name: &str,
    force_model: u32,
    mu_km3_s2_enabled: bool,
    mu_km3_s2: f64,
) -> Result<ForceModelKind, SidereonStatus> {
    let mut config = PropagationConfig::new(0.0, [0.0; 3], [0.0; 3]);
    config.force_model = propagation_force_model_from_c(fn_name, force_model)?;
    config.mu_km3_s2 = mu_km3_s2_enabled.then_some(mu_km3_s2);
    Ok(config.force_model_kind())
}

impl SidereonTcaCandidate {
    pub(crate) fn from_core(c: &core_tca::TcaCandidate) -> Self {
        Self {
            tca_time_jd_whole: c.tca_time.0,
            tca_time_jd_fraction: c.tca_time.1,
            tca_seconds_since_window_start: c.tca_seconds_since_window_start,
            miss_distance_km: c.miss_distance_km,
            relative_position_km: c.relative_position_km,
            relative_velocity_km_s: c.relative_velocity_km_s,
        }
    }

    pub(crate) fn to_core(self) -> core_tca::TcaCandidate {
        core_tca::TcaCandidate {
            tca_time: TcaJulianDate(self.tca_time_jd_whole, self.tca_time_jd_fraction),
            tca_seconds_since_window_start: self.tca_seconds_since_window_start,
            miss_distance_km: self.miss_distance_km,
            relative_position_km: self.relative_position_km,
            relative_velocity_km_s: self.relative_velocity_km_s,
        }
    }

    pub(crate) const ZERO: Self = Self {
        tca_time_jd_whole: 0.0,
        tca_time_jd_fraction: 0.0,
        tca_seconds_since_window_start: 0.0,
        miss_distance_km: 0.0,
        relative_position_km: [0.0; 3],
        relative_velocity_km_s: [0.0; 3],
    };
}

impl SidereonTcaConjunction {
    pub(crate) fn from_core(c: &core_tca::TcaConjunction) -> Self {
        Self {
            candidate: SidereonTcaCandidate::from_core(&c.candidate),
            collision_probability: SidereonCollisionPc {
                pc: c.collision_probability.pc,
                miss_km: c.collision_probability.miss_km,
                relative_speed_km_s: c.collision_probability.relative_speed_km_s,
                sigma_x_km: c.collision_probability.sigma_x_km,
                sigma_z_km: c.collision_probability.sigma_z_km,
            },
        }
    }
}

fn map_tca_error(fn_name: &str, err: core_tca::TcaError) -> SidereonStatus {
    record_engine_error(
        SidereonEngineErrorFamily::Tca,
        fn_name,
        tca_error_value(&err),
    );
    extra_invalid_arg(fn_name, err)
}

fn tca_finder_options_from_c(o: &SidereonTcaFinderOptions) -> core_tca::TcaFinderOptions {
    let mut opts = core_tca::TcaFinderOptions::default();
    opts.coarse_step_seconds = o.coarse_step_seconds;
    opts.time_tolerance_seconds = o.time_tolerance_seconds;
    opts
}

fn tca_pc_options_from_c(
    fn_name: &str,
    o: &SidereonTcaPcOptions,
) -> Result<core_tca::TcaPcOptions, SidereonStatus> {
    let method = pc_method_from_c(fn_name, o.method)?;
    Ok(if o.use_default_covariance {
        core_tca::TcaPcOptions::with_default_covariance(o.hard_body_radius_km, method)
    } else {
        core_tca::TcaPcOptions::with_covariances(
            o.hard_body_radius_km,
            method,
            o.primary_covariance_km2,
            o.secondary_covariance_km2,
        )
    })
}

unsafe fn tca_secondary_lines_from_c(
    fn_name: &str,
    secondaries: *const SidereonTcaTlePair,
    count: usize,
) -> Result<Vec<(String, String)>, SidereonStatus> {
    let rows = require_slice(secondaries, count, fn_name, "secondaries")?;
    let mut out = Vec::with_capacity(count);
    for row in rows {
        let line1 = tle_line_from_c(fn_name, "secondary.line1", row.line1)?;
        let line2 = tle_line_from_c(fn_name, "secondary.line2", row.line2)?;
        out.push((line1, line2));
    }
    Ok(out)
}

fn pc_method_from_c(fn_name: &str, method: u32) -> Result<PcMethod, SidereonStatus> {
    match method {
        v if v == SidereonPcMethod::FosterEqualArea as u32 => Ok(PcMethod::FosterEqualArea),
        v if v == SidereonPcMethod::FosterNumerical as u32 => Ok(PcMethod::FosterNumerical),
        v if v == SidereonPcMethod::Alfano2005 as u32 => Ok(PcMethod::Alfano2005),
        _ => {
            set_last_error(format!("{fn_name}: invalid Pc method"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_error::{
        clear_engine_error, sidereon_last_engine_error_info, sidereon_last_engine_error_payload,
        SidereonEngineErrorInfo,
    };
    use serde_json::json;
    use sidereon_core::astro::conjunction::ConjunctionError;
    use sidereon_core::astro::events::EventFinderError;
    use sidereon_core::astro::sgp4::Error as Sgp4Error;
    use sidereon_core::astro::tca::{TcaError, TcaObject};
    use sidereon_core::astro::time::DegradeReason;
    use std::ffi::CStr;
    use std::ptr;

    fn get_last_error_string() -> String {
        unsafe {
            let len = sidereon_last_error_message(ptr::null_mut(), 0);
            if len == 0 {
                return String::new();
            }
            let mut buf = vec![0 as c_char; len + 1];
            sidereon_last_error_message(buf.as_mut_ptr(), buf.len());
            CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned()
        }
    }

    fn get_last_engine_error_payload() -> (SidereonEngineErrorInfo, serde_json::Value) {
        unsafe {
            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            let status = sidereon_last_engine_error_info(&mut info);
            assert_eq!(status, SidereonStatus::Ok);

            let mut written = 0usize;
            let mut required = 0usize;
            let status =
                sidereon_last_engine_error_payload(ptr::null_mut(), 0, &mut written, &mut required);
            assert_eq!(status, SidereonStatus::Ok);
            assert_eq!(written, 0);
            assert_eq!(required, info.payload_len);

            if required == 0 {
                return (info, json!(null));
            }

            let mut buf = vec![0u8; required];
            let status = sidereon_last_engine_error_payload(
                buf.as_mut_ptr(),
                buf.len(),
                &mut written,
                &mut required,
            );
            assert_eq!(status, SidereonStatus::Ok);
            assert_eq!(written, required);
            let payload_str = String::from_utf8(buf).expect("valid utf-8 payload");
            let parsed = serde_json::from_str(&payload_str).expect("parse json");
            (info, parsed)
        }
    }

    #[test]
    fn test_conjunction_error_mapper_variants() {
        let cases = vec![
            (
                ConjunctionError::NonFinite {
                    field: "object1.position_km",
                },
                json!({
                    "kind": "non_finite",
                    "fields": { "field": "object1.position_km" }
                }),
            ),
            (
                ConjunctionError::NotPositive {
                    field: "hard_body_radius_km",
                },
                json!({
                    "kind": "not_positive",
                    "fields": { "field": "hard_body_radius_km" }
                }),
            ),
            (
                ConjunctionError::UndefinedFrame,
                json!({
                    "kind": "undefined_frame",
                    "fields": {}
                }),
            ),
        ];

        for (err, expected) in cases {
            assert_eq!(conjunction_error_value(&err), expected);
        }
    }

    #[test]
    fn test_event_finder_error_mapper_variants() {
        let cases = vec![
            (
                EventFinderError::InvalidInput {
                    field: "step_seconds",
                    reason: "must be positive",
                },
                json!({
                    "kind": "invalid_input",
                    "fields": {
                        "field": "step_seconds",
                        "reason": "must be positive"
                    }
                }),
            ),
            (
                EventFinderError::Ut1OutsideCoverage(DegradeReason::BeforeCoverage),
                json!({
                    "kind": "ut1_outside_coverage",
                    "fields": { "reason": "before_coverage" }
                }),
            ),
            (
                EventFinderError::Ut1OutsideCoverage(DegradeReason::AfterCoverage),
                json!({
                    "kind": "ut1_outside_coverage",
                    "fields": { "reason": "after_coverage" }
                }),
            ),
        ];

        for (err, expected) in cases {
            assert_eq!(event_finder_error_value(&err), expected);
        }
    }

    #[test]
    fn test_tca_error_mapper_variants() {
        let cases = vec![
            (
                TcaError::InvalidInput {
                    field: "window_start",
                    reason: "not finite",
                },
                json!({
                    "kind": "invalid_input",
                    "fields": {
                        "field": "window_start",
                        "reason": "not finite"
                    }
                }),
            ),
            (
                TcaError::Init {
                    object: TcaObject::Primary,
                    source: Sgp4Error::InvalidTle("corrupt line 2".to_string()),
                },
                json!({
                    "kind": "init",
                    "fields": {
                        "object": "primary",
                        "cause": {
                            "kind": "invalid_tle",
                            "fields": { "message": "corrupt line 2" }
                        }
                    }
                }),
            ),
            (
                TcaError::Propagate {
                    object: TcaObject::Secondary,
                    source: Sgp4Error::NonFiniteOutput { field: "velocity" },
                },
                json!({
                    "kind": "propagate",
                    "fields": {
                        "object": "secondary",
                        "cause": {
                            "kind": "non_finite_output",
                            "fields": { "field": "velocity" }
                        }
                    }
                }),
            ),
            (
                TcaError::CovariancePropagation {
                    object: TcaObject::Primary,
                    reason: "integration divergence".to_string(),
                },
                json!({
                    "kind": "covariance_propagation",
                    "fields": {
                        "object": "primary",
                        "reason": "integration divergence"
                    }
                }),
            ),
            (
                TcaError::EventFinder(EventFinderError::InvalidInput {
                    field: "window",
                    reason: "empty window",
                }),
                json!({
                    "kind": "event_finder",
                    "fields": {
                        "cause": {
                            "kind": "invalid_input",
                            "fields": {
                                "field": "window",
                                "reason": "empty window"
                            }
                        }
                    }
                }),
            ),
            (
                TcaError::Conjunction(ConjunctionError::UndefinedFrame),
                json!({
                    "kind": "conjunction",
                    "fields": {
                        "cause": {
                            "kind": "undefined_frame",
                            "fields": {}
                        }
                    }
                }),
            ),
        ];

        for (err, expected) in cases {
            assert_eq!(tca_error_value(&err), expected);
        }
    }

    #[test]
    fn test_encounter_frame_valid_control_and_real_producer_refusal() {
        clear_engine_error();

        // 1. Valid control
        let r1 = [7000.0, 0.0, 0.0];
        let v1 = [0.0, 7.5, 0.0];
        let r2 = [7000.05, 0.02, 0.0];
        let v2 = [0.0, -7.5, 0.1];
        let mut frame = SidereonEncounterFrame {
            x_hat: [0.0; 3],
            y_hat: [0.0; 3],
            z_hat: [0.0; 3],
            relative_position_km: [0.0; 3],
            relative_velocity_km_s: [0.0; 3],
            miss_km: 0.0,
            relative_speed_km_s: 0.0,
        };

        let status = unsafe {
            sidereon_encounter_frame(
                r1.as_ptr(),
                v1.as_ptr(),
                r2.as_ptr(),
                v2.as_ptr(),
                &mut frame,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert!(frame.relative_speed_km_s > 0.0);

        // TLS cleared on success
        let (info, payload) = get_last_engine_error_payload();
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(payload, json!(null));

        // 2. Real refusal: identical velocities -> zero relative velocity -> UndefinedFrame
        let v2_same = [0.0, 7.5, 0.0];
        let status = unsafe {
            sidereon_encounter_frame(
                r1.as_ptr(),
                v1.as_ptr(),
                r2.as_ptr(),
                v2_same.as_ptr(),
                &mut frame,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);

        let (info, payload) = get_last_engine_error_payload();
        assert_eq!(info.family, SidereonEngineErrorFamily::Conjunction);
        assert_eq!(payload["schema_version"], 1);
        assert_eq!(payload["family"], "conjunction");
        assert_eq!(payload["operation"], "sidereon_encounter_frame");
        assert_eq!(payload["error"]["kind"], "undefined_frame");
        assert_eq!(payload["error"]["fields"], json!({}));

        let msg = get_last_error_string();
        assert!(msg.contains("encounter frame is undefined"));

        // 3. Seeded success reset: running valid control clears previous failure
        let status = unsafe {
            sidereon_encounter_frame(
                r1.as_ptr(),
                v1.as_ptr(),
                r2.as_ptr(),
                v2.as_ptr(),
                &mut frame,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        let (info, payload) = get_last_engine_error_payload();
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(payload, json!(null));
    }

    #[test]
    fn test_tca_find_candidates_real_refusal_and_early_null_clear() {
        clear_engine_error();

        let l1 = b"1 25544U 98067A   18184.80969102  .00001614  00000-0  31745-4 0  9993\0";
        let l2 = b"2 25544  51.6414 295.8524 0003435 262.6267 204.2868 15.54005638121106\0";
        let l2_bad = b"INVALID LINE 2\0";

        let opts = SidereonTcaFinderOptions {
            coarse_step_seconds: 60.0,
            time_tolerance_seconds: 0.1,
        };

        // 1. Valid control: good pair succeeds and leaves engine error empty
        let mut out_written = 999usize;
        let mut out_required = 999usize;

        let status = unsafe {
            sidereon_find_tca_candidates_from_tles(
                l1.as_ptr() as *const c_char,
                l2.as_ptr() as *const c_char,
                l1.as_ptr() as *const c_char,
                l2.as_ptr() as *const c_char,
                2458303.0,
                0.5,
                2458304.0,
                0.5,
                &opts,
                ptr::null_mut(),
                0,
                &mut out_written,
                &mut out_required,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        let (info, payload) = get_last_engine_error_payload();
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(payload, json!(null));

        // 2. Real producer refusal: corrupt only line 2 to isolate Init/primary/invalid_tle
        let status = unsafe {
            sidereon_find_tca_candidates_from_tles(
                l1.as_ptr() as *const c_char,
                l2_bad.as_ptr() as *const c_char,
                l1.as_ptr() as *const c_char,
                l2.as_ptr() as *const c_char,
                2458303.0,
                0.5,
                2458304.0,
                0.5,
                &opts,
                ptr::null_mut(),
                0,
                &mut out_written,
                &mut out_required,
            )
        };

        assert_eq!(status, SidereonStatus::InvalidArgument);
        assert_eq!(out_written, 0);
        assert_eq!(out_required, 0);

        let (info, payload) = get_last_engine_error_payload();
        assert_eq!(info.family, SidereonEngineErrorFamily::Tca);
        assert_eq!(payload["schema_version"], 1);
        assert_eq!(payload["family"], "tca");
        assert_eq!(
            payload["operation"],
            "sidereon_find_tca_candidates_from_tles"
        );
        assert_eq!(payload["error"]["kind"], "init");
        assert_eq!(payload["error"]["fields"]["object"], "primary");
        assert_eq!(payload["error"]["fields"]["cause"]["kind"], "invalid_tle");

        // 3. Early NULL call clears retained engine error
        let status = unsafe {
            sidereon_find_tca_candidates_from_tles(
                ptr::null(),
                l2.as_ptr() as *const c_char,
                l1.as_ptr() as *const c_char,
                l2.as_ptr() as *const c_char,
                2458303.0,
                0.5,
                2458304.0,
                0.5,
                &opts,
                ptr::null_mut(),
                0,
                &mut out_written,
                &mut out_required,
            )
        };
        assert_eq!(status, SidereonStatus::NullPointer);
        let (info, payload) = get_last_engine_error_payload();
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(payload, json!(null));
    }
}
