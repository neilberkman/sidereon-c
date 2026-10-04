//! window_continuity_smoke.c: sidereon-core's stencil extent, window
//! continuity verdicts and merge-report verdict on the COD final SP3 with the
//! G01 seam jump the test injects, and the next nominal igs_ult issue, as the
//! JSON the C routes write (bindings/c/src/sp3.rs, data_distribution.rs).

#[path = "../w6_shared.rs"]
mod shared;

use shared::*;
use sidereon_core::data::{
    self as core_data, AnalysisCenter, ProductDate, ProductDateTime, ProductIdentity, ProductType,
};
use sidereon_core::ephemeris::{
    check_continuity, merge, ContinuityOptions, EpochWindow, MergeOptions, OrbitClass, Sp3,
    Sp3InterpolationOptions, SpeedBound, StencilExtent,
};
use valgen::{read_bytes, tests_path};

/// The test's inject_g01_seam_jump: every PG01 record from the 146th epoch on
/// has its X field moved by 3000 km, rewritten as `%14.6f`.
fn inject_g01_seam_jump(bytes: &[u8]) -> Vec<u8> {
    let text = std::str::from_utf8(bytes).expect("UTF-8 SP3");
    let mut out = String::with_capacity(text.len());
    let mut epoch_index: i64 = -1;
    let mut shifted = 0usize;
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        if line.len() >= 2 && line.starts_with("* ") {
            epoch_index += 1;
            out.push_str(line);
        } else if epoch_index >= 145 && line.len() >= 18 && line.starts_with("PG01") {
            let field = &line[4..18];
            let x_km: f64 = field.trim().parse().expect("PG01 X field");
            let replacement = format!("{:14.6}", x_km + 3000.0);
            assert_eq!(replacement.len(), 14, "replacement width");
            out.push_str(&line[..4]);
            out.push_str(&replacement);
            out.push_str(&line[18..]);
            shifted += 1;
        } else {
            out.push_str(line);
        }
    }
    assert!(shifted > 0, "seam injected");
    out.into_bytes()
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

fn main() {
    let guard = "SIDEREON_W6_WINDOW_CONTINUITY_PINS_H";
    valgen::header_start("w6_window_continuity", guard);
    let bytes = inject_g01_seam_jump(&read_bytes(&tests_path(
        "fixtures/sp3/COD0MGXFIN_20201770000_01D_05M_ORB.SP3",
    )));
    let sp3 = Sp3::parse(&bytes).expect("seam-injected SP3 parses");
    let epochs = sp3.epochs_j2000_seconds();
    let seam = epochs[144];
    let stencil = StencilExtent::for_sp3(&sp3).expect("stencil extent");
    comment("StencilExtent::for_sp3 of the seam-injected product.");
    def_bits("W6_WINDOW_STENCIL_BEFORE_S_BITS", stencil.before_s());
    def_bits("W6_WINDOW_STENCIL_AFTER_S_BITS", stencil.after_s());
    let after_s = stencil.after_s();

    // sidereon_sp3_continuity_verdict_json(sp3, 0, -1.0, from, through):
    // MEO speed bound, no residual tolerance, default interpolation.
    let options = ContinuityOptions::new(Some(SpeedBound::OrbitClass(OrbitClass::MeoGnss)), None)
        .expect("valid MEO continuity options")
        .with_interpolation_options(Sp3InterpolationOptions::default());
    let report = check_continuity(&sp3.precise_ephemeris_samples(), &options)
        .expect("valid MEO continuity options");
    let windows = [
        ("INSIDE_DAY", epochs[24], epochs[72]),
        ("STRADDLING", seam - 600.0, seam + 600.0),
        ("STENCIL_BOUNDARY", seam - 7200.0, seam - after_s),
        ("OUTSIDE_STENCIL", seam - 7200.0, seam - after_s - 0.001),
    ];
    for (name, from, through) in windows {
        let window = EpochWindow::new(from, through).expect("window");
        let stencil = StencilExtent::for_sp3(&sp3).expect("stencil extent");
        let json = window_continuity_verdict_json(report.verdict_for_window(window, stencil));
        comment(&format!("Continuity verdict JSON for the {name} window."));
        def_str(
            &format!("W6_WINDOW_VERDICT_{name}_JSON"),
            &serde_json::to_string(&json).expect("serialize verdict"),
        );
    }

    // sidereon_sp3_merge of the product alone with min_agree and
    // clock_min_common 1, then its report's verdict over epochs 24..72.
    let mut merge_options = MergeOptions::default();
    merge_options.min_agree = 1;
    merge_options.clock_min_common = 1;
    let (_merged, merge_report) = merge(&[sp3.clone()], &merge_options).expect("merge");
    let window = EpochWindow::new(epochs[24], epochs[72]).expect("window");
    let merged_json = merge_report
        .continuity_verdict_for_window(window)
        .map(window_continuity_verdict_json)
        .unwrap_or(serde_json::Value::Null);
    comment("The merge report's continuity verdict JSON over epochs 24..72.");
    def_str(
        "W6_WINDOW_MERGE_VERDICT_JSON",
        &serde_json::to_string(&merged_json).expect("serialize merge verdict"),
    );

    // sidereon_data_next_issue_due_json(igs_ult, SP3, 2026-08-04 02:59:59).
    let now =
        ProductDateTime::new(ProductDate::new(2026, 8, 4).expect("date"), 2, 59, 59).expect("now");
    let issue = core_data::next_issue_due(
        AnalysisCenter::from_code("igs_ult").expect("igs_ult"),
        ProductType::Sp3,
        now,
    )
    .expect("next issue");
    let value = serde_json::json!({
        "identity": product_identity_json(&issue.identity),
        "due_at": issue.due_at.to_string(),
        "covers": {
            "observed": nominal_coverage_interval_json(issue.covers.observed),
            "predicted": nominal_coverage_interval_json(issue.covers.predicted),
        },
    });
    comment("next_issue_due(igs_ult, SP3, 2026-08-04T02:59:59), as JSON.");
    def_str(
        "W6_WINDOW_NEXT_ISSUE_JSON",
        &serde_json::to_string(&value).expect("serialize next issue"),
    );

    let wum_now =
        ProductDateTime::new(ProductDate::new(2026, 8, 4).expect("date"), 0, 0, 0).expect("now");
    comment("next_issue_due(wum_nrt, SP3, 2026-08-04T00:00:00) is refused (true).");
    def_bool(
        "W6_WINDOW_WUM_NRT_NEXT_ISSUE_REFUSED",
        core_data::next_issue_due(
            AnalysisCenter::from_code("wum_nrt").expect("wum_nrt"),
            ProductType::Sp3,
            wum_now,
        )
        .is_err(),
    );
    valgen::header_end(guard);
}
