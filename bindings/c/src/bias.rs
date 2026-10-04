use super::*;

// --- GNSS code and phase bias products ---------------------------------------

pub const MAX_BIAS_OBS_BYTES: usize = 16;

pub const BIAS_OBS_C_BYTES: usize = MAX_BIAS_OBS_BYTES + 1;

pub const MAX_BIAS_TEXT_BYTES: usize = 64;

pub const BIAS_TEXT_C_BYTES: usize = MAX_BIAS_TEXT_BYTES + 1;

pub struct SidereonBiasSet {
    pub(crate) inner: BiasSet,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonBiasErrorKind {
    None = 0,
    InvalidInput = 1,
    InvalidEpoch = 2,
    UnknownObservable = 3,
    UnsupportedVersion = 4,
    MissingDcbMetadata = 5,
    MissingClockReference = 6,
    MissingWriterMetadata = 7,
    Utf8 = 8,
    Departure = 9,
    InvalidUtf8Line = 10,
    UnsupportedTimeSystem = 11,
    DcbRecordMismatch = 12,
    Unknown = 999,
}

#[repr(C)]
#[derive(Clone, Copy)]
/// Fixed numeric payload of the last bias operation error on this thread.
pub struct SidereonBiasErrorInfo {
    /// Stable engine-variant discriminant.
    pub kind: SidereonBiasErrorKind,
    /// Line number for line-bearing failures.
    pub line: usize,
    /// Record index for record-bearing failures.
    pub record: usize,
    /// Whether `time_scale` is present.
    pub has_time_scale: bool,
    /// SidereonTimeScale value for an unsupported-time-system failure.
    pub time_scale: u32,
    /// Full notice payload for a departure error; otherwise fields are inert.
    pub departure: SidereonBiasNotice,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
/// Text categories copied by `sidereon_last_bias_error_text`.
pub enum SidereonBiasErrorText {
    Message = 0,
    Field = 1,
    Reason = 2,
    Code = 3,
    Version = 4,
    DepartureNotice = 5,
}

thread_local! {
    static LAST_BIAS_ERROR: RefCell<Option<sidereon_core::bias::BiasError>> =
        const { RefCell::new(None) };
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonBiasKind {
    Osb = 0,
    Dsb = 1,
    Isb = 2,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonBiasMode {
    Absolute = 0,
    Relative = 1,
    Unspecified = 2,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonBiasTargetKind {
    System = 0,
    Satellite = 1,
    Receiver = 2,
    SatelliteReceiver = 3,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonBiasEpoch {
    pub year: i32,
    pub day_of_year: u16,
    pub second_of_day: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonBiasRecord {
    pub kind: SidereonBiasKind,
    pub target_kind: SidereonBiasTargetKind,
    pub system: SidereonGnssSystem,
    pub has_sat_id: bool,
    pub sat_id: SidereonSatelliteToken,
    pub station: [c_char; BIAS_TEXT_C_BYTES],
    pub svn: [c_char; BIAS_TEXT_C_BYTES],
    pub obs1: [c_char; BIAS_OBS_C_BYTES],
    pub has_obs2: bool,
    pub obs2: [c_char; BIAS_OBS_C_BYTES],
    pub has_valid_from: bool,
    pub valid_from: SidereonBiasEpoch,
    pub has_valid_until: bool,
    pub valid_until: SidereonBiasEpoch,
    pub value: f64,
    pub has_sigma: bool,
    pub sigma: f64,
    pub has_slope: bool,
    pub slope: f64,
    pub has_slope_sigma: bool,
    pub slope_sigma: f64,
    /// True for a phase bias, one whose observable is an `L` code.
    pub is_phase: bool,
    /// Code, phase or mixed, from the observable codes (Bias-SINEX 1.00
    /// section 4.8).
    pub family: SidereonBiasObservableFamily,
    /// Unit the source row states its values in. A nanosecond row holds its
    /// value in seconds, a cycle row in cycles.
    pub unit: SidereonBiasUnit,
    /// Whether `line` is present.
    pub has_line: bool,
    /// One-based source line of the row, when read from text.
    pub line: usize,
}

/// Whether a bias record is a code, a phase or a mixed code/phase bias.
/// Mirrors sidereon_core::bias::BiasObservableFamily.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonBiasObservableFamily {
    /// Every observable is a code observable.
    Code = 0,
    /// Every observable is a phase observable.
    Phase = 1,
    /// A DSB or ISB pairing a code with a phase observable. Code and phase
    /// lookups do not use it.
    Mixed = 2,
}

/// Unit a bias row states its values in. Mirrors
/// sidereon_core::bias::BiasUnit.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonBiasUnit {
    /// Nanoseconds; the record value is held in seconds.
    Nanoseconds = 0,
    /// Cycles.
    Cycles = 1,
}

/// How the Bias-SINEX and CODE DCB readers treat a file that departs from the
/// format. Mirrors sidereon_core::bias::BiasReadPolicy.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonBiasReadPolicy {
    /// Refuse the file, naming the first departure.
    Strict = 0,
    /// Read the file and report every departure as a notice.
    Lenient = 1,
}

/// Outcome of a bias lookup. Mirrors the variants of
/// sidereon_core::bias::BiasLookup.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonBiasLookupStatus {
    /// The value is available.
    Available = 0,
    /// No record covers the query.
    Absent = 1,
    /// The query epoch is not on the product's time scale, or the product
    /// declares no usable time scale.
    UnsupportedScale = 2,
    /// Several records apply at the query epoch and give different values.
    Ambiguous = 3,
    /// A phase bias stated in nanoseconds was requested in cycles without a
    /// carrier frequency.
    CarrierFrequencyRequired = 4,
    /// A carrier frequency given or needed is not finite and positive.
    InvalidCarrierFrequency = 5,
    /// No carrier frequency is known for an observable.
    CarrierFrequencyUnknown = 6,
    /// A sloped record has neither a start nor an end, so no epoch is defined
    /// for its value.
    UndefinedSlopeReference = 7,
    /// The query epoch cannot be converted to a split Julian date.
    InvalidEpoch = 8,
    /// An outcome a later engine adds that this binding has no code for yet;
    /// unknown_variant names it.
    Unknown = 999,
}

/// Result of one bias lookup. The fields that do not belong to `status` are
/// zero. Record indices are into the set's records
/// (sidereon_bias_set_record).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonBiasLookup {
    /// Outcome of the lookup.
    pub status: SidereonBiasLookupStatus,
    /// The value when `status` is AVAILABLE, in the unit the query names.
    pub value: f64,
    /// Number of record indices the value comes from (AVAILABLE) or that
    /// conflict (AMBIGUOUS). The lookup copies up to records_len of them into
    /// out_records.
    pub record_count: usize,
    /// Number of records that also cover the query epoch but are overridden
    /// by a later start (AVAILABLE). The lookup copies up to overridden_len of
    /// them into out_overridden.
    pub overridden_count: usize,
    /// The record named by CARRIER_FREQUENCY_REQUIRED or
    /// UNDEFINED_SLOPE_REFERENCE.
    pub record: usize,
    /// Whether the product has a time scale (UNSUPPORTED_SCALE).
    pub has_product_time_scale: bool,
    /// Product time scale as SidereonTimeScale (UNSUPPORTED_SCALE).
    pub product_time_scale: u32,
    /// Whether the query has a time scale (UNSUPPORTED_SCALE). The caller
    /// epoch is read on the product's scale, so it has none when the product
    /// has none.
    pub has_query_time_scale: bool,
    /// Query time scale as SidereonTimeScale (UNSUPPORTED_SCALE).
    pub query_time_scale: u32,
    /// Observable code with no known carrier (CARRIER_FREQUENCY_UNKNOWN).
    pub observable: [c_char; BIAS_OBS_C_BYTES],
    /// The engine's name for the outcome when `status` is UNKNOWN; empty
    /// otherwise.
    pub unknown_variant: [c_char; SIDEREON_UNKNOWN_VARIANT_C_BYTES],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonCodeDcbOptions {
    pub obs1: *const c_char,
    pub obs2: *const c_char,
    pub year: i32,
    pub month: u8,
    pub time_scale: u32,
    pub has_receiver_system: bool,
    pub receiver_system: u32,
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_bias_sinex_parse(
    bytes: *const u8,
    len: usize,
    out: *mut *mut SidereonBiasSet,
) -> SidereonStatus {
    let fn_name = "sidereon_bias_sinex_parse";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        clear_last_bias_error();
        let out = c_try!(require_out(out, fn_name, "out"));
        *out = ptr::null_mut();
        let data = c_try!(require_slice(bytes, len, fn_name, "bytes"));
        let inner = c_try!(guard(fn_name, SidereonStatus::InvalidArgument, || {
            sidereon::parse_bias_sinex(data)
        }));
        write_bias_handle(out, inner);
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_bias_sinex_parse_lossy(
    bytes: *const u8,
    len: usize,
    out: *mut *mut SidereonBiasSet,
) -> SidereonStatus {
    let fn_name = "sidereon_bias_sinex_parse_lossy";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        clear_last_bias_error();
        let out = c_try!(require_out(out, fn_name, "out"));
        *out = ptr::null_mut();
        let data = c_try!(require_slice(bytes, len, fn_name, "bytes"));
        let parsed = c_try!(guard(fn_name, SidereonStatus::InvalidArgument, || {
            sidereon::parse_bias_sinex_lossy(data)
        }));
        write_bias_handle(out, parsed.value);
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_bias_sinex_load(
    path: *const c_char,
    out: *mut *mut SidereonBiasSet,
) -> SidereonStatus {
    let fn_name = "sidereon_bias_sinex_load";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        clear_last_bias_error();
        let out = c_try!(require_out(out, fn_name, "out"));
        *out = ptr::null_mut();
        let path = c_try!(parse_c_string(fn_name, "path", path));
        let inner = c_try!(guard(fn_name, SidereonStatus::InvalidArgument, || {
            sidereon::load_bias_sinex(&path)
        }));
        write_bias_handle(out, inner);
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_bias_sinex_load_lossy(
    path: *const c_char,
    out: *mut *mut SidereonBiasSet,
) -> SidereonStatus {
    let fn_name = "sidereon_bias_sinex_load_lossy";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        clear_last_bias_error();
        let out = c_try!(require_out(out, fn_name, "out"));
        *out = ptr::null_mut();
        let path = c_try!(parse_c_string(fn_name, "path", path));
        let parsed = c_try!(guard(fn_name, SidereonStatus::InvalidArgument, || {
            sidereon::load_bias_sinex_lossy(&path)
        }));
        write_bias_handle(out, parsed.value);
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_code_dcb_parse(
    bytes: *const u8,
    len: usize,
    options: *const SidereonCodeDcbOptions,
    out: *mut *mut SidereonBiasSet,
) -> SidereonStatus {
    let fn_name = "sidereon_code_dcb_parse";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        clear_last_bias_error();
        let out = c_try!(require_out(out, fn_name, "out"));
        *out = ptr::null_mut();
        let data = c_try!(require_slice(bytes, len, fn_name, "bytes"));
        let options = c_try!(code_dcb_options_from_c(fn_name, options));
        let inner = c_try!(guard(fn_name, SidereonStatus::InvalidArgument, || {
            sidereon::parse_code_dcb(data, options)
        }));
        write_bias_handle(out, inner);
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_code_dcb_parse_lossy(
    bytes: *const u8,
    len: usize,
    options: *const SidereonCodeDcbOptions,
    out: *mut *mut SidereonBiasSet,
) -> SidereonStatus {
    let fn_name = "sidereon_code_dcb_parse_lossy";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        clear_last_bias_error();
        let out = c_try!(require_out(out, fn_name, "out"));
        *out = ptr::null_mut();
        let data = c_try!(require_slice(bytes, len, fn_name, "bytes"));
        let options = c_try!(code_dcb_options_from_c(fn_name, options));
        let parsed = c_try!(guard(fn_name, SidereonStatus::InvalidArgument, || {
            sidereon::parse_code_dcb_lossy(data, options)
        }));
        write_bias_handle(out, parsed.value);
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_code_dcb_load(
    path: *const c_char,
    options: *const SidereonCodeDcbOptions,
    out: *mut *mut SidereonBiasSet,
) -> SidereonStatus {
    let fn_name = "sidereon_code_dcb_load";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        clear_last_bias_error();
        let out = c_try!(require_out(out, fn_name, "out"));
        *out = ptr::null_mut();
        let path = c_try!(parse_c_string(fn_name, "path", path));
        let options = c_try!(code_dcb_options_from_c(fn_name, options));
        let inner = c_try!(guard(fn_name, SidereonStatus::InvalidArgument, || {
            sidereon::load_code_dcb(&path, options)
        }));
        write_bias_handle(out, inner);
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_code_dcb_load_lossy(
    path: *const c_char,
    options: *const SidereonCodeDcbOptions,
    out: *mut *mut SidereonBiasSet,
) -> SidereonStatus {
    let fn_name = "sidereon_code_dcb_load_lossy";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        clear_last_bias_error();
        let out = c_try!(require_out(out, fn_name, "out"));
        *out = ptr::null_mut();
        let path = c_try!(parse_c_string(fn_name, "path", path));
        let options = c_try!(code_dcb_options_from_c(fn_name, options));
        let parsed = c_try!(guard(fn_name, SidereonStatus::InvalidArgument, || {
            sidereon::load_code_dcb_lossy(&path, options)
        }));
        write_bias_handle(out, parsed.value);
        SidereonStatus::Ok
    })
}

fn bias_read_policy_from_c(fn_name: &str, policy: u32) -> Result<BiasReadPolicy, SidereonStatus> {
    match policy {
        x if x == SidereonBiasReadPolicy::Strict as u32 => Ok(BiasReadPolicy::Strict),
        x if x == SidereonBiasReadPolicy::Lenient as u32 => Ok(BiasReadPolicy::Lenient),
        other => {
            set_last_error(format!("{fn_name}: unknown bias read policy {other}"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

/// Parse Bias-SINEX bytes under `policy`, a SidereonBiasReadPolicy value.
/// SIDEREON_BIAS_READ_POLICY_LENIENT reads a file that departs from
/// Bias-SINEX 1.00 and records each departure as a notice
/// (sidereon_bias_set_notice). sidereon_bias_sinex_parse reads strictly.
///
/// Safety: bytes points to len bytes; out points to a SidereonBiasSet pointer.
#[no_mangle]
pub unsafe extern "C" fn sidereon_bias_sinex_parse_with_policy(
    bytes: *const u8,
    len: usize,
    policy: u32,
    out: *mut *mut SidereonBiasSet,
) -> SidereonStatus {
    let fn_name = "sidereon_bias_sinex_parse_with_policy";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        clear_last_bias_error();
        let out = c_try!(require_out(out, fn_name, "out"));
        *out = ptr::null_mut();
        let data = c_try!(require_slice(bytes, len, fn_name, "bytes"));
        let policy = c_try!(bias_read_policy_from_c(fn_name, policy));
        let parsed = c_try!(guard(fn_name, SidereonStatus::InvalidArgument, || {
            sidereon::parse_bias_sinex_lossy_with_policy(data, policy)
        }));
        write_bias_handle(out, parsed.value);
        SidereonStatus::Ok
    })
}

/// Read and parse a Bias-SINEX product (`.gz` is decompressed) under
/// `policy`, as sidereon_bias_sinex_parse_with_policy.
///
/// Safety: path is a null-terminated string; out points to a
/// SidereonBiasSet pointer.
#[no_mangle]
pub unsafe extern "C" fn sidereon_bias_sinex_load_with_policy(
    path: *const c_char,
    policy: u32,
    out: *mut *mut SidereonBiasSet,
) -> SidereonStatus {
    let fn_name = "sidereon_bias_sinex_load_with_policy";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        clear_last_bias_error();
        let out = c_try!(require_out(out, fn_name, "out"));
        *out = ptr::null_mut();
        let path = c_try!(parse_c_string(fn_name, "path", path));
        let policy = c_try!(bias_read_policy_from_c(fn_name, policy));
        let parsed = c_try!(guard(fn_name, SidereonStatus::InvalidArgument, || {
            sidereon::load_bias_sinex_lossy_with_policy(&path, policy)
        }));
        write_bias_handle(out, parsed.value);
        SidereonStatus::Ok
    })
}

/// Parse CODE DCB bytes under `policy`, a SidereonBiasReadPolicy value.
/// SIDEREON_BIAS_READ_POLICY_LENIENT reads a generated title whose
/// time-system label names no known scale, leaving the set without a time
/// scale, and records the departure as a notice.
///
/// Safety: bytes points to len bytes; options is NULL or points to a
/// SidereonCodeDcbOptions; out points to a SidereonBiasSet pointer.
#[no_mangle]
pub unsafe extern "C" fn sidereon_code_dcb_parse_with_policy(
    bytes: *const u8,
    len: usize,
    options: *const SidereonCodeDcbOptions,
    policy: u32,
    out: *mut *mut SidereonBiasSet,
) -> SidereonStatus {
    let fn_name = "sidereon_code_dcb_parse_with_policy";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        clear_last_bias_error();
        let out = c_try!(require_out(out, fn_name, "out"));
        *out = ptr::null_mut();
        let data = c_try!(require_slice(bytes, len, fn_name, "bytes"));
        let options = c_try!(code_dcb_options_from_c(fn_name, options));
        let policy = c_try!(bias_read_policy_from_c(fn_name, policy));
        let parsed = c_try!(guard(fn_name, SidereonStatus::InvalidArgument, || {
            sidereon::parse_code_dcb_lossy_with_policy(data, options, policy)
        }));
        write_bias_handle(out, parsed.value);
        SidereonStatus::Ok
    })
}

/// Read and parse a CODE DCB product (`.gz` is decompressed) under `policy`,
/// as sidereon_code_dcb_parse_with_policy.
///
/// Safety: path is a null-terminated string; options is NULL or points to a
/// SidereonCodeDcbOptions; out points to a SidereonBiasSet pointer.
#[no_mangle]
pub unsafe extern "C" fn sidereon_code_dcb_load_with_policy(
    path: *const c_char,
    options: *const SidereonCodeDcbOptions,
    policy: u32,
    out: *mut *mut SidereonBiasSet,
) -> SidereonStatus {
    let fn_name = "sidereon_code_dcb_load_with_policy";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        clear_last_bias_error();
        let out = c_try!(require_out(out, fn_name, "out"));
        *out = ptr::null_mut();
        let path = c_try!(parse_c_string(fn_name, "path", path));
        let options = c_try!(code_dcb_options_from_c(fn_name, options));
        let policy = c_try!(bias_read_policy_from_c(fn_name, policy));
        let parsed = c_try!(guard(fn_name, SidereonStatus::InvalidArgument, || {
            sidereon::load_code_dcb_lossy_with_policy(&path, options, policy)
        }));
        write_bias_handle(out, parsed.value);
        SidereonStatus::Ok
    })
}

unsafe fn bias_write_to_c(
    fn_name: &str,
    set: *const SidereonBiasSet,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
    write: impl FnOnce(&BiasSet) -> Result<Vec<u8>, sidereon_core::bias::BiasError>,
) -> SidereonStatus {
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        clear_last_bias_error();
        c_try!(init_copy_counts(fn_name, out_written, out_required));
        let set = c_try!(require_ref(set, fn_name, "set"));
        let bytes = match write(&set.inner) {
            Ok(bytes) => bytes,
            Err(err) => return record_bias_error(fn_name, err),
        };
        c_try!(copy_prefix_to_c(
            fn_name,
            "out",
            &bytes,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Write a bias set as Bias-SINEX UTF-8 text. If the buffer is too small, the
/// call copies its prefix and reports the required byte count. Typed writer
/// refusals are available from `sidereon_last_bias_error`.
///
/// Safety: set is a live handle; out points to len writable bytes or is NULL
/// when len is 0; out_written and out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_bias_sinex_to_text(
    set: *const SidereonBiasSet,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    bias_write_to_c(
        "sidereon_bias_sinex_to_text",
        set,
        out,
        len,
        out_written,
        out_required,
        |inner| sidereon_core::bias::write_bias_sinex(inner).map(String::into_bytes),
    )
}

/// Write a bias set as Bias-SINEX bytes, preserving non-UTF-8 source lines.
///
/// Safety: set is a live handle; out points to len writable bytes or is NULL
/// when len is 0; out_written and out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_bias_sinex_to_bytes(
    set: *const SidereonBiasSet,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    bias_write_to_c(
        "sidereon_bias_sinex_to_bytes",
        set,
        out,
        len,
        out_written,
        out_required,
        sidereon_core::bias::write_bias_sinex_bytes,
    )
}

/// Write a bias set as CODE DCB UTF-8 text. Typed writer refusals are available
/// from `sidereon_last_bias_error`.
///
/// Safety: set is a live handle; out points to len writable bytes or is NULL
/// when len is 0; out_written and out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_code_dcb_to_text(
    set: *const SidereonBiasSet,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    bias_write_to_c(
        "sidereon_code_dcb_to_text",
        set,
        out,
        len,
        out_written,
        out_required,
        |inner| sidereon_core::bias::write_code_dcb(inner).map(String::into_bytes),
    )
}

/// Write a bias set as CODE DCB bytes, preserving non-UTF-8 source lines.
///
/// Safety: set is a live handle; out points to len writable bytes or is NULL
/// when len is 0; out_written and out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_code_dcb_to_bytes(
    set: *const SidereonBiasSet,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    bias_write_to_c(
        "sidereon_code_dcb_to_bytes",
        set,
        out,
        len,
        out_written,
        out_required,
        sidereon_core::bias::write_code_dcb_bytes,
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_bias_set_record_count(
    set: *const SidereonBiasSet,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_bias_set_record_count",
        SidereonStatus::Panic,
        || {
            clear_last_bias_error();
            let out = c_try!(require_out(
                out_count,
                "sidereon_bias_set_record_count",
                "out_count"
            ));
            *out = 0;
            let set = c_try!(require_ref(set, "sidereon_bias_set_record_count", "set"));
            *out = set.inner.records().len();
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_bias_set_skipped_record_count(
    set: *const SidereonBiasSet,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_bias_set_skipped_record_count",
        SidereonStatus::Panic,
        || {
            clear_last_bias_error();
            let out = c_try!(require_out(
                out_count,
                "sidereon_bias_set_skipped_record_count",
                "out_count"
            ));
            *out = 0;
            let set = c_try!(require_ref(
                set,
                "sidereon_bias_set_skipped_record_count",
                "set"
            ));
            *out = set.inner.skipped_records();
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_bias_set_warning_count(
    set: *const SidereonBiasSet,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_bias_set_warning_count",
        SidereonStatus::Panic,
        || {
            clear_last_bias_error();
            let out = c_try!(require_out(
                out_count,
                "sidereon_bias_set_warning_count",
                "out_count"
            ));
            *out = 0;
            let set = c_try!(require_ref(set, "sidereon_bias_set_warning_count", "set"));
            *out = set.inner.diagnostics().warnings.len();
            SidereonStatus::Ok
        },
    )
}

/// Read the product's bias mode and time scale. A product without a usable
/// time scale (a lenient read of one without TIME_SYSTEM, or with a label
/// naming no scale) sets *out_has_time_scale to false and *out_time_scale to
/// zero.
#[no_mangle]
pub unsafe extern "C" fn sidereon_bias_set_mode(
    set: *const SidereonBiasSet,
    out_mode: *mut SidereonBiasMode,
    out_has_time_scale: *mut bool,
    out_time_scale: *mut u32,
) -> SidereonStatus {
    ffi_boundary("sidereon_bias_set_mode", SidereonStatus::Panic, || {
        clear_last_bias_error();
        let out_mode = c_try!(require_out(out_mode, "sidereon_bias_set_mode", "out_mode"));
        let out_has_time_scale = c_try!(require_out(
            out_has_time_scale,
            "sidereon_bias_set_mode",
            "out_has_time_scale"
        ));
        *out_has_time_scale = false;
        let out_time_scale = c_try!(require_out(
            out_time_scale,
            "sidereon_bias_set_mode",
            "out_time_scale"
        ));
        *out_time_scale = 0;
        let set = c_try!(require_ref(set, "sidereon_bias_set_mode", "set"));
        *out_mode = bias_mode_to_c(set.inner.mode());
        if let Some(scale) = set.inner.time_scale() {
            *out_has_time_scale = true;
            *out_time_scale = time_scale_to_c_code(scale);
        }
        SidereonStatus::Ok
    })
}

/// Write the number of notices the read recorded: departures accepted by a
/// lenient read, lines that are not valid UTF-8, and repeated or conflicting
/// declarations.
///
/// Safety: set is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_bias_set_notice_count(
    set: *const SidereonBiasSet,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_bias_set_notice_count",
        SidereonStatus::Panic,
        || {
            clear_last_bias_error();
            let out = c_try!(require_out(
                out_count,
                "sidereon_bias_set_notice_count",
                "out_count"
            ));
            *out = 0;
            let set = c_try!(require_ref(set, "sidereon_bias_set_notice_count", "set"));
            *out = set.inner.notices().len();
            SidereonStatus::Ok
        },
    )
}

/// What a bias notice reports. Mirrors sidereon_core::bias::BiasNotice.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonBiasNoticeKind {
    /// A departure a lenient read accepted; `departure` names it.
    Departure = 0,
    /// A line that is not valid UTF-8 (`line`); its bytes are kept exactly.
    InvalidUtf8 = 1,
    /// A declaration repeated with the same meaning (`line`, text KEYWORD).
    RepeatedDeclaration = 2,
    /// A declaration repeated with a different meaning (`line`, KEYWORD).
    ConflictingDeclaration = 3,
    /// Two records for one target and observable overlap (`first`, `second`,
    /// record indices).
    Overlap = 4,
    /// A CODE DCB title states no time system and no options were given.
    DcbTimeSystemAssumed = 5,
    /// A CODE DCB title names its time system by a constellation name
    /// (`line`, text LABEL).
    DcbTimeSystemAlias = 6,
    /// A notice a later engine adds; unknown_variant names it.
    Unknown = 999,
}

/// Which departure from Bias-SINEX 1.00 a DEPARTURE notice names. Mirrors
/// sidereon_core::bias::BiasDeparture.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonBiasDepartureKind {
    /// Not a departure notice.
    None = 0,
    /// The header line is not the section 4.1 layout (text REASON).
    HeaderLayout = 1,
    /// A version other than 1.00 (text VERSION).
    OtherVersion = 2,
    /// No `%=ENDBIA` footer.
    MissingFooter = 3,
    /// Content after the footer (`line`).
    ContentAfterFooter = 4,
    /// A control line where none belongs (`line`).
    UnexpectedControlLine = 5,
    /// A block never closed (`line`, text NAME).
    UnclosedBlock = 6,
    /// A block end with no start (`line`, NAME).
    UnopenedBlockEnd = 7,
    /// A block closed by another name (`line`, OPEN, CLOSE).
    MismatchedBlockEnd = 8,
    /// A block opened inside another (`line`, OPEN, INNER).
    NestedBlock = 9,
    /// A mandatory block is missing (NAME).
    MissingBlock = 10,
    /// A block section 2.1 does not allow (`line`, NAME).
    UnknownBlock = 11,
    /// Text after a block start name (`line`).
    BlockStartSuffix = 12,
    /// A data line outside any block (`line`).
    DataOutsideBlock = 13,
    /// A mandatory declaration is missing (KEYWORD).
    MissingDeclaration = 14,
    /// A bias mode the format does not define (`line`, LABEL).
    UnsupportedBiasMode = 15,
    /// A time system the format does not define (`line`, LABEL).
    NonStandardTimeSystem = 16,
    /// The header mode differs from BIAS_MODE (HEADER, `bias_mode`).
    HeaderModeMismatch = 17,
    /// A DCB title time-system label naming no scale (`line`, LABEL).
    UnknownDcbTimeSystem = 18,
    /// The header estimate count differs from the solution rows
    /// (`declared_count`, `solution_rows`).
    EstimateCountMismatch = 19,
    /// A departure a later engine adds; unknown_variant names it.
    Unknown = 999,
}

/// Which text part of a bias notice sidereon_bias_set_notice_text copies.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonBiasNoticeText {
    /// The declaration keyword.
    Keyword = 0,
    /// A label as written.
    Label = 1,
    /// A block name.
    Name = 2,
    /// The block that was open.
    Open = 3,
    /// The block end that was read.
    Close = 4,
    /// The block opened inside another.
    Inner = 5,
    /// The version as written.
    Version = 6,
    /// Why the header layout departs.
    Reason = 7,
    /// The header mode as written.
    Header = 8,
}

/// One bias notice, with every number the engine's notice carries. Its text
/// parts are copied with sidereon_bias_set_notice_text.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonBiasNotice {
    /// A SidereonBiasNoticeKind value.
    pub kind: u32,
    /// For a DEPARTURE notice, a SidereonBiasDepartureKind value; NONE
    /// otherwise.
    pub departure: u32,
    /// Whether `line` is present.
    pub has_line: bool,
    /// One-based source line.
    pub line: usize,
    /// For OVERLAP, the record with the earlier start.
    pub first: usize,
    /// For OVERLAP, the record with the later or equal start.
    pub second: usize,
    /// For ESTIMATE_COUNT_MISMATCH, the header count.
    pub declared_count: u64,
    /// For ESTIMATE_COUNT_MISMATCH, the solution rows.
    pub solution_rows: usize,
    /// For HEADER_MODE_MISMATCH, the BIAS_MODE declared.
    pub bias_mode: SidereonBiasMode,
    /// The engine's name for a kind or departure that reads UNKNOWN.
    pub unknown_variant: [c_char; SIDEREON_UNKNOWN_VARIANT_C_BYTES],
}

fn bias_notice_parts(
    notice: &sidereon_core::bias::BiasNotice,
) -> (SidereonBiasNotice, Vec<(SidereonBiasNoticeText, String)>) {
    use sidereon_core::bias::{BiasDeparture as D, BiasNotice as N};
    use SidereonBiasDepartureKind as DK;
    use SidereonBiasNoticeText as T;
    let mut out = SidereonBiasNotice {
        kind: SidereonBiasNoticeKind::Unknown as u32,
        departure: DK::None as u32,
        has_line: false,
        line: 0,
        first: 0,
        second: 0,
        declared_count: 0,
        solution_rows: 0,
        bias_mode: SidereonBiasMode::Unspecified,
        unknown_variant: [0; SIDEREON_UNKNOWN_VARIANT_C_BYTES],
    };
    let mut texts = Vec::new();
    let line = |out: &mut SidereonBiasNotice, value: usize| {
        out.has_line = true;
        out.line = value;
    };
    let kind = match notice {
        N::Departure(departure) => {
            let dk = match departure {
                D::HeaderLayout { reason } => {
                    texts.push((T::Reason, reason.to_string()));
                    DK::HeaderLayout
                }
                D::OtherVersion { version } => {
                    texts.push((T::Version, version.clone()));
                    DK::OtherVersion
                }
                D::MissingFooter => DK::MissingFooter,
                D::ContentAfterFooter { line: l } => {
                    line(&mut out, *l);
                    DK::ContentAfterFooter
                }
                D::UnexpectedControlLine { line: l } => {
                    line(&mut out, *l);
                    DK::UnexpectedControlLine
                }
                D::UnclosedBlock { name, line: l } => {
                    line(&mut out, *l);
                    texts.push((T::Name, name.clone()));
                    DK::UnclosedBlock
                }
                D::UnopenedBlockEnd { name, line: l } => {
                    line(&mut out, *l);
                    texts.push((T::Name, name.clone()));
                    DK::UnopenedBlockEnd
                }
                D::MismatchedBlockEnd {
                    open,
                    close,
                    line: l,
                } => {
                    line(&mut out, *l);
                    texts.push((T::Open, open.clone()));
                    texts.push((T::Close, close.clone()));
                    DK::MismatchedBlockEnd
                }
                D::NestedBlock {
                    open,
                    inner,
                    line: l,
                } => {
                    line(&mut out, *l);
                    texts.push((T::Open, open.clone()));
                    texts.push((T::Inner, inner.clone()));
                    DK::NestedBlock
                }
                D::MissingBlock { name } => {
                    texts.push((T::Name, name.to_string()));
                    DK::MissingBlock
                }
                D::UnknownBlock { name, line: l } => {
                    line(&mut out, *l);
                    texts.push((T::Name, name.clone()));
                    DK::UnknownBlock
                }
                D::BlockStartSuffix { line: l } => {
                    line(&mut out, *l);
                    DK::BlockStartSuffix
                }
                D::DataOutsideBlock { line: l } => {
                    line(&mut out, *l);
                    DK::DataOutsideBlock
                }
                D::MissingDeclaration { keyword } => {
                    texts.push((T::Keyword, keyword.to_string()));
                    DK::MissingDeclaration
                }
                D::UnsupportedBiasMode { line: l, label } => {
                    line(&mut out, *l);
                    texts.push((T::Label, label.clone()));
                    DK::UnsupportedBiasMode
                }
                D::NonStandardTimeSystem { line: l, label } => {
                    line(&mut out, *l);
                    texts.push((T::Label, label.clone()));
                    DK::NonStandardTimeSystem
                }
                D::HeaderModeMismatch {
                    header,
                    description,
                } => {
                    texts.push((T::Header, header.clone()));
                    out.bias_mode = bias_mode_to_c(*description);
                    DK::HeaderModeMismatch
                }
                D::UnknownDcbTimeSystem { line: l, label } => {
                    line(&mut out, *l);
                    texts.push((T::Label, label.clone()));
                    DK::UnknownDcbTimeSystem
                }
                D::EstimateCountMismatch {
                    declared,
                    solution_rows,
                } => {
                    out.declared_count = *declared;
                    out.solution_rows = *solution_rows;
                    DK::EstimateCountMismatch
                }
                // `BiasDeparture` is non-exhaustive: a departure a later
                // engine adds reads as UNKNOWN with its name.
                other => {
                    out.unknown_variant = unknown_variant_name(other);
                    DK::Unknown
                }
            };
            out.departure = dk as u32;
            SidereonBiasNoticeKind::Departure
        }
        N::InvalidUtf8 { line: l } => {
            line(&mut out, *l);
            SidereonBiasNoticeKind::InvalidUtf8
        }
        N::RepeatedDeclaration { line: l, keyword } => {
            line(&mut out, *l);
            texts.push((T::Keyword, keyword.to_string()));
            SidereonBiasNoticeKind::RepeatedDeclaration
        }
        N::ConflictingDeclaration { line: l, keyword } => {
            line(&mut out, *l);
            texts.push((T::Keyword, keyword.to_string()));
            SidereonBiasNoticeKind::ConflictingDeclaration
        }
        N::Overlap { first, second } => {
            out.first = *first;
            out.second = *second;
            SidereonBiasNoticeKind::Overlap
        }
        N::DcbTimeSystemAssumed => SidereonBiasNoticeKind::DcbTimeSystemAssumed,
        N::DcbTimeSystemAlias { line: l, label } => {
            line(&mut out, *l);
            texts.push((T::Label, label.clone()));
            SidereonBiasNoticeKind::DcbTimeSystemAlias
        }
        #[allow(unreachable_patterns)]
        other => {
            out.unknown_variant = unknown_variant_name(other);
            SidereonBiasNoticeKind::Unknown
        }
    };
    out.kind = kind as u32;
    (out, texts)
}

fn empty_bias_error() -> SidereonBiasErrorInfo {
    SidereonBiasErrorInfo {
        kind: SidereonBiasErrorKind::None,
        line: 0,
        record: 0,
        has_time_scale: false,
        time_scale: 0,
        departure: SidereonBiasNotice {
            kind: SidereonBiasNoticeKind::Unknown as u32,
            departure: SidereonBiasDepartureKind::None as u32,
            has_line: false,
            line: 0,
            first: 0,
            second: 0,
            declared_count: 0,
            solution_rows: 0,
            bias_mode: SidereonBiasMode::Unspecified,
            unknown_variant: [0; SIDEREON_UNKNOWN_VARIANT_C_BYTES],
        },
    }
}

fn bias_error_to_c(err: &sidereon_core::bias::BiasError) -> SidereonBiasErrorInfo {
    use sidereon_core::bias::BiasError as E;
    let mut out = empty_bias_error();
    out.departure = SidereonBiasNotice {
        kind: SidereonBiasNoticeKind::Unknown as u32,
        departure: SidereonBiasDepartureKind::None as u32,
        has_line: false,
        line: 0,
        first: 0,
        second: 0,
        declared_count: 0,
        solution_rows: 0,
        bias_mode: SidereonBiasMode::Unspecified,
        unknown_variant: [0; SIDEREON_UNKNOWN_VARIANT_C_BYTES],
    };
    match err {
        E::InvalidInput { .. } => out.kind = SidereonBiasErrorKind::InvalidInput,
        E::InvalidEpoch => out.kind = SidereonBiasErrorKind::InvalidEpoch,
        E::UnknownObservable { .. } => out.kind = SidereonBiasErrorKind::UnknownObservable,
        E::UnsupportedVersion { .. } => out.kind = SidereonBiasErrorKind::UnsupportedVersion,
        E::MissingDcbMetadata => out.kind = SidereonBiasErrorKind::MissingDcbMetadata,
        E::MissingClockReference => out.kind = SidereonBiasErrorKind::MissingClockReference,
        E::MissingWriterMetadata { .. } => out.kind = SidereonBiasErrorKind::MissingWriterMetadata,
        E::Utf8 => out.kind = SidereonBiasErrorKind::Utf8,
        E::Departure { departure } => {
            out.kind = SidereonBiasErrorKind::Departure;
            out.departure = bias_notice_parts(&sidereon_core::bias::BiasNotice::Departure(
                departure.clone(),
            ))
            .0;
        }
        E::InvalidUtf8Line { line } => {
            out.kind = SidereonBiasErrorKind::InvalidUtf8Line;
            out.line = *line;
        }
        E::UnsupportedTimeSystem { scale } => {
            out.kind = SidereonBiasErrorKind::UnsupportedTimeSystem;
            if let Some(scale) = scale {
                out.has_time_scale = true;
                out.time_scale = time_scale_to_c_code(*scale);
            }
        }
        E::DcbRecordMismatch { record, .. } => {
            out.kind = SidereonBiasErrorKind::DcbRecordMismatch;
            out.record = *record;
        }
    }
    out
}

fn bias_error_text(
    err: &sidereon_core::bias::BiasError,
    part: SidereonBiasErrorText,
    departure_part: u32,
) -> String {
    use sidereon_core::bias::BiasError as E;
    match (err, part) {
        (err, SidereonBiasErrorText::Message) => err.to_string(),
        (
            E::InvalidInput { field, .. } | E::MissingWriterMetadata { field },
            SidereonBiasErrorText::Field,
        ) => (*field).to_owned(),
        (E::InvalidInput { reason, .. }, SidereonBiasErrorText::Reason) => (*reason).to_owned(),
        (E::UnknownObservable { code }, SidereonBiasErrorText::Code) => code.clone(),
        (E::UnsupportedVersion { version }, SidereonBiasErrorText::Version) => version.clone(),
        (E::DcbRecordMismatch { field, .. }, SidereonBiasErrorText::Field) => (*field).to_owned(),
        (E::Departure { departure }, SidereonBiasErrorText::DepartureNotice)
            if departure_part <= SidereonBiasNoticeText::Header as u32 =>
        {
            bias_notice_parts(&sidereon_core::bias::BiasNotice::Departure(
                departure.clone(),
            ))
            .1
            .into_iter()
            .find(|(kind, _)| *kind as u32 == departure_part)
            .map(|(_, text)| text)
            .unwrap_or_default()
        }
        _ => String::new(),
    }
}

fn clear_last_bias_error() {
    LAST_BIAS_ERROR.with(|slot| *slot.borrow_mut() = None);
}

fn record_bias_error(fn_name: &str, err: sidereon_core::bias::BiasError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    LAST_BIAS_ERROR.with(|slot| *slot.borrow_mut() = Some(err));
    SidereonStatus::InvalidArgument
}

fn guard<T>(
    fn_name: &str,
    status_on_err: SidereonStatus,
    body: impl FnOnce() -> sidereon::Result<T>,
) -> Result<T, SidereonStatus> {
    match body() {
        Ok(value) => Ok(value),
        Err(err) => {
            crate::engine_error::RecordEngineError::record_engine_error(fn_name, &err);
            if let sidereon::Error::Bias(ref bias_err) = err {
                LAST_BIAS_ERROR.with(|slot| *slot.borrow_mut() = Some(bias_err.clone()));
                set_last_error(bias_err.to_string());
            } else {
                set_last_error(err.to_string());
            }
            if super::ut1_refusal(&err) {
                Err(SidereonStatus::Ut1OutsideCoverage)
            } else {
                Err(status_on_err)
            }
        }
    }
}

/// Copy the typed details of the most recent failed bias read/write on this
/// thread. The error kind is None if no bias failure has been recorded.
/// Reading the record does not clear it.
///
/// Safety: out_error points to a SidereonBiasErrorInfo.
#[no_mangle]
pub unsafe extern "C" fn sidereon_last_bias_error(
    out_error: *mut SidereonBiasErrorInfo,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_last_bias_error";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_error, FN_NAME, "out_error"));
        *out = LAST_BIAS_ERROR.with(|slot| {
            slot.borrow()
                .as_ref()
                .map_or_else(empty_bias_error, bias_error_to_c)
        });
        SidereonStatus::Ok
    })
}

/// Copy a text part of the most recent typed bias error. For
/// DEPARTURE_NOTICE, departure_part is a SidereonBiasNoticeText value.
///
/// Safety: out points to len writable bytes or is NULL when len is zero;
/// out_written and out_required point to writable size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_last_bias_error_text(
    part: u32,
    departure_part: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_last_bias_error_text";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let part = match part {
            0 => SidereonBiasErrorText::Message,
            1 => SidereonBiasErrorText::Field,
            2 => SidereonBiasErrorText::Reason,
            3 => SidereonBiasErrorText::Code,
            4 => SidereonBiasErrorText::Version,
            5 => SidereonBiasErrorText::DepartureNotice,
            _ => {
                set_last_error(format!("{FN_NAME}: invalid part {part}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        if part == SidereonBiasErrorText::DepartureNotice
            && departure_part > SidereonBiasNoticeText::Header as u32
        {
            set_last_error(format!(
                "{FN_NAME}: invalid departure part {departure_part}"
            ));
            return SidereonStatus::InvalidArgument;
        }
        let text = LAST_BIAS_ERROR.with(|slot| {
            slot.borrow()
                .as_ref()
                .map(|err| bias_error_text(err, part, departure_part))
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

/// Copy notice `index` into *out_notice.
///
/// Safety: set is a live handle; out_notice points to a SidereonBiasNotice.
#[no_mangle]
pub unsafe extern "C" fn sidereon_bias_set_notice(
    set: *const SidereonBiasSet,
    index: usize,
    out_notice: *mut SidereonBiasNotice,
) -> SidereonStatus {
    let fn_name = "sidereon_bias_set_notice";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        clear_last_bias_error();
        let out = c_try!(require_out(out_notice, fn_name, "out_notice"));
        let set = c_try!(require_ref(set, fn_name, "set"));
        let notice = c_try!(bias_notice_at(fn_name, set, index));
        *out = bias_notice_parts(notice).0;
        SidereonStatus::Ok
    })
}

/// Copy text part `part` (a SidereonBiasNoticeText value) of notice `index`,
/// not null-terminated, under the variable-length output contract: call once
/// with out=NULL and len 0 to learn *out_required, then again with a buffer of
/// that size. A part the notice does not carry copies nothing.
///
/// Safety: set is a live handle; out points to len writable bytes or is NULL
/// when len is 0; out_written and out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_bias_set_notice_text(
    set: *const SidereonBiasSet,
    index: usize,
    part: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_bias_set_notice_text";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        clear_last_bias_error();
        c_try!(init_copy_counts(fn_name, out_written, out_required));
        let set = c_try!(require_ref(set, fn_name, "set"));
        let notice = c_try!(bias_notice_at(fn_name, set, index));
        if part > SidereonBiasNoticeText::Header as u32 {
            set_last_error(format!("{fn_name}: unknown text part {part}"));
            return SidereonStatus::InvalidArgument;
        }
        let text = bias_notice_parts(notice)
            .1
            .into_iter()
            .find(|(kind, _)| *kind as u32 == part)
            .map(|(_, text)| text)
            .unwrap_or_default();
        c_try!(copy_prefix_to_c(
            fn_name,
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

fn bias_notice_at<'a>(
    fn_name: &str,
    set: &'a SidereonBiasSet,
    index: usize,
) -> Result<&'a sidereon_core::bias::BiasNotice, SidereonStatus> {
    set.inner.notices().get(index).ok_or_else(|| {
        set_last_error(format!(
            "{fn_name}: index {index} out of range ({} notices)",
            set.inner.notices().len()
        ));
        SidereonStatus::InvalidArgument
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_bias_set_record(
    set: *const SidereonBiasSet,
    index: usize,
    out_record: *mut SidereonBiasRecord,
) -> SidereonStatus {
    ffi_boundary("sidereon_bias_set_record", SidereonStatus::Panic, || {
        clear_last_bias_error();
        let out = c_try!(require_out(
            out_record,
            "sidereon_bias_set_record",
            "out_record"
        ));
        let set = c_try!(require_ref(set, "sidereon_bias_set_record", "set"));
        let Some(record) = set.inner.records().get(index) else {
            set_last_error(format!(
                "sidereon_bias_set_record: index {index} out of range ({} records)",
                set.inner.records().len()
            ));
            return SidereonStatus::InvalidArgument;
        };
        *out = bias_record_to_c(record);
        SidereonStatus::Ok
    })
}

/// Look up the code OSB of `obs` for satellite `sat_id` at `epoch`, in
/// seconds. The epoch is read on the product's time scale; a product without
/// one gives SIDEREON_BIAS_LOOKUP_STATUS_UNSUPPORTED_SCALE. The outcome is
/// written to *out_lookup; up to records_len record indices go to out_records
/// and up to overridden_len overridden record indices to out_overridden. Each
/// buffer may be NULL with a zero length; the counts in *out_lookup give the
/// full lengths.
///
/// Safety: set is a live handle; sat_id and obs are null-terminated strings;
/// out_lookup points to a SidereonBiasLookup; out_records and out_overridden
/// are NULL with a zero length or point to that many size_t.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn sidereon_bias_set_code_osb_seconds(
    set: *const SidereonBiasSet,
    sat_id: *const c_char,
    obs: *const c_char,
    epoch: SidereonBiasEpoch,
    out_lookup: *mut SidereonBiasLookup,
    out_records: *mut usize,
    records_len: usize,
    out_overridden: *mut usize,
    overridden_len: usize,
) -> SidereonStatus {
    let fn_name = "sidereon_bias_set_code_osb_seconds";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        clear_last_bias_error();
        let out = c_try!(BiasLookupOut::new(
            fn_name,
            out_lookup,
            out_records,
            records_len,
            out_overridden,
            overridden_len
        ));
        let set = c_try!(require_ref(set, fn_name, "set"));
        let sat = c_try!(parse_satellite_token(fn_name, sat_id));
        let obs = c_try!(parse_bounded_c_string(
            fn_name,
            "obs",
            obs,
            MAX_BIAS_OBS_BYTES
        ));
        let lookup = match c_try!(bias_epoch_to_instant(fn_name, &set.inner, epoch)) {
            Some(instant) => set.inner.code_osb_seconds(sat, &obs, instant),
            None => {
                out.write_no_product_scale();
                return SidereonStatus::Ok;
            }
        };
        c_try!(out.write(&lookup));
        SidereonStatus::Ok
    })
}

/// Look up the phase OSB of `obs` for satellite `sat_id` at `epoch`, in
/// cycles. A phase bias stated in nanoseconds is converted with the carrier
/// frequency `carrier_hz`, used only when has_carrier_hz is true; without it
/// such a bias gives
/// SIDEREON_BIAS_LOOKUP_STATUS_CARRIER_FREQUENCY_REQUIRED. Epoch, outputs and
/// buffers are as for sidereon_bias_set_code_osb_seconds.
///
/// Safety: as for sidereon_bias_set_code_osb_seconds.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn sidereon_bias_set_phase_osb_cycles(
    set: *const SidereonBiasSet,
    sat_id: *const c_char,
    obs: *const c_char,
    epoch: SidereonBiasEpoch,
    has_carrier_hz: bool,
    carrier_hz: f64,
    out_lookup: *mut SidereonBiasLookup,
    out_records: *mut usize,
    records_len: usize,
    out_overridden: *mut usize,
    overridden_len: usize,
) -> SidereonStatus {
    let fn_name = "sidereon_bias_set_phase_osb_cycles";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        clear_last_bias_error();
        let out = c_try!(BiasLookupOut::new(
            fn_name,
            out_lookup,
            out_records,
            records_len,
            out_overridden,
            overridden_len
        ));
        let set = c_try!(require_ref(set, fn_name, "set"));
        let sat = c_try!(parse_satellite_token(fn_name, sat_id));
        let obs = c_try!(parse_bounded_c_string(
            fn_name,
            "obs",
            obs,
            MAX_BIAS_OBS_BYTES
        ));
        let carrier = has_carrier_hz.then_some(carrier_hz);
        let lookup = match c_try!(bias_epoch_to_instant(fn_name, &set.inner, epoch)) {
            Some(instant) => set.inner.phase_osb_cycles(sat, &obs, instant, carrier),
            None => {
                out.write_no_product_scale();
                return SidereonStatus::Ok;
            }
        };
        c_try!(out.write(&lookup));
        SidereonStatus::Ok
    })
}

/// Look up the code DSB `obs1` - `obs2` for satellite `sat_id` at `epoch`, in
/// seconds. Epoch, outputs and buffers are as for
/// sidereon_bias_set_code_osb_seconds.
///
/// Safety: as for sidereon_bias_set_code_osb_seconds; obs1 and obs2 are
/// null-terminated strings.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn sidereon_bias_set_code_dsb_seconds(
    set: *const SidereonBiasSet,
    sat_id: *const c_char,
    obs1: *const c_char,
    obs2: *const c_char,
    epoch: SidereonBiasEpoch,
    out_lookup: *mut SidereonBiasLookup,
    out_records: *mut usize,
    records_len: usize,
    out_overridden: *mut usize,
    overridden_len: usize,
) -> SidereonStatus {
    let fn_name = "sidereon_bias_set_code_dsb_seconds";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        clear_last_bias_error();
        let out = c_try!(BiasLookupOut::new(
            fn_name,
            out_lookup,
            out_records,
            records_len,
            out_overridden,
            overridden_len
        ));
        let set = c_try!(require_ref(set, fn_name, "set"));
        let sat = c_try!(parse_satellite_token(fn_name, sat_id));
        let obs1 = c_try!(parse_bounded_c_string(
            fn_name,
            "obs1",
            obs1,
            MAX_BIAS_OBS_BYTES
        ));
        let obs2 = c_try!(parse_bounded_c_string(
            fn_name,
            "obs2",
            obs2,
            MAX_BIAS_OBS_BYTES
        ));
        let lookup = match c_try!(bias_epoch_to_instant(fn_name, &set.inner, epoch)) {
            Some(instant) => set.inner.code_dsb_seconds(sat, &obs1, &obs2, instant),
            None => {
                out.write_no_product_scale();
                return SidereonStatus::Ok;
            }
        };
        c_try!(out.write(&lookup));
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_bias_set_free(set: *mut SidereonBiasSet) {
    clear_last_bias_error();
    free_boxed(set);
}

fn bias_mode_to_c(mode: BiasMode) -> SidereonBiasMode {
    match mode {
        BiasMode::Absolute => SidereonBiasMode::Absolute,
        BiasMode::Relative => SidereonBiasMode::Relative,
        BiasMode::Unspecified => SidereonBiasMode::Unspecified,
    }
}

fn bias_record_to_c(record: &BiasRecord) -> SidereonBiasRecord {
    let (target_kind, system, sat, station) = match &record.target {
        BiasTarget::System(system) => {
            (SidereonBiasTargetKind::System, *system, None, String::new())
        }
        BiasTarget::Satellite(sat) => (
            SidereonBiasTargetKind::Satellite,
            sat.system,
            Some(*sat),
            String::new(),
        ),
        BiasTarget::Receiver { system, station } => (
            SidereonBiasTargetKind::Receiver,
            *system,
            None,
            station.clone(),
        ),
        BiasTarget::SatelliteReceiver { sat, station } => (
            SidereonBiasTargetKind::SatelliteReceiver,
            sat.system,
            Some(*sat),
            station.clone(),
        ),
    };
    SidereonBiasRecord {
        kind: bias_kind_to_c(record.kind),
        target_kind,
        system: gnss_system_to_c(system),
        has_sat_id: sat.is_some(),
        sat_id: sat
            .map(satellite_token)
            .unwrap_or_else(|| satellite_token_from_text("")),
        station: fixed_c_chars(&station),
        svn: fixed_c_chars(record.svn.as_deref().unwrap_or("")),
        obs1: fixed_c_chars(&record.obs1),
        has_obs2: record.obs2.is_some(),
        obs2: fixed_c_chars(record.obs2.as_deref().unwrap_or("")),
        has_valid_from: record.valid_from.is_some(),
        valid_from: record
            .valid_from
            .map(bias_epoch_to_c)
            .unwrap_or_else(empty_bias_epoch),
        has_valid_until: record.valid_until.is_some(),
        valid_until: record
            .valid_until
            .map(bias_epoch_to_c)
            .unwrap_or_else(empty_bias_epoch),
        value: record.value,
        has_sigma: record.sigma.is_some(),
        sigma: record.sigma.unwrap_or(0.0),
        has_slope: record.slope.is_some(),
        slope: record.slope.unwrap_or(0.0),
        has_slope_sigma: record.slope_sigma.is_some(),
        slope_sigma: record.slope_sigma.unwrap_or(0.0),
        is_phase: record.is_phase(),
        family: match record.family {
            BiasObservableFamily::Code => SidereonBiasObservableFamily::Code,
            BiasObservableFamily::Phase => SidereonBiasObservableFamily::Phase,
            BiasObservableFamily::Mixed => SidereonBiasObservableFamily::Mixed,
        },
        unit: match record.unit {
            BiasUnit::Nanoseconds => SidereonBiasUnit::Nanoseconds,
            BiasUnit::Cycles => SidereonBiasUnit::Cycles,
        },
        has_line: record.line.is_some(),
        line: record.line.unwrap_or(0),
    }
}

unsafe fn code_dcb_options_from_c(
    fn_name: &str,
    options: *const SidereonCodeDcbOptions,
) -> Result<Option<CodeDcbOptions>, SidereonStatus> {
    let Some(options) = options.as_ref() else {
        return Ok(None);
    };
    let pair = (
        parse_bounded_c_string(fn_name, "options.obs1", options.obs1, MAX_BIAS_OBS_BYTES)?,
        parse_bounded_c_string(fn_name, "options.obs2", options.obs2, MAX_BIAS_OBS_BYTES)?,
    );
    let time_scale = time_scale_from_c_code(fn_name, "options.time_scale", options.time_scale)?;
    let receiver_system = if options.has_receiver_system {
        Some(gnss_system_from_c_code(
            fn_name,
            "options.receiver_system",
            options.receiver_system,
        )?)
    } else {
        None
    };
    let mut o = CodeDcbOptions::new(pair, options.year, options.month, time_scale);
    o.receiver_system = receiver_system;
    Ok(Some(o))
}

unsafe fn write_bias_handle(out: *mut *mut SidereonBiasSet, inner: BiasSet) {
    write_boxed_handle(out, SidereonBiasSet { inner });
}

/// Output slots of one bias lookup, validated and cleared before the lookup
/// runs.
struct BiasLookupOut {
    lookup: *mut SidereonBiasLookup,
    records: *mut usize,
    records_len: usize,
    overridden: *mut usize,
    overridden_len: usize,
}

impl BiasLookupOut {
    unsafe fn new(
        fn_name: &str,
        lookup: *mut SidereonBiasLookup,
        records: *mut usize,
        records_len: usize,
        overridden: *mut usize,
        overridden_len: usize,
    ) -> Result<Self, SidereonStatus> {
        let out = require_out(lookup, fn_name, "out_lookup")?;
        *out = empty_bias_lookup();
        if records.is_null() && records_len != 0 {
            set_last_error(format!("{fn_name}: null out_records"));
            return Err(SidereonStatus::NullPointer);
        }
        if overridden.is_null() && overridden_len != 0 {
            set_last_error(format!("{fn_name}: null out_overridden"));
            return Err(SidereonStatus::NullPointer);
        }
        validate_element_count::<usize>(fn_name, "records_len", records_len)?;
        validate_element_count::<usize>(fn_name, "overridden_len", overridden_len)?;
        Ok(Self {
            lookup,
            records,
            records_len,
            overridden,
            overridden_len,
        })
    }

    unsafe fn write(&self, lookup: &BiasLookup) -> Result<(), SidereonStatus> {
        let mut out = empty_bias_lookup();
        let copy = |values: &[usize], dst: *mut usize, len: usize| {
            let n = values.len().min(len);
            if n > 0 {
                ptr::copy_nonoverlapping(values.as_ptr(), dst, n);
            }
        };
        match lookup {
            BiasLookup::Available {
                value,
                records,
                overridden,
            } => {
                out.status = SidereonBiasLookupStatus::Available;
                out.value = *value;
                out.record_count = records.len();
                out.overridden_count = overridden.len();
                copy(records, self.records, self.records_len);
                copy(overridden, self.overridden, self.overridden_len);
            }
            BiasLookup::Absent => out.status = SidereonBiasLookupStatus::Absent,
            BiasLookup::UnsupportedScale { product, query } => {
                out.status = SidereonBiasLookupStatus::UnsupportedScale;
                out.has_product_time_scale = product.is_some();
                out.product_time_scale = product.map_or(0, time_scale_to_c_code);
                out.has_query_time_scale = true;
                out.query_time_scale = time_scale_to_c_code(*query);
            }
            BiasLookup::Ambiguous { records } => {
                out.status = SidereonBiasLookupStatus::Ambiguous;
                out.record_count = records.len();
                copy(records, self.records, self.records_len);
            }
            BiasLookup::CarrierFrequencyRequired { record } => {
                out.status = SidereonBiasLookupStatus::CarrierFrequencyRequired;
                out.record = *record;
            }
            BiasLookup::InvalidCarrierFrequency => {
                out.status = SidereonBiasLookupStatus::InvalidCarrierFrequency;
            }
            BiasLookup::CarrierFrequencyUnknown { observable } => {
                out.status = SidereonBiasLookupStatus::CarrierFrequencyUnknown;
                out.observable = fixed_c_chars(observable);
            }
            BiasLookup::UndefinedSlopeReference { record } => {
                out.status = SidereonBiasLookupStatus::UndefinedSlopeReference;
                out.record = *record;
            }
            BiasLookup::InvalidEpoch => out.status = SidereonBiasLookupStatus::InvalidEpoch,
            other => {
                out.status = SidereonBiasLookupStatus::Unknown;
                out.unknown_variant = unknown_variant_name(other);
            }
        }
        *self.lookup = out;
        Ok(())
    }

    /// The caller epoch is read on the product's time scale, so a product
    /// without one places it on no scale and no lookup can run.
    unsafe fn write_no_product_scale(&self) {
        let mut out = empty_bias_lookup();
        out.status = SidereonBiasLookupStatus::UnsupportedScale;
        *self.lookup = out;
    }
}

fn empty_bias_lookup() -> SidereonBiasLookup {
    SidereonBiasLookup {
        status: SidereonBiasLookupStatus::Absent,
        value: 0.0,
        record_count: 0,
        overridden_count: 0,
        record: 0,
        has_product_time_scale: false,
        product_time_scale: 0,
        has_query_time_scale: false,
        query_time_scale: 0,
        observable: [0; BIAS_OBS_C_BYTES],
        unknown_variant: [0; SIDEREON_UNKNOWN_VARIANT_C_BYTES],
    }
}

fn bias_kind_to_c(kind: BiasKind) -> SidereonBiasKind {
    match kind {
        BiasKind::Osb => SidereonBiasKind::Osb,
        BiasKind::Dsb => SidereonBiasKind::Dsb,
        BiasKind::Isb => SidereonBiasKind::Isb,
    }
}

fn bias_epoch_to_c(epoch: BiasEpoch) -> SidereonBiasEpoch {
    SidereonBiasEpoch {
        year: epoch.year,
        day_of_year: epoch.day_of_year,
        second_of_day: epoch.second_of_day,
    }
}

fn empty_bias_epoch() -> SidereonBiasEpoch {
    SidereonBiasEpoch {
        year: 0,
        day_of_year: 0,
        second_of_day: 0,
    }
}

fn bias_epoch_to_instant(
    fn_name: &str,
    set: &BiasSet,
    epoch: SidereonBiasEpoch,
) -> Result<Option<Instant>, SidereonStatus> {
    let epoch = bias_epoch_from_c(fn_name, epoch)?;
    let Some(scale) = set.time_scale() else {
        return Ok(None);
    };
    bias_epoch_instant(epoch, scale).map(Some).map_err(|err| {
        set_last_error(format!("{fn_name}: invalid bias epoch: {err}"));
        SidereonStatus::InvalidArgument
    })
}

#[cfg(test)]
mod typed_error_tests {
    use super::*;

    #[test]
    fn typed_bias_error_keeps_fields_scale_and_departure_notice() {
        let invalid = sidereon_core::bias::BiasError::InvalidInput {
            field: "month",
            reason: "out of range",
        };
        LAST_BIAS_ERROR.with(|slot| *slot.borrow_mut() = Some(invalid.clone()));
        let mapped = LAST_BIAS_ERROR.with(|slot| {
            slot.borrow()
                .as_ref()
                .map(bias_error_to_c)
                .expect("stored typed error")
        });
        assert_eq!(mapped.kind, SidereonBiasErrorKind::InvalidInput);
        assert_eq!(
            bias_error_text(&invalid, SidereonBiasErrorText::Field, 0),
            "month"
        );
        assert_eq!(
            bias_error_text(&invalid, SidereonBiasErrorText::Reason, 0),
            "out of range"
        );

        let unsupported = sidereon_core::bias::BiasError::UnsupportedTimeSystem {
            scale: Some(TimeScale::Gpst),
        };
        let mapped = bias_error_to_c(&unsupported);
        assert_eq!(mapped.kind, SidereonBiasErrorKind::UnsupportedTimeSystem);
        assert!(mapped.has_time_scale);
        assert_eq!(mapped.time_scale, SidereonTimeScale::Gpst as u32);

        let departure = sidereon_core::bias::BiasError::Departure {
            departure: sidereon_core::bias::BiasDeparture::OtherVersion {
                version: "2.00".to_owned(),
            },
        };
        let mapped = bias_error_to_c(&departure);
        assert_eq!(mapped.kind, SidereonBiasErrorKind::Departure);
        assert_eq!(
            mapped.departure.kind,
            SidereonBiasNoticeKind::Departure as u32
        );
        assert_eq!(
            bias_error_text(
                &departure,
                SidereonBiasErrorText::DepartureNotice,
                SidereonBiasNoticeText::Version as u32,
            ),
            "2.00"
        );
        clear_last_bias_error();
        assert!(LAST_BIAS_ERROR.with(|slot| slot.borrow().is_none()));
    }

    #[test]
    fn test_bias_strict_parse_refusal_preserves_legacy_and_records_payload() {
        use crate::engine_error::{
            clear_engine_error, sidereon_last_engine_error_info,
            sidereon_last_engine_error_payload, snapshot_engine_error_for_test,
            SidereonEngineErrorFamily, SidereonEngineErrorInfo,
        };
        use serde_json::Value;

        clear_engine_error();
        clear_last_bias_error();

        let invalid_bytes = b"%=BIA 1.00\n+BIAS/DESCRIPTION\n";
        let expected_inner_err = match sidereon::parse_bias_sinex(invalid_bytes) {
            Err(sidereon::Error::Bias(e)) => e,
            other => panic!("expected bias error, got {other:?}"),
        };

        let mut out_set = ptr::null_mut();
        let status = unsafe {
            sidereon_bias_sinex_parse(invalid_bytes.as_ptr(), invalid_bytes.len(), &mut out_set)
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);
        assert!(out_set.is_null());

        let mut legacy_buf = vec![0 as c_char; 512];
        let needed = unsafe {
            crate::sidereon_last_error_message(legacy_buf.as_mut_ptr(), legacy_buf.len())
        };
        assert!(needed > 0);
        let legacy_str = unsafe { std::ffi::CStr::from_ptr(legacy_buf.as_ptr()) }
            .to_str()
            .unwrap();
        assert_eq!(legacy_str, expected_inner_err.to_string());

        let stored_bias_err = LAST_BIAS_ERROR
            .with(|slot| slot.borrow().clone())
            .expect("stored bias error");
        assert_eq!(stored_bias_err, expected_inner_err);

        let mut info = SidereonEngineErrorInfo {
            family: SidereonEngineErrorFamily::None,
            payload_len: 0,
        };
        assert_eq!(
            unsafe { sidereon_last_engine_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.family, SidereonEngineErrorFamily::Facade);
        assert!(info.payload_len > 0);

        let mut payload_buf = vec![0u8; info.payload_len];
        let mut payload_written = 0;
        let mut required = 0;
        assert_eq!(
            unsafe {
                sidereon_last_engine_error_payload(
                    payload_buf.as_mut_ptr(),
                    payload_buf.len(),
                    &mut payload_written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(payload_written, info.payload_len);
        let payload: Value = serde_json::from_slice(&payload_buf).expect("valid JSON payload");
        assert_eq!(payload["schema_version"], 1);
        assert_eq!(payload["family"], "facade");
        assert_eq!(payload["operation"], "sidereon_bias_sinex_parse");
        assert_eq!(payload["error"]["kind"], "bias");
        assert!(payload["error"]["fields"]["cause"].is_object());

        let status = unsafe {
            sidereon_bias_sinex_parse(invalid_bytes.as_ptr(), invalid_bytes.len(), ptr::null_mut())
        };
        assert_eq!(status, SidereonStatus::NullPointer);
        assert_eq!(
            unsafe { sidereon_last_engine_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(info.payload_len, 0);

        // Reuse the valid Bias-SINEX writer input from round2_smoke.c as a
        // successful public producer control after a real recorded refusal.
        let valid_bytes = b"%=BIA 1.00 TST 2020:001:00000 TST 2020:001:00000 2020:011:00000 A 00000000\n+FILE/REFERENCE\n DESCRIPTION        TEST\n-FILE/REFERENCE\n+BIAS/DESCRIPTION\n BIAS_MODE                               ABSOLUTE\n TIME_SYSTEM                             G\n-BIAS/DESCRIPTION\n+BIAS/SOLUTION\n-BIAS/SOLUTION\n%=ENDBIA\n";
        let mut refused_again = ptr::null_mut();
        let status = unsafe {
            sidereon_bias_sinex_parse(
                invalid_bytes.as_ptr(),
                invalid_bytes.len(),
                &mut refused_again,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);
        assert!(refused_again.is_null());
        let (seed_info, seed_payload) =
            snapshot_engine_error_for_test().expect("real Bias refusal recorded");
        assert_eq!(seed_info.family, SidereonEngineErrorFamily::Facade);
        assert!(!seed_payload.is_empty());

        let mut live_set = ptr::null_mut();
        let status = unsafe {
            sidereon_bias_sinex_parse(valid_bytes.as_ptr(), valid_bytes.len(), &mut live_set)
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert!(!live_set.is_null());
        assert!(snapshot_engine_error_for_test().is_none());

        // Seed again and prove a real getter and free preserve the full payload.
        let status = unsafe {
            sidereon_bias_sinex_parse(
                invalid_bytes.as_ptr(),
                invalid_bytes.len(),
                &mut refused_again,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);
        assert!(refused_again.is_null());
        let (base_info, base_payload) =
            snapshot_engine_error_for_test().expect("Bias refusal recorded before getter");
        assert_eq!(base_info.family, SidereonEngineErrorFamily::Facade);
        assert!(!base_payload.is_empty());

        let mut mode = SidereonBiasMode::Unspecified;
        let mut has_time_scale = false;
        let mut time_scale = 0;
        assert_eq!(
            unsafe {
                sidereon_bias_set_mode(live_set, &mut mode, &mut has_time_scale, &mut time_scale)
            },
            SidereonStatus::Ok
        );
        assert_eq!(mode, SidereonBiasMode::Absolute);
        assert!(has_time_scale);
        let (after_getter_info, after_getter_payload) =
            snapshot_engine_error_for_test().expect("Bias error retained across live getter");
        assert_eq!(after_getter_info.family, base_info.family);
        assert_eq!(after_getter_info.payload_len, base_info.payload_len);
        assert_eq!(after_getter_payload.as_bytes(), base_payload.as_bytes());

        unsafe { sidereon_bias_set_free(live_set) };
        let (after_free_info, after_free_payload) =
            snapshot_engine_error_for_test().expect("Bias error retained across live free");
        assert_eq!(after_free_info.family, base_info.family);
        assert_eq!(after_free_info.payload_len, base_info.payload_len);
        assert_eq!(after_free_payload.as_bytes(), base_payload.as_bytes());

        clear_engine_error();
        clear_last_bias_error();
    }
}
