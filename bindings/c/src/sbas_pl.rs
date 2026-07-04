use super::*;

// --- SBAS protection levels -------------------------------------------------

/// Typed error detail for SBAS protection-level functions.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSbasPlError {
    /// No SBAS protection-level error occurred.
    None = 0,
    /// The geometry does not have enough independent rows.
    InsufficientGeometry = 1,
    /// A matrix operation or covariance projection failed.
    NumericalFailure = 2,
    /// The supplied error model is missing, non-finite, or outside its domain.
    InvalidErrorModel = 3,
}

/// Fixed SBAS protection-level multipliers.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSbasKMultipliers {
    /// Horizontal multiplier applied to the horizontal semi-major axis.
    pub k_h: f64,
    /// Vertical multiplier applied to the vertical one-sigma standard deviation.
    pub k_v: f64,
}

/// SBAS protection-level output for one geometry snapshot.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSbasProtection {
    /// Horizontal protection level, meters.
    pub hpl_m: f64,
    /// Vertical protection level, meters.
    pub vpl_m: f64,
    /// Horizontal one-sigma semi-major axis, meters.
    pub d_major_m: f64,
    /// Vertical one-sigma standard deviation, meters.
    pub sigma_u_m: f64,
    /// East one-sigma standard deviation, meters.
    pub d_east_m: f64,
    /// North one-sigma standard deviation, meters.
    pub d_north_m: f64,
    /// East-north covariance term, square meters.
    pub d_en_m2: f64,
}

/// One satellite row in an SBAS protection geometry snapshot.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSbasProtectionRow {
    /// Null-terminated satellite token.
    pub sat_id: *const c_char,
    /// Receiver-to-satellite ECEF unit line of sight.
    pub line_of_sight: SidereonLineOfSight,
    /// GNSS system as SidereonGnssSystem.
    pub system: u32,
    /// Elevation angle at the receiver, radians.
    pub elevation_rad: f64,
}

/// SBAS protection geometry input. Arrays are caller-owned.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSbasProtectionGeometry {
    /// Satellite rows.
    pub rows: *const SidereonSbasProtectionRow,
    /// Number of satellite rows.
    pub row_count: usize,
    /// Receiver WGS84 geodetic position.
    pub receiver: SidereonGeodetic,
    /// Receiver-clock systems as SidereonGnssSystem values.
    pub clock_systems: *const u32,
    /// Number of receiver-clock systems.
    pub clock_system_count: usize,
}

/// One satellite's SBAS one-sigma range-error budget.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSbasSisError {
    /// Null-terminated satellite token matching a protection geometry row.
    pub sat_id: *const c_char,
    /// Fast and long-term correction residual sigma, meters.
    pub sigma_flt_m: f64,
    /// User ionospheric range-error sigma, meters.
    pub sigma_uire_m: f64,
    /// Airborne receiver noise, divergence, and multipath sigma, meters.
    pub sigma_air_m: f64,
    /// Tropospheric residual sigma, meters.
    pub sigma_tropo_m: f64,
}

/// Index-aligned SBAS error model for protection-level geometry rows.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSbasErrorModel {
    /// Per-satellite range-error rows.
    pub rows: *const SidereonSbasSisError,
    /// Number of range-error rows.
    pub row_count: usize,
}

/// Airborne receiver and multipath contribution model.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonAirborneModel {
    /// Receiver noise and code-carrier divergence sigma, meters.
    pub sigma_noise_divergence_m: f64,
}

/// Supplied SBAS degradation terms.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonDegradationParams {
    /// Variance multiplier applied to the UDRE variance table.
    pub delta_udre: f64,
    /// Fast-correction degradation term, meters.
    pub eps_fc_m: f64,
    /// Range-rate-correction degradation term, meters.
    pub eps_rrc_m: f64,
    /// Long-term-correction degradation term, meters.
    pub eps_ltc_m: f64,
    /// En-route degradation term, meters.
    pub eps_er_m: f64,
    /// Ionospheric degradation term added to UIRE, meters.
    pub eps_iono_m: f64,
    /// True when UDRE degradation terms are combined by root-sum-square.
    pub rss_udre: bool,
}

