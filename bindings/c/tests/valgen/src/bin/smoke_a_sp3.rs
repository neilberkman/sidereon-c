// smoke.c exercise_sp3_surface: sidereon-core's prediction summary, parsed
// state flags, merge defaults and single-source, two-source and asserted
// frame-reconciliation merges of the SP3 surface fixture, as the binding
// routes read them.

#[path = "smoke_a_support/mod.rs"]
mod support;

use sidereon_core::astro::time::civil::j2000_seconds_from_split;
use sidereon_core::astro::time::{Instant, InstantRepr};
use sidereon_core::ephemeris::{
    merge, MergeCombine, MergeOptions, MergePrecedenceScope, MergeReport, Sp3, Sp3FrameLabelSet,
    Sp3FrameReconciliationMethod, Sp3FrameReconciliationOptions, Sp3InterpolationOptions,
};
use sidereon_core::GnssSatelliteId;
use support::*;
use valgen::{header_end, header_start, read_bytes, tests_path};

const GUARD: &str = "SIDEREON_SMOKE_A_SP3_PINS_H";

/// The binding's instant_to_j2000_seconds: J2000 seconds of a Julian-date
/// instant, NaN for any other representation.
fn j2000(epoch: &Instant) -> f64 {
    match epoch.repr {
        InstantRepr::JulianDate(jd) => j2000_seconds_from_split(jd.jd_whole, jd.fraction),
        InstantRepr::Nanos(_) => f64::NAN,
    }
}

/// sidereon_sp3_load: Sp3::parse with the default interpolation options.
fn load(bytes: &[u8]) -> Sp3 {
    Sp3::parse(bytes)
        .expect("parse SP3")
        .with_interpolation_options(Sp3InterpolationOptions::default())
}

/// sidereon_sp3_merge_options_init followed by the smoke's min_agree = 1 and
/// clock_min_common = 1, through the binding's sp3_merge_options_from_c.
fn smoke_merge_options(label_sets: Vec<Sp3FrameLabelSet>) -> MergeOptions {
    let defaults = MergeOptions::default();
    let mut options = MergeOptions::default();
    options.position_tolerance_m = defaults.position_tolerance_m;
    options.clock_tolerance_s = defaults.clock_tolerance_s;
    options.min_agree = 1;
    options.clock_min_common = 1;
    options.combine = MergeCombine::Mean;
    options.precedence_scope = MergePrecedenceScope::Cell;
    options.outlier_reject = defaults.outlier_reject;
    options.target_epoch_interval_s = defaults.target_epoch_interval_s;
    options.systems = None;
    let mut frame = Sp3FrameReconciliationOptions::default();
    frame.asserted_equivalent_label_sets = label_sets;
    frame.helmert = false;
    options.frame_reconciliation = frame;
    options
}

fn agreement_pins(prefix: &str, report: &MergeReport) {
    let per_epoch = report.per_epoch_agreement();
    def_usize(&format!("{prefix}_EPOCH_AGREEMENT_COUNT"), per_epoch.len());
    let first = per_epoch.first().expect("epoch agreement row");
    def_bits(
        &format!("{prefix}_AGREEMENT0_EPOCH_BITS"),
        j2000(&first.epoch),
    );
    def_usize(&format!("{prefix}_AGREEMENT0_SATELLITES"), first.satellites);
    let optional = |name: &str, value: Option<f64>| {
        def_bool(&format!("{prefix}_{name}_PRESENT"), value.is_some());
        def_bits(&format!("{prefix}_{name}_BITS"), value.unwrap_or(f64::NAN));
    };
    optional("AGREEMENT0_POSITION_RMS", first.position_rms_m);
    optional("AGREEMENT0_POSITION_MAX", first.position_max_m);
    optional("AGREEMENT0_CLOCK_RMS", first.clock_rms_s);
    optional("AGREEMENT0_CLOCK_MAX", first.clock_max_s);
    optional("SUMMARY_POSITION_RMS", report.position_agreement_rms_m());
    optional("SUMMARY_POSITION_MAX", report.position_agreement_max_m());
    optional("SUMMARY_CLOCK_RMS", report.clock_agreement_rms_s());
    optional("SUMMARY_CLOCK_MAX", report.clock_agreement_max_s());
}

