// smoke.c exercise_ionex_rms_presence_distinction,
// exercise_ionex_owned_mapping_code_lifetime and
// exercise_ionex_undeclared_default_header: sidereon-core's reading of the
// 2x2x2 sample products those checks build (fill_ionex_sample_grid), its
// slant-delay rows under the single-layer and declared policies, and the
// MAPPING FUNCTION record its writer emits and its reader reads back.
//
// The products are built as src/ionex.rs builds them from the C samples:
// whole-second UTC map epochs, the flat values nested map by latitude by
// longitude with a false presence flag as an absent node, and, with no header
// supplied, a header declaring no mapping function.

#[path = "smoke_a_support/mod.rs"]
mod support;

use sidereon_core::astro::time::civil::split_julian_date_from_j2000_seconds;
use sidereon_core::astro::time::{Instant, JulianDateSplit, TimeScale};
use sidereon_core::atmosphere::ionosphere::{
    ionex_slant_delay_results, Ionex, IonexCoveragePolicy, IonexHeader, IonexMappingDeclaration,
    IonexMappingFunction, IonexMappingPolicy, IonexMissingNodePolicy, IonexSlantDelayEvaluation,
    IonexSlantPolicy, IonexSlantRefusal, IonexSlantRequest, TecGridSamples,
};
use sidereon_core::frame::Wgs84Geodetic;
use support::*;
use valgen::{header_end, header_start};

const GUARD: &str = "SIDEREON_SMOKE_A_IONEX_PINS_H";
const DEG: f64 = std::f64::consts::PI / 180.0;

/// src/ionex.rs undeclared_ionex_header: the header a NULL header stands for.
fn undeclared_header() -> IonexHeader {
    let mut header = IonexHeader::new(IonexMappingFunction::CosZ);
    header.mapping_function = None;
    header
}

fn epoch(seconds: i64) -> Instant {
    let (jd_whole, fraction) = split_julian_date_from_j2000_seconds(seconds);
    Instant::from_julian_date(
        TimeScale::Utc,
        JulianDateSplit::new(jd_whole, fraction).expect("split Julian date"),
    )
}

/// smoke.c fill_ionex_sample_grid with every TEC node 10 TECU and present,
/// the RMS maps left out (`rms` None) or given, and `header` or none.
fn samples(rms: Option<[Option<f64>; 8]>, header: Option<IonexHeader>) -> TecGridSamples {
    let nest = |flat: [Option<f64>; 8]| -> Vec<Vec<Vec<Option<f64>>>> {
        (0..2)
            .map(|map| {
                (0..2)
                    .map(|lat| (0..2).map(|lon| flat[(map * 2 + lat) * 2 + lon]).collect())
                    .collect()
            })
            .collect()
    };
    TecGridSamples {
        map_epochs: vec![epoch(0), epoch(3600)],
        lat_nodes_deg: vec![40.0, -40.0],
        lon_nodes_deg: vec![-20.0, 20.0],
        dlat_deg: -80.0,
        dlon_deg: 40.0,
        shell_height_km: 450.0,
        base_radius_km: 6371.0,
        exponent: -1,
        tec_maps: nest([Some(10.0); 8]),
        rms_maps: rms.map(nest).unwrap_or_default(),
        height_maps: Vec::new(),
        header: header.unwrap_or_else(undeclared_header),
    }
}

/// smoke.c's one request: receiver at 0, 0, azimuth 0, elevation 85 degrees,
/// J2000 second 1800, the L1 frequency.
fn request() -> IonexSlantRequest {
    IonexSlantRequest::new(
        Wgs84Geodetic::new(0.0 * DEG, 0.0 * DEG, 0.0).expect("receiver"),
        85.0 * DEG,
        0.0 * DEG,
        epoch(1800),
        1_575_420_000.0,
    )
}

fn policy(mapping: IonexMappingPolicy) -> IonexSlantPolicy {
    IonexSlantPolicy::default()
        .with_coverage(IonexCoveragePolicy::Strict)
        .with_missing_nodes(IonexMissingNodePolicy::Strict)
        .with_mapping(mapping)
}

fn row(
    ionex: &Ionex,
    mapping: IonexMappingPolicy,
) -> sidereon_core::Result<IonexSlantDelayEvaluation> {
    ionex_slant_delay_results(ionex, &[request()], policy(mapping))
        .into_iter()
        .next()
        .expect("one row")
}

