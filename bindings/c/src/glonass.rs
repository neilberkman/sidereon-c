use super::*;

/// One GLONASS satellite's FDMA carrier channel, used to resolve the
/// per-satellite G1 frequency for the ionosphere `(f_L1 / f_k)^2` scaling.
/// Supplied as an array on SidereonSppInputsV2; see glonass_channels there.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonGlonassChannel {
    /// GLONASS slot (PRN), e.g. 1 for R01.
    pub slot: u8,
    /// FDMA frequency channel k, valid range [-7, +6] (engine-enforced).
    pub channel: i8,
}

/// GLONASS G1 FDMA carrier frequency in Hz for an integer channel. Delegates to
/// sidereon_core::frequencies::glonass_g1_frequency_hz (infallible).
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_glonass_g1_frequency_hz(
    channel: i8,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_glonass_g1_frequency_hz",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_glonass_g1_frequency_hz", "out"));
            *out = sidereon_core::frequencies::glonass_g1_frequency_hz(channel);
            SidereonStatus::Ok
        },
    )
}
