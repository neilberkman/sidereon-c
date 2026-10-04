//! smoke.c exercise_constellation_surface: the time-aware NAVCEN assessments
//! of the committed forecast fixture, the merged GPS catalog's records, the
//! SP3-id builder, and the CelesTrak-only catalog's validation, as
//! sidereon-core computes them.

#[path = "../smoke_b_shared.rs"]
mod shared;

use shared::*;
use sidereon_core::astro::passes::UtcInstant;
use sidereon_core::constellation::{
    from_celestrak_omm, gnss_sp3_id, is_valid, merge_navcen, merge_navcen_at, parse_navcen,
    parse_navcen_at, validate_against_sp3_ids, NavcenTiming, Record,
};
use sidereon_core::GnssSystem;
use valgen::{header_end, header_start, read, read_bytes, tests_path};

const BIN: &str = "smoke_b_constellation";
const GUARD: &str = "SIDEREON_SMOKE_B_CONSTELLATION_PINS_H";
const P: &str = "SMOKE_B_CONSTELLATION";

/// The instants the test evaluates the forecast fixture at: one minute before
/// the PRN 7 forecast outage it states (JDAY 205/0115 - 205/1315 of 2026),
/// its start, and its end.
const EVAL_BEFORE_US: i64 = 1_784_855_640_000_000;
const EVAL_START_US: i64 = 1_784_855_700_000_000;
const EVAL_END_US: i64 = 1_784_898_900_000_000;

/// The one-row NAVCEN table the test merges into the CelesTrak catalog: PRN 19
/// with the same forecast interval.
const MERGE_FORECAST_HTML: &str = concat!(
    "<table><tr>",
    "<td class=\"views-field-field-gps-prn\">19</td>",
    "<td class=\"views-field-field-gps-svn\">59</td>",
    "<td class=\"views-field-field-gps-con-block-type\">IIR</td>",
    "<td class=\"views-field-field-nanu-outage-start-date\">24 JUL 2026</td>",
    "<td class=\"views-field-field-nanu-type\">FCSTDV</td>",
    "<td class=\"views-field-field-nanu-subject\">",
    "SVN59 (PRN19) FORECAST OUTAGE JDAY 205/0115 - JDAY 205/1315</td>",
    "<td class=\"nanu-active-check\">1</td>",
    "</tr></table>"
);

/// The SP3 ids the CelesTrak-only catalog is validated against.
const ALL_IDS: [&str; 4] = ["G03", "G05", "G13", "G19"];

fn timing_c(timing: &NavcenTiming) -> &'static str {
    match timing {
        NavcenTiming::NotApplicable => "SIDEREON_NAVCEN_TIMING_NOT_APPLICABLE",
        NavcenTiming::Parsed(_) => "SIDEREON_NAVCEN_TIMING_PARSED",
        NavcenTiming::Unparseable => "SIDEREON_NAVCEN_TIMING_UNPARSEABLE",
    }
}

fn celestrak_records() -> Vec<Record> {
    let text = read(&tests_path("fixtures/constellation/gps_ops_sample.json"));
    let omms = sidereon_core::astro::omm::parse_json_array(&text).expect("OMM JSON array");
    from_celestrak_omm(GnssSystem::Gps, &omms.omms).expect("CelesTrak records")
}

