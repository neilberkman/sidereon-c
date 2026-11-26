use super::*;
use crate::engine_error::{
    engine_error_operation_boundary, engine_f64, record_engine_error, SidereonEngineErrorFamily,
};
use serde_json::json;
use sidereon_core::astro::propagator::analytic::{
    self, AnalyticPropagationError, AnalyticTheory, BrouwerLyddanePropagator, CircularElements,
    EcksteinHechlerPropagator, KeplerianElements, MeanConversionOptions, ZonalHarmonics,
};

/// Zonal theory used by an analytic propagator.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidereonAnalyticTheory {
    /// Eckstein-Hechler theory.
    EcksteinHechler = 0,
    /// Brouwer-Lyddane theory.
    BrouwerLyddane = 1,
}

fn analytic_error_value(error: AnalyticPropagationError) -> serde_json::Value {
    use AnalyticPropagationError as E;
    let (kind, fields) = match error {
        E::NonFinite { field } => ("non_finite", json!({"field":field})),
        E::InvalidParameter { field, reason } => {
            ("invalid_parameter", json!({"field":field, "reason":reason}))
        }
        E::NotElliptic { field } => ("not_elliptic", json!({"field":field})),
        E::RetrogradeEquatorial { inclination_rad } => (
            "retrograde_equatorial",
            json!({"inclination_rad":engine_f64(inclination_rad)}),
        ),
        E::InsideReferenceSphere {
            semi_major_axis_km,
            reference_radius_km,
        } => (
            "inside_reference_sphere",
            json!({
                "semi_major_axis_km":engine_f64(semi_major_axis_km),
                "reference_radius_km":engine_f64(reference_radius_km),
            }),
        ),
        E::EccentricityTooLarge {
            eccentricity,
            limit,
        } => (
            "eccentricity_too_large",
            json!({"eccentricity":engine_f64(eccentricity), "limit":engine_f64(limit)}),
        ),
        E::NearlyEquatorial { inclination_rad } => (
            "nearly_equatorial",
            json!({"inclination_rad":engine_f64(inclination_rad)}),
        ),
        E::NearCriticalInclination { inclination_rad } => (
            "near_critical_inclination",
            json!({"inclination_rad":engine_f64(inclination_rad)}),
        ),
        E::MeanElementsNotConverged { theory, iterations } => (
            "mean_elements_not_converged",
            json!({"theory":theory.to_string(), "iterations":iterations}),
        ),
        E::KeplerNotConverged { field } => ("kepler_not_converged", json!({"field":field})),
        E::NonFiniteResult { theory, field } => (
            "non_finite_result",
            json!({"theory":theory.to_string(), "field":field}),
        ),
    };
    json!({"kind":kind, "fields":fields})
}

fn map_analytic_error(operation: &str, error: AnalyticPropagationError) -> SidereonStatus {
    record_engine_error(
        SidereonEngineErrorFamily::AnalyticPropagation,
        operation,
        analytic_error_value(error),
    );
    extra_invalid_arg(operation, error)
}

unsafe fn write_analytic<T: Copy>(
    operation: &str,
    out: *mut T,
    evaluate: impl FnOnce() -> Result<T, AnalyticPropagationError>,
) -> SidereonStatus {
    engine_error_operation_boundary(operation, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, operation, "out"));
        let value = c_try!(evaluate().map_err(|err| map_analytic_error(operation, err)));
        out.write(value);
        SidereonStatus::Ok
    })
}

