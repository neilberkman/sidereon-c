use super::*;

// --- SBAS decode, correction store, and corrected broadcast source ----------

pub struct SidereonSbasBlock {
    pub(crate) inner: SbasBlock,
}

pub struct SidereonSbasCorrectionStore {
    pub(crate) inner: SbasCorrectionStore,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSbasWireForm {
    Framed250 = 0,
    Body226 = 1,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSbasMessageKind {
    DoNotUse = 0,
    PrnMask = 1,
    FastCorrections = 2,
    Integrity = 3,
    FastDegradation = 4,
    GeoNav = 5,
    NetworkTime = 6,
    GeoAlmanac = 7,
    IgpMask = 8,
    MixedCorrections = 9,
    LongTermCorrections = 10,
    IonoDelays = 11,
    Unsupported = 12,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSbasSolveMode {
    MixedAugmentation = 0,
    SbasOnly = 1,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSbasMessageInfo {
    pub form: SidereonSbasWireForm,
    pub kind: SidereonSbasMessageKind,
    pub message_type: u8,
    pub preamble: u8,
    pub fast_count: usize,
    pub long_term_count: usize,
    pub iono_delay_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSbasFastCorrection {
    pub prc_m: f64,
    pub rrc_m_s: f64,
    pub udrei: u8,
    pub t_of_j2000_s: f64,
    pub iodf: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSbasLongTermCorrection {
    pub iode: u8,
    pub delta_ecef_m: [f64; 3],
    pub delta_ecef_rate_m_s: [f64; 3],
    pub delta_af0_s: f64,
    pub delta_af1_s_s: f64,
    pub t0_j2000_s: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSbasGeoState {
    pub position_ecef_m: [f64; 3],
    pub velocity_ecef_m_s: [f64; 3],
    pub acceleration_ecef_m_s2: [f64; 3],
    pub clock_offset_s: f64,
    pub clock_drift_s_s: f64,
    pub t0_j2000_s: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSbasIgp {
    pub lat_deg: f64,
    pub lon_deg: f64,
    pub vertical_delay_m: f64,
    pub has_give_variance_m2: bool,
    pub give_variance_m2: f64,
}

/// Decoded SBAS PRN mask message payload.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSbasPrnMask {
    /// SBAS preamble byte.
    pub preamble: u8,
    /// IODP value.
    pub iodp: u8,
    /// PRN mask bits in SBAS broadcast order.
    pub mask: [bool; 210],
}

/// Decoded SBAS fast-correction message payload.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSbasRawFastCorrections {
    /// SBAS preamble byte.
    pub preamble: u8,
    /// Message type, 2 through 5.
    pub message_type: u8,
    /// IODF value.
    pub iodf: u8,
    /// IODP value.
    pub iodp: u8,
    /// Raw pseudorange correction fields.
    pub prc: [i16; 13],
    /// UDREI values.
    pub udrei: [u8; 13],
}

/// Decoded SBAS integrity message payload.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSbasIntegrity {
    /// SBAS preamble byte.
    pub preamble: u8,
    /// IODF values.
    pub iodf: [u8; 4],
    /// UDREI values.
    pub udrei: [u8; 51],
}

/// Decoded SBAS fast-degradation message payload.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSbasFastDegradation {
    /// SBAS preamble byte.
    pub preamble: u8,
    /// System latency, seconds.
    pub system_latency_s: u8,
    /// IODP value.
    pub iodp: u8,
    /// Degradation indicator values.
    pub ai: [u8; 51],
}

/// Decoded SBAS GEO navigation message payload. Position fields are meters,
/// velocity fields are meters per second, acceleration fields are meters per
/// second squared, and clock fields are seconds or seconds per second.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSbasGeoNavMessage {
    /// SBAS preamble byte.
    pub preamble: u8,
    /// Time of day, seconds.
    pub time_of_day_s: u16,
    /// URA indicator.
    pub ura: u8,
    /// Raw X position, scaled by the core store when ingested.
    pub x_m: i32,
    /// Raw Y position, scaled by the core store when ingested.
    pub y_m: i32,
    /// Raw Z position, scaled by the core store when ingested.
    pub z_m: i32,
    /// Raw X velocity field.
    pub x_rate_m_s: i32,
    /// Raw Y velocity field.
    pub y_rate_m_s: i32,
    /// Raw Z velocity field.
    pub z_rate_m_s: i32,
    /// Raw X acceleration field.
    pub x_accel_m_s2: i16,
    /// Raw Y acceleration field.
    pub y_accel_m_s2: i16,
    /// Raw Z acceleration field.
    pub z_accel_m_s2: i16,
    /// Raw clock offset field.
    pub a_gf0_s: i16,
    /// Raw clock drift field.
    pub a_gf1_s_s: i16,
}

/// Decoded SBAS IGP mask message payload.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSbasIgpMask {
    /// SBAS preamble byte.
    pub preamble: u8,
    /// IGP band number.
    pub band_number: u8,
    /// IODI value.
    pub iodi: u8,
    /// IGP mask bits in SBAS broadcast order.
    pub mask: [bool; 201],
}

/// Decoded fast-correction part of an SBAS mixed-correction message.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSbasMixedFastCorrections {
    /// IODF value.
    pub iodf: u8,
    /// IODP value.
    pub iodp: u8,
    /// SBAS block id.
    pub block_id: u8,
    /// Raw pseudorange correction fields.
    pub prc: [i16; 6],
    /// UDREI values.
    pub udrei: [u8; 6],
}

/// Metadata for one SBAS long-term-correction half.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSbasLongTermHalfInfo {
    /// Whether the half carries velocity-code records.
    pub velocity_code: bool,
    /// IODP value.
    pub iodp: u8,
    /// Number of long-term records in the half.
    pub record_count: usize,
}

