// Print tests/w3_data_distribution_pins.h: sidereon-core's product-catalog
// answers for every query data_distribution_smoke.c makes, and the merge-input
// identities of the artifacts it builds. Each table row carries the query
// inputs and the engine's answer, so the C program runs the rows and compares.

use sidereon_core::data::{
    self as core_data, AnalysisCenter, ArchiveCompression, DistributionSource, ProductDate,
    ProductIdentity, ProductType,
};
use sidereon_core::ephemeris::{
    MergeCombine, MergeOptions, MergePrecedenceScope, OutlierRejectOptions, Sp3ArtifactIdentity,
    Sp3FrameLabelSet, Sp3FrameReconciliationOptions, Sp3MergeInputIdentity,
};
use sidereon_core::GnssSystem;
use std::collections::BTreeSet;
use valgen::{c_string, header_end, header_start};

const P: &str = "W3DD";

fn def(name: &str, value: impl std::fmt::Display) {
    println!("#define {P}_{name} {value}");
}

fn def_str(name: &str, value: &str) {
    def(name, c_string(value));
}

fn def_bool(name: &str, value: bool) {
    def(name, if value { "true" } else { "false" });
}

fn variant(debug: &str) -> String {
    let cut = debug.find(['{', ' ', '(']).unwrap_or(debug.len());
    debug[..cut].trim().to_string()
}

fn snake(name: &str) -> String {
    let mut out = String::new();
    let mut previous: Option<char> = None;
    for c in name.chars() {
        if c.is_ascii_uppercase()
            && previous.is_some_and(|p| p.is_ascii_lowercase() || p.is_ascii_digit())
        {
            out.push('_');
        }
        out.push(c.to_ascii_uppercase());
        previous = Some(c);
    }
    out
}

/// The C constant the binding writes for an engine enum value.
fn c_const(prefix: &str, value: &impl std::fmt::Debug) -> String {
    format!("{prefix}_{}", snake(&variant(&format!("{value:?}"))))
}

fn opt_str(value: Option<&str>) -> String {
    value.map_or("NULL".to_string(), c_string)
}

/// Every catalog refusal is SIDEREON_STATUS_INVALID_ARGUMENT at the C
/// boundary (map_error in src/data_distribution.rs).
fn status<T, E>(result: &Result<T, E>) -> &'static str {
    if result.is_ok() {
        "SIDEREON_STATUS_OK"
    } else {
        "SIDEREON_STATUS_INVALID_ARGUMENT"
    }
}

#[derive(Clone, Copy)]
enum Family {
    Sp3,
    Clk,
    Ionex,
    Nav,
}

impl Family {
    fn core(self) -> ProductType {
        match self {
            Family::Sp3 => ProductType::Sp3,
            Family::Clk => ProductType::Clk,
            Family::Ionex => ProductType::Ionex,
            Family::Nav => ProductType::Nav,
        }
    }

    fn c(self) -> &'static str {
        match self {
            Family::Sp3 => "SIDEREON_PRODUCT_FAMILY_SP3",
            Family::Clk => "SIDEREON_PRODUCT_FAMILY_RINEX_CLOCK",
            Family::Ionex => "SIDEREON_PRODUCT_FAMILY_IONEX",
            Family::Nav => "SIDEREON_PRODUCT_FAMILY_RINEX_NAVIGATION",
        }
    }
}

use Family::{Clk, Ionex, Nav, Sp3};

fn center(code: &str) -> AnalysisCenter {
    AnalysisCenter::from_code(code).unwrap_or_else(|| panic!("analysis center {code}"))
}

fn date(y: i32, m: u8, d: u8) -> Result<ProductDate, core_data::DataCatalogError> {
    ProductDate::new(y, m, d)
}

fn spec(
    c: &str,
    f: Family,
    y: i32,
    m: u8,
    d: u8,
    sample: Option<&str>,
    issue: Option<&str>,
) -> Result<core_data::ProductSpec, core_data::DataCatalogError> {
    core_data::product(center(c), f.core(), date(y, m, d)?, sample, issue)
}

fn identity(
    c: &str,
    f: Family,
    y: i32,
    m: u8,
    d: u8,
    sample: Option<&str>,
    issue: Option<&str>,
) -> ProductIdentity {
    spec(c, f, y, m, d, sample, issue)
        .and_then(|spec| spec.identity())
        .unwrap_or_else(|err| panic!("{c} {y}-{m}-{d}: {err}"))
}

