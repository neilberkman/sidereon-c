use super::*;

/// Write the frequency-dependent phase-center offset (north/east/up, meters) for
/// `frequency` into out_neu (three doubles). Reports SIDEREON_STATUS_INVALID_ARGUMENT
/// if the antenna has no such frequency.
///
/// Safety: antenna must be a live handle from sidereon_antex_antenna; frequency
/// must be a null-terminated C string; out_neu must point to three writable
/// doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antenna_pco(
    antenna: *const SidereonAntenna,
    frequency: *const c_char,
    out_neu: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_antenna_pco", SidereonStatus::Panic, || {
        let antenna = c_try!(require_ref(antenna, "sidereon_antenna_pco", "antenna"));
        c_try!(require_out(out_neu, "sidereon_antenna_pco", "out_neu"));
        zero_f64_prefix(out_neu, 3, 3);
        let frequency = c_try!(parse_bounded_c_string(
            "sidereon_antenna_pco",
            "frequency",
            frequency,
            MAX_ANTEX_FREQUENCY_BYTES
        ));
        let pco = match antenna.inner.pco(&frequency) {
            Ok(pco) => pco,
            Err(err) => return map_antex_error("sidereon_antenna_pco", err),
        };
        c_try!(copy_exact_f64s(
            "sidereon_antenna_pco",
            "out_neu",
            out_neu,
            3,
            &pco
        ));
        SidereonStatus::Ok
    })
}

/// Write the frequency-dependent phase-center variation (meters) at `zenith_deg`
/// to *out_value, with the engine's linear zenith/azimuth interpolation. When
/// has_azimuth is false the no-azimuth grid is used and azimuth_deg is ignored;
/// when true azimuth_deg selects the azimuth slice (the no-azimuth grid is still
/// used if the antenna has no azimuth-dependent samples).
///
/// Safety: antenna must be a live handle from sidereon_antex_antenna; frequency
/// must be a null-terminated C string; out_value must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antenna_pcv(
    antenna: *const SidereonAntenna,
    frequency: *const c_char,
    zenith_deg: f64,
    has_azimuth: bool,
    azimuth_deg: f64,
    out_value: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_antenna_pcv", SidereonStatus::Panic, || {
        let out_value = c_try!(require_out(out_value, "sidereon_antenna_pcv", "out_value"));
        *out_value = 0.0;
        let antenna = c_try!(require_ref(antenna, "sidereon_antenna_pcv", "antenna"));
        let frequency = c_try!(parse_bounded_c_string(
            "sidereon_antenna_pcv",
            "frequency",
            frequency,
            MAX_ANTEX_FREQUENCY_BYTES
        ));
        let azimuth = if has_azimuth { Some(azimuth_deg) } else { None };
        let value = match antenna.inner.pcv(&frequency, zenith_deg, azimuth) {
            Ok(value) => value,
            Err(err) => return map_antex_error("sidereon_antenna_pcv", err),
        };
        *out_value = value;
        SidereonStatus::Ok
    })
}

/// Release an antenna handle from sidereon_antex_antenna. Passing NULL is a
/// no-op.
///
/// Safety: antenna must be NULL or a live handle from sidereon_antex_antenna
/// that has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_antenna_free(antenna: *mut SidereonAntenna) {
    ffi_boundary("sidereon_antenna_free", (), || {
        free_boxed(antenna);
    });
}

/// One receiver-antenna no-azimuth PCV sample in meters.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonReceiverAntennaNoaziPcvSample {
    /// Zenith angle in degrees.
    pub zenith_deg: f64,
    /// PCV correction in meters.
    pub value_m: f64,
}

/// One receiver-antenna azimuth-dependent PCV sample in meters.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonReceiverAntennaAzimuthPcvSample {
    /// Azimuth angle in degrees.
    pub azimuth_deg: f64,
    /// Zenith angle in degrees.
    pub zenith_deg: f64,
    /// PCV correction in meters.
    pub value_m: f64,
}

/// Receiver-antenna calibration for one frequency. The PCO vector is local
/// north/east/up in meters. Set noazi_pcv_m or azimuth_pcv_m to NULL only when
/// the corresponding count is zero.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonReceiverAntennaCalibration {
    /// Phase-center offset in local north/east/up meters.
    pub pco_neu_m: [f64; 3],
    /// Pointer to noazi_pcv_count no-azimuth PCV samples.
    pub noazi_pcv_m: *const SidereonReceiverAntennaNoaziPcvSample,
    /// Number of no-azimuth PCV samples.
    pub noazi_pcv_count: usize,
    /// Pointer to azimuth_pcv_count azimuth-dependent PCV samples.
    pub azimuth_pcv_m: *const SidereonReceiverAntennaAzimuthPcvSample,
    /// Number of azimuth-dependent PCV samples.
    pub azimuth_pcv_count: usize,
}
