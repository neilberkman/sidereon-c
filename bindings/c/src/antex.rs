use super::*;
use sidereon_core::antex::{
    AntennaKind, AntexDateTime, Calibration, Frequency, PcvGrid, PcvSample, PcvType, SecondFraction,
};

/// A parsed ANTEX antenna-calibration product. Create with sidereon_antex_parse
/// or sidereon_antex_parse_result and release with sidereon_antex_free.
///
/// The product retains every record ANTEX 1.4 defines and keeps absent records
/// absent: the header (`ANTEX VERSION / SYST`, `PCV TYPE / REFANT`, comments
/// and whether `END OF HEADER` is present), comments between and after the
/// antenna blocks, and every antenna block in file order.
pub struct SidereonAntex {
    pub(crate) inner: Antex,
}

/// A single ANTEX antenna calibration block (receiver or satellite), owned
/// independently of the parent product. Obtain one with sidereon_antex_antenna,
/// sidereon_antex_block, sidereon_antex_antenna_at or
/// sidereon_antex_satellite_antenna and release it with sidereon_antenna_free.
pub struct SidereonAntenna {
    pub(crate) inner: Antenna,
}

// --- typed ANTEX errors ------------------------------------------------------

/// Which failure an ANTEX read, lookup or write reported. Every kind but None
/// names an `AntexError` variant of the engine.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonAntexErrorKind {
    /// No failure is recorded.
    None = 0,
    /// A date-time component is outside the GPS calendar and clock ranges,
    /// including a second of 60, which GPS time does not have.
    InvalidDateTime = 1,
    /// A record field holds text that is not a valid value for it. Carries the
    /// antenna id (absent for a header record), record, field and value.
    InvalidField = 2,
    /// A record the format allows once per block, section or header appears
    /// again with different content. Carries the antenna id (absent for a
    /// header record) and record.
    RepeatedRecord = 3,
    /// A PCV row cannot be placed on a grid of distinct positions. Carries the
    /// antenna id, frequency and reason.
    DegenerateGrid = 4,
    /// A caller input was refused by shared validation. Carries field and
    /// reason.
    InvalidInput = 5,
    /// The requested frequency label is not among the antenna's sections.
    /// Carries the antenna id and frequency.
    UnknownFrequency = 6,
    /// Several frequency sections carry the label and their contents differ, so
    /// no single calibration answers. Carries the antenna id, frequency and
    /// sections.
    AmbiguousFrequency = 7,
    /// A frequency section ended without a `NORTH / EAST / UP` record. Carries
    /// the antenna id and frequency.
    MissingPco = 8,
    /// The selected PCV interpolation input holds no samples. Carries the
    /// antenna id and frequency.
    EmptyPcvGrid = 9,
    /// The product or a record cannot be written as ANTEX text exactly.
    /// Carries field and reason.
    Unwritable = 10,
}

/// Which text part of an ANTEX failure a text route copies.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonAntexErrorText {
    /// The complete failure text.
    Message = 0,
    /// The antenna id the failure names.
    AntennaId = 1,
    /// The record label the failure names.
    Record = 2,
    /// The field name the failure names.
    Field = 3,
    /// The field text an InvalidField failure carries.
    Value = 4,
    /// The frequency label the failure names.
    Frequency = 5,
    /// The reason the failure gives.
    Reason = 6,
}

/// Typed detail of an ANTEX failure. Only the fields the kind names carry
/// meaning; each has_ flag says whether the failure carries that text part,
/// which is copied with the text route of its owner.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SidereonAntexError {
    /// Which failure was reported.
    pub kind: SidereonAntexErrorKind,
    /// Whether the failure names an antenna id.
    pub has_antenna_id: bool,
    /// Whether the failure names a record label.
    pub has_record: bool,
    /// Whether the failure names a field.
    pub has_field: bool,
    /// Whether the failure carries field text.
    pub has_value: bool,
    /// Whether the failure names a frequency label.
    pub has_frequency: bool,
    /// Whether the failure gives a reason.
    pub has_reason: bool,
    /// Whether sections carries an AmbiguousFrequency section count.
    pub has_sections: bool,
    /// Number of frequency sections carrying the label.
    pub sections: usize,
}

/// The fixed-width outcome of one ANTEX parse or encode.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SidereonAntexOutcome {
    /// Whether the operation succeeded.
    pub is_ok: bool,
    /// SIDEREON_STATUS_OK on success, otherwise
    /// SIDEREON_STATUS_INVALID_ARGUMENT, which every failure maps to.
    pub status: SidereonStatus,
    /// The typed failure; kind is None when is_ok is true.
    pub error: SidereonAntexError,
}

/// An owned record of one ANTEX parse or encode: the encoded text on success,
/// or the typed failure with its owned text parts. It owns every string it
/// reports, so they stay readable after the product is freed and after later
/// failing calls have overwritten the thread-local message. Create with
/// sidereon_antex_parse_result or sidereon_antex_encode_result and release with
/// sidereon_antex_result_free.
pub struct SidereonAntexResult {
    pub(crate) outcome: SidereonAntexOutcome,
    /// The ANTEX text of a successful encode; empty otherwise.
    pub(crate) text: String,
    /// The failure text, prefixed with the route that produced it; empty on
    /// success.
    pub(crate) message: String,
    pub(crate) error: Option<AntexError>,
}

thread_local! {
    static LAST_ANTEX_ERROR: RefCell<Option<AntexError>> = const { RefCell::new(None) };
}

fn no_antex_error() -> SidereonAntexError {
    SidereonAntexError {
        kind: SidereonAntexErrorKind::None,
        has_antenna_id: false,
        has_record: false,
        has_field: false,
        has_value: false,
        has_frequency: false,
        has_reason: false,
        has_sections: false,
        sections: 0,
    }
}

fn antex_error_to_c(err: &AntexError) -> SidereonAntexError {
    use SidereonAntexErrorKind as Kind;
    let mut out = no_antex_error();
    match err {
        AntexError::InvalidDateTime => out.kind = Kind::InvalidDateTime,
        AntexError::InvalidField { antenna_id, .. } => {
            out.kind = Kind::InvalidField;
            out.has_antenna_id = antenna_id.is_some();
            out.has_record = true;
            out.has_field = true;
            out.has_value = true;
        }
        AntexError::RepeatedRecord { antenna_id, .. } => {
            out.kind = Kind::RepeatedRecord;
            out.has_antenna_id = antenna_id.is_some();
            out.has_record = true;
        }
        AntexError::DegenerateGrid { .. } => {
            out.kind = Kind::DegenerateGrid;
            out.has_antenna_id = true;
            out.has_frequency = true;
            out.has_reason = true;
        }
        AntexError::InvalidInput { .. } => {
            out.kind = Kind::InvalidInput;
            out.has_field = true;
            out.has_reason = true;
        }
        AntexError::UnknownFrequency { .. } => {
            out.kind = Kind::UnknownFrequency;
            out.has_antenna_id = true;
            out.has_frequency = true;
        }
        AntexError::AmbiguousFrequency { sections, .. } => {
            out.kind = Kind::AmbiguousFrequency;
            out.has_antenna_id = true;
            out.has_frequency = true;
            out.has_sections = true;
            out.sections = *sections;
        }
        AntexError::MissingPco { .. } => {
            out.kind = Kind::MissingPco;
            out.has_antenna_id = true;
            out.has_frequency = true;
        }
        AntexError::EmptyPcvGrid { .. } => {
            out.kind = Kind::EmptyPcvGrid;
            out.has_antenna_id = true;
            out.has_frequency = true;
        }
        AntexError::Unwritable { .. } => {
            out.kind = Kind::Unwritable;
            out.has_field = true;
            out.has_reason = true;
        }
    }
    out
}

