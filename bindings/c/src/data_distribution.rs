//! GNSS product identity, public-distributor derivation, and exact cache IO.
//!
//! Callers own transport and product parsing. The exact-cache functions expose
//! the same cross-process lock and atomic immutable-entry protocol as the other
//! Sidereon interfaces.

use std::ffi::c_char;
use std::ptr;
use std::time::Duration;

use sidereon_core::data::{
    self as core_data, AnalysisCenter, ArchiveCompression, DistributionSource, ProductCampaign,
    ProductDate, ProductDateTime, ProductFormat, ProductIdentity, ProductPublisher, ProductType,
    SolutionClass, Sp3ContentStartConvention,
};
use sidereon_core::exact_cache::{
    CommittedExactCacheEntry, ExactCacheError, ExactCacheGuard, ExactCacheOpen, ExactCacheOwner,
    ExactCacheSingleFlightOptions, ExactProductCache,
};

use super::{
    copy_prefix_to_c, ffi_boundary, free_boxed, init_copy_counts, parse_bounded_c_string,
    require_mut, require_out, require_ref, require_slice, set_last_error, write_boxed_handle,
    SidereonStatus,
};

pub const PRODUCT_TOKEN_C_BYTES: usize = 16;
pub const ANALYSIS_CENTER_C_BYTES: usize = 32;
pub const FORMAT_VERSION_C_BYTES: usize = 16;
pub const OFFICIAL_FILENAME_C_BYTES: usize = 160;
pub const ARCHIVE_FILENAME_C_BYTES: usize = 164;
pub const DISTRIBUTION_URL_C_BYTES: usize = 1024;

/// Standard GNSS product family.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidereonProductFamily {
    Sp3 = 0,
    Ionex = 1,
    RinexClock = 2,
    RinexNavigation = 3,
}

/// Public organization that produced or combined the product.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidereonProductPublisher {
    Igs = 0,
    Code = 1,
    Esa = 2,
    Gfz = 3,
    /// Wuhan University IGS Analysis Center (`WUM`).
    Whu = 4,
}

/// Public product solution class.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidereonSolutionClass {
    Final = 0,
    Rapid = 1,
    UltraRapid = 2,
    Predicted = 3,
    Broadcast = 4,
    /// Near-real-time product line, published on an hourly rhythm.
    NearRealTime = 5,
}

/// Public campaign token encoded by the official filename.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidereonProductCampaign {
    Operational = 0,
    MultiGnss = 1,
    MultiGnssExperiment = 2,
    Broadcast = 3,
}

/// Standard serialization format.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidereonProductFormat {
    Sp3 = 0,
    Ionex = 1,
    RinexClock = 2,
    RinexNavigation = 3,
}

/// Explicit public distributor or caller-provided input.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidereonDistributionSource {
    Direct = 0,
    NasaCddis = 1,
    LocalFile = 2,
    InMemory = 3,
}

/// Transport compression applied by a distributor.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidereonArchiveCompression {
    None = 0,
    Gzip = 1,
    /// Historical Unix-compress transport with a `.Z` suffix.
    ///
    /// Appended in ABI version 0.33.0; the existing numeric values are
    /// unchanged.
    UnixCompress = 2,
}

/// Cataloged relationship between an SP3 filename epoch and its first content
/// epoch.
///
/// This is archive metadata, not a value inferred from product bytes. New
/// variants may be appended in later ABI revisions.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidereonSp3ContentStartConvention {
    /// The first content epoch equals the epoch encoded by the filename.
    FilenameEpoch = 0,
    /// The first content epoch is exactly 24 hours before the filename epoch.
    FilenameEpochMinusOneDay = 1,
}

/// One catalog-supported product sampling token.
///
/// `token` is null-terminated. Its storage uses the same documented product-
/// token bound as identity `sample`, `span`, and `issue` fields. Retrieve an
/// exact number of these records with `sidereon_data_supported_samples`'s
/// standard caller-buffer/count contract.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonProductSample {
    pub token: [c_char; PRODUCT_TOKEN_C_BYTES],
}

/// Exact product identity, independent of distributor.
///
/// Fixed text buffers are always null-terminated. `official_filename` excludes
/// distributor transport-compression suffixes.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonProductIdentity {
    /// One of SidereonProductFamily_*, encoded as uint32_t so malformed C
    /// input can be rejected without constructing an invalid Rust enum.
    pub family: u32,
    pub analysis_center: [c_char; ANALYSIS_CENTER_C_BYTES],
    /// One of SidereonProductPublisher_*.
    pub publisher: u32,
    /// One of SidereonSolutionClass_*.
    pub solution_class: u32,
    /// One of SidereonProductCampaign_*.
    pub campaign: u32,
    pub filename_version: u8,
    pub year: i32,
    pub month: u8,
    pub day: u8,
    /// Exactly 0 or 1.
    pub has_issue: u8,
    pub issue: [c_char; PRODUCT_TOKEN_C_BYTES],
    pub span: [c_char; PRODUCT_TOKEN_C_BYTES],
    pub sample: [c_char; PRODUCT_TOKEN_C_BYTES],
    pub official_filename: [c_char; OFFICIAL_FILENAME_C_BYTES],
    /// One of SidereonProductFormat_*.
    pub format: u32,
    /// Exactly 0 or 1.
    pub has_format_version: u8,
    pub format_version: [c_char; FORMAT_VERSION_C_BYTES],
    /// Exactly 0 or 1.
    pub has_prediction_horizon_days: u8,
    pub prediction_horizon_days: u8,
}

/// Public location and transport metadata for one exact product identity.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonDistributionLocation {
    pub source: SidereonDistributionSource,
    pub has_original_url: bool,
    pub original_url: [c_char; DISTRIBUTION_URL_C_BYTES],
    pub archive_filename: [c_char; ARCHIVE_FILENAME_C_BYTES],
    pub compression: SidereonArchiveCompression,
}

/// Lock-owning native exact-product cache transaction.
///
/// Release with `sidereon_exact_cache_free`; releasing it also releases the
/// cross-process entry lock.
pub struct SidereonExactCache {
    cache: ExactProductCache,
    guard: Option<ExactCacheGuard>,
}

/// Immutable digest-verified exact-product cache entry.
///
/// Byte, path, and identifier accessors copy from this handle. Release it with
/// `sidereon_exact_cache_entry_free` after the required copies are complete.
pub struct SidereonExactCacheEntry {
    entry: CommittedExactCacheEntry,
}

/// ABI version for SidereonExactCacheSingleFlightOptions.
pub const SIDEREON_EXACT_CACHE_SINGLE_FLIGHT_OPTIONS_ABI_VERSION: u32 = 1;

/// Bounded timing policy for exact-cache single-flight coordination.
///
/// Initialize with `sidereon_exact_cache_single_flight_options_init`, then
/// override durations as needed. `struct_size` and `abi_version` are checked on
/// every non-NULL use so incompatible layouts fail instead of using defaults.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonExactCacheSingleFlightOptions {
    /// Must remain the initialized `sizeof(SidereonExactCacheSingleFlightOptions)`.
    pub struct_size: u32,
    /// Must remain SIDEREON_EXACT_CACHE_SINGLE_FLIGHT_OPTIONS_ABI_VERSION.
    pub abi_version: u32,
    /// Interval between committed-entry and heartbeat observations.
    pub poll_interval_ms: u64,
    /// Interval between automatic owner heartbeat writes.
    pub heartbeat_interval_ms: u64,
    /// Continuous no-progress interval required before owner retirement.
    pub liveness_timeout_ms: u64,
    /// Maximum total time spent waiting for another owner.
    pub wait_timeout_ms: u64,
}

/// Result discriminant written by `sidereon_exact_cache_open_single_flight`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidereonExactCacheOpenResult {
    /// `out_entry` owns the verified committed entry; `out_owner` is NULL.
    Hit = 0,
    /// `out_owner` owns acquisition; `out_entry` is NULL.
    Owner = 1,
}

/// Exclusive right to fetch and publish one single-flight cache miss.
///
/// Release with `sidereon_exact_cache_owner_free`. Releasing an unpublished
/// owner abandons the attempt and best-effort removes its in-flight marker.
pub struct SidereonExactCacheOwner {
    owner: Option<ExactCacheOwner>,
}

/// Byte/path component of an immutable exact-cache entry.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidereonExactCacheComponent {
    Product = 0,
    Archive = 1,
    Provenance = 2,
}

fn map_error(fn_name: &str, error: impl core::fmt::Display) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {error}"));
    SidereonStatus::InvalidArgument
}

fn map_cache_error(fn_name: &str, error: ExactCacheError) -> SidereonStatus {
    let status = if matches!(
        error,
        ExactCacheError::LockTimeout | ExactCacheError::SingleFlightTimeout
    ) {
        SidereonStatus::Timeout
    } else {
        SidereonStatus::InvalidArgument
    };
    set_last_error(format!("{fn_name}: {error}"));
    status
}

fn default_single_flight_options() -> SidereonExactCacheSingleFlightOptions {
    let defaults = ExactCacheSingleFlightOptions::default();
    SidereonExactCacheSingleFlightOptions {
        struct_size: std::mem::size_of::<SidereonExactCacheSingleFlightOptions>() as u32,
        abi_version: SIDEREON_EXACT_CACHE_SINGLE_FLIGHT_OPTIONS_ABI_VERSION,
        poll_interval_ms: defaults.poll_interval.as_millis() as u64,
        heartbeat_interval_ms: defaults.heartbeat_interval.as_millis() as u64,
        liveness_timeout_ms: defaults.liveness_timeout.as_millis() as u64,
        wait_timeout_ms: defaults.wait_timeout.as_millis() as u64,
    }
}

unsafe fn single_flight_options_from_c(
    fn_name: &str,
    options: *const SidereonExactCacheSingleFlightOptions,
) -> Result<ExactCacheSingleFlightOptions, SidereonStatus> {
    let Some(options) = options.as_ref() else {
        return Ok(ExactCacheSingleFlightOptions::default());
    };
    let expected_size = std::mem::size_of::<SidereonExactCacheSingleFlightOptions>();
    if options.struct_size as usize != expected_size {
        set_last_error(format!(
            "{fn_name}: options struct_size must be {expected_size}"
        ));
        return Err(SidereonStatus::InvalidArgument);
    }
    if options.abi_version != SIDEREON_EXACT_CACHE_SINGLE_FLIGHT_OPTIONS_ABI_VERSION {
        set_last_error(format!(
            "{fn_name}: unsupported options abi_version {}",
            options.abi_version
        ));
        return Err(SidereonStatus::InvalidArgument);
    }
    Ok(ExactCacheSingleFlightOptions {
        poll_interval: Duration::from_millis(options.poll_interval_ms),
        heartbeat_interval: Duration::from_millis(options.heartbeat_interval_ms),
        liveness_timeout: Duration::from_millis(options.liveness_timeout_ms),
        wait_timeout: Duration::from_millis(options.wait_timeout_ms),
    })
}

pub(super) fn fixed_text<const N: usize>(
    fn_name: &str,
    label: &str,
    value: &str,
) -> Result<[c_char; N], SidereonStatus> {
    if value.as_bytes().contains(&0) || value.len() >= N {
        set_last_error(format!(
            "{fn_name}: {label} exceeds its fixed output buffer"
        ));
        return Err(SidereonStatus::InvalidArgument);
    }
    let mut output = [0; N];
    for (target, byte) in output.iter_mut().zip(value.bytes()) {
        *target = byte as c_char;
    }
    Ok(output)
}

