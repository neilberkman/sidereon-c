use super::*;
use crate::covariance::map_propagation_error;
use crate::engine_error::engine_error_operation_boundary;
use nalgebra::{Matrix3, Vector3};
use sidereon_core::astro::events::eclipse::ShadowFunction;
use sidereon_core::astro::forces::{
    AccelerationPartials, CompositeForceModel, EarthRadiationPressure, Ecom1Coefficients,
    Ecom1SolarRadiationPressure, Ecom2D2b1Coefficients, Ecom2D4b1Coefficients,
    Ecom2SolarRadiationPressure, EmpiricalHarmonic, EmpiricalRtnAcceleration, RtnAxis,
    SourcedDragForce, SpaceWeatherSource,
};

pub(crate) trait CForceModel: ForceModel {
    fn as_any(&self) -> &dyn std::any::Any;
}

impl<T: ForceModel + 'static> CForceModel for T {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// An immutable force model. Release with sidereon_force_model_free.
/// A composite retains each component independently of the input handles.
pub struct SidereonForceModel {
    pub(crate) inner: Arc<dyn CForceModel>,
}

/// A Cartesian vector. Compare components by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SidereonVector3 {
    /// Vector components in x, y, z order.
    pub xyz: [f64; 3],
}

impl From<Vector3<f64>> for SidereonVector3 {
    fn from(value: Vector3<f64>) -> Self {
        Self { xyz: value.into() }
    }
}

/// Acceleration derivatives in row-major order. Compare elements by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SidereonAccelerationPartials {
    /// Derivatives with respect to position.
    pub wrt_position: [f64; 9],
    /// Derivatives with respect to velocity.
    pub wrt_velocity: [f64; 9],
}

impl From<AccelerationPartials> for SidereonAccelerationPartials {
    fn from(value: AccelerationPartials) -> Self {
        Self {
            wrt_position: std::array::from_fn(|i| value.wrt_position[(i / 3, i % 3)]),
            wrt_velocity: std::array::from_fn(|i| value.wrt_velocity[(i / 3, i % 3)]),
        }
    }
}

/// Acceleration derivatives with both matrices set to zero.
#[no_mangle]
pub extern "C" fn sidereon_acceleration_partials_zeros() -> SidereonAccelerationPartials {
    AccelerationPartials::zeros().into()
}

/// Construct derivatives from a row-major position matrix and a zero velocity matrix.
///
/// Safety: wrt_position points to nine doubles and out points to writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_acceleration_partials_position_only(
    wrt_position: *const f64,
    out: *mut SidereonAccelerationPartials,
) -> SidereonStatus {
    const OP: &str = "sidereon_acceleration_partials_position_only";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        let position = c_try!(require_f64_array::<9>(wrt_position, OP, "wrt_position"));
        let partials = AccelerationPartials::position_only(Matrix3::from_row_slice(position));
        out.write(partials.into());
        SidereonStatus::Ok
    })
}

/// Propagation inputs shared by force models. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPropagationContext {
    /// Optional JPL DE ephemeris. NULL selects analytic Sun and Moon positions.
    pub de_ephemeris: *const SidereonDeEphemeris,
    /// Optional IERS Earth orientation provider. NULL uses each operation's default.
    pub earth_orientation: *const SidereonIers2010EarthOrientationProvider,
}

pub(crate) unsafe fn force_context_from_c(
    context: *const SidereonPropagationContext,
    default: PropagationContext,
) -> PropagationContext {
    let Some(context) = context.as_ref() else {
        return default;
    };
    let mut result = default;
    if let Some(ephemeris) = context.de_ephemeris.as_ref() {
        result = result.with_de_ephemeris(Arc::clone(&ephemeris.inner));
    }
    if let Some(provider) = context.earth_orientation.as_ref() {
        result = result.with_body_fixed_frame_provider(provider.inner.clone());
    }
    result
}

/// Solar radiation pressure lighting configuration. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SidereonRadiationShadowOptions {
    /// Lighting function as SidereonShadowFunction.
    pub shadow_function: u32,
    /// Earth figure as SidereonEarthShadowModel.
    pub earth_shadow_model: u32,
    /// Whether the Moon is included under apparent-disc occultation.
    pub moon_eclipse_enabled: bool,
}

/// Default cannonball solar radiation pressure lighting configuration.
#[no_mangle]
pub extern "C" fn sidereon_radiation_shadow_options_default() -> SidereonRadiationShadowOptions {
    let value = SolarRadiationPressure::new(1.0, 1.0).expect("positive reference inputs");
    SidereonRadiationShadowOptions {
        shadow_function: match value.shadow_function() {
            ShadowFunction::ApparentDisc => 0,
            ShadowFunction::Conical => 1,
            _ => u32::MAX,
        },
        earth_shadow_model: match value.earth_shadow_model() {
            sidereon_core::astro::events::eclipse::EarthShadowModel::Spherical => 0,
            sidereon_core::astro::events::eclipse::EarthShadowModel::Wgs84Oblate => 1,
        },
        moon_eclipse_enabled: value.moon_eclipse_enabled(),
    }
}

fn shadow_function_from_c(operation: &str, code: u32) -> Result<ShadowFunction, SidereonStatus> {
    match code {
        0 => Ok(ShadowFunction::ApparentDisc),
        1 => Ok(ShadowFunction::Conical),
        _ => Err(extra_invalid_arg(
            operation,
            format_args!("invalid shadow function {code}"),
        )),
    }
}

macro_rules! configure_shadow {
    ($operation:expr, $model:expr, $options:expr) => {{
        let options = $options;
        let shadow_function = shadow_function_from_c($operation, options.shadow_function)?;
        let earth_model =
            crate::almanac::earth_shadow_model_from_c($operation, options.earth_shadow_model)?;
        Ok($model
            .with_shadow_function(shadow_function)
            .with_earth_shadow_model(earth_model)
            .with_moon_eclipse(options.moon_eclipse_enabled))
    }};
}

/// Earth albedo and infrared pressure options for a built-in force set.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SidereonEarthRadiationPressureOptions {
    /// Radiation pressure coefficient.
    pub cr: f64,
    /// Area to mass ratio in m²/kg.
    pub area_to_mass_m2_kg: f64,
    /// Whether to include lunar occultation.
    pub moon_eclipse_enabled: bool,
}

pub(crate) unsafe fn configure_radiation(
    operation: &str,
    model: &mut ForceModelKind,
    shadow: *const SidereonRadiationShadowOptions,
    earth: *const SidereonEarthRadiationPressureOptions,
) -> Result<(), SidereonStatus> {
    let shadow = shadow.as_ref();
    let earth = earth.as_ref();
    let ForceModelKind::Composite { components } = model else {
        if shadow.is_some() || earth.is_some() {
            return Err(extra_invalid_arg(
                operation,
                "radiation options require an additive force set",
            ));
        }
        return Ok(());
    };
    if let Some(shadow) = shadow {
        let srp = components.solar_radiation_pressure.ok_or_else(|| {
            extra_invalid_arg(
                operation,
                "solar shadow options require solar radiation pressure",
            )
        })?;
        components.solar_radiation_pressure = Some(configure_shadow!(operation, srp, *shadow)?);
    }
    if let Some(earth) = earth {
        let pressure = EarthRadiationPressure::new(earth.cr, earth.area_to_mass_m2_kg)
            .map_err(|err| map_propagation_error(operation, err))?;
        components.earth_radiation_pressure =
            Some(pressure.with_moon_eclipse(earth.moon_eclipse_enabled));
    }
    Ok(())
}

pub(crate) unsafe fn write_force_model(
    operation: &str,
    out: *mut *mut SidereonForceModel,
    build: impl FnOnce() -> Result<Arc<dyn CForceModel>, SidereonStatus>,
) -> SidereonStatus {
    write_guarded_force_model(operation, out, || Ok(()), build)
}

pub(crate) unsafe fn write_guarded_force_model(
    operation: &str,
    out: *mut *mut SidereonForceModel,
    guard: impl FnOnce() -> Result<(), SidereonStatus>,
    build: impl FnOnce() -> Result<Arc<dyn CForceModel>, SidereonStatus>,
) -> SidereonStatus {
    engine_error_operation_boundary(operation, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, operation, "out"));
        c_try!(guard());
        out.write(ptr::null_mut());
        let inner = c_try!(build());
        out.write(Box::into_raw(Box::new(SidereonForceModel { inner })));
        SidereonStatus::Ok
    })
}

/// Release an owned force model.
///
/// Safety: model is NULL or a live owned force model handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_force_model_free(model: *mut SidereonForceModel) {
    ffi_boundary("sidereon_force_model_free", (), || free_boxed(model));
}

/// Construct a composite force model from its components in evaluation order.
///
/// Safety: models points to count live handles. out is disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_composite_force_model_new(
    models: *const *const SidereonForceModel,
    count: usize,
    out: *mut *mut SidereonForceModel,
) -> SidereonStatus {
    const OP: &str = "sidereon_composite_force_model_new";
    write_guarded_force_model(
        OP,
        out,
        || reject_output_overlaps_forces(OP, out, "out", models, count),
        || {
            let mut composite = CompositeForceModel::new();
            for &model in require_slice(models, count, OP, "models")? {
                let model = require_ref(model, OP, "model")?;
                composite.add(Box::new(Arc::clone(&model.inner)));
            }
            Ok(Arc::new(composite))
        },
    )
}

/// Create a cannonball solar radiation pressure force model.
///
/// Safety: out points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solar_radiation_pressure_new(
    cr: f64,
    area_to_mass_m2_kg: f64,
    shadow: SidereonRadiationShadowOptions,
    out: *mut *mut SidereonForceModel,
) -> SidereonStatus {
    const OP: &str = "sidereon_solar_radiation_pressure_new";
    write_force_model(OP, out, || {
        let value = SolarRadiationPressure::new(cr, area_to_mass_m2_kg)
            .map_err(|err| map_propagation_error(OP, err))?;
        Ok(Arc::new(configure_shadow!(OP, value, shadow)?))
    })
}

/// Create an Earth albedo and infrared radiation pressure force model.
///
/// Safety: out points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_earth_radiation_pressure_new(
    cr: f64,
    area_to_mass_m2_kg: f64,
    moon_eclipse_enabled: bool,
    out: *mut *mut SidereonForceModel,
) -> SidereonStatus {
    const OP: &str = "sidereon_earth_radiation_pressure_new";
    write_force_model(OP, out, || {
        let value = EarthRadiationPressure::new(cr, area_to_mass_m2_kg)
            .map_err(|err| map_propagation_error(OP, err))?;
        Ok(Arc::new(value.with_moon_eclipse(moon_eclipse_enabled)))
    })
}

/// Create an atmospheric drag force with fixed space weather.
///
/// Safety: out points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_drag_force_new(
    parameters: SidereonDragParameters,
    out: *mut *mut SidereonForceModel,
) -> SidereonStatus {
    const OP: &str = "sidereon_drag_force_new";
    write_force_model(OP, out, || {
        let parameters = drag_parameters_from_c(OP, parameters)?;
        Ok(Arc::new(parameters.to_force()))
    })
}

