//! tests/w5_sp3_exact_pins.h: sidereon-core's results for the products and
//! requests sp3_exact_smoke.c passes (exact loads and validation, the shared
//! terminal-record corpus, a refused SP3 write, a clock-only merge).

#[path = "w5_support/pins.rs"]
mod pins;

use pins::{variant_name, Pins};
use sidereon_core::data::{self as core_data, AnalysisCenter, ProductDate, ProductType};
use sidereon_core::ephemeris::{
    merge, parse_exact_sp3, validate_exact_sp3, ExactSp3Coverage, ExactSp3Request,
    ExactSp3ValidationError, MergeCombine, MergeOptions, MergePrecedenceScope, Sp3,
};
use valgen::{c_string_literals, header_end, header_start, read, read_bytes, tests_path};

const GUARD: &str = "SIDEREON_W5_SP3_EXACT_PINS_H";

fn request(
    date: (i32, u8, u8),
    issue: Option<&str>,
    span: &str,
    sample: &str,
    agency: Option<&str>,
) -> Result<ExactSp3Request, ExactSp3ValidationError> {
    let date = ProductDate::new(date.0, date.1, date.2).expect("product date");
    let request = ExactSp3Request::new(date, issue, span, sample)?;
    match agency {
        Some(agency) => request.with_expected_agency(agency),
        None => Ok(request),
    }
}

fn identity_request(
    center: &str,
    family: ProductType,
    date: (i32, u8, u8),
    sample: Option<&str>,
    issue: Option<&str>,
) -> Result<ExactSp3Request, ExactSp3ValidationError> {
    let product = core_data::product(
        AnalysisCenter::from_code(center).expect("analysis center"),
        family,
        ProductDate::new(date.0, date.1, date.2).expect("product date"),
        sample,
        issue,
    )
    .expect("product");
    ExactSp3Request::from_identity(&product.identity().expect("identity"))
}

fn coverage(p: &Pins, name: &str, value: Option<ExactSp3Coverage>) {
    p.bool(&format!("{name}_OK"), value.is_some());
    p.variant_named(
        &format!("{name}_COVERAGE"),
        "SIDEREON_EXACT_SP3_COVERAGE_",
        &value.map_or("HalfOpen".to_string(), |c| variant_name(&c)),
    );
}

/// The GFZ ultra-rapid product historical_gfz_ultra_sp3 in sp3_exact_smoke.c
/// writes, formatted as its printf calls format it.
fn historical_gfz_ultra_sp3() -> Vec<u8> {
    let mut text = String::new();
    text.push_str(&format!(
        "#dP{:4} {:2} {:2} {:2} {:2} {:11.8} {:7} {:<5}{:>6}{:>4} {}\n",
        2022, 9, 3, 0, 0, 0.0f64, 576, "ORBIT", "IGS20", "FIT", "GFZ"
    ));
    text.push_str(&format!(
        "## {:4} {:15.8} {:>14} {:5} {:.13}\n",
        2225, 518400.0f64, "300.00000000", 59825, 0.0f64
    ));
    text.push_str("+    2   G01G02");
    for _ in 2..17 {
        text.push_str("  0");
    }
    text.push('\n');
    for _ in 1..5 {
        text.push_str("+        ");
        for _ in 0..17 {
            text.push_str("  0");
        }
        text.push('\n');
    }
    for _ in 0..5 {
        text.push_str("++       ");
        for _ in 0..17 {
            text.push_str("  0");
        }
        text.push('\n');
    }
    text.push_str(&c_string_literals(
        &tests_path("sp3_exact_smoke.c"),
        "static const char header_tail[] =",
    ));
    for index in 0..576usize {
        let offset_s = index * 300;
        let day = 3 + offset_s / 86400;
        let second_of_day = offset_s % 86400;
        text.push_str(&format!(
            "*  {:4} {:2} {:2} {:2} {:2} {:11.8}\n\
             PG01  15000.000000 -20000.000000   5000.000000    123.456789\n\
             PG02  16000.000000 -21000.000000   6000.000000    124.456789\n",
            2022,
            9,
            day,
            second_of_day / 3600,
            (second_of_day % 3600) / 60,
            (second_of_day % 60) as f64
        ));
    }
    text.push_str("EOF\n");
    text.into_bytes()
}

fn hex(value: &serde_json::Value) -> Vec<u8> {
    let text = value.as_str().expect("hex field");
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).expect("hex byte"))
        .collect()
}