fn main() {
    let path = tests_path("fixtures/sp3/IGS0OPSFIN_20261200945_02H30M_15M_ORB.SP3");
    let sp3 = load(&read_bytes(&path));

    header_start("smoke_a_sp3", GUARD);

    comment("sidereon_sp3_prediction_summary and sidereon_sp3_epoch_prediction(0).");
    let summary = sp3.prediction_summary();
    def_usize("SMOKE_A_SP3_PREDICTION_EPOCH_COUNT", summary.epochs.len());
    let observed_through = summary.observed_through.as_ref().map(j2000);
    def_bool(
        "SMOKE_A_SP3_OBSERVED_THROUGH_PRESENT",
        observed_through.is_some(),
    );
    def_bits(
        "SMOKE_A_SP3_OBSERVED_THROUGH_BITS",
        observed_through.unwrap_or(0.0),
    );
    let epoch0 = summary.epochs.first().expect("first epoch");
    def_bool("SMOKE_A_SP3_EPOCH0_OBSERVED", epoch0.is_observed());
    def_usize(
        "SMOKE_A_SP3_EPOCH0_ORBIT_PREDICTED_COUNT",
        epoch0.orbit_predicted_satellites.len(),
    );
    def_usize(
        "SMOKE_A_SP3_EPOCH0_CLOCK_PREDICTED_COUNT",
        epoch0.clock_predicted_satellites.len(),
    );

    comment("sidereon_sp3_state(G01, 0): the optional fields the smoke reads.");
    let g01: GnssSatelliteId = "G01".parse().expect("G01");
    let state = sp3.state(g01, 0).expect("G01 epoch 0 state");
    def_bool("SMOKE_A_SP3_G01_E0_HAS_VELOCITY", state.velocity.is_some());
    def_bool(
        "SMOKE_A_SP3_G01_E0_HAS_CLOCK_RATE",
        state.clock_rate_s_s.is_some(),
    );

    comment("MergeOptions::default(), which sidereon_sp3_merge_options_init reports.");
    let defaults = MergeOptions::default();
    def_bool(
        "SMOKE_A_SP3_MERGE_DEFAULT_SCOPE_IS_CELL",
        defaults.precedence_scope == MergePrecedenceScope::Cell,
    );
    def_bool(
        "SMOKE_A_SP3_MERGE_DEFAULT_OUTLIER_REJECT",
        defaults.outlier_reject.is_some(),
    );

    comment("merge([sp3]) under the smoke's options.");
    let (merged, report) =
        merge(std::slice::from_ref(&sp3), &smoke_merge_options(Vec::new())).expect("merge one");
    def_usize(
        "SMOKE_A_SP3_SINGLE_MERGED_EPOCH_COUNT",
        merged.epoch_count(),
    );
    def_usize("SMOKE_A_SP3_SINGLE_QUARANTINED", report.quarantined.len());
    def_usize(
        "SMOKE_A_SP3_SINGLE_SINGLE_SOURCE",
        report.single_source.len(),
    );
    def_usize(
        "SMOKE_A_SP3_SINGLE_POSITION_OUTLIERS",
        report.position_outliers.len(),
    );
    def_usize(
        "SMOKE_A_SP3_SINGLE_CLOCK_OUTLIERS",
        report.clock_outliers.len(),
    );
    let flag = report.single_source.first().expect("single-source flag");
    def_bits("SMOKE_A_SP3_SINGLE_FLAG0_EPOCH_BITS", j2000(&flag.epoch));
    def_str(
        "SMOKE_A_SP3_SINGLE_FLAG0_SAT_ID",
        &flag.satellite.to_string(),
    );
    def_usize("SMOKE_A_SP3_SINGLE_FLAG0_SOURCE_COUNT", flag.sources.len());
    def_usize(
        "SMOKE_A_SP3_SINGLE_FLAG0_SOURCE0",
        *flag.sources.first().expect("flag source"),
    );
    agreement_pins("SMOKE_A_SP3_SINGLE", &report);

    comment("merge([sp3, sp3]) under the smoke's options.");
    let (_, report2) = merge(
        &[sp3.clone(), sp3.clone()],
        &smoke_merge_options(Vec::new()),
    )
    .expect("merge two");
    agreement_pins("SMOKE_A_SP3_DOUBLE", &report2);

    comment("The asserted frame reconciliation of the two inline one-record products.");
    let smoke = tests_path("smoke.c");
    let frame_a = valgen::c_string_literals(&smoke, "const char *frame_a_text =");
    let frame_b = valgen::c_string_literals(&smoke, "const char *frame_b_text =");
    let sources = [load(frame_a.as_bytes()), load(frame_b.as_bytes())];
    let label_sets = vec![Sp3FrameLabelSet::new(["IGS14", "ITRF2"])];
    let (_, frame_report) = merge(&sources, &smoke_merge_options(label_sets)).expect("frame merge");
    def_usize(
        "SMOKE_A_SP3_FRAME_RECONCILIATION_COUNT",
        frame_report.frame_reconciliations.len(),
    );
    let row = frame_report
        .frame_reconciliations
        .first()
        .expect("reconciliation row");
    def_bool(
        "SMOKE_A_SP3_FRAME_METHOD_IS_ASSERTED",
        row.method == Sp3FrameReconciliationMethod::AssertedEquivalence,
    );
    def_usize("SMOKE_A_SP3_FRAME_SOURCE_INDEX", row.source_index);
    let asserted = row.asserted_label_set.clone().unwrap_or_default();
    def_usize("SMOKE_A_SP3_FRAME_ASSERTED_LABEL_COUNT", asserted.len());
    def_usize("SMOKE_A_SP3_FRAME_RECORDS_AFFECTED", row.records_affected);
    def_bool(
        "SMOKE_A_SP3_FRAME_PARAMETERS_PRESENT",
        row.parameters.is_some(),
    );
    def_str("SMOKE_A_SP3_FRAME_SOURCE_LABEL", &row.source_label);
    def_str("SMOKE_A_SP3_FRAME_TARGET_LABEL", &row.target_label);
    def_str(
        "SMOKE_A_SP3_FRAME_ASSERTED_LABEL0",
        asserted.first().map(String::as_str).unwrap_or(""),
    );

    header_end(GUARD);
}