fn invalid_discriminant(fn_name: &str, label: &str, value: u32) -> SidereonStatus {
    set_last_error(format!(
        "{fn_name}: {label} has invalid discriminant {value}"
    ));
    SidereonStatus::InvalidArgument
}

pub(super) fn bool_from_c(fn_name: &str, label: &str, value: u8) -> Result<bool, SidereonStatus> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => {
            set_last_error(format!("{fn_name}: {label} must be exactly 0 or 1"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn family_from_c(fn_name: &str, label: &str, value: u32) -> Result<ProductType, SidereonStatus> {
    match value {
        value if value == SidereonProductFamily::Sp3 as u32 => Ok(ProductType::Sp3),
        value if value == SidereonProductFamily::Ionex as u32 => Ok(ProductType::Ionex),
        value if value == SidereonProductFamily::RinexClock as u32 => Ok(ProductType::Clk),
        value if value == SidereonProductFamily::RinexNavigation as u32 => Ok(ProductType::Nav),
        _ => Err(invalid_discriminant(fn_name, label, value)),
    }
}

pub(super) fn source_from_c(
    fn_name: &str,
    label: &str,
    value: u32,
) -> Result<DistributionSource, SidereonStatus> {
    match value {
        value if value == SidereonDistributionSource::Direct as u32 => {
            Ok(DistributionSource::Direct)
        }
        value if value == SidereonDistributionSource::NasaCddis as u32 => {
            Ok(DistributionSource::NasaCddis)
        }
        value if value == SidereonDistributionSource::LocalFile as u32 => {
            Ok(DistributionSource::LocalFile)
        }
        value if value == SidereonDistributionSource::InMemory as u32 => {
            Ok(DistributionSource::InMemory)
        }
        _ => Err(invalid_discriminant(fn_name, label, value)),
    }
}

pub(super) fn compression_from_c(
    fn_name: &str,
    label: &str,
    value: u32,
) -> Result<ArchiveCompression, SidereonStatus> {
    match value {
        value if value == SidereonArchiveCompression::None as u32 => Ok(ArchiveCompression::None),
        value if value == SidereonArchiveCompression::Gzip as u32 => Ok(ArchiveCompression::Gzip),
        value if value == SidereonArchiveCompression::UnixCompress as u32 => {
            Ok(ArchiveCompression::UnixCompress)
        }
        _ => Err(invalid_discriminant(fn_name, label, value)),
    }
}

fn publisher_from_c(
    fn_name: &str,
    label: &str,
    value: u32,
) -> Result<ProductPublisher, SidereonStatus> {
    match value {
        value if value == SidereonProductPublisher::Igs as u32 => Ok(ProductPublisher::Igs),
        value if value == SidereonProductPublisher::Code as u32 => Ok(ProductPublisher::Code),
        value if value == SidereonProductPublisher::Esa as u32 => Ok(ProductPublisher::Esa),
        value if value == SidereonProductPublisher::Gfz as u32 => Ok(ProductPublisher::Gfz),
        value if value == SidereonProductPublisher::Whu as u32 => Ok(ProductPublisher::Whu),
        _ => Err(invalid_discriminant(fn_name, label, value)),
    }
}

fn solution_from_c(
    fn_name: &str,
    label: &str,
    value: u32,
) -> Result<SolutionClass, SidereonStatus> {
    match value {
        value if value == SidereonSolutionClass::Final as u32 => Ok(SolutionClass::Final),
        value if value == SidereonSolutionClass::Rapid as u32 => Ok(SolutionClass::Rapid),
        value if value == SidereonSolutionClass::UltraRapid as u32 => Ok(SolutionClass::UltraRapid),
        value if value == SidereonSolutionClass::Predicted as u32 => Ok(SolutionClass::Predicted),
        value if value == SidereonSolutionClass::Broadcast as u32 => Ok(SolutionClass::Broadcast),
        value if value == SidereonSolutionClass::NearRealTime as u32 => {
            Ok(SolutionClass::NearRealTime)
        }
        _ => Err(invalid_discriminant(fn_name, label, value)),
    }
}

fn campaign_from_c(
    fn_name: &str,
    label: &str,
    value: u32,
) -> Result<ProductCampaign, SidereonStatus> {
    match value {
        value if value == SidereonProductCampaign::Operational as u32 => {
            Ok(ProductCampaign::Operational)
        }
        value if value == SidereonProductCampaign::MultiGnss as u32 => {
            Ok(ProductCampaign::MultiGnss)
        }
        value if value == SidereonProductCampaign::MultiGnssExperiment as u32 => {
            Ok(ProductCampaign::MultiGnssExperiment)
        }
        value if value == SidereonProductCampaign::Broadcast as u32 => {
            Ok(ProductCampaign::Broadcast)
        }
        _ => Err(invalid_discriminant(fn_name, label, value)),
    }
}

fn format_from_c(fn_name: &str, label: &str, value: u32) -> Result<ProductFormat, SidereonStatus> {
    match value {
        value if value == SidereonProductFormat::Sp3 as u32 => Ok(ProductFormat::Sp3),
        value if value == SidereonProductFormat::Ionex as u32 => Ok(ProductFormat::Ionex),
        value if value == SidereonProductFormat::RinexClock as u32 => Ok(ProductFormat::RinexClock),
        value if value == SidereonProductFormat::RinexNavigation as u32 => {
            Ok(ProductFormat::RinexNavigation)
        }
        _ => Err(invalid_discriminant(fn_name, label, value)),
    }
}

fn publisher_from_core(value: ProductPublisher) -> SidereonProductPublisher {
    match value {
        ProductPublisher::Igs => SidereonProductPublisher::Igs,
        ProductPublisher::Code => SidereonProductPublisher::Code,
        ProductPublisher::Esa => SidereonProductPublisher::Esa,
        ProductPublisher::Gfz => SidereonProductPublisher::Gfz,
        ProductPublisher::Whu => SidereonProductPublisher::Whu,
    }
}

fn solution_from_core(value: SolutionClass) -> SidereonSolutionClass {
    match value {
        SolutionClass::Final => SidereonSolutionClass::Final,
        SolutionClass::Rapid => SidereonSolutionClass::Rapid,
        SolutionClass::UltraRapid => SidereonSolutionClass::UltraRapid,
        SolutionClass::Predicted => SidereonSolutionClass::Predicted,
        SolutionClass::Broadcast => SidereonSolutionClass::Broadcast,
        SolutionClass::NearRealTime => SidereonSolutionClass::NearRealTime,
    }
}

fn campaign_from_core(value: ProductCampaign) -> SidereonProductCampaign {
    match value {
        ProductCampaign::Operational => SidereonProductCampaign::Operational,
        ProductCampaign::MultiGnss => SidereonProductCampaign::MultiGnss,
        ProductCampaign::MultiGnssExperiment => SidereonProductCampaign::MultiGnssExperiment,
        ProductCampaign::Broadcast => SidereonProductCampaign::Broadcast,
    }
}

fn format_from_core(value: ProductFormat) -> SidereonProductFormat {
    match value {
        ProductFormat::Sp3 => SidereonProductFormat::Sp3,
        ProductFormat::Ionex => SidereonProductFormat::Ionex,
        ProductFormat::RinexClock => SidereonProductFormat::RinexClock,
        ProductFormat::RinexNavigation => SidereonProductFormat::RinexNavigation,
    }
}

fn compression_from_core(value: ArchiveCompression) -> SidereonArchiveCompression {
    match value {
        ArchiveCompression::None => SidereonArchiveCompression::None,
        ArchiveCompression::Gzip => SidereonArchiveCompression::Gzip,
        ArchiveCompression::UnixCompress => SidereonArchiveCompression::UnixCompress,
    }
}

fn sp3_content_start_from_core(
    value: Sp3ContentStartConvention,
) -> Option<SidereonSp3ContentStartConvention> {
    match value {
        Sp3ContentStartConvention::FilenameEpoch => {
            Some(SidereonSp3ContentStartConvention::FilenameEpoch)
        }
        Sp3ContentStartConvention::FilenameEpochMinusOneDay => {
            Some(SidereonSp3ContentStartConvention::FilenameEpochMinusOneDay)
        }
        // The core enum is non-exhaustive. An interface release must add an
        // explicit C discriminant before it can expose any future convention.
        _ => None,
    }
}

pub(super) fn identity_to_c(
    fn_name: &str,
    identity: &ProductIdentity,
) -> Result<SidereonProductIdentity, SidereonStatus> {
    Ok(SidereonProductIdentity {
        family: match identity.family {
            ProductType::Sp3 => SidereonProductFamily::Sp3 as u32,
            ProductType::Ionex => SidereonProductFamily::Ionex as u32,
            ProductType::Clk => SidereonProductFamily::RinexClock as u32,
            ProductType::Nav => SidereonProductFamily::RinexNavigation as u32,
        },
        analysis_center: fixed_text(fn_name, "analysis_center", identity.analysis_center.code())?,
        publisher: publisher_from_core(identity.publisher) as u32,
        solution_class: solution_from_core(identity.solution) as u32,
        campaign: campaign_from_core(identity.campaign) as u32,
        filename_version: identity.version,
        year: identity.date.year,
        month: identity.date.month,
        day: identity.date.day,
        has_issue: u8::from(identity.issue.is_some()),
        issue: fixed_text(fn_name, "issue", identity.issue.as_deref().unwrap_or(""))?,
        span: fixed_text(fn_name, "span", &identity.span)?,
        sample: fixed_text(fn_name, "sample", &identity.sample)?,
        official_filename: fixed_text(fn_name, "official_filename", &identity.official_filename)?,
        format: format_from_core(identity.format) as u32,
        has_format_version: u8::from(identity.format_version.is_some()),
        format_version: fixed_text(
            fn_name,
            "format_version",
            identity.format_version.as_deref().unwrap_or(""),
        )?,
        has_prediction_horizon_days: u8::from(identity.prediction_horizon_days.is_some()),
        prediction_horizon_days: identity.prediction_horizon_days.unwrap_or(0),
    })
}

fn product_identity_json(identity: &ProductIdentity) -> serde_json::Value {
    serde_json::json!({
        "family": identity.family.code(),
        "analysis_center": identity.analysis_center.code(),
        "publisher": identity.publisher.code(),
        "solution_class": identity.solution.code(),
        "campaign": identity.campaign.code(),
        "filename_version": identity.version,
        "date": format!(
            "{:04}-{:02}-{:02}",
            identity.date.year, identity.date.month, identity.date.day
        ),
        "issue": identity.issue.as_deref().unwrap_or(""),
        "span": identity.span,
        "sample": identity.sample,
        "official_filename": identity.official_filename,
        "format": identity.format.code(),
        "format_version": identity.format_version,
        "prediction_horizon_days": identity.prediction_horizon_days,
    })
}

fn nominal_coverage_interval_json(
    interval: Option<core_data::NominalCoverageInterval>,
) -> serde_json::Value {
    match interval {
        Some(interval) => serde_json::json!({
            "from": interval.from.to_string(),
            "until": interval.until.to_string(),
        }),
        None => serde_json::Value::Null,
    }
}

pub(super) fn fixed_text_from_c<const N: usize>(
    fn_name: &str,
    label: &str,
    value: &[c_char; N],
) -> Result<String, SidereonStatus> {
    let end = value.iter().position(|&byte| byte == 0).ok_or_else(|| {
        set_last_error(format!("{fn_name}: {label} is not null-terminated"));
        SidereonStatus::InvalidArgument
    })?;
    let bytes = value[..end]
        .iter()
        .map(|&byte| byte as u8)
        .collect::<Vec<_>>();
    String::from_utf8(bytes).map_err(|_| {
        set_last_error(format!("{fn_name}: {label} is not UTF-8"));
        SidereonStatus::InvalidArgument
    })
}

pub(super) fn identity_from_c(
    fn_name: &str,
    identity: &SidereonProductIdentity,
) -> Result<ProductIdentity, SidereonStatus> {
    let issue = fixed_text_from_c(fn_name, "identity.issue", &identity.issue)?;
    let has_issue = bool_from_c(fn_name, "identity.has_issue", identity.has_issue)?;
    let has_format_version = bool_from_c(
        fn_name,
        "identity.has_format_version",
        identity.has_format_version,
    )?;
    let has_prediction_horizon_days = bool_from_c(
        fn_name,
        "identity.has_prediction_horizon_days",
        identity.has_prediction_horizon_days,
    )?;
    let product = ProductIdentity {
        family: family_from_c(fn_name, "identity.family", identity.family)?,
        analysis_center: fixed_text_from_c(
            fn_name,
            "identity.analysis_center",
            &identity.analysis_center,
        )?
        .parse()
        .map_err(|error| map_error(fn_name, error))?,
        publisher: publisher_from_c(fn_name, "identity.publisher", identity.publisher)?,
        solution: solution_from_c(fn_name, "identity.solution_class", identity.solution_class)?,
        campaign: campaign_from_c(fn_name, "identity.campaign", identity.campaign)?,
        version: identity.filename_version,
        date: ProductDate::new(identity.year, identity.month, identity.day)
            .map_err(|error| map_error(fn_name, error))?,
        issue: if has_issue { Some(issue) } else { None },
        span: fixed_text_from_c(fn_name, "identity.span", &identity.span)?,
        sample: fixed_text_from_c(fn_name, "identity.sample", &identity.sample)?,
        official_filename: fixed_text_from_c(
            fn_name,
            "identity.official_filename",
            &identity.official_filename,
        )?,
        format: format_from_c(fn_name, "identity.format", identity.format)?,
        format_version: if has_format_version {
            Some(fixed_text_from_c(
                fn_name,
                "identity.format_version",
                &identity.format_version,
            )?)
        } else {
            None
        },
        prediction_horizon_days: has_prediction_horizon_days
            .then_some(identity.prediction_horizon_days),
    };
    product
        .validate()
        .map_err(|error| map_error(fn_name, error))?;
    Ok(product)
}

#[derive(Clone, Copy)]
struct ProductInputs {
    center: *const c_char,
    family: u32,
    year: i32,
    month: u8,
    day: u8,
    sample: *const c_char,
    issue: *const c_char,
}

unsafe fn center_from_c(
    fn_name: &str,
    label: &str,
    center: *const c_char,
) -> Result<AnalysisCenter, SidereonStatus> {
    let center = parse_bounded_c_string(fn_name, label, center, ANALYSIS_CENTER_C_BYTES)?;
    AnalysisCenter::from_code(&center).ok_or_else(|| {
        set_last_error(format!("{fn_name}: unknown analysis center {center:?}"));
        SidereonStatus::InvalidArgument
    })
}

unsafe fn product_spec(
    fn_name: &str,
    input: ProductInputs,
) -> Result<core_data::ProductSpec, SidereonStatus> {
    let center = center_from_c(fn_name, "center", input.center)?;
    let date = ProductDate::new(input.year, input.month, input.day)
        .map_err(|error| map_error(fn_name, error))?;
    let sample = if input.sample.is_null() {
        None
    } else {
        Some(parse_bounded_c_string(fn_name, "sample", input.sample, 16)?)
    };
    let issue = if input.issue.is_null() {
        None
    } else {
        Some(parse_bounded_c_string(fn_name, "issue", input.issue, 16)?)
    };
    let family = family_from_c(fn_name, "family", input.family)?;
    core_data::product(center, family, date, sample.as_deref(), issue.as_deref())
        .map_err(|error| map_error(fn_name, error))
}

/// Return the next catalog issue nominally due at or after a UTC instant as a
/// JSON object.
///
/// This is a pure catalog query and performs no archive access. The object
/// contains the exact product `identity`, its UTC `due_at`, and half-open
/// `covers.observed` / `covers.predicted` UTC intervals (each nullable).
/// `family` is one SidereonProductFamily_* value encoded as uint32_t.
///
/// Uses the standard variable-length byte-output contract; JSON bytes are not
/// null-terminated.
///
/// Safety: `center` must reference a null-terminated UTF-8 string; `out` must
/// reference `out_len` writable bytes, or be NULL when `out_len` is zero; both
/// count pointers must reference writable size_t values.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn sidereon_data_next_issue_due_json(
    center: *const c_char,
    family: u32,
    year: i32,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_data_next_issue_due_json";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        if let Err(status) = init_copy_counts(FN_NAME, out_written, out_required) {
            return status;
        }
        let center = match center_from_c(FN_NAME, "center", center) {
            Ok(center) => center,
            Err(status) => return status,
        };
        let family = match family_from_c(FN_NAME, "family", family) {
            Ok(family) => family,
            Err(status) => return status,
        };
        let date = match ProductDate::new(year, month, day) {
            Ok(date) => date,
            Err(error) => return map_error(FN_NAME, error),
        };
        let now = match ProductDateTime::new(date, hour, minute, second) {
            Ok(now) => now,
            Err(error) => return map_error(FN_NAME, error),
        };
        let issue = match core_data::next_issue_due(center, family, now) {
            Ok(issue) => issue,
            Err(error) => return map_error(FN_NAME, error),
        };
        let value = serde_json::json!({
            "identity": product_identity_json(&issue.identity),
            "due_at": issue.due_at.to_string(),
            "covers": {
                "observed": nominal_coverage_interval_json(issue.covers.observed),
                "predicted": nominal_coverage_interval_json(issue.covers.predicted),
            },
        });
        let json = match serde_json::to_vec(&value) {
            Ok(json) => json,
            Err(error) => return map_error(FN_NAME, error),
        };
        match copy_prefix_to_c(
            FN_NAME,
            "out",
            &json,
            out,
            out_len,
            out_written,
            out_required,
        ) {
            Ok(()) => SidereonStatus::Ok,
            Err(status) => status,
        }
    })
}