fn main() {
    header_start("w5_sp3_exact", GUARD);
    let p = Pins::new("W5_SP3_EXACT");

    // Exact load of the GRG product.
    let valid = read_bytes(&tests_path(
        "fixtures/sp3/GRG0MGXFIN_20201760000_01D_15M_ORB.SP3",
    ));
    let grg_request =
        request((2020, 6, 24), None, "01D", "15M", Some("GRGS")).expect("GRG request");
    let (grg, grg_coverage) = parse_exact_sp3(&valid, &grg_request).expect("GRG exact load");
    p.comment("parse_exact_sp3 of the GRG product for 2020-06-24, 01D, 15M, agency GRGS.");
    coverage(&p, "GRG_LOAD", Some(grg_coverage));
    p.int("GRG_EPOCH_COUNT", grg.epoch_count() as i128);
    p.int(
        "GRG_DECLARED_EPOCH_COUNT",
        i128::from(grg.declared_epoch_count()),
    );
    let start = grg.declared_start_j2000_s();
    p.bool("GRG_DECLARED_START_PRESENT", start.is_some());
    p.f64("GRG_DECLARED_START_J2000_S_BITS", start.unwrap_or(0.0));
    p.f64(
        "GRG_FIRST_EPOCH_J2000_S_BITS",
        grg.epochs_j2000_seconds()[0],
    );
    coverage(
        &p,
        "GRG_VALIDATE",
        validate_exact_sp3(&grg, &grg_request).ok(),
    );

    // The shared terminal-record corpus on the GRG prefix.
    let corpus: serde_json::Value =
        serde_json::from_str(&read(&tests_path("fixtures/sp3-terminal-record-v1.json")))
            .expect("terminal-record corpus");
    let prefix = &valid[..valid.len() - 4];
    let cases = corpus["cases"].as_array().expect("cases");
    p.comment("parse_exact_sp3 of the GRG prefix with each terminal record of\nfixtures/sp3-terminal-record-v1.json, in corpus order: success, coverage,\nand the refusal text the binding appends to its message.");
    p.int("TERMINAL_CASE_COUNT", cases.len() as i128);
    let mut names = Vec::new();
    let mut ok = Vec::new();
    let mut half_open = Vec::new();
    let mut errors = Vec::new();
    for case in cases {
        let mut candidate = prefix.to_vec();
        candidate.extend(hex(&case["leading_hex"]));
        if let Some(marker) = case["marker"].as_str() {
            candidate.extend(marker.as_bytes());
        }
        candidate.extend(
            std::iter::repeat(b' ')
                .take(case["padding_spaces"].as_u64().expect("padding_spaces") as usize),
        );
        candidate.extend(hex(&case["suffix_hex"]));
        candidate.extend(hex(&case["separator_hex"]));
        candidate.extend(hex(&case["trailing_hex"]));
        names.push(case["name"].as_str().expect("name").to_string());
        match parse_exact_sp3(&candidate, &grg_request) {
            Ok((_, c)) => {
                ok.push(1);
                half_open.push(i128::from(c == ExactSp3Coverage::HalfOpen));
                errors.push(String::new());
            }
            Err(err) => {
                ok.push(0);
                half_open.push(0);
                errors.push(err.to_string());
            }
        }
    }
    println!(
        "static const char *const W5_SP3_EXACT_TERMINAL_NAMES[{}] = {{ {} }};",
        names.len(),
        names
            .iter()
            .map(|n| valgen::c_string(n))
            .collect::<Vec<_>>()
            .join(", ")
    );
    p.ints("TERMINAL_OK", "bool", &ok);
    p.ints("TERMINAL_HALF_OPEN", "bool", &half_open);
    println!(
        "static const char *const W5_SP3_EXACT_TERMINAL_ERRORS[{}] = {{ {} }};",
        errors.len(),
        errors
            .iter()
            .map(|n| valgen::c_string(n))
            .collect::<Vec<_>>()
            .join(", ")
    );

    // Request construction.
    p.comment("ExactSp3Request construction from literal tokens and from catalog\nidentities.");
    p.outcome(
        "REQUEST_24H",
        &request((2020, 6, 24), None, "01D", "24H", None),
    );
    p.outcome(
        "REQUEST_07D",
        &request((2020, 6, 24), None, "07D", "15M", None),
    );
    p.outcome(
        "REQUEST_IGS_SP3_IDENTITY",
        &identity_request("igs", ProductType::Sp3, (2022, 11, 27), None, None),
    );
    p.outcome(
        "REQUEST_IGS_NAV_IDENTITY",
        &identity_request("igs", ProductType::Nav, (2022, 11, 27), None, None),
    );

    // Historical GFZ ultra-rapid product.
    let historical = historical_gfz_ultra_sp3();
    let gfz_identity = identity_request(
        "gfz_ult",
        ProductType::Sp3,
        (2022, 9, 4),
        Some("05M"),
        Some("0000"),
    )
    .expect("GFZ identity request");
    coverage(
        &p,
        "GFZ_IDENTITY_LOAD",
        parse_exact_sp3(&historical, &gfz_identity)
            .ok()
            .map(|r| r.1),
    );
    let gfz_literal = request((2022, 9, 4), Some("0000"), "02D", "05M", Some("GFZ"))
        .expect("GFZ literal request");
    let literal = parse_exact_sp3(&historical, &gfz_literal);
    p.outcome("GFZ_LITERAL_LOAD", &literal);

    // Permissive load of the truncated IGS product.
    let truncated = Sp3::parse(&read_bytes(&tests_path(
        "fixtures/sp3/IGS0OPSFIN_20261200945_02H30M_15M_ORB.SP3",
    )))
    .expect("truncated IGS product");
    p.comment("Sp3::parse of the truncated IGS product, then validate_exact_sp3 for\n2026-04-30, 01D, 15M, agency IGS.");
    p.int("TRUNCATED_EPOCH_COUNT", truncated.epoch_count() as i128);
    p.int(
        "TRUNCATED_DECLARED_EPOCH_COUNT",
        i128::from(truncated.declared_epoch_count()),
    );
    let igs = request((2026, 4, 30), None, "01D", "15M", Some("IGS")).expect("IGS request");
    p.outcome("TRUNCATED_VALIDATE", &validate_exact_sp3(&truncated, &igs));

    // A text that is not an SP3 product.
    let plain = request((2020, 6, 24), None, "01D", "15M", None).expect("request");
    p.outcome(
        "MALFORMED_LOAD",
        &parse_exact_sp3(b"not an SP3 product", &plain),
    );

    // The writer refusal.
    let fine = Sp3::parse(
        c_string_literals(
            &tests_path("sp3_exact_smoke.c"),
            "static const char FINE_BASE_SP3[] =",
        )
        .as_bytes(),
    )
    .expect("fine-base SP3");
    let written = fine.to_sp3_string();
    p.comment("Sp3::to_sp3_string of FINE_BASE_SP3.");
    p.outcome("FINE_BASE_WRITE", &written);
    match written {
        Err(sidereon_core::ephemeris::Sp3WriteError::PrecisionNotRepresentable {
            field,
            columns,
            decimals,
            value,
        }) => {
            p.str("FINE_BASE_FIELD", field);
            p.int("FINE_BASE_COLUMNS", columns as i128);
            p.int("FINE_BASE_DECIMALS", decimals as i128);
            p.f64("FINE_BASE_NUMBER_BITS", value);
        }
        other => panic!("fine-base write outcome {other:?}"),
    }

    // Clock-only merge.
    let clock_only = Sp3::parse(
        c_string_literals(
            &tests_path("sp3_exact_smoke.c"),
            "static const char CLOCK_ONLY_SP3[] =",
        )
        .as_bytes(),
    )
    .expect("clock-only SP3");
    let mut options = MergeOptions::default();
    options.min_agree = 1;
    options.clock_min_common = 1;
    options.combine = MergeCombine::Mean;
    options.precedence_scope = MergePrecedenceScope::Cell;
    let (_, report) = merge(&[clock_only.clone(), clock_only], &options).expect("merge");
    let agreement = report.per_epoch_agreement();
    p.comment("merge of CLOCK_ONLY_SP3 with itself, min_agree 1, clock_min_common 1.");
    p.int("MERGE_EPOCH_AGREEMENT_COUNT", agreement.len() as i128);
    let a = &agreement[0];
    p.int("MERGE_SATELLITES", a.satellites as i128);
    p.bool("MERGE_POSITION_RMS_PRESENT", a.position_rms_m.is_some());
    p.bool("MERGE_POSITION_MAX_PRESENT", a.position_max_m.is_some());
    p.bool("MERGE_CLOCK_RMS_PRESENT", a.clock_rms_s.is_some());
    p.f64("MERGE_CLOCK_RMS_S_BITS", a.clock_rms_s.unwrap_or(f64::NAN));
    p.bool("MERGE_CLOCK_MAX_PRESENT", a.clock_max_s.is_some());
    p.f64("MERGE_CLOCK_MAX_S_BITS", a.clock_max_s.unwrap_or(f64::NAN));
    p.bool(
        "SUMMARY_POSITION_RMS_PRESENT",
        report.position_agreement_rms_m().is_some(),
    );
    p.bool(
        "SUMMARY_POSITION_MAX_PRESENT",
        report.position_agreement_max_m().is_some(),
    );
    p.bool(
        "SUMMARY_CLOCK_RMS_PRESENT",
        report.clock_agreement_rms_s().is_some(),
    );
    p.f64(
        "SUMMARY_CLOCK_RMS_S_BITS",
        report.clock_agreement_rms_s().unwrap_or(f64::NAN),
    );
    p.bool(
        "SUMMARY_CLOCK_MAX_PRESENT",
        report.clock_agreement_max_s().is_some(),
    );
    p.f64(
        "SUMMARY_CLOCK_MAX_S_BITS",
        report.clock_agreement_max_s().unwrap_or(f64::NAN),
    );

    header_end(GUARD);
}