/// One text part of an ANTEX failure; empty when the failure carries none, and
/// None for a part value this binding does not name.
fn antex_error_text(err: &AntexError, part: u32) -> Option<String> {
    use SidereonAntexErrorText as Part;
    if part > Part::Reason as u32 {
        return None;
    }
    if part == Part::Message as u32 {
        return Some(err.to_string());
    }
    let text = match err {
        AntexError::InvalidDateTime => None,
        AntexError::InvalidField {
            antenna_id,
            record,
            field,
            value,
        } => match part {
            p if p == Part::AntennaId as u32 => antenna_id.clone(),
            p if p == Part::Record as u32 => Some((*record).to_owned()),
            p if p == Part::Field as u32 => Some((*field).to_owned()),
            p if p == Part::Value as u32 => Some(value.clone()),
            _ => None,
        },
        AntexError::RepeatedRecord { antenna_id, record } => match part {
            p if p == Part::AntennaId as u32 => antenna_id.clone(),
            p if p == Part::Record as u32 => Some((*record).to_owned()),
            _ => None,
        },
        AntexError::DegenerateGrid {
            antenna_id,
            frequency,
            reason,
        } => match part {
            p if p == Part::AntennaId as u32 => Some(antenna_id.clone()),
            p if p == Part::Frequency as u32 => Some(frequency.clone()),
            p if p == Part::Reason as u32 => Some(reason.clone()),
            _ => None,
        },
        AntexError::InvalidInput { field, reason } => match part {
            p if p == Part::Field as u32 => Some((*field).to_owned()),
            p if p == Part::Reason as u32 => Some((*reason).to_owned()),
            _ => None,
        },
        AntexError::UnknownFrequency {
            antenna_id,
            frequency,
        }
        | AntexError::MissingPco {
            antenna_id,
            frequency,
        }
        | AntexError::EmptyPcvGrid {
            antenna_id,
            frequency,
        }
        | AntexError::AmbiguousFrequency {
            antenna_id,
            frequency,
            ..
        } => match part {
            p if p == Part::AntennaId as u32 => Some(antenna_id.clone()),
            p if p == Part::Frequency as u32 => Some(frequency.clone()),
            _ => None,
        },
        AntexError::Unwritable { field, reason } => match part {
            p if p == Part::Field as u32 => Some((*field).to_owned()),
            p if p == Part::Reason as u32 => Some(reason.clone()),
            _ => None,
        },
    };
    Some(text.unwrap_or_default())
}

/// Record an ANTEX failure for sidereon_last_antex_error and the thread-local
/// message, and return the status every ANTEX failure maps to.
pub(crate) fn record_antex_error(fn_name: &str, err: AntexError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    LAST_ANTEX_ERROR.with(|slot| *slot.borrow_mut() = Some(err));
    SidereonStatus::InvalidArgument
}

/// Copy the last typed ANTEX failure for this thread: the most recent ANTEX
/// parse, lookup (sidereon_antenna_pco, sidereon_antenna_pcv, epoch lookups)
/// or encode that failed. If none is recorded, kind is
/// SidereonAntexErrorKind::None.
///
/// Safety: out_error must point to a SidereonAntexError.
#[no_mangle]
pub unsafe extern "C" fn sidereon_last_antex_error(
    out_error: *mut SidereonAntexError,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_last_antex_error";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_error, FN_NAME, "out_error"));
        *out = LAST_ANTEX_ERROR.with(|slot| {
            slot.borrow()
                .as_ref()
                .map_or_else(no_antex_error, antex_error_to_c)
        });
        SidereonStatus::Ok
    })
}

/// Copy one text part of the last typed ANTEX failure for this thread,
/// selected by a SidereonAntexErrorText value. With no failure recorded, or a
/// part the failure does not carry, nothing is copied. Uses the variable-length
/// output contract; the bytes are copied verbatim and not null-terminated.
///
/// Safety: out points to len writable bytes or is NULL when len is 0;
/// out_written and out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_last_antex_error_text(
    part: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_last_antex_error_text";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        if part > SidereonAntexErrorText::Reason as u32 {
            set_last_error(format!("{FN_NAME}: invalid part {part}"));
            return SidereonStatus::InvalidArgument;
        }
        let text = LAST_ANTEX_ERROR.with(|slot| {
            slot.borrow()
                .as_ref()
                .and_then(|err| antex_error_text(err, part))
                .unwrap_or_default()
        });
        c_try!(copy_prefix_to_c(
            FN_NAME,
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

// --- read --------------------------------------------------------------------

unsafe fn antex_text_from_c<'a>(
    fn_name: &str,
    data: *const u8,
    len: usize,
) -> Result<&'a str, SidereonStatus> {
    let bytes = require_slice(data, len, fn_name, "data")?;
    str::from_utf8(bytes).map_err(|_| {
        set_last_error(format!("{fn_name}: data is not valid UTF-8"));
        SidereonStatus::InvalidToken
    })
}

/// Parse an ANTEX 1.4 antenna-calibration byte buffer. On success writes a newly
/// owned handle to *out_antex. Release it with sidereon_antex_free. PCO/PCV
/// values are exposed in meters, `mm * 1e-3` as RTKLIB `readantex` converts
/// them. A failure returns SIDEREON_STATUS_INVALID_ARGUMENT; the typed failure
/// is read with sidereon_last_antex_error, or returned owned by
/// sidereon_antex_parse_result.
///
/// Safety: data must point to len readable bytes; out_antex must point to
/// storage for a SidereonAntex*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antex_parse(
    data: *const u8,
    len: usize,
    out_antex: *mut *mut SidereonAntex,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_antex_parse";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_antex = c_try!(require_out(out_antex, FN_NAME, "out_antex"));
        *out_antex = ptr::null_mut();
        let text = c_try!(antex_text_from_c(FN_NAME, data, len));
        let inner = match Antex::parse(text) {
            Ok(antex) => antex,
            Err(err) => return map_antex_error(FN_NAME, err),
        };
        write_boxed_handle(out_antex, SidereonAntex { inner });
        SidereonStatus::Ok
    })
}

/// Parse an ANTEX byte buffer and take an owned record of the attempt.
///
/// Returns SIDEREON_STATUS_OK whenever the call itself is well formed and hands
/// back a newly owned SidereonAntexResult; on a refusal of the text the typed
/// failure is inside it and `*out_antex` stays NULL. A structural failure (a
/// null pointer, data that is not UTF-8) leaves both outputs NULL and returns a
/// status that is not OK.
///
/// Safety: data must point to len readable bytes; out_antex points to a
/// SidereonAntex*; out_result points to a SidereonAntexResult*. Both output
/// slots must be disjoint and are set to NULL before any work.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antex_parse_result(
    data: *const u8,
    len: usize,
    out_antex: *mut *mut SidereonAntex,
    out_result: *mut *mut SidereonAntexResult,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_antex_parse_result";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        if !out_antex.is_null() && !out_result.is_null() {
            let antex_range = c_try!(super::checked_output_range(
                FN_NAME,
                out_antex,
                1,
                "out_antex"
            ));
            let result_range = c_try!(super::checked_output_range(
                FN_NAME,
                out_result,
                1,
                "out_result"
            ));
            c_try!(super::reject_overlapping_outputs(
                FN_NAME,
                antex_range,
                result_range,
                "out_antex",
                "out_result"
            ));
        }
        if !out_antex.is_null() {
            *out_antex = ptr::null_mut();
        }
        if !out_result.is_null() {
            *out_result = ptr::null_mut();
        }
        let out_antex = c_try!(require_out(out_antex, FN_NAME, "out_antex"));
        let out_result = c_try!(require_out(out_result, FN_NAME, "out_result"));
        let text = c_try!(antex_text_from_c(FN_NAME, data, len));
        let result = match Antex::parse(text) {
            Ok(inner) => {
                write_boxed_handle(out_antex, SidereonAntex { inner });
                antex_result_ok(String::new())
            }
            Err(err) => antex_result_refused(FN_NAME, err),
        };
        write_boxed_handle(out_result, result);
        SidereonStatus::Ok
    })
}

/// Write the number of distinct antenna ids in the product to *out_count. An id
/// with several validity blocks counts once; sidereon_antex_block_count counts
/// every block.
///
/// Safety: antex must be a live handle from sidereon_antex_parse; out_count must
/// point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antex_antenna_count(
    antex: *const SidereonAntex,
    out_count: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_antex_antenna_count";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_count = c_try!(require_out(out_count, FN_NAME, "out_count"));
        *out_count = 0;
        let antex = c_try!(require_ref(antex, FN_NAME, "antex"));
        *out_count = antex.inner.antennas.len();
        SidereonStatus::Ok
    })
}

/// Look up an antenna by its exact `TYPE / SERIAL` id. On success writes a newly
/// owned handle for the latest block with that id to *out_antenna, or NULL if no
/// block has that id (a successful query that found nothing, not an error).
/// Release a non-NULL handle with sidereon_antenna_free.
///
/// Safety: antex must be a live handle from sidereon_antex_parse; id must be a
/// null-terminated C string; out_antenna must point to storage for a
/// SidereonAntenna*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antex_antenna(
    antex: *const SidereonAntex,
    id: *const c_char,
    out_antenna: *mut *mut SidereonAntenna,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_antex_antenna";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_antenna = c_try!(require_out(out_antenna, FN_NAME, "out_antenna"));
        *out_antenna = ptr::null_mut();
        let antex = c_try!(require_ref(antex, FN_NAME, "antex"));
        let id = c_try!(parse_bounded_c_string(
            FN_NAME,
            "id",
            id,
            MAX_ANTEX_ID_BYTES
        ));
        if let Some(antenna) = antex.inner.antenna(&id) {
            write_boxed_handle(
                out_antenna,
                SidereonAntenna {
                    inner: antenna.clone(),
                },
            );
        }
        SidereonStatus::Ok
    })
}