/// Resolve the solution class for one supported center/product family.
///
/// Unlike the legacy center-wide classification, this reports IGS combined
/// final SP3 as `SIDEREON_SOLUTION_CLASS_FINAL` while preserving
/// `SIDEREON_SOLUTION_CLASS_BROADCAST` for IGS broadcast navigation.
/// Unsupported center/product combinations fail before acquisition.
///
/// `family` is one SidereonProductFamily_* value encoded as uint32_t.
///
/// Safety: `center` must reference a null-terminated UTF-8 string and
/// `out_solution_class` must reference writable storage.
/// Copy the bounded archive listing URLs answering "newest published issue"
/// for one center + product family, as a JSON array of strings.
///
/// At most two URLs, newest directory first (or one whole-tree listing);
/// never a polling loop. The output uses the standard variable-length byte
/// contract and is not null-terminated.
///
/// Safety: `center` must reference a null-terminated UTF-8 string; `out` must
/// reference `out_len` writable bytes, or may be NULL when `out_len` is zero;
/// the count pointers must reference writable size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_data_publication_listing_urls_json(
    center: *const c_char,
    family: u32,
    year: i32,
    month: u8,
    day: u8,
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_data_publication_listing_urls_json";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        if let Err(status) = init_copy_counts(FN_NAME, out_written, out_required) {
            return status;
        }
        let center = match center_from_c(FN_NAME, "center", center) {
            Ok(center) => center,
            Err(status) => return status,
        };
        let family = match family_from_c(FN_NAME, "family", family) {
            Ok(family) => family,
            Err(status) => return status,
        };
        let date = match ProductDate::new(year, month, day) {
            Ok(date) => date,
            Err(error) => return map_error(FN_NAME, error),
        };
        let urls = match core_data::publication_listing_urls(center, family, date) {
            Ok(urls) => urls,
            Err(error) => return map_error(FN_NAME, error),
        };
        let json = match serde_json::to_string(&urls) {
            Ok(json) => json,
            Err(error) => return map_error(FN_NAME, error),
        };
        match copy_prefix_to_c(
            FN_NAME,
            "out",
            json.as_bytes(),
            out,
            out_len,
            out_written,
            out_required,
        ) {
            Ok(()) => SidereonStatus::Ok,
            Err(status) => status,
        }
    })
}

/// Parse an archive listing body and copy the newest published issue of one
/// center + product family as a JSON object, or JSON `null` when the listing
/// is readable but holds no object of the line.
///
/// Listing dialect detection is closed: a body that fits none of the
/// recognized listing surfaces is an error status, never an empty result -
/// a silent empty parse would be indistinguishable from "nothing
/// published". The JSON object carries `date` (`YYYY-MM-DD`), `issue`
/// (`HHMM`), `filename`, and `observed_at` (the archive-reported
/// modification text, verbatim, or `null`).
///
/// Safety: `center` and `listing_body` must reference null-terminated UTF-8
/// strings; `out` must reference `out_len` writable bytes, or may be NULL
/// when `out_len` is zero; the count pointers must reference writable size_t
/// values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_data_newest_published_product_json(
    center: *const c_char,
    family: u32,
    listing_body: *const c_char,
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_data_newest_published_product_json";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        if let Err(status) = init_copy_counts(FN_NAME, out_written, out_required) {
            return status;
        }
        let center = match center_from_c(FN_NAME, "center", center) {
            Ok(center) => center,
            Err(status) => return status,
        };
        let family = match family_from_c(FN_NAME, "family", family) {
            Ok(family) => family,
            Err(status) => return status,
        };
        // Listing bodies are bounded by the largest recorded surface (AIUB's
        // whole-tree CSV, ~41 MiB); 64 MiB matches the scoreboard's cap.
        let body =
            match parse_bounded_c_string(FN_NAME, "listing_body", listing_body, 64 * 1024 * 1024) {
                Ok(body) => body,
                Err(status) => return status,
            };
        let objects = match core_data::parse_archive_listing(&body) {
            Ok(objects) => objects,
            Err(error) => return map_error(FN_NAME, error),
        };
        let newest = match core_data::newest_published_product(center, family, &objects) {
            Ok(newest) => newest,
            Err(error) => return map_error(FN_NAME, error),
        };
        let json = match newest {
            None => "null".to_string(),
            Some(product) => {
                let value = serde_json::json!({
                    "date": format!(
                        "{:04}-{:02}-{:02}",
                        product.date.year, product.date.month, product.date.day
                    ),
                    "issue": product.issue,
                    "filename": product.filename,
                    "observed_at": product.observed_at,
                });
                match serde_json::to_string(&value) {
                    Ok(json) => json,
                    Err(error) => return map_error(FN_NAME, error),
                }
            }
        };
        match copy_prefix_to_c(
            FN_NAME,
            "out",
            json.as_bytes(),
            out,
            out_len,
            out_written,
            out_required,
        ) {
            Ok(()) => SidereonStatus::Ok,
            Err(status) => status,
        }
    })
}

