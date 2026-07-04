use super::*;

/// A parsed ANTEX antenna-calibration product. Create with sidereon_antex_parse
/// and release with sidereon_antex_free.
pub struct SidereonAntex {
    pub(crate) inner: Antex,
}

/// A single ANTEX antenna calibration block (receiver or satellite), owned
/// independently of the parent product. Obtain one with sidereon_antex_antenna
/// and release it with sidereon_antenna_free.
pub struct SidereonAntenna {
    pub(crate) inner: Antenna,
}

/// Parse an ANTEX 1.4 antenna-calibration byte buffer. On success writes a newly
/// owned handle to *out_antex. Release it with sidereon_antex_free. PCO/PCV
/// values are exposed in meters, exactly as the engine produces them.
///
/// Safety: data must point to len readable bytes; out_antex must point to
/// storage for a SidereonAntex*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antex_parse(
    data: *const u8,
    len: usize,
    out_antex: *mut *mut SidereonAntex,
) -> SidereonStatus {
    ffi_boundary("sidereon_antex_parse", SidereonStatus::Panic, || {
        let out_antex = c_try!(require_out(out_antex, "sidereon_antex_parse", "out_antex"));
        *out_antex = ptr::null_mut();
        let bytes = c_try!(require_slice(data, len, "sidereon_antex_parse", "data"));
        let text = match str::from_utf8(bytes) {
            Ok(text) => text,
            Err(_) => {
                set_last_error("sidereon_antex_parse: data is not valid UTF-8".to_string());
                return SidereonStatus::InvalidToken;
            }
        };
        let inner = match Antex::parse(text) {
            Ok(antex) => antex,
            Err(err) => return map_antex_error("sidereon_antex_parse", err),
        };
        write_boxed_handle(out_antex, SidereonAntex { inner });
        SidereonStatus::Ok
    })
}

/// Write the number of antenna blocks in the product to *out_count.
///
/// Safety: antex must be a live handle from sidereon_antex_parse; out_count must
/// point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antex_antenna_count(
    antex: *const SidereonAntex,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_antex_antenna_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_antex_antenna_count",
                "out_count"
            ));
            *out_count = 0;
            let antex = c_try!(require_ref(antex, "sidereon_antex_antenna_count", "antex"));
            *out_count = antex.inner.antennas.len();
            SidereonStatus::Ok
        },
    )
}

/// Look up an antenna by its exact `TYPE / SERIAL` id. On success writes a newly
/// owned antenna handle to *out_antenna, or NULL if no block has that id (a
/// successful query that found nothing, not an error). Release a non-NULL handle
/// with sidereon_antenna_free.
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
    ffi_boundary("sidereon_antex_antenna", SidereonStatus::Panic, || {
        let out_antenna = c_try!(require_out(
            out_antenna,
            "sidereon_antex_antenna",
            "out_antenna"
        ));
        *out_antenna = ptr::null_mut();
        let antex = c_try!(require_ref(antex, "sidereon_antex_antenna", "antex"));
        let id = c_try!(parse_bounded_c_string(
            "sidereon_antex_antenna",
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

/// Serialize a parsed ANTEX product back to ANTEX text. The output is not
/// null-terminated. Delegates to sidereon_core::antex::Antex::encode. Uses the
/// variable-length output contract documented at the top of the header.
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
    ffi_boundary("sidereon_antex_encode", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_antex_encode",
            out_written,
            out_required
        ));
        let antex = c_try!(require_ref(antex, "sidereon_antex_encode", "antex"));
        let text = antex.inner.encode();
        c_try!(copy_prefix_to_c(
            "sidereon_antex_encode",
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

/// Release an ANTEX product handle from sidereon_antex_parse. Passing NULL is a
/// no-op.
///
/// Safety: antex must be NULL or a live handle from sidereon_antex_parse that
/// has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antex_free(antex: *mut SidereonAntex) {
    ffi_boundary("sidereon_antex_free", (), || {
        free_boxed(antex);
    });
}