/// Decoded SBAS long-term-correction record.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSbasLongTermRecord {
    /// Monitored satellite index within the active PRN mask.
    pub monitored_index: u8,
    /// IODE value.
    pub iode: u8,
    /// Raw X correction field.
    pub delta_x: i32,
    /// Raw Y correction field.
    pub delta_y: i32,
    /// Raw Z correction field.
    pub delta_z: i32,
    /// Raw X-rate correction field.
    pub delta_x_rate: i32,
    /// Raw Y-rate correction field.
    pub delta_y_rate: i32,
    /// Raw Z-rate correction field.
    pub delta_z_rate: i32,
    /// Raw clock-offset correction field.
    pub delta_a_f0: i32,
    /// Raw clock-drift correction field.
    pub delta_a_f1: i32,
    /// Whether time_of_day_s carries a value.
    pub has_time_of_day_s: bool,
    /// Time of day, seconds, when present.
    pub time_of_day_s: u32,
}

/// Decoded SBAS ionospheric grid-point delay.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSbasIgpDelay {
    /// Raw vertical-delay field.
    pub vertical_delay: u16,
    /// GIVEI value.
    pub givei: u8,
}

/// Decoded SBAS ionospheric-delay message payload.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSbasIonoDelays {
    /// SBAS preamble byte.
    pub preamble: u8,
    /// IGP band number.
    pub band_number: u8,
    /// SBAS block id.
    pub block_id: u8,
    /// IODI value.
    pub iodi: u8,
    /// Fifteen decoded grid-point delays.
    pub entries: [SidereonSbasIgpDelay; 15],
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_block_decode(
    bytes: *const u8,
    len: usize,
    form: u32,
    out_block: *mut *mut SidereonSbasBlock,
) -> SidereonStatus {
    ffi_boundary("sidereon_sbas_block_decode", SidereonStatus::Panic, || {
        let out = c_try!(require_out(
            out_block,
            "sidereon_sbas_block_decode",
            "out_block"
        ));
        *out = ptr::null_mut();
        let bytes = c_try!(require_slice(
            bytes,
            len,
            "sidereon_sbas_block_decode",
            "bytes"
        ));
        let form = c_try!(sbas_wire_form_from_c("sidereon_sbas_block_decode", form));
        match SbasBlock::decode(bytes, form) {
            Ok(inner) => {
                write_boxed_handle(out, SidereonSbasBlock { inner });
                SidereonStatus::Ok
            }
            Err(err) => map_sbas_error("sidereon_sbas_block_decode", err),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_block_info(
    block: *const SidereonSbasBlock,
    out_info: *mut SidereonSbasMessageInfo,
) -> SidereonStatus {
    ffi_boundary("sidereon_sbas_block_info", SidereonStatus::Panic, || {
        let out = c_try!(require_out(
            out_info,
            "sidereon_sbas_block_info",
            "out_info"
        ));
        let block = c_try!(require_ref(block, "sidereon_sbas_block_info", "block"));
        *out = sbas_message_info(&block.inner);
        SidereonStatus::Ok
    })
}

/// Read a decoded SBAS PRN mask payload. If the block is not a PRN mask,
/// out_present is false and out_mask is zeroed.
///
/// Safety: block must be a live handle; out_present and out_mask must point to
/// writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_block_prn_mask(
    block: *const SidereonSbasBlock,
    out_present: *mut bool,
    out_mask: *mut SidereonSbasPrnMask,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sbas_block_prn_mask",
        SidereonStatus::Panic,
        || {
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_sbas_block_prn_mask",
                "out_present"
            ));
            *out_present = false;
            let out = c_try!(require_out(
                out_mask,
                "sidereon_sbas_block_prn_mask",
                "out_mask"
            ));
            *out = SidereonSbasPrnMask {
                preamble: 0,
                iodp: 0,
                mask: [false; 210],
            };
            let block = c_try!(require_ref(block, "sidereon_sbas_block_prn_mask", "block"));
            if let SbasMessage::PrnMask(value) = &block.inner.message {
                *out_present = true;
                *out = sbas_prn_mask_to_c(value);
            }
            SidereonStatus::Ok
        },
    )
}

/// Read a decoded SBAS fast-correction payload. If the block is not a fast
/// correction, out_present is false and out_fast is zeroed.
///
/// Safety: block must be a live handle; out_present and out_fast must point to
/// writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_block_fast_corrections(
    block: *const SidereonSbasBlock,
    out_present: *mut bool,
    out_fast: *mut SidereonSbasRawFastCorrections,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sbas_block_fast_corrections",
        SidereonStatus::Panic,
        || {
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_sbas_block_fast_corrections",
                "out_present"
            ));
            *out_present = false;
            let out = c_try!(require_out(
                out_fast,
                "sidereon_sbas_block_fast_corrections",
                "out_fast"
            ));
            *out = SidereonSbasRawFastCorrections {
                preamble: 0,
                message_type: 0,
                iodf: 0,
                iodp: 0,
                prc: [0; 13],
                udrei: [0; 13],
            };
            let block = c_try!(require_ref(
                block,
                "sidereon_sbas_block_fast_corrections",
                "block"
            ));
            if let SbasMessage::FastCorrections(value) = &block.inner.message {
                *out_present = true;
                *out = sbas_raw_fast_to_c(value);
            }
            SidereonStatus::Ok
        },
    )
}

/// Read a decoded SBAS integrity payload. If the block is not an integrity
/// message, out_present is false and out_integrity is zeroed.
///
/// Safety: block must be a live handle; out_present and out_integrity must point
/// to writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_block_integrity(
    block: *const SidereonSbasBlock,
    out_present: *mut bool,
    out_integrity: *mut SidereonSbasIntegrity,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sbas_block_integrity",
        SidereonStatus::Panic,
        || {
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_sbas_block_integrity",
                "out_present"
            ));
            *out_present = false;
            let out = c_try!(require_out(
                out_integrity,
                "sidereon_sbas_block_integrity",
                "out_integrity"
            ));
            *out = SidereonSbasIntegrity {
                preamble: 0,
                iodf: [0; 4],
                udrei: [0; 51],
            };
            let block = c_try!(require_ref(block, "sidereon_sbas_block_integrity", "block"));
            if let SbasMessage::Integrity(value) = &block.inner.message {
                *out_present = true;
                *out = sbas_integrity_to_c(value);
            }
            SidereonStatus::Ok
        },
    )
}