/// Copy the ordered cross-line candidates for one predicted IONEX map date
/// as a JSON array.
///
/// Both CODE predicted lines publish the same official filename for a map
/// date, but the two-day line is produced a day earlier, so `cod_prd2` is
/// routinely published while `cod_prd1` is still absent when CODE runs
/// behind. Candidates are ordered `cod_prd1` first, all cover the SAME map
/// date (never a neighboring day's map), and each keeps its own line
/// identity so resolved provenance names the line actually served. Each
/// element carries `center`, `date`, `sample`, `issue`, `filename`, and
/// `url`. The walk is opt-in; single-line requests keep their fail-closed
/// behavior.
///
/// Safety: a non-NULL `sample` must reference a null-terminated UTF-8
/// string; `out` must reference `out_len` writable bytes, or may be NULL
/// when `out_len` is zero; the count pointers must reference writable size_t
/// values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_data_predicted_ionex_line_candidates_json(
    year: i32,
    month: u8,
    day: u8,
    sample: *const c_char,
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_data_predicted_ionex_line_candidates_json";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        if let Err(status) = init_copy_counts(FN_NAME, out_written, out_required) {
            return status;
        }
        let sample = if sample.is_null() {
            None
        } else {
            match parse_bounded_c_string(FN_NAME, "sample", sample, PRODUCT_TOKEN_C_BYTES) {
                Ok(sample) => Some(sample),
                Err(status) => return status,
            }
        };
        let date = match ProductDate::new(year, month, day) {
            Ok(date) => date,
            Err(error) => return map_error(FN_NAME, error),
        };
        let candidates = match core_data::predicted_ionex_line_candidates(date, sample.as_deref()) {
            Ok(candidates) => candidates,
            Err(error) => return map_error(FN_NAME, error),
        };
        let mut rows = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            let filename = match candidate.canonical_filename() {
                Ok(filename) => filename,
                Err(error) => return map_error(FN_NAME, error),
            };
            let url = match candidate.archive_url() {
                Ok(url) => url,
                Err(error) => return map_error(FN_NAME, error),
            };
            rows.push(serde_json::json!({
                "center": candidate.center.code(),
                "date": format!(
                    "{:04}-{:02}-{:02}",
                    candidate.date.year, candidate.date.month, candidate.date.day
                ),
                "sample": candidate.sample,
                "issue": candidate.issue,
                "filename": filename,
                "url": url,
            }));
        }
        let json = match serde_json::to_string(&rows) {
            Ok(json) => json,
            Err(error) => return map_error(FN_NAME, error),
        };
        match copy_prefix_to_c(
            FN_NAME,
            "out",
            json.as_bytes(),
            out,
            out_len,
            out_written,
            out_required,
        ) {
            Ok(()) => SidereonStatus::Ok,
            Err(status) => status,
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_data_product_solution_class(
    center: *const c_char,
    family: u32,
    out_solution_class: *mut SidereonSolutionClass,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_data_product_solution_class";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = match require_out(out_solution_class, FN_NAME, "out_solution_class") {
            Ok(out) => out,
            Err(status) => return status,
        };
        *out = SidereonSolutionClass::Final;
        let center = match center_from_c(FN_NAME, "center", center) {
            Ok(center) => center,
            Err(status) => return status,
        };
        let family = match family_from_c(FN_NAME, "family", family) {
            Ok(family) => family,
            Err(status) => return status,
        };
        match core_data::product_solution_class(center, family) {
            Ok(solution) => {
                *out = solution_from_core(solution);
                SidereonStatus::Ok
            }
            Err(error) => map_error(FN_NAME, error),
        }
    })
}

/// Copy the published default sampling token for a center/product/date.
///
/// This is the date-aware catalog query. It preserves historical cadence
/// transitions such as GFZ rapid SP3 changing from `15M` to `05M` on
/// 2021-05-18 and GFZ ultra-rapid changing on 2021-05-16. On ESA ultra-rapid's
/// issue-level transition date, this query reports the `0000`/start-of-day
/// default; product identity derivation also considers the requested issue.
/// The output uses the standard variable-length byte contract and is not
/// null-terminated.
///
/// `family` is one SidereonProductFamily_* value encoded as uint32_t.
///
/// Safety: `center` must reference a null-terminated UTF-8 string; `out` must
/// reference `out_len` writable bytes, or may be NULL when `out_len` is zero;
/// the count pointers must reference writable size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_data_default_sample_for_date(
    center: *const c_char,
    family: u32,
    year: i32,
    month: u8,
    day: u8,
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_data_default_sample_for_date";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        if let Err(status) = init_copy_counts(FN_NAME, out_written, out_required) {
            return status;
        }
        let center = match center_from_c(FN_NAME, "center", center) {
            Ok(center) => center,
            Err(status) => return status,
        };
        let family = match family_from_c(FN_NAME, "family", family) {
            Ok(family) => family,
            Err(status) => return status,
        };
        let date = match ProductDate::new(year, month, day) {
            Ok(date) => date,
            Err(error) => return map_error(FN_NAME, error),
        };
        let sample = match core_data::default_sample_for_date(center, family, date) {
            Ok(sample) => sample,
            Err(error) => return map_error(FN_NAME, error),
        };
        match copy_prefix_to_c(
            FN_NAME,
            "out",
            sample.as_bytes(),
            out,
            out_len,
            out_written,
            out_required,
        ) {
            Ok(()) => SidereonStatus::Ok,
            Err(status) => status,
        }
    })
}

/// Copy every officially cataloged sampling token for a product date and issue.
///
/// This is the complete date- and issue-aware catalog query used by product
/// constructors. `out_len`, `out_written`, and `out_required` count
/// `SidereonProductSample` records, not bytes. Pass `(NULL, 0)` to obtain the
/// exact required count before allocating; no caller-selected token width is
/// involved.
///
/// For issue-based product lines, a NULL `issue` selects `0000`, matching
/// `sidereon_data_default_sample_for_date`. Product construction still requires
/// an explicit issue. Product lines without issues reject a non-NULL issue.
///
/// `family` is one SidereonProductFamily_* value encoded as uint32_t.
///
/// Safety: `center` must reference a null-terminated UTF-8 string; a non-NULL
/// `issue` must do the same; `out` must reference `out_len` writable records,
/// or may be NULL when `out_len` is zero; both count pointers must reference
/// writable size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_data_supported_samples(
    center: *const c_char,
    family: u32,
    year: i32,
    month: u8,
    day: u8,
    issue: *const c_char,
    out: *mut SidereonProductSample,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_data_supported_samples";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        if let Err(status) = init_copy_counts(FN_NAME, out_written, out_required) {
            return status;
        }
        let center = match center_from_c(FN_NAME, "center", center) {
            Ok(center) => center,
            Err(status) => return status,
        };
        let family = match family_from_c(FN_NAME, "family", family) {
            Ok(family) => family,
            Err(status) => return status,
        };
        let date = match ProductDate::new(year, month, day) {
            Ok(date) => date,
            Err(error) => return map_error(FN_NAME, error),
        };
        let issue = if issue.is_null() {
            None
        } else {
            match parse_bounded_c_string(FN_NAME, "issue", issue, PRODUCT_TOKEN_C_BYTES) {
                Ok(issue) => Some(issue),
                Err(status) => return status,
            }
        };
        let samples = match core_data::supported_samples(center, family, date, issue.as_deref()) {
            Ok(samples) => samples,
            Err(error) => return map_error(FN_NAME, error),
        };
        let samples = match samples
            .iter()
            .map(|sample| {
                fixed_text(FN_NAME, "sample", sample).map(|token| SidereonProductSample { token })
            })
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(samples) => samples,
            Err(status) => return status,
        };
        match copy_prefix_to_c(
            FN_NAME,
            "out",
            &samples,
            out,
            out_len,
            out_written,
            out_required,
        ) {
            Ok(()) => SidereonStatus::Ok,
            Err(status) => status,
        }
    })
}

/// Return the cataloged relationship between an SP3 filename epoch and its
/// first content epoch.
///
/// `issue` follows the product catalog rules: it is required for ultra-rapid
/// centers, must name a published issue, and must be NULL for product lines
/// without issue times. The signed offset is added to the filename epoch to
/// obtain the required first content epoch. Both outputs describe the same
/// catalog result.
///
/// Safety: `center` must reference a null-terminated UTF-8 string; a non-NULL
/// `issue` must do the same; both output pointers must reference writable
/// storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_data_sp3_content_start_convention(
    center: *const c_char,
    year: i32,
    month: u8,
    day: u8,
    issue: *const c_char,
    out_convention: *mut SidereonSp3ContentStartConvention,
    out_content_start_offset_s: *mut i64,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_data_sp3_content_start_convention";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_convention = match require_out(out_convention, FN_NAME, "out_convention") {
            Ok(out) => out,
            Err(status) => return status,
        };
        let out_offset = match require_out(
            out_content_start_offset_s,
            FN_NAME,
            "out_content_start_offset_s",
        ) {
            Ok(out) => out,
            Err(status) => return status,
        };
        *out_convention = SidereonSp3ContentStartConvention::FilenameEpoch;
        *out_offset = 0;

        let center = match center_from_c(FN_NAME, "center", center) {
            Ok(center) => center,
            Err(status) => return status,
        };
        let date = match ProductDate::new(year, month, day) {
            Ok(date) => date,
            Err(error) => return map_error(FN_NAME, error),
        };
        let issue = if issue.is_null() {
            None
        } else {
            match parse_bounded_c_string(FN_NAME, "issue", issue, PRODUCT_TOKEN_C_BYTES) {
                Ok(issue) => Some(issue),
                Err(status) => return status,
            }
        };

        match core_data::sp3_content_start_convention(center, date, issue.as_deref()) {
            Ok(convention) => {
                let Some(c_convention) = sp3_content_start_from_core(convention) else {
                    set_last_error(format!(
                        "{FN_NAME}: the core returned a content-start convention not exposed by this C ABI"
                    ));
                    return SidereonStatus::InvalidArgument;
                };
                *out_convention = c_convention;
                *out_offset = convention.content_start_offset_s();
                SidereonStatus::Ok
            }
            Err(error) => map_error(FN_NAME, error),
        }
    })
}

/// Resolve an exact catalog product identity independently from distributor.
///
/// `family` is one of SidereonProductFamily_* encoded as uint32_t. Invalid
/// values fail closed with SIDEREON_STATUS_INVALID_ARGUMENT.
///
/// `sample` may be NULL to use the catalog default, including issue-aware
/// ultra-rapid transitions. `issue` may be NULL only for product lines that do
/// not require an ultra-rapid issue.
///
/// Safety: non-null text pointers must reference null-terminated UTF-8 strings;
/// `out_identity` must reference writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_data_product_identity(
    center: *const c_char,
    family: u32,
    year: i32,
    month: u8,
    day: u8,
    sample: *const c_char,
    issue: *const c_char,
    out_identity: *mut SidereonProductIdentity,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_data_product_identity";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = match require_out(out_identity, FN_NAME, "out_identity") {
            Ok(out) => out,
            Err(status) => return status,
        };
        let product = match product_spec(
            FN_NAME,
            ProductInputs {
                center,
                family,
                year,
                month,
                day,
                sample,
                issue,
            },
        ) {
            Ok(product) => product,
            Err(status) => return status,
        };
        let identity = match product.identity() {
            Ok(identity) => identity,
            Err(error) => return map_error(FN_NAME, error),
        };
        match identity_to_c(FN_NAME, &identity) {
            Ok(identity) => {
                *out = identity;
                SidereonStatus::Ok
            }
            Err(status) => status,
        }
    })
}