/// Copy the analytic theory name as UTF-8 without a terminator.
/// NULL and zero capacity query the full byte count.
///
/// Safety: all outputs are writable and disjoint. out_text has capacity bytes.
#[no_mangle]
pub unsafe extern "C" fn sidereon_analytic_theory_text(
    theory: u32,
    out_text: *mut u8,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_analytic_theory_text";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(OP, out_written, out_required));
        let theory = match theory {
            0 => AnalyticTheory::EcksteinHechler,
            1 => AnalyticTheory::BrouwerLyddane,
            _ => return extra_invalid_arg(OP, format_args!("invalid analytic theory {theory}")),
        };
        let text = theory.to_string();
        c_try!(copy_prefix_to_c(
            OP,
            "out_text",
            text.as_bytes(),
            out_text,
            capacity,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Unnormalized zonal gravity coefficients. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SidereonZonalHarmonics {
    /// Gravitational parameter in cubic kilometres per second squared.
    pub mu_km3_s2: f64,
    /// Reference radius in kilometres.
    pub re_km: f64,
    /// Unnormalized degree-2 zonal coefficient.
    pub c20: f64,
    /// Unnormalized degree-3 zonal coefficient.
    pub c30: f64,
    /// Unnormalized degree-4 zonal coefficient.
    pub c40: f64,
    /// Unnormalized degree-5 zonal coefficient.
    pub c50: f64,
    /// Unnormalized degree-6 zonal coefficient.
    pub c60: f64,
}

impl From<SidereonZonalHarmonics> for ZonalHarmonics {
    fn from(value: SidereonZonalHarmonics) -> Self {
        Self {
            mu_km3_s2: value.mu_km3_s2,
            re_km: value.re_km,
            c20: value.c20,
            c30: value.c30,
            c40: value.c40,
            c50: value.c50,
            c60: value.c60,
        }
    }
}

impl From<ZonalHarmonics> for SidereonZonalHarmonics {
    fn from(value: ZonalHarmonics) -> Self {
        Self {
            mu_km3_s2: value.mu_km3_s2,
            re_km: value.re_km,
            c20: value.c20,
            c30: value.c30,
            c40: value.c40,
            c50: value.c50,
            c60: value.c60,
        }
    }
}

/// Mean element iteration settings. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SidereonMeanConversionOptions {
    /// Relative convergence tolerance.
    pub tolerance: f64,
    /// Maximum number of iterations.
    pub max_iterations: u32,
}

impl From<SidereonMeanConversionOptions> for MeanConversionOptions {
    fn from(value: SidereonMeanConversionOptions) -> Self {
        Self {
            tolerance: value.tolerance,
            max_iterations: value.max_iterations,
        }
    }
}

impl From<MeanConversionOptions> for SidereonMeanConversionOptions {
    fn from(value: MeanConversionOptions) -> Self {
        Self {
            tolerance: value.tolerance,
            max_iterations: value.max_iterations,
        }
    }
}

/// Inertial Cartesian coordinates for an analytic orbit. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SidereonAnalyticCartesian {
    /// Position in kilometres.
    pub position_km: [f64; 3],
    /// Velocity in kilometres per second.
    pub velocity_km_s: [f64; 3],
}

/// Analytic orbit elements. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SidereonCircularElements {
    /// Semi-major axis in kilometres.
    pub a_km: f64,
    /// Eccentricity times cosine of argument of perigee.
    pub ex: f64,
    /// Eccentricity times sine of argument of perigee.
    pub ey: f64,
    /// Inclination in radians.
    pub i_rad: f64,
    /// Right ascension of the ascending node in radians.
    pub raan_rad: f64,
    /// Mean latitude argument in radians.
    pub mean_latitude_argument_rad: f64,
}

impl From<SidereonCircularElements> for CircularElements {
    fn from(value: SidereonCircularElements) -> Self {
        Self {
            a_km: value.a_km,
            ex: value.ex,
            ey: value.ey,
            i_rad: value.i_rad,
            raan_rad: value.raan_rad,
            mean_latitude_argument_rad: value.mean_latitude_argument_rad,
        }
    }
}

impl From<CircularElements> for SidereonCircularElements {
    fn from(value: CircularElements) -> Self {
        Self {
            a_km: value.a_km,
            ex: value.ex,
            ey: value.ey,
            i_rad: value.i_rad,
            raan_rad: value.raan_rad,
            mean_latitude_argument_rad: value.mean_latitude_argument_rad,
        }
    }
}

/// Convert inertial Cartesian coordinates to circular elements.
///
/// Safety: out points to writable element storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_circular_elements_from_cartesian(
    state: SidereonAnalyticCartesian,
    mu_km3_s2: f64,
    out: *mut SidereonCircularElements,
) -> SidereonStatus {
    write_analytic("sidereon_circular_elements_from_cartesian", out, || {
        CircularElements::from_cartesian(state.position_km, state.velocity_km_s, mu_km3_s2)
            .map(Into::into)
    })
}