/// Read a decoded SBAS fast-degradation payload. If the block is not a
/// fast-degradation message, out_present is false and out_degradation is zeroed.
///
/// Safety: block must be a live handle; out_present and out_degradation must
/// point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_block_fast_degradation(
    block: *const SidereonSbasBlock,
    out_present: *mut bool,
    out_degradation: *mut SidereonSbasFastDegradation,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sbas_block_fast_degradation",
        SidereonStatus::Panic,
        || {
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_sbas_block_fast_degradation",
                "out_present"
            ));
            *out_present = false;
            let out = c_try!(require_out(
                out_degradation,
                "sidereon_sbas_block_fast_degradation",
                "out_degradation"
            ));
            *out = SidereonSbasFastDegradation {
                preamble: 0,
                system_latency_s: 0,
                iodp: 0,
                ai: [0; 51],
            };
            let block = c_try!(require_ref(
                block,
                "sidereon_sbas_block_fast_degradation",
                "block"
            ));
            if let SbasMessage::FastDegradation(value) = &block.inner.message {
                *out_present = true;
                *out = sbas_fast_degradation_to_c(value);
            }
            SidereonStatus::Ok
        },
    )
}

/// Read a decoded SBAS GEO navigation payload. If the block is not a GEO
/// navigation message, out_present is false and out_geo_nav is zeroed.
///
/// Safety: block must be a live handle; out_present and out_geo_nav must point
/// to writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_block_geo_nav(
    block: *const SidereonSbasBlock,
    out_present: *mut bool,
    out_geo_nav: *mut SidereonSbasGeoNavMessage,
) -> SidereonStatus {
    ffi_boundary("sidereon_sbas_block_geo_nav", SidereonStatus::Panic, || {
        let out_present = c_try!(require_out(
            out_present,
            "sidereon_sbas_block_geo_nav",
            "out_present"
        ));
        *out_present = false;
        let out = c_try!(require_out(
            out_geo_nav,
            "sidereon_sbas_block_geo_nav",
            "out_geo_nav"
        ));
        *out = SidereonSbasGeoNavMessage {
            preamble: 0,
            time_of_day_s: 0,
            ura: 0,
            x_m: 0,
            y_m: 0,
            z_m: 0,
            x_rate_m_s: 0,
            y_rate_m_s: 0,
            z_rate_m_s: 0,
            x_accel_m_s2: 0,
            y_accel_m_s2: 0,
            z_accel_m_s2: 0,
            a_gf0_s: 0,
            a_gf1_s_s: 0,
        };
        let block = c_try!(require_ref(block, "sidereon_sbas_block_geo_nav", "block"));
        if let SbasMessage::GeoNav(value) = &block.inner.message {
            *out_present = true;
            *out = sbas_geo_nav_message_to_c(value);
        }
        SidereonStatus::Ok
    })
}

/// Read a decoded SBAS IGP mask payload. If the block is not an IGP mask,
/// out_present is false and out_mask is zeroed.
///
/// Safety: block must be a live handle; out_present and out_mask must point to
/// writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_block_igp_mask(
    block: *const SidereonSbasBlock,
    out_present: *mut bool,
    out_mask: *mut SidereonSbasIgpMask,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sbas_block_igp_mask",
        SidereonStatus::Panic,
        || {
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_sbas_block_igp_mask",
                "out_present"
            ));
            *out_present = false;
            let out = c_try!(require_out(
                out_mask,
                "sidereon_sbas_block_igp_mask",
                "out_mask"
            ));
            *out = SidereonSbasIgpMask {
                preamble: 0,
                band_number: 0,
                iodi: 0,
                mask: [false; 201],
            };
            let block = c_try!(require_ref(block, "sidereon_sbas_block_igp_mask", "block"));
            if let SbasMessage::IgpMask(value) = &block.inner.message {
                *out_present = true;
                *out = sbas_igp_mask_to_c(value);
            }
            SidereonStatus::Ok
        },
    )
}

/// Read the decoded fast-correction part of an SBAS mixed-correction payload.
/// If the block is not mixed corrections, out_present is false and out_fast is
/// zeroed.
///
/// Safety: block must be a live handle; out_present and out_fast must point to
/// writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_block_mixed_fast_corrections(
    block: *const SidereonSbasBlock,
    out_present: *mut bool,
    out_fast: *mut SidereonSbasMixedFastCorrections,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sbas_block_mixed_fast_corrections",
        SidereonStatus::Panic,
        || {
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_sbas_block_mixed_fast_corrections",
                "out_present"
            ));
            *out_present = false;
            let out = c_try!(require_out(
                out_fast,
                "sidereon_sbas_block_mixed_fast_corrections",
                "out_fast"
            ));
            *out = SidereonSbasMixedFastCorrections {
                iodf: 0,
                iodp: 0,
                block_id: 0,
                prc: [0; 6],
                udrei: [0; 6],
            };
            let block = c_try!(require_ref(
                block,
                "sidereon_sbas_block_mixed_fast_corrections",
                "block"
            ));
            if let SbasMessage::MixedCorrections(value) = &block.inner.message {
                *out_present = true;
                *out = sbas_mixed_fast_to_c(&value.fast);
            }
            SidereonStatus::Ok
        },
    )
}

