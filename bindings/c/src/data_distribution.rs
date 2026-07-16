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
    ProductDate, ProductFormat, ProductIdentity, ProductPublisher, ProductType, SolutionClass,
};
use sidereon_core::exact_cache::{
    CommittedExactCacheEntry, ExactCacheError, ExactCacheGuard, ExactProductCache,
};

use super::{
    copy_prefix_to_c, ffi_boundary, free_boxed, init_copy_counts, parse_bounded_c_string,
    require_out, require_ref, require_slice, set_last_error, write_boxed_handle, SidereonStatus,
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
    let status = if matches!(error, ExactCacheError::LockTimeout) {
        SidereonStatus::Timeout
    } else {
        SidereonStatus::InvalidArgument
    };
    set_last_error(format!("{fn_name}: {error}"));
    status
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
    }
}

fn solution_from_core(value: SolutionClass) -> SidereonSolutionClass {
    match value {
        SolutionClass::Final => SidereonSolutionClass::Final,
        SolutionClass::Rapid => SidereonSolutionClass::Rapid,
        SolutionClass::UltraRapid => SidereonSolutionClass::UltraRapid,
        SolutionClass::Predicted => SidereonSolutionClass::Predicted,
        SolutionClass::Broadcast => SidereonSolutionClass::Broadcast,
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

unsafe fn product_spec(
    fn_name: &str,
    input: ProductInputs,
) -> Result<core_data::ProductSpec, SidereonStatus> {
    let center = parse_bounded_c_string(fn_name, "center", input.center, 32)?;
    let center = AnalysisCenter::from_code(&center).ok_or_else(|| {
        set_last_error(format!("{fn_name}: unknown analysis center {center:?}"));
        SidereonStatus::InvalidArgument
    })?;
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

/// Resolve an exact catalog product identity independently from distributor.
///
/// `family` is one of SidereonProductFamily_* encoded as uint32_t. Invalid
/// values fail closed with SIDEREON_STATUS_INVALID_ARGUMENT.
///
/// `sample` may be NULL to use the catalog default. `issue` may be NULL only
/// for product lines that do not require an ultra-rapid issue.
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
}