/// Convert circular elements to inertial Cartesian coordinates.
///
/// Safety: out points to writable coordinate storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_circular_elements_to_cartesian(
    elements: SidereonCircularElements,
    mu_km3_s2: f64,
    out: *mut SidereonAnalyticCartesian,
) -> SidereonStatus {
    write_analytic("sidereon_circular_elements_to_cartesian", out, || {
        let (position_km, velocity_km_s) =
            CircularElements::from(elements).to_cartesian(mu_km3_s2)?;
        Ok(SidereonAnalyticCartesian {
            position_km,
            velocity_km_s,
        })
    })
}

/// Analytic orbit elements. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SidereonAnalyticKeplerianElements {
    /// Semi-major axis in kilometres.
    pub a_km: f64,
    /// Eccentricity.
    pub e: f64,
    /// Inclination in radians.
    pub i_rad: f64,
    /// Argument of perigee in radians.
    pub arg_perigee_rad: f64,
    /// Right ascension of the ascending node in radians.
    pub raan_rad: f64,
    /// Mean anomaly in radians.
    pub mean_anomaly_rad: f64,
}

impl From<SidereonAnalyticKeplerianElements> for KeplerianElements {
    fn from(value: SidereonAnalyticKeplerianElements) -> Self {
        Self {
            a_km: value.a_km,
            e: value.e,
            i_rad: value.i_rad,
            arg_perigee_rad: value.arg_perigee_rad,
            raan_rad: value.raan_rad,
            mean_anomaly_rad: value.mean_anomaly_rad,
        }
    }
}

impl From<KeplerianElements> for SidereonAnalyticKeplerianElements {
    fn from(value: KeplerianElements) -> Self {
        Self {
            a_km: value.a_km,
            e: value.e,
            i_rad: value.i_rad,
            arg_perigee_rad: value.arg_perigee_rad,
            raan_rad: value.raan_rad,
            mean_anomaly_rad: value.mean_anomaly_rad,
        }
    }
}

/// Convert inertial Cartesian coordinates to analytic keplerian elements.
///
/// Safety: out points to writable element storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_analytic_keplerian_elements_from_cartesian(
    state: SidereonAnalyticCartesian,
    mu_km3_s2: f64,
    out: *mut SidereonAnalyticKeplerianElements,
) -> SidereonStatus {
    write_analytic(
        "sidereon_analytic_keplerian_elements_from_cartesian",
        out,
        || {
            KeplerianElements::from_cartesian(state.position_km, state.velocity_km_s, mu_km3_s2)
                .map(Into::into)
        },
    )
}

/// Convert analytic keplerian elements to inertial Cartesian coordinates.
///
/// Safety: out points to writable coordinate storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_analytic_keplerian_elements_to_cartesian(
    elements: SidereonAnalyticKeplerianElements,
    mu_km3_s2: f64,
    out: *mut SidereonAnalyticCartesian,
) -> SidereonStatus {
    write_analytic(
        "sidereon_analytic_keplerian_elements_to_cartesian",
        out,
        || {
            let (position_km, velocity_km_s) =
                KeplerianElements::from(elements).to_cartesian(mu_km3_s2)?;
            Ok(SidereonAnalyticCartesian {
                position_km,
                velocity_km_s,
            })
        },
    )
}

/// Eccentricity of circular elements.
#[no_mangle]
pub extern "C" fn sidereon_circular_elements_eccentricity(
    elements: SidereonCircularElements,
) -> f64 {
    CircularElements::from(elements).eccentricity()
}

/// Earth zonal gravity coefficients.
#[no_mangle]
pub extern "C" fn sidereon_zonal_harmonics_earth() -> SidereonZonalHarmonics {
    ZonalHarmonics::earth().into()
}

/// Default zonal gravity coefficients.
#[no_mangle]
pub extern "C" fn sidereon_zonal_harmonics_default() -> SidereonZonalHarmonics {
    ZonalHarmonics::default().into()
}