fn mapping_function_name(function: &IonexMappingFunction) -> String {
    variant_name(function)
}

/// One slant row: success, delay bits and status, or the engine error kind
/// with, for a mapping-function refusal, the declaration it names.
fn row_pins(prefix: &str, result: &sidereon_core::Result<IonexSlantDelayEvaluation>) {
    match result {
        Ok(evaluation) => {
            def_bool(&format!("{prefix}_OK"), true);
            def_bits(&format!("{prefix}_DELAY_BITS"), evaluation.delay_m);
            def_bool(&format!("{prefix}_IS_VALID"), evaluation.status.is_valid());
            def_bool(
                &format!("{prefix}_HAS_HELD"),
                evaluation.status.held.is_some(),
            );
            def_bool(
                &format!("{prefix}_HAS_DEGRADED"),
                evaluation.status.degraded.is_some(),
            );
            def_bool(
                &format!("{prefix}_HAS_ASSUMED_MAPPING"),
                evaluation.status.assumed_mapping.is_some(),
            );
            def_str(
                &format!("{prefix}_ASSUMED_MAPPING"),
                &evaluation
                    .status
                    .assumed_mapping
                    .as_ref()
                    .map(variant_name)
                    .unwrap_or_default(),
            );
            def_str(&format!("{prefix}_ERROR"), "");
            def_str(&format!("{prefix}_REFUSAL"), "");
            def_str(&format!("{prefix}_DECLARATION"), "");
            def_str(&format!("{prefix}_DECLARED_FUNCTION"), "");
        }
        Err(err) => {
            def_bool(&format!("{prefix}_OK"), false);
            def_bits(&format!("{prefix}_DELAY_BITS"), f64::NAN);
            def_bool(&format!("{prefix}_IS_VALID"), false);
            def_bool(&format!("{prefix}_HAS_HELD"), false);
            def_bool(&format!("{prefix}_HAS_DEGRADED"), false);
            def_bool(&format!("{prefix}_HAS_ASSUMED_MAPPING"), false);
            def_str(&format!("{prefix}_ASSUMED_MAPPING"), "");
            def_str(&format!("{prefix}_ERROR"), &variant_name(err));
            let (refusal, declaration, function) = match err {
                sidereon_core::Error::IonexSlantUnavailable(refusal) => match refusal {
                    IonexSlantRefusal::MappingFunction(IonexMappingDeclaration::Declared(f)) => (
                        "MappingFunction".to_string(),
                        "Declared".to_string(),
                        mapping_function_name(f),
                    ),
                    IonexSlantRefusal::MappingFunction(IonexMappingDeclaration::Absent) => (
                        "MappingFunction".to_string(),
                        "Absent".to_string(),
                        String::new(),
                    ),
                    other => (variant_name(other), String::new(), String::new()),
                },
                _ => (String::new(), String::new(), String::new()),
            };
            def_str(&format!("{prefix}_REFUSAL"), &refusal);
            def_str(&format!("{prefix}_DECLARATION"), &declaration);
            def_str(&format!("{prefix}_DECLARED_FUNCTION"), &function);
        }
    }
}

/// smoke.c ionex_mapping_field: the trimmed 60-column data of the MAPPING
/// FUNCTION record, or None when the text has no such record.
fn mapping_field(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let bytes = line.as_bytes();
        (bytes.len() > 60 && line[60..].starts_with("MAPPING FUNCTION"))
            .then(|| line[..60].trim_matches(' ').to_string())
    })
}

/// A product's MAPPING FUNCTION declaration: whether one is present, and its
/// kind and code when it is.
fn declaration_pins(prefix: &str, header: &IonexHeader) {
    def_bool(
        &format!("{prefix}_DECLARED"),
        header.mapping_function.is_some(),
    );
    def_str(
        &format!("{prefix}_FUNCTION"),
        &header
            .mapping_function
            .as_ref()
            .map(mapping_function_name)
            .unwrap_or_default(),
    );
    def_str(
        &format!("{prefix}_CODE"),
        header
            .mapping_function
            .as_ref()
            .map(|f| f.code())
            .unwrap_or(""),
    );
}