/// Write the number of antenna blocks in the product, every validity block of
/// every id.
///
/// Safety: antex is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antex_block_count(
    antex: *const SidereonAntex,
    out_count: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_antex_block_count";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_count = c_try!(require_out(out_count, FN_NAME, "out_count"));
        *out_count = 0;
        let antex = c_try!(require_ref(antex, FN_NAME, "antex"));
        *out_count = antex.inner.antenna_blocks().count();
        SidereonStatus::Ok
    })
}

/// Take a newly owned handle for the antenna block at index, in file order.
///
/// Safety: antex is a live handle; out_antenna points to a SidereonAntenna*,
/// set to NULL before any work.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antex_block(
    antex: *const SidereonAntex,
    index: usize,
    out_antenna: *mut *mut SidereonAntenna,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_antex_block";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_antenna = c_try!(require_out(out_antenna, FN_NAME, "out_antenna"));
        *out_antenna = ptr::null_mut();
        let antex = c_try!(require_ref(antex, FN_NAME, "antex"));
        let Some(antenna) = antex.inner.antenna_blocks().nth(index) else {
            set_last_error(format!("{FN_NAME}: index {index} out of range"));
            return SidereonStatus::InvalidArgument;
        };
        write_boxed_handle(
            out_antenna,
            SidereonAntenna {
                inner: antenna.clone(),
            },
        );
        SidereonStatus::Ok
    })
}

/// Exact GPS-time calendar instant of an ANTEX `VALID FROM` / `VALID UNTIL`
/// bound. The fraction of the second is `fraction_digits / 10^fraction_scale`,
/// kept exactly as the `F13.7` seconds field states it (a field with an
/// exponent can state a fraction far below a nanosecond). An instant this
/// binding returns is normalized: fraction_digits has no trailing zero digit,
/// and a zero fraction has digits and scale 0.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SidereonAntexDateTime {
    /// Calendar year.
    pub year: i32,
    /// Month, 1..=12.
    pub month: u8,
    /// Day of month.
    pub day: u8,
    /// Hour, 0..=23.
    pub hour: u8,
    /// Minute, 0..=59.
    pub minute: u8,
    /// Whole second, 0..=59.
    pub second: u8,
    /// Significant digits of the fraction of the second.
    pub fraction_digits: u64,
    /// Power of ten dividing fraction_digits.
    pub fraction_scale: u64,
}

fn antex_date_time_to_c(value: AntexDateTime) -> SidereonAntexDateTime {
    SidereonAntexDateTime {
        year: value.year,
        month: value.month,
        day: value.day,
        hour: value.hour,
        minute: value.minute,
        second: value.second,
        fraction_digits: value.fraction.digits(),
        fraction_scale: value.fraction.scale(),
    }
}

fn absent_antex_date_time() -> SidereonAntexDateTime {
    SidereonAntexDateTime {
        year: 0,
        month: 0,
        day: 0,
        hour: 0,
        minute: 0,
        second: 0,
        fraction_digits: 0,
        fraction_scale: 0,
    }
}

/// Read a caller date-time. A fraction of one second or more is refused as
/// InvalidDateTime, as is a component outside the GPS calendar and clock.
fn antex_date_time_from_c(
    fn_name: &str,
    value: &SidereonAntexDateTime,
) -> Result<AntexDateTime, SidereonStatus> {
    let fraction = SecondFraction::new(value.fraction_digits, value.fraction_scale)
        .ok_or(AntexError::InvalidDateTime)
        .and_then(|fraction| {
            AntexDateTime::new_with_fraction(
                value.year,
                value.month,
                value.day,
                value.hour,
                value.minute,
                value.second,
                fraction,
            )
        });
    fraction.map_err(|err| map_antex_error(fn_name, err))
}

/// Take a newly owned handle for the block of an antenna id valid at an epoch,
/// or NULL with status OK when none is.
///
/// Safety: antex is a live handle; id is a null-terminated UTF-8 string; epoch
/// points to a SidereonAntexDateTime; out_antenna points to a
/// SidereonAntenna*, set to NULL before any work.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antex_antenna_at(
    antex: *const SidereonAntex,
    id: *const c_char,
    epoch: *const SidereonAntexDateTime,
    out_antenna: *mut *mut SidereonAntenna,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_antex_antenna_at";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_antenna = c_try!(require_out(out_antenna, FN_NAME, "out_antenna"));
        *out_antenna = ptr::null_mut();
        let antex = c_try!(require_ref(antex, FN_NAME, "antex"));
        let id = c_try!(parse_bounded_c_string(
            FN_NAME,
            "id",
            id,
            MAX_ANTEX_ID_BYTES
        ));
        let epoch = c_try!(antex_date_time_from_c(
            FN_NAME,
            c_try!(require_ref(epoch, FN_NAME, "epoch"))
        ));
        if let Some(antenna) = antex.inner.antenna_at(&id, epoch) {
            write_boxed_handle(
                out_antenna,
                SidereonAntenna {
                    inner: antenna.clone(),
                },
            );
        }
        SidereonStatus::Ok
    })
}

/// Take a newly owned handle for the satellite antenna block of a PRN (such as
/// `G05`) valid at an epoch, or NULL with status OK when none is.
///
/// Safety: antex is a live handle; prn is a null-terminated UTF-8 string; epoch
/// points to a SidereonAntexDateTime; out_antenna points to a
/// SidereonAntenna*, set to NULL before any work.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antex_satellite_antenna(
    antex: *const SidereonAntex,
    prn: *const c_char,
    epoch: *const SidereonAntexDateTime,
    out_antenna: *mut *mut SidereonAntenna,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_antex_satellite_antenna";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_antenna = c_try!(require_out(out_antenna, FN_NAME, "out_antenna"));
        *out_antenna = ptr::null_mut();
        let antex = c_try!(require_ref(antex, FN_NAME, "antex"));
        let prn = c_try!(parse_bounded_c_string(
            FN_NAME,
            "prn",
            prn,
            MAX_ANTEX_ID_BYTES
        ));
        let epoch = c_try!(antex_date_time_from_c(
            FN_NAME,
            c_try!(require_ref(epoch, FN_NAME, "epoch"))
        ));
        if let Some(antenna) = antex.inner.satellite_antenna(&prn, epoch) {
            write_boxed_handle(
                out_antenna,
                SidereonAntenna {
                    inner: antenna.clone(),
                },
            );
        }
        SidereonStatus::Ok
    })
}

// --- header, outer comments, skipped records -----------------------------------

/// Phase center variation type from `PCV TYPE / REFANT` column 1.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonAntexPcvType {
    /// `A`: absolute values.
    Absolute = 0,
    /// `R`: values relative to a reference antenna.
    Relative = 1,
}

/// The retained ANTEX header records. Text is read with
/// sidereon_antex_header_text and sidereon_antex_header_comment.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SidereonAntexHeader {
    /// Whether the source carries `ANTEX VERSION / SYST`.
    pub has_version: bool,
    /// Format version.
    pub version: f64,
    /// Whether the system column of `ANTEX VERSION / SYST` is not blank.
    pub has_system: bool,
    /// Satellite system flag code point (`G`, `R`, `E`, `C`, `J`, `S` or `M`).
    pub system: u32,
    /// Whether the source carries `PCV TYPE / REFANT`.
    pub has_pcv_type: bool,
    /// Whether the values are absolute or relative, as SidereonAntexPcvType.
    pub pcv_type: u32,
    /// Whether the values are relative to a reference antenna, the stated type
    /// or `AOAD/M_T` when a relative file leaves it blank.
    pub has_reference_antenna: bool,
    /// Number of header `COMMENT` records.
    pub comment_count: usize,
    /// Whether the source carries `END OF HEADER`.
    pub end_of_header: bool,
}

/// Which header text sidereon_antex_header_text copies.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonAntexHeaderText {
    /// Reference antenna type from `PCV TYPE / REFANT` columns 21-40, trimmed;
    /// empty when blank or when the record is absent.
    ReferenceAntennaType = 0,
    /// Reference antenna serial number from columns 41-60, trimmed; empty when
    /// blank or when the record is absent.
    ReferenceAntennaSerial = 1,
    /// The antenna type relative values refer to (the stated type, or
    /// `AOAD/M_T` when a relative file leaves it blank); empty when
    /// has_reference_antenna is false.
    ReferenceAntenna = 2,
}