/// Mean conversion settings for Eckstein-Hechler.
#[no_mangle]
pub extern "C" fn sidereon_mean_conversion_options_eckstein_hechler_default(
) -> SidereonMeanConversionOptions {
    MeanConversionOptions::ECKSTEIN_HECHLER_DEFAULT.into()
}

/// Mean conversion settings for Brouwer-Lyddane.
#[no_mangle]
pub extern "C" fn sidereon_mean_conversion_options_brouwer_lyddane_default(
) -> SidereonMeanConversionOptions {
    MeanConversionOptions::BROUWER_LYDDANE_DEFAULT.into()
}

unsafe fn new_analytic<T>(
    operation: &str,
    out: *mut *mut T,
    build: impl FnOnce() -> Result<T, AnalyticPropagationError>,
) -> SidereonStatus {
    engine_error_operation_boundary(operation, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, operation, "out"));
        out.write(ptr::null_mut());
        let value = c_try!(build().map_err(|err| map_analytic_error(operation, err)));
        out.write(Box::into_raw(Box::new(value)));
        SidereonStatus::Ok
    })
}

unsafe fn analytic_query<T, U: Copy>(
    operation: &str,
    propagator: *const T,
    out: *mut U,
    evaluate: impl FnOnce(&T) -> Result<U, AnalyticPropagationError>,
) -> SidereonStatus {
    engine_error_operation_boundary(operation, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, operation, "out"));
        c_try!(reject_output_overlaps_handle(
            operation,
            out,
            "out",
            propagator,
            "propagator"
        ));
        let propagator = c_try!(require_ref(propagator, operation, "propagator"));
        let value = c_try!(evaluate(propagator).map_err(|err| map_analytic_error(operation, err)));
        out.write(value);
        SidereonStatus::Ok
    })
}

/// Owned EcksteinHechler propagator. Release with sidereon_eckstein_hechler_propagator_free.
pub struct SidereonEcksteinHechlerPropagator {
    pub(crate) inner: EcksteinHechlerPropagator,
}

/// Mean element conversion result. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SidereonEcksteinHechlerMeanElements {
    /// Converged mean elements.
    pub elements: SidereonCircularElements,
    /// Number of iterations.
    pub iterations: u32,
}

/// Convert an osculating Cartesian position and velocity to mean elements.
///
/// Safety: out points to writable mean element storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_eckstein_hechler_mean_elements(
    state: SidereonAnalyticCartesian,
    gravity: SidereonZonalHarmonics,
    options: SidereonMeanConversionOptions,
    out: *mut SidereonEcksteinHechlerMeanElements,
) -> SidereonStatus {
    write_analytic("sidereon_eckstein_hechler_mean_elements", out, || {
        let state = CartesianState::new(0.0, state.position_km, state.velocity_km_s);
        let value =
            analytic::eckstein_hechler_mean_elements(&state, &gravity.into(), options.into())?;
        Ok(SidereonEcksteinHechlerMeanElements {
            elements: value.elements.into(),
            iterations: value.iterations,
        })
    })
}

/// Convert mean elements to osculating elements at the same epoch.
///
/// Safety: out points to writable element storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_eckstein_hechler_osculating_elements(
    mean: SidereonCircularElements,
    gravity: SidereonZonalHarmonics,
    out: *mut SidereonCircularElements,
) -> SidereonStatus {
    write_analytic("sidereon_eckstein_hechler_osculating_elements", out, || {
        analytic::eckstein_hechler_osculating_elements(&mean.into(), &gravity.into())
            .map(Into::into)
    })
}

/// Create an analytic propagator from an osculating state.
///
/// Safety: out points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_eckstein_hechler_propagator_new(
    initial: SidereonCartesianState,
    gravity: SidereonZonalHarmonics,
    options: SidereonMeanConversionOptions,
    out: *mut *mut SidereonEcksteinHechlerPropagator,
) -> SidereonStatus {
    new_analytic("sidereon_eckstein_hechler_propagator_new", out, || {
        let inner = EcksteinHechlerPropagator::new(
            &cartesian_state_from_c(&initial),
            &gravity.into(),
            options.into(),
        )?;
        Ok(SidereonEcksteinHechlerPropagator { inner })
    })
}

