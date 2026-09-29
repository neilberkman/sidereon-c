use super::*;
use crate::engine_error::{
    engine_error_operation_boundary, engine_f64, record_engine_error, SidereonEngineErrorFamily,
};

const FRAME_PROVENANCE_C_BYTES: usize = 129;

/// Terrestrial reference-frame realization selector.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonTerrestrialFrame {
    /// ITRF2020.
    Itrf2020 = 0,
    /// ITRF2014.
    Itrf2014 = 1,
    /// ITRF2008.
    Itrf2008 = 2,
    /// ETRF2020.
    Etrf2020 = 3,
}

/// Cartesian terrestrial position in meters.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTerrestrialPosition {
    /// Position components [x, y, z], meters.
    pub position_m: [f64; 3],
}

/// Cartesian terrestrial station velocity in meters per year.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTerrestrialVelocity {
    /// Velocity components [vx, vy, vz], meters per year.
    pub velocity_m_per_year: [f64; 3],
}

/// Cartesian terrestrial state with optional station velocity.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTerrestrialState {
    /// Terrestrial position.
    pub position: SidereonTerrestrialPosition,
    /// Whether velocity is present.
    pub has_velocity: bool,
    /// Terrestrial station velocity when present.
    pub velocity: SidereonTerrestrialVelocity,
}

/// Helmert parameters in published table units.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonHelmertParameters {
    /// Translation components [Tx, Ty, Tz], millimeters.
    pub translation_mm: [f64; 3],
    /// Scale difference, parts per billion.
    pub scale_ppb: f64,
    /// Rotation components [Rx, Ry, Rz], milliarcseconds.
    pub rotation_mas: [f64; 3],
}

/// Helmert parameter rates in published table units.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonHelmertRates {
    /// Translation rates [Tx, Ty, Tz], millimeters per year.
    pub translation_mm_per_year: [f64; 3],
    /// Scale rate, parts per billion per year.
    pub scale_ppb_per_year: f64,
    /// Rotation rates [Rx, Ry, Rz], milliarcseconds per year.
    pub rotation_mas_per_year: [f64; 3],
}

/// Published terrestrial-frame Helmert transform.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonHelmertTransform {
    /// Source frame as SidereonTerrestrialFrame.
    pub from: u32,
    /// Target frame as SidereonTerrestrialFrame.
    pub to: u32,
    /// Parameter reference epoch, decimal year.
    pub reference_epoch_year: f64,
    /// Helmert parameters at the reference epoch.
    pub parameters: SidereonHelmertParameters,
    /// Linear rates of the Helmert parameters.
    pub rates: SidereonHelmertRates,
    /// Null-terminated provenance string.
    pub provenance: [c_char; 129],
}

/// Write the number of built-in terrestrial frame catalog entries to
/// *out_count.
///
/// Safety: out_count must point to writable size_t storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_frame_catalog_count(out_count: *mut usize) -> SidereonStatus {
    ffi_boundary(
        "sidereon_frame_catalog_count",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_count,
                "sidereon_frame_catalog_count",
                "out_count"
            ));
            *out = sidereon_core::frame_catalog::catalog().len();
            SidereonStatus::Ok
        },
    )
}

/// Copy built-in terrestrial frame catalog entries.
///
/// Safety: out must point to len writable entries or be NULL when len is 0;
/// out_written and out_required must point to size_t storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_frame_catalog_entries(
    out: *mut SidereonHelmertTransform,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_frame_catalog_entries",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_frame_catalog_entries",
                out_written,
                out_required
            ));
            let entries: Vec<_> = sidereon_core::frame_catalog::catalog()
                .iter()
                .map(helmert_transform_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_frame_catalog_entries",
                "out",
                &entries,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy one published catalog entry for a forward frame pair.
///
/// Safety: out_transform must point to writable SidereonHelmertTransform
/// storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_frame_catalog_entry(
    from: u32,
    to: u32,
    out_transform: *mut SidereonHelmertTransform,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_frame_catalog_entry",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_transform,
                "sidereon_frame_catalog_entry",
                "out_transform"
            ));
            *out = zero_helmert_transform();
            let from = c_try!(terrestrial_frame_from_c(
                "sidereon_frame_catalog_entry",
                "from",
                from
            ));
            let to = c_try!(terrestrial_frame_from_c(
                "sidereon_frame_catalog_entry",
                "to",
                to
            ));
            match sidereon_core::frame_catalog::catalog_entry(from, to) {
                Some(transform) => {
                    *out = helmert_transform_to_c(transform);
                    SidereonStatus::Ok
                }
                None => {
                    set_last_error(format!(
                        "sidereon_frame_catalog_entry: no catalog entry for {from} to {to}"
                    ));
                    SidereonStatus::InvalidArgument
                }
            }
        },
    )
}

