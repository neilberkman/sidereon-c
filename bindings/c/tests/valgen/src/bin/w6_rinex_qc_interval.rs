//! rinex_qc_interval_smoke.c: sidereon-core's lint findings, observation QC
//! summaries and JSON, and repaired INTERVAL records for the ESBC trim file
//! with the INTERVAL record set to 0.000 and to -1.000, and for its first
//! epoch alone, as the test builds those texts.

#[path = "../w6_shared.rs"]
mod shared;

use shared::*;
use sidereon_core::observation_qc::{
    observation_qc_with_options, IntervalSource, ObservationQcOptions,
};
use sidereon_core::rinex::observations::RinexObs;
use sidereon_core::rinex::qc::{lint_obs_text, repair_obs_text, RepairOptions, Severity};
use valgen::{read, tests_path};

/// The code bindings/c/src/rinex.rs rinex_qc_severity_to_c gives a severity
/// (SidereonRinexQcSeverity, a u32 field in the header).
fn severity_code(severity: Severity) -> u32 {
    match severity {
        Severity::Fatal => 0,
        Severity::Error => 1,
        Severity::Warning => 2,
        Severity::Info => 3,
    }
}

/// The code bindings/c/src/observation.rs qc_interval_source_to_c gives an
/// interval source (SidereonObservationQcIntervalSource, a u32 field).
fn interval_source_code(source: IntervalSource) -> u32 {
    match source {
        IntervalSource::Override => 0,
        IntervalSource::Header => 1,
        IntervalSource::Inferred => 2,
        IntervalSource::Unresolved => 3,
    }
}

const VALID: &str = "    30.000                                                  INTERVAL";

/// The test's copy_with_interval_record.
fn with_interval(source: &str, replacement: &str) -> String {
    assert_eq!(replacement.len(), VALID.len());
    let at = source.find(VALID).expect("INTERVAL record");
    format!(
        "{}{}{}",
        &source[..at],
        replacement,
        &source[at + VALID.len()..]
    )
}

/// The test's one_epoch_length: up to and including the newline that ends
/// the first epoch's last data row.
fn one_epoch(text: &str) -> &str {
    let bytes = text.as_bytes();
    let mut first = None;
    for i in 0..bytes.len().saturating_sub(2) {
        if bytes[i] == b'\n' && bytes[i + 1] == b'>' && bytes[i + 2] == b' ' {
            if first.is_none() {
                first = Some(i);
            } else {
                return &text[..i + 1];
            }
        }
    }
    panic!("two observation epochs");
}

fn lint(label: &str, text: &str, code: &str) {
    let report = lint_obs_text(text);
    let finding = report.findings.iter().find(|f| f.code() == code);
    def_bool(
        &format!("W6_QCI_{label}_HAS_{}", code.replace('-', "_")),
        finding.is_some(),
    );
    // An absent finding pins severity 0 and not repairable; the test reads
    // them only when the finding is present.
    def(
        &format!("W6_QCI_{label}_{}_SEVERITY", code.replace('-', "_")),
        finding.map_or(0, |f| severity_code(f.severity())),
    );
    def_bool(
        &format!("W6_QCI_{label}_{}_REPAIRABLE", code.replace('-', "_")),
        finding.is_some_and(|f| f.is_repairable()),
    );
}

fn qc(label: &str, text: &str, with_json: bool) {
    let obs = RinexObs::parse(text).expect("observation text parses");
    let report = observation_qc_with_options(&obs, ObservationQcOptions::default()).expect("QC");
    def_bool(
        &format!("W6_QCI_{label}_HAS_INTERVAL"),
        report.interval_s.is_some(),
    );
    def_bits(
        &format!("W6_QCI_{label}_INTERVAL_S_BITS"),
        report.interval_s.unwrap_or(0.0),
    );
    def(
        &format!("W6_QCI_{label}_INTERVAL_SOURCE"),
        interval_source_code(report.interval_source),
    );
    def_size(&format!("W6_QCI_{label}_NOTE_COUNT"), report.notes.len());
    if with_json {
        def_str(
            &format!("W6_QCI_{label}_QC_JSON"),
            &serde_json::to_string(&report).expect("QC JSON"),
        );
    }
}

/// The repaired text's INTERVAL record, or an empty string when the repair
/// leaves none, and the repaired text itself.
fn repair(label: &str, text: &str, set_interval: bool) -> String {
    let mut options = RepairOptions::default();
    options.set_interval = set_interval;
    let repaired = repair_obs_text(text, &options)
        .expect("repair")
        .repaired
        .to_rinex_string()
        .expect("repaired text");
    let line = repaired
        .lines()
        .find(|line| line.contains("INTERVAL"))
        .unwrap_or("");
    def_str(&format!("W6_QCI_{label}_INTERVAL_RECORD"), line);
    repaired
}

fn main() {
    let guard = "SIDEREON_W6_RINEX_QC_INTERVAL_PINS_H";
    valgen::header_start("w6_rinex_qc_interval", guard);
    let source = read(&tests_path(
        "fixtures/obs/ESBC00DNK_R_20201770000_01D_30S_MO_trim.rnx",
    ));
    let unavailable = with_interval(
        &source,
        "     0.000                                                  INTERVAL",
    );
    let invalid = with_interval(
        &source,
        "    -1.000                                                  INTERVAL",
    );
    let single = one_epoch(&unavailable).to_string();

    comment("INTERVAL 0.000: lint, default QC, and a refused 0 s override.");
    lint("UNAVAILABLE", &unavailable, "OBS-H19");
    qc("UNAVAILABLE", &unavailable, true);
    let obs = RinexObs::parse(&unavailable).expect("parse");
    let mut zero_override = ObservationQcOptions::default();
    zero_override.interval_override_s = Some(0.0);
    def_bool(
        "W6_QCI_ZERO_OVERRIDE_REFUSED",
        observation_qc_with_options(&obs, zero_override).is_err(),
    );
    let preserved = repair("UNAVAILABLE_PRESERVED", &unavailable, false);
    lint("UNAVAILABLE_PRESERVED", &preserved, "OBS-H19");
    let repaired = repair("UNAVAILABLE_REPAIRED", &unavailable, true);
    lint("UNAVAILABLE_REPAIRED", &repaired, "OBS-H19");

    comment("INTERVAL -1.000: lint, default QC, and both repairs.");
    lint("INVALID", &invalid, "OBS-H20");
    qc("INVALID", &invalid, true);
    let preserved = repair("INVALID_PRESERVED", &invalid, false);
    lint("INVALID_PRESERVED", &preserved, "OBS-H20");
    let repaired = repair("INVALID_REPAIRED", &invalid, true);
    lint("INVALID_REPAIRED", &repaired, "OBS-H20");

    comment("The first epoch alone with INTERVAL 0.000.");
    qc("SINGLE", &single, false);
    let repaired = repair("SINGLE_REPAIRED", &single, true);
    lint("SINGLE_REPAIRED", &repaired, "OBS-H19");
    valgen::header_end(guard);
}