/// Create a central point-mass gravity model.
///
/// Safety: out points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_two_body_gravity_new(
    mu_km3_s2: f64,
    out: *mut *mut SidereonForceModel,
) -> SidereonStatus {
    write_force_model("sidereon_two_body_gravity_new", out, || {
        Ok(Arc::new(TwoBodyGravity { mu: mu_km3_s2 }))
    })
}

/// Create a degree-two zonal gravity perturbation.
///
/// Safety: out points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_j2_gravity_new(
    mu_km3_s2: f64,
    reference_radius_km: f64,
    j2: f64,
    out: *mut *mut SidereonForceModel,
) -> SidereonStatus {
    write_force_model("sidereon_j2_gravity_new", out, || {
        Ok(Arc::new(J2Gravity {
            mu: mu_km3_s2,
            re: reference_radius_km,
            j2,
        }))
    })
}

/// Create a solar and lunar third-body gravity model.
///
/// Safety: out points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_third_body_gravity_new(
    sun: bool,
    moon: bool,
    out: *mut *mut SidereonForceModel,
) -> SidereonStatus {
    write_force_model("sidereon_third_body_gravity_new", out, || {
        Ok(Arc::new(ThirdBodyGravity {
            bodies: ThirdBodyBodies { sun, moon },
            ..ThirdBodyGravity::default()
        }))
    })
}

/// Create an atmospheric drag force with per-epoch space weather.
///
/// Safety: table is live and out points to disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sourced_drag_force_new(
    bc_factor_m2_kg: f64,
    cutoff_altitude_km: f64,
    table: *const SidereonSpaceWeatherTable,
    out: *mut *mut SidereonForceModel,
) -> SidereonStatus {
    const OP: &str = "sidereon_sourced_drag_force_new";
    write_guarded_force_model(
        OP,
        out,
        || reject_output_overlaps_handle(OP, out, "out", table, "table"),
        || {
            let table = require_ref(table, OP, "table")?;
            let parameters = DragParameters::from_bc_factor_m2_kg(
                bc_factor_m2_kg,
                SpaceWeather::default(),
                cutoff_altitude_km,
            )
            .map_err(|err| map_propagation_error(OP, err))?;
            Ok(Arc::new(SourcedDragForce::new(
                parameters,
                SpaceWeatherSource::Table(table.inner.clone()),
            )))
        },
    )
}

unsafe fn force_query<T: Copy>(
    operation: &str,
    model: *const SidereonForceModel,
    out: *mut T,
    read: impl FnOnce(&dyn CForceModel) -> Result<T, SidereonStatus>,
) -> SidereonStatus {
    engine_error_operation_boundary(operation, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, operation, "out"));
        c_try!(reject_output_overlaps_handle(
            operation, out, "out", model, "model"
        ));
        let model = c_try!(require_ref(model, operation, "model"));
        out.write(c_try!(read(model.inner.as_ref())));
        SidereonStatus::Ok
    })
}

/// Number of estimable force parameters.
///
/// Safety: model is live and out points to disjoint writable count storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_force_model_parameter_count(
    model: *const SidereonForceModel,
    out: *mut usize,
) -> SidereonStatus {
    force_query(
        "sidereon_force_model_parameter_count",
        model,
        out,
        |model| Ok(model.parameter_count()),
    )
}

/// Read one estimable parameter in model parameter order. The UTF-8 name has
/// no terminator. NULL and zero capacity query the full byte count.
///
/// Safety: model is live. All outputs are writable and disjoint from each
/// other and the handle. out_name has capacity bytes.
#[no_mangle]
pub unsafe extern "C" fn sidereon_force_model_parameter(
    model: *const SidereonForceModel,
    index: usize,
    out_name: *mut u8,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
    out_value: *mut f64,
) -> SidereonStatus {
    const OP: &str = "sidereon_force_model_parameter";
    engine_error_operation_boundary(OP, SidereonStatus::Panic, || {
        c_try!(check_named_value_outputs(
            OP,
            out_name,
            0,
            out_written,
            out_required,
            out_value
        ));
        c_try!(init_handle_copy_counts(
            OP,
            out_name,
            capacity,
            out_written,
            out_required,
            model,
            "model"
        ));
        let out_value = c_try!(require_out(out_value, OP, "out_value"));
        c_try!(reject_output_overlaps_handle(
            OP,
            out_value,
            "out_value",
            model,
            "model"
        ));
        let model = c_try!(require_ref(model, OP, "model"));
        let parameters = model.inner.parameters();
        let Some(parameter) = parameters.get(index) else {
            return extra_invalid_arg(OP, format_args!("parameter index {index} out of range"));
        };
        if capacity >= parameter.name.len() {
            c_try!(check_named_value_outputs(
                OP,
                out_name,
                parameter.name.len(),
                out_written,
                out_required,
                out_value
            ));
        }
        c_try!(copy_prefix_to_c(
            OP,
            "out_name",
            parameter.name.as_bytes(),
            out_name,
            capacity,
            out_written,
            out_required
        ));
        out_value.write(parameter.value);
        SidereonStatus::Ok
    })
}

/// Evaluate acceleration in km/s² with the configured force parameters.
///
/// Safety: model is live. context is NULL or readable with live handles.
/// out is writable and disjoint from inputs.
#[no_mangle]
pub unsafe extern "C" fn sidereon_force_model_acceleration(
    model: *const SidereonForceModel,
    state: SidereonCartesianState,
    context: *const SidereonPropagationContext,
    out: *mut SidereonVector3,
) -> SidereonStatus {
    const OP: &str = "sidereon_force_model_acceleration";
    force_query(OP, model, out, |model| {
        let context = force_context_from_c(context, PropagationContext::default());
        model
            .acceleration(&cartesian_state_from_c(&state), &context)
            .map(Into::into)
            .map_err(|err| map_propagation_error(OP, err))
    })
}

/// Evaluate acceleration derivatives with respect to each force parameter.
/// NULL and zero capacity query the number of parameter vectors.
///
/// Safety: model is live and parameters has parameter_count entries.
/// context is NULL or readable with live handles. All outputs are writable
/// and disjoint from each other and inputs. out_partials has capacity entries.
#[no_mangle]
pub unsafe extern "C" fn sidereon_force_model_parameter_partials(
    model: *const SidereonForceModel,
    state: SidereonCartesianState,
    parameters: *const f64,
    parameter_count: usize,
    context: *const SidereonPropagationContext,
    out_partials: *mut SidereonVector3,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_force_model_parameter_partials";
    engine_error_operation_boundary(OP, SidereonStatus::Panic, || {
        c_try!(init_handle_copy_counts(
            OP,
            out_partials,
            capacity,
            out_written,
            out_required,
            model,
            "model"
        ));
        let model = c_try!(require_ref(model, OP, "model"));
        let parameters = c_try!(require_slice(parameters, parameter_count, OP, "parameters"));
        let context = force_context_from_c(context, PropagationContext::default());
        let value = c_try!(model
            .inner
            .parameter_partials(parameters, &cartesian_state_from_c(&state), &context,)
            .map_err(|err| map_propagation_error(OP, err)));
        let values: Vec<_> = value.into_iter().map(SidereonVector3::from).collect();
        c_try!(copy_prefix_to_c(
            OP,
            "out_partials",
            &values,
            out_partials,
            capacity,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Evaluate acceleration in km/s² with replacement parameter values.
///
/// Safety: model is live and parameters has parameter_count entries.
/// context is NULL or readable with live handles. out is writable and disjoint from inputs.
#[no_mangle]
pub unsafe extern "C" fn sidereon_force_model_acceleration_with_parameters(
    model: *const SidereonForceModel,
    state: SidereonCartesianState,
    parameters: *const f64,
    parameter_count: usize,
    context: *const SidereonPropagationContext,
    out: *mut SidereonVector3,
) -> SidereonStatus {
    const OP: &str = "sidereon_force_model_acceleration_with_parameters";
    force_query(OP, model, out, |model| {
        let parameters = require_slice(parameters, parameter_count, OP, "parameters")?;
        let context = force_context_from_c(context, PropagationContext::default());
        model
            .acceleration_with_parameters(parameters, &cartesian_state_from_c(&state), &context)
            .map(Into::into)
            .map_err(|err| map_propagation_error(OP, err))
    })
}

/// Evaluate the position and velocity derivatives with replacement parameter values.
///
/// Safety: model is live and parameters has parameter_count entries.
/// context is NULL or readable with live handles. out is writable and disjoint from inputs.
#[no_mangle]
pub unsafe extern "C" fn sidereon_force_model_acceleration_partials(
    model: *const SidereonForceModel,
    state: SidereonCartesianState,
    parameters: *const f64,
    parameter_count: usize,
    context: *const SidereonPropagationContext,
    out: *mut SidereonAccelerationPartials,
) -> SidereonStatus {
    const OP: &str = "sidereon_force_model_acceleration_partials";
    force_query(OP, model, out, |model| {
        let parameters = require_slice(parameters, parameter_count, OP, "parameters")?;
        let context = force_context_from_c(context, PropagationContext::default());
        model
            .acceleration_partials(parameters, &cartesian_state_from_c(&state), &context)
            .map(Into::into)
            .map_err(|err| map_propagation_error(OP, err))
    })
}

/// Radiation pressure coefficients in m/s². Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SidereonEcom1Coefficients {
    /// Coefficient D0.
    pub d0: f64,
    /// Coefficient DC.
    pub dc: f64,
    /// Coefficient DS.
    pub ds: f64,
    /// Coefficient Y0.
    pub y0: f64,
    /// Coefficient YC.
    pub yc: f64,
    /// Coefficient YS.
    pub ys: f64,
    /// Coefficient B0.
    pub b0: f64,
    /// Coefficient BC.
    pub bc: f64,
    /// Coefficient BS.
    pub bs: f64,
}

impl From<SidereonEcom1Coefficients> for Ecom1Coefficients {
    fn from(value: SidereonEcom1Coefficients) -> Self {
        Self {
            d0: value.d0,
            dc: value.dc,
            ds: value.ds,
            y0: value.y0,
            yc: value.yc,
            ys: value.ys,
            b0: value.b0,
            bc: value.bc,
            bs: value.bs,
        }
    }
}

impl From<Ecom1Coefficients> for SidereonEcom1Coefficients {
    fn from(value: Ecom1Coefficients) -> Self {
        Self {
            d0: value.d0,
            dc: value.dc,
            ds: value.ds,
            y0: value.y0,
            yc: value.yc,
            ys: value.ys,
            b0: value.b0,
            bc: value.bc,
            bs: value.bs,
        }
    }
}

/// Number of coefficients in this model.
#[no_mangle]
pub extern "C" fn sidereon_ecom1_coefficient_count() -> usize {
    sidereon_core::astro::forces::ECOM1_COEFFICIENT_COUNT
}

/// Build coefficients from the canonical array order.
///
/// Safety: values points to 9 doubles and out points to writable coefficient storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom1_coefficients_from_array(
    values: *const f64,
    out: *mut SidereonEcom1Coefficients,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom1_coefficients_from_array";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        let values = *c_try!(require_f64_array::<9>(values, OP, "values"));
        out.write(Ecom1Coefficients::from_array(values).into());
        SidereonStatus::Ok
    })
}

/// Copy coefficients in canonical array order.
///
/// Safety: out points to 9 writable doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom1_coefficients_to_array(
    coefficients: SidereonEcom1Coefficients,
    out: *mut f64,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom1_coefficients_to_array";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out_f64_array::<9>(out, OP, "out"));
        out.write(Ecom1Coefficients::from(coefficients).to_array());
        SidereonStatus::Ok
    })
}