fn solution_class_table() {
    let rows: [(&str, Family); 3] = [("igs", Sp3), ("igs", Nav), ("igs", Clk)];
    println!("typedef struct W3DdSolutionClassRow {{");
    println!("    const char *center;");
    println!("    uint32_t family;");
    println!("    int status;");
    println!("    uint32_t solution_class;");
    println!("}} W3DdSolutionClassRow;");
    println!("static const W3DdSolutionClassRow {P}_SOLUTION_CLASS_ROWS[] = {{");
    for (c, f) in rows {
        let result = core_data::product_solution_class(center(c), f.core());
        println!(
            "    {{ {}, {}, {}, {} }},",
            c_string(c),
            f.c(),
            status(&result),
            result
                .as_ref()
                .map_or("SIDEREON_SOLUTION_CLASS_FINAL".to_string(), |s| c_const(
                    "SIDEREON_SOLUTION_CLASS",
                    s
                ))
        );
    }
    println!("}};");
    println!();
}

fn default_sample_table() {
    let rows: [(&str, Family, i32, u8, u8); 8] = [
        ("gfz", Sp3, 2021, 5, 17),
        ("gfz", Sp3, 2021, 5, 18),
        ("gfz", Sp3, 2026, 7, 19),
        ("esa_ult", Sp3, 2024, 9, 3),
        ("esa_ult", Sp3, 2025, 2, 2),
        ("esa_ult", Sp3, 2025, 2, 3),
        ("gfz_ult", Sp3, 2021, 5, 15),
        ("gfz_ult", Sp3, 2021, 5, 16),
    ];
    println!("typedef struct W3DdDefaultSampleRow {{");
    println!("    const char *center;");
    println!("    uint32_t family;");
    println!("    int32_t year;");
    println!("    uint8_t month;");
    println!("    uint8_t day;");
    println!("    int status;");
    println!("    const char *sample;");
    println!("}} W3DdDefaultSampleRow;");
    println!("static const W3DdDefaultSampleRow {P}_DEFAULT_SAMPLE_ROWS[] = {{");
    for (c, f, y, m, d) in rows {
        let result = date(y, m, d)
            .and_then(|date| core_data::default_sample_for_date(center(c), f.core(), date));
        println!(
            "    {{ {}, {}, {y}, {m}, {d}, {}, {} }},",
            c_string(c),
            f.c(),
            status(&result),
            c_string(result.as_deref().unwrap_or(""))
        );
    }
    println!("}};");
    println!();
}

