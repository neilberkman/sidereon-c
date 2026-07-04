use super::*;

/// A parsed CCSDS OPM (Orbit Parameter Message). Opaque to C. Create with
/// sidereon_opm_parse_kvn or sidereon_opm_parse_xml; serialize with
/// sidereon_opm_to_kvn or sidereon_opm_to_xml; release with sidereon_opm_free.
pub struct SidereonOpm {
    pub(crate) inner: Opm,
}

/// Parse a CCSDS OPM in KVN (keyword=value) form. On success writes a newly
/// owned handle to *out_opm. Release it with sidereon_opm_free.
///
/// Safety: data must point to len readable bytes; out_opm must point to storage
/// for a SidereonOpm*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_opm_parse_kvn(
    data: *const u8,
    len: usize,
    out_opm: *mut *mut SidereonOpm,
) -> SidereonStatus {
    ffi_boundary("sidereon_opm_parse_kvn", SidereonStatus::Panic, || {
        let out_opm = c_try!(require_out(out_opm, "sidereon_opm_parse_kvn", "out_opm"));
        *out_opm = ptr::null_mut();
        let text = c_try!(ndm_text_from_utf8(data, len, "sidereon_opm_parse_kvn"));
        let inner = match core_opm::parse_kvn(text) {
            Ok(opm) => opm,
            Err(err) => {
                set_last_error(format!("sidereon_opm_parse_kvn: {err}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        write_boxed_handle(out_opm, SidereonOpm { inner });
        SidereonStatus::Ok
    })
}

/// Parse a CCSDS OPM in XML (NDM/XML) form. On success writes a newly owned
/// handle to *out_opm. Release it with sidereon_opm_free.
///
/// Safety: data must point to len readable bytes; out_opm must point to storage
/// for a SidereonOpm*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_opm_parse_xml(
    data: *const u8,
    len: usize,
    out_opm: *mut *mut SidereonOpm,
) -> SidereonStatus {
    ffi_boundary("sidereon_opm_parse_xml", SidereonStatus::Panic, || {
        let out_opm = c_try!(require_out(out_opm, "sidereon_opm_parse_xml", "out_opm"));
        *out_opm = ptr::null_mut();
        let text = c_try!(ndm_text_from_utf8(data, len, "sidereon_opm_parse_xml"));
        let inner = match core_opm::parse_xml(text) {
            Ok(opm) => opm,
            Err(err) => {
                set_last_error(format!("sidereon_opm_parse_xml: {err}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        write_boxed_handle(out_opm, SidereonOpm { inner });
        SidereonStatus::Ok
    })
}

/// Serialize an OPM to KVN text. The output is not null-terminated. Uses the
/// variable-length output contract documented at the top of the header: call once
/// with out=NULL to learn *out_required, then again with a buffer of that size.
/// Round-trips with sidereon_opm_parse_kvn.
///
/// Safety: opm must be a live handle; out must point to at least len writable
/// bytes or be NULL when len is 0; out_written and out_required must point to
/// size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_opm_to_kvn(
    opm: *const SidereonOpm,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_opm_to_kvn", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_opm_to_kvn",
            out_written,
            out_required
        ));
        let opm = c_try!(require_ref(opm, "sidereon_opm_to_kvn", "opm"));
        let text = core_opm::encode_kvn(&opm.inner);
        c_try!(copy_prefix_to_c(
            "sidereon_opm_to_kvn",
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

/// Serialize an OPM to XML text. The output is not null-terminated. Uses the
/// variable-length output contract documented at the top of the header: call once
/// with out=NULL to learn *out_required, then again with a buffer of that size.
/// Round-trips with sidereon_opm_parse_xml.
///
/// Safety: opm must be a live handle; out must point to at least len writable
/// bytes or be NULL when len is 0; out_written and out_required must point to
/// size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_opm_to_xml(
    opm: *const SidereonOpm,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_opm_to_xml", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_opm_to_xml",
            out_written,
            out_required
        ));
        let opm = c_try!(require_ref(opm, "sidereon_opm_to_xml", "opm"));
        let text = core_opm::encode_xml(&opm.inner);
        c_try!(copy_prefix_to_c(
            "sidereon_opm_to_xml",
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

/// Release an OPM handle from a sidereon_opm_parse_* call. Passing NULL is a
/// no-op.
///
/// Safety: opm must be NULL or a live handle that has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_opm_free(opm: *mut SidereonOpm) {
    ffi_boundary("sidereon_opm_free", (), || {
        free_boxed(opm);
    });
}

// ===========================================================================
// SP3-backed geometry: visible / visibility_series / passes.
//
// These delegate to sidereon_core::geometry::{visible, visibility_series,
// passes} with the loaded SP3 product as the ObservableEphemerisSource and the
// product's own satellite list. No geometry, weighting, or elevation algebra
// lives here; the binding only marshals options and copies results.