/// Radiation pressure coefficients in m/s². Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SidereonEcom2D2b1Coefficients {
    /// Coefficient B0.
    pub b0: f64,
    /// Coefficient B1C.
    pub b1c: f64,
    /// Coefficient B1S.
    pub b1s: f64,
    /// Coefficient D0.
    pub d0: f64,
    /// Coefficient D2C.
    pub d2c: f64,
    /// Coefficient D2S.
    pub d2s: f64,
    /// Coefficient Y0.
    pub y0: f64,
}

impl From<SidereonEcom2D2b1Coefficients> for Ecom2D2b1Coefficients {
    fn from(value: SidereonEcom2D2b1Coefficients) -> Self {
        Self {
            b0: value.b0,
            b1c: value.b1c,
            b1s: value.b1s,
            d0: value.d0,
            d2c: value.d2c,
            d2s: value.d2s,
            y0: value.y0,
        }
    }
}

impl From<Ecom2D2b1Coefficients> for SidereonEcom2D2b1Coefficients {
    fn from(value: Ecom2D2b1Coefficients) -> Self {
        Self {
            b0: value.b0,
            b1c: value.b1c,
            b1s: value.b1s,
            d0: value.d0,
            d2c: value.d2c,
            d2s: value.d2s,
            y0: value.y0,
        }
    }
}

/// Number of coefficients in this model.
#[no_mangle]
pub extern "C" fn sidereon_ecom2_d2b1_coefficient_count() -> usize {
    sidereon_core::astro::forces::ECOM2_D2B1_COEFFICIENT_COUNT
}

/// Build coefficients from the canonical array order.
///
/// Safety: values points to 7 doubles and out points to writable coefficient storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom2_d2b1_coefficients_from_array(
    values: *const f64,
    out: *mut SidereonEcom2D2b1Coefficients,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom2_d2b1_coefficients_from_array";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        let values = *c_try!(require_f64_array::<7>(values, OP, "values"));
        out.write(Ecom2D2b1Coefficients::from_array(values).into());
        SidereonStatus::Ok
    })
}

/// Copy coefficients in canonical array order.
///
/// Safety: out points to 7 writable doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom2_d2b1_coefficients_to_array(
    coefficients: SidereonEcom2D2b1Coefficients,
    out: *mut f64,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom2_d2b1_coefficients_to_array";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out_f64_array::<7>(out, OP, "out"));
        out.write(Ecom2D2b1Coefficients::from(coefficients).to_array());
        SidereonStatus::Ok
    })
}

/// Radiation pressure coefficients in m/s². Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SidereonEcom2D4b1Coefficients {
    /// Coefficient B0.
    pub b0: f64,
    /// Coefficient B1C.
    pub b1c: f64,
    /// Coefficient B1S.
    pub b1s: f64,
    /// Coefficient D0.
    pub d0: f64,
    /// Coefficient D2C.
    pub d2c: f64,
    /// Coefficient D4C.
    pub d4c: f64,
    /// Coefficient D2S.
    pub d2s: f64,
    /// Coefficient D4S.
    pub d4s: f64,
    /// Coefficient Y0.
    pub y0: f64,
}

impl From<SidereonEcom2D4b1Coefficients> for Ecom2D4b1Coefficients {
    fn from(value: SidereonEcom2D4b1Coefficients) -> Self {
        Self {
            b0: value.b0,
            b1c: value.b1c,
            b1s: value.b1s,
            d0: value.d0,
            d2c: value.d2c,
            d4c: value.d4c,
            d2s: value.d2s,
            d4s: value.d4s,
            y0: value.y0,
        }
    }
}

impl From<Ecom2D4b1Coefficients> for SidereonEcom2D4b1Coefficients {
    fn from(value: Ecom2D4b1Coefficients) -> Self {
        Self {
            b0: value.b0,
            b1c: value.b1c,
            b1s: value.b1s,
            d0: value.d0,
            d2c: value.d2c,
            d4c: value.d4c,
            d2s: value.d2s,
            d4s: value.d4s,
            y0: value.y0,
        }
    }
}

/// Number of coefficients in this model.
#[no_mangle]
pub extern "C" fn sidereon_ecom2_d4b1_coefficient_count() -> usize {
    sidereon_core::astro::forces::ECOM2_D4B1_COEFFICIENT_COUNT
}

/// Build coefficients from the canonical array order.
///
/// Safety: values points to 9 doubles and out points to writable coefficient storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom2_d4b1_coefficients_from_array(
    values: *const f64,
    out: *mut SidereonEcom2D4b1Coefficients,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom2_d4b1_coefficients_from_array";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        let values = *c_try!(require_f64_array::<9>(values, OP, "values"));
        out.write(Ecom2D4b1Coefficients::from_array(values).into());
        SidereonStatus::Ok
    })
}

/// Copy coefficients in canonical array order.
///
/// Safety: out points to 9 writable doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom2_d4b1_coefficients_to_array(
    coefficients: SidereonEcom2D4b1Coefficients,
    out: *mut f64,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom2_d4b1_coefficients_to_array";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out_f64_array::<9>(out, OP, "out"));
        out.write(Ecom2D4b1Coefficients::from(coefficients).to_array());
        SidereonStatus::Ok
    })
}
/// Create the ECOM1 radiation pressure model with explicit shadow options.
///
/// Safety: out points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom1_solar_radiation_pressure_new(
    coefficients: SidereonEcom1Coefficients,
    shadow: SidereonRadiationShadowOptions,
    out: *mut *mut SidereonForceModel,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom1_solar_radiation_pressure_new";
    write_force_model(OP, out, || {
        let value = Ecom1SolarRadiationPressure::new(coefficients.into())
            .map_err(|err| map_propagation_error(OP, err))?;
        Ok(Arc::new(configure_shadow!(OP, value, shadow)?))
    })
}

/// Create the ECOM2 radiation pressure model with explicit shadow options.
///
/// Safety: out points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom2_solar_radiation_pressure_d2b1(
    coefficients: SidereonEcom2D2b1Coefficients,
    shadow: SidereonRadiationShadowOptions,
    out: *mut *mut SidereonForceModel,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom2_solar_radiation_pressure_d2b1";
    write_force_model(OP, out, || {
        let value = Ecom2SolarRadiationPressure::d2b1(coefficients.into())
            .map_err(|err| map_propagation_error(OP, err))?;
        Ok(Arc::new(configure_shadow!(OP, value, shadow)?))
    })
}

/// Create the ECOM2 radiation pressure model with explicit shadow options.
///
/// Safety: out points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom2_solar_radiation_pressure_d4b1(
    coefficients: SidereonEcom2D4b1Coefficients,
    shadow: SidereonRadiationShadowOptions,
    out: *mut *mut SidereonForceModel,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom2_solar_radiation_pressure_d4b1";
    write_force_model(OP, out, || {
        let value = Ecom2SolarRadiationPressure::d4b1(coefficients.into())
            .map_err(|err| map_propagation_error(OP, err))?;
        Ok(Arc::new(configure_shadow!(OP, value, shadow)?))
    })
}

/// Create the ECOM2 radiation pressure model with explicit shadow options.
///
/// Safety: out points to writable handle storage. coefficients has count readable entries.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom2_solar_radiation_pressure_new(
    n_d: usize,
    n_b: usize,
    coefficients: *const f64,
    count: usize,
    shadow: SidereonRadiationShadowOptions,
    out: *mut *mut SidereonForceModel,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom2_solar_radiation_pressure_new";
    write_force_model(OP, out, || {
        let value = Ecom2SolarRadiationPressure::new(
            n_d,
            n_b,
            require_slice(coefficients, count, OP, "coefficients")?.to_vec(),
        )
        .map_err(|err| map_propagation_error(OP, err))?;
        Ok(Arc::new(configure_shadow!(OP, value, shadow)?))
    })
}

/// Axis of a radial, along-track, cross-track frame.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidereonRtnAxis {
    /// Radial.
    Radial = 0,
    /// Along track.
    AlongTrack = 1,
    /// Cross track.
    CrossTrack = 2,
}

/// Argument-of-latitude dependence of an empirical RTN term.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidereonEmpiricalHarmonic {
    /// Constant.
    Constant = 0,
    /// Cosine of argument of latitude.
    Cosine = 1,
    /// Sine of argument of latitude.
    Sine = 2,
}

/// One empirical RTN coefficient. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SidereonEmpiricalRtnTerm {
    /// Axis as SidereonRtnAxis.
    pub axis: u32,
    /// Harmonic as SidereonEmpiricalHarmonic.
    pub harmonic: u32,
    /// Coefficient in km/s².
    pub coefficient_km_s2: f64,
}

fn rtn_term_from_c(
    operation: &str,
    axis: u32,
    harmonic: u32,
) -> Result<(RtnAxis, EmpiricalHarmonic), SidereonStatus> {
    let axis = RtnAxis::ALL
        .get(axis as usize)
        .copied()
        .ok_or_else(|| extra_invalid_arg(operation, format_args!("invalid RTN axis {axis}")))?;
    let harmonic = EmpiricalHarmonic::ALL
        .get(harmonic as usize)
        .copied()
        .ok_or_else(|| {
            extra_invalid_arg(
                operation,
                format_args!("invalid empirical harmonic {harmonic}"),
            )
        })?;
    Ok((axis, harmonic))
}

/// Create an empirical RTN model with the supplied coefficients. Later entries
/// for the same axis and harmonic replace earlier entries.
///
/// Safety: terms points to count entries and out is disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_empirical_rtn_acceleration_new(
    terms: *const SidereonEmpiricalRtnTerm,
    count: usize,
    out: *mut *mut SidereonForceModel,
) -> SidereonStatus {
    const OP: &str = "sidereon_empirical_rtn_acceleration_new";
    write_force_model(OP, out, || {
        let mut model = EmpiricalRtnAcceleration::new();
        for term in require_slice(terms, count, OP, "terms")? {
            let (axis, harmonic) = rtn_term_from_c(OP, term.axis, term.harmonic)?;
            model = model
                .with_term(axis, harmonic, term.coefficient_km_s2)
                .map_err(|err| map_propagation_error(OP, err))?;
        }
        Ok(Arc::new(model))
    })
}

/// Create an empirical RTN model with the three constant terms set to zero.
///
/// Safety: out points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_empirical_rtn_acceleration_constant_terms(
    out: *mut *mut SidereonForceModel,
) -> SidereonStatus {
    write_force_model(
        "sidereon_empirical_rtn_acceleration_constant_terms",
        out,
        || Ok(Arc::new(EmpiricalRtnAcceleration::constant_terms())),
    )
}

/// Create an empirical RTN model with all nine terms set to zero.
///
/// Safety: out points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_empirical_rtn_acceleration_all_terms(
    out: *mut *mut SidereonForceModel,
) -> SidereonStatus {
    write_force_model("sidereon_empirical_rtn_acceleration_all_terms", out, || {
        Ok(Arc::new(EmpiricalRtnAcceleration::all_terms()))
    })
}

