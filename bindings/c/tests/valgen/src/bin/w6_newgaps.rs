//! newgaps.c: the lenient OMM feeds the test parses (built here and emitted
//! for it), sidereon-core's lenient catalog of them, the LAMBDA and bounded
//! ILS results for the test's float vectors, and the SP3 merge agreement of a
//! product merged with itself.

#[path = "../w6_shared.rs"]
mod shared;

use shared::*;
use sidereon_core::astro::omm::parse_json_array;
use sidereon_core::constellation::from_celestrak_omm_lenient;
use sidereon_core::ephemeris::{merge, MergeOptions, Sp3};
use sidereon_core::ils::{bounded_ils_search, lambda_ils_search, IlsResult};
use sidereon_core::GnssSystem;
use valgen::{read_bytes, tests_path};

const SOURCE: &str = "newgaps.c";

/// One CelesTrak OMM JSON object with the required mean-element fields.
fn omm_obj(name: &str, norad: &str) -> String {
    format!(
        "{{\"OBJECT_NAME\":\"{name}\",\"EPOCH\":\"2020-06-25T00:00:00.000000\",\
         \"MEAN_MOTION\":2.0056,\"ECCENTRICITY\":0.0001,\"INCLINATION\":55.0,\
         \"RA_OF_ASC_NODE\":100.0,\"ARG_OF_PERICENTER\":50.0,\"MEAN_ANOMALY\":10.0,\
         \"NORAD_CAT_ID\":{norad},\"BSTAR\":0.0,\"MEAN_MOTION_DOT\":0.0,\
         \"MEAN_MOTION_DDOT\":0.0}}"
    )
}

fn catalog(label: &str, feed: &str) {
    def_str(&format!("W6_NEWGAPS_{label}_FEED"), feed);
    let array = parse_json_array(feed).expect("OMM JSON array");
    let catalog = from_celestrak_omm_lenient(GnssSystem::Gps, &array.omms);
    def_size(
        &format!("W6_NEWGAPS_{label}_RECORD_COUNT"),
        catalog.records.len(),
    );
    def_size(
        &format!("W6_NEWGAPS_{label}_SKIPPED_COUNT"),
        catalog.skipped.len(),
    );
    def_size(
        &format!("W6_NEWGAPS_{label}_MALFORMED_COUNT"),
        array.skipped.len(),
    );
    for (i, record) in catalog.records.iter().enumerate() {
        def(
            &format!("W6_NEWGAPS_{label}_RECORD{i}_SYSTEM"),
            c_enum("SIDEREON_GNSS_SYSTEM", &record.system),
        );
        def(&format!("W6_NEWGAPS_{label}_RECORD{i}_PRN"), record.prn);
        def(
            &format!("W6_NEWGAPS_{label}_RECORD{i}_NORAD_ID"),
            record.norad_id,
        );
    }
    for (i, skipped) in catalog.skipped.iter().enumerate() {
        def_bool(
            &format!("W6_NEWGAPS_{label}_SKIPPED{i}_NORAD_PRESENT"),
            skipped.norad_id.is_some(),
        );
        def(
            &format!("W6_NEWGAPS_{label}_SKIPPED{i}_NORAD_ID"),
            skipped.norad_id.unwrap_or(0),
        );
        def_bool(
            &format!("W6_NEWGAPS_{label}_SKIPPED{i}_NAME_PRESENT"),
            skipped.object_name.is_some(),
        );
        def_str(
            &format!("W6_NEWGAPS_{label}_SKIPPED{i}_NAME"),
            skipped.object_name.as_deref().unwrap_or(""),
        );
    }
}