/// Copy the retained header records of an ANTEX product.
///
/// Safety: antex is a live handle; out_header points to a SidereonAntexHeader.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antex_header(
    antex: *const SidereonAntex,
    out_header: *mut SidereonAntexHeader,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_antex_header";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_header, FN_NAME, "out_header"));
        *out = SidereonAntexHeader {
            has_version: false,
            version: f64::NAN,
            has_system: false,
            system: 0,
            has_pcv_type: false,
            pcv_type: 0,
            has_reference_antenna: false,
            comment_count: 0,
            end_of_header: false,
        };
        let header = &c_try!(require_ref(antex, FN_NAME, "antex")).inner.header;
        let version = header.version.as_ref();
        let system = version.and_then(|version| version.system);
        let pcv_type = header.pcv_type.as_ref();
        *out = SidereonAntexHeader {
            has_version: version.is_some(),
            version: version.map_or(f64::NAN, |version| version.version),
            has_system: system.is_some(),
            system: system.map_or(0, u32::from),
            has_pcv_type: pcv_type.is_some(),
            pcv_type: pcv_type.map_or(0, |record| match record.pcv_type {
                PcvType::Absolute => SidereonAntexPcvType::Absolute as u32,
                PcvType::Relative => SidereonAntexPcvType::Relative as u32,
            }),
            has_reference_antenna: pcv_type
                .is_some_and(|record| record.reference_antenna().is_some()),
            comment_count: header.comments.len(),
            end_of_header: header.end_of_header,
        };
        SidereonStatus::Ok
    })
}

/// Copy one header text, selected by a SidereonAntexHeaderText value. Uses the
/// variable-length output contract; the bytes are not null-terminated.
///
/// Safety: antex is a live handle; out points to len writable bytes or is NULL
/// when len is 0; out_written and out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antex_header_text(
    antex: *const SidereonAntex,
    part: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_antex_header_text";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let header = &c_try!(require_ref(antex, FN_NAME, "antex")).inner.header;
        let record = header.pcv_type.as_ref();
        let text = match part {
            value if value == SidereonAntexHeaderText::ReferenceAntennaType as u32 => {
                record.map_or("", |record| record.reference_antenna_type.as_str())
            }
            value if value == SidereonAntexHeaderText::ReferenceAntennaSerial as u32 => {
                record.map_or("", |record| record.reference_antenna_serial.as_str())
            }
            value if value == SidereonAntexHeaderText::ReferenceAntenna as u32 => record
                .and_then(|record| record.reference_antenna())
                .unwrap_or(""),
            other => {
                set_last_error(format!("{FN_NAME}: invalid part {other}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        c_try!(copy_prefix_to_c(
            FN_NAME,
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

/// Copy the text of one header `COMMENT` record, in file order, trailing blanks
/// removed. Uses the variable-length output contract; the bytes are not
/// null-terminated.
///
/// Safety: antex is a live handle; out points to len writable bytes or is NULL
/// when len is 0; out_written and out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antex_header_comment(
    antex: *const SidereonAntex,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_antex_header_comment";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let header = &c_try!(require_ref(antex, FN_NAME, "antex")).inner.header;
        let Some(text) = header.comments.get(index) else {
            set_last_error(format!("{FN_NAME}: index {index} out of range"));
            return SidereonStatus::InvalidArgument;
        };
        c_try!(copy_prefix_to_c(
            FN_NAME,
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

/// Write the number of `COMMENT` records after `END OF HEADER` that lie outside
/// every antenna block.
///
/// Safety: antex is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antex_outer_comment_count(
    antex: *const SidereonAntex,
    out_count: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_antex_outer_comment_count";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_count = c_try!(require_out(out_count, FN_NAME, "out_count"));
        *out_count = 0;
        let antex = c_try!(require_ref(antex, FN_NAME, "antex"));
        *out_count = antex.inner.outer_comments.len();
        SidereonStatus::Ok
    })
}

/// Copy one comment outside the antenna blocks, in file order: its text,
/// trailing blanks removed, on the variable-length output contract, and in
/// *out_blocks_before the number of antenna blocks that precede it.
///
/// Safety: antex is a live handle; out_blocks_before points to a size_t; out
/// points to len writable bytes or is NULL when len is 0; out_written and
/// out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antex_outer_comment(
    antex: *const SidereonAntex,
    index: usize,
    out_blocks_before: *mut usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_antex_outer_comment";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        if !out_blocks_before.is_null() {
            *out_blocks_before = 0;
        }
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let out_blocks_before =
            c_try!(require_out(out_blocks_before, FN_NAME, "out_blocks_before"));
        let antex = c_try!(require_ref(antex, FN_NAME, "antex"));
        let Some(comment) = antex.inner.outer_comments.get(index) else {
            set_last_error(format!("{FN_NAME}: index {index} out of range"));
            return SidereonStatus::InvalidArgument;
        };
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            comment.text.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        *out_blocks_before = comment.blocks_before;
        SidereonStatus::Ok
    })
}

/// Write the number of records a forgiving read skipped or found inconsistent:
/// a corrupt PCV value, an unrecognized grid-row head, a line outside any
/// record the format defines, a `# OF FREQUENCIES` count that disagrees with
/// the sections read, a frequency end record naming another frequency, a
/// header record after `END OF HEADER`, and a block or section its own end
/// record does not close. The blocks and sections are kept. A clean file
/// reports zero.
///
/// Safety: antex is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antex_skipped_records(
    antex: *const SidereonAntex,
    out_count: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_antex_skipped_records";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_count = c_try!(require_out(out_count, FN_NAME, "out_count"));
        *out_count = 0;
        let antex = c_try!(require_ref(antex, FN_NAME, "antex"));
        *out_count = antex.inner.skipped_records();
        SidereonStatus::Ok
    })
}

// --- antenna blocks ------------------------------------------------------------

/// ANTEX antenna block role.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonAntennaKind {
    /// A receiver antenna.
    Receiver = 0,
    /// A satellite antenna: the serial is one system letter and two digits.
    Satellite = 1,
}

/// Fixed-width fields of one ANTEX antenna block. Text is read with
/// sidereon_antenna_text, sidereon_antenna_comment,
/// sidereon_antenna_calibration_text and sidereon_antenna_frequency_label.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SidereonAntennaInfo {
    /// Receiver or satellite, as SidereonAntennaKind.
    pub kind: u32,
    /// Whether dazi_deg carries the `DAZI` record.
    pub has_dazi_deg: bool,
    /// Azimuth increment, degrees.
    pub dazi_deg: f64,
    /// Whether the zenith fields carry the `ZEN1 / ZEN2 / DZEN` record.
    pub has_zenith_grid: bool,
    /// `ZEN1`, degrees.
    pub zenith_start_deg: f64,
    /// `ZEN2`, degrees.
    pub zenith_end_deg: f64,
    /// `DZEN`, degrees.
    pub zenith_step_deg: f64,
    /// Whether the block carries `# OF FREQUENCIES`.
    pub has_frequency_count_record: bool,
    /// Whether the block carries `SINEX CODE`.
    pub has_sinex_code: bool,
    /// Whether valid_from carries `VALID FROM`.
    pub has_valid_from: bool,
    /// `VALID FROM`, exactly as stated.
    pub valid_from: SidereonAntexDateTime,
    /// Whether valid_until carries `VALID UNTIL`.
    pub has_valid_until: bool,
    /// `VALID UNTIL`, exactly as stated.
    pub valid_until: SidereonAntexDateTime,
    /// Number of `METH / BY / # / DATE` records.
    pub calibration_count: usize,
    /// Number of comments before `TYPE / SERIAL NO`.
    pub leading_comment_count: usize,
    /// Number of comments after `TYPE / SERIAL NO`.
    pub comment_count: usize,
    /// Number of frequency sections, in file order; a repeated label keeps
    /// every section.
    pub frequency_count: usize,
}

/// Which antenna text sidereon_antenna_text copies.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonAntennaText {
    /// The trimmed `TYPE / SERIAL NO` body, the antenna id.
    Id = 0,
    /// The trimmed antenna type field.
    AntennaType = 1,
    /// The trimmed serial number field.
    Serial = 2,
    /// The `SINEX CODE` text; empty when has_sinex_code is false.
    SinexCode = 3,
}

