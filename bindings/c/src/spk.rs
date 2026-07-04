use super::*;

/// A parsed JPL/NAIF SPK (DAF/SPK .bsp) ephemeris kernel. Opaque to C. Create
/// with sidereon_spk_load and release with sidereon_spk_free.
pub struct SidereonSpk {
    pub(crate) inner: Spk,
}

/// State of an SPK target body relative to a center body at a queried epoch,
/// as produced by sidereon_spk_state. All vectors are in the kernel's own
/// reference frame (identified by `frame`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSpkState {
    /// NAIF target body identifier echoed from the query.
    pub target: i32,
    /// NAIF center body identifier echoed from the query.
    pub center: i32,
    /// Position of the target relative to the center, in kilometers.
    pub position_km: [f64; 3],
    /// Whether velocity_km_s is present. Type-3 and type-21 segments provide
    /// velocity; a path that traverses any position-only type-2 segment does
    /// not, in which case this is false and velocity_km_s is all zero.
    pub has_velocity_km_s: bool,
    /// Velocity of the target relative to the center, in kilometers per second,
    /// when has_velocity_km_s is true.
    pub velocity_km_s: [f64; 3],
    /// NAIF reference-frame identifier shared by all segments in the resolved
    /// path (0 for the trivial target == center query).
    pub frame: i32,
}

/// Parse a JPL/NAIF SPK (DAF/SPK `.bsp`) ephemeris kernel from a byte buffer.
/// On success writes a newly owned handle to *out_spk. Release it with
/// sidereon_spk_free.
///
/// Safety: data must point to len readable bytes; out_spk must point to storage
/// for a SidereonSpk*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spk_load(
    data: *const u8,
    len: usize,
    out_spk: *mut *mut SidereonSpk,
) -> SidereonStatus {
    ffi_boundary("sidereon_spk_load", SidereonStatus::Panic, || {
        let out_spk = c_try!(require_out(out_spk, "sidereon_spk_load", "out_spk"));
        *out_spk = ptr::null_mut();
        let bytes = c_try!(require_slice(data, len, "sidereon_spk_load", "data"));
        let inner = match Spk::from_bytes(bytes) {
            Ok(spk) => spk,
            Err(err) => return map_spk_error("sidereon_spk_load", err),
        };
        write_boxed_handle(out_spk, SidereonSpk { inner });
        SidereonStatus::Ok
    })
}

/// Query the state of NAIF `target` relative to NAIF `center` at
/// `et_seconds_tdb` (ET/TDB seconds past J2000), writing the resolved relative
/// state into *out_state. Resolves the segment chain connecting the two bodies
/// and evaluates SPK Types 2, 3, and 21. The numbers are exactly what the
/// engine's SPK reader produces.
///
/// Safety: spk must be a live handle from sidereon_spk_load that has not been
/// freed; out_state must point to a SidereonSpkState.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spk_state(
    spk: *const SidereonSpk,
    target: i32,
    center: i32,
    et_seconds_tdb: f64,
    out_state: *mut SidereonSpkState,
) -> SidereonStatus {
    ffi_boundary("sidereon_spk_state", SidereonStatus::Panic, || {
        let out_state = c_try!(require_out(out_state, "sidereon_spk_state", "out_state"));
        *out_state = empty_spk_state();
        let spk = c_try!(require_ref(spk, "sidereon_spk_state", "spk"));
        let state = match spk.inner.spk_state(target, center, et_seconds_tdb) {
            Ok(state) => state,
            Err(err) => return map_spk_error("sidereon_spk_state", err),
        };
        *out_state = spk_state_to_c(state);
        SidereonStatus::Ok
    })
}

/// Release an SPK kernel handle returned by sidereon_spk_load. Passing NULL is
/// a no-op.
///
/// Safety: spk must be NULL or a live handle from sidereon_spk_load that has not
/// already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spk_free(spk: *mut SidereonSpk) {
    ffi_boundary("sidereon_spk_free", (), || {
        free_boxed(spk);
    });
}

/// Map an SPK reader error to a binding status code, recording its message.
///
/// Malformed-kernel and bad-caller-input conditions report
/// SIDEREON_STATUS_INVALID_ARGUMENT (an unusable buffer or an unknown body /
/// non-finite epoch). Conditions where the kernel is well-formed but cannot
/// satisfy the query (no covering segment for the epoch, no connecting path, or
/// a segment data type the reader does not evaluate) report
/// SIDEREON_STATUS_SOLVE, mirroring the SP3 reader's argument-vs-range split.
fn map_spk_error(fn_name: &str, err: SpkError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        SpkError::Io { .. }
        | SpkError::Truncated { .. }
        | SpkError::UnsupportedDafId { .. }
        | SpkError::UnsupportedBinaryFormat { .. }
        | SpkError::UnsupportedSummaryShape { .. }
        | SpkError::InvalidField { .. }
        | SpkError::InvalidSegmentLayout { .. }
        | SpkError::InvalidDoubleField { .. }
        | SpkError::UnknownBody { .. } => SidereonStatus::InvalidArgument,
        SpkError::OutOfCoverage { .. }
        | SpkError::CoverageGap { .. }
        | SpkError::NoSegmentPath { .. }
        | SpkError::UnsupportedSegmentType { .. }
        | SpkError::UnsupportedStateSegmentType { .. }
        | SpkError::FrameMismatch { .. } => SidereonStatus::Solve,
    }
}

fn empty_spk_state() -> SidereonSpkState {
    SidereonSpkState {
        target: 0,
        center: 0,
        position_km: [0.0; 3],
        has_velocity_km_s: false,
        velocity_km_s: [0.0; 3],
        frame: 0,
    }
}

fn spk_state_to_c(state: SpkState) -> SidereonSpkState {
    SidereonSpkState {
        target: state.target,
        center: state.center,
        position_km: state.position_km,
        has_velocity_km_s: state.velocity_km_s.is_some(),
        velocity_km_s: state.velocity_km_s.unwrap_or([0.0; 3]),
        frame: state.frame,
    }
}