/// Read long-term half metadata from a long-term or mixed-correction block.
/// Long-term correction blocks have half_index 0 and 1. Mixed correction blocks
/// expose their long-term half at half_index 0.
///
/// Safety: block must be a live handle; out_present and out_info must point to
/// writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_block_long_term_half_info(
    block: *const SidereonSbasBlock,
    half_index: usize,
    out_present: *mut bool,
    out_info: *mut SidereonSbasLongTermHalfInfo,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sbas_block_long_term_half_info",
        SidereonStatus::Panic,
        || {
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_sbas_block_long_term_half_info",
                "out_present"
            ));
            *out_present = false;
            let out = c_try!(require_out(
                out_info,
                "sidereon_sbas_block_long_term_half_info",
                "out_info"
            ));
            *out = SidereonSbasLongTermHalfInfo {
                velocity_code: false,
                iodp: 0,
                record_count: 0,
            };
            let block = c_try!(require_ref(
                block,
                "sidereon_sbas_block_long_term_half_info",
                "block"
            ));
            if let Some(half) = sbas_long_half_for_index(&block.inner.message, half_index) {
                *out_present = true;
                *out = sbas_long_half_to_c(half);
            }
            SidereonStatus::Ok
        },
    )
}

/// Copy decoded long-term records from one long-term half. Uses the
/// variable-length output contract.
///
/// Safety: block must be a live handle; out points to len
/// SidereonSbasLongTermRecord entries or NULL when len is 0; out_written and
/// out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_block_long_term_records(
    block: *const SidereonSbasBlock,
    half_index: usize,
    out: *mut SidereonSbasLongTermRecord,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sbas_block_long_term_records",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_sbas_block_long_term_records",
                out_written,
                out_required
            ));
            let block = c_try!(require_ref(
                block,
                "sidereon_sbas_block_long_term_records",
                "block"
            ));
            let records: Vec<SidereonSbasLongTermRecord> =
                sbas_long_half_for_index(&block.inner.message, half_index)
                    .map(|half| half.records.iter().map(sbas_long_record_to_c).collect())
                    .unwrap_or_default();
            c_try!(copy_prefix_to_c(
                "sidereon_sbas_block_long_term_records",
                "out",
                &records,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Read a decoded SBAS ionospheric-delay payload. If the block is not an
/// ionospheric-delay message, out_present is false and out_delays is zeroed.
///
/// Safety: block must be a live handle; out_present and out_delays must point to
/// writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_block_iono_delays(
    block: *const SidereonSbasBlock,
    out_present: *mut bool,
    out_delays: *mut SidereonSbasIonoDelays,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sbas_block_iono_delays",
        SidereonStatus::Panic,
        || {
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_sbas_block_iono_delays",
                "out_present"
            ));
            *out_present = false;
            let out = c_try!(require_out(
                out_delays,
                "sidereon_sbas_block_iono_delays",
                "out_delays"
            ));
            *out = SidereonSbasIonoDelays {
                preamble: 0,
                band_number: 0,
                block_id: 0,
                iodi: 0,
                entries: [SidereonSbasIgpDelay {
                    vertical_delay: 0,
                    givei: 0,
                }; 15],
            };
            let block = c_try!(require_ref(
                block,
                "sidereon_sbas_block_iono_delays",
                "block"
            ));
            if let SbasMessage::IonoDelays(value) = &block.inner.message {
                *out_present = true;
                *out = sbas_iono_delays_to_c(value);
            }
            SidereonStatus::Ok
        },
    )
}