/// Create an analytic propagator from mean elements.
///
/// Safety: out points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_eckstein_hechler_propagator_from_mean_elements(
    epoch_tdb_seconds: f64,
    mean: SidereonCircularElements,
    gravity: SidereonZonalHarmonics,
    out: *mut *mut SidereonEcksteinHechlerPropagator,
) -> SidereonStatus {
    new_analytic(
        "sidereon_eckstein_hechler_propagator_from_mean_elements",
        out,
        || {
            let inner = EcksteinHechlerPropagator::from_mean_elements(
                epoch_tdb_seconds,
                mean.into(),
                &gravity.into(),
            )?;
            Ok(SidereonEcksteinHechlerPropagator { inner })
        },
    )
}

/// Release an analytic propagator.
///
/// Safety: propagator is NULL or a live owned propagator.
#[no_mangle]
pub unsafe extern "C" fn sidereon_eckstein_hechler_propagator_free(
    propagator: *mut SidereonEcksteinHechlerPropagator,
) {
    ffi_boundary("sidereon_eckstein_hechler_propagator_free", (), || {
        free_boxed(propagator)
    });
}

/// Read the initial TDB epoch in seconds past J2000.
///
/// Safety: propagator is live and out points to writable disjoint storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_eckstein_hechler_propagator_epoch_tdb_seconds(
    propagator: *const SidereonEcksteinHechlerPropagator,
    out: *mut f64,
) -> SidereonStatus {
    analytic_query(
        "sidereon_eckstein_hechler_propagator_epoch_tdb_seconds",
        propagator,
        out,
        |propagator| Ok(propagator.inner.epoch_tdb_seconds()),
    )
}

/// Read the initial mean elements.
///
/// Safety: propagator is live and out points to writable disjoint storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_eckstein_hechler_propagator_mean_elements(
    propagator: *const SidereonEcksteinHechlerPropagator,
    out: *mut SidereonCircularElements,
) -> SidereonStatus {
    analytic_query(
        "sidereon_eckstein_hechler_propagator_mean_elements",
        propagator,
        out,
        |propagator| Ok(propagator.inner.mean_elements().into()),
    )
}

/// Read the zonal gravity coefficients.
///
/// Safety: propagator is live and out points to writable disjoint storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_eckstein_hechler_propagator_gravity(
    propagator: *const SidereonEcksteinHechlerPropagator,
    out: *mut SidereonZonalHarmonics,
) -> SidereonStatus {
    analytic_query(
        "sidereon_eckstein_hechler_propagator_gravity",
        propagator,
        out,
        |propagator| Ok((*propagator.inner.gravity()).into()),
    )
}

/// Evaluate osculating elements at the TDB epoch.
///
/// Safety: propagator is live and out points to writable disjoint storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_eckstein_hechler_propagator_osculating_elements(
    propagator: *const SidereonEcksteinHechlerPropagator,
    epoch_tdb_seconds: f64,
    out: *mut SidereonCircularElements,
) -> SidereonStatus {
    analytic_query(
        "sidereon_eckstein_hechler_propagator_osculating_elements",
        propagator,
        out,
        |propagator| {
            propagator
                .inner
                .osculating_elements(epoch_tdb_seconds)
                .map(Into::into)
        },
    )
}

/// Evaluate an inertial state at the TDB epoch.
///
/// Safety: propagator is live and out points to writable disjoint storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_eckstein_hechler_propagator_propagate(
    propagator: *const SidereonEcksteinHechlerPropagator,
    epoch_tdb_seconds: f64,
    out: *mut SidereonCartesianState,
) -> SidereonStatus {
    analytic_query(
        "sidereon_eckstein_hechler_propagator_propagate",
        propagator,
        out,
        |propagator| {
            propagator
                .inner
                .propagate(epoch_tdb_seconds)
                .map(|value| cartesian_state_to_c(&value))
        },
    )
}