/// Initialize the precision-approach SBAS K multipliers.
///
/// Safety: out_k must point to writable SidereonSbasKMultipliers storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_k_multipliers_precision_approach(
    out_k: *mut SidereonSbasKMultipliers,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sbas_k_multipliers_precision_approach",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_k,
                "sidereon_sbas_k_multipliers_precision_approach",
                "out_k"
            ));
            *out = sbas_k_to_c(CoreSbasKMultipliers::PRECISION_APPROACH);
            SidereonStatus::Ok
        },
    )
}

/// Initialize the en-route through non-precision-approach SBAS K multipliers.
///
/// Safety: out_k must point to writable SidereonSbasKMultipliers storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_k_multipliers_en_route_npa(
    out_k: *mut SidereonSbasKMultipliers,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sbas_k_multipliers_en_route_npa",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_k,
                "sidereon_sbas_k_multipliers_en_route_npa",
                "out_k"
            ));
            *out = sbas_k_to_c(CoreSbasKMultipliers::EN_ROUTE_NPA);
            SidereonStatus::Ok
        },
    )
}

/// Initialize the AAD-A airborne model.
///
/// Safety: out_model must point to writable SidereonAirborneModel storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_airborne_model_aad_a(
    out_model: *mut SidereonAirborneModel,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sbas_airborne_model_aad_a",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_model,
                "sidereon_sbas_airborne_model_aad_a",
                "out_model"
            ));
            *out = airborne_model_to_c(CoreAirborneModel::aad_a());
            SidereonStatus::Ok
        },
    )
}

/// Initialize degradation terms to the no-extra-degradation values.
///
/// Safety: out_params must point to writable SidereonDegradationParams storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_degradation_params_none(
    out_params: *mut SidereonDegradationParams,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sbas_degradation_params_none",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_params,
                "sidereon_sbas_degradation_params_none",
                "out_params"
            ));
            *out = degradation_params_to_c(CoreDegradationParams::none());
            SidereonStatus::Ok
        },
    )
}

/// Compute fast and long-term residual sigma from UDREI and degradation terms.
///
/// Safety: degradation and out_sigma_m must point to their documented storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_sigma_flt_m_for_udrei(
    udrei: u8,
    degradation: *const SidereonDegradationParams,
    out_sigma_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sbas_sigma_flt_m_for_udrei",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_sigma_m,
                "sidereon_sbas_sigma_flt_m_for_udrei",
                "out_sigma_m"
            ));
            *out = 0.0;
            let degradation = c_try!(require_ref(
                degradation,
                "sidereon_sbas_sigma_flt_m_for_udrei",
                "degradation"
            ));
            match sidereon_core::sbas_pl::sigma_flt_m_for_udrei(
                udrei,
                &degradation_params_from_c(degradation),
            ) {
                Some(sigma_m) => {
                    *out = sigma_m;
                    SidereonStatus::Ok
                }
                None => {
                    set_last_error("sidereon_sbas_sigma_flt_m_for_udrei: invalid input");
                    SidereonStatus::InvalidArgument
                }
            }
        },
    )
}

/// Compute SBAS tropospheric residual sigma at an elevation angle.
///
/// Safety: out_sigma_m must point to writable double storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_sigma_tropo_m(
    elevation_rad: f64,
    out_sigma_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_sbas_sigma_tropo_m", SidereonStatus::Panic, || {
        let out = c_try!(require_out(
            out_sigma_m,
            "sidereon_sbas_sigma_tropo_m",
            "out_sigma_m"
        ));
        *out = 0.0;
        match sidereon_core::sbas_pl::sigma_tropo_m(elevation_rad) {
            Some(sigma_m) => {
                *out = sigma_m;
                SidereonStatus::Ok
            }
            None => {
                set_last_error("sidereon_sbas_sigma_tropo_m: invalid elevation_rad");
                SidereonStatus::InvalidArgument
            }
        }
    })
}