/// Propagate a terrestrial station position between decimal-year epochs.
///
/// Safety: position, velocity, and out_position must point to their documented
/// storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_frame_catalog_propagate_position(
    position: *const SidereonTerrestrialPosition,
    velocity: *const SidereonTerrestrialVelocity,
    from_epoch_year: f64,
    to_epoch_year: f64,
    out_position: *mut SidereonTerrestrialPosition,
) -> SidereonStatus {
    engine_error_operation_boundary(
        "sidereon_frame_catalog_propagate_position",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_position,
                "sidereon_frame_catalog_propagate_position",
                "out_position"
            ));
            *out = zero_terrestrial_position();
            let position = c_try!(terrestrial_position_from_c(
                "sidereon_frame_catalog_propagate_position",
                position
            ));
            let velocity = c_try!(terrestrial_velocity_from_c(
                "sidereon_frame_catalog_propagate_position",
                velocity
            ));
            match sidereon_core::frame_catalog::propagate_position(
                position,
                velocity,
                from_epoch_year,
                to_epoch_year,
            ) {
                Ok(position) => {
                    *out = terrestrial_position_to_c(position);
                    SidereonStatus::Ok
                }
                Err(err) => {
                    map_frame_catalog_error("sidereon_frame_catalog_propagate_position", err)
                }
            }
        },
    )
}

/// Transform a terrestrial position and optional velocity between frames.
///
/// Safety: state and out_state must point to their documented storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_frame_catalog_transform(
    state: *const SidereonTerrestrialState,
    from: u32,
    to: u32,
    epoch_year: f64,
    out_state: *mut SidereonTerrestrialState,
) -> SidereonStatus {
    engine_error_operation_boundary(
        "sidereon_frame_catalog_transform",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_state,
                "sidereon_frame_catalog_transform",
                "out_state"
            ));
            *out = zero_terrestrial_state();
            let state = c_try!(require_ref(
                state,
                "sidereon_frame_catalog_transform",
                "state"
            ));
            let position = c_try!(position_value_from_c(
                "sidereon_frame_catalog_transform",
                state.position
            ));
            let velocity = if state.has_velocity {
                Some(c_try!(velocity_value_from_c(
                    "sidereon_frame_catalog_transform",
                    state.velocity
                )))
            } else {
                None
            };
            let from = c_try!(terrestrial_frame_from_c(
                "sidereon_frame_catalog_transform",
                "from",
                from
            ));
            let to = c_try!(terrestrial_frame_from_c(
                "sidereon_frame_catalog_transform",
                "to",
                to
            ));
            match sidereon_core::frame_catalog::transform(position, velocity, from, to, epoch_year)
            {
                Ok(state) => {
                    *out = terrestrial_state_to_c(state);
                    SidereonStatus::Ok
                }
                Err(err) => map_frame_catalog_error("sidereon_frame_catalog_transform", err),
            }
        },
    )
}