/// Copy the canonical name of an empirical RTN term without a terminator.
/// NULL and zero capacity query the full byte count.
///
/// Safety: all outputs are writable and disjoint. out_name has capacity bytes.
#[no_mangle]
pub unsafe extern "C" fn sidereon_empirical_rtn_acceleration_parameter_name(
    axis: u32,
    harmonic: u32,
    out_name: *mut u8,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_empirical_rtn_acceleration_parameter_name";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(OP, out_written, out_required));
        let (axis, harmonic) = c_try!(rtn_term_from_c(OP, axis, harmonic));
        let name = EmpiricalRtnAcceleration::parameter_name(axis, harmonic);
        c_try!(copy_prefix_to_c(
            OP,
            "out_name",
            name.as_bytes(),
            out_name,
            capacity,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Supplementary options for a built-in force configuration.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonStatePropagationEnvironment {
    /// Gravity tide system as SidereonGravityTideSystem.
    pub gravity_tide_system: u32,
    /// Optional solar radiation pressure shadow override.
    pub solar_shadow: *const SidereonRadiationShadowOptions,
    /// Optional Earth albedo and infrared radiation pressure.
    pub earth_radiation_pressure: *const SidereonEarthRadiationPressureOptions,
    /// Optional propagation context.
    pub context: *const SidereonPropagationContext,
    /// Optional space-weather table when drag is enabled.
    pub space_weather: *const SidereonSpaceWeatherTable,
}

pub(crate) unsafe fn state_propagator_with_environment(
    operation: &str,
    config: &SidereonStatePropagationConfig,
    environment: *const SidereonStatePropagationEnvironment,
) -> Result<(StatePropagator, PropagationContext), SidereonStatus> {
    let context = propagation_context_from_c(config);
    let Some(environment) = environment.as_ref() else {
        let propagator =
            build_state_propagator_from_c(operation, config, CoreTideSystem::TideFree, false)?;
        return Ok((propagator, context));
    };
    let tide = gravity_tide_system_from_c(operation, environment.gravity_tide_system)?;
    let mut propagator = build_state_propagator_from_c(operation, config, tide, false)?;
    configure_radiation(
        operation,
        &mut propagator.force_model,
        environment.solar_shadow,
        environment.earth_radiation_pressure,
    )?;
    if let Some(table) = environment.space_weather.as_ref() {
        propagator.space_weather = Some(SpaceWeatherSource::Table(table.inner.clone()));
    }
    Ok((
        propagator,
        force_context_from_c(environment.context, context),
    ))
}

pub(crate) unsafe fn reject_output_overlaps_forces<T>(
    operation: &str,
    out: *mut T,
    out_name: &str,
    forces: *const *const SidereonForceModel,
    count: usize,
) -> Result<(), SidereonStatus> {
    if forces.is_null() || count == 0 {
        return Ok(());
    }
    let forces_range = checked_output_range(operation, forces.cast_mut(), count, "forces")?;
    let output_range = checked_output_range(operation, out, 1, out_name)?;
    reject_overlapping_outputs(operation, output_range, forces_range, out_name, "forces")?;
    for &force in require_slice(forces, count, operation, "forces")? {
        reject_output_overlaps_handle(operation, out, out_name, force, "force")?;
    }
    Ok(())
}

pub(crate) unsafe fn additional_forces_from_c(
    operation: &str,
    forces: *const *const SidereonForceModel,
    count: usize,
) -> Result<Vec<Arc<dyn ForceModel>>, SidereonStatus> {
    require_slice(forces, count, operation, "additional_forces")?
        .iter()
        .map(|&force| {
            require_ref(force, operation, "additional_force")
                .map(|force| -> Arc<dyn ForceModel> { force.inner.clone() })
        })
        .collect()
}

fn concrete_model<'a, T: 'static>(
    operation: &str,
    model: &'a dyn CForceModel,
    name: &str,
) -> Result<&'a T, SidereonStatus> {
    model
        .as_any()
        .downcast_ref()
        .ok_or_else(|| extra_invalid_arg(operation, format_args!("force model must be {name}")))
}

/// Sun-fixed geometry used by the ECOM radiation pressure models. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SidereonEcomGeometry {
    /// Sun-fixed unit vectors.
    pub dyb: SidereonDybAxes,
    /// Difference in latitude argument, radians, when defined.
    pub delta_u_rad: SidereonOptionalOrbitScalar,
    /// Satellite argument of latitude, radians, when defined.
    pub argument_of_latitude_rad: SidereonOptionalOrbitScalar,
    /// Fraction of incident sunlight.
    pub lighting_ratio: f64,
}

impl From<sidereon_core::astro::forces::EcomGeometry> for SidereonEcomGeometry {
    fn from(value: sidereon_core::astro::forces::EcomGeometry) -> Self {
        Self {
            dyb: SidereonDybAxes {
                d: value.dyb.d,
                y: value.dyb.y,
                b: value.dyb.b,
            },
            delta_u_rad: value.delta_u_rad.into(),
            argument_of_latitude_rad: value.argument_of_latitude_rad.into(),
            lighting_ratio: value.lighting_ratio,
        }
    }
}

/// Evaluate ECOM geometry with WGS84 oblate Earth.
///
/// Safety: out points to writable geometry storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom_geometry(
    position_km: SidereonVector3,
    velocity_km_s: SidereonVector3,
    sun_position_km: SidereonVector3,
    out: *mut SidereonEcomGeometry,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom_geometry";
    engine_error_operation_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        let result = c_try!(sidereon_core::astro::forces::ecom_geometry(
            position_km.xyz.into(),
            velocity_km_s.xyz.into(),
            sun_position_km.xyz.into(),
        )
        .map_err(|err| map_propagation_error(OP, err)));
        out.write(result.into());
        SidereonStatus::Ok
    })
}

/// Evaluate ECOM geometry with an explicit Earth figure and optional Moon.
///
/// Safety: moon_position_km is NULL or readable. out is disjoint writable geometry storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom_geometry_with_shadow(
    position_km: SidereonVector3,
    velocity_km_s: SidereonVector3,
    sun_position_km: SidereonVector3,
    moon_position_km: *const SidereonVector3,
    earth_shadow_model: u32,
    out: *mut SidereonEcomGeometry,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom_geometry_with_shadow";
    engine_error_operation_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        let earth_shadow_model = c_try!(crate::almanac::earth_shadow_model_from_c(
            OP,
            earth_shadow_model
        ));
        let result = c_try!(sidereon_core::astro::forces::ecom_geometry_with_shadow(
            position_km.xyz.into(),
            velocity_km_s.xyz.into(),
            sun_position_km.xyz.into(),
            moon_position_km.as_ref().map(|moon| moon.xyz.into()),
            earth_shadow_model,
        )
        .map_err(|err| map_propagation_error(OP, err)));
        out.write(result.into());
        SidereonStatus::Ok
    })
}