/// Evaluate inertial states in the order of the TDB epoch array.
/// NULL and zero capacity query the full result count.
///
/// Safety: propagator is live and epochs_tdb_seconds has epoch_count entries.
/// All outputs are writable and disjoint from each other, the input and the handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_eckstein_hechler_propagator_ephemeris(
    propagator: *const SidereonEcksteinHechlerPropagator,
    epochs_tdb_seconds: *const f64,
    epoch_count: usize,
    out_states: *mut SidereonCartesianState,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_eckstein_hechler_propagator_ephemeris";
    engine_error_operation_boundary(OP, SidereonStatus::Panic, || {
        c_try!(init_handle_copy_counts(
            OP,
            out_states,
            capacity,
            out_written,
            out_required,
            propagator,
            "propagator"
        ));
        let propagator = c_try!(require_ref(propagator, OP, "propagator"));
        let epochs = c_try!(require_slice(
            epochs_tdb_seconds,
            epoch_count,
            OP,
            "epochs_tdb_seconds"
        ));
        let states = c_try!(propagator
            .inner
            .ephemeris(epochs)
            .map_err(|err| map_analytic_error(OP, err)));
        let states: Vec<_> = states.iter().map(cartesian_state_to_c).collect();
        c_try!(copy_prefix_to_c(
            OP,
            "out_states",
            &states,
            out_states,
            capacity,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Owned BrouwerLyddane propagator. Release with sidereon_brouwer_lyddane_propagator_free.
pub struct SidereonBrouwerLyddanePropagator {
    pub(crate) inner: BrouwerLyddanePropagator,
}

/// Mean element conversion result. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SidereonBrouwerLyddaneMeanElements {
    /// Converged mean elements.
    pub elements: SidereonAnalyticKeplerianElements,
    /// Number of iterations.
    pub iterations: u32,
}

/// Convert an osculating Cartesian position and velocity to mean elements.
///
/// Safety: out points to writable mean element storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_brouwer_lyddane_mean_elements(
    state: SidereonAnalyticCartesian,
    gravity: SidereonZonalHarmonics,
    options: SidereonMeanConversionOptions,
    out: *mut SidereonBrouwerLyddaneMeanElements,
) -> SidereonStatus {
    write_analytic("sidereon_brouwer_lyddane_mean_elements", out, || {
        let state = CartesianState::new(0.0, state.position_km, state.velocity_km_s);
        let value =
            analytic::brouwer_lyddane_mean_elements(&state, &gravity.into(), options.into())?;
        Ok(SidereonBrouwerLyddaneMeanElements {
            elements: value.elements.into(),
            iterations: value.iterations,
        })
    })
}

/// Convert mean elements to osculating elements at the same epoch.
///
/// Safety: out points to writable element storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_brouwer_lyddane_osculating_elements(
    mean: SidereonAnalyticKeplerianElements,
    gravity: SidereonZonalHarmonics,
    out: *mut SidereonAnalyticKeplerianElements,
) -> SidereonStatus {
    write_analytic("sidereon_brouwer_lyddane_osculating_elements", out, || {
        analytic::brouwer_lyddane_osculating_elements(&mean.into(), &gravity.into()).map(Into::into)
    })
}

/// Create an analytic propagator from an osculating state.
///
/// Safety: out points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_brouwer_lyddane_propagator_new(
    initial: SidereonCartesianState,
    gravity: SidereonZonalHarmonics,
    m2_rad_s2: f64,
    options: SidereonMeanConversionOptions,
    out: *mut *mut SidereonBrouwerLyddanePropagator,
) -> SidereonStatus {
    new_analytic("sidereon_brouwer_lyddane_propagator_new", out, || {
        let inner = BrouwerLyddanePropagator::new(
            &cartesian_state_from_c(&initial),
            &gravity.into(),
            m2_rad_s2,
            options.into(),
        )?;
        Ok(SidereonBrouwerLyddanePropagator { inner })
    })
}

/// Create an analytic propagator from mean elements.
///
/// Safety: out points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_brouwer_lyddane_propagator_from_mean_elements(
    epoch_tdb_seconds: f64,
    mean: SidereonAnalyticKeplerianElements,
    gravity: SidereonZonalHarmonics,
    m2_rad_s2: f64,
    out: *mut *mut SidereonBrouwerLyddanePropagator,
) -> SidereonStatus {
    new_analytic(
        "sidereon_brouwer_lyddane_propagator_from_mean_elements",
        out,
        || {
            let inner = BrouwerLyddanePropagator::from_mean_elements(
                epoch_tdb_seconds,
                mean.into(),
                &gravity.into(),
                m2_rad_s2,
            )?;
            Ok(SidereonBrouwerLyddanePropagator { inner })
        },
    )
}