/// Compute airborne receiver, divergence, and multipath sigma.
///
/// Safety: model and out_sigma_m must point to their documented storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_airborne_sigma_air_m(
    model: *const SidereonAirborneModel,
    elevation_rad: f64,
    out_sigma_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sbas_airborne_sigma_air_m",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_sigma_m,
                "sidereon_sbas_airborne_sigma_air_m",
                "out_sigma_m"
            ));
            *out = 0.0;
            let model = c_try!(require_ref(
                model,
                "sidereon_sbas_airborne_sigma_air_m",
                "model"
            ));
            match airborne_model_from_c(model).sigma_air_m(elevation_rad) {
                Some(sigma_m) => {
                    *out = sigma_m;
                    SidereonStatus::Ok
                }
                None => {
                    set_last_error("sidereon_sbas_airborne_sigma_air_m: invalid input");
                    SidereonStatus::InvalidArgument
                }
            }
        },
    )
}

/// Compute the total one-sigma range error for one SBAS SIS error row.
///
/// Safety: row and out_sigma_m must point to their documented storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_sis_error_sigma_m(
    row: *const SidereonSbasSisError,
    out_sigma_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sbas_sis_error_sigma_m",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_sigma_m,
                "sidereon_sbas_sis_error_sigma_m",
                "out_sigma_m"
            ));
            *out = 0.0;
            let row = c_try!(require_ref(row, "sidereon_sbas_sis_error_sigma_m", "row"));
            let row = c_try!(sbas_sis_error_from_c(
                "sidereon_sbas_sis_error_sigma_m",
                row
            ));
            match row.sigma_m() {
                Some(sigma_m) => {
                    *out = sigma_m;
                    SidereonStatus::Ok
                }
                None => {
                    set_last_error("sidereon_sbas_sis_error_sigma_m: invalid row");
                    SidereonStatus::InvalidArgument
                }
            }
        },
    )
}

/// Compute SBAS HPL and VPL from protection geometry and range-error rows.
///
/// Safety: geometry, model, out_protection, and out_error must point to their
/// documented storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sbas_protection_levels(
    geometry: *const SidereonSbasProtectionGeometry,
    model: *const SidereonSbasErrorModel,
    k: SidereonSbasKMultipliers,
    out_protection: *mut SidereonSbasProtection,
    out_error: *mut SidereonSbasPlError,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sbas_protection_levels",
        SidereonStatus::Panic,
        || {
            let out_protection = c_try!(require_out(
                out_protection,
                "sidereon_sbas_protection_levels",
                "out_protection"
            ));
            *out_protection = empty_sbas_protection();
            let out_error = c_try!(require_out(
                out_error,
                "sidereon_sbas_protection_levels",
                "out_error"
            ));
            *out_error = SidereonSbasPlError::None;
            let geometry = c_try!(require_ref(
                geometry,
                "sidereon_sbas_protection_levels",
                "geometry"
            ));
            let model = c_try!(require_ref(
                model,
                "sidereon_sbas_protection_levels",
                "model"
            ));
            let geometry = c_try!(sbas_protection_geometry_from_c(
                "sidereon_sbas_protection_levels",
                geometry
            ));
            let model = c_try!(sbas_error_model_from_c(
                "sidereon_sbas_protection_levels",
                model
            ));
            match core_sbas_protection_levels(&geometry, &model, sbas_k_from_c(k)) {
                Ok(protection) => {
                    *out_protection = sbas_protection_to_c(protection);
                    SidereonStatus::Ok
                }
                Err(err) => map_sbas_pl_error("sidereon_sbas_protection_levels", err, out_error),
            }
        },
    )
}

