use super::*;

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
    ffi_boundary(
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
    ffi_boundary(
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
    ffi_boundary(
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

fn map_frame_catalog_error(
    fn_name: &str,
    err: sidereon_core::frame_catalog::FrameCatalogError,
) -> SidereonStatus {
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