/// Release an analytic propagator.
///
/// Safety: propagator is NULL or a live owned propagator.
#[no_mangle]
pub unsafe extern "C" fn sidereon_brouwer_lyddane_propagator_free(
    propagator: *mut SidereonBrouwerLyddanePropagator,
) {
    ffi_boundary("sidereon_brouwer_lyddane_propagator_free", (), || {
        free_boxed(propagator)
    });
}

/// Read the initial TDB epoch in seconds past J2000.
///
/// Safety: propagator is live and out points to writable disjoint storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_brouwer_lyddane_propagator_epoch_tdb_seconds(
    propagator: *const SidereonBrouwerLyddanePropagator,
    out: *mut f64,
) -> SidereonStatus {
    analytic_query(
        "sidereon_brouwer_lyddane_propagator_epoch_tdb_seconds",
        propagator,
        out,
        |propagator| Ok(propagator.inner.epoch_tdb_seconds()),
    )
}

/// Read the initial mean elements.
///
/// Safety: propagator is live and out points to writable disjoint storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_brouwer_lyddane_propagator_mean_elements(
    propagator: *const SidereonBrouwerLyddanePropagator,
    out: *mut SidereonAnalyticKeplerianElements,
) -> SidereonStatus {
    analytic_query(
        "sidereon_brouwer_lyddane_propagator_mean_elements",
        propagator,
        out,
        |propagator| Ok(propagator.inner.mean_elements().into()),
    )
}

/// Read the zonal gravity coefficients.
///
/// Safety: propagator is live and out points to writable disjoint storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_brouwer_lyddane_propagator_gravity(
    propagator: *const SidereonBrouwerLyddanePropagator,
    out: *mut SidereonZonalHarmonics,
) -> SidereonStatus {
    analytic_query(
        "sidereon_brouwer_lyddane_propagator_gravity",
        propagator,
        out,
        |propagator| Ok((*propagator.inner.gravity()).into()),
    )
}

/// Evaluate osculating elements at the TDB epoch.
///
/// Safety: propagator is live and out points to writable disjoint storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_brouwer_lyddane_propagator_osculating_elements(
    propagator: *const SidereonBrouwerLyddanePropagator,
    epoch_tdb_seconds: f64,
    out: *mut SidereonAnalyticKeplerianElements,
) -> SidereonStatus {
    analytic_query(
        "sidereon_brouwer_lyddane_propagator_osculating_elements",
        propagator,
        out,
        |propagator| {
            propagator
                .inner
                .osculating_elements(epoch_tdb_seconds)
                .map(Into::into)
        },
    )
}

/// Evaluate an inertial state at the TDB epoch.
///
/// Safety: propagator is live and out points to writable disjoint storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_brouwer_lyddane_propagator_propagate(
    propagator: *const SidereonBrouwerLyddanePropagator,
    epoch_tdb_seconds: f64,
    out: *mut SidereonCartesianState,
) -> SidereonStatus {
    analytic_query(
        "sidereon_brouwer_lyddane_propagator_propagate",
        propagator,
        out,
        |propagator| {
            propagator
                .inner
                .propagate(epoch_tdb_seconds)
                .map(|value| cartesian_state_to_c(&value))
        },
    )
}

/// Read the drag coefficient M2 in radians per second squared.
///
/// Safety: propagator is live and out points to writable disjoint storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_brouwer_lyddane_propagator_m2_rad_s2(
    propagator: *const SidereonBrouwerLyddanePropagator,
    out: *mut f64,
) -> SidereonStatus {
    analytic_query(
        "sidereon_brouwer_lyddane_propagator_m2_rad_s2",
        propagator,
        out,
        |propagator| Ok(propagator.inner.m2_rad_s2()),
    )
}

