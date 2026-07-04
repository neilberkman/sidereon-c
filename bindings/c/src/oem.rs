use super::*;

/// A parsed CCSDS OEM (Orbit Ephemeris Message). Opaque to C. Create with
/// sidereon_oem_parse_kvn or sidereon_oem_parse_xml; serialize with
/// sidereon_oem_to_kvn or sidereon_oem_to_xml; release with sidereon_oem_free.
pub struct SidereonOem {
    pub(crate) inner: Oem,
}

/// Parse a CCSDS OEM in KVN (keyword=value) form. On success writes a newly
/// owned handle to *out_oem. Release it with sidereon_oem_free.
///
/// Safety: data must point to len readable bytes; out_oem must point to storage
/// for a SidereonOem*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_oem_parse_kvn(
    data: *const u8,
    len: usize,
    out_oem: *mut *mut SidereonOem,
) -> SidereonStatus {
    ffi_boundary("sidereon_oem_parse_kvn", SidereonStatus::Panic, || {
        let out_oem = c_try!(require_out(out_oem, "sidereon_oem_parse_kvn", "out_oem"));
        *out_oem = ptr::null_mut();
        let text = c_try!(ndm_text_from_utf8(data, len, "sidereon_oem_parse_kvn"));
        let inner = match core_oem::parse_kvn(text) {
            Ok(oem) => oem,
            Err(err) => {
                set_last_error(format!("sidereon_oem_parse_kvn: {err}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        write_boxed_handle(out_oem, SidereonOem { inner });
        SidereonStatus::Ok
    })
}

/// Parse a CCSDS OEM in XML (NDM/XML) form. On success writes a newly owned
/// handle to *out_oem. Release it with sidereon_oem_free.
///
/// Safety: data must point to len readable bytes; out_oem must point to storage
/// for a SidereonOem*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_oem_parse_xml(
    data: *const u8,
    len: usize,
    out_oem: *mut *mut SidereonOem,
) -> SidereonStatus {
    ffi_boundary("sidereon_oem_parse_xml", SidereonStatus::Panic, || {
        let out_oem = c_try!(require_out(out_oem, "sidereon_oem_parse_xml", "out_oem"));
        *out_oem = ptr::null_mut();
        let text = c_try!(ndm_text_from_utf8(data, len, "sidereon_oem_parse_xml"));
        let inner = match core_oem::parse_xml(text) {
            Ok(oem) => oem,
            Err(err) => {
                set_last_error(format!("sidereon_oem_parse_xml: {err}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        write_boxed_handle(out_oem, SidereonOem { inner });
        SidereonStatus::Ok
    })
}

/// Write the number of metadata/data segments in the OEM to *out_count.
///
/// Safety: oem must be a live handle from a sidereon_oem_parse_* call; out_count
/// must point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_oem_segment_count(
    oem: *const SidereonOem,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_oem_segment_count", SidereonStatus::Panic, || {
        let out_count = c_try!(require_out(
            out_count,
            "sidereon_oem_segment_count",
            "out_count"
        ));
        *out_count = 0;
        let oem = c_try!(require_ref(oem, "sidereon_oem_segment_count", "oem"));
        *out_count = oem.inner.segments.len();
        SidereonStatus::Ok
    })
}

/// Serialize an OEM to KVN text. The output is not null-terminated. Uses the
/// variable-length output contract documented at the top of the header: call once
/// with out=NULL to learn *out_required, then again with a buffer of that size.
/// Round-trips with sidereon_oem_parse_kvn.
///
/// Safety: oem must be a live handle; out must point to at least len writable
/// bytes or be NULL when len is 0; out_written and out_required must point to
/// size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_oem_to_kvn(
    oem: *const SidereonOem,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_oem_to_kvn", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_oem_to_kvn",
            out_written,
            out_required
        ));
        let oem = c_try!(require_ref(oem, "sidereon_oem_to_kvn", "oem"));
        let text = core_oem::encode_kvn(&oem.inner);
        c_try!(copy_prefix_to_c(
            "sidereon_oem_to_kvn",
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

/// Serialize an OEM to XML text. The output is not null-terminated. Uses the
/// variable-length output contract documented at the top of the header: call once
/// with out=NULL to learn *out_required, then again with a buffer of that size.
/// Round-trips with sidereon_oem_parse_xml.
///
/// Safety: oem must be a live handle; out must point to at least len writable
/// bytes or be NULL when len is 0; out_written and out_required must point to
/// size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_oem_to_xml(
    oem: *const SidereonOem,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_oem_to_xml", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_oem_to_xml",
            out_written,
            out_required
        ));
        let oem = c_try!(require_ref(oem, "sidereon_oem_to_xml", "oem"));
        let text = core_oem::encode_xml(&oem.inner);
        c_try!(copy_prefix_to_c(
            "sidereon_oem_to_xml",
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

/// Release an OEM handle from a sidereon_oem_parse_* call. Passing NULL is a
/// no-op.
///
/// Safety: oem must be NULL or a live handle that has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_oem_free(oem: *mut SidereonOem) {
    ffi_boundary("sidereon_oem_free", (), || {
        free_boxed(oem);
    });
}