/// Propagate a station to a transform epoch, then transform it between frames.
///
/// Safety: position, velocity, and out_state must point to their documented
/// storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_frame_catalog_transform_from_epoch(
    position: *const SidereonTerrestrialPosition,
    velocity: *const SidereonTerrestrialVelocity,
    position_epoch_year: f64,
    from: u32,
    to: u32,
    transform_epoch_year: f64,
    out_state: *mut SidereonTerrestrialState,
) -> SidereonStatus {
    engine_error_operation_boundary(
        "sidereon_frame_catalog_transform_from_epoch",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_state,
                "sidereon_frame_catalog_transform_from_epoch",
                "out_state"
            ));
            *out = zero_terrestrial_state();
            let position = c_try!(terrestrial_position_from_c(
                "sidereon_frame_catalog_transform_from_epoch",
                position
            ));
            let velocity = c_try!(terrestrial_velocity_from_c(
                "sidereon_frame_catalog_transform_from_epoch",
                velocity
            ));
            let from = c_try!(terrestrial_frame_from_c(
                "sidereon_frame_catalog_transform_from_epoch",
                "from",
                from
            ));
            let to = c_try!(terrestrial_frame_from_c(
                "sidereon_frame_catalog_transform_from_epoch",
                "to",
                to
            ));
            match sidereon_core::frame_catalog::transform_from_epoch(
                position,
                velocity,
                position_epoch_year,
                from,
                to,
                transform_epoch_year,
            ) {
                Ok(state) => {
                    *out = terrestrial_state_to_c(state);
                    SidereonStatus::Ok
                }
                Err(err) => {
                    map_frame_catalog_error("sidereon_frame_catalog_transform_from_epoch", err)
                }
            }
        },
    )
}