/// Which comment list of an antenna block sidereon_antenna_comment reads.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonAntennaCommentList {
    /// Comments between `START OF ANTENNA` and `TYPE / SERIAL NO`.
    Leading = 0,
    /// Comments after `TYPE / SERIAL NO`.
    Block = 1,
}

/// One `METH / BY / # / DATE` record. Its method, agency and date are read with
/// sidereon_antenna_calibration_text.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SidereonAntexCalibration {
    /// Whether antennas_calibrated carries the `I6` count.
    pub has_antennas_calibrated: bool,
    /// Number of individual antennas calibrated.
    pub antennas_calibrated: u32,
}

/// Which calibration text sidereon_antenna_calibration_text copies.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonAntexCalibrationText {
    /// Method, columns 1-20, trailing blanks removed.
    Method = 0,
    /// Agency, columns 21-40, trailing blanks removed.
    Agency = 1,
    /// Date text, columns 51-60, trailing blanks removed.
    Date = 2,
}

/// Fixed-width fields of one frequency section. Its label is read with
/// sidereon_antenna_frequency_label and its samples with
/// sidereon_antenna_frequency_pcv_samples.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SidereonAntexFrequencyInfo {
    /// `NORTH / EAST / UP` phase-center offset, meters.
    pub pco_m: [f64; 3],
    /// Number of PCV samples.
    pub pcv_sample_count: usize,
    /// Whether the section has a `START OF FREQ RMS` section.
    pub has_rms: bool,
    /// Whether rms_pco_m carries the RMS section's `NORTH / EAST / UP` record.
    pub has_rms_pco_m: bool,
    /// RMS of the eccentricities, meters.
    pub rms_pco_m: [f64; 3],
    /// Number of RMS pattern samples.
    pub rms_pcv_sample_count: usize,
}

/// PCV grid type of a sample.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonAntexPcvGrid {
    /// A `NOAZI` row.
    NoAzimuth = 0,
    /// A numeric-azimuth row.
    Azimuth = 1,
}

/// One phase-center-variation grid value.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SidereonAntexPcvSample {
    /// Row type, as SidereonAntexPcvGrid.
    pub grid: u32,
    /// Whether azimuth_deg carries the row's azimuth; false for `NOAZI`.
    pub has_azimuth_deg: bool,
    /// Row azimuth as parsed, degrees; NaN for `NOAZI`.
    pub azimuth_deg: f64,
    /// Zenith (receiver) or nadir (satellite) angle, degrees.
    pub zenith_deg: f64,
    /// Value, meters.
    pub value_m: f64,
}

unsafe fn antenna_ref<'a>(
    fn_name: &str,
    antenna: *const SidereonAntenna,
) -> Result<&'a Antenna, SidereonStatus> {
    Ok(&require_ref(antenna, fn_name, "antenna")?.inner)
}

fn antenna_frequency_at<'a>(
    fn_name: &str,
    antenna: &'a Antenna,
    index: usize,
) -> Result<&'a Frequency, SidereonStatus> {
    antenna.frequencies.get(index).ok_or_else(|| {
        set_last_error(format!("{fn_name}: index {index} out of range"));
        SidereonStatus::InvalidArgument
    })
}

fn pcv_sample_to_c(sample: &PcvSample) -> SidereonAntexPcvSample {
    SidereonAntexPcvSample {
        grid: match sample.grid {
            PcvGrid::NoAzimuth => SidereonAntexPcvGrid::NoAzimuth as u32,
            PcvGrid::Azimuth => SidereonAntexPcvGrid::Azimuth as u32,
        },
        has_azimuth_deg: sample.azimuth_deg.is_some(),
        azimuth_deg: sample.azimuth_deg.unwrap_or(f64::NAN),
        zenith_deg: sample.zenith_deg,
        value_m: sample.value_m,
    }
}

/// Copy the fixed-width fields of an antenna block.
///
/// Safety: antenna is a live handle; out_info points to a SidereonAntennaInfo.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antenna_info(
    antenna: *const SidereonAntenna,
    out_info: *mut SidereonAntennaInfo,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_antenna_info";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_info, FN_NAME, "out_info"));
        *out = SidereonAntennaInfo {
            kind: SidereonAntennaKind::Receiver as u32,
            has_dazi_deg: false,
            dazi_deg: f64::NAN,
            has_zenith_grid: false,
            zenith_start_deg: f64::NAN,
            zenith_end_deg: f64::NAN,
            zenith_step_deg: f64::NAN,
            has_frequency_count_record: false,
            has_sinex_code: false,
            has_valid_from: false,
            valid_from: absent_antex_date_time(),
            has_valid_until: false,
            valid_until: absent_antex_date_time(),
            calibration_count: 0,
            leading_comment_count: 0,
            comment_count: 0,
            frequency_count: 0,
        };
        let antenna = c_try!(antenna_ref(FN_NAME, antenna));
        let grid = antenna.zenith_grid;
        *out = SidereonAntennaInfo {
            kind: match antenna.kind {
                AntennaKind::Receiver => SidereonAntennaKind::Receiver as u32,
                AntennaKind::Satellite => SidereonAntennaKind::Satellite as u32,
            },
            has_dazi_deg: antenna.dazi_deg.is_some(),
            dazi_deg: antenna.dazi_deg.unwrap_or(f64::NAN),
            has_zenith_grid: grid.is_some(),
            zenith_start_deg: grid.map_or(f64::NAN, |grid| grid.start_deg),
            zenith_end_deg: grid.map_or(f64::NAN, |grid| grid.end_deg),
            zenith_step_deg: grid.map_or(f64::NAN, |grid| grid.step_deg),
            has_frequency_count_record: antenna.has_frequency_count,
            has_sinex_code: antenna.sinex_code.is_some(),
            has_valid_from: antenna.valid_from.is_some(),
            valid_from: antenna
                .valid_from
                .map_or_else(absent_antex_date_time, antex_date_time_to_c),
            has_valid_until: antenna.valid_until.is_some(),
            valid_until: antenna
                .valid_until
                .map_or_else(absent_antex_date_time, antex_date_time_to_c),
            calibration_count: antenna.calibrations.len(),
            leading_comment_count: antenna.leading_comments.len(),
            comment_count: antenna.comments.len(),
            frequency_count: antenna.frequencies.len(),
        };
        SidereonStatus::Ok
    })
}