/// Copy the stable cache key derived from every exact identity field.
///
/// Uses the standard variable-length output contract; output is not
/// null-terminated.
///
/// Safety: `identity` and count pointers must be live; `out` must have
/// `out_len` writable bytes or be NULL when `out_len` is zero.
#[no_mangle]
pub unsafe extern "C" fn sidereon_data_product_identity_cache_key(
    identity: *const SidereonProductIdentity,
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_data_product_identity_cache_key";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        if let Err(status) = init_copy_counts(FN_NAME, out_written, out_required) {
            return status;
        }
        let identity = match require_ref(identity, FN_NAME, "identity")
            .and_then(|identity| identity_from_c(FN_NAME, identity))
        {
            Ok(identity) => identity,
            Err(status) => return status,
        };
        let key = match identity.key() {
            Ok(key) => key,
            Err(error) => return map_error(FN_NAME, error),
        };
        match copy_prefix_to_c(
            FN_NAME,
            "out",
            key.as_bytes(),
            out,
            out_len,
            out_written,
            out_required,
        ) {
            Ok(()) => SidereonStatus::Ok,
            Err(status) => status,
        }
    })
}

/// Require available identities to be exactly the declared product set.
///
/// The expected set must be non-empty. Both inputs reject duplicates; missing
/// and undeclared identities fail. Comparison includes every identity field,
/// not only the official filename. For SP3 observed/predicted timing, use
/// `sidereon_sp3_prediction_summary`; catalog fields and issue times are not
/// substitutes for product record flags.
///
/// Safety: each pointer must reference `count` readable identities, or may be
/// NULL only when its count is zero.
#[no_mangle]
pub unsafe extern "C" fn sidereon_data_validate_exact_product_set(
    expected: *const SidereonProductIdentity,
    expected_count: usize,
    available: *const SidereonProductIdentity,
    available_count: usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_data_validate_exact_product_set";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let expected = match require_slice(expected, expected_count, FN_NAME, "expected") {
            Ok(values) => values,
            Err(status) => return status,
        };
        let available = match require_slice(available, available_count, FN_NAME, "available") {
            Ok(values) => values,
            Err(status) => return status,
        };
        let expected = match expected
            .iter()
            .map(|identity| identity_from_c(FN_NAME, identity))
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(values) => values,
            Err(status) => return status,
        };
        let available = match available
            .iter()
            .map(|identity| identity_from_c(FN_NAME, identity))
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(values) => values,
            Err(status) => return status,
        };
        match core_data::validate_exact_product_set(&expected, &available) {
            Ok(()) => SidereonStatus::Ok,
            Err(error) => map_error(FN_NAME, error),
        }
    })
}

/// Resolve one explicit distributor for an exact catalog product.
///
/// This function performs no network or file IO. `original_url` is absent for
/// local-file and in-memory sources. `family` and `source` are the corresponding
/// SidereonProductFamily_* and SidereonDistributionSource_* values encoded as
/// uint32_t; invalid values fail closed.
///
/// Safety: non-null text pointers must reference null-terminated UTF-8 strings;
/// `out_location` must reference writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_data_distribution_location(
    center: *const c_char,
    family: u32,
    year: i32,
    month: u8,
    day: u8,
    sample: *const c_char,
    issue: *const c_char,
    source: u32,
    out_location: *mut SidereonDistributionLocation,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_data_distribution_location";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = match require_out(out_location, FN_NAME, "out_location") {
            Ok(out) => out,
            Err(status) => return status,
        };
        let product = match product_spec(
            FN_NAME,
            ProductInputs {
                center,
                family,
                year,
                month,
                day,
                sample,
                issue,
            },
        ) {
            Ok(product) => product,
            Err(status) => return status,
        };
        let source = match source_from_c(FN_NAME, "source", source) {
            Ok(source) => source,
            Err(status) => return status,
        };
        let location = match product.distribution_location(source) {
            Ok(location) => location,
            Err(error) => return map_error(FN_NAME, error),
        };
        let original_url = location.original_url.as_deref().unwrap_or("");
        let converted = (|| {
            Ok::<_, SidereonStatus>(SidereonDistributionLocation {
                source: match source {
                    DistributionSource::Direct => SidereonDistributionSource::Direct,
                    DistributionSource::NasaCddis => SidereonDistributionSource::NasaCddis,
                    DistributionSource::LocalFile => SidereonDistributionSource::LocalFile,
                    DistributionSource::InMemory => SidereonDistributionSource::InMemory,
                },
                has_original_url: location.original_url.is_some(),
                original_url: fixed_text(FN_NAME, "original_url", original_url)?,
                archive_filename: fixed_text(
                    FN_NAME,
                    "archive_filename",
                    &location.archive_filename,
                )?,
                compression: compression_from_core(location.compression),
            })
        })();
        match converted {
            Ok(location) => {
                *out = location;
                SidereonStatus::Ok
            }
            Err(status) => status,
        }
    })
}

/// Initialize exact-cache single-flight options with the engine defaults.
///
/// The defaults are a 50 ms poll interval, 5 s heartbeat interval, 30 s
/// liveness timeout, and 30 minute total wait timeout. This also initializes
/// the required `struct_size` and `abi_version` guards.
///
/// Safety: `out_options` must point to writable storage for one
/// SidereonExactCacheSingleFlightOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_exact_cache_single_flight_options_init(
    out_options: *mut SidereonExactCacheSingleFlightOptions,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_exact_cache_single_flight_options_init";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = match require_out(out_options, FN_NAME, "out_options") {
            Ok(out) => out,
            Err(status) => return status,
        };
        *out = default_single_flight_options();
        SidereonStatus::Ok
    })
}

/// Open one exact identity/source cache with bounded single-flight coordination.
///
/// A successful call writes either Hit and one owned `out_entry`, or Owner and
/// one owned `out_owner`; the other handle output remains NULL. Only an owner
/// should fetch and validate bytes. Acquisition waiting, owner liveness, and
/// every filesystem transition are performed by the Rust engine.
///
/// `source` is one SidereonDistributionSource_* value encoded as uint32_t.
/// `options` may be NULL for engine defaults. A non-NULL options struct must
/// have been initialized by `sidereon_exact_cache_single_flight_options_init`.
/// A live owner that does not publish before `wait_timeout_ms` produces
/// SIDEREON_STATUS_TIMEOUT and does not grant ownership to this caller.
///
/// Safety: `stable_path` and `identity` must be readable; all three output
/// pointers must reference writable storage.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn sidereon_exact_cache_open_single_flight(
    stable_path: *const c_char,
    identity: *const SidereonProductIdentity,
    source: u32,
    options: *const SidereonExactCacheSingleFlightOptions,
    out_result: *mut SidereonExactCacheOpenResult,
    out_entry: *mut *mut SidereonExactCacheEntry,
    out_owner: *mut *mut SidereonExactCacheOwner,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_exact_cache_open_single_flight";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        if !out_result.is_null() {
            *out_result = SidereonExactCacheOpenResult::Hit;
        }
        if !out_entry.is_null() {
            *out_entry = ptr::null_mut();
        }
        if !out_owner.is_null() {
            *out_owner = ptr::null_mut();
        }
        let result = match require_out(out_result, FN_NAME, "out_result") {
            Ok(result) => result,
            Err(status) => return status,
        };
        let entry_out = match require_out(out_entry, FN_NAME, "out_entry") {
            Ok(out) => out,
            Err(status) => return status,
        };
        let owner_out = match require_out(out_owner, FN_NAME, "out_owner") {
            Ok(out) => out,
            Err(status) => return status,
        };
        let stable_path = match parse_bounded_c_string(FN_NAME, "stable_path", stable_path, 4096) {
            Ok(path) => path,
            Err(status) => return status,
        };
        let identity = match require_ref(identity, FN_NAME, "identity")
            .and_then(|identity| identity_from_c(FN_NAME, identity))
        {
            Ok(identity) => identity,
            Err(status) => return status,
        };
        let source = match source_from_c(FN_NAME, "source", source) {
            Ok(source) => source,
            Err(status) => return status,
        };
        let options = match single_flight_options_from_c(FN_NAME, options) {
            Ok(options) => options,
            Err(status) => return status,
        };
        let cache = match ExactProductCache::new(stable_path, identity, source) {
            Ok(cache) => cache,
            Err(error) => return map_cache_error(FN_NAME, error),
        };
        match cache.open_single_flight(options) {
            Ok(ExactCacheOpen::Hit(entry)) => {
                *result = SidereonExactCacheOpenResult::Hit;
                write_boxed_handle(entry_out, SidereonExactCacheEntry { entry });
                SidereonStatus::Ok
            }
            Ok(ExactCacheOpen::Owner(owner)) => {
                *result = SidereonExactCacheOpenResult::Owner;
                write_boxed_handle(owner_out, SidereonExactCacheOwner { owner: Some(owner) });
                SidereonStatus::Ok
            }
            Err(error) => map_cache_error(FN_NAME, error),
        }
    })
}

/// Refresh one single-flight owner's liveness heartbeat immediately.
///
/// Safety: `owner` must be a live, unpublished owner handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_exact_cache_owner_heartbeat(
    owner: *const SidereonExactCacheOwner,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_exact_cache_owner_heartbeat";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let owner = match require_ref(owner, FN_NAME, "owner") {
            Ok(owner) => owner,
            Err(status) => return status,
        };
        let Some(owner) = owner.owner.as_ref() else {
            set_last_error(format!("{FN_NAME}: owner is closed"));
            return SidereonStatus::InvalidArgument;
        };
        match owner.heartbeat() {
            Ok(()) => SidereonStatus::Ok,
            Err(error) => map_cache_error(FN_NAME, error),
        }
    })
}

/// Publish validated bytes and close one single-flight owner.
///
/// Product semantics must be validated before this call. Once the core publish
/// attempt begins the owner is closed, including when the core reports an
/// error. The caller must still release the owner allocation with
/// `sidereon_exact_cache_owner_free` and owns the returned entry on success.
/// C argument-validation failures leave the owner open for a corrected call.
///
/// Safety: each byte pointer must reference its declared length; `owner` must
/// be a live handle; `out_entry` must be writable storage for one handle
/// pointer.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn sidereon_exact_cache_owner_publish(
    owner: *mut SidereonExactCacheOwner,
    product: *const u8,
    product_len: usize,
    archive: *const u8,
    archive_len: usize,
    provenance: *const u8,
    provenance_len: usize,
    out_entry: *mut *mut SidereonExactCacheEntry,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_exact_cache_owner_publish";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = match require_out(out_entry, FN_NAME, "out_entry") {
            Ok(out) => out,
            Err(status) => return status,
        };
        *out = ptr::null_mut();
        let product = match require_slice(product, product_len, FN_NAME, "product") {
            Ok(bytes) => bytes,
            Err(status) => return status,
        };
        let archive = match require_slice(archive, archive_len, FN_NAME, "archive") {
            Ok(bytes) => bytes,
            Err(status) => return status,
        };
        let provenance = match require_slice(provenance, provenance_len, FN_NAME, "provenance") {
            Ok(bytes) => bytes,
            Err(status) => return status,
        };
        let owner = match require_mut(owner, FN_NAME, "owner") {
            Ok(owner) => owner,
            Err(status) => return status,
        };
        let Some(owner) = owner.owner.take() else {
            set_last_error(format!("{FN_NAME}: owner is closed"));
            return SidereonStatus::InvalidArgument;
        };
        match owner.publish(product, archive, provenance) {
            Ok(entry) => {
                write_boxed_handle(out, SidereonExactCacheEntry { entry });
                SidereonStatus::Ok
            }
            Err(error) => map_cache_error(FN_NAME, error),
        }
    })
}