/// Coefficient count of a general ECOM2 series. An overflow leaves the value absent.
///
/// Safety: out points to writable count storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom2_solar_radiation_pressure_coefficient_count(
    n_d: usize,
    n_b: usize,
    out: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom2_solar_radiation_pressure_coefficient_count";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        let Some(count) = Ecom2SolarRadiationPressure::coefficient_count(n_d, n_b) else {
            return extra_invalid_arg(OP, "coefficient count overflows");
        };
        out.write(count);
        SidereonStatus::Ok
    })
}
/// Evaluate acceleration with explicit geocentric body positions in kilometres.
///
/// Safety: model is live. All outputs are writable and disjoint from the handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom1_solar_radiation_pressure_acceleration_with_sun(
    model: *const SidereonForceModel,
    position_km: SidereonVector3,
    velocity_km_s: SidereonVector3,
    sun_position_km: SidereonVector3,
    out: *mut SidereonVector3,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom1_solar_radiation_pressure_acceleration_with_sun";

    force_query(OP, model, out, |model| {
        let model = concrete_model::<Ecom1SolarRadiationPressure>(
            OP,
            model,
            "Ecom1SolarRadiationPressure",
        )?;
        model
            .acceleration_with_sun(
                position_km.xyz.into(),
                velocity_km_s.xyz.into(),
                sun_position_km.xyz.into(),
            )
            .map(Into::into)
            .map_err(|err| map_propagation_error(OP, err))
    })
}
/// Evaluate acceleration with explicit geocentric body positions in kilometres.
///
/// Safety: model is live. All outputs are writable and disjoint from the handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom1_solar_radiation_pressure_acceleration_with_sun_moon(
    model: *const SidereonForceModel,
    position_km: SidereonVector3,
    velocity_km_s: SidereonVector3,
    sun_position_km: SidereonVector3,
    moon_position_km: SidereonVector3,
    out: *mut SidereonVector3,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom1_solar_radiation_pressure_acceleration_with_sun_moon";

    force_query(OP, model, out, |model| {
        let model = concrete_model::<Ecom1SolarRadiationPressure>(
            OP,
            model,
            "Ecom1SolarRadiationPressure",
        )?;
        model
            .acceleration_with_sun_moon(
                position_km.xyz.into(),
                velocity_km_s.xyz.into(),
                sun_position_km.xyz.into(),
                moon_position_km.xyz.into(),
            )
            .map(Into::into)
            .map_err(|err| map_propagation_error(OP, err))
    })
}
/// Evaluate state partials with explicit geocentric body positions in kilometres.
///
/// Safety: model is live. All outputs are writable and disjoint from the handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom1_solar_radiation_pressure_acceleration_partials_with_sun(
    model: *const SidereonForceModel,
    position_km: SidereonVector3,
    velocity_km_s: SidereonVector3,
    sun_position_km: SidereonVector3,
    out: *mut SidereonAccelerationPartials,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom1_solar_radiation_pressure_acceleration_partials_with_sun";

    force_query(OP, model, out, |model| {
        let model = concrete_model::<Ecom1SolarRadiationPressure>(
            OP,
            model,
            "Ecom1SolarRadiationPressure",
        )?;
        model
            .acceleration_partials_with_sun(
                position_km.xyz.into(),
                velocity_km_s.xyz.into(),
                sun_position_km.xyz.into(),
            )
            .map(Into::into)
            .map_err(|err| map_propagation_error(OP, err))
    })
}
/// Evaluate state partials with explicit geocentric body positions in kilometres.
///
/// Safety: model is live. All outputs are writable and disjoint from the handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom1_solar_radiation_pressure_acceleration_partials_with_sun_moon(
    model: *const SidereonForceModel,
    position_km: SidereonVector3,
    velocity_km_s: SidereonVector3,
    sun_position_km: SidereonVector3,
    moon_position_km: SidereonVector3,
    out: *mut SidereonAccelerationPartials,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom1_solar_radiation_pressure_acceleration_partials_with_sun_moon";

    force_query(OP, model, out, |model| {
        let model = concrete_model::<Ecom1SolarRadiationPressure>(
            OP,
            model,
            "Ecom1SolarRadiationPressure",
        )?;
        model
            .acceleration_partials_with_sun_moon(
                position_km.xyz.into(),
                velocity_km_s.xyz.into(),
                sun_position_km.xyz.into(),
                moon_position_km.xyz.into(),
            )
            .map(Into::into)
            .map_err(|err| map_propagation_error(OP, err))
    })
}
/// Evaluate parameter partials with explicit geocentric body positions in kilometres.
/// NULL and zero capacity query the full vector count.
///
/// Safety: model is live. All outputs are writable and disjoint from the handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom1_solar_radiation_pressure_parameter_partials_with_sun(
    model: *const SidereonForceModel,
    position_km: SidereonVector3,
    velocity_km_s: SidereonVector3,
    sun_position_km: SidereonVector3,
    out: *mut SidereonVector3,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom1_solar_radiation_pressure_parameter_partials_with_sun";

    engine_error_operation_boundary(OP, SidereonStatus::Panic, || {
        c_try!(init_handle_copy_counts(
            OP,
            out,
            capacity,
            out_written,
            out_required,
            model,
            "model"
        ));
        let model = c_try!(require_ref(model, OP, "model"));
        let model = c_try!(concrete_model::<Ecom1SolarRadiationPressure>(
            OP,
            model.inner.as_ref(),
            "Ecom1SolarRadiationPressure"
        ));
        let vectors = c_try!(model
            .parameter_partials_with_sun(
                position_km.xyz.into(),
                velocity_km_s.xyz.into(),
                sun_position_km.xyz.into()
            )
            .map_err(|err| map_propagation_error(OP, err)));
        let vectors: Vec<_> = vectors.into_iter().map(SidereonVector3::from).collect();
        c_try!(copy_prefix_to_c(
            OP,
            "out",
            &vectors,
            out,
            capacity,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}
/// Evaluate parameter partials with explicit geocentric body positions in kilometres.
/// NULL and zero capacity query the full vector count.
///
/// Safety: model is live. All outputs are writable and disjoint from the handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom1_solar_radiation_pressure_parameter_partials_with_sun_moon(
    model: *const SidereonForceModel,
    position_km: SidereonVector3,
    velocity_km_s: SidereonVector3,
    sun_position_km: SidereonVector3,
    moon_position_km: SidereonVector3,
    out: *mut SidereonVector3,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom1_solar_radiation_pressure_parameter_partials_with_sun_moon";

    engine_error_operation_boundary(OP, SidereonStatus::Panic, || {
        c_try!(init_handle_copy_counts(
            OP,
            out,
            capacity,
            out_written,
            out_required,
            model,
            "model"
        ));
        let model = c_try!(require_ref(model, OP, "model"));
        let model = c_try!(concrete_model::<Ecom1SolarRadiationPressure>(
            OP,
            model.inner.as_ref(),
            "Ecom1SolarRadiationPressure"
        ));
        let vectors = c_try!(model
            .parameter_partials_with_sun_moon(
                position_km.xyz.into(),
                velocity_km_s.xyz.into(),
                sun_position_km.xyz.into(),
                moon_position_km.xyz.into()
            )
            .map_err(|err| map_propagation_error(OP, err)));
        let vectors: Vec<_> = vectors.into_iter().map(SidereonVector3::from).collect();
        c_try!(copy_prefix_to_c(
            OP,
            "out",
            &vectors,
            out,
            capacity,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}
/// Evaluate acceleration with explicit geocentric body positions in kilometres.
///
/// Safety: model is live. All outputs are writable and disjoint from the handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom2_solar_radiation_pressure_acceleration_with_sun(
    model: *const SidereonForceModel,
    position_km: SidereonVector3,
    velocity_km_s: SidereonVector3,
    sun_position_km: SidereonVector3,
    out: *mut SidereonVector3,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom2_solar_radiation_pressure_acceleration_with_sun";

    force_query(OP, model, out, |model| {
        let model = concrete_model::<Ecom2SolarRadiationPressure>(
            OP,
            model,
            "Ecom2SolarRadiationPressure",
        )?;
        model
            .acceleration_with_sun(
                position_km.xyz.into(),
                velocity_km_s.xyz.into(),
                sun_position_km.xyz.into(),
            )
            .map(Into::into)
            .map_err(|err| map_propagation_error(OP, err))
    })
}
/// Evaluate acceleration with explicit geocentric body positions in kilometres.
///
/// Safety: model is live. All outputs are writable and disjoint from the handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom2_solar_radiation_pressure_acceleration_with_sun_moon(
    model: *const SidereonForceModel,
    position_km: SidereonVector3,
    velocity_km_s: SidereonVector3,
    sun_position_km: SidereonVector3,
    moon_position_km: SidereonVector3,
    out: *mut SidereonVector3,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom2_solar_radiation_pressure_acceleration_with_sun_moon";

    force_query(OP, model, out, |model| {
        let model = concrete_model::<Ecom2SolarRadiationPressure>(
            OP,
            model,
            "Ecom2SolarRadiationPressure",
        )?;
        model
            .acceleration_with_sun_moon(
                position_km.xyz.into(),
                velocity_km_s.xyz.into(),
                sun_position_km.xyz.into(),
                moon_position_km.xyz.into(),
            )
            .map(Into::into)
            .map_err(|err| map_propagation_error(OP, err))
    })
}
/// Evaluate state partials with explicit geocentric body positions in kilometres.
///
/// Safety: model is live. All outputs are writable and disjoint from the handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom2_solar_radiation_pressure_acceleration_partials_with_sun(
    model: *const SidereonForceModel,
    position_km: SidereonVector3,
    velocity_km_s: SidereonVector3,
    sun_position_km: SidereonVector3,
    out: *mut SidereonAccelerationPartials,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom2_solar_radiation_pressure_acceleration_partials_with_sun";

    force_query(OP, model, out, |model| {
        let model = concrete_model::<Ecom2SolarRadiationPressure>(
            OP,
            model,
            "Ecom2SolarRadiationPressure",
        )?;
        model
            .acceleration_partials_with_sun(
                position_km.xyz.into(),
                velocity_km_s.xyz.into(),
                sun_position_km.xyz.into(),
            )
            .map(Into::into)
            .map_err(|err| map_propagation_error(OP, err))
    })
}
/// Evaluate state partials with explicit geocentric body positions in kilometres.
///
/// Safety: model is live. All outputs are writable and disjoint from the handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom2_solar_radiation_pressure_acceleration_partials_with_sun_moon(
    model: *const SidereonForceModel,
    position_km: SidereonVector3,
    velocity_km_s: SidereonVector3,
    sun_position_km: SidereonVector3,
    moon_position_km: SidereonVector3,
    out: *mut SidereonAccelerationPartials,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom2_solar_radiation_pressure_acceleration_partials_with_sun_moon";

    force_query(OP, model, out, |model| {
        let model = concrete_model::<Ecom2SolarRadiationPressure>(
            OP,
            model,
            "Ecom2SolarRadiationPressure",
        )?;
        model
            .acceleration_partials_with_sun_moon(
                position_km.xyz.into(),
                velocity_km_s.xyz.into(),
                sun_position_km.xyz.into(),
                moon_position_km.xyz.into(),
            )
            .map(Into::into)
            .map_err(|err| map_propagation_error(OP, err))
    })
}
/// Evaluate parameter partials with explicit geocentric body positions in kilometres.
/// NULL and zero capacity query the full vector count.
///
/// Safety: model is live. All outputs are writable and disjoint from the handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom2_solar_radiation_pressure_parameter_partials_with_sun(
    model: *const SidereonForceModel,
    position_km: SidereonVector3,
    velocity_km_s: SidereonVector3,
    sun_position_km: SidereonVector3,
    out: *mut SidereonVector3,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom2_solar_radiation_pressure_parameter_partials_with_sun";

    engine_error_operation_boundary(OP, SidereonStatus::Panic, || {
        c_try!(init_handle_copy_counts(
            OP,
            out,
            capacity,
            out_written,
            out_required,
            model,
            "model"
        ));
        let model = c_try!(require_ref(model, OP, "model"));
        let model = c_try!(concrete_model::<Ecom2SolarRadiationPressure>(
            OP,
            model.inner.as_ref(),
            "Ecom2SolarRadiationPressure"
        ));
        let vectors = c_try!(model
            .parameter_partials_with_sun(
                position_km.xyz.into(),
                velocity_km_s.xyz.into(),
                sun_position_km.xyz.into()
            )
            .map_err(|err| map_propagation_error(OP, err)));
        let vectors: Vec<_> = vectors.into_iter().map(SidereonVector3::from).collect();
        c_try!(copy_prefix_to_c(
            OP,
            "out",
            &vectors,
            out,
            capacity,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}
/// Evaluate parameter partials with explicit geocentric body positions in kilometres.
/// NULL and zero capacity query the full vector count.
///
/// Safety: model is live. All outputs are writable and disjoint from the handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom2_solar_radiation_pressure_parameter_partials_with_sun_moon(
    model: *const SidereonForceModel,
    position_km: SidereonVector3,
    velocity_km_s: SidereonVector3,
    sun_position_km: SidereonVector3,
    moon_position_km: SidereonVector3,
    out: *mut SidereonVector3,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom2_solar_radiation_pressure_parameter_partials_with_sun_moon";

    engine_error_operation_boundary(OP, SidereonStatus::Panic, || {
        c_try!(init_handle_copy_counts(
            OP,
            out,
            capacity,
            out_written,
            out_required,
            model,
            "model"
        ));
        let model = c_try!(require_ref(model, OP, "model"));
        let model = c_try!(concrete_model::<Ecom2SolarRadiationPressure>(
            OP,
            model.inner.as_ref(),
            "Ecom2SolarRadiationPressure"
        ));
        let vectors = c_try!(model
            .parameter_partials_with_sun_moon(
                position_km.xyz.into(),
                velocity_km_s.xyz.into(),
                sun_position_km.xyz.into(),
                moon_position_km.xyz.into()
            )
            .map_err(|err| map_propagation_error(OP, err)));
        let vectors: Vec<_> = vectors.into_iter().map(SidereonVector3::from).collect();
        c_try!(copy_prefix_to_c(
            OP,
            "out",
            &vectors,
            out,
            capacity,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}
/// Evaluate acceleration with explicit geocentric body positions in kilometres.
///
/// Safety: model is live. All outputs are writable and disjoint from the handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solar_radiation_pressure_acceleration_with_sun_moon(
    model: *const SidereonForceModel,
    position_km: SidereonVector3,
    sun_position_km: SidereonVector3,
    moon_position_km: SidereonVector3,
    out: *mut SidereonVector3,
) -> SidereonStatus {
    const OP: &str = "sidereon_solar_radiation_pressure_acceleration_with_sun_moon";

    force_query(OP, model, out, |model| {
        let model = concrete_model::<SolarRadiationPressure>(OP, model, "SolarRadiationPressure")?;
        model
            .acceleration_with_sun_moon(
                position_km.xyz.into(),
                sun_position_km.xyz.into(),
                moon_position_km.xyz.into(),
            )
            .map(Into::into)
            .map_err(|err| map_propagation_error(OP, err))
    })
}
/// Evaluate acceleration with explicit geocentric body positions in kilometres.
///
/// Safety: model is live. All outputs are writable and disjoint from the handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_earth_radiation_pressure_acceleration_with_sun_moon(
    model: *const SidereonForceModel,
    state: SidereonCartesianState,
    sun_position_km: SidereonVector3,
    moon_position_km: SidereonVector3,
    out: *mut SidereonVector3,
) -> SidereonStatus {
    const OP: &str = "sidereon_earth_radiation_pressure_acceleration_with_sun_moon";

    force_query(OP, model, out, |model| {
        let model = concrete_model::<EarthRadiationPressure>(OP, model, "EarthRadiationPressure")?;
        model
            .acceleration_with_sun_moon(
                &cartesian_state_from_c(&state),
                sun_position_km.xyz.into(),
                moon_position_km.xyz.into(),
            )
            .map(Into::into)
            .map_err(|err| map_propagation_error(OP, err))
    })
}

unsafe fn copy_concrete_force(
    operation: &str,
    model: *const SidereonForceModel,
    out: *mut *mut SidereonForceModel,
    copy: impl FnOnce(&dyn CForceModel) -> Result<Arc<dyn CForceModel>, SidereonStatus>,
) -> SidereonStatus {
    write_guarded_force_model(
        operation,
        out,
        || reject_output_overlaps_handle(operation, out, "out", model, "model"),
        || copy(require_ref(model, operation, "model")?.inner.as_ref()),
    )
}

/// Read the configured shadow function.
///
/// Safety: model is live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solar_radiation_pressure_shadow_function(
    model: *const SidereonForceModel,
    out: *mut SidereonShadowFunction,
) -> SidereonStatus {
    const OP: &str = "sidereon_solar_radiation_pressure_shadow_function";
    force_query(OP, model, out, |model| {
        let model = concrete_model::<SolarRadiationPressure>(OP, model, "SolarRadiationPressure")?;
        Ok(match model.shadow_function() {
            ShadowFunction::ApparentDisc => SidereonShadowFunction::ApparentDisc,
            ShadowFunction::Conical => SidereonShadowFunction::Conical,
            _ => return Err(extra_invalid_arg(OP, "unsupported shadow function")),
        })
    })
}

/// Read the configured Earth figure.
///
/// Safety: model is live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solar_radiation_pressure_earth_shadow_model(
    model: *const SidereonForceModel,
    out: *mut SidereonEarthShadowModel,
) -> SidereonStatus {
    const OP: &str = "sidereon_solar_radiation_pressure_earth_shadow_model";
    force_query(OP, model, out, |model| {
        let model = concrete_model::<SolarRadiationPressure>(OP, model, "SolarRadiationPressure")?;
        Ok(match model.earth_shadow_model() {
            sidereon_core::astro::events::eclipse::EarthShadowModel::Spherical => {
                SidereonEarthShadowModel::Spherical
            }
            sidereon_core::astro::events::eclipse::EarthShadowModel::Wgs84Oblate => {
                SidereonEarthShadowModel::Wgs84Oblate
            }
        })
    })
}

/// Read whether lunar occultation is enabled.
///
/// Safety: model is live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solar_radiation_pressure_moon_eclipse_enabled(
    model: *const SidereonForceModel,
    out: *mut bool,
) -> SidereonStatus {
    const OP: &str = "sidereon_solar_radiation_pressure_moon_eclipse_enabled";
    force_query(OP, model, out, |model| {
        let model = concrete_model::<SolarRadiationPressure>(OP, model, "SolarRadiationPressure")?;
        Ok(model.moon_eclipse_enabled())
    })
}

/// Copy the force with the specified shadow function.
///
/// Safety: model is live and out is disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solar_radiation_pressure_with_shadow_function(
    model: *const SidereonForceModel,
    function: u32,
    out: *mut *mut SidereonForceModel,
) -> SidereonStatus {
    const OP: &str = "sidereon_solar_radiation_pressure_with_shadow_function";
    copy_concrete_force(OP, model, out, |model| {
        let model = concrete_model::<SolarRadiationPressure>(OP, model, "SolarRadiationPressure")?;
        Ok(Arc::new((*model).with_shadow_function(
            shadow_function_from_c(OP, function)?,
        )))
    })
}

/// Copy the force with the specified Earth figure.
///
/// Safety: model is live and out is disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solar_radiation_pressure_with_earth_shadow_model(
    model: *const SidereonForceModel,
    earth_model: u32,
    out: *mut *mut SidereonForceModel,
) -> SidereonStatus {
    const OP: &str = "sidereon_solar_radiation_pressure_with_earth_shadow_model";
    copy_concrete_force(OP, model, out, |model| {
        let model = concrete_model::<SolarRadiationPressure>(OP, model, "SolarRadiationPressure")?;
        Ok(Arc::new((*model).with_earth_shadow_model(
            crate::almanac::earth_shadow_model_from_c(OP, earth_model)?,
        )))
    })
}

/// Copy the force with the specified lunar occultation setting.
///
/// Safety: model is live and out is disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solar_radiation_pressure_with_moon_eclipse(
    model: *const SidereonForceModel,
    enabled: bool,
    out: *mut *mut SidereonForceModel,
) -> SidereonStatus {
    const OP: &str = "sidereon_solar_radiation_pressure_with_moon_eclipse";
    copy_concrete_force(OP, model, out, |model| {
        let model = concrete_model::<SolarRadiationPressure>(OP, model, "SolarRadiationPressure")?;
        Ok(Arc::new((*model).with_moon_eclipse(enabled)))
    })
}

