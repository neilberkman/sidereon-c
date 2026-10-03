//! publication_resilience_smoke.c: sidereon-core's cross-line predicted IONEX
//! candidates, newest published issues from the recorded AIUB listing, the
//! listing URLs and the WUM NRT solution class, written as the JSON text the
//! C routes form from them (bindings/c/src/data_distribution.rs).

#[path = "../w6_shared.rs"]
mod shared;

use shared::*;
use sidereon_core::data::{self as core_data, AnalysisCenter, ProductDate, ProductType};
use valgen::tests_path;

const SOURCE: &str = "publication_resilience_smoke.c";

fn center(code: &str) -> AnalysisCenter {
    AnalysisCenter::from_code(code).expect("known analysis center")
}

/// sidereon_data_newest_published_product_json's text for one line.
fn newest_json(code: &str, listing: &str) -> String {
    let objects = core_data::parse_archive_listing(listing).expect("recorded listing parses");
    let newest = core_data::newest_published_product(center(code), ProductType::Ionex, &objects)
        .expect("newest published product");
    match newest {
        None => "null".to_string(),
        Some(product) => serde_json::to_string(&serde_json::json!({
            "date": format!(
                "{:04}-{:02}-{:02}",
                product.date.year, product.date.month, product.date.day
            ),
            "issue": product.issue,
            "filename": product.filename,
            "observed_at": product.observed_at,
        }))
        .expect("serialize newest product"),
    }
}

fn main() {
    let guard = "SIDEREON_W6_PUBLICATION_PINS_H";
    valgen::header_start("w6_publication", guard);
    let source = tests_path(SOURCE);
    let listing = c_literal(&source, "static const char AIUB_LISTING[] =");
    let error_page = c_literal(&source, "static const char ERROR_PAGE_LISTING[] =");

    // sidereon_data_predicted_ionex_line_candidates_json(2026, 9, 23, NULL).
    let date = ProductDate::new(2026, 9, 23).expect("map date");
    let candidates = core_data::predicted_ionex_line_candidates(date, None).expect("candidates");
    let rows: Vec<serde_json::Value> = candidates
        .iter()
        .map(|candidate| {
            serde_json::json!({
                "center": candidate.center.code(),
                "date": format!(
                    "{:04}-{:02}-{:02}",
                    candidate.date.year, candidate.date.month, candidate.date.day
                ),
                "sample": candidate.sample,
                "issue": candidate.issue,
                "filename": candidate.canonical_filename().expect("filename"),
                "url": candidate.archive_url().expect("url"),
            })
        })
        .collect();
    comment("predicted_ionex_line_candidates(2026-09-23, no sample), as JSON.");
    def_str(
        "W6_PUBLICATION_CANDIDATES_JSON",
        &serde_json::to_string(&rows).expect("serialize candidates"),
    );

    comment("newest_published_product over the recorded listing, per line, as JSON.");
    def_str(
        "W6_PUBLICATION_NEWEST_COD_PRD1_JSON",
        &newest_json("cod_prd1", &listing),
    );
    def_str(
        "W6_PUBLICATION_NEWEST_COD_PRD2_JSON",
        &newest_json("cod_prd2", &listing),
    );

    comment("parse_archive_listing refuses the error page (true) or reads it (false).");
    def_bool(
        "W6_PUBLICATION_ERROR_PAGE_REFUSED",
        core_data::parse_archive_listing(&error_page).is_err(),
    );

    let urls = core_data::publication_listing_urls(
        center("gfz_ult"),
        ProductType::Sp3,
        ProductDate::new(2026, 8, 4).expect("date"),
    )
    .expect("listing URLs");
    comment("publication_listing_urls(gfz_ult, SP3, 2026-08-04), as JSON.");
    def_str(
        "W6_PUBLICATION_LISTING_URLS_JSON",
        &serde_json::to_string(&urls).expect("serialize URLs"),
    );

    let class = core_data::product_solution_class(center("wum_nrt"), ProductType::Sp3)
        .expect("wum_nrt solution class");
    comment("product_solution_class(wum_nrt, SP3).");
    def(
        "W6_PUBLICATION_WUM_NRT_SOLUTION_CLASS",
        c_enum("SIDEREON_SOLUTION_CLASS", &class),
    );
    valgen::header_end(guard);
}