fn supported_samples_table() {
    let rows: [(&str, Family, i32, u8, u8, Option<&str>); 11] = [
        ("esa", Sp3, 2026, 6, 15, None),
        ("gfz", Sp3, 2021, 5, 17, None),
        ("gfz", Sp3, 2021, 5, 18, None),
        ("esa_ult", Sp3, 2025, 2, 2, Some("0600")),
        ("esa_ult", Sp3, 2025, 2, 2, Some("1200")),
        ("gfz_ult", Sp3, 2021, 5, 15, Some("0000")),
        ("gfz_ult", Sp3, 2021, 5, 15, Some("2100")),
        ("gfz_ult", Sp3, 2021, 5, 15, Some("0130")),
        ("cod", Clk, 2026, 6, 15, None),
        ("cod", Ionex, 2026, 6, 15, None),
        ("igs", Nav, 2026, 6, 15, None),
    ];
    println!("typedef struct W3DdSupportedSamplesRow {{");
    println!("    const char *center;");
    println!("    uint32_t family;");
    println!("    int32_t year;");
    println!("    uint8_t month;");
    println!("    uint8_t day;");
    println!("    const char *issue;");
    println!("    int status;");
    println!("    size_t count;");
    println!("    const char *samples[4];");
    println!("}} W3DdSupportedSamplesRow;");
    println!("static const W3DdSupportedSamplesRow {P}_SUPPORTED_SAMPLES_ROWS[] = {{");
    for (c, f, y, m, d, issue) in rows {
        let result = date(y, m, d)
            .and_then(|date| core_data::supported_samples(center(c), f.core(), date, issue));
        let samples: Vec<&str> = result.as_ref().map_or(Vec::new(), |s| s.to_vec());
        assert!(samples.len() <= 4, "at most four samples per row");
        println!(
            "    {{ {}, {}, {y}, {m}, {d}, {}, {}, {}, {{ {} }} }},",
            c_string(c),
            f.c(),
            opt_str(issue),
            status(&result),
            samples.len(),
            if samples.is_empty() {
                "NULL".to_string()
            } else {
                samples
                    .iter()
                    .map(|s| c_string(s))
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        );
    }
    println!("}};");
    println!();
}

fn content_start_table() {
    let issues = [
        "0000", "0300", "0600", "0900", "1200", "1500", "1800", "2100",
    ];
    let mut rows: Vec<(&str, i32, u8, u8, Option<&str>)> = vec![
        ("gfz_ult", 2022, 9, 6, Some("2100")),
        ("gfz_ult", 2022, 9, 9, Some("0000")),
        ("igs", 2022, 9, 7, None),
    ];
    for day in [7, 8] {
        for issue in issues {
            rows.push(("gfz_ult", 2022, 9, day, Some(issue)));
        }
    }
    rows.push(("gfz_ult", 2022, 9, 7, Some("0130")));
    rows.push(("gfz", 2022, 9, 7, Some("0000")));
    rows.push(("gfz_ult", 2022, 9, 7, None));
    println!("typedef struct W3DdContentStartRow {{");
    println!("    const char *center;");
    println!("    int32_t year;");
    println!("    uint8_t month;");
    println!("    uint8_t day;");
    println!("    const char *issue;");
    println!("    int status;");
    println!("    uint32_t convention;");
    println!("    int64_t offset_s;");
    println!("}} W3DdContentStartRow;");
    println!("static const W3DdContentStartRow {P}_CONTENT_START_ROWS[] = {{");
    for (c, y, m, d, issue) in rows {
        let result = date(y, m, d)
            .and_then(|date| core_data::sp3_content_start_convention(center(c), date, issue));
        // A refused query leaves the C outputs at FILENAME_EPOCH and 0
        // (sidereon_data_sp3_content_start_convention).
        let (convention, offset) = match &result {
            Ok(convention) => (
                c_const("SIDEREON_SP3_CONTENT_START_CONVENTION", convention),
                convention.content_start_offset_s(),
            ),
            Err(_) => (
                "SIDEREON_SP3_CONTENT_START_CONVENTION_FILENAME_EPOCH".to_string(),
                0,
            ),
        };
        println!(
            "    {{ {}, {y}, {m}, {d}, {}, {}, {convention}, {offset} }},",
            c_string(c),
            opt_str(issue),
            status(&result)
        );
    }
    println!("}};");
    println!();
}

type Query = (
    &'static str,
    Family,
    i32,
    u8,
    u8,
    Option<&'static str>,
    Option<&'static str>,
);

fn identity_table() {
    let rows: Vec<Query> = vec![
        ("igs", Sp3, 2022, 11, 26, None, None),
        ("igs", Sp3, 2022, 11, 27, None, None),
        ("igs", Sp3, 1994, 1, 2, None, None),
        ("igs", Sp3, 1994, 1, 1, None, None),
        ("esa", Sp3, 2014, 1, 4, None, None),
        ("esa", Sp3, 2014, 1, 5, None, None),
        ("gfz", Sp3, 2020, 5, 12, None, None),
        ("gfz", Sp3, 2020, 5, 13, None, None),
        ("esa", Clk, 2014, 1, 4, None, None),
        ("esa", Clk, 2014, 1, 5, None, None),
        ("gfz", Clk, 2020, 5, 12, None, None),
        ("gfz", Clk, 2020, 5, 13, None, None),
        ("igs_ult", Sp3, 2022, 11, 26, None, Some("0600")),
        ("igs_ult", Sp3, 2022, 11, 27, None, Some("0600")),
        ("cod_ult", Sp3, 2022, 11, 26, None, Some("0000")),
        ("cod_ult", Sp3, 2022, 11, 27, None, Some("0000")),
        ("esa_ult", Sp3, 2022, 10, 3, None, Some("0600")),
        ("esa_ult", Sp3, 2022, 10, 4, None, Some("0600")),
        ("gfz_ult", Sp3, 2020, 10, 5, None, Some("0600")),
        ("gfz_ult", Sp3, 2020, 10, 6, None, Some("0600")),
        ("esa_ult", Sp3, 2024, 9, 3, None, Some("0600")),
        ("esa_ult", Sp3, 2025, 2, 2, None, Some("0600")),
        ("esa_ult", Sp3, 2025, 2, 2, None, Some("1200")),
        ("gfz_ult", Sp3, 2021, 5, 15, None, Some("0600")),
        ("gfz_ult", Sp3, 2021, 5, 16, None, Some("0600")),
        ("esa", Sp3, 2026, 6, 15, Some("15M"), None),
        ("gfz", Sp3, 2021, 5, 17, Some("05M"), None),
        ("esa_ult", Sp3, 2025, 2, 2, Some("05M"), Some("0600")),
        ("gfz_ult", Sp3, 2021, 5, 15, Some("05M"), Some("2100")),
        ("gfz_ult", Sp3, 2021, 5, 15, Some("05M"), Some("0000")),
        ("igs", Nav, 2020, 6, 25, None, None),
        ("cod", Sp3, 2022, 11, 26, None, None),
        ("cod", Clk, 2022, 11, 26, None, None),
        ("cod", Ionex, 2022, 11, 26, None, None),
        ("cod_rap", Sp3, 2026, 4, 30, None, None),
        ("cod", Sp3, 2026, 7, 12, None, None),
        ("cod", Sp3, 2026, 7, 13, None, None),
    ];
    println!("typedef struct W3DdIdentityRow {{");
    println!("    const char *center;");
    println!("    uint32_t family;");
    println!("    int32_t year;");
    println!("    uint8_t month;");
    println!("    uint8_t day;");
    println!("    const char *sample;");
    println!("    const char *issue;");
    println!("    int status;");
    println!("    const char *official_filename;");
    println!("    const char *sample_out;");
    println!("    uint32_t solution_class;");
    println!("}} W3DdIdentityRow;");
    println!("static const W3DdIdentityRow {P}_IDENTITY_ROWS[] = {{");
    for (c, f, y, m, d, sample, issue) in rows {
        let result = spec(c, f, y, m, d, sample, issue).and_then(|spec| spec.identity());
        let (filename, sample_out, class) = match &result {
            Ok(identity) => (
                identity.official_filename.clone(),
                identity.sample.clone(),
                c_const("SIDEREON_SOLUTION_CLASS", &identity.solution),
            ),
            Err(_) => (
                String::new(),
                String::new(),
                "SIDEREON_SOLUTION_CLASS_FINAL".to_string(),
            ),
        };
        println!(
            "    {{ {}, {}, {y}, {m}, {d}, {}, {}, {}, {}, {}, {class} }},",
            c_string(c),
            f.c(),
            opt_str(sample),
            opt_str(issue),
            status(&result),
            c_string(&filename),
            c_string(&sample_out)
        );
    }
    println!("}};");
    println!();
}

fn location_table() {
    use DistributionSource::{Direct, NasaCddis};
    let rows: Vec<(Query, DistributionSource)> = vec![
        (("igs", Sp3, 2022, 11, 26, None, None), NasaCddis),
        (("igs", Sp3, 2022, 11, 26, None, None), Direct),
        (("igs", Sp3, 2022, 11, 27, None, None), Direct),
        (("igs", Sp3, 2022, 11, 27, None, None), NasaCddis),
        (("igs", Sp3, 1994, 1, 2, None, None), NasaCddis),
        (("esa", Sp3, 2020, 6, 24, None, None), Direct),
        (("esa", Sp3, 2020, 6, 24, None, None), NasaCddis),
        (("gfz", Sp3, 2020, 6, 24, None, None), Direct),
        (("gfz", Sp3, 2020, 6, 24, None, None), NasaCddis),
        (("cod_ult", Sp3, 2022, 11, 26, None, Some("0000")), Direct),
        (("esa_ult", Sp3, 2022, 10, 4, None, Some("0600")), Direct),
        (("esa_ult", Sp3, 2022, 10, 4, None, Some("0600")), NasaCddis),
        (("gfz_ult", Sp3, 2020, 10, 6, None, Some("0600")), Direct),
        (("gfz_ult", Sp3, 2020, 10, 6, None, Some("0600")), NasaCddis),
        (("igs", Sp3, 2020, 6, 24, None, None), NasaCddis),
        (("esa", Sp3, 2024, 6, 24, None, None), NasaCddis),
        (("igs", Nav, 2020, 6, 25, None, None), Direct),
        (("cod", Sp3, 2026, 4, 30, None, None), Direct),
        (("cod", Clk, 2026, 4, 30, None, None), Direct),
        (("cod", Ionex, 2026, 4, 30, None, None), Direct),
        (("cod_rap", Ionex, 2026, 4, 30, None, None), Direct),
        (("esa", Ionex, 2022, 11, 26, None, None), NasaCddis),
        (("esa", Ionex, 2024, 6, 24, None, None), NasaCddis),
        (("cod", Sp3, 2026, 7, 12, None, None), NasaCddis),
    ];
    println!("typedef struct W3DdLocationRow {{");
    println!("    const char *center;");
    println!("    uint32_t family;");
    println!("    int32_t year;");
    println!("    uint8_t month;");
    println!("    uint8_t day;");
    println!("    const char *sample;");
    println!("    const char *issue;");
    println!("    uint32_t source;");
    println!("    int status;");
    println!("    bool has_original_url;");
    println!("    const char *original_url;");
    println!("    const char *archive_filename;");
    println!("    uint32_t compression;");
    println!("}} W3DdLocationRow;");
    println!("static const W3DdLocationRow {P}_LOCATION_ROWS[] = {{");
    for ((c, f, y, m, d, sample, issue), source) in rows {
        let result =
            spec(c, f, y, m, d, sample, issue).and_then(|spec| spec.distribution_location(source));
        let (has_url, url, archive, compression) = match &result {
            Ok(location) => (
                location.original_url.is_some(),
                location.original_url.clone().unwrap_or_default(),
                location.archive_filename.clone(),
                c_const("SIDEREON_ARCHIVE_COMPRESSION", &location.compression),
            ),
            Err(_) => (
                false,
                String::new(),
                String::new(),
                "SIDEREON_ARCHIVE_COMPRESSION_NONE".to_string(),
            ),
        };
        println!(
            "    {{ {}, {}, {y}, {m}, {d}, {}, {}, {}, {}, {has_url}, {}, {}, {compression} }},",
            c_string(c),
            f.c(),
            opt_str(sample),
            opt_str(issue),
            c_const("SIDEREON_DISTRIBUTION_SOURCE", &source),
            status(&result),
            c_string(&url),
            c_string(&archive)
        );
    }
    println!("}};");
    println!();
}

/// data_distribution_smoke.c's artifact_from_identity.
fn artifact(identity: &ProductIdentity, digit: char) -> Sp3ArtifactIdentity {
    let mut resolved = identity.clone();
    resolved.format_version = Some("SP3-d".to_string());
    let next = char::from(digit as u8 + 1);
    Sp3ArtifactIdentity {
        requested_identity: identity.clone(),
        resolved_identity: resolved,
        distribution_source: DistributionSource::Direct,
        official_filename: identity.official_filename.clone(),
        product_sha256: digit.to_string().repeat(64),
        product_byte_length: 12345,
        archive_sha256: next.to_string().repeat(64),
        archive_byte_length: 6789,
        compression: ArchiveCompression::Gzip,
    }
}

fn pair_digest(first: char, second: char) -> String {
    format!("{first}{second}").repeat(32)
}

/// sidereon_sp3_merge_options_init, read back by sp3_merge_options_from_c.
fn init_options() -> MergeOptions {
    let defaults = MergeOptions::default();
    let mut options = MergeOptions::default();
    options.position_tolerance_m = defaults.position_tolerance_m;
    options.clock_tolerance_s = defaults.clock_tolerance_s;
    options.min_agree = defaults.min_agree;
    options.clock_min_common = defaults.clock_min_common;
    options.combine = MergeCombine::Mean;
    options.precedence_scope = MergePrecedenceScope::Cell;
    options.outlier_reject = defaults.outlier_reject;
    options.target_epoch_interval_s = defaults.target_epoch_interval_s;
    options.systems = None;
    let mut frame = Sp3FrameReconciliationOptions::default();
    frame.asserted_equivalent_label_sets = Vec::new();
    frame.helmert = false;
    options.frame_reconciliation = frame;
    options
}

/// The options merge_input_identity_checks sets over the init values.
fn test_options(
    combine: MergeCombine,
    systems: [GnssSystem; 2],
    label_sets: [[&str; 2]; 2],
) -> MergeOptions {
    let mut options = init_options();
    options.position_tolerance_m = 0.0;
    options.clock_tolerance_s = 2.5e-9;
    options.min_agree = 2;
    options.clock_min_common = 3;
    options.combine = combine;
    options.precedence_scope = MergePrecedenceScope::SatelliteArc;
    options.outlier_reject = Some(OutlierRejectOptions::new(1.25, 7.5e-9));
    options.target_epoch_interval_s = Some(900.0);
    options.systems = Some(systems.into_iter().collect::<BTreeSet<_>>());
    let mut frame = Sp3FrameReconciliationOptions::default();
    frame.asserted_equivalent_label_sets = label_sets
        .iter()
        .map(|set| Sp3FrameLabelSet::new(set.iter().copied()))
        .collect();
    frame.helmert = true;
    options.frame_reconciliation = frame;
    options
}

fn merge_section() {
    let first = artifact(&identity("esa", Sp3, 2026, 7, 16, None, None), '1');
    let second = artifact(&identity("cod", Sp3, 2026, 7, 16, None, None), '2');
    let mut first = first;
    let mut second = second;
    first.archive_sha256 = pair_digest('1', '2');
    second.archive_sha256 = pair_digest('2', '3');
    let artifacts = vec![first.clone(), second.clone()];
    let reversed = vec![second.clone(), first.clone()];

    println!("/* merge_input_identity_checks: Sp3MergeInputIdentity of the artifacts the");
    println!(" * C program builds. */");
    let mut legacy = artifact(&identity("igs", Sp3, 2022, 11, 26, None, None), 'a');
    legacy.distribution_source = DistributionSource::NasaCddis;
    legacy.compression = ArchiveCompression::UnixCompress;
    let legacy_identity =
        Sp3MergeInputIdentity::new(&[legacy], &init_options()).expect("legacy merge identity");
    def(
        "LEGACY_CANONICAL_COMPRESSION",
        c_const(
            "SIDEREON_ARCHIVE_COMPRESSION",
            &legacy_identity.contributors[0].compression,
        ),
    );

    let systems = [GnssSystem::Gps, GnssSystem::Galileo];
    let labels = [["IGS20", "ITRF2020"], ["IGS14", "ITRF2014"]];
    let options = test_options(MergeCombine::Mean, systems, labels);
    let id = |contributors: &[Sp3ArtifactIdentity], options: &MergeOptions| {
        Sp3MergeInputIdentity::new(contributors, options).expect("merge identity")
    };
    let base = id(&artifacts, &options);
    def("SCHEMA_VERSION", base.schema_version);
    def_str("STABLE_ID", &base.stable_id);
    def(
        "CONTRIBUTOR_COUNT",
        format!("((size_t){})", base.contributors.len()),
    );
    def_bool("PRECEDENCE_PRESENT", base.precedence_contributors.is_some());
    def(
        "PRECEDENCE_COUNT",
        format!(
            "((size_t){})",
            base.precedence_contributors.as_ref().map_or(0, Vec::len)
        ),
    );
    def_str("REVERSED_STABLE_ID", &id(&reversed, &options).stable_id);

    let precedence_options = test_options(MergeCombine::Precedence, systems, labels);
    let precedence = id(&artifacts, &precedence_options);
    def_str("PRECEDENCE_STABLE_ID", &precedence.stable_id);
    def_bool(
        "PRECEDENCE_ID_PRESENT",
        precedence.precedence_contributors.is_some(),
    );
    let precedence_list = precedence
        .precedence_contributors
        .clone()
        .unwrap_or_default();
    def(
        "PRECEDENCE_ID_COUNT",
        format!("((size_t){})", precedence_list.len()),
    );
    def_str(
        "PRECEDENCE_ID_FIRST_PRODUCT_SHA256",
        &precedence_list
            .first()
            .map_or(String::new(), |a| a.product_sha256.clone()),
    );
    def_str(
        "REVERSED_PRECEDENCE_STABLE_ID",
        &id(&reversed, &precedence_options).stable_id,
    );
    def_str(
        "MEDIAN_STABLE_ID",
        &id(
            &artifacts,
            &test_options(MergeCombine::Median, systems, labels),
        )
        .stable_id,
    );
    def_str("SINGLE_STABLE_ID", &id(&artifacts[..1], &options).stable_id);

    let mut negative_zero = options.clone();
    negative_zero.position_tolerance_m = -0.0;
    def_str(
        "NEGATIVE_ZERO_STABLE_ID",
        &id(&artifacts, &negative_zero).stable_id,
    );
    let mut positive_clock = options.clone();
    positive_clock.clock_tolerance_s = 0.0;
    let mut negative_clock = options.clone();
    negative_clock.clock_tolerance_s = -0.0;
    def_str(
        "POSITIVE_ZERO_CLOCK_STABLE_ID",
        &id(&artifacts, &positive_clock).stable_id,
    );
    def_str(
        "NEGATIVE_ZERO_CLOCK_STABLE_ID",
        &id(&artifacts, &negative_clock).stable_id,
    );
    let reordered = test_options(
        MergeCombine::Mean,
        [GnssSystem::Galileo, GnssSystem::Gps],
        [["ITRF2014", "IGS14"], ["ITRF2020", "IGS20"]],
    );
    def_str("REORDERED_STABLE_ID", &id(&artifacts, &reordered).stable_id);

    let mut changed = artifacts.clone();
    changed[1].product_sha256 = "3".repeat(64);
    def_str(
        "CHANGED_DIGEST_STABLE_ID",
        &id(&changed, &options).stable_id,
    );
    let mut changed = artifacts.clone();
    changed[1].resolved_identity.format_version = Some("SP3-c".to_string());
    def_str(
        "CHANGED_VERSION_STABLE_ID",
        &id(&changed, &options).stable_id,
    );
    let mut changed_policy = options.clone();
    changed_policy.clock_tolerance_s = 3.5e-9;
    def_str(
        "CHANGED_POLICY_STABLE_ID",
        &id(&artifacts, &changed_policy).stable_id,
    );

    let mut bad = artifacts.clone();
    bad[1].product_sha256 = "not-a-digest".to_string();
    def_bool(
        "BAD_PRODUCT_DIGEST_REFUSED",
        Sp3MergeInputIdentity::new(&bad, &options).is_err(),
    );
    let mut bad = artifacts.clone();
    bad[1].archive_sha256 = String::new();
    def_bool(
        "EMPTY_ARCHIVE_DIGEST_REFUSED",
        Sp3MergeInputIdentity::new(&bad, &options).is_err(),
    );
    def_bool(
        "NO_CONTRIBUTORS_REFUSED",
        Sp3MergeInputIdentity::new(&[], &options).is_err(),
    );
    println!();
}

fn main_section() {
    println!("/* main: the exact product set and cache-key checks. */");
    let first = identity("cod", Sp3, 2026, 7, 12, None, None);
    let next = identity("cod", Sp3, 2026, 7, 13, None, None);
    def_bool(
        "EXACT_SET_COMPLETE_OK",
        core_data::validate_exact_product_set(
            &[first.clone(), next.clone()],
            &[next.clone(), first.clone()],
        )
        .is_ok(),
    );
    def_bool(
        "EXACT_SET_MISSING_OK",
        core_data::validate_exact_product_set(&[first.clone(), next.clone()], &[next.clone()])
            .is_ok(),
    );
    // The test's inconsistent identity: a 02D span with the filename's span
    // token changed to match.
    let mut inconsistent = first.clone();
    inconsistent.span = "02D".to_string();
    inconsistent.official_filename = inconsistent.official_filename.replacen("_01D_", "_02D_", 1);
    assert_ne!(
        inconsistent.official_filename, first.official_filename,
        "the filename carries a _01D_ span token"
    );
    def_bool("INCONSISTENT_KEY_REFUSED", inconsistent.key().is_err());
    println!();
}

fn main() {
    let guard = "SIDEREON_W3_DATA_DISTRIBUTION_PINS_H";
    header_start("w3_data_distribution", guard);
    println!("#include \"sidereon.h\"");
    println!();
    solution_class_table();
    default_sample_table();
    supported_samples_table();
    content_start_table();
    identity_table();
    location_table();
    merge_section();
    main_section();
    header_end(guard);
}