/// Read the configured shadow function.
///
/// Safety: model is live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom1_solar_radiation_pressure_shadow_function(
    model: *const SidereonForceModel,
    out: *mut SidereonShadowFunction,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom1_solar_radiation_pressure_shadow_function";
    force_query(OP, model, out, |model| {
        let model = concrete_model::<Ecom1SolarRadiationPressure>(
            OP,
            model,
            "Ecom1SolarRadiationPressure",
        )?;
        Ok(match model.shadow_function() {
            ShadowFunction::ApparentDisc => SidereonShadowFunction::ApparentDisc,
            ShadowFunction::Conical => SidereonShadowFunction::Conical,
            _ => return Err(extra_invalid_arg(OP, "unsupported shadow function")),
        })
    })
}

/// Read the configured Earth figure.
///
/// Safety: model is live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom1_solar_radiation_pressure_earth_shadow_model(
    model: *const SidereonForceModel,
    out: *mut SidereonEarthShadowModel,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom1_solar_radiation_pressure_earth_shadow_model";
    force_query(OP, model, out, |model| {
        let model = concrete_model::<Ecom1SolarRadiationPressure>(
            OP,
            model,
            "Ecom1SolarRadiationPressure",
        )?;
        Ok(match model.earth_shadow_model() {
            sidereon_core::astro::events::eclipse::EarthShadowModel::Spherical => {
                SidereonEarthShadowModel::Spherical
            }
            sidereon_core::astro::events::eclipse::EarthShadowModel::Wgs84Oblate => {
                SidereonEarthShadowModel::Wgs84Oblate
            }
        })
    })
}

/// Read whether lunar occultation is enabled.
///
/// Safety: model is live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom1_solar_radiation_pressure_moon_eclipse_enabled(
    model: *const SidereonForceModel,
    out: *mut bool,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom1_solar_radiation_pressure_moon_eclipse_enabled";
    force_query(OP, model, out, |model| {
        let model = concrete_model::<Ecom1SolarRadiationPressure>(
            OP,
            model,
            "Ecom1SolarRadiationPressure",
        )?;
        Ok(model.moon_eclipse_enabled())
    })
}

/// Copy the force with the specified shadow function.
///
/// Safety: model is live and out is disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom1_solar_radiation_pressure_with_shadow_function(
    model: *const SidereonForceModel,
    function: u32,
    out: *mut *mut SidereonForceModel,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom1_solar_radiation_pressure_with_shadow_function";
    copy_concrete_force(OP, model, out, |model| {
        let model = concrete_model::<Ecom1SolarRadiationPressure>(
            OP,
            model,
            "Ecom1SolarRadiationPressure",
        )?;
        Ok(Arc::new((*model).with_shadow_function(
            shadow_function_from_c(OP, function)?,
        )))
    })
}

/// Copy the force with the specified Earth figure.
///
/// Safety: model is live and out is disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom1_solar_radiation_pressure_with_earth_shadow_model(
    model: *const SidereonForceModel,
    earth_model: u32,
    out: *mut *mut SidereonForceModel,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom1_solar_radiation_pressure_with_earth_shadow_model";
    copy_concrete_force(OP, model, out, |model| {
        let model = concrete_model::<Ecom1SolarRadiationPressure>(
            OP,
            model,
            "Ecom1SolarRadiationPressure",
        )?;
        Ok(Arc::new((*model).with_earth_shadow_model(
            crate::almanac::earth_shadow_model_from_c(OP, earth_model)?,
        )))
    })
}

/// Copy the force with the specified lunar occultation setting.
///
/// Safety: model is live and out is disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom1_solar_radiation_pressure_with_moon_eclipse(
    model: *const SidereonForceModel,
    enabled: bool,
    out: *mut *mut SidereonForceModel,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom1_solar_radiation_pressure_with_moon_eclipse";
    copy_concrete_force(OP, model, out, |model| {
        let model = concrete_model::<Ecom1SolarRadiationPressure>(
            OP,
            model,
            "Ecom1SolarRadiationPressure",
        )?;
        Ok(Arc::new((*model).with_moon_eclipse(enabled)))
    })
}

/// Read the configured shadow function.
///
/// Safety: model is live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom2_solar_radiation_pressure_shadow_function(
    model: *const SidereonForceModel,
    out: *mut SidereonShadowFunction,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom2_solar_radiation_pressure_shadow_function";
    force_query(OP, model, out, |model| {
        let model = concrete_model::<Ecom2SolarRadiationPressure>(
            OP,
            model,
            "Ecom2SolarRadiationPressure",
        )?;
        Ok(match model.shadow_function() {
            ShadowFunction::ApparentDisc => SidereonShadowFunction::ApparentDisc,
            ShadowFunction::Conical => SidereonShadowFunction::Conical,
            _ => return Err(extra_invalid_arg(OP, "unsupported shadow function")),
        })
    })
}

/// Read the configured Earth figure.
///
/// Safety: model is live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom2_solar_radiation_pressure_earth_shadow_model(
    model: *const SidereonForceModel,
    out: *mut SidereonEarthShadowModel,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom2_solar_radiation_pressure_earth_shadow_model";
    force_query(OP, model, out, |model| {
        let model = concrete_model::<Ecom2SolarRadiationPressure>(
            OP,
            model,
            "Ecom2SolarRadiationPressure",
        )?;
        Ok(match model.earth_shadow_model() {
            sidereon_core::astro::events::eclipse::EarthShadowModel::Spherical => {
                SidereonEarthShadowModel::Spherical
            }
            sidereon_core::astro::events::eclipse::EarthShadowModel::Wgs84Oblate => {
                SidereonEarthShadowModel::Wgs84Oblate
            }
        })
    })
}

/// Read whether lunar occultation is enabled.
///
/// Safety: model is live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom2_solar_radiation_pressure_moon_eclipse_enabled(
    model: *const SidereonForceModel,
    out: *mut bool,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom2_solar_radiation_pressure_moon_eclipse_enabled";
    force_query(OP, model, out, |model| {
        let model = concrete_model::<Ecom2SolarRadiationPressure>(
            OP,
            model,
            "Ecom2SolarRadiationPressure",
        )?;
        Ok(model.moon_eclipse_enabled())
    })
}