fn ils(label: &str, result: &IlsResult) {
    println!(
        "static const int64_t W6_NEWGAPS_{label}_FIXED[{}] = {{ {} }};",
        result.fixed.len(),
        result
            .fixed
            .iter()
            .map(|v| format!("INT64_C({v})"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    def_bool(
        &format!("W6_NEWGAPS_{label}_FIXED_STATUS"),
        result.fixed_status,
    );
    def_bits(&format!("W6_NEWGAPS_{label}_RATIO_BITS"), result.ratio);
    def_bits(
        &format!("W6_NEWGAPS_{label}_BEST_SCORE_BITS"),
        result.best_score,
    );
    def_bool(
        &format!("W6_NEWGAPS_{label}_SECOND_BEST_PRESENT"),
        result.second_best_score.is_some(),
    );
    def_bits(
        &format!("W6_NEWGAPS_{label}_SECOND_BEST_SCORE_BITS"),
        result.second_best_score.unwrap_or(0.0),
    );
}

fn square(values: &[f64]) -> Vec<Vec<f64>> {
    let n = (values.len() as f64).sqrt() as usize;
    assert_eq!(n * n, values.len());
    values.chunks(n).map(<[f64]>::to_vec).collect()
}

fn main() {
    let guard = "SIDEREON_W6_NEWGAPS_PINS_H";
    valgen::header_start("w6_newgaps", guard);
    let source = tests_path(SOURCE);

    comment("Lenient OMM feeds: three objects (two GPS, one QZSS), and a bare number with one GPS object.");
    let feed = format!(
        "[{},{},{}]",
        omm_obj("GPS BIIF-8  (PRN 03)", "40294"),
        omm_obj("GPS BIII-1  (PRN 04)", "43873"),
        omm_obj("QZS-2 (QZSS/PRN 194)", "42738")
    );
    catalog("CLEAN", &feed);
    let bad_feed = format!("[42,{}]", omm_obj("GPS BIIF-8  (PRN 03)", "40294"));
    catalog("MALFORMED", &bad_feed);

    comment("lambda_ils_search of the RTKLIB utest1 floats, ratio threshold 3.");
    let a = c_doubles(&source, "const double a[6] =");
    let q = c_doubles(&source, "const double q[36] =");
    ils(
        "LAMBDA",
        &lambda_ils_search(&a, &square(&q), 3.0).expect("LAMBDA"),
    );
    comment("lambda_ils_search refusals: a 6 x 35 covariance, and DBL_MAX.");
    def_bool(
        "W6_NEWGAPS_LAMBDA_DBL_MAX_REFUSED",
        lambda_ils_search(&[f64::MAX], &[vec![1.0]], 3.0).is_err(),
    );
    comment("bounded_ils_search radius 1, limit 200000, ratio threshold 3.");
    let bf = c_doubles(&source, "const double bf[2] =");
    let bcov = c_doubles(&source, "const double bcov[4] =");
    ils(
        "BOUNDED",
        &bounded_ils_search(&bf, &square(&bcov), 1, 200000, 3.0).expect("bounded ILS"),
    );

    comment("The 2026 IGS final SP3 merged with itself, min_agree and clock_min_common 1.");
    let sp3 = Sp3::parse(&read_bytes(&tests_path(
        "fixtures/sp3/IGS0OPSFIN_20261200945_02H30M_15M_ORB.SP3",
    )))
    .expect("SP3");
    let mut options = MergeOptions::default();
    options.min_agree = 1;
    options.clock_min_common = 1;
    let (_, report) = merge(&[sp3.clone(), sp3], &options).expect("merge");
    let epochs = report.per_epoch_agreement();
    def_size("W6_NEWGAPS_AGREEMENT_EPOCH_COUNT", epochs.len());
    def_bool(
        "W6_NEWGAPS_AGREEMENT_FIRST_RMS_PRESENT",
        epochs[0].position_rms_m.is_some(),
    );
    def_bits(
        "W6_NEWGAPS_AGREEMENT_FIRST_RMS_BITS",
        epochs[0].position_rms_m.unwrap_or(f64::NAN),
    );
    let summary = report.position_agreement_rms_m();
    def_bool(
        "W6_NEWGAPS_AGREEMENT_SUMMARY_RMS_PRESENT",
        summary.is_some(),
    );
    def_bits(
        "W6_NEWGAPS_AGREEMENT_SUMMARY_RMS_BITS",
        summary.unwrap_or(f64::NAN),
    );
    valgen::header_end(guard);
}