fn text_pins(prefix: &str, ionex: &Ionex) {
    let text = ionex.to_ionex_string().expect("IONEX text");
    let field = mapping_field(&text);
    def_bool(&format!("{prefix}_MAPPING_RECORD"), field.is_some());
    def_str(
        &format!("{prefix}_MAPPING_FIELD"),
        field.as_deref().unwrap_or(""),
    );
    let reparsed = Ionex::parse(text.as_bytes()).expect("reparse IONEX text");
    declaration_pins(&format!("{prefix}_REPARSED"), reparsed.header());
}

fn main() {
    header_start("smoke_a_ionex", GUARD);

    comment("exercise_ionex_rms_presence_distinction: RMS maps left out, then all absent.");
    let without = Ionex::from_samples(samples(None, None)).expect("product without RMS");
    let without_samples = without.tec_grid_samples();
    def_bool(
        "SMOKE_A_IONEX_NO_RMS_HAS_RMS_MAPS",
        !without_samples.rms_maps.is_empty(),
    );
    let flat = |maps: &[Vec<Vec<Option<f64>>>]| -> Vec<Option<f64>> {
        maps.iter().flatten().flatten().copied().collect()
    };
    def_usize(
        "SMOKE_A_IONEX_NO_RMS_RMS_VALUE_COUNT",
        flat(&without_samples.rms_maps).len(),
    );
    let with = Ionex::from_samples(samples(Some([None; 8]), None)).expect("product with RMS");
    let with_samples = with.tec_grid_samples();
    def_bool(
        "SMOKE_A_IONEX_MISSING_RMS_HAS_RMS_MAPS",
        !with_samples.rms_maps.is_empty(),
    );
    let with_rms = flat(&with_samples.rms_maps);
    def_usize("SMOKE_A_IONEX_MISSING_RMS_RMS_VALUE_COUNT", with_rms.len());
    let presence: Vec<String> = with_rms
        .iter()
        .map(|v| if v.is_some() { "true" } else { "false" }.to_string())
        .collect();
    println!(
        "static const bool SMOKE_A_IONEX_MISSING_RMS_PRESENCE[{}] = {{ {} }};",
        presence.len().max(1),
        if presence.is_empty() {
            "false".to_string()
        } else {
            presence.join(", ")
        }
    );

    comment("exercise_ionex_owned_mapping_code_lifetime: a header declaring SLAB_3D_CUSTOM.");
    // sidereon_ionex_header_new, then set_mapping_function(OTHER, code).
    let mut custom_header = undeclared_header();
    custom_header.mapping_function =
        Some(IonexMappingFunction::Other("SLAB_3D_CUSTOM".to_string()));
    let custom =
        Ionex::from_samples(samples(None, Some(custom_header))).expect("custom-code product");
    row_pins(
        "SMOKE_A_IONEX_CUSTOM_ROW",
        &row(&custom, IonexMappingPolicy::SingleLayer),
    );

    comment("exercise_ionex_undeclared_default_header: no header, then a declared COSZ.");
    let bare = Ionex::from_samples(samples(None, None)).expect("bare product");
    declaration_pins("SMOKE_A_IONEX_BARE_HEADER", bare.header());
    row_pins(
        "SMOKE_A_IONEX_BARE_SINGLE_LAYER",
        &row(&bare, IonexMappingPolicy::SingleLayer),
    );
    row_pins(
        "SMOKE_A_IONEX_BARE_DECLARED",
        &row(&bare, IonexMappingPolicy::Declared),
    );
    text_pins("SMOKE_A_IONEX_BARE_TEXT", &bare);
    let mut cosz_header = undeclared_header();
    cosz_header.mapping_function = Some(IonexMappingFunction::CosZ);
    let declared = Ionex::from_samples(samples(None, Some(cosz_header))).expect("COSZ product");
    row_pins(
        "SMOKE_A_IONEX_COSZ_SINGLE_LAYER",
        &row(&declared, IonexMappingPolicy::SingleLayer),
    );
    row_pins(
        "SMOKE_A_IONEX_COSZ_DECLARED",
        &row(&declared, IonexMappingPolicy::Declared),
    );
    text_pins("SMOKE_A_IONEX_COSZ_TEXT", &declared);

    header_end(GUARD);
}