/// Copy the force with the specified shadow function.
///
/// Safety: model is live and out is disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom2_solar_radiation_pressure_with_shadow_function(
    model: *const SidereonForceModel,
    function: u32,
    out: *mut *mut SidereonForceModel,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom2_solar_radiation_pressure_with_shadow_function";
    copy_concrete_force(OP, model, out, |model| {
        let model = concrete_model::<Ecom2SolarRadiationPressure>(
            OP,
            model,
            "Ecom2SolarRadiationPressure",
        )?;
        Ok(Arc::new((model.clone()).with_shadow_function(
            shadow_function_from_c(OP, function)?,
        )))
    })
}

/// Copy the force with the specified Earth figure.
///
/// Safety: model is live and out is disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom2_solar_radiation_pressure_with_earth_shadow_model(
    model: *const SidereonForceModel,
    earth_model: u32,
    out: *mut *mut SidereonForceModel,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom2_solar_radiation_pressure_with_earth_shadow_model";
    copy_concrete_force(OP, model, out, |model| {
        let model = concrete_model::<Ecom2SolarRadiationPressure>(
            OP,
            model,
            "Ecom2SolarRadiationPressure",
        )?;
        Ok(Arc::new((model.clone()).with_earth_shadow_model(
            crate::almanac::earth_shadow_model_from_c(OP, earth_model)?,
        )))
    })
}

/// Copy the force with the specified lunar occultation setting.
///
/// Safety: model is live and out is disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom2_solar_radiation_pressure_with_moon_eclipse(
    model: *const SidereonForceModel,
    enabled: bool,
    out: *mut *mut SidereonForceModel,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom2_solar_radiation_pressure_with_moon_eclipse";
    copy_concrete_force(OP, model, out, |model| {
        let model = concrete_model::<Ecom2SolarRadiationPressure>(
            OP,
            model,
            "Ecom2SolarRadiationPressure",
        )?;
        Ok(Arc::new((model.clone()).with_moon_eclipse(enabled)))
    })
}

/// Read whether lunar occultation is enabled.
///
/// Safety: model is live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_earth_radiation_pressure_moon_eclipse_enabled(
    model: *const SidereonForceModel,
    out: *mut bool,
) -> SidereonStatus {
    const OP: &str = "sidereon_earth_radiation_pressure_moon_eclipse_enabled";
    force_query(OP, model, out, |model| {
        let model = concrete_model::<EarthRadiationPressure>(OP, model, "EarthRadiationPressure")?;
        Ok(model.moon_eclipse_enabled())
    })
}

/// Copy the force with the specified lunar occultation setting.
///
/// Safety: model is live and out is disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_earth_radiation_pressure_with_moon_eclipse(
    model: *const SidereonForceModel,
    enabled: bool,
    out: *mut *mut SidereonForceModel,
) -> SidereonStatus {
    const OP: &str = "sidereon_earth_radiation_pressure_with_moon_eclipse";
    copy_concrete_force(OP, model, out, |model| {
        let model = concrete_model::<EarthRadiationPressure>(OP, model, "EarthRadiationPressure")?;
        Ok(Arc::new((*model).with_moon_eclipse(enabled)))
    })
}

/// Read the ECOM1 coefficients.
///
/// Safety: model is live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom1_solar_radiation_pressure_coefficients(
    model: *const SidereonForceModel,
    out: *mut SidereonEcom1Coefficients,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom1_solar_radiation_pressure_coefficients";
    force_query(OP, model, out, |model| {
        let model = concrete_model::<Ecom1SolarRadiationPressure>(
            OP,
            model,
            "Ecom1SolarRadiationPressure",
        )?;
        Ok(model.coefficients().into())
    })
}

/// Read the ECOM2 D-series order.
///
/// Safety: model is live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom2_solar_radiation_pressure_n_d(
    model: *const SidereonForceModel,
    out: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom2_solar_radiation_pressure_n_d";
    force_query(OP, model, out, |model| {
        let model = concrete_model::<Ecom2SolarRadiationPressure>(
            OP,
            model,
            "Ecom2SolarRadiationPressure",
        )?;
        Ok(model.n_d())
    })
}

/// Read the ECOM2 B-series order.
///
/// Safety: model is live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom2_solar_radiation_pressure_n_b(
    model: *const SidereonForceModel,
    out: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom2_solar_radiation_pressure_n_b";
    force_query(OP, model, out, |model| {
        let model = concrete_model::<Ecom2SolarRadiationPressure>(
            OP,
            model,
            "Ecom2SolarRadiationPressure",
        )?;
        Ok(model.n_b())
    })
}

/// Copy ECOM2 coefficient values in its canonical series order.
/// NULL and zero capacity query the full coefficient count.
///
/// Safety: model is live. Outputs are writable and disjoint from each other
/// and the handle. out has capacity doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom2_solar_radiation_pressure_coefficients(
    model: *const SidereonForceModel,
    out: *mut f64,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom2_solar_radiation_pressure_coefficients";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        c_try!(init_handle_copy_counts(
            OP,
            out,
            capacity,
            out_written,
            out_required,
            model,
            "model"
        ));
        let model = c_try!(require_ref(model, OP, "model"));
        let model = c_try!(concrete_model::<Ecom2SolarRadiationPressure>(
            OP,
            model.inner.as_ref(),
            "ECOM2"
        ));
        c_try!(copy_prefix_to_c(
            OP,
            "out",
            model.coefficients(),
            out,
            capacity,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Copy one ECOM2 parameter name by canonical coefficient index. The UTF-8
/// name has no terminator. NULL and zero capacity query the full byte count.
///
/// Safety: outputs are writable and disjoint. out_name has capacity bytes.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom2_solar_radiation_pressure_parameter_name(
    n_d: usize,
    n_b: usize,
    index: usize,
    out_name: *mut u8,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom2_solar_radiation_pressure_parameter_name";
    engine_error_operation_boundary(OP, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(OP, out_written, out_required));
        let names = c_try!(Ecom2SolarRadiationPressure::parameter_names(n_d, n_b)
            .map_err(|err| map_propagation_error(OP, err)));
        let Some(name) = names.get(index) else {
            return extra_invalid_arg(OP, format_args!("parameter index {index} out of range"));
        };
        c_try!(copy_prefix_to_c(
            OP,
            "out_name",
            name.as_bytes(),
            out_name,
            capacity,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Copy an empirical RTN model with one coefficient set or replaced.
///
/// Safety: model is live and out is disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_empirical_rtn_acceleration_with_term(
    model: *const SidereonForceModel,
    term: SidereonEmpiricalRtnTerm,
    out: *mut *mut SidereonForceModel,
) -> SidereonStatus {
    const OP: &str = "sidereon_empirical_rtn_acceleration_with_term";
    copy_concrete_force(OP, model, out, |model| {
        let model = concrete_model::<EmpiricalRtnAcceleration>(OP, model, "empirical RTN")?;
        let (axis, harmonic) = rtn_term_from_c(OP, term.axis, term.harmonic)?;
        let result = model
            .with_term(axis, harmonic, term.coefficient_km_s2)
            .map_err(|err| map_propagation_error(OP, err))?;
        Ok(Arc::new(result))
    })
}

/// Copy the active empirical RTN coefficients in canonical axis and harmonic order.
/// NULL and zero capacity query the full term count.
///
/// Safety: model is live. Outputs are writable and disjoint from each other
/// and the handle. out has capacity terms.
#[no_mangle]
pub unsafe extern "C" fn sidereon_empirical_rtn_acceleration_terms(
    model: *const SidereonForceModel,
    out: *mut SidereonEmpiricalRtnTerm,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_empirical_rtn_acceleration_terms";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        c_try!(init_handle_copy_counts(
            OP,
            out,
            capacity,
            out_written,
            out_required,
            model,
            "model"
        ));
        let model = c_try!(require_ref(model, OP, "model"));
        let model = c_try!(concrete_model::<EmpiricalRtnAcceleration>(
            OP,
            model.inner.as_ref(),
            "empirical RTN"
        ));
        let terms: Vec<_> = model
            .terms()
            .into_iter()
            .map(
                |(axis, harmonic, coefficient_km_s2)| SidereonEmpiricalRtnTerm {
                    axis: match axis {
                        RtnAxis::Radial => 0,
                        RtnAxis::AlongTrack => 1,
                        RtnAxis::CrossTrack => 2,
                    },
                    harmonic: match harmonic {
                        EmpiricalHarmonic::Constant => 0,
                        EmpiricalHarmonic::Cosine => 1,
                        EmpiricalHarmonic::Sine => 2,
                    },
                    coefficient_km_s2,
                },
            )
            .collect();
        c_try!(copy_prefix_to_c(
            OP,
            "out",
            &terms,
            out,
            capacity,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Read an active empirical RTN coefficient in km/s².
///
/// Safety: model is live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_empirical_rtn_acceleration_coefficient_km_s2(
    model: *const SidereonForceModel,
    axis: u32,
    harmonic: u32,
    out: *mut SidereonOptionalOrbitScalar,
) -> SidereonStatus {
    const OP: &str = "sidereon_empirical_rtn_acceleration_coefficient_km_s2";
    force_query(OP, model, out, |model| {
        let model = concrete_model::<EmpiricalRtnAcceleration>(OP, model, "empirical RTN")?;
        let (axis, harmonic) = rtn_term_from_c(OP, axis, harmonic)?;
        Ok(model.coefficient_km_s2(axis, harmonic).into())
    })
}

/// Compare two Ecom1SolarRadiationPressure models by value.
///
/// Safety: both models are live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom1_solar_radiation_pressure_equal(
    left: *const SidereonForceModel,
    right: *const SidereonForceModel,
    out: *mut bool,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom1_solar_radiation_pressure_equal";
    force_query(OP, left, out, |left| {
        reject_output_overlaps_handle(OP, out, "out", right, "right")?;
        let right = require_ref(right, OP, "right")?;
        let left =
            concrete_model::<Ecom1SolarRadiationPressure>(OP, left, "Ecom1SolarRadiationPressure")?;
        let right = concrete_model::<Ecom1SolarRadiationPressure>(
            OP,
            right.inner.as_ref(),
            "Ecom1SolarRadiationPressure",
        )?;
        Ok(left == right)
    })
}

/// Compare two Ecom2SolarRadiationPressure models by value.
///
/// Safety: both models are live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom2_solar_radiation_pressure_equal(
    left: *const SidereonForceModel,
    right: *const SidereonForceModel,
    out: *mut bool,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom2_solar_radiation_pressure_equal";
    force_query(OP, left, out, |left| {
        reject_output_overlaps_handle(OP, out, "out", right, "right")?;
        let right = require_ref(right, OP, "right")?;
        let left =
            concrete_model::<Ecom2SolarRadiationPressure>(OP, left, "Ecom2SolarRadiationPressure")?;
        let right = concrete_model::<Ecom2SolarRadiationPressure>(
            OP,
            right.inner.as_ref(),
            "Ecom2SolarRadiationPressure",
        )?;
        Ok(left == right)
    })
}

/// Compare two EmpiricalRtnAcceleration models by value.
///
/// Safety: both models are live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_empirical_rtn_acceleration_equal(
    left: *const SidereonForceModel,
    right: *const SidereonForceModel,
    out: *mut bool,
) -> SidereonStatus {
    const OP: &str = "sidereon_empirical_rtn_acceleration_equal";
    force_query(OP, left, out, |left| {
        reject_output_overlaps_handle(OP, out, "out", right, "right")?;
        let right = require_ref(right, OP, "right")?;
        let left =
            concrete_model::<EmpiricalRtnAcceleration>(OP, left, "EmpiricalRtnAcceleration")?;
        let right = concrete_model::<EmpiricalRtnAcceleration>(
            OP,
            right.inner.as_ref(),
            "EmpiricalRtnAcceleration",
        )?;
        Ok(left == right)
    })
}