fn terrestrial_frame_from_c(
    fn_name: &str,
    arg_name: &str,
    value: u32,
) -> Result<sidereon_core::frame_catalog::TerrestrialFrame, SidereonStatus> {
    match value {
        value if value == SidereonTerrestrialFrame::Itrf2020 as u32 => {
            Ok(sidereon_core::frame_catalog::TerrestrialFrame::Itrf2020)
        }
        value if value == SidereonTerrestrialFrame::Itrf2014 as u32 => {
            Ok(sidereon_core::frame_catalog::TerrestrialFrame::Itrf2014)
        }
        value if value == SidereonTerrestrialFrame::Itrf2008 as u32 => {
            Ok(sidereon_core::frame_catalog::TerrestrialFrame::Itrf2008)
        }
        value if value == SidereonTerrestrialFrame::Etrf2020 as u32 => {
            Ok(sidereon_core::frame_catalog::TerrestrialFrame::Etrf2020)
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid {arg_name} terrestrial frame"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn terrestrial_frame_to_c(value: sidereon_core::frame_catalog::TerrestrialFrame) -> u32 {
    match value {
        sidereon_core::frame_catalog::TerrestrialFrame::Itrf2020 => {
            SidereonTerrestrialFrame::Itrf2020 as u32
        }
        sidereon_core::frame_catalog::TerrestrialFrame::Itrf2014 => {
            SidereonTerrestrialFrame::Itrf2014 as u32
        }
        sidereon_core::frame_catalog::TerrestrialFrame::Itrf2008 => {
            SidereonTerrestrialFrame::Itrf2008 as u32
        }
        sidereon_core::frame_catalog::TerrestrialFrame::Etrf2020 => {
            SidereonTerrestrialFrame::Etrf2020 as u32
        }
    }
}

fn terrestrial_position_from_c(
    fn_name: &str,
    position: *const SidereonTerrestrialPosition,
) -> Result<sidereon_core::frame_catalog::TerrestrialPositionM, SidereonStatus> {
    let position = unsafe { require_ref(position, fn_name, "position") }?;
    position_value_from_c(fn_name, *position)
}

fn terrestrial_velocity_from_c(
    fn_name: &str,
    velocity: *const SidereonTerrestrialVelocity,
) -> Result<sidereon_core::frame_catalog::TerrestrialVelocityMPerYear, SidereonStatus> {
    let velocity = unsafe { require_ref(velocity, fn_name, "velocity") }?;
    velocity_value_from_c(fn_name, *velocity)
}

fn position_value_from_c(
    fn_name: &str,
    position: SidereonTerrestrialPosition,
) -> Result<sidereon_core::frame_catalog::TerrestrialPositionM, SidereonStatus> {
    sidereon_core::frame_catalog::TerrestrialPositionM::from_array(position.position_m).map_err(
        |err| {
            set_last_error(format!("{fn_name}: {err}"));
            SidereonStatus::InvalidArgument
        },
    )
}

fn velocity_value_from_c(
    fn_name: &str,
    velocity: SidereonTerrestrialVelocity,
) -> Result<sidereon_core::frame_catalog::TerrestrialVelocityMPerYear, SidereonStatus> {
    sidereon_core::frame_catalog::TerrestrialVelocityMPerYear::from_array(
        velocity.velocity_m_per_year,
    )
    .map_err(|err| {
        set_last_error(format!("{fn_name}: {err}"));
        SidereonStatus::InvalidArgument
    })
}

fn terrestrial_position_to_c(
    position: sidereon_core::frame_catalog::TerrestrialPositionM,
) -> SidereonTerrestrialPosition {
    SidereonTerrestrialPosition {
        position_m: position.as_array(),
    }
}

fn terrestrial_velocity_to_c(
    velocity: sidereon_core::frame_catalog::TerrestrialVelocityMPerYear,
) -> SidereonTerrestrialVelocity {
    SidereonTerrestrialVelocity {
        velocity_m_per_year: velocity.as_array(),
    }
}

fn terrestrial_state_to_c(
    state: sidereon_core::frame_catalog::TerrestrialState,
) -> SidereonTerrestrialState {
    SidereonTerrestrialState {
        position: terrestrial_position_to_c(state.position),
        has_velocity: state.velocity.is_some(),
        velocity: state
            .velocity
            .map(terrestrial_velocity_to_c)
            .unwrap_or_else(zero_terrestrial_velocity),
    }
}

fn helmert_transform_to_c(
    transform: &sidereon_core::frame_catalog::HelmertTransform,
) -> SidereonHelmertTransform {
    SidereonHelmertTransform {
        from: terrestrial_frame_to_c(transform.from),
        to: terrestrial_frame_to_c(transform.to),
        reference_epoch_year: transform.reference_epoch_year,
        parameters: SidereonHelmertParameters {
            translation_mm: transform.parameters.translation_mm,
            scale_ppb: transform.parameters.scale_ppb,
            rotation_mas: transform.parameters.rotation_mas,
        },
        rates: SidereonHelmertRates {
            translation_mm_per_year: transform.rates.translation_mm_per_year,
            scale_ppb_per_year: transform.rates.scale_ppb_per_year,
            rotation_mas_per_year: transform.rates.rotation_mas_per_year,
        },
        provenance: fixed_c_chars::<FRAME_PROVENANCE_C_BYTES>(transform.provenance),
    }
}

fn zero_terrestrial_position() -> SidereonTerrestrialPosition {
    SidereonTerrestrialPosition {
        position_m: [0.0; 3],
    }
}

fn zero_terrestrial_velocity() -> SidereonTerrestrialVelocity {
    SidereonTerrestrialVelocity {
        velocity_m_per_year: [0.0; 3],
    }
}

fn zero_terrestrial_state() -> SidereonTerrestrialState {
    SidereonTerrestrialState {
        position: zero_terrestrial_position(),
        has_velocity: false,
        velocity: zero_terrestrial_velocity(),
    }
}

fn zero_helmert_transform() -> SidereonHelmertTransform {
    SidereonHelmertTransform {
        from: 0,
        to: 0,
        reference_epoch_year: 0.0,
        parameters: SidereonHelmertParameters {
            translation_mm: [0.0; 3],
            scale_ppb: 0.0,
            rotation_mas: [0.0; 3],
        },
        rates: SidereonHelmertRates {
            translation_mm_per_year: [0.0; 3],
            scale_ppb_per_year: 0.0,
            rotation_mas_per_year: [0.0; 3],
        },
        provenance: [0; FRAME_PROVENANCE_C_BYTES],
    }
}

fn frame_catalog_node(kind: &str, fields: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "kind": kind,
        "fields": fields,
    })
}

fn terrestrial_frame_name(frame: sidereon_core::frame_catalog::TerrestrialFrame) -> &'static str {
    use sidereon_core::frame_catalog::TerrestrialFrame as F;
    match frame {
        F::Itrf2020 => "itrf2020",
        F::Itrf2014 => "itrf2014",
        F::Itrf2008 => "itrf2008",
        F::Etrf2020 => "etrf2020",
    }
}