fn sbas_k_to_c(value: CoreSbasKMultipliers) -> SidereonSbasKMultipliers {
    SidereonSbasKMultipliers {
        k_h: value.k_h,
        k_v: value.k_v,
    }
}

fn sbas_k_from_c(value: SidereonSbasKMultipliers) -> CoreSbasKMultipliers {
    CoreSbasKMultipliers {
        k_h: value.k_h,
        k_v: value.k_v,
    }
}

fn sbas_protection_to_c(value: CoreSbasProtection) -> SidereonSbasProtection {
    SidereonSbasProtection {
        hpl_m: value.hpl_m,
        vpl_m: value.vpl_m,
        d_major_m: value.d_major_m,
        sigma_u_m: value.sigma_u_m,
        d_east_m: value.d_east_m,
        d_north_m: value.d_north_m,
        d_en_m2: value.d_en_m2,
    }
}

fn empty_sbas_protection() -> SidereonSbasProtection {
    SidereonSbasProtection {
        hpl_m: 0.0,
        vpl_m: 0.0,
        d_major_m: 0.0,
        sigma_u_m: 0.0,
        d_east_m: 0.0,
        d_north_m: 0.0,
        d_en_m2: 0.0,
    }
}

fn airborne_model_to_c(value: CoreAirborneModel) -> SidereonAirborneModel {
    SidereonAirborneModel {
        sigma_noise_divergence_m: value.sigma_noise_divergence_m,
    }
}

fn airborne_model_from_c(value: &SidereonAirborneModel) -> CoreAirborneModel {
    CoreAirborneModel::new(value.sigma_noise_divergence_m)
}

fn degradation_params_to_c(value: CoreDegradationParams) -> SidereonDegradationParams {
    SidereonDegradationParams {
        delta_udre: value.delta_udre,
        eps_fc_m: value.eps_fc_m,
        eps_rrc_m: value.eps_rrc_m,
        eps_ltc_m: value.eps_ltc_m,
        eps_er_m: value.eps_er_m,
        eps_iono_m: value.eps_iono_m,
        rss_udre: value.rss_udre,
    }
}

fn degradation_params_from_c(value: &SidereonDegradationParams) -> CoreDegradationParams {
    CoreDegradationParams {
        delta_udre: value.delta_udre,
        eps_fc_m: value.eps_fc_m,
        eps_rrc_m: value.eps_rrc_m,
        eps_ltc_m: value.eps_ltc_m,
        eps_er_m: value.eps_er_m,
        eps_iono_m: value.eps_iono_m,
        rss_udre: value.rss_udre,
    }
}

unsafe fn sbas_protection_geometry_from_c(
    fn_name: &str,
    value: &SidereonSbasProtectionGeometry,
) -> Result<AraimGeometry, SidereonStatus> {
    let rows = require_slice(value.rows, value.row_count, fn_name, "geometry.rows")?;
    let mut parsed_rows = Vec::with_capacity(rows.len());
    for (idx, row) in rows.iter().enumerate() {
        let id = parse_satellite_token(fn_name, row.sat_id)?;
        let system =
            gnss_system_from_c_code(fn_name, &format!("geometry.rows[{idx}].system"), row.system)?;
        parsed_rows.push(AraimRow {
            id,
            line_of_sight: LineOfSight::new(
                row.line_of_sight.e_x,
                row.line_of_sight.e_y,
                row.line_of_sight.e_z,
            ),
            system,
            elevation_rad: row.elevation_rad,
        });
    }
    let receiver = geodetic_to_wgs84(fn_name, "geometry.receiver", value.receiver)?;
    let raw_systems = require_slice(
        value.clock_systems,
        value.clock_system_count,
        fn_name,
        "geometry.clock_systems",
    )?;
    let mut clock_systems = Vec::with_capacity(raw_systems.len());
    for (idx, &system) in raw_systems.iter().enumerate() {
        clock_systems.push(gnss_system_from_c_code(
            fn_name,
            &format!("geometry.clock_systems[{idx}]"),
            system,
        )?);
    }
    Ok(AraimGeometry {
        rows: parsed_rows,
        receiver,
        clock_systems,
    })
}