/// Read the radiation pressure coefficient.
///
/// Safety: model is live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solar_radiation_pressure_cr(
    model: *const SidereonForceModel,
    out: *mut f64,
) -> SidereonStatus {
    const OP: &str = "sidereon_solar_radiation_pressure_cr";
    force_query(OP, model, out, |model| {
        let model =
            concrete_model::<SolarRadiationPressure>(OP, model, "solar radiation pressure")?;
        Ok(model.cr())
    })
}

/// Read the astronomical unit in kilometres.
///
/// Safety: model is live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solar_radiation_pressure_au_km(
    model: *const SidereonForceModel,
    out: *mut f64,
) -> SidereonStatus {
    const OP: &str = "sidereon_solar_radiation_pressure_au_km";
    force_query(OP, model, out, |model| {
        let model =
            concrete_model::<SolarRadiationPressure>(OP, model, "solar radiation pressure")?;
        Ok(model.au_km())
    })
}

/// Read the area to mass ratio in m²/kg.
///
/// Safety: model is live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solar_radiation_pressure_area_to_mass_m2_kg(
    model: *const SidereonForceModel,
    out: *mut f64,
) -> SidereonStatus {
    const OP: &str = "sidereon_solar_radiation_pressure_area_to_mass_m2_kg";
    force_query(OP, model, out, |model| {
        let model =
            concrete_model::<SolarRadiationPressure>(OP, model, "solar radiation pressure")?;
        Ok(model.area_to_mass_m2_kg())
    })
}

/// Copy one coefficient name by canonical index without a terminator.
/// NULL and zero capacity query the full byte count.
///
/// Safety: outputs are writable and disjoint. out_name has capacity bytes.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom1_coefficients_names(
    index: usize,
    out_name: *mut u8,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom1_coefficients_names";
    copy_radiation_name(
        OP,
        &Ecom1Coefficients::NAMES,
        index,
        out_name,
        capacity,
        out_written,
        out_required,
    )
}

/// Copy one force parameter name by canonical index without a terminator.
/// NULL and zero capacity query the full byte count.
///
/// Safety: outputs are writable and disjoint. out_name has capacity bytes.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom1_coefficients_parameter_names(
    index: usize,
    out_name: *mut u8,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom1_coefficients_parameter_names";
    copy_radiation_name(
        OP,
        &Ecom1Coefficients::PARAMETER_NAMES,
        index,
        out_name,
        capacity,
        out_written,
        out_required,
    )
}

/// Copy one coefficient name by canonical index without a terminator.
/// NULL and zero capacity query the full byte count.
///
/// Safety: outputs are writable and disjoint. out_name has capacity bytes.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom2_d2b1_coefficients_names(
    index: usize,
    out_name: *mut u8,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom2_d2b1_coefficients_names";
    copy_radiation_name(
        OP,
        &Ecom2D2b1Coefficients::NAMES,
        index,
        out_name,
        capacity,
        out_written,
        out_required,
    )
}

/// Copy one force parameter name by canonical index without a terminator.
/// NULL and zero capacity query the full byte count.
///
/// Safety: outputs are writable and disjoint. out_name has capacity bytes.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom2_d2b1_coefficients_parameter_names(
    index: usize,
    out_name: *mut u8,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom2_d2b1_coefficients_parameter_names";
    copy_radiation_name(
        OP,
        &Ecom2D2b1Coefficients::PARAMETER_NAMES,
        index,
        out_name,
        capacity,
        out_written,
        out_required,
    )
}

/// Copy one coefficient name by canonical index without a terminator.
/// NULL and zero capacity query the full byte count.
///
/// Safety: outputs are writable and disjoint. out_name has capacity bytes.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom2_d4b1_coefficients_names(
    index: usize,
    out_name: *mut u8,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom2_d4b1_coefficients_names";
    copy_radiation_name(
        OP,
        &Ecom2D4b1Coefficients::NAMES,
        index,
        out_name,
        capacity,
        out_written,
        out_required,
    )
}

/// Copy one force parameter name by canonical index without a terminator.
/// NULL and zero capacity query the full byte count.
///
/// Safety: outputs are writable and disjoint. out_name has capacity bytes.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecom2_d4b1_coefficients_parameter_names(
    index: usize,
    out_name: *mut u8,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_ecom2_d4b1_coefficients_parameter_names";
    copy_radiation_name(
        OP,
        &Ecom2D4b1Coefficients::PARAMETER_NAMES,
        index,
        out_name,
        capacity,
        out_written,
        out_required,
    )
}

unsafe fn copy_radiation_name(
    operation: &str,
    names: &[&str],
    index: usize,
    out_name: *mut u8,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(operation, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(operation, out_written, out_required));
        let Some(name) = names.get(index) else {
            return extra_invalid_arg(operation, format_args!("name index {index} out of range"));
        };
        c_try!(copy_prefix_to_c(
            operation,
            "out_name",
            name.as_bytes(),
            out_name,
            capacity,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Copy the solar radiation pressure coefficient's parameter name without a
/// terminator. NULL and zero capacity query the full byte count.
///
/// Safety: outputs are writable and disjoint. out_name has capacity bytes.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solar_radiation_pressure_cr_parameter(
    out_name: *mut u8,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    copy_radiation_name(
        "sidereon_solar_radiation_pressure_cr_parameter",
        &[SolarRadiationPressure::CR_PARAMETER],
        0,
        out_name,
        capacity,
        out_written,
        out_required,
    )
}

/// Empirical RTN term selected by axis and harmonic.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonEmpiricalRtnTermSelector {
    /// Axis as SidereonRtnAxis.
    pub axis: u32,
    /// Harmonic as SidereonEmpiricalHarmonic.
    pub harmonic: u32,
}

/// Create an empirical RTN model with the selected coefficients set to zero.
///
/// Safety: terms points to count entries and out is disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_empirical_rtn_acceleration_with_zero_terms(
    terms: *const SidereonEmpiricalRtnTermSelector,
    count: usize,
    out: *mut *mut SidereonForceModel,
) -> SidereonStatus {
    const OP: &str = "sidereon_empirical_rtn_acceleration_with_zero_terms";
    write_force_model(OP, out, || {
        let terms = require_slice(terms, count, OP, "terms")?
            .iter()
            .map(|term| rtn_term_from_c(OP, term.axis, term.harmonic))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Arc::new(EmpiricalRtnAcceleration::with_zero_terms(&terms)))
    })
}

/// Create geocentric Schwarzschild relativity with an explicit gravity constant
/// and speed of light.
///
/// Safety: out points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_schwarzschild_relativity_new(
    mu_km3_s2: f64,
    c_km_s: f64,
    out: *mut *mut SidereonForceModel,
) -> SidereonStatus {
    write_force_model("sidereon_schwarzschild_relativity_new", out, || {
        Ok(Arc::new(SchwarzschildRelativity::new(mu_km3_s2, c_km_s)))
    })
}

/// Create zonal gravity through the requested degree with the supplied tide system.
///
/// Safety: out points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_zonal_gravity_new(
    mu_km3_s2: f64,
    reference_radius_km: f64,
    max_degree: u8,
    tide_system: u32,
    out: *mut *mut SidereonForceModel,
) -> SidereonStatus {
    const OP: &str = "sidereon_zonal_gravity_new";
    write_force_model(OP, out, || {
        let degrees =
            ZonalDegrees::through(max_degree).map_err(|err| map_propagation_error(OP, err))?;
        let tide_system = gravity_tide_system_from_c(OP, tide_system)?;
        let coefficients = ZonalCoefficients {
            tide_system,
            ..ZonalCoefficients::default()
        };
        Ok(Arc::new(ZonalGravity::new(
            mu_km3_s2,
            reference_radius_km,
            degrees,
            coefficients,
        )))
    })
}

/// Create the solid Earth tide gravity model for the specified static tide system.
///
/// Safety: out points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solid_earth_tide_gravity_new(
    tide_system: u32,
    out: *mut *mut SidereonForceModel,
) -> SidereonStatus {
    const OP: &str = "sidereon_solid_earth_tide_gravity_new";
    write_force_model(OP, out, || {
        let tide_system = gravity_tide_system_from_c(OP, tide_system)?;
        Ok(Arc::new(SolidEarthTideGravity {
            tide_system,
            ..SolidEarthTideGravity::default()
        }))
    })
}

/// Create the solid Earth pole tide gravity model with the default constants.
///
/// Safety: out points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solid_earth_pole_tide_gravity_new(
    out: *mut *mut SidereonForceModel,
) -> SidereonStatus {
    write_force_model("sidereon_solid_earth_pole_tide_gravity_new", out, || {
        Ok(Arc::new(SolidEarthPoleTideGravity::default()))
    })
}

/// Copy all RTN axes in radial, along-track, cross-track order.
///
/// Safety: out points to three writable axis codes.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtn_axis_all(out: *mut SidereonRtnAxis) -> SidereonStatus {
    const OP: &str = "sidereon_rtn_axis_all";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out.cast::<[SidereonRtnAxis; 3]>(), OP, "out"));
        out.write(RtnAxis::ALL.map(|axis| match axis {
            RtnAxis::Radial => SidereonRtnAxis::Radial,
            RtnAxis::AlongTrack => SidereonRtnAxis::AlongTrack,
            RtnAxis::CrossTrack => SidereonRtnAxis::CrossTrack,
        }));
        SidereonStatus::Ok
    })
}

/// Copy all empirical harmonics in constant, cosine, sine order.
///
/// Safety: out points to three writable harmonic codes.
#[no_mangle]
pub unsafe extern "C" fn sidereon_empirical_harmonic_all(
    out: *mut SidereonEmpiricalHarmonic,
) -> SidereonStatus {
    const OP: &str = "sidereon_empirical_harmonic_all";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(
            out.cast::<[SidereonEmpiricalHarmonic; 3]>(),
            OP,
            "out"
        ));
        out.write(EmpiricalHarmonic::ALL.map(|harmonic| match harmonic {
            EmpiricalHarmonic::Constant => SidereonEmpiricalHarmonic::Constant,
            EmpiricalHarmonic::Cosine => SidereonEmpiricalHarmonic::Cosine,
            EmpiricalHarmonic::Sine => SidereonEmpiricalHarmonic::Sine,
        }));
        SidereonStatus::Ok
    })
}