/// Convert a `FrameCatalogError` into a structured JSON error node.
pub(crate) fn frame_catalog_error_value(
    error: &sidereon_core::frame_catalog::FrameCatalogError,
) -> serde_json::Value {
    use sidereon_core::frame_catalog::FrameCatalogError as E;
    match error {
        E::InvalidInput { field, reason } => frame_catalog_node(
            "invalid_input",
            serde_json::json!({
                "field": *field,
                "reason": *reason,
            }),
        ),
        E::NoCatalogPath { from, to } => frame_catalog_node(
            "no_catalog_path",
            serde_json::json!({
                "from": terrestrial_frame_name(*from),
                "to": terrestrial_frame_name(*to),
            }),
        ),
        E::SingularTransform {
            from,
            to,
            epoch_year,
        } => frame_catalog_node(
            "singular_transform",
            serde_json::json!({
                "from": terrestrial_frame_name(*from),
                "to": terrestrial_frame_name(*to),
                "epoch_year": engine_f64(*epoch_year),
            }),
        ),
    }
}

fn map_frame_catalog_error(
    fn_name: &str,
    err: sidereon_core::frame_catalog::FrameCatalogError,
) -> SidereonStatus {
    record_engine_error(
        SidereonEngineErrorFamily::FrameCatalog,
        fn_name,
        frame_catalog_error_value(&err),
    );
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        sidereon_core::frame_catalog::FrameCatalogError::InvalidInput { .. } => {
            SidereonStatus::InvalidArgument
        }
        sidereon_core::frame_catalog::FrameCatalogError::NoCatalogPath { .. }
        | sidereon_core::frame_catalog::FrameCatalogError::SingularTransform { .. } => {
            SidereonStatus::Solve
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_error::{
        clear_engine_error, engine_f64, sidereon_last_engine_error_info,
        sidereon_last_engine_error_payload, SidereonEngineErrorFamily, SidereonEngineErrorInfo,
    };
    use crate::SidereonStatus;
    use serde_json::Value;
    use std::ptr;

    #[test]
    fn engine_f64_exact_bits_behavior() {
        let v_zero = engine_f64(0.0);
        assert_eq!(v_zero["decimal"], "0");
        assert_eq!(v_zero["bits_hex"], "0000000000000000");

        let v_neg_zero = engine_f64(-0.0);
        assert_eq!(v_neg_zero["decimal"], "-0");
        assert_eq!(v_neg_zero["bits_hex"], "8000000000000000");

        let v_nan = engine_f64(f64::NAN);
        assert_eq!(v_nan["decimal"], "NaN");
        assert_eq!(v_nan["bits_hex"], format!("{:016x}", f64::NAN.to_bits()));

        let v_inf = engine_f64(f64::INFINITY);
        assert_eq!(v_inf["decimal"], "inf");
        assert_eq!(v_inf["bits_hex"], "7ff0000000000000");

        let v_neginf = engine_f64(f64::NEG_INFINITY);
        assert_eq!(v_neginf["decimal"], "-inf");
        assert_eq!(v_neginf["bits_hex"], "fff0000000000000");

        let v_val = engine_f64(2015.5);
        assert_eq!(v_val["decimal"], "2015.5");
        assert_eq!(v_val["bits_hex"], format!("{:016x}", (2015.5f64).to_bits()));
    }

    #[test]
    fn table_driven_frame_catalog_error_mapping() {
        use sidereon_core::frame_catalog::{FrameCatalogError as E, TerrestrialFrame as F};

        let cases: Vec<(E, &'static str)> = vec![
            (
                E::InvalidInput {
                    field: "epoch_year",
                    reason: "must be finite",
                },
                "invalid_input",
            ),
            (
                E::InvalidInput {
                    field: "position_m",
                    reason: "must be finite",
                },
                "invalid_input",
            ),
            (
                E::NoCatalogPath {
                    from: F::Itrf2020,
                    to: F::Etrf2020,
                },
                "no_catalog_path",
            ),
            (
                E::NoCatalogPath {
                    from: F::Itrf2014,
                    to: F::Itrf2008,
                },
                "no_catalog_path",
            ),
            (
                E::SingularTransform {
                    from: F::Itrf2020,
                    to: F::Itrf2014,
                    epoch_year: 2015.0,
                },
                "singular_transform",
            ),
            (
                E::SingularTransform {
                    from: F::Itrf2008,
                    to: F::Etrf2020,
                    epoch_year: -0.0,
                },
                "singular_transform",
            ),
            (
                E::SingularTransform {
                    from: F::Itrf2014,
                    to: F::Itrf2020,
                    epoch_year: 0.0,
                },
                "singular_transform",
            ),
            (
                E::SingularTransform {
                    from: F::Etrf2020,
                    to: F::Itrf2008,
                    epoch_year: f64::NAN,
                },
                "singular_transform",
            ),
            (
                E::SingularTransform {
                    from: F::Itrf2020,
                    to: F::Itrf2008,
                    epoch_year: f64::INFINITY,
                },
                "singular_transform",
            ),
            (
                E::SingularTransform {
                    from: F::Itrf2014,
                    to: F::Etrf2020,
                    epoch_year: f64::NEG_INFINITY,
                },
                "singular_transform",
            ),
        ];

        for (err, expected_kind) in cases {
            let val = frame_catalog_error_value(&err);
            assert_eq!(val["kind"], expected_kind);
            match &err {
                E::InvalidInput { field, reason } => {
                    assert_eq!(val["fields"]["field"], *field);
                    assert_eq!(val["fields"]["reason"], *reason);
                }
                E::NoCatalogPath { from, to } => {
                    assert_eq!(val["fields"]["from"], terrestrial_frame_name(*from));
                    assert_eq!(val["fields"]["to"], terrestrial_frame_name(*to));
                }
                E::SingularTransform {
                    from,
                    to,
                    epoch_year,
                } => {
                    assert_eq!(val["fields"]["from"], terrestrial_frame_name(*from));
                    assert_eq!(val["fields"]["to"], terrestrial_frame_name(*to));
                    assert_eq!(
                        val["fields"]["epoch_year"]["decimal"],
                        epoch_year.to_string()
                    );
                    assert_eq!(
                        val["fields"]["epoch_year"]["bits_hex"],
                        format!("{:016x}", epoch_year.to_bits())
                    );
                }
            }
        }
    }

    #[test]
    fn frame_catalog_public_producer_control_and_refusals() {
        clear_engine_error();

        unsafe {
            let in_state = SidereonTerrestrialState {
                position: SidereonTerrestrialPosition {
                    position_m: [6378137.0, 0.0, 0.0],
                },
                has_velocity: false,
                velocity: SidereonTerrestrialVelocity {
                    velocity_m_per_year: [0.0; 3],
                },
            };
            let mut out_state = zero_terrestrial_state();

            // Seed real existing domain refusal immediately before valid success control
            assert_eq!(
                sidereon_frame_catalog_transform(
                    &in_state,
                    SidereonTerrestrialFrame::Itrf2020 as u32,
                    SidereonTerrestrialFrame::Itrf2014 as u32,
                    f64::NAN,
                    &mut out_state,
                ),
                SidereonStatus::InvalidArgument
            );

            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::FrameCatalog);
            assert!(info.payload_len > 0);

            // Valid success control clears TLS error
            assert_eq!(
                sidereon_frame_catalog_transform(
                    &in_state,
                    SidereonTerrestrialFrame::Itrf2020 as u32,
                    SidereonTerrestrialFrame::Itrf2014 as u32,
                    2020.0,
                    &mut out_state,
                ),
                SidereonStatus::Ok
            );
            assert_ne!(out_state.position.position_m, [0.0; 3]);

            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::None);
            assert_eq!(info.payload_len, 0);

            // Non-producing reader retains clean state
            let mut count = 0;
            assert_eq!(sidereon_frame_catalog_count(&mut count), SidereonStatus::Ok);
            assert!(count > 0);
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::None);
            assert_eq!(info.payload_len, 0);
        }

        // Real public refusal: non-finite epoch in sidereon_frame_catalog_transform
        unsafe {
            let in_state = SidereonTerrestrialState {
                position: SidereonTerrestrialPosition {
                    position_m: [6378137.0, 0.0, 0.0],
                },
                has_velocity: false,
                velocity: SidereonTerrestrialVelocity {
                    velocity_m_per_year: [0.0; 3],
                },
            };
            let mut out_state = zero_terrestrial_state();
            assert_eq!(
                sidereon_frame_catalog_transform(
                    &in_state,
                    SidereonTerrestrialFrame::Itrf2020 as u32,
                    SidereonTerrestrialFrame::Itrf2014 as u32,
                    f64::NAN,
                    &mut out_state,
                ),
                SidereonStatus::InvalidArgument
            );

            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::FrameCatalog);
            assert!(info.payload_len > 0);

            let mut written = 0;
            let mut required = 0;
            let mut buf = vec![0u8; info.payload_len];
            assert_eq!(
                sidereon_last_engine_error_payload(
                    buf.as_mut_ptr(),
                    buf.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, info.payload_len);
            let payload: Value = serde_json::from_slice(&buf).expect("valid JSON");
            assert_eq!(payload["schema_version"], 1);
            assert_eq!(payload["family"], "frame_catalog");
            assert_eq!(payload["operation"], "sidereon_frame_catalog_transform");
            assert_eq!(payload["error"]["kind"], "invalid_input");
            assert_eq!(payload["error"]["fields"]["field"], "epoch_year");
            assert_eq!(payload["error"]["fields"]["reason"], "must be finite");
        }
    }

    #[test]
    fn frame_catalog_producer_early_clearing_and_retention() {
        clear_engine_error();

        unsafe {
            // Seed error via a real refusal
            let in_state = SidereonTerrestrialState {
                position: SidereonTerrestrialPosition {
                    position_m: [6378137.0, 0.0, 0.0],
                },
                has_velocity: false,
                velocity: SidereonTerrestrialVelocity {
                    velocity_m_per_year: [0.0; 3],
                },
            };
            let mut out_state = zero_terrestrial_state();
            assert_eq!(
                sidereon_frame_catalog_transform(
                    &in_state,
                    SidereonTerrestrialFrame::Itrf2020 as u32,
                    SidereonTerrestrialFrame::Itrf2014 as u32,
                    f64::NAN,
                    &mut out_state,
                ),
                SidereonStatus::InvalidArgument
            );

            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::FrameCatalog);
            let expected_len = info.payload_len;
            assert!(expected_len > 0);

            // Inspection calls retain error: sidereon_frame_catalog_count
            let mut count = 0;
            assert_eq!(sidereon_frame_catalog_count(&mut count), SidereonStatus::Ok);
            assert!(count > 0);
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::FrameCatalog);
            assert_eq!(info.payload_len, expected_len);

            // Inspection calls retain error: sidereon_frame_catalog_entry
            let mut transform = zero_helmert_transform();
            assert_eq!(
                sidereon_frame_catalog_entry(
                    SidereonTerrestrialFrame::Itrf2020 as u32,
                    SidereonTerrestrialFrame::Itrf2014 as u32,
                    &mut transform,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::FrameCatalog);
            assert_eq!(info.payload_len, expected_len);

            // Two-pass payload retrieval: Pass 1 query length
            let mut written = 999;
            let mut required = 0;
            assert_eq!(
                sidereon_last_engine_error_payload(ptr::null_mut(), 0, &mut written, &mut required),
                SidereonStatus::Ok
            );
            assert_eq!(written, 0);
            assert_eq!(required, expected_len);

            // Short buffer query returns InvalidArgument and retains
            let mut short_buf = vec![0u8; expected_len - 1];
            assert_eq!(
                sidereon_last_engine_error_payload(
                    short_buf.as_mut_ptr(),
                    short_buf.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::InvalidArgument
            );
            assert_eq!(written, 0);
            assert_eq!(required, expected_len);

            // Pass 2: Exact buffer query succeeds
            let mut buf = vec![0u8; expected_len];
            assert_eq!(
                sidereon_last_engine_error_payload(
                    buf.as_mut_ptr(),
                    buf.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, expected_len);
            assert_eq!(required, expected_len);

            // Slot still retains
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::FrameCatalog);

            // Producer early argument failure (null pointer) clears slot before checks!
            assert_eq!(
                sidereon_frame_catalog_transform(
                    &in_state,
                    SidereonTerrestrialFrame::Itrf2020 as u32,
                    SidereonTerrestrialFrame::Itrf2014 as u32,
                    2020.0,
                    ptr::null_mut(),
                ),
                SidereonStatus::NullPointer
            );
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::None);
            assert_eq!(info.payload_len, 0);
        }
    }
}