/// Evaluate inertial states in the order of the TDB epoch array.
/// NULL and zero capacity query the full result count.
///
/// Safety: propagator is live and epochs_tdb_seconds has epoch_count entries.
/// All outputs are writable and disjoint from each other, the input and the handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_brouwer_lyddane_propagator_ephemeris(
    propagator: *const SidereonBrouwerLyddanePropagator,
    epochs_tdb_seconds: *const f64,
    epoch_count: usize,
    out_states: *mut SidereonCartesianState,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_brouwer_lyddane_propagator_ephemeris";
    engine_error_operation_boundary(OP, SidereonStatus::Panic, || {
        c_try!(init_handle_copy_counts(
            OP,
            out_states,
            capacity,
            out_written,
            out_required,
            propagator,
            "propagator"
        ));
        let propagator = c_try!(require_ref(propagator, OP, "propagator"));
        let epochs = c_try!(require_slice(
            epochs_tdb_seconds,
            epoch_count,
            OP,
            "epochs_tdb_seconds"
        ));
        let states = c_try!(propagator
            .inner
            .ephemeris(epochs)
            .map_err(|err| map_analytic_error(OP, err)));
        let states: Vec<_> = states.iter().map(cartesian_state_to_c).collect();
        c_try!(copy_prefix_to_c(
            OP,
            "out_states",
            &states,
            out_states,
            capacity,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

#[cfg(test)]
mod error_tests {
    use super::*;

    #[test]
    fn analytic_propagation_error_details() {
        use AnalyticPropagationError as E;
        let cases = [
            (
                E::NonFinite { field: "sample" },
                "non_finite",
                json!({"field":"sample"}),
            ),
            (
                E::InvalidParameter {
                    field: "sample",
                    reason: "sample",
                },
                "invalid_parameter",
                json!({"field":"sample", "reason":"sample"}),
            ),
            (
                E::NotElliptic { field: "sample" },
                "not_elliptic",
                json!({"field":"sample"}),
            ),
            (
                E::RetrogradeEquatorial {
                    inclination_rad: 0.5,
                },
                "retrograde_equatorial",
                json!({"inclination_rad":{"decimal":"0.5","bits_hex":"3fe0000000000000"}}),
            ),
            (
                E::InsideReferenceSphere {
                    semi_major_axis_km: 0.5,
                    reference_radius_km: 0.5,
                },
                "inside_reference_sphere",
                json!({"semi_major_axis_km":{"decimal":"0.5","bits_hex":"3fe0000000000000"}, "reference_radius_km":{"decimal":"0.5","bits_hex":"3fe0000000000000"}}),
            ),
            (
                E::EccentricityTooLarge {
                    eccentricity: 0.5,
                    limit: 0.5,
                },
                "eccentricity_too_large",
                json!({"eccentricity":{"decimal":"0.5","bits_hex":"3fe0000000000000"}, "limit":{"decimal":"0.5","bits_hex":"3fe0000000000000"}}),
            ),
            (
                E::NearlyEquatorial {
                    inclination_rad: 0.5,
                },
                "nearly_equatorial",
                json!({"inclination_rad":{"decimal":"0.5","bits_hex":"3fe0000000000000"}}),
            ),
            (
                E::NearCriticalInclination {
                    inclination_rad: 0.5,
                },
                "near_critical_inclination",
                json!({"inclination_rad":{"decimal":"0.5","bits_hex":"3fe0000000000000"}}),
            ),
            (
                E::MeanElementsNotConverged {
                    theory: AnalyticTheory::BrouwerLyddane,
                    iterations: 3,
                },
                "mean_elements_not_converged",
                json!({"theory":"Brouwer-Lyddane", "iterations":3}),
            ),
            (
                E::KeplerNotConverged { field: "sample" },
                "kepler_not_converged",
                json!({"field":"sample"}),
            ),
            (
                E::NonFiniteResult {
                    theory: AnalyticTheory::BrouwerLyddane,
                    field: "sample",
                },
                "non_finite_result",
                json!({"theory":"Brouwer-Lyddane", "field":"sample"}),
            ),
        ];
        for (error, kind, fields) in cases {
            assert_eq!(
                analytic_error_value(error),
                json!({"kind":kind,"fields":fields})
            );
        }
    }
}