/// Release a single-flight owner handle. NULL is a no-op.
///
/// Releasing an unpublished owner abandons acquisition and best-effort removes
/// its in-flight marker.
#[no_mangle]
pub unsafe extern "C" fn sidereon_exact_cache_owner_free(owner: *mut SidereonExactCacheOwner) {
    free_boxed(owner);
}

/// Open one exact identity/source cache and acquire its bounded cross-process lock.
///
/// `source` is one SidereonDistributionSource_* value encoded as uint32_t.
///
/// `stable_path` names the official product below its identity/source cache
/// directory. The returned handle owns the lock until
/// `sidereon_exact_cache_free` is called.
///
/// Safety: `stable_path` and `identity` must be readable; `out_cache` must be
/// writable storage for one handle pointer.
#[no_mangle]
pub unsafe extern "C" fn sidereon_exact_cache_open(
    stable_path: *const c_char,
    identity: *const SidereonProductIdentity,
    source: u32,
    timeout_ms: u64,
    out_cache: *mut *mut SidereonExactCache,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_exact_cache_open";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = match require_out(out_cache, FN_NAME, "out_cache") {
            Ok(out) => out,
            Err(status) => return status,
        };
        *out = ptr::null_mut();
        let stable_path = match parse_bounded_c_string(FN_NAME, "stable_path", stable_path, 4096) {
            Ok(path) => path,
            Err(status) => return status,
        };
        let identity = match require_ref(identity, FN_NAME, "identity")
            .and_then(|identity| identity_from_c(FN_NAME, identity))
        {
            Ok(identity) => identity,
            Err(status) => return status,
        };
        let source = match source_from_c(FN_NAME, "source", source) {
            Ok(source) => source,
            Err(status) => return status,
        };
        let cache = match ExactProductCache::new(stable_path, identity, source) {
            Ok(cache) => cache,
            Err(error) => return map_cache_error(FN_NAME, error),
        };
        let guard = match cache.lock(Duration::from_millis(timeout_ms)) {
            Ok(guard) => guard,
            Err(error) => return map_cache_error(FN_NAME, error),
        };
        write_boxed_handle(
            out,
            SidereonExactCache {
                cache,
                guard: Some(guard),
            },
        );
        SidereonStatus::Ok
    })
}

/// Read the current digest-verified immutable cache entry.
///
/// A cache miss returns `SIDEREON_STATUS_OK`, writes false to `out_hit`, and
/// leaves `out_entry` NULL. Corruption, an incomplete entry, or an
/// identity/source mismatch is an error, never a miss.
///
/// Safety: all pointers must be live and writable as documented.
#[no_mangle]
pub unsafe extern "C" fn sidereon_exact_cache_read(
    cache: *const SidereonExactCache,
    out_hit: *mut bool,
    out_entry: *mut *mut SidereonExactCacheEntry,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_exact_cache_read";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let cache = match require_ref(cache, FN_NAME, "cache") {
            Ok(cache) => cache,
            Err(status) => return status,
        };
        if cache.guard.is_none() {
            set_last_error(format!("{FN_NAME}: cache lock is closed"));
            return SidereonStatus::InvalidArgument;
        }
        let hit = match require_out(out_hit, FN_NAME, "out_hit") {
            Ok(hit) => hit,
            Err(status) => return status,
        };
        let out = match require_out(out_entry, FN_NAME, "out_entry") {
            Ok(out) => out,
            Err(status) => return status,
        };
        *hit = false;
        *out = ptr::null_mut();
        match cache.cache.read() {
            Ok(None) => SidereonStatus::Ok,
            Ok(Some(entry)) => {
                *hit = true;
                write_boxed_handle(out, SidereonExactCacheEntry { entry });
                SidereonStatus::Ok
            }
            Err(error) => map_cache_error(FN_NAME, error),
        }
    })
}

/// Read the current digest-verified immutable cache entry without acquiring
/// the writer lock.
///
/// This is the read-only counterpart to `sidereon_exact_cache_open`. The
/// single atomic commit marker ensures a reader observes either the previous
/// complete entry or the newly committed complete entry while a cooperating
/// writer publishes. Miss and error behavior matches
/// `sidereon_exact_cache_read`.
/// `source` is one SidereonDistributionSource_* value encoded as uint32_t.
///
/// Safety: `stable_path` and `identity` must be readable; `out_hit` and
/// `out_entry` must be writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_exact_cache_read_unlocked(
    stable_path: *const c_char,
    identity: *const SidereonProductIdentity,
    source: u32,
    out_hit: *mut bool,
    out_entry: *mut *mut SidereonExactCacheEntry,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_exact_cache_read_unlocked";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let hit = match require_out(out_hit, FN_NAME, "out_hit") {
            Ok(hit) => hit,
            Err(status) => return status,
        };
        let out = match require_out(out_entry, FN_NAME, "out_entry") {
            Ok(out) => out,
            Err(status) => return status,
        };
        *hit = false;
        *out = ptr::null_mut();
        let stable_path = match parse_bounded_c_string(FN_NAME, "stable_path", stable_path, 4096) {
            Ok(path) => path,
            Err(status) => return status,
        };
        let identity = match require_ref(identity, FN_NAME, "identity")
            .and_then(|identity| identity_from_c(FN_NAME, identity))
        {
            Ok(identity) => identity,
            Err(status) => return status,
        };
        let source = match source_from_c(FN_NAME, "source", source) {
            Ok(source) => source,
            Err(status) => return status,
        };
        let cache = match ExactProductCache::new(stable_path, identity, source) {
            Ok(cache) => cache,
            Err(error) => return map_cache_error(FN_NAME, error),
        };
        match cache.read() {
            Ok(None) => SidereonStatus::Ok,
            Ok(Some(entry)) => {
                *hit = true;
                write_boxed_handle(out, SidereonExactCacheEntry { entry });
                SidereonStatus::Ok
            }
            Err(error) => map_cache_error(FN_NAME, error),
        }
    })
}

/// Publish validated product, distributor archive, and provenance bytes as one
/// immutable cache transaction.
///
/// Product semantics must be validated before this call. The shared cache
/// binds the full identity/source and all three byte digests in the commit.
///
/// Safety: each byte pointer must reference its declared length; `cache` must
/// be live; `out_entry` must be writable storage for a handle pointer.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn sidereon_exact_cache_publish(
    cache: *const SidereonExactCache,
    product: *const u8,
    product_len: usize,
    archive: *const u8,
    archive_len: usize,
    provenance: *const u8,
    provenance_len: usize,
    out_entry: *mut *mut SidereonExactCacheEntry,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_exact_cache_publish";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let cache = match require_ref(cache, FN_NAME, "cache") {
            Ok(cache) => cache,
            Err(status) => return status,
        };
        let Some(guard) = cache.guard.as_ref() else {
            set_last_error(format!("{FN_NAME}: cache lock is closed"));
            return SidereonStatus::InvalidArgument;
        };
        let product = match require_slice(product, product_len, FN_NAME, "product") {
            Ok(bytes) => bytes,
            Err(status) => return status,
        };
        let archive = match require_slice(archive, archive_len, FN_NAME, "archive") {
            Ok(bytes) => bytes,
            Err(status) => return status,
        };
        let provenance = match require_slice(provenance, provenance_len, FN_NAME, "provenance") {
            Ok(bytes) => bytes,
            Err(status) => return status,
        };
        let out = match require_out(out_entry, FN_NAME, "out_entry") {
            Ok(out) => out,
            Err(status) => return status,
        };
        *out = ptr::null_mut();
        match cache.cache.publish(guard, product, archive, provenance) {
            Ok(entry) => {
                write_boxed_handle(out, SidereonExactCacheEntry { entry });
                SidereonStatus::Ok
            }
            Err(error) => map_cache_error(FN_NAME, error),
        }
    })
}

/// Remove unreferenced transaction artifacts under the held cache lock.
///
/// Safety: `cache` must be a live handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_exact_cache_cleanup(
    cache: *const SidereonExactCache,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_exact_cache_cleanup";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let cache = match require_ref(cache, FN_NAME, "cache") {
            Ok(cache) => cache,
            Err(status) => return status,
        };
        let Some(guard) = cache.guard.as_ref() else {
            set_last_error(format!("{FN_NAME}: cache lock is closed"));
            return SidereonStatus::InvalidArgument;
        };
        match cache.cache.cleanup_abandoned(guard) {
            Ok(()) => SidereonStatus::Ok,
            Err(error) => map_cache_error(FN_NAME, error),
        }
    })
}

#[derive(Clone, Copy)]
enum ExactCacheComponent {
    Product,
    Archive,
    Provenance,
}

fn cache_component_from_c(
    fn_name: &str,
    value: u32,
) -> Result<ExactCacheComponent, SidereonStatus> {
    match value {
        value if value == SidereonExactCacheComponent::Product as u32 => {
            Ok(ExactCacheComponent::Product)
        }
        value if value == SidereonExactCacheComponent::Archive as u32 => {
            Ok(ExactCacheComponent::Archive)
        }
        value if value == SidereonExactCacheComponent::Provenance as u32 => {
            Ok(ExactCacheComponent::Provenance)
        }
        _ => Err(invalid_discriminant(fn_name, "component", value)),
    }
}

fn entry_component_bytes(entry: &SidereonExactCacheEntry, component: ExactCacheComponent) -> &[u8] {
    match component {
        ExactCacheComponent::Product => &entry.entry.product,
        ExactCacheComponent::Archive => &entry.entry.archive,
        ExactCacheComponent::Provenance => &entry.entry.provenance,
    }
}

fn entry_component_path(
    entry: &SidereonExactCacheEntry,
    component: ExactCacheComponent,
) -> Vec<u8> {
    let path = match component {
        ExactCacheComponent::Product => &entry.entry.product_path,
        ExactCacheComponent::Archive => &entry.entry.archive_path,
        ExactCacheComponent::Provenance => &entry.entry.provenance_path,
    };
    path.to_string_lossy().as_bytes().to_vec()
}

/// Copy one authenticated byte component from a verified cache entry.
///
/// `component` is one SidereonExactCacheComponent_* value encoded as uint32_t.
///
/// Uses the standard variable-length output contract; output is not
/// null-terminated.
///
/// Safety: `entry` and count pointers must be live; `out` must have `out_len`
/// writable bytes or be NULL when `out_len` is zero.
#[no_mangle]
pub unsafe extern "C" fn sidereon_exact_cache_entry_copy_bytes(
    entry: *const SidereonExactCacheEntry,
    component: u32,
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_exact_cache_entry_copy_bytes";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        if let Err(status) = init_copy_counts(FN_NAME, out_written, out_required) {
            return status;
        }
        let component = match cache_component_from_c(FN_NAME, component) {
            Ok(component) => component,
            Err(status) => return status,
        };
        let entry = match require_ref(entry, FN_NAME, "entry") {
            Ok(entry) => entry,
            Err(status) => return status,
        };
        match copy_prefix_to_c(
            FN_NAME,
            "out",
            entry_component_bytes(entry, component),
            out,
            out_len,
            out_written,
            out_required,
        ) {
            Ok(()) => SidereonStatus::Ok,
            Err(status) => status,
        }
    })
}