/// Copy one antenna text, selected by a SidereonAntennaText value. Uses the
/// variable-length output contract; the bytes are not null-terminated.
///
/// Safety: antenna is a live handle; out points to len writable bytes or is
/// NULL when len is 0; out_written and out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antenna_text(
    antenna: *const SidereonAntenna,
    part: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_antenna_text";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let antenna = c_try!(antenna_ref(FN_NAME, antenna));
        let text = match part {
            value if value == SidereonAntennaText::Id as u32 => antenna.id.as_str(),
            value if value == SidereonAntennaText::AntennaType as u32 => {
                antenna.antenna_type.as_str()
            }
            value if value == SidereonAntennaText::Serial as u32 => antenna.serial.as_str(),
            value if value == SidereonAntennaText::SinexCode as u32 => {
                antenna.sinex_code.as_deref().unwrap_or("")
            }
            other => {
                set_last_error(format!("{FN_NAME}: invalid part {other}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        c_try!(copy_prefix_to_c(
            FN_NAME,
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

/// Copy one comment of an antenna block, from the list a
/// SidereonAntennaCommentList value selects, in file order, trailing blanks
/// removed. Uses the variable-length output contract; the bytes are not
/// null-terminated.
///
/// Safety: antenna is a live handle; out points to len writable bytes or is
/// NULL when len is 0; out_written and out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antenna_comment(
    antenna: *const SidereonAntenna,
    list: u32,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_antenna_comment";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let antenna = c_try!(antenna_ref(FN_NAME, antenna));
        let comments = match list {
            value if value == SidereonAntennaCommentList::Leading as u32 => {
                &antenna.leading_comments
            }
            value if value == SidereonAntennaCommentList::Block as u32 => &antenna.comments,
            other => {
                set_last_error(format!("{FN_NAME}: invalid list {other}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        let Some(text) = comments.get(index) else {
            set_last_error(format!("{FN_NAME}: index {index} out of range"));
            return SidereonStatus::InvalidArgument;
        };
        c_try!(copy_prefix_to_c(
            FN_NAME,
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

fn antenna_calibration_at<'a>(
    fn_name: &str,
    antenna: &'a Antenna,
    index: usize,
) -> Result<&'a Calibration, SidereonStatus> {
    antenna.calibrations.get(index).ok_or_else(|| {
        set_last_error(format!("{fn_name}: index {index} out of range"));
        SidereonStatus::InvalidArgument
    })
}

/// Copy the fixed-width fields of one `METH / BY / # / DATE` record.
///
/// Safety: antenna is a live handle; out_calibration points to a
/// SidereonAntexCalibration.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antenna_calibration(
    antenna: *const SidereonAntenna,
    index: usize,
    out_calibration: *mut SidereonAntexCalibration,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_antenna_calibration";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_calibration, FN_NAME, "out_calibration"));
        *out = SidereonAntexCalibration {
            has_antennas_calibrated: false,
            antennas_calibrated: 0,
        };
        let antenna = c_try!(antenna_ref(FN_NAME, antenna));
        let calibration = c_try!(antenna_calibration_at(FN_NAME, antenna, index));
        *out = SidereonAntexCalibration {
            has_antennas_calibrated: calibration.antennas_calibrated.is_some(),
            antennas_calibrated: calibration.antennas_calibrated.unwrap_or(0),
        };
        SidereonStatus::Ok
    })
}

/// Copy one text of a `METH / BY / # / DATE` record, selected by a
/// SidereonAntexCalibrationText value. Uses the variable-length output
/// contract; the bytes are not null-terminated.
///
/// Safety: antenna is a live handle; out points to len writable bytes or is
/// NULL when len is 0; out_written and out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antenna_calibration_text(
    antenna: *const SidereonAntenna,
    index: usize,
    part: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_antenna_calibration_text";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let antenna = c_try!(antenna_ref(FN_NAME, antenna));
        let calibration = c_try!(antenna_calibration_at(FN_NAME, antenna, index));
        let text = match part {
            value if value == SidereonAntexCalibrationText::Method as u32 => &calibration.method,
            value if value == SidereonAntexCalibrationText::Agency as u32 => &calibration.agency,
            value if value == SidereonAntexCalibrationText::Date as u32 => &calibration.date,
            other => {
                set_last_error(format!("{FN_NAME}: invalid part {other}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        c_try!(copy_prefix_to_c(
            FN_NAME,
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

/// Copy the fixed-width fields of one frequency section, by index in file
/// order.
///
/// Safety: antenna is a live handle; out_info points to a
/// SidereonAntexFrequencyInfo.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antenna_frequency(
    antenna: *const SidereonAntenna,
    index: usize,
    out_info: *mut SidereonAntexFrequencyInfo,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_antenna_frequency";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_info, FN_NAME, "out_info"));
        *out = SidereonAntexFrequencyInfo {
            pco_m: [f64::NAN; 3],
            pcv_sample_count: 0,
            has_rms: false,
            has_rms_pco_m: false,
            rms_pco_m: [f64::NAN; 3],
            rms_pcv_sample_count: 0,
        };
        let antenna = c_try!(antenna_ref(FN_NAME, antenna));
        let frequency = c_try!(antenna_frequency_at(FN_NAME, antenna, index));
        let rms = frequency.rms.as_ref();
        let rms_pco = rms.and_then(|rms| rms.pco_m);
        *out = SidereonAntexFrequencyInfo {
            pco_m: frequency.pco_m,
            pcv_sample_count: frequency.pcv_samples.len(),
            has_rms: rms.is_some(),
            has_rms_pco_m: rms_pco.is_some(),
            rms_pco_m: rms_pco.unwrap_or([f64::NAN; 3]),
            rms_pcv_sample_count: rms.map_or(0, |rms| rms.pcv_samples.len()),
        };
        SidereonStatus::Ok
    })
}

/// Copy the label of one frequency section, such as `G01`. Uses the
/// variable-length output contract; the bytes are not null-terminated.
///
/// Safety: antenna is a live handle; out points to len writable bytes or is
/// NULL when len is 0; out_written and out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antenna_frequency_label(
    antenna: *const SidereonAntenna,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_antenna_frequency_label";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let antenna = c_try!(antenna_ref(FN_NAME, antenna));
        let frequency = c_try!(antenna_frequency_at(FN_NAME, antenna, index));
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            frequency.frequency.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Copy the PCV samples of one frequency section, or of its RMS section when
/// rms is true (none when the section has no RMS section), in row and token
/// order. Uses the standard caller-buffer convention.
///
/// Safety: antenna is a live handle; out points to len writable
/// SidereonAntexPcvSample values or is NULL when len is 0; out_written and
/// out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antenna_frequency_pcv_samples(
    antenna: *const SidereonAntenna,
    index: usize,
    rms: bool,
    out: *mut SidereonAntexPcvSample,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_antenna_frequency_pcv_samples";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let antenna = c_try!(antenna_ref(FN_NAME, antenna));
        let frequency = c_try!(antenna_frequency_at(FN_NAME, antenna, index));
        let samples: &[PcvSample] = if rms {
            frequency
                .rms
                .as_ref()
                .map_or(&[][..], |rms| rms.pcv_samples.as_slice())
        } else {
            &frequency.pcv_samples
        };
        let values: Vec<SidereonAntexPcvSample> = samples.iter().map(pcv_sample_to_c).collect();
        c_try!(copy_prefix_to_c(
            FN_NAME,
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

/// Write whether an antenna block is valid at an epoch: at or after its
/// `VALID FROM` and at or before its `VALID UNTIL`, each compared exactly, an
/// absent bound leaving that side open.
///
/// Safety: antenna is a live handle; epoch points to a SidereonAntexDateTime;
/// out_valid points to a bool.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antenna_valid_at(
    antenna: *const SidereonAntenna,
    epoch: *const SidereonAntexDateTime,
    out_valid: *mut bool,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_antenna_valid_at";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_valid = c_try!(require_out(out_valid, FN_NAME, "out_valid"));
        *out_valid = false;
        let antenna = c_try!(antenna_ref(FN_NAME, antenna));
        let epoch = c_try!(antex_date_time_from_c(
            FN_NAME,
            c_try!(require_ref(epoch, FN_NAME, "epoch"))
        ));
        *out_valid = antenna.valid_at(epoch);
        SidereonStatus::Ok
    })
}

// --- write -------------------------------------------------------------------

/// Serialize a parsed ANTEX product back to ANTEX text. The output is not
/// null-terminated. Delegates to sidereon_core::antex::Antex::encode. Uses the
/// variable-length output contract documented at the top of the header.
///
/// Every record is written from a retained value, and no record the source did
/// not carry is written, apart from the start and end records of blocks and
/// sections. The encode is refused when the product holds something ANTEX text
/// cannot state exactly, such as a field that overflows its columns, a value its
/// column would round, validity seconds no `F13.7` form with a decimal point
/// states, sample coordinates the grid cannot reconstruct, or public antenna
/// fields that disagree with the retained validity intervals. A refusal returns
/// SIDEREON_STATUS_INVALID_ARGUMENT, reports a required length of zero and
/// writes nothing; sidereon_last_antex_error reports it typed, and
/// sidereon_antex_encode_result returns it owned.
///
/// Safety: antex must be a live handle from sidereon_antex_parse; out must point
/// to at least len writable bytes or be NULL when len is 0; out_written and
/// out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antex_encode(
    antex: *const SidereonAntex,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_antex_encode";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let antex = c_try!(require_ref(antex, FN_NAME, "antex"));
        let text = match antex.inner.encode() {
            Ok(text) => text,
            Err(err) => return map_antex_error(FN_NAME, err),
        };
        c_try!(copy_prefix_to_c(
            FN_NAME,
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

/// Encode a parsed ANTEX product and take an owned record of the attempt.
///
/// Returns SIDEREON_STATUS_OK whenever the call itself is well formed and hands
/// back a newly owned SidereonAntexResult: the ANTEX text when the product was
/// written, or the typed refusal. A null argument leaves `*out_result` NULL,
/// returns SIDEREON_STATUS_NULL_POINTER and allocates nothing.
///
/// Safety: `antex` must be a live SidereonAntex handle; `out_result` must point
/// to writable storage for one `SidereonAntexResult *`, which is set to NULL
/// before any work and receives a newly owned handle only on
/// SIDEREON_STATUS_OK; release it with sidereon_antex_result_free.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antex_encode_result(
    antex: *const SidereonAntex,
    out_result: *mut *mut SidereonAntexResult,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_antex_encode_result";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_result = c_try!(require_out(out_result, FN_NAME, "out_result"));
        *out_result = ptr::null_mut();
        let antex = c_try!(require_ref(antex, FN_NAME, "antex"));
        let result = match antex.inner.encode() {
            Ok(text) => antex_result_ok(text),
            Err(err) => antex_result_refused(FN_NAME, err),
        };
        write_boxed_handle(out_result, result);
        SidereonStatus::Ok
    })
}

fn antex_result_ok(text: String) -> SidereonAntexResult {
    SidereonAntexResult {
        outcome: SidereonAntexOutcome {
            is_ok: true,
            status: SidereonStatus::Ok,
            error: no_antex_error(),
        },
        text,
        message: String::new(),
        error: None,
    }
}

/// Record the whole failure. Nothing partial is kept from a refused operation.
fn antex_result_refused(fn_name: &str, err: AntexError) -> SidereonAntexResult {
    SidereonAntexResult {
        outcome: SidereonAntexOutcome {
            is_ok: false,
            status: SidereonStatus::InvalidArgument,
            error: antex_error_to_c(&err),
        },
        text: String::new(),
        message: format!("{fn_name}: {err}"),
        error: Some(err),
    }
}

/// Release an owned ANTEX result. Passing NULL is a no-op.
///
/// Safety: `result` may be NULL; otherwise it must be a live
/// SidereonAntexResult handle this binding produced, passed here exactly once.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antex_result_free(result: *mut SidereonAntexResult) {
    ffi_boundary("sidereon_antex_result_free", (), || {
        free_boxed(result);
    });
}

/// Copy the fixed-width outcome of an owned ANTEX result. `*out_outcome` is
/// written before the result pointer is validated.
///
/// Safety: `result` must be a live SidereonAntexResult handle; `out_outcome`
/// must point to one writable SidereonAntexOutcome.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antex_result_get_outcome(
    result: *const SidereonAntexResult,
    out_outcome: *mut SidereonAntexOutcome,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_antex_result_get_outcome";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_outcome = c_try!(require_out(out_outcome, FN_NAME, "out_outcome"));
        *out_outcome = SidereonAntexOutcome {
            is_ok: false,
            status: SidereonStatus::InvalidArgument,
            error: no_antex_error(),
        };
        let result = c_try!(require_ref(result, FN_NAME, "result"));
        *out_outcome = result.outcome;
        SidereonStatus::Ok
    })
}