fn main() {
    let forecast = read_bytes(&tests_path(
        "fixtures/constellation/navcen_forecast_cases.html",
    ));
    let navcen = read_bytes(&tests_path("fixtures/constellation/navcen_gps_sample.html"));
    let base = celestrak_records();

    header_start(BIN, GUARD);
    define_i64(&format!("{P}_EVAL_BEFORE_US"), EVAL_BEFORE_US);
    define_i64(&format!("{P}_EVAL_START_US"), EVAL_START_US);
    define_i64(&format!("{P}_EVAL_END_US"), EVAL_END_US);
    println!();

    // The assessments at the forecast start, row by row in the order the
    // engine returns them.
    let at_start = parse_navcen_at(&forecast, UtcInstant::from_unix_microseconds(EVAL_START_US))
        .expect("NAVCEN assessments at start");
    define_typed(&format!("{P}_ASSESSMENT_COUNT"), "size_t", at_start.len());
    let mut prns = Vec::new();
    let mut usable = Vec::new();
    let mut active = Vec::new();
    let mut timing = Vec::new();
    let mut start_present = Vec::new();
    let mut start_us = Vec::new();
    let mut end_present = Vec::new();
    let mut end_us = Vec::new();
    for a in &at_start {
        prns.push(a.status.prn.to_string());
        usable.push(a.status.usable.to_string());
        active.push(a.status.active_nanu.to_string());
        timing.push(timing_c(&a.timing).to_string());
        let (s, e) = match a.timing {
            NavcenTiming::Parsed(interval) => (
                Some(interval.start_utc.unix_microseconds()),
                Some(interval.end_utc.unix_microseconds()),
            ),
            _ => (None, None),
        };
        start_present.push(s.is_some().to_string());
        start_us.push(format!("INT64_C({})", s.unwrap_or(0)));
        end_present.push(e.is_some().to_string());
        end_us.push(format!("INT64_C({})", e.unwrap_or(0)));
    }
    let array = |name: &str, c_type: &str, items: &[String]| {
        println!(
            "static const {c_type} {P}_{name}[] = {{ {} }};",
            items.join(", ")
        );
    };
    array("ASSESSMENT_PRN", "uint16_t", &prns);
    array("ASSESSMENT_USABLE", "bool", &usable);
    array("ASSESSMENT_ACTIVE_NANU", "bool", &active);
    array("ASSESSMENT_TIMING", "uint32_t", &timing);
    array("ASSESSMENT_START_PRESENT", "bool", &start_present);
    array("ASSESSMENT_START_US", "int64_t", &start_us);
    array("ASSESSMENT_END_PRESENT", "bool", &end_present);
    array("ASSESSMENT_END_US", "int64_t", &end_us);
    // The NANU type text of row 1 (the forecast row), as the engine keeps it.
    let nanu_type = at_start[1].status.nanu_type.clone().unwrap_or_default();
    define_text(&format!("{P}_ROW1_NANU_TYPE"), &nanu_type);
    println!();

    // Row 1's usability one minute before the outage and at its end.
    for (name, us) in [("BEFORE", EVAL_BEFORE_US), ("END", EVAL_END_US)] {
        let rows = parse_navcen_at(&forecast, UtcInstant::from_unix_microseconds(us))
            .expect("NAVCEN assessments");
        define_bool(&format!("{P}_ROW1_USABLE_{name}"), rows[1].status.usable);
    }
    println!();

    // The CelesTrak catalog merged with the one-row forecast table at the
    // outage start and end; record 3 is the one the test reads.
    define_text(&format!("{P}_MERGE_FORECAST_HTML"), MERGE_FORECAST_HTML);
    for (name, us) in [("START", EVAL_START_US), ("END", EVAL_END_US)] {
        let assessments = parse_navcen_at(
            MERGE_FORECAST_HTML.as_bytes(),
            UtcInstant::from_unix_microseconds(us),
        )
        .expect("merge forecast assessments");
        let merged = merge_navcen_at(&base, &assessments);
        define(&format!("{P}_TIMED_RECORD3_PRN_{name}"), merged[3].prn);
        define_bool(
            &format!("{P}_TIMED_RECORD3_USABLE_{name}"),
            merged[3].usable,
        );
    }
    println!();

    // The catalog merged with the NAVCEN status overlay.
    let statuses = parse_navcen(&navcen).expect("NAVCEN overlay");
    let catalog = merge_navcen(&base, &statuses);
    for index in [0usize, 3] {
        let r = &catalog[index];
        println!(
            "#define {P}_RECORD{index}_SYSTEM {}",
            gnss_system_c(r.system)
        );
        define(&format!("{P}_RECORD{index}_PRN"), r.prn);
        define_bool(&format!("{P}_RECORD{index}_ACTIVE"), r.active);
        define_bool(&format!("{P}_RECORD{index}_USABLE"), r.usable);
        define_bool(
            &format!("{P}_RECORD{index}_FDMA_CHANNEL_PRESENT"),
            r.fdma_channel.is_some(),
        );
    }
    println!();

    define_text(
        &format!("{P}_SP3_ID_GPS_5"),
        &gnss_sp3_id(GnssSystem::Gps, 5),
    );
    define_text(
        &format!("{P}_SP3_ID_GLONASS_12"),
        &gnss_sp3_id(GnssSystem::Glonass, 12),
    );
    println!();

    // The CelesTrak-only catalog validated against G03, G05, G13 and G19.
    println!(
        "static const char *const {P}_ALL_IDS[] = {{ {} }};",
        ALL_IDS
            .iter()
            .map(|id| valgen::c_string(id))
            .collect::<Vec<_>>()
            .join(", ")
    );
    define_typed(&format!("{P}_ALL_ID_COUNT"), "size_t", ALL_IDS.len());
    let validation = validate_against_sp3_ids(&base, &ALL_IDS);
    define_bool(&format!("{P}_CELESTRAK_ONLY_VALID"), is_valid(&validation));
    header_end(GUARD);
}