unsafe fn sbas_error_model_from_c(
    fn_name: &str,
    value: &SidereonSbasErrorModel,
) -> Result<CoreSbasErrorModel, SidereonStatus> {
    let rows = require_slice(value.rows, value.row_count, fn_name, "model.rows")?;
    let mut out = Vec::with_capacity(rows.len());
    for (idx, row) in rows.iter().enumerate() {
        let id = parse_satellite_token_for_arg(
            fn_name,
            &format!("model.rows[{idx}].sat_id"),
            row.sat_id,
        )?;
        out.push(CoreSbasSisError {
            id,
            sigma_flt_m: row.sigma_flt_m,
            sigma_uire_m: row.sigma_uire_m,
            sigma_air_m: row.sigma_air_m,
            sigma_tropo_m: row.sigma_tropo_m,
        });
    }
    Ok(CoreSbasErrorModel::new(out))
}

unsafe fn sbas_sis_error_from_c(
    fn_name: &str,
    row: &SidereonSbasSisError,
) -> Result<CoreSbasSisError, SidereonStatus> {
    Ok(CoreSbasSisError {
        id: parse_satellite_token_for_arg(fn_name, "row.sat_id", row.sat_id)?,
        sigma_flt_m: row.sigma_flt_m,
        sigma_uire_m: row.sigma_uire_m,
        sigma_air_m: row.sigma_air_m,
        sigma_tropo_m: row.sigma_tropo_m,
    })
}

unsafe fn parse_satellite_token_for_arg(
    fn_name: &str,
    arg_name: &str,
    sat_id: *const c_char,
) -> Result<GnssSatelliteId, SidereonStatus> {
    if sat_id.is_null() {
        set_last_error(format!("{fn_name}: null {arg_name}"));
        return Err(SidereonStatus::NullPointer);
    }
    let mut token_len = None;
    for idx in 0..=MAX_SATELLITE_TOKEN_BYTES {
        if *sat_id.add(idx) == 0 {
            token_len = Some(idx);
            break;
        }
    }
    let Some(token_len) = token_len else {
        set_last_error(format!(
            "{fn_name}: {arg_name} is not null-terminated within {MAX_SATELLITE_TOKEN_BYTES} bytes"
        ));
        return Err(SidereonStatus::InvalidArgument);
    };
    let bytes = slice::from_raw_parts(sat_id.cast::<u8>(), token_len);
    let token = match str::from_utf8(bytes) {
        Ok(token) => token,
        Err(_) => {
            set_last_error(format!("{fn_name}: {arg_name} is not valid UTF-8"));
            return Err(SidereonStatus::InvalidToken);
        }
    };
    GnssSatelliteId::from_str(token).map_err(|_| {
        set_last_error(format!("{fn_name}: invalid {arg_name}: {token}"));
        SidereonStatus::InvalidToken
    })
}

fn map_sbas_pl_error(
    fn_name: &str,
    err: CoreSbasPlError,
    out_error: &mut SidereonSbasPlError,
) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    *out_error = match err {
        CoreSbasPlError::InsufficientGeometry => SidereonSbasPlError::InsufficientGeometry,
        CoreSbasPlError::NumericalFailure => SidereonSbasPlError::NumericalFailure,
        CoreSbasPlError::InvalidErrorModel => SidereonSbasPlError::InvalidErrorModel,
    };
    match err {
        CoreSbasPlError::InvalidErrorModel => SidereonStatus::InvalidArgument,
        CoreSbasPlError::InsufficientGeometry | CoreSbasPlError::NumericalFailure => {
            SidereonStatus::Solve
        }
    }
}