/// Copy one filesystem path from a verified cache entry as UTF-8 bytes.
///
/// Uses the standard variable-length output contract; output is not
/// null-terminated.
///
/// Safety: pointer requirements match `sidereon_exact_cache_entry_copy_bytes`.
#[no_mangle]
pub unsafe extern "C" fn sidereon_exact_cache_entry_copy_path(
    entry: *const SidereonExactCacheEntry,
    component: u32,
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_exact_cache_entry_copy_path";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        if let Err(status) = init_copy_counts(FN_NAME, out_written, out_required) {
            return status;
        }
        let component = match cache_component_from_c(FN_NAME, component) {
            Ok(component) => component,
            Err(status) => return status,
        };
        let entry = match require_ref(entry, FN_NAME, "entry") {
            Ok(entry) => entry,
            Err(status) => return status,
        };
        let path = entry_component_path(entry, component);
        match copy_prefix_to_c(
            FN_NAME,
            "out",
            &path,
            out,
            out_len,
            out_written,
            out_required,
        ) {
            Ok(()) => SidereonStatus::Ok,
            Err(status) => status,
        }
    })
}

/// Copy the immutable 32-character transaction identifier from a verified
/// cache entry.
///
/// Uses the standard variable-length output contract; output is not
/// null-terminated.
///
/// Safety: pointer requirements match `sidereon_exact_cache_entry_copy_bytes`.
#[no_mangle]
pub unsafe extern "C" fn sidereon_exact_cache_entry_copy_id(
    entry: *const SidereonExactCacheEntry,
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_exact_cache_entry_copy_id";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        if let Err(status) = init_copy_counts(FN_NAME, out_written, out_required) {
            return status;
        }
        let entry = match require_ref(entry, FN_NAME, "entry") {
            Ok(entry) => entry,
            Err(status) => return status,
        };
        match copy_prefix_to_c(
            FN_NAME,
            "out",
            entry.entry.entry_id.as_bytes(),
            out,
            out_len,
            out_written,
            out_required,
        ) {
            Ok(()) => SidereonStatus::Ok,
            Err(status) => status,
        }
    })
}

/// Release an exact-cache entry handle. NULL is a no-op.
#[no_mangle]
pub unsafe extern "C" fn sidereon_exact_cache_entry_free(entry: *mut SidereonExactCacheEntry) {
    free_boxed(entry);
}

/// Release an exact-cache handle and its cross-process lock. NULL is a no-op.
#[no_mangle]
pub unsafe extern "C" fn sidereon_exact_cache_free(cache: *mut SidereonExactCache) {
    free_boxed(cache);
}

#[cfg(test)]
mod tests {
    use std::ffi::{CStr, CString};
    use std::fs;
    use std::mem::MaybeUninit;
    use std::ptr;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn exact_cache_test_identity() -> SidereonProductIdentity {
        let center = CString::new("cod").unwrap();
        let mut identity = MaybeUninit::<SidereonProductIdentity>::uninit();
        assert_eq!(
            unsafe {
                sidereon_data_product_identity(
                    center.as_ptr(),
                    SidereonProductFamily::Sp3 as u32,
                    2026,
                    7,
                    12,
                    ptr::null(),
                    ptr::null(),
                    identity.as_mut_ptr(),
                )
            },
            SidereonStatus::Ok
        );
        unsafe { identity.assume_init() }
    }

