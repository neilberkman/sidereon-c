//! Pure GNSS product identity and public-distributor derivation.
//!
//! The C interface intentionally performs no HTTP or cache IO. Callers select a
//! source, use the returned public URL and compression metadata with their own
//! transport, then pass verified SP3/IONEX bytes to the existing parsers.

use std::ffi::c_char;

use sidereon_core::data::{
    self as core_data, AnalysisCenter, ArchiveCompression, DistributionSource, ProductCampaign,
    ProductDate, ProductFormat, ProductIdentity, ProductPublisher, ProductType, SolutionClass,
};

use super::{ffi_boundary, parse_bounded_c_string, require_out, set_last_error, SidereonStatus};

pub const PRODUCT_TOKEN_C_BYTES: usize = 16;
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
    pub family: SidereonProductFamily,
    pub publisher: SidereonProductPublisher,
    pub solution_class: SidereonSolutionClass,
    pub campaign: SidereonProductCampaign,
    pub filename_version: u8,
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub has_issue: bool,
    pub issue: [c_char; PRODUCT_TOKEN_C_BYTES],
    pub span: [c_char; PRODUCT_TOKEN_C_BYTES],
    pub sample: [c_char; PRODUCT_TOKEN_C_BYTES],
    pub official_filename: [c_char; OFFICIAL_FILENAME_C_BYTES],
    pub format: SidereonProductFormat,
    pub has_prediction_horizon_days: bool,
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

fn map_error(fn_name: &str, error: impl core::fmt::Display) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {error}"));
    SidereonStatus::InvalidArgument
}

fn fixed_text<const N: usize>(
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

fn family_to_core(family: SidereonProductFamily) -> ProductType {
    match family {
        SidereonProductFamily::Sp3 => ProductType::Sp3,
        SidereonProductFamily::Ionex => ProductType::Ionex,
        SidereonProductFamily::RinexClock => ProductType::Clk,
        SidereonProductFamily::RinexNavigation => ProductType::Nav,
    }
}

fn source_to_core(source: SidereonDistributionSource) -> DistributionSource {
    match source {
        SidereonDistributionSource::Direct => DistributionSource::Direct,
        SidereonDistributionSource::NasaCddis => DistributionSource::NasaCddis,
        SidereonDistributionSource::LocalFile => DistributionSource::LocalFile,
        SidereonDistributionSource::InMemory => DistributionSource::InMemory,
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

fn identity_to_c(
    fn_name: &str,
    identity: &ProductIdentity,
) -> Result<SidereonProductIdentity, SidereonStatus> {
    Ok(SidereonProductIdentity {
        family: match identity.family {
            ProductType::Sp3 => SidereonProductFamily::Sp3,
            ProductType::Ionex => SidereonProductFamily::Ionex,
            ProductType::Clk => SidereonProductFamily::RinexClock,
            ProductType::Nav => SidereonProductFamily::RinexNavigation,
        },
        publisher: publisher_from_core(identity.publisher),
        solution_class: solution_from_core(identity.solution),
        campaign: campaign_from_core(identity.campaign),
        filename_version: identity.version,
        year: identity.date.year,
        month: identity.date.month,
        day: identity.date.day,
        has_issue: identity.issue.is_some(),
        issue: fixed_text(fn_name, "issue", identity.issue.as_deref().unwrap_or(""))?,
        span: fixed_text(fn_name, "span", &identity.span)?,
        sample: fixed_text(fn_name, "sample", &identity.sample)?,
        official_filename: fixed_text(fn_name, "official_filename", &identity.official_filename)?,
        format: format_from_core(identity.format),
        has_prediction_horizon_days: identity.prediction_horizon_days.is_some(),
        prediction_horizon_days: identity.prediction_horizon_days.unwrap_or(0),
    })
}

#[derive(Clone, Copy)]
struct ProductInputs {
    center: *const c_char,
    family: SidereonProductFamily,
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
    core_data::product(
        center,
        family_to_core(input.family),
        date,
        sample.as_deref(),
        issue.as_deref(),
    )
    .map_err(|error| map_error(fn_name, error))
}

/// Resolve an exact catalog product identity independently from distributor.
///
/// `sample` may be NULL to use the catalog default. `issue` may be NULL only
/// for product lines that do not require an ultra-rapid issue.
///
/// Safety: non-null text pointers must reference null-terminated UTF-8 strings;
/// `out_identity` must reference writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_data_product_identity(
    center: *const c_char,
    family: SidereonProductFamily,
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

/// Resolve one explicit distributor for an exact catalog product.
///
/// This function performs no network or file IO. `original_url` is absent for
/// local-file and in-memory sources.
///
/// Safety: non-null text pointers must reference null-terminated UTF-8 strings;
/// `out_location` must reference writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_data_distribution_location(
    center: *const c_char,
    family: SidereonProductFamily,
    year: i32,
    month: u8,
    day: u8,
    sample: *const c_char,
    issue: *const c_char,
    source: SidereonDistributionSource,
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
        let location = match product.distribution_location(source_to_core(source)) {
            Ok(location) => location,
            Err(error) => return map_error(FN_NAME, error),
        };
        let original_url = location.original_url.as_deref().unwrap_or("");
        let converted = (|| {
            Ok::<_, SidereonStatus>(SidereonDistributionLocation {
                source,
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

#[cfg(test)]
mod tests {
    use std::ffi::{CStr, CString};
    use std::mem::MaybeUninit;
    use std::ptr;

    use super::*;

    #[test]
    fn exact_identity_and_cddis_path_match_the_core() {
        let center = CString::new("cod").unwrap();
        let mut identity = MaybeUninit::<SidereonProductIdentity>::uninit();
        let status = unsafe {
            sidereon_data_product_identity(
                center.as_ptr(),
                SidereonProductFamily::Sp3,
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
        assert_eq!(identity.publisher, SidereonProductPublisher::Code);
        assert_eq!(identity.solution_class, SidereonSolutionClass::Final);
        assert_eq!(
            identity.campaign,
            SidereonProductCampaign::MultiGnssExperiment
        );
        assert_eq!(
            unsafe { CStr::from_ptr(identity.official_filename.as_ptr()) }
                .to_str()
                .unwrap(),
            "COD0MGXFIN_20261930000_01D_05M_ORB.SP3"
        );

        let mut location = MaybeUninit::<SidereonDistributionLocation>::uninit();
        let status = unsafe {
            sidereon_data_distribution_location(
                center.as_ptr(),
                SidereonProductFamily::Sp3,
                2026,
                7,
                12,
                ptr::null(),
                ptr::null(),
                SidereonDistributionSource::NasaCddis,
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
    fn cddis_rejects_unsupported_family_without_substitution() {
        let center = CString::new("igs").unwrap();
        let mut location = MaybeUninit::<SidereonDistributionLocation>::uninit();
        let status = unsafe {
            sidereon_data_distribution_location(
                center.as_ptr(),
                SidereonProductFamily::RinexNavigation,
                2020,
                6,
                25,
                ptr::null(),
                ptr::null(),
                SidereonDistributionSource::NasaCddis,
                location.as_mut_ptr(),
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);
    }
}