/// Copy raw 212-bit message data bytes for DoNotUse, NetworkTime, GeoAlmanac,
/// and unsupported SBAS message blocks. Other decoded message kinds return zero
/// required bytes.
///
/// Safety: block must be a live handle; out points to len writable bytes or NULL
/// when len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_block_raw_data(
    block: *const SidereonSbasBlock,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sbas_block_raw_data",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_sbas_block_raw_data",
                out_written,
                out_required
            ));
            let block = c_try!(require_ref(block, "sidereon_sbas_block_raw_data", "block"));
            let data = sbas_raw_data(&block.inner.message).unwrap_or(&[]);
            c_try!(copy_prefix_to_c(
                "sidereon_sbas_block_raw_data",
                "out",
                data,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_block_encode(
    block: *const SidereonSbasBlock,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_sbas_block_encode", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_sbas_block_encode",
            out_written,
            out_required
        ));
        let block = c_try!(require_ref(block, "sidereon_sbas_block_encode", "block"));
        let bytes = block.inner.encode();
        c_try!(copy_prefix_to_c(
            "sidereon_sbas_block_encode",
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

#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_block_free(block: *mut SidereonSbasBlock) {
    free_boxed(block);
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_store_new(
    out_store: *mut *mut SidereonSbasCorrectionStore,
) -> SidereonStatus {
    ffi_boundary("sidereon_sbas_store_new", SidereonStatus::Panic, || {
        let out = c_try!(require_out(
            out_store,
            "sidereon_sbas_store_new",
            "out_store"
        ));
        *out = ptr::null_mut();
        write_boxed_handle(
            out,
            SidereonSbasCorrectionStore {
                inner: SbasCorrectionStore::new(),
            },
        );
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_store_ingest(
    store: *mut SidereonSbasCorrectionStore,
    block: *const SidereonSbasBlock,
    geo_sat_id: *const c_char,
    epoch: *const SidereonGnssWeekTow,
) -> SidereonStatus {
    ffi_boundary("sidereon_sbas_store_ingest", SidereonStatus::Panic, || {
        let store = c_try!(require_out(store, "sidereon_sbas_store_ingest", "store"));
        let block = c_try!(require_ref(block, "sidereon_sbas_store_ingest", "block"));
        let geo = c_try!(parse_satellite_token(
            "sidereon_sbas_store_ingest",
            geo_sat_id
        ));
        let epoch = c_try!(require_ref(epoch, "sidereon_sbas_store_ingest", "epoch"));
        let epoch = c_try!(gnss_week_tow_from_c("sidereon_sbas_store_ingest", epoch));
        match store.inner.ingest(&block.inner.message, geo, epoch) {
            Ok(()) => SidereonStatus::Ok,
            Err(err) => map_sbas_error("sidereon_sbas_store_ingest", err),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_store_ready_geos(
    store: *const SidereonSbasCorrectionStore,
    t_j2000_s: f64,
    out: *mut SidereonSatelliteToken,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sbas_store_ready_geos",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_sbas_store_ready_geos",
                out_written,
                out_required
            ));
            let store = c_try!(require_ref(
                store,
                "sidereon_sbas_store_ready_geos",
                "store"
            ));
            let values: Vec<SidereonSatelliteToken> = store
                .inner
                .ready_geos(t_j2000_s)
                .into_iter()
                .map(satellite_token)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_sbas_store_ready_geos",
                "out",
                &values,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_store_preferred_geo(
    store: *const SidereonSbasCorrectionStore,
    t_j2000_s: f64,
    out_present: *mut bool,
    out_geo: *mut SidereonSatelliteToken,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sbas_store_preferred_geo",
        SidereonStatus::Panic,
        || {
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_sbas_store_preferred_geo",
                "out_present"
            ));
            *out_present = false;
            let out_geo = c_try!(require_out(
                out_geo,
                "sidereon_sbas_store_preferred_geo",
                "out_geo"
            ));
            *out_geo = satellite_token_from_text("");
            let store = c_try!(require_ref(
                store,
                "sidereon_sbas_store_preferred_geo",
                "store"
            ));
            if let Some(geo) = store.inner.ready_geos(t_j2000_s).first().copied() {
                *out_present = true;
                *out_geo = satellite_token(geo);
            }
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_store_fast_correction(
    store: *const SidereonSbasCorrectionStore,
    geo_sat_id: *const c_char,
    sat_id: *const c_char,
    out_present: *mut bool,
    out_correction: *mut SidereonSbasFastCorrection,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sbas_store_fast_correction",
        SidereonStatus::Panic,
        || {
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_sbas_store_fast_correction",
                "out_present"
            ));
            *out_present = false;
            let out = c_try!(require_out(
                out_correction,
                "sidereon_sbas_store_fast_correction",
                "out_correction"
            ));
            *out = SidereonSbasFastCorrection {
                prc_m: 0.0,
                rrc_m_s: 0.0,
                udrei: 0,
                t_of_j2000_s: 0.0,
                iodf: 0,
            };
            let store = c_try!(require_ref(
                store,
                "sidereon_sbas_store_fast_correction",
                "store"
            ));
            let geo = c_try!(parse_satellite_token(
                "sidereon_sbas_store_fast_correction",
                geo_sat_id
            ));
            let sat = c_try!(parse_satellite_token(
                "sidereon_sbas_store_fast_correction",
                sat_id
            ));
            if let Some(value) = store.inner.fast(geo, sat) {
                *out_present = true;
                *out = sbas_fast_to_c(value);
            }
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_store_long_term_correction(
    store: *const SidereonSbasCorrectionStore,
    geo_sat_id: *const c_char,
    sat_id: *const c_char,
    out_present: *mut bool,
    out_correction: *mut SidereonSbasLongTermCorrection,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sbas_store_long_term_correction",
        SidereonStatus::Panic,
        || {
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_sbas_store_long_term_correction",
                "out_present"
            ));
            *out_present = false;
            let out = c_try!(require_out(
                out_correction,
                "sidereon_sbas_store_long_term_correction",
                "out_correction"
            ));
            *out = SidereonSbasLongTermCorrection {
                iode: 0,
                delta_ecef_m: [0.0; 3],
                delta_ecef_rate_m_s: [0.0; 3],
                delta_af0_s: 0.0,
                delta_af1_s_s: 0.0,
                t0_j2000_s: 0.0,
            };
            let store = c_try!(require_ref(
                store,
                "sidereon_sbas_store_long_term_correction",
                "store"
            ));
            let geo = c_try!(parse_satellite_token(
                "sidereon_sbas_store_long_term_correction",
                geo_sat_id
            ));
            let sat = c_try!(parse_satellite_token(
                "sidereon_sbas_store_long_term_correction",
                sat_id
            ));
            if let Some(value) = store.inner.long_term(geo, sat) {
                *out_present = true;
                *out = sbas_long_to_c(value);
            }
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_store_geo_nav(
    store: *const SidereonSbasCorrectionStore,
    geo_sat_id: *const c_char,
    out_present: *mut bool,
    out_state: *mut SidereonSbasGeoState,
) -> SidereonStatus {
    ffi_boundary("sidereon_sbas_store_geo_nav", SidereonStatus::Panic, || {
        let out_present = c_try!(require_out(
            out_present,
            "sidereon_sbas_store_geo_nav",
            "out_present"
        ));
        *out_present = false;
        let out = c_try!(require_out(
            out_state,
            "sidereon_sbas_store_geo_nav",
            "out_state"
        ));
        *out = SidereonSbasGeoState {
            position_ecef_m: [0.0; 3],
            velocity_ecef_m_s: [0.0; 3],
            acceleration_ecef_m_s2: [0.0; 3],
            clock_offset_s: 0.0,
            clock_drift_s_s: 0.0,
            t0_j2000_s: 0.0,
        };
        let store = c_try!(require_ref(store, "sidereon_sbas_store_geo_nav", "store"));
        let geo = c_try!(parse_satellite_token(
            "sidereon_sbas_store_geo_nav",
            geo_sat_id
        ));
        if let Some(value) = store.inner.geo_nav(geo) {
            *out_present = true;
            *out = sbas_geo_state_to_c(value);
        }
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_store_iono_grid_igps(
    store: *const SidereonSbasCorrectionStore,
    geo_sat_id: *const c_char,
    out: *mut SidereonSbasIgp,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sbas_store_iono_grid_igps",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_sbas_store_iono_grid_igps",
                out_written,
                out_required
            ));
            let store = c_try!(require_ref(
                store,
                "sidereon_sbas_store_iono_grid_igps",
                "store"
            ));
            let geo = c_try!(parse_satellite_token(
                "sidereon_sbas_store_iono_grid_igps",
                geo_sat_id
            ));
            let values: Vec<SidereonSbasIgp> = store
                .inner
                .iono_grid(geo)
                .map(|grid| grid.igps().iter().map(sbas_igp_to_c).collect())
                .unwrap_or_else(Vec::new);
            c_try!(copy_prefix_to_c(
                "sidereon_sbas_store_iono_grid_igps",
                "out",
                &values,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_store_iono_slant_delay_m(
    store: *const SidereonSbasCorrectionStore,
    geo_sat_id: *const c_char,
    receiver: *const SidereonGeodetic,
    elevation_rad: f64,
    azimuth_rad: f64,
    frequency_hz: f64,
    out_present: *mut bool,
    out_delay_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sbas_store_iono_slant_delay_m",
        SidereonStatus::Panic,
        || {
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_sbas_store_iono_slant_delay_m",
                "out_present"
            ));
            *out_present = false;
            let out = c_try!(require_out(
                out_delay_m,
                "sidereon_sbas_store_iono_slant_delay_m",
                "out_delay_m"
            ));
            *out = 0.0;
            let store = c_try!(require_ref(
                store,
                "sidereon_sbas_store_iono_slant_delay_m",
                "store"
            ));
            let geo = c_try!(parse_satellite_token(
                "sidereon_sbas_store_iono_slant_delay_m",
                geo_sat_id
            ));
            let receiver = c_try!(require_ref(
                receiver,
                "sidereon_sbas_store_iono_slant_delay_m",
                "receiver"
            ));
            let receiver = c_try!(geodetic_to_wgs84(
                "sidereon_sbas_store_iono_slant_delay_m",
                "receiver",
                *receiver
            ));
            if let Some(delay) = store.inner.iono_grid(geo).and_then(|grid| {
                grid.slant_delay_m(receiver, elevation_rad, azimuth_rad, frequency_hz)
            }) {
                *out_present = true;
                *out = delay;
            }
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_corrected_state(
    broadcast: *const SidereonBroadcastEphemeris,
    store: *const SidereonSbasCorrectionStore,
    geo_sat_id: *const c_char,
    mode: u32,
    sat_id: *const c_char,
    t_j2000_s: f64,
    out_present: *mut bool,
    out_position_ecef_m: *mut f64,
    out_clock_s: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sbas_corrected_state",
        SidereonStatus::Panic,
        || {
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_sbas_corrected_state",
                "out_present"
            ));
            *out_present = false;
            c_try!(require_out(
                out_position_ecef_m,
                "sidereon_sbas_corrected_state",
                "out_position_ecef_m"
            ));
            zero_f64_prefix(out_position_ecef_m, 3, 3);
            let out_clock = c_try!(require_out(
                out_clock_s,
                "sidereon_sbas_corrected_state",
                "out_clock_s"
            ));
            *out_clock = 0.0;
            let broadcast = c_try!(require_ref(
                broadcast,
                "sidereon_sbas_corrected_state",
                "broadcast"
            ));
            let store = c_try!(require_ref(store, "sidereon_sbas_corrected_state", "store"));
            let geo = c_try!(parse_satellite_token(
                "sidereon_sbas_corrected_state",
                geo_sat_id
            ));
            let mode = c_try!(sbas_solve_mode_from_c(
                "sidereon_sbas_corrected_state",
                mode
            ));
            let sat = c_try!(parse_satellite_token(
                "sidereon_sbas_corrected_state",
                sat_id
            ));
            let corrected =
                SbasCorrectedEphemeris::new(&broadcast.inner, &store.inner, geo).with_mode(mode);
            if let Some((position, clock)) = corrected.position_clock_at_j2000_s(sat, t_j2000_s) {
                c_try!(copy_exact_f64s(
                    "sidereon_sbas_corrected_state",
                    "out_position_ecef_m",
                    out_position_ecef_m,
                    3,
                    &position
                ));
                *out_present = true;
                *out_clock = clock;
            }
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_solve_broadcast(
    broadcast: *const SidereonBroadcastEphemeris,
    store: *const SidereonSbasCorrectionStore,
    geo_sat_id: *const c_char,
    mode: u32,
    inputs: *const SidereonSppInputs,
    out_solution: *mut *mut SidereonSppSolution,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sbas_solve_broadcast",
        SidereonStatus::Panic,
        || {
            let out_solution = c_try!(require_out(
                out_solution,
                "sidereon_sbas_solve_broadcast",
                "out_solution"
            ));
            *out_solution = ptr::null_mut();
            let broadcast = c_try!(require_ref(
                broadcast,
                "sidereon_sbas_solve_broadcast",
                "broadcast"
            ));
            let store = c_try!(require_ref(store, "sidereon_sbas_solve_broadcast", "store"));
            let geo = c_try!(parse_satellite_token(
                "sidereon_sbas_solve_broadcast",
                geo_sat_id
            ));
            let mode = c_try!(sbas_solve_mode_from_c(
                "sidereon_sbas_solve_broadcast",
                mode
            ));
            let inputs = c_try!(require_ref(
                inputs,
                "sidereon_sbas_solve_broadcast",
                "inputs"
            ));
            let corrected =
                SbasCorrectedEphemeris::new(&broadcast.inner, &store.inner, geo).with_mode(mode);
            let mut solve_inputs = c_try!(build_spp_solve_inputs(
                "sidereon_sbas_solve_broadcast",
                inputs,
                None,
                None,
                BTreeMap::new(),
            ));
            solve_inputs.sbas_iono = corrected.iono_grid().cloned();
            let inner = c_try!(guard(SidereonStatus::Solve, || {
                sidereon::solve_spp(
                    &corrected,
                    &solve_inputs,
                    inputs.with_geodetic,
                    SolvePolicy::default(),
                )
            }));
            write_boxed_handle(out_solution, SidereonSppSolution { inner });
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_store_free(store: *mut SidereonSbasCorrectionStore) {
    free_boxed(store);
}

fn sbas_wire_form_from_c(fn_name: &str, form: u32) -> Result<SbasWireForm, SidereonStatus> {
    match form {
        value if value == SidereonSbasWireForm::Framed250 as u32 => Ok(SbasWireForm::Framed250),
        value if value == SidereonSbasWireForm::Body226 as u32 => Ok(SbasWireForm::Body226),
        _ => {
            set_last_error(format!("{fn_name}: invalid SBAS wire form"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn sbas_solve_mode_from_c(fn_name: &str, mode: u32) -> Result<SbasSolveMode, SidereonStatus> {
    match mode {
        value if value == SidereonSbasSolveMode::MixedAugmentation as u32 => {
            Ok(SbasSolveMode::MixedAugmentation)
        }
        value if value == SidereonSbasSolveMode::SbasOnly as u32 => Ok(SbasSolveMode::SbasOnly),
        _ => {
            set_last_error(format!("{fn_name}: invalid SBAS solve mode"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn sbas_message_info(block: &SbasBlock) -> SidereonSbasMessageInfo {
    let message = &block.message;
    SidereonSbasMessageInfo {
        form: match block.form {
            SbasWireForm::Framed250 => SidereonSbasWireForm::Framed250,
            SbasWireForm::Body226 => SidereonSbasWireForm::Body226,
        },
        kind: sbas_message_kind_to_c(message),
        message_type: message.message_type(),
        preamble: sbas_message_preamble(message),
        fast_count: match message {
            SbasMessage::FastCorrections(_) => 13,
            SbasMessage::MixedCorrections(_) => 6,
            _ => 0,
        },
        long_term_count: match message {
            SbasMessage::LongTermCorrections(m) => m.halves.iter().map(|h| h.records.len()).sum(),
            SbasMessage::MixedCorrections(m) => m.long_term.records.len(),
            _ => 0,
        },
        iono_delay_count: match message {
            SbasMessage::IonoDelays(_) => 15,
            _ => 0,
        },
    }
}

fn sbas_fast_to_c(value: &SbasFastCorrection) -> SidereonSbasFastCorrection {
    SidereonSbasFastCorrection {
        prc_m: value.prc_m,
        rrc_m_s: value.rrc_m_s,
        udrei: value.udrei,
        t_of_j2000_s: value.t_of_j2000_s,
        iodf: value.iodf,
    }
}

fn sbas_long_to_c(value: &SbasLongTermCorrection) -> SidereonSbasLongTermCorrection {
    SidereonSbasLongTermCorrection {
        iode: value.iode,
        delta_ecef_m: value.delta_ecef_m,
        delta_ecef_rate_m_s: value.delta_ecef_rate_m_s,
        delta_af0_s: value.delta_af0_s,
        delta_af1_s_s: value.delta_af1_s_s,
        t0_j2000_s: value.t0_j2000_s,
    }
}

fn sbas_geo_state_to_c(value: &SbasGeoState) -> SidereonSbasGeoState {
    SidereonSbasGeoState {
        position_ecef_m: value.position_ecef_m,
        velocity_ecef_m_s: value.velocity_ecef_m_s,
        acceleration_ecef_m_s2: value.acceleration_ecef_m_s2,
        clock_offset_s: value.clock_offset_s,
        clock_drift_s_s: value.clock_drift_s_s,
        t0_j2000_s: value.t0_j2000_s,
    }
}

fn sbas_igp_to_c(value: &SbasIgp) -> SidereonSbasIgp {
    SidereonSbasIgp {
        lat_deg: value.lat_deg,
        lon_deg: value.lon_deg,
        vertical_delay_m: value.vertical_delay_m,
        has_give_variance_m2: value.give_variance_m2.is_some(),
        give_variance_m2: value.give_variance_m2.unwrap_or(0.0),
    }
}

fn sbas_prn_mask_to_c(value: &SbasPrnMask) -> SidereonSbasPrnMask {
    SidereonSbasPrnMask {
        preamble: value.preamble,
        iodp: value.iodp,
        mask: value.mask,
    }
}

fn sbas_raw_fast_to_c(value: &SbasFastCorrections) -> SidereonSbasRawFastCorrections {
    SidereonSbasRawFastCorrections {
        preamble: value.preamble,
        message_type: value.message_type,
        iodf: value.iodf,
        iodp: value.iodp,
        prc: value.prc,
        udrei: value.udrei,
    }
}

fn sbas_integrity_to_c(value: &SbasIntegrity) -> SidereonSbasIntegrity {
    SidereonSbasIntegrity {
        preamble: value.preamble,
        iodf: value.iodf,
        udrei: value.udrei,
    }
}

fn sbas_fast_degradation_to_c(value: &SbasFastDegradation) -> SidereonSbasFastDegradation {
    SidereonSbasFastDegradation {
        preamble: value.preamble,
        system_latency_s: value.system_latency_s,
        iodp: value.iodp,
        ai: value.ai,
    }
}

fn sbas_geo_nav_message_to_c(value: &SbasGeoNav) -> SidereonSbasGeoNavMessage {
    SidereonSbasGeoNavMessage {
        preamble: value.preamble,
        time_of_day_s: value.time_of_day_s,
        ura: value.ura,
        x_m: value.x_m,
        y_m: value.y_m,
        z_m: value.z_m,
        x_rate_m_s: value.x_rate_m_s,
        y_rate_m_s: value.y_rate_m_s,
        z_rate_m_s: value.z_rate_m_s,
        x_accel_m_s2: value.x_accel_m_s2,
        y_accel_m_s2: value.y_accel_m_s2,
        z_accel_m_s2: value.z_accel_m_s2,
        a_gf0_s: value.a_gf0_s,
        a_gf1_s_s: value.a_gf1_s_s,
    }
}

fn sbas_igp_mask_to_c(value: &SbasIgpMask) -> SidereonSbasIgpMask {
    SidereonSbasIgpMask {
        preamble: value.preamble,
        band_number: value.band_number,
        iodi: value.iodi,
        mask: value.mask,
    }
}

fn sbas_mixed_fast_to_c(value: &SbasMixedFastCorrections) -> SidereonSbasMixedFastCorrections {
    SidereonSbasMixedFastCorrections {
        iodf: value.iodf,
        iodp: value.iodp,
        block_id: value.block_id,
        prc: value.prc,
        udrei: value.udrei,
    }
}

fn sbas_long_half_to_c(value: &SbasLongTermHalf) -> SidereonSbasLongTermHalfInfo {
    SidereonSbasLongTermHalfInfo {
        velocity_code: value.velocity_code,
        iodp: value.iodp,
        record_count: value.records.len(),
    }
}

fn sbas_long_record_to_c(value: &SbasLongTermRecord) -> SidereonSbasLongTermRecord {
    SidereonSbasLongTermRecord {
        monitored_index: value.monitored_index,
        iode: value.iode,
        delta_x: value.delta_x,
        delta_y: value.delta_y,
        delta_z: value.delta_z,
        delta_x_rate: value.delta_x_rate,
        delta_y_rate: value.delta_y_rate,
        delta_z_rate: value.delta_z_rate,
        delta_a_f0: value.delta_a_f0,
        delta_a_f1: value.delta_a_f1,
        has_time_of_day_s: value.time_of_day_s.is_some(),
        time_of_day_s: value.time_of_day_s.unwrap_or(0),
    }
}

fn sbas_iono_delays_to_c(value: &SbasIonoDelays) -> SidereonSbasIonoDelays {
    SidereonSbasIonoDelays {
        preamble: value.preamble,
        band_number: value.band_number,
        block_id: value.block_id,
        iodi: value.iodi,
        entries: std::array::from_fn(|idx| sbas_igp_delay_to_c(&value.entries[idx])),
    }
}

fn sbas_long_half_for_index(message: &SbasMessage, index: usize) -> Option<&SbasLongTermHalf> {
    match message {
        SbasMessage::LongTermCorrections(value) => value.halves.get(index),
        SbasMessage::MixedCorrections(value) if index == 0 => Some(&value.long_term),
        _ => None,
    }
}

fn sbas_raw_data(message: &SbasMessage) -> Option<&[u8]> {
    match message {
        SbasMessage::DoNotUse(value) => Some(&value.data),
        SbasMessage::NetworkTime(value) => Some(&value.data),
        SbasMessage::GeoAlmanac(value) => Some(&value.data),
        SbasMessage::Unsupported(value) => Some(&value.data),
        _ => None,
    }
}

fn map_sbas_error(fn_name: &str, err: CoreError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        CoreError::InvalidInput(_) | CoreError::Parse(_) => SidereonStatus::InvalidArgument,
        _ => SidereonStatus::Solve,
    }
}

fn sbas_message_kind_to_c(message: &SbasMessage) -> SidereonSbasMessageKind {
    match message {
        SbasMessage::DoNotUse(_) => SidereonSbasMessageKind::DoNotUse,
        SbasMessage::PrnMask(_) => SidereonSbasMessageKind::PrnMask,
        SbasMessage::FastCorrections(_) => SidereonSbasMessageKind::FastCorrections,
        SbasMessage::Integrity(_) => SidereonSbasMessageKind::Integrity,
        SbasMessage::FastDegradation(_) => SidereonSbasMessageKind::FastDegradation,
        SbasMessage::GeoNav(_) => SidereonSbasMessageKind::GeoNav,
        SbasMessage::NetworkTime(_) => SidereonSbasMessageKind::NetworkTime,
        SbasMessage::GeoAlmanac(_) => SidereonSbasMessageKind::GeoAlmanac,
        SbasMessage::IgpMask(_) => SidereonSbasMessageKind::IgpMask,
        SbasMessage::MixedCorrections(_) => SidereonSbasMessageKind::MixedCorrections,
        SbasMessage::LongTermCorrections(_) => SidereonSbasMessageKind::LongTermCorrections,
        SbasMessage::IonoDelays(_) => SidereonSbasMessageKind::IonoDelays,
        SbasMessage::Unsupported(_) => SidereonSbasMessageKind::Unsupported,
    }
}

fn sbas_message_preamble(message: &SbasMessage) -> u8 {
    match message {
        SbasMessage::DoNotUse(m) => m.preamble,
        SbasMessage::PrnMask(m) => m.preamble,
        SbasMessage::FastCorrections(m) => m.preamble,
        SbasMessage::Integrity(m) => m.preamble,
        SbasMessage::FastDegradation(m) => m.preamble,
        SbasMessage::GeoNav(m) => m.preamble,
        SbasMessage::NetworkTime(m) => m.preamble,
        SbasMessage::GeoAlmanac(m) => m.preamble,
        SbasMessage::IgpMask(m) => m.preamble,
        SbasMessage::MixedCorrections(m) => m.preamble,
        SbasMessage::LongTermCorrections(m) => m.preamble,
        SbasMessage::IonoDelays(m) => m.preamble,
        SbasMessage::Unsupported(m) => m.preamble,
    }
}

fn sbas_igp_delay_to_c(value: &SbasIgpDelay) -> SidereonSbasIgpDelay {
    SidereonSbasIgpDelay {
        vertical_delay: value.vertical_delay,
        givei: value.givei,
    }
}
