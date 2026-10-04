use super::*;

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonStationTideConstants {
    Conventions = 0,
    IersRoutine = 1,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonStationTideValidityMode {
    Strict = 0,
    Permissive = 1,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonStationTideDegradeReason {
    None = 0,
    BeforeCoverage = 1,
    AfterCoverage = 2,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonStationTideErrorKind {
    None = 0,
    InvalidInput = 1,
    TimeScale = 2,
    FrameTransform = 3,
    SunMoon = 4,
    MissingInput = 5,
    InvalidTag = 6,
    Other = 255,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonStationTideNestedErrorKind {
    None = 0,
    InvalidInput = 1,
    OutsideCoverage = 2,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonStationTideSunMoonCause {
    None = 0,
    FrameTransform = 1,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonStationTideInputErrorKind {
    None = 0,
    Missing = 1,
    NonFinite = 2,
    NotPositive = 3,
    Negative = 4,
    OutOfRange = 5,
    FloatParse = 6,
    IntParse = 7,
    InvalidCivilDate = 8,
    InvalidCivilTime = 9,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SidereonStationTideError {
    pub kind: SidereonStationTideErrorKind,
    pub nested_kind: SidereonStationTideNestedErrorKind,
    /// Child variant when the root is `SunMoon`; distinguishes direct
    /// validation from a nested frame-transform failure.
    pub sun_moon_cause: SidereonStationTideSunMoonCause,
    pub input_kind: SidereonStationTideInputErrorKind,
    pub degrade_reason: SidereonStationTideDegradeReason,
    pub has_field: bool,
    pub has_reason: bool,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonStationTideErrorText {
    Field = 0,
    Reason = 1,
}

#[derive(Clone)]
struct StationTideErrorDetails {
    error: SidereonStationTideError,
    field: Option<String>,
    reason: Option<String>,
}

thread_local! {
    static LAST_STATION_TIDE_ERROR: std::cell::RefCell<Option<StationTideErrorDetails>> = const { std::cell::RefCell::new(None) };
    static LAST_STATION_TIDE_BATCH_ERRORS: std::cell::RefCell<Vec<Option<StationTideErrorDetails>>> = const { std::cell::RefCell::new(Vec::new()) };
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonStationTideEpoch {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: f64,
    pub has_polar_motion: bool,
    pub xp_arcsec: f64,
    pub yp_arcsec: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonStationTideOptions {
    pub solid_earth_tide: bool,
    pub pole_tide: bool,
    pub has_ocean_loading: bool,
    pub ocean_loading: SidereonOceanLoadingBlq,
    pub constants: u32,
    pub validity_mode: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonStationTideDisplacement {
    pub ecef_m: [f64; 3],
    pub has_solid_earth_tide: bool,
    pub solid_earth_tide_ecef_m: [f64; 3],
    pub has_pole_tide: bool,
    pub pole_tide_ecef_m: [f64; 3],
    pub has_ocean_loading: bool,
    pub ocean_loading_ecef_m: [f64; 3],
    pub degrade_reason: SidereonStationTideDegradeReason,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonStationTideBatchRow {
    pub status: SidereonStatus,
    pub displacement: SidereonStationTideDisplacement,
    pub error: SidereonStationTideError,
}

pub(crate) fn station_tide_error(
    fn_name: &str,
    error: sidereon_core::tides::TideError,
) -> SidereonStatus {
    let status = station_tide_status(&error);
    set_last_error(format!("{fn_name}: {error}"));
    let details = station_tide_error_details(&error);
    LAST_STATION_TIDE_ERROR.with(|slot| *slot.borrow_mut() = Some(details));
    status
}

fn station_tide_status(error: &sidereon_core::tides::TideError) -> SidereonStatus {
    use sidereon_core::astro::bodies::sun_moon::SunMoonError;
    use sidereon_core::astro::frames::transforms::FrameTransformError;
    use sidereon_core::astro::time::CoverageError;
    use sidereon_core::tides::TideError as CoreError;
    match error {
        CoreError::TimeScale(CoverageError::OutsideCoverage(_))
        | CoreError::FrameTransform(FrameTransformError::Ut1OutsideCoverage { .. })
        | CoreError::SunMoon(SunMoonError::FrameTransform(
            FrameTransformError::Ut1OutsideCoverage { .. },
        )) => SidereonStatus::Ut1OutsideCoverage,
        _ => SidereonStatus::InvalidArgument,
    }
}

fn tide_input_error_kind(
    kind: sidereon_core::tides::TideInputErrorKind,
) -> SidereonStationTideInputErrorKind {
    use sidereon_core::tides::TideInputErrorKind as Kind;
    match kind {
        Kind::Missing => SidereonStationTideInputErrorKind::Missing,
        Kind::NonFinite => SidereonStationTideInputErrorKind::NonFinite,
        Kind::NotPositive => SidereonStationTideInputErrorKind::NotPositive,
        Kind::Negative => SidereonStationTideInputErrorKind::Negative,
        Kind::OutOfRange => SidereonStationTideInputErrorKind::OutOfRange,
        Kind::FloatParse => SidereonStationTideInputErrorKind::FloatParse,
        Kind::IntParse => SidereonStationTideInputErrorKind::IntParse,
        Kind::InvalidCivilDate => SidereonStationTideInputErrorKind::InvalidCivilDate,
        Kind::InvalidCivilTime => SidereonStationTideInputErrorKind::InvalidCivilTime,
    }
}

fn timescale_input_error_kind(
    kind: sidereon_core::astro::time::TimeScaleInputErrorKind,
) -> SidereonStationTideInputErrorKind {
    use sidereon_core::astro::time::TimeScaleInputErrorKind as Kind;
    match kind {
        Kind::Missing => SidereonStationTideInputErrorKind::Missing,
        Kind::NonFinite => SidereonStationTideInputErrorKind::NonFinite,
        Kind::NotPositive => SidereonStationTideInputErrorKind::NotPositive,
        Kind::Negative => SidereonStationTideInputErrorKind::Negative,
        Kind::OutOfRange => SidereonStationTideInputErrorKind::OutOfRange,
        Kind::FloatParse => SidereonStationTideInputErrorKind::FloatParse,
        Kind::IntParse => SidereonStationTideInputErrorKind::IntParse,
        Kind::InvalidCivilDate => SidereonStationTideInputErrorKind::InvalidCivilDate,
        Kind::InvalidCivilTime => SidereonStationTideInputErrorKind::InvalidCivilTime,
    }
}

fn tide_degrade_reason(
    reason: sidereon_core::astro::time::DegradeReason,
) -> SidereonStationTideDegradeReason {
    match reason {
        sidereon_core::astro::time::DegradeReason::BeforeCoverage => {
            SidereonStationTideDegradeReason::BeforeCoverage
        }
        sidereon_core::astro::time::DegradeReason::AfterCoverage => {
            SidereonStationTideDegradeReason::AfterCoverage
        }
    }
}

fn station_tide_frame_error(
    error: &sidereon_core::astro::frames::transforms::FrameTransformError,
) -> (
    SidereonStationTideNestedErrorKind,
    Option<String>,
    Option<String>,
    SidereonStationTideDegradeReason,
) {
    use sidereon_core::astro::frames::transforms::FrameTransformError as Error;
    match error {
        Error::InvalidInput { field, reason } => (
            SidereonStationTideNestedErrorKind::InvalidInput,
            Some((*field).to_owned()),
            Some((*reason).to_owned()),
            SidereonStationTideDegradeReason::None,
        ),
        Error::Ut1OutsideCoverage { reason } => (
            SidereonStationTideNestedErrorKind::OutsideCoverage,
            None,
            None,
            tide_degrade_reason(*reason),
        ),
    }
}

fn station_tide_error_details(error: &sidereon_core::tides::TideError) -> StationTideErrorDetails {
    use sidereon_core::astro::bodies::sun_moon::SunMoonError;
    use sidereon_core::astro::time::CoverageError;
    use sidereon_core::tides::TideError as Error;

    let mut details = StationTideErrorDetails {
        error: SidereonStationTideError {
            kind: SidereonStationTideErrorKind::Other,
            nested_kind: SidereonStationTideNestedErrorKind::None,
            sun_moon_cause: SidereonStationTideSunMoonCause::None,
            input_kind: SidereonStationTideInputErrorKind::None,
            degrade_reason: SidereonStationTideDegradeReason::None,
            has_field: false,
            has_reason: false,
        },
        field: None,
        reason: None,
    };
    match error {
        Error::InvalidInput { field, kind } => {
            details.error.kind = SidereonStationTideErrorKind::InvalidInput;
            details.error.input_kind = tide_input_error_kind(*kind);
            details.field = Some((*field).to_owned());
        }
        Error::TimeScale(CoverageError::InvalidInput { field, kind }) => {
            details.error.kind = SidereonStationTideErrorKind::TimeScale;
            details.error.nested_kind = SidereonStationTideNestedErrorKind::InvalidInput;
            details.error.input_kind = timescale_input_error_kind(*kind);
            details.field = Some((*field).to_owned());
        }
        Error::TimeScale(CoverageError::OutsideCoverage(reason)) => {
            details.error.kind = SidereonStationTideErrorKind::TimeScale;
            details.error.nested_kind = SidereonStationTideNestedErrorKind::OutsideCoverage;
            details.error.degrade_reason = tide_degrade_reason(*reason);
        }
        Error::FrameTransform(error) => {
            details.error.kind = SidereonStationTideErrorKind::FrameTransform;
            let (nested, field, reason, degrade) = station_tide_frame_error(error);
            details.error.nested_kind = nested;
            details.field = field;
            details.reason = reason;
            details.error.degrade_reason = degrade;
        }
        Error::SunMoon(SunMoonError::InvalidInput { field, reason }) => {
            details.error.kind = SidereonStationTideErrorKind::SunMoon;
            details.error.nested_kind = SidereonStationTideNestedErrorKind::InvalidInput;
            details.field = Some((*field).to_owned());
            details.reason = Some((*reason).to_owned());
        }
        Error::SunMoon(SunMoonError::FrameTransform(error)) => {
            details.error.kind = SidereonStationTideErrorKind::SunMoon;
            details.error.sun_moon_cause = SidereonStationTideSunMoonCause::FrameTransform;
            let (nested, field, reason, degrade) = station_tide_frame_error(error);
            details.error.nested_kind = nested;
            details.field = field;
            details.reason = reason;
            details.error.degrade_reason = degrade;
        }
        Error::MissingInput { field } => {
            details.error.kind = SidereonStationTideErrorKind::MissingInput;
            details.field = Some((*field).to_owned());
        }
        Error::BlqParse { .. } | Error::BlqWrite { .. } => {
            details.reason = Some("not produced by station displacement evaluation".to_owned());
            details.error.has_reason = true;
        }
    }
    details.error.has_field = details.field.is_some();
    details.error.has_reason = details.reason.is_some();
    details
}

fn station_tide_constants_from_c(
    constants: u32,
) -> Option<sidereon_core::tides::StationTideConstants> {
    match constants {
        value if value == SidereonStationTideConstants::Conventions as u32 => {
            Some(sidereon_core::tides::StationTideConstants::Conventions)
        }
        value if value == SidereonStationTideConstants::IersRoutine as u32 => {
            Some(sidereon_core::tides::StationTideConstants::IersRoutine)
        }
        _ => None,
    }
}

fn station_tide_invalid_tag(fn_name: &str, field: &str, value: u32) -> SidereonStatus {
    set_last_error(format!("{fn_name}: invalid {field} tag {value}"));
    let details = StationTideErrorDetails {
        error: SidereonStationTideError {
            kind: SidereonStationTideErrorKind::InvalidTag,
            nested_kind: SidereonStationTideNestedErrorKind::None,
            sun_moon_cause: SidereonStationTideSunMoonCause::None,
            input_kind: SidereonStationTideInputErrorKind::None,
            degrade_reason: SidereonStationTideDegradeReason::None,
            has_field: true,
            has_reason: true,
        },
        field: Some(field.to_owned()),
        reason: Some(format!("unknown integer tag {value}")),
    };
    LAST_STATION_TIDE_ERROR.with(|slot| *slot.borrow_mut() = Some(details));
    SidereonStatus::InvalidArgument
}

/// Copy the last station-tide failure for this thread, or `None` if no such
/// call has failed.
///
/// # Safety
/// `out_error` must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_station_tide_last_error(
    out_error: *mut SidereonStationTideError,
) -> SidereonStatus {
    const FN: &str = "sidereon_station_tide_last_error";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_error, FN, "out_error"));
        *out = LAST_STATION_TIDE_ERROR.with(|slot| {
            slot.borrow().as_ref().map_or(
                SidereonStationTideError {
                    kind: SidereonStationTideErrorKind::None,
                    nested_kind: SidereonStationTideNestedErrorKind::None,
                    sun_moon_cause: SidereonStationTideSunMoonCause::None,
                    input_kind: SidereonStationTideInputErrorKind::None,
                    degrade_reason: SidereonStationTideDegradeReason::None,
                    has_field: false,
                    has_reason: false,
                },
                |details| details.error,
            )
        });
        SidereonStatus::Ok
    })
}

/// Copy a field or reason from the last station-tide error using the standard
/// count-query buffer convention; returned bytes are not NUL-terminated.
///
/// # Safety
/// `out_written` and `out_required` must be writable. `out` must point to
/// `len` writable bytes unless `len` is zero.
#[no_mangle]
pub unsafe extern "C" fn sidereon_station_tide_last_error_text(
    part: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN: &str = "sidereon_station_tide_last_error_text";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN, out_written, out_required));
        if part > SidereonStationTideErrorText::Reason as u32 {
            set_last_error(format!("{FN}: invalid text part {part}"));
            return SidereonStatus::InvalidArgument;
        }
        let text = LAST_STATION_TIDE_ERROR.with(|slot| {
            slot.borrow()
                .as_ref()
                .and_then(|details| match part {
                    value if value == SidereonStationTideErrorText::Field as u32 => {
                        details.field.clone()
                    }
                    value if value == SidereonStationTideErrorText::Reason as u32 => {
                        details.reason.clone()
                    }
                    _ => None,
                })
                .unwrap_or_default()
        });
        c_try!(copy_prefix_to_c(
            FN,
            "out",
            text.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Evaluate station displacement in ECEF metres using the core tide models.
/// The output marks omitted optional components as absent and reports UT1
/// extrapolation separately when permissive validity is selected.
///
/// # Safety
/// `position` must point to three readable doubles (ECEF x, y, z metres);
/// `epoch` and `options` must point to readable records and `out` to writable
/// storage. A NULL pointer is refused with SIDEREON_STATUS_NULL_POINTER.
#[no_mangle]
pub unsafe extern "C" fn sidereon_station_tide_displacement(
    position: *const f64,
    epoch: *const SidereonStationTideEpoch,
    options: *const SidereonStationTideOptions,
    out: *mut SidereonStationTideDisplacement,
) -> SidereonStatus {
    const FN: &str = "sidereon_station_tide_displacement";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, FN, "out"));
        let inputs = (|| {
            let position = *require_f64_array::<3>(position, FN, "position")?;
            let epoch = *require_ref(epoch, FN, "epoch")?;
            let options = *require_ref(options, FN, "options")?;
            Ok((position, epoch, options))
        })();
        *out = SidereonStationTideDisplacement {
            ecef_m: [0.0; 3],
            has_solid_earth_tide: false,
            solid_earth_tide_ecef_m: [0.0; 3],
            has_pole_tide: false,
            pole_tide_ecef_m: [0.0; 3],
            has_ocean_loading: false,
            ocean_loading_ecef_m: [0.0; 3],
            degrade_reason: SidereonStationTideDegradeReason::None,
        };
        let (position, epoch, options) = c_try!(inputs);
        let position =
            match sidereon_core::tides::StationDisplacementPosition::from_ecef_m(position) {
                Ok(value) => value,
                Err(error) => return station_tide_error(FN, error),
            };
        let mut epoch_value = sidereon_core::tides::StationDisplacementEpoch::from_utc(
            epoch.year,
            epoch.month,
            epoch.day,
            epoch.hour,
            epoch.minute,
            epoch.second,
        );
        if epoch.has_polar_motion {
            epoch_value = epoch_value.with_polar_motion_arcsec(epoch.xp_arcsec, epoch.yp_arcsec);
        }
        let loading = options
            .has_ocean_loading
            .then_some(sidereon_core::tides::OceanLoadingBlq {
                amplitude_m: options.ocean_loading.amplitude_m,
                phase_deg: options.ocean_loading.phase_deg,
            });
        let constants = match station_tide_constants_from_c(options.constants) {
            Some(value) => value,
            None => return station_tide_invalid_tag(FN, "constants", options.constants),
        };
        let validity = match options.validity_mode {
            value if value == SidereonStationTideValidityMode::Strict as u32 => {
                sidereon_core::astro::time::ValidityMode::Strict
            }
            value if value == SidereonStationTideValidityMode::Permissive as u32 => {
                sidereon_core::astro::time::ValidityMode::Permissive
            }
            value => return station_tide_invalid_tag(FN, "validity_mode", value),
        };
        let mut tide_options = sidereon_core::tides::StationDisplacementOptions::default();
        tide_options.solid_earth_tide = options.solid_earth_tide;
        tide_options.pole_tide = options.pole_tide;
        tide_options.ocean_loading = loading.as_ref();
        tide_options.solid_earth_tide_constants = constants;
        let result = match sidereon_core::tides::station_displacement_ecef_m_with_validity(
            position,
            epoch_value,
            tide_options,
            validity,
        ) {
            Ok(value) => value,
            Err(error) => return station_tide_error(FN, error),
        };
        let value = result.value;
        let degrade_reason = match result.degraded {
            None => SidereonStationTideDegradeReason::None,
            Some(sidereon_core::astro::time::DegradeReason::BeforeCoverage) => {
                SidereonStationTideDegradeReason::BeforeCoverage
            }
            Some(sidereon_core::astro::time::DegradeReason::AfterCoverage) => {
                SidereonStationTideDegradeReason::AfterCoverage
            }
        };
        *out = SidereonStationTideDisplacement {
            ecef_m: value.ecef_m,
            has_solid_earth_tide: value.solid_earth_tide_ecef_m.is_some(),
            solid_earth_tide_ecef_m: value.solid_earth_tide_ecef_m.unwrap_or([0.0; 3]),
            has_pole_tide: value.pole_tide_ecef_m.is_some(),
            pole_tide_ecef_m: value.pole_tide_ecef_m.unwrap_or([0.0; 3]),
            has_ocean_loading: value.ocean_loading_ecef_m.is_some(),
            ocean_loading_ecef_m: value.ocean_loading_ecef_m.unwrap_or([0.0; 3]),
            degrade_reason,
        };
        SidereonStatus::Ok
    })
}

/// Evaluate station displacement for each epoch. Per-row failures are returned
/// in `out_rows`; text fields for a failed row remain available through
/// `sidereon_station_tide_batch_error_text` until the next batch call on this
/// thread.
///
/// # Safety
/// `position` must point to three readable doubles (ECEF x, y, z metres) and
/// `options` to a readable record. `epochs` and `out_rows` must reference `count`
/// readable and writable rows when `count` is nonzero. A NULL pointer is refused with SIDEREON_STATUS_NULL_POINTER.
#[no_mangle]
pub unsafe extern "C" fn sidereon_station_tide_displacement_batch(
    position: *const f64,
    epochs: *const SidereonStationTideEpoch,
    count: usize,
    options: *const SidereonStationTideOptions,
    out_rows: *mut SidereonStationTideBatchRow,
) -> SidereonStatus {
    const FN: &str = "sidereon_station_tide_displacement_batch";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        LAST_STATION_TIDE_BATCH_ERRORS.with(|slot| slot.borrow_mut().clear());
        let position = *c_try!(require_f64_array::<3>(position, FN, "position"));
        let options = *c_try!(require_ref(options, FN, "options"));
        let epoch_rows = c_try!(require_slice(epochs, count, FN, "epochs")).to_vec();
        c_try!(require_out_array(out_rows, count, FN, "out_rows"));
        let output = if count == 0 {
            ptr::NonNull::<SidereonStationTideBatchRow>::dangling().as_ptr()
        } else {
            out_rows
        };
        // Initialize raw storage without creating a reference to possibly
        // uninitialized C memory. Only form the mutable slice after every row
        // contains valid Rust enum and bool values.
        for index in 0..count {
            output.add(index).write(SidereonStationTideBatchRow {
                status: SidereonStatus::Ok,
                displacement: SidereonStationTideDisplacement {
                    ecef_m: [0.0; 3],
                    has_solid_earth_tide: false,
                    solid_earth_tide_ecef_m: [0.0; 3],
                    has_pole_tide: false,
                    pole_tide_ecef_m: [0.0; 3],
                    has_ocean_loading: false,
                    ocean_loading_ecef_m: [0.0; 3],
                    degrade_reason: SidereonStationTideDegradeReason::None,
                },
                error: empty_station_tide_error(),
            });
        }
        let output_rows = std::slice::from_raw_parts_mut(output, count);
        let position =
            match sidereon_core::tides::StationDisplacementPosition::from_ecef_m(position) {
                Ok(value) => value,
                Err(error) => return station_tide_error(FN, error),
            };
        let loading = options
            .has_ocean_loading
            .then_some(sidereon_core::tides::OceanLoadingBlq {
                amplitude_m: options.ocean_loading.amplitude_m,
                phase_deg: options.ocean_loading.phase_deg,
            });
        let constants = match station_tide_constants_from_c(options.constants) {
            Some(value) => value,
            None => return station_tide_invalid_tag(FN, "constants", options.constants),
        };
        let validity = match options.validity_mode {
            value if value == SidereonStationTideValidityMode::Strict as u32 => {
                sidereon_core::astro::time::ValidityMode::Strict
            }
            value if value == SidereonStationTideValidityMode::Permissive as u32 => {
                sidereon_core::astro::time::ValidityMode::Permissive
            }
            value => return station_tide_invalid_tag(FN, "validity_mode", value),
        };
        let core_epochs: Vec<_> = epoch_rows
            .iter()
            .map(|epoch| {
                let mut value = sidereon_core::tides::StationDisplacementEpoch::from_utc(
                    epoch.year,
                    epoch.month,
                    epoch.day,
                    epoch.hour,
                    epoch.minute,
                    epoch.second,
                );
                if epoch.has_polar_motion {
                    value = value.with_polar_motion_arcsec(epoch.xp_arcsec, epoch.yp_arcsec);
                }
                value
            })
            .collect();
        let mut core_options = sidereon_core::tides::StationDisplacementOptions::default();
        core_options.solid_earth_tide = options.solid_earth_tide;
        core_options.pole_tide = options.pole_tide;
        core_options.ocean_loading = loading.as_ref();
        core_options.solid_earth_tide_constants = constants;
        let results = sidereon_core::tides::station_displacement_ecef_m_batch_with_validity(
            position,
            &core_epochs,
            core_options,
            validity,
        );
        let mut batch_errors = Vec::with_capacity(count);
        for (output, result) in output_rows.iter_mut().zip(results) {
            match result {
                Ok(value) => {
                    let displacement = value.value;
                    output.displacement = station_displacement_to_c(displacement, value.degraded);
                    batch_errors.push(None);
                }
                Err(error) => {
                    let details = station_tide_error_details(&error);
                    output.status = station_tide_status(&error);
                    output.error = details.error;
                    set_last_error(format!("{FN}: {error}"));
                    batch_errors.push(Some(details));
                }
            }
        }
        LAST_STATION_TIDE_BATCH_ERRORS.with(|slot| *slot.borrow_mut() = batch_errors);
        SidereonStatus::Ok
    })
}

/// Copy field or reason text for one failed batch row. The text storage is
/// thread-local and is replaced by the next batch call on this thread.
///
/// # Safety
/// `out_written` and `out_required` must be writable; `out` must reference
/// `len` writable bytes unless `len` is zero.
#[no_mangle]
pub unsafe extern "C" fn sidereon_station_tide_batch_error_text(
    row: usize,
    part: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN: &str = "sidereon_station_tide_batch_error_text";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN, out_written, out_required));
        if part > SidereonStationTideErrorText::Reason as u32 {
            set_last_error(format!("{FN}: invalid text part {part}"));
            return SidereonStatus::InvalidArgument;
        }
        let row_count = LAST_STATION_TIDE_BATCH_ERRORS.with(|slot| slot.borrow().len());
        if row >= row_count {
            set_last_error(format!(
                "{FN}: row {row} is outside the last batch of {row_count} rows"
            ));
            return SidereonStatus::InvalidArgument;
        }
        let text = LAST_STATION_TIDE_BATCH_ERRORS.with(|slot| {
            slot.borrow()
                .get(row)
                .and_then(Option::as_ref)
                .and_then(|details| match part {
                    value if value == SidereonStationTideErrorText::Field as u32 => {
                        details.field.clone()
                    }
                    value if value == SidereonStationTideErrorText::Reason as u32 => {
                        details.reason.clone()
                    }
                    _ => None,
                })
                .unwrap_or_default()
        });
        c_try!(copy_prefix_to_c(
            FN,
            "out",
            text.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

fn empty_station_tide_error() -> SidereonStationTideError {
    SidereonStationTideError {
        kind: SidereonStationTideErrorKind::None,
        nested_kind: SidereonStationTideNestedErrorKind::None,
        sun_moon_cause: SidereonStationTideSunMoonCause::None,
        input_kind: SidereonStationTideInputErrorKind::None,
        degrade_reason: SidereonStationTideDegradeReason::None,
        has_field: false,
        has_reason: false,
    }
}

fn station_displacement_to_c(
    value: sidereon_core::tides::StationDisplacement,
    degraded: Option<sidereon_core::astro::time::DegradeReason>,
) -> SidereonStationTideDisplacement {
    SidereonStationTideDisplacement {
        ecef_m: value.ecef_m,
        has_solid_earth_tide: value.solid_earth_tide_ecef_m.is_some(),
        solid_earth_tide_ecef_m: value.solid_earth_tide_ecef_m.unwrap_or([0.0; 3]),
        has_pole_tide: value.pole_tide_ecef_m.is_some(),
        pole_tide_ecef_m: value.pole_tide_ecef_m.unwrap_or([0.0; 3]),
        has_ocean_loading: value.ocean_loading_ecef_m.is_some(),
        ocean_loading_ecef_m: value.ocean_loading_ecef_m.unwrap_or([0.0; 3]),
        degrade_reason: degraded
            .map_or(SidereonStationTideDegradeReason::None, tide_degrade_reason),
    }
}

/// Return the core default station Step 2 constant set (Conventions).
#[no_mangle]
pub extern "C" fn sidereon_station_tide_constants_default() -> SidereonStationTideConstants {
    SidereonStationTideConstants::Conventions
}

/// Evaluate the core solid-Earth tide using caller-provided Sun and Moon ECEF
/// positions. `constants` is the integer value of `SidereonStationTideConstants`.
///
/// # Safety
/// `station_ecef_m`, `sun_ecef_m` and `moon_ecef_m` must each point to three
/// readable doubles and `out_displacement_ecef_m` to three writable doubles.
/// A NULL pointer is refused with SIDEREON_STATUS_NULL_POINTER.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solid_earth_tide_with_constants(
    station_ecef_m: *const f64,
    year: i32,
    month: i32,
    day: i32,
    fractional_hour: f64,
    sun_ecef_m: *const f64,
    moon_ecef_m: *const f64,
    constants: u32,
    out_displacement_ecef_m: *mut f64,
) -> SidereonStatus {
    const FN: &str = "sidereon_solid_earth_tide_with_constants";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out_f64_array::<3>(
            out_displacement_ecef_m,
            FN,
            "out_displacement_ecef_m"
        ));
        *out = [0.0; 3];
        let station = c_try!(require_f64_array::<3>(station_ecef_m, FN, "station_ecef_m"));
        let sun = c_try!(require_f64_array::<3>(sun_ecef_m, FN, "sun_ecef_m"));
        let moon = c_try!(require_f64_array::<3>(moon_ecef_m, FN, "moon_ecef_m"));
        let constants = match station_tide_constants_from_c(constants) {
            Some(value) => value,
            None => return station_tide_invalid_tag(FN, "constants", constants),
        };
        match sidereon_core::tides::solid_earth_tide_with_constants(
            station,
            year,
            month,
            day,
            fractional_hour,
            sun,
            moon,
            constants,
        ) {
            Ok(value) => {
                *out = value;
                SidereonStatus::Ok
            }
            Err(error) => station_tide_error(FN, error),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[repr(C)]
    union TidePositionOutputStorage {
        position: [f64; 3],
        output: SidereonStationTideDisplacement,
    }

    fn default_options() -> SidereonStationTideOptions {
        SidereonStationTideOptions {
            solid_earth_tide: false,
            pole_tide: false,
            has_ocean_loading: false,
            ocean_loading: SidereonOceanLoadingBlq {
                amplitude_m: [[0.0; SIDEREON_PPP_OCEAN_CONSTITUENTS]; 3],
                phase_deg: [[0.0; SIDEREON_PPP_OCEAN_CONSTITUENTS]; 3],
            },
            constants: SidereonStationTideConstants::Conventions as u32,
            validity_mode: SidereonStationTideValidityMode::Strict as u32,
        }
    }

    fn epoch() -> SidereonStationTideEpoch {
        SidereonStationTideEpoch {
            year: 2024,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0.0,
            has_polar_motion: false,
            xp_arcsec: 0.0,
            yp_arcsec: 0.0,
        }
    }

    fn empty_displacement() -> SidereonStationTideDisplacement {
        SidereonStationTideDisplacement {
            ecef_m: [f64::NAN; 3],
            has_solid_earth_tide: true,
            solid_earth_tide_ecef_m: [f64::NAN; 3],
            has_pole_tide: true,
            pole_tide_ecef_m: [f64::NAN; 3],
            has_ocean_loading: true,
            ocean_loading_ecef_m: [f64::NAN; 3],
            degrade_reason: SidereonStationTideDegradeReason::AfterCoverage,
        }
    }

    #[test]
    fn invalid_option_tags_reset_outputs_and_report_typed_field() {
        let position = [6_378_137.0, 0.0, 0.0];
        for (field, invalid_tag) in [("constants", 44), ("validity_mode", 44)] {
            let mut options = default_options();
            if field == "constants" {
                options.constants = invalid_tag;
            } else {
                options.validity_mode = invalid_tag;
            }
            let mut output = empty_displacement();
            assert_eq!(
                unsafe {
                    sidereon_station_tide_displacement(
                        position.as_ptr(),
                        &epoch(),
                        &options,
                        &mut output,
                    )
                },
                SidereonStatus::InvalidArgument
            );
            assert_eq!(output.ecef_m, [0.0; 3]);
            assert!(!output.has_solid_earth_tide);
            let mut error = SidereonStationTideError {
                kind: SidereonStationTideErrorKind::None,
                nested_kind: SidereonStationTideNestedErrorKind::None,
                sun_moon_cause: SidereonStationTideSunMoonCause::None,
                input_kind: SidereonStationTideInputErrorKind::None,
                degrade_reason: SidereonStationTideDegradeReason::None,
                has_field: false,
                has_reason: false,
            };
            assert_eq!(
                unsafe { sidereon_station_tide_last_error(&mut error) },
                SidereonStatus::Ok
            );
            assert_eq!(error.kind, SidereonStationTideErrorKind::InvalidTag);
            let details = LAST_STATION_TIDE_ERROR.with(|slot| slot.borrow().clone().unwrap());
            assert_eq!(details.field.as_deref(), Some(field));
            assert_eq!(details.reason.as_deref(), Some("unknown integer tag 44"));
            let mut text = [0u8; 32];
            let mut written = 0;
            let mut required = 0;
            assert_eq!(
                unsafe {
                    sidereon_station_tide_last_error_text(
                        SidereonStationTideErrorText::Field as u32,
                        text.as_mut_ptr(),
                        text.len(),
                        &mut written,
                        &mut required,
                    )
                },
                SidereonStatus::Ok
            );
            assert_eq!(written, field.len());
            assert_eq!(&text[..written], field.as_bytes());
        }
    }

    #[test]
    fn nested_coverage_error_remains_typed_and_intact() {
        use sidereon_core::astro::bodies::sun_moon::SunMoonError;
        use sidereon_core::astro::frames::transforms::FrameTransformError;
        use sidereon_core::astro::time::DegradeReason;

        let error = sidereon_core::tides::TideError::SunMoon(SunMoonError::FrameTransform(
            FrameTransformError::Ut1OutsideCoverage {
                reason: DegradeReason::AfterCoverage,
            },
        ));
        let details = station_tide_error_details(&error);
        assert_eq!(details.error.kind, SidereonStationTideErrorKind::SunMoon);
        assert_eq!(
            details.error.nested_kind,
            SidereonStationTideNestedErrorKind::OutsideCoverage
        );
        assert_eq!(
            details.error.sun_moon_cause,
            SidereonStationTideSunMoonCause::FrameTransform
        );
        assert_eq!(
            details.error.degrade_reason,
            SidereonStationTideDegradeReason::AfterCoverage
        );
        assert!(!details.error.has_field && !details.error.has_reason);
    }

    #[test]
    fn direct_sun_moon_input_error_is_distinct_from_nested_frame_error() {
        use sidereon_core::astro::bodies::sun_moon::SunMoonError;

        let direct = station_tide_error_details(&sidereon_core::tides::TideError::SunMoon(
            SunMoonError::InvalidInput {
                field: "jd_tt",
                reason: "not finite",
            },
        ));
        assert_eq!(direct.error.kind, SidereonStationTideErrorKind::SunMoon);
        assert_eq!(
            direct.error.nested_kind,
            SidereonStationTideNestedErrorKind::InvalidInput
        );
        assert_eq!(
            direct.error.sun_moon_cause,
            SidereonStationTideSunMoonCause::None
        );
        assert_eq!(direct.field.as_deref(), Some("jd_tt"));
        assert_eq!(direct.reason.as_deref(), Some("not finite"));

        let nested = station_tide_error_details(&sidereon_core::tides::TideError::SunMoon(
            SunMoonError::FrameTransform(
                sidereon_core::astro::frames::transforms::FrameTransformError::InvalidInput {
                    field: "jd_ut1",
                    reason: "not finite",
                },
            ),
        ));
        assert_eq!(nested.error.kind, SidereonStationTideErrorKind::SunMoon);
        assert_eq!(
            nested.error.nested_kind,
            SidereonStationTideNestedErrorKind::InvalidInput
        );
        assert_eq!(
            nested.error.sun_moon_cause,
            SidereonStationTideSunMoonCause::FrameTransform
        );
        assert_eq!(nested.field.as_deref(), Some("jd_ut1"));
        assert_eq!(nested.reason.as_deref(), Some("not finite"));
    }

    #[test]
    fn station_evaluation_returns_components_without_rewriting_disabled_modes() {
        let position = [6_378_137.0, 0.0, 0.0];
        let mut options = default_options();
        options.has_ocean_loading = true;
        let mut output = empty_displacement();
        assert_eq!(
            unsafe {
                sidereon_station_tide_displacement(
                    position.as_ptr(),
                    &epoch(),
                    &options,
                    &mut output,
                )
            },
            SidereonStatus::Ok
        );
        assert!(output.has_ocean_loading);
        assert!(!output.has_solid_earth_tide && !output.has_pole_tide);
        assert_eq!(output.ocean_loading_ecef_m, [0.0; 3]);
    }

    #[test]
    fn displacement_output_may_overlap_position_input() {
        let position = [6_378_137.0, 0.0, 0.0];
        let epoch = epoch();
        let options = default_options();
        let mut expected = empty_displacement();
        assert_eq!(
            unsafe {
                sidereon_station_tide_displacement(
                    position.as_ptr(),
                    &epoch,
                    &options,
                    &mut expected,
                )
            },
            SidereonStatus::Ok
        );

        let mut storage = TidePositionOutputStorage { position };
        let storage_ptr = &mut storage as *mut TidePositionOutputStorage;
        let position_ptr = unsafe { std::ptr::addr_of!((*storage_ptr).position).cast::<f64>() };
        let output_ptr = unsafe { std::ptr::addr_of_mut!((*storage_ptr).output) };
        assert_eq!(
            unsafe {
                sidereon_station_tide_displacement(position_ptr, &epoch, &options, output_ptr)
            },
            SidereonStatus::Ok
        );
        let actual = unsafe { output_ptr.read() };
        assert_eq!(actual.ecef_m, expected.ecef_m);
        assert_eq!(actual.has_solid_earth_tide, expected.has_solid_earth_tide);
        assert_eq!(
            actual.solid_earth_tide_ecef_m,
            expected.solid_earth_tide_ecef_m
        );
        assert_eq!(actual.has_pole_tide, expected.has_pole_tide);
        assert_eq!(actual.pole_tide_ecef_m, expected.pole_tide_ecef_m);
        assert_eq!(actual.has_ocean_loading, expected.has_ocean_loading);
        assert_eq!(actual.ocean_loading_ecef_m, expected.ocean_loading_ecef_m);
        assert_eq!(actual.degrade_reason, expected.degrade_reason);
    }

    #[test]
    fn batch_keeps_per_epoch_typed_failure_and_success_rows() {
        let position = [6_378_137.0, 0.0, 0.0];
        let mut valid_epoch = epoch();
        valid_epoch.year = 2024;
        let mut invalid_epoch = epoch();
        invalid_epoch.month = 13;
        let epochs = [valid_epoch, invalid_epoch];
        let options = default_options();
        let mut rows = [
            SidereonStationTideBatchRow {
                status: SidereonStatus::Panic,
                displacement: empty_displacement(),
                error: empty_station_tide_error(),
            },
            SidereonStationTideBatchRow {
                status: SidereonStatus::Panic,
                displacement: empty_displacement(),
                error: empty_station_tide_error(),
            },
        ];
        assert_eq!(
            unsafe {
                sidereon_station_tide_displacement_batch(
                    position.as_ptr(),
                    epochs.as_ptr(),
                    epochs.len(),
                    &options,
                    rows.as_mut_ptr(),
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(rows[0].status, SidereonStatus::Ok);
        assert_eq!(rows[0].error.kind, SidereonStationTideErrorKind::None);
        assert_eq!(rows[1].status, SidereonStatus::InvalidArgument);
        assert_eq!(
            rows[1].error.kind,
            SidereonStationTideErrorKind::InvalidInput
        );
        assert!(rows[1].error.has_field);
        let mut text = [0u8; 32];
        let mut written = 0;
        let mut required = 0;
        assert_eq!(
            unsafe {
                sidereon_station_tide_batch_error_text(
                    1,
                    SidereonStationTideErrorText::Field as u32,
                    text.as_mut_ptr(),
                    text.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert!(written > 0);
        assert_eq!(
            unsafe {
                sidereon_station_tide_batch_error_text(
                    rows.len(),
                    SidereonStationTideErrorText::Field as u32,
                    text.as_mut_ptr(),
                    text.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::InvalidArgument
        );
    }

    #[test]
    fn empty_batch_clears_previous_row_text_without_dereferencing_arrays() {
        let position = [6_378_137.0, 0.0, 0.0];
        let options = default_options();
        let mut row = SidereonStationTideBatchRow {
            status: SidereonStatus::Panic,
            displacement: empty_displacement(),
            error: empty_station_tide_error(),
        };
        let invalid = SidereonStationTideEpoch {
            month: 13,
            ..epoch()
        };
        assert_eq!(
            unsafe {
                sidereon_station_tide_displacement_batch(
                    position.as_ptr(),
                    &invalid,
                    1,
                    &options,
                    &mut row,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(row.error.kind, SidereonStationTideErrorKind::InvalidInput);
        assert_eq!(
            unsafe {
                sidereon_station_tide_displacement_batch(
                    position.as_ptr(),
                    ptr::null(),
                    0,
                    &options,
                    ptr::null_mut(),
                )
            },
            SidereonStatus::Ok
        );
        let mut written = usize::MAX;
        let mut required = usize::MAX;
        let mut text = [0u8; 8];
        assert_eq!(
            unsafe {
                sidereon_station_tide_batch_error_text(
                    0,
                    SidereonStationTideErrorText::Field as u32,
                    text.as_mut_ptr(),
                    text.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert_eq!(written, 0);
        assert_eq!(required, 0);
    }

    #[test]
    fn low_level_tide_route_delegates_and_refuses_unknown_constant_tags() {
        let station = [6_378_137.0, 0.0, 0.0];
        let sun = [149_597_870_700.0, 0.0, 0.0];
        let moon = [384_400_000.0, 0.0, 0.0];
        let mut output = [0.0; 3];
        assert_eq!(
            unsafe {
                sidereon_solid_earth_tide_with_constants(
                    station.as_ptr(),
                    2024,
                    1,
                    1,
                    0.0,
                    sun.as_ptr(),
                    moon.as_ptr(),
                    SidereonStationTideConstants::Conventions as u32,
                    output.as_mut_ptr(),
                )
            },
            SidereonStatus::Ok
        );
        let expected = sidereon_core::tides::solid_earth_tide_with_constants(
            &station,
            2024,
            1,
            1,
            0.0,
            &sun,
            &moon,
            sidereon_core::tides::StationTideConstants::Conventions,
        )
        .unwrap();
        assert_eq!(output.map(f64::to_bits), expected.map(f64::to_bits));

        output = [1.0; 3];
        assert_eq!(
            unsafe {
                sidereon_solid_earth_tide_with_constants(
                    station.as_ptr(),
                    2024,
                    1,
                    1,
                    0.0,
                    sun.as_ptr(),
                    moon.as_ptr(),
                    88,
                    output.as_mut_ptr(),
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert_eq!(output, [0.0; 3]);
        let error = LAST_STATION_TIDE_ERROR.with(|slot| slot.borrow().clone().unwrap());
        assert_eq!(error.error.kind, SidereonStationTideErrorKind::InvalidTag);
        assert_eq!(error.field.as_deref(), Some("constants"));
        assert_eq!(error.reason.as_deref(), Some("unknown integer tag 88"));
    }
}