    fn exact_cache_test_paths(label: &str) -> (std::path::PathBuf, CString) {
        let root = std::env::temp_dir().join(format!(
            "sidereon-c-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let stable = root.join("COD0MGXFIN_20261930000_01D_05M_ORB.SP3");
        let stable_c = CString::new(stable.to_string_lossy().as_bytes()).unwrap();
        (root, stable_c)
    }

    #[test]
    fn exact_identity_and_cddis_path_match_the_core() {
        let center = CString::new("cod").unwrap();
        let mut identity = MaybeUninit::<SidereonProductIdentity>::uninit();
        let status = unsafe {
            sidereon_data_product_identity(
                center.as_ptr(),
                SidereonProductFamily::Sp3 as u32,
                2026,
                7,
                12,
                ptr::null(),
                ptr::null(),
                identity.as_mut_ptr(),
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        let identity = unsafe { identity.assume_init() };
        assert_eq!(identity.publisher, SidereonProductPublisher::Code as u32);
        assert_eq!(identity.solution_class, SidereonSolutionClass::Final as u32);
        assert_eq!(
            unsafe { CStr::from_ptr(identity.analysis_center.as_ptr()) }
                .to_str()
                .unwrap(),
            "cod"
        );
        assert_eq!(
            identity.campaign,
            SidereonProductCampaign::MultiGnssExperiment as u32
        );
        assert_eq!(
            unsafe { CStr::from_ptr(identity.official_filename.as_ptr()) }
                .to_str()
                .unwrap(),
            "COD0MGXFIN_20261930000_01D_05M_ORB.SP3"
        );
        let mut key_required = 0;
        let mut key_written = 0;
        assert_eq!(
            unsafe {
                sidereon_data_product_identity_cache_key(
                    &identity,
                    ptr::null_mut(),
                    0,
                    &mut key_written,
                    &mut key_required,
                )
            },
            SidereonStatus::Ok
        );
        let mut key = vec![0; key_required];
        assert_eq!(
            unsafe {
                sidereon_data_product_identity_cache_key(
                    &identity,
                    key.as_mut_ptr(),
                    key.len(),
                    &mut key_written,
                    &mut key_required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(&key[..key_written], b"cod-final-a91258c21fa4860c34ce");

        let mut location = MaybeUninit::<SidereonDistributionLocation>::uninit();
        let status = unsafe {
            sidereon_data_distribution_location(
                center.as_ptr(),
                SidereonProductFamily::Sp3 as u32,
                2026,
                7,
                12,
                ptr::null(),
                ptr::null(),
                SidereonDistributionSource::NasaCddis as u32,
                location.as_mut_ptr(),
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        let location = unsafe { location.assume_init() };
        assert_eq!(location.compression, SidereonArchiveCompression::Gzip);
        assert_eq!(
            unsafe { CStr::from_ptr(location.original_url.as_ptr()) }
                .to_str()
                .unwrap(),
            "https://cddis.nasa.gov/archive/gnss/products/2427/\
COD0MGXFIN_20261930000_01D_05M_ORB.SP3.gz"
        );
    }

    #[test]
    fn c_exact_cache_owns_lock_and_returns_verified_bytes() {
        let center = CString::new("cod").unwrap();
        let mut identity = MaybeUninit::<SidereonProductIdentity>::uninit();
        assert_eq!(
            unsafe {
                sidereon_data_product_identity(
                    center.as_ptr(),
                    SidereonProductFamily::Sp3 as u32,
                    2026,
                    7,
                    12,
                    ptr::null(),
                    ptr::null(),
                    identity.as_mut_ptr(),
                )
            },
            SidereonStatus::Ok
        );
        let identity = unsafe { identity.assume_init() };
        let root = std::env::temp_dir().join(format!(
            "sidereon-c-exact-cache-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let stable = root.join("COD0MGXFIN_20261930000_01D_05M_ORB.SP3");
        let stable_c = CString::new(stable.to_string_lossy().as_bytes()).unwrap();
        let mut cache = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_exact_cache_open(
                    stable_c.as_ptr(),
                    &identity,
                    SidereonDistributionSource::InMemory as u32,
                    1_000,
                    &mut cache,
                )
            },
            SidereonStatus::Ok
        );
        assert!(!cache.is_null());

        let mut blocked = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_exact_cache_open(
                    stable_c.as_ptr(),
                    &identity,
                    SidereonDistributionSource::InMemory as u32,
                    0,
                    &mut blocked,
                )
            },
            SidereonStatus::Timeout
        );
        assert!(blocked.is_null());

        let product = b"validated product";
        let archive = b"archive";
        let provenance = b"{\"identity\":\"exact\"}";
        let mut published = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_exact_cache_publish(
                    cache,
                    product.as_ptr(),
                    product.len(),
                    archive.as_ptr(),
                    archive.len(),
                    provenance.as_ptr(),
                    provenance.len(),
                    &mut published,
                )
            },
            SidereonStatus::Ok
        );

        let mut required = 0;
        let mut written = 0;
        assert_eq!(
            unsafe {
                sidereon_exact_cache_entry_copy_bytes(
                    published,
                    SidereonExactCacheComponent::Product as u32,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        let mut copied = vec![0; required];
        assert_eq!(
            unsafe {
                sidereon_exact_cache_entry_copy_bytes(
                    published,
                    SidereonExactCacheComponent::Product as u32,
                    copied.as_mut_ptr(),
                    copied.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(&copied[..written], product);

        let mut id = [0_u8; 32];
        assert_eq!(
            unsafe {
                sidereon_exact_cache_entry_copy_id(
                    published,
                    id.as_mut_ptr(),
                    id.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(written, 32);
        assert!(id
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte)));

        let mut hit = false;
        let mut read = ptr::null_mut();
        assert_eq!(
            unsafe { sidereon_exact_cache_read(cache, &mut hit, &mut read) },
            SidereonStatus::Ok
        );
        assert!(hit);
        assert!(!read.is_null());
        unsafe {
            sidereon_exact_cache_entry_free(read);
            sidereon_exact_cache_entry_free(published);
            sidereon_exact_cache_free(cache);
        }

        let mut unlocked_hit = false;
        let mut unlocked = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_exact_cache_read_unlocked(
                    stable_c.as_ptr(),
                    &identity,
                    SidereonDistributionSource::InMemory as u32,
                    &mut unlocked_hit,
                    &mut unlocked,
                )
            },
            SidereonStatus::Ok
        );
        assert!(unlocked_hit);
        assert!(!unlocked.is_null());
        unsafe { sidereon_exact_cache_entry_free(unlocked) };

        let mut reopened = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_exact_cache_open(
                    stable_c.as_ptr(),
                    &identity,
                    SidereonDistributionSource::InMemory as u32,
                    1_000,
                    &mut reopened,
                )
            },
            SidereonStatus::Ok
        );
        unsafe { sidereon_exact_cache_free(reopened) };
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn c_exact_cache_single_flight_hit_skips_fetch_stub() {
        let identity = exact_cache_test_identity();
        let (root, stable_c) = exact_cache_test_paths("single-flight-hit");
        let product = b"precommitted validated product";
        let archive = b"precommitted archive";
        let provenance = b"{\"identity\":\"precommitted\"}";

        let mut cache = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_exact_cache_open(
                    stable_c.as_ptr(),
                    &identity,
                    SidereonDistributionSource::InMemory as u32,
                    1_000,
                    &mut cache,
                )
            },
            SidereonStatus::Ok
        );
        let mut committed = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_exact_cache_publish(
                    cache,
                    product.as_ptr(),
                    product.len(),
                    archive.as_ptr(),
                    archive.len(),
                    provenance.as_ptr(),
                    provenance.len(),
                    &mut committed,
                )
            },
            SidereonStatus::Ok
        );
        unsafe {
            sidereon_exact_cache_entry_free(committed);
            sidereon_exact_cache_free(cache);
        }

        let mut options = MaybeUninit::<SidereonExactCacheSingleFlightOptions>::uninit();
        assert_eq!(
            unsafe { sidereon_exact_cache_single_flight_options_init(options.as_mut_ptr()) },
            SidereonStatus::Ok
        );
        let options = unsafe { options.assume_init() };
        let mut result = SidereonExactCacheOpenResult::Owner;
        let mut entry = ptr::null_mut();
        let mut owner = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_exact_cache_open_single_flight(
                    stable_c.as_ptr(),
                    &identity,
                    SidereonDistributionSource::InMemory as u32,
                    &options,
                    &mut result,
                    &mut entry,
                    &mut owner,
                )
            },
            SidereonStatus::Ok
        );
        let mut fetch_stub_calls = 0;
        if result == SidereonExactCacheOpenResult::Owner {
            let fetch_stub = |calls: &mut usize| *calls += 1;
            fetch_stub(&mut fetch_stub_calls);
        }
        assert_eq!(result, SidereonExactCacheOpenResult::Hit);
        assert_eq!(fetch_stub_calls, 0);
        assert!(!entry.is_null());
        assert!(owner.is_null());

        let mut written = 0;
        let mut required = 0;
        let mut copied = vec![0; product.len()];
        assert_eq!(
            unsafe {
                sidereon_exact_cache_entry_copy_bytes(
                    entry,
                    SidereonExactCacheComponent::Product as u32,
                    copied.as_mut_ptr(),
                    copied.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(&copied[..written], product);
        unsafe { sidereon_exact_cache_entry_free(entry) };
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn c_exact_cache_single_flight_owner_publishes_then_opens_as_hit() {
        let identity = exact_cache_test_identity();
        let (root, stable_c) = exact_cache_test_paths("single-flight-owner");
        let options = default_single_flight_options();
        let mut result = SidereonExactCacheOpenResult::Hit;
        let mut entry = ptr::null_mut();
        let mut owner = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_exact_cache_open_single_flight(
                    stable_c.as_ptr(),
                    &identity,
                    SidereonDistributionSource::InMemory as u32,
                    &options,
                    &mut result,
                    &mut entry,
                    &mut owner,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(result, SidereonExactCacheOpenResult::Owner);
        assert!(entry.is_null());
        assert!(!owner.is_null());
        assert_eq!(
            unsafe { sidereon_exact_cache_owner_heartbeat(owner) },
            SidereonStatus::Ok
        );

        let product = b"single-flight validated product";
        let archive = b"single-flight archive";
        let provenance = b"{\"identity\":\"single-flight\"}";
        let mut published = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_exact_cache_owner_publish(
                    owner,
                    product.as_ptr(),
                    product.len(),
                    archive.as_ptr(),
                    archive.len(),
                    provenance.as_ptr(),
                    provenance.len(),
                    &mut published,
                )
            },
            SidereonStatus::Ok
        );
        assert!(!published.is_null());
        assert_eq!(
            unsafe { sidereon_exact_cache_owner_heartbeat(owner) },
            SidereonStatus::InvalidArgument
        );
        unsafe {
            sidereon_exact_cache_owner_free(owner);
            sidereon_exact_cache_entry_free(published);
        }

        result = SidereonExactCacheOpenResult::Owner;
        entry = ptr::null_mut();
        owner = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_exact_cache_open_single_flight(
                    stable_c.as_ptr(),
                    &identity,
                    SidereonDistributionSource::InMemory as u32,
                    &options,
                    &mut result,
                    &mut entry,
                    &mut owner,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(result, SidereonExactCacheOpenResult::Hit);
        assert!(!entry.is_null());
        assert!(owner.is_null());
        unsafe { sidereon_exact_cache_entry_free(entry) };
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn c_exact_cache_single_flight_maps_live_owner_wait_timeout() {
        let identity = exact_cache_test_identity();
        let (root, stable_c) = exact_cache_test_paths("single-flight-timeout");
        let mut owner_options = default_single_flight_options();
        owner_options.poll_interval_ms = 1;
        owner_options.heartbeat_interval_ms = 5;
        owner_options.liveness_timeout_ms = 200;
        owner_options.wait_timeout_ms = 1_000;

        let mut result = SidereonExactCacheOpenResult::Hit;
        let mut entry = ptr::null_mut();
        let mut owner = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_exact_cache_open_single_flight(
                    stable_c.as_ptr(),
                    &identity,
                    SidereonDistributionSource::InMemory as u32,
                    &owner_options,
                    &mut result,
                    &mut entry,
                    &mut owner,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(result, SidereonExactCacheOpenResult::Owner);

        let mut waiter_options = owner_options;
        waiter_options.wait_timeout_ms = 20;
        let mut waiter_result = SidereonExactCacheOpenResult::Owner;
        let mut waiter_entry = ptr::dangling_mut::<SidereonExactCacheEntry>();
        let mut waiter_owner = ptr::dangling_mut::<SidereonExactCacheOwner>();
        assert_eq!(
            unsafe {
                sidereon_exact_cache_open_single_flight(
                    stable_c.as_ptr(),
                    &identity,
                    SidereonDistributionSource::InMemory as u32,
                    &waiter_options,
                    &mut waiter_result,
                    &mut waiter_entry,
                    &mut waiter_owner,
                )
            },
            SidereonStatus::Timeout
        );
        assert_eq!(waiter_result, SidereonExactCacheOpenResult::Hit);
        assert!(waiter_entry.is_null());
        assert!(waiter_owner.is_null());

        unsafe { sidereon_exact_cache_owner_free(owner) };
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn c_exact_cache_single_flight_rejects_invalid_options_without_fallback() {
        let identity = exact_cache_test_identity();
        let (root, stable_c) = exact_cache_test_paths("single-flight-options");
        let defaults = default_single_flight_options();

        for options in [
            SidereonExactCacheSingleFlightOptions {
                poll_interval_ms: 0,
                ..defaults
            },
            SidereonExactCacheSingleFlightOptions {
                heartbeat_interval_ms: 0,
                ..defaults
            },
            SidereonExactCacheSingleFlightOptions {
                liveness_timeout_ms: 0,
                ..defaults
            },
            SidereonExactCacheSingleFlightOptions {
                wait_timeout_ms: 0,
                ..defaults
            },
            SidereonExactCacheSingleFlightOptions {
                heartbeat_interval_ms: defaults.liveness_timeout_ms,
                ..defaults
            },
            SidereonExactCacheSingleFlightOptions {
                struct_size: 0,
                ..defaults
            },
            SidereonExactCacheSingleFlightOptions {
                abi_version: u32::MAX,
                ..defaults
            },
        ] {
            let mut result = SidereonExactCacheOpenResult::Owner;
            let mut entry = ptr::dangling_mut::<SidereonExactCacheEntry>();
            let mut owner = ptr::dangling_mut::<SidereonExactCacheOwner>();
            assert_eq!(
                unsafe {
                    sidereon_exact_cache_open_single_flight(
                        stable_c.as_ptr(),
                        &identity,
                        SidereonDistributionSource::InMemory as u32,
                        &options,
                        &mut result,
                        &mut entry,
                        &mut owner,
                    )
                },
                SidereonStatus::InvalidArgument
            );
            assert_eq!(result, SidereonExactCacheOpenResult::Hit);
            assert!(entry.is_null());
            assert!(owner.is_null());
        }
        assert!(!root.exists());
    }

    #[test]
    fn cddis_rejects_unsupported_family_without_substitution() {
        let center = CString::new("igs").unwrap();
        let mut location = MaybeUninit::<SidereonDistributionLocation>::uninit();
        let status = unsafe {
            sidereon_data_distribution_location(
                center.as_ptr(),
                SidereonProductFamily::RinexNavigation as u32,
                2020,
                6,
                25,
                ptr::null(),
                ptr::null(),
                SidereonDistributionSource::NasaCddis as u32,
                location.as_mut_ptr(),
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);
    }

    #[test]
    fn predicted_ionex_direct_locations_preserve_tier_and_identity_year() {
        for (center, year, month, day, expected) in [
            (
                "cod_prd1",
                2026,
                7,
                15,
                "https://www.aiub.unibe.ch/download/CODE/IONO/P1/2026/\
COD0OPSPRD_20261960000_01D_01H_GIM.INX.gz",
            ),
            (
                "cod_prd2",
                2026,
                7,
                16,
                "https://www.aiub.unibe.ch/download/CODE/IONO/P2/2026/\
COD0OPSPRD_20261970000_01D_01H_GIM.INX.gz",
            ),
            (
                "cod_prd2",
                2027,
                1,
                1,
                "https://www.aiub.unibe.ch/download/CODE/IONO/P2/2027/\
COD0OPSPRD_20270010000_01D_01H_GIM.INX.gz",
            ),
        ] {
            let center = CString::new(center).unwrap();
            let mut location = MaybeUninit::<SidereonDistributionLocation>::uninit();
            let status = unsafe {
                sidereon_data_distribution_location(
                    center.as_ptr(),
                    SidereonProductFamily::Ionex as u32,
                    year,
                    month,
                    day,
                    ptr::null(),
                    ptr::null(),
                    SidereonDistributionSource::Direct as u32,
                    location.as_mut_ptr(),
                )
            };
            assert_eq!(status, SidereonStatus::Ok);
            let location = unsafe { location.assume_init() };
            assert_eq!(location.compression, SidereonArchiveCompression::Gzip);
            assert_eq!(
                unsafe { CStr::from_ptr(location.original_url.as_ptr()) }
                    .to_str()
                    .unwrap(),
                expected
            );
        }
    }

    #[test]
    fn c_next_issue_due_maps_identity_due_time_and_split_coverage() {
        let center = CString::new("igs_ult").unwrap();
        let mut written = usize::MAX;
        let mut required = usize::MAX;
        assert_eq!(
            unsafe {
                sidereon_data_next_issue_due_json(
                    center.as_ptr(),
                    SidereonProductFamily::Sp3 as u32,
                    2026,
                    8,
                    4,
                    2,
                    59,
                    59,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(written, 0);
        assert!(required > 0);

        let mut bytes = vec![0_u8; required];
        assert_eq!(
            unsafe {
                sidereon_data_next_issue_due_json(
                    center.as_ptr(),
                    SidereonProductFamily::Sp3 as u32,
                    2026,
                    8,
                    4,
                    2,
                    59,
                    59,
                    bytes.as_mut_ptr(),
                    bytes.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(written, required);
        let value: serde_json::Value =
            serde_json::from_slice(&bytes[..written]).expect("nominal issue JSON");
        assert_eq!(value["identity"]["family"], "sp3");
        assert_eq!(value["identity"]["analysis_center"], "igs_ult");
        assert_eq!(value["identity"]["date"], "2026-08-03");
        assert_eq!(value["identity"]["issue"], "0000");
        assert_eq!(value["due_at"], "2026-08-04T03:00:00Z");
        assert_eq!(
            value["covers"]["observed"],
            serde_json::json!({
                "from": "2026-08-03T00:00:00Z",
                "until": "2026-08-04T00:00:00Z",
            })
        );
        assert_eq!(
            value["covers"]["predicted"],
            serde_json::json!({
                "from": "2026-08-04T00:00:00Z",
                "until": "2026-08-05T00:00:00Z",
            })
        );
    }

    #[test]
    fn c_next_issue_due_rejects_unsupported_schedule() {
        let center = CString::new("wum_nrt").unwrap();
        let mut written = usize::MAX;
        let mut required = usize::MAX;
        assert_eq!(
            unsafe {
                sidereon_data_next_issue_due_json(
                    center.as_ptr(),
                    SidereonProductFamily::Sp3 as u32,
                    2026,
                    8,
                    4,
                    0,
                    0,
                    0,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert_eq!((written, required), (0, 0));
    }
}