/// Copy the ANTEX text of an owned encode result. Uses the variable-length
/// output contract; the bytes are not null-terminated.
///
/// A refused result has no text: the call returns
/// SIDEREON_STATUS_INVALID_ARGUMENT, reports a required length of zero, writes
/// nothing, and sets the thread-local message to the result's own refusal text.
/// A successful parse result holds no text and reports a required length of
/// zero.
///
/// Safety: `result` must be a live SidereonAntexResult handle; `out` may be
/// NULL only when `len` is 0; `out_written` and `out_required` must each point
/// to a writable size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antex_result_get_text(
    result: *const SidereonAntexResult,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_antex_result_get_text";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let result = c_try!(require_ref(result, FN_NAME, "result"));
        if !result.outcome.is_ok {
            set_last_error(result.message.clone());
            return result.outcome.status;
        }
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            result.text.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Copy one text part of an owned result's failure, selected by a
/// SidereonAntexErrorText value; Message is prefixed with the route that
/// produced it. A successful result, or a part the failure does not carry,
/// copies nothing. Uses the variable-length output contract; the bytes are
/// copied verbatim and not null-terminated.
///
/// Safety: as sidereon_antex_result_get_text.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antex_result_error_text(
    result: *const SidereonAntexResult,
    part: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_antex_result_error_text";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let result = c_try!(require_ref(result, FN_NAME, "result"));
        if part > SidereonAntexErrorText::Reason as u32 {
            set_last_error(format!("{FN_NAME}: invalid part {part}"));
            return SidereonStatus::InvalidArgument;
        }
        let text = if part == SidereonAntexErrorText::Message as u32 {
            result.message.clone()
        } else {
            result
                .error
                .as_ref()
                .and_then(|err| antex_error_text(err, part))
                .unwrap_or_default()
        };
        c_try!(copy_prefix_to_c(
            FN_NAME,
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

/// Release an ANTEX product handle. Passing NULL is a no-op.
///
/// Safety: antex must be NULL or a live handle from an ANTEX parse route that
/// has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antex_free(antex: *mut SidereonAntex) {
    ffi_boundary("sidereon_antex_free", (), || {
        free_boxed(antex);
    });
}

#[cfg(test)]
mod antex_c_tests {
    use super::*;

    const ANTEX_FIXTURE: &str = include_str!("../tests/fixtures/antex/igs20_wettzell_trim.atx");

    fn text_of(
        call: impl Fn(*mut u8, usize, *mut usize, *mut usize) -> SidereonStatus,
    ) -> (SidereonStatus, String) {
        let mut written = 0;
        let mut required = 0;
        let status = call(ptr::null_mut(), 0, &mut written, &mut required);
        if status != SidereonStatus::Ok {
            return (status, String::new());
        }
        let mut bytes = vec![0u8; required];
        let status = call(bytes.as_mut_ptr(), bytes.len(), &mut written, &mut required);
        bytes.truncate(written);
        (status, String::from_utf8(bytes).expect("UTF-8"))
    }

    #[test]
    fn encode_result_carries_the_text_of_a_writable_product() {
        let handle = SidereonAntex {
            inner: Antex::parse(ANTEX_FIXTURE).expect("parse ANTEX fixture"),
        };
        unsafe {
            let mut result = ptr::null_mut();
            assert_eq!(
                sidereon_antex_encode_result(&handle, &mut result),
                SidereonStatus::Ok
            );
            let owned = &*result;
            assert!(owned.outcome.is_ok);
            assert_eq!(owned.outcome.error.kind, SidereonAntexErrorKind::None);
            assert_eq!(
                owned.text,
                handle.inner.encode().expect("the fixture writes back")
            );
            assert!(owned.message.is_empty() && owned.error.is_none());
            sidereon_antex_result_free(result);
        }
    }

    #[test]
    fn encode_refusal_keeps_the_unwritable_field_and_reason() {
        let mut antex = Antex::parse(ANTEX_FIXTURE).expect("parse ANTEX fixture");
        // The public antenna map no longer matches the validity intervals the
        // product retains, so one of the two would be lost in the text.
        antex.antennas.clear();
        let handle = SidereonAntex { inner: antex };
        // sidereon-core's own refusal of the same product.
        let Err(AntexError::Unwritable {
            field: core_field,
            reason: core_reason,
        }) = handle.inner.encode()
        else {
            panic!("sidereon-core writes a product whose antenna map diverges");
        };
        unsafe {
            let mut written = usize::MAX;
            let mut required = usize::MAX;
            assert_eq!(
                sidereon_antex_encode(&handle, ptr::null_mut(), 0, &mut written, &mut required),
                SidereonStatus::InvalidArgument
            );
            assert_eq!((written, required), (0, 0));
            let mut last = no_antex_error();
            assert_eq!(sidereon_last_antex_error(&mut last), SidereonStatus::Ok);
            assert_eq!(last.kind, SidereonAntexErrorKind::Unwritable);

            let mut result = ptr::null_mut();
            assert_eq!(
                sidereon_antex_encode_result(&handle, &mut result),
                SidereonStatus::Ok
            );
            let owned = &*result;
            assert!(!owned.outcome.is_ok);
            assert_eq!(owned.outcome.status, SidereonStatus::InvalidArgument);
            let error = owned.outcome.error;
            assert_eq!(error.kind, SidereonAntexErrorKind::Unwritable);
            assert!(error.has_field && error.has_reason && !error.has_antenna_id);
            assert!(owned.text.is_empty());
            assert!(owned.message.starts_with("sidereon_antex_encode_result: "));
            let (status, field) = text_of(|out, len, written, required| {
                sidereon_antex_result_error_text(
                    result,
                    SidereonAntexErrorText::Field as u32,
                    out,
                    len,
                    written,
                    required,
                )
            });
            assert_eq!((status, field.as_str()), (SidereonStatus::Ok, core_field));
            let (_, reason) = text_of(|out, len, written, required| {
                sidereon_antex_result_error_text(
                    result,
                    SidereonAntexErrorText::Reason as u32,
                    out,
                    len,
                    written,
                    required,
                )
            });
            assert_eq!(reason, core_reason);

            written = usize::MAX;
            required = usize::MAX;
            assert_eq!(
                sidereon_antex_result_get_text(
                    result,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required
                ),
                SidereonStatus::InvalidArgument
            );
            assert_eq!((written, required), (0, 0));
            sidereon_antex_result_free(result);
        }
    }

    #[test]
    fn the_retained_header_and_blocks_read_back_exactly() {
        let handle = SidereonAntex {
            inner: Antex::parse(ANTEX_FIXTURE).expect("parse ANTEX fixture"),
        };
        // Every expected value is sidereon-core's own reading of the fixture;
        // the texts compared as literals are stated verbatim in it.
        let core = &handle.inner;
        let core_version = core.header.version.as_ref().expect("ANTEX VERSION record");
        let core_blocks: Vec<_> = core.antenna_blocks().collect();
        unsafe {
            let mut header = std::mem::zeroed::<SidereonAntexHeader>();
            assert_eq!(
                sidereon_antex_header(&handle, &mut header),
                SidereonStatus::Ok
            );
            assert!(header.has_version);
            assert_eq!(header.version.to_bits(), core_version.version.to_bits());
            assert!(header.has_system);
            assert_eq!(
                header.system,
                u32::from(core_version.system.expect("system"))
            );
            assert!(header.has_pcv_type);
            let core_pcv = core.header.pcv_type.as_ref().expect("PCV TYPE record");
            assert_eq!(
                header.pcv_type,
                match core_pcv.pcv_type {
                    PcvType::Absolute => SidereonAntexPcvType::Absolute as u32,
                    PcvType::Relative => SidereonAntexPcvType::Relative as u32,
                }
            );
            assert_eq!(
                header.has_reference_antenna,
                core_pcv.reference_antenna().is_some()
            );
            assert_eq!(header.comment_count, core.header.comments.len());
            assert_eq!(header.end_of_header, core.header.end_of_header);

            let mut blocks = 0;
            assert_eq!(
                sidereon_antex_block_count(&handle, &mut blocks),
                SidereonStatus::Ok
            );
            assert_eq!(blocks, core_blocks.len());

            // The first block, a GPS satellite antenna with a validity bound.
            let core_first = core_blocks[0];
            let mut first = ptr::null_mut();
            assert_eq!(
                sidereon_antex_block(&handle, 0, &mut first),
                SidereonStatus::Ok
            );
            let mut info = std::mem::zeroed::<SidereonAntennaInfo>();
            assert_eq!(sidereon_antenna_info(first, &mut info), SidereonStatus::Ok);
            assert_eq!(info.kind, SidereonAntennaKind::Satellite as u32);
            assert_eq!(core_first.kind, AntennaKind::Satellite);
            assert!(info.has_dazi_deg);
            assert_eq!(
                info.dazi_deg.to_bits(),
                core_first.dazi_deg.expect("DAZI").to_bits()
            );
            let core_grid = core_first.zenith_grid.expect("zenith grid");
            assert!(info.has_zenith_grid);
            assert_eq!(info.zenith_end_deg.to_bits(), core_grid.end_deg.to_bits());
            assert_eq!(
                info.has_frequency_count_record,
                core_first.has_frequency_count
            );
            assert_eq!(info.frequency_count, core_first.frequencies.len());
            assert_eq!(info.has_valid_until, core_first.valid_until.is_some());
            assert!(info.has_valid_from);
            assert_eq!(
                info.valid_from,
                antex_date_time_to_c(core_first.valid_from.expect("VALID FROM"))
            );
            let (_, serial) = text_of(|out, len, written, required| {
                sidereon_antenna_text(
                    first,
                    SidereonAntennaText::Serial as u32,
                    out,
                    len,
                    written,
                    required,
                )
            });
            // igs20_wettzell_trim.atx line 550 states the serial G05.
            assert_eq!(serial, "G05");
            let (_, sinex) = text_of(|out, len, written, required| {
                sidereon_antenna_text(
                    first,
                    SidereonAntennaText::SinexCode as u32,
                    out,
                    len,
                    written,
                    required,
                )
            });
            // igs20_wettzell_trim.atx line 556 states the SINEX code.
            assert_eq!(sinex, "IGS20_2417");
            // One tenth of a microsecond before the VALID FROM bound: the
            // bound is compared with every digit of its second.
            let at = SidereonAntexDateTime {
                year: 2009,
                month: 8,
                day: 16,
                hour: 23,
                minute: 59,
                second: 59,
                fraction_digits: 9_999_999,
                fraction_scale: 7,
            };
            let mut valid = true;
            assert_eq!(
                sidereon_antenna_valid_at(first, &at, &mut valid),
                SidereonStatus::Ok
            );
            let core_at = antex_date_time_from_c("test", &at).expect("valid instant");
            assert_eq!(valid, core_first.valid_at(core_at));
            sidereon_antenna_free(first);

            // The last block, a receiver antenna with its frequency sections
            // in file order and its method record.
            let last_index = core_blocks.len() - 1;
            let core_last = core_blocks[last_index];
            let mut last = ptr::null_mut();
            assert_eq!(
                sidereon_antex_block(&handle, last_index, &mut last),
                SidereonStatus::Ok
            );
            assert_eq!(sidereon_antenna_info(last, &mut info), SidereonStatus::Ok);
            assert_eq!(info.kind, SidereonAntennaKind::Receiver as u32);
            assert_eq!(core_last.kind, AntennaKind::Receiver);
            assert_eq!(info.frequency_count, core_last.frequencies.len());
            assert_eq!(info.calibration_count, core_last.calibrations.len());
            assert_eq!(info.comment_count, core_last.comments.len());
            assert_eq!(
                info.dazi_deg.to_bits(),
                core_last.dazi_deg.expect("DAZI").to_bits()
            );
            assert_eq!(
                info.zenith_step_deg.to_bits(),
                core_last
                    .zenith_grid
                    .expect("zenith grid")
                    .step_deg
                    .to_bits()
            );
            let (_, label) = text_of(|out, len, written, required| {
                sidereon_antenna_frequency_label(last, 1, out, len, written, required)
            });
            // igs20_wettzell_trim.atx line 805 opens the block's second
            // frequency section, E01.
            assert_eq!(label, "E01");
            let mut calibration = SidereonAntexCalibration {
                has_antennas_calibrated: false,
                antennas_calibrated: 0,
            };
            assert_eq!(
                sidereon_antenna_calibration(last, 0, &mut calibration),
                SidereonStatus::Ok
            );
            let core_calibration = &core_last.calibrations[0];
            assert_eq!(
                calibration.has_antennas_calibrated,
                core_calibration.antennas_calibrated.is_some()
            );
            assert_eq!(
                calibration.antennas_calibrated,
                core_calibration.antennas_calibrated.unwrap_or(0)
            );
            let (_, agency) = text_of(|out, len, written, required| {
                sidereon_antenna_calibration_text(
                    last,
                    0,
                    SidereonAntexCalibrationText::Agency as u32,
                    out,
                    len,
                    written,
                    required,
                )
            });
            // igs20_wettzell_trim.atx line 712 states the agency.
            assert_eq!(agency, "Geo++ GmbH");
            let mut frequency = std::mem::zeroed::<SidereonAntexFrequencyInfo>();
            assert_eq!(
                sidereon_antenna_frequency(last, 0, &mut frequency),
                SidereonStatus::Ok
            );
            let core_frequency = &core_last.frequencies[0];
            for axis in 0..3 {
                assert_eq!(
                    frequency.pco_m[axis].to_bits(),
                    core_frequency.pco_m[axis].to_bits()
                );
            }
            assert_eq!(frequency.has_rms, core_frequency.rms.is_some());
            // An absent RMS section leaves the RMS offsets NaN.
            assert!(frequency.has_rms || frequency.rms_pco_m[0].is_nan());
            sidereon_antenna_free(last);
        }
    }

    #[test]
    fn a_leap_second_bound_is_refused_typed() {
        let handle = SidereonAntex {
            inner: Antex::parse(ANTEX_FIXTURE).expect("parse ANTEX fixture"),
        };
        let leap = SidereonAntexDateTime {
            year: 2016,
            month: 12,
            day: 31,
            hour: 23,
            minute: 59,
            second: 60,
            fraction_digits: 0,
            fraction_scale: 0,
        };
        // sidereon-core's own reading of the same instant.
        assert_eq!(
            AntexDateTime::new_with_fraction(
                leap.year,
                leap.month,
                leap.day,
                leap.hour,
                leap.minute,
                leap.second,
                SecondFraction::new(0, 0).expect("zero fraction"),
            ),
            Err(AntexError::InvalidDateTime)
        );
        unsafe {
            let mut antenna = ptr::null_mut();
            let id = CString::new("G05").expect("prn");
            assert_eq!(
                sidereon_antex_satellite_antenna(&handle, id.as_ptr(), &leap, &mut antenna),
                SidereonStatus::InvalidArgument
            );
            assert!(antenna.is_null());
            let mut last = no_antex_error();
            sidereon_last_antex_error(&mut last);
            assert_eq!(last.kind, SidereonAntexErrorKind::InvalidDateTime);
        }
    }
}
