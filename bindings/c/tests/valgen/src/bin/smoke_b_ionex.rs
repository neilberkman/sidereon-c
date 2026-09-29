//! smoke.c exercise_iono_surface, exercise_ionex_3_0_surface and the IONEX
//! helpers they call: every value those checks compare against, as
//! sidereon-core computes it for the same inputs. Slant evaluations are
//! emitted as SmokeBSlantPin records holding the whole C view of the
//! outcome: status, delay bits, status flags and the typed error detail.

#[path = "../smoke_b_shared.rs"]
mod shared;

use shared::*;
use sidereon_core::astro::time::{
    split_julian_date_from_j2000_seconds, Instant, JulianDateSplit, TimeScale,
};
use sidereon_core::atmosphere::ionosphere::{
    ionex_slant_delay_results, ionex_slant_delay_with_policy, klobuchar_native, Ionex,
    IonexAssumedMapping, IonexCoverageError, IonexCoveragePolicy, IonexHeader,
    IonexMappingDeclaration, IonexMappingFunction, IonexMappingPolicy, IonexMissingNodePolicy,
    IonexMissingNodes, IonexNodeGap, IonexSlantDelayEvaluation, IonexSlantPolicy,
    IonexSlantRefusal, IonexSlantRequest, IonexWarning, KlobucharParams, TecGrid, TecGridEpoch,
    TecGridError, TecGridSamples, TecSamplesError,
};
use sidereon_core::frame::Wgs84Geodetic;
use sidereon_core::Error;
use valgen::{bits, c_string, header_end, header_start, read, read_bytes, tests_path};

const BIN: &str = "smoke_b_ionex";
const GUARD: &str = "SIDEREON_SMOKE_B_IONEX_PINS_H";
const P: &str = "SMOKE_B_IONEX";
const DEG: f64 = std::f64::consts::PI / 180.0;
const F_L1: f64 = 1_575_420_000.0;

// ---- the C view of a slant outcome ---------------------------------------

fn missing_c(nodes: Option<IonexMissingNodes>) -> String {
    match nodes {
        Some(n) => format!(
            "{{ true, {}, {}, {}, {}, {{ {}, {}, {}, {} }} }}",
            n.map_number,
            n.lat_index,
            n.lon_index,
            n.lon_index_next,
            n.missing[0],
            n.missing[1],
            n.missing[2],
            n.missing[3]
        ),
        None => "{ false, 0, 0, 0, 0, { false, false, false, false } }".to_string(),
    }
}

fn gap_c(gap: Option<IonexNodeGap>) -> String {
    match gap {
        Some(g) => format!(
            "{{ {}, {}, {} }}",
            g.earlier.is_some() || g.later.is_some(),
            missing_c(g.earlier),
            missing_c(g.later)
        ),
        None => format!("{{ false, {}, {} }}", missing_c(None), missing_c(None)),
    }
}

fn coverage_c(error: Option<IonexCoverageError>) -> String {
    match error {
        Some(e) => c_enum("SIDEREON_IONEX_COVERAGE_ERROR_KIND_", &e),
        None => "SIDEREON_IONEX_COVERAGE_ERROR_KIND_NONE".to_string(),
    }
}

fn assumed_c(assumed: Option<IonexAssumedMapping>) -> String {
    match assumed {
        Some(a) => c_enum("SIDEREON_IONEX_ASSUMED_MAPPING_KIND_", &a),
        None => "SIDEREON_IONEX_ASSUMED_MAPPING_KIND_NONE".to_string(),
    }
}

fn mapping_function_c(function: &IonexMappingFunction) -> String {
    match function {
        IonexMappingFunction::NoMapping => "SIDEREON_IONEX_MAPPING_FUNCTION_KIND_NO_MAPPING",
        IonexMappingFunction::CosZ => "SIDEREON_IONEX_MAPPING_FUNCTION_KIND_COS_Z",
        IonexMappingFunction::QFactor => "SIDEREON_IONEX_MAPPING_FUNCTION_KIND_Q_FACTOR",
        IonexMappingFunction::Other(_) => "SIDEREON_IONEX_MAPPING_FUNCTION_KIND_OTHER",
    }
    .to_string()
}

/// The status src/ionex.rs gives an engine failure (ionex_error_status and
/// map_iono_error agree).
fn status_c(err: &Error) -> &'static str {
    match err {
        Error::InvalidInput(_) => "SIDEREON_STATUS_INVALID_ARGUMENT",
        Error::Ut1OutsideCoverage(_) => "SIDEREON_STATUS_UT1_OUTSIDE_COVERAGE",
        _ => "SIDEREON_STATUS_SOLVE",
    }
}

/// The typed error detail src/ionex.rs ionex_slant_error_to_c writes, as the
/// tail of a SmokeBSlantPin initializer.
fn error_c(err: Option<&Error>) -> String {
    let none_refusal = "SIDEREON_IONEX_SLANT_REFUSAL_KIND_NONE, 0, 0, 0, false, \
                        SIDEREON_IONEX_MAPPING_DECLARATION_KIND_DECLARED, false, \
                        SIDEREON_IONEX_MAPPING_FUNCTION_KIND_NO_MAPPING";
    let empty_gap = gap_c(None);
    match err {
        None => format!(
            "SIDEREON_IONEX_SLANT_ERROR_KIND_NONE, SIDEREON_IONEX_COVERAGE_ERROR_KIND_NONE, \
             false, {empty_gap}, {none_refusal}"
        ),
        Some(Error::InvalidInput(_)) => format!(
            "SIDEREON_IONEX_SLANT_ERROR_KIND_INVALID_INPUT, \
             SIDEREON_IONEX_COVERAGE_ERROR_KIND_NONE, false, {empty_gap}, {none_refusal}"
        ),
        Some(Error::IonexOutOfCoverage(e)) => format!(
            "SIDEREON_IONEX_SLANT_ERROR_KIND_OUT_OF_COVERAGE, {}, false, {empty_gap}, \
             {none_refusal}",
            coverage_c(Some(*e))
        ),
        Some(Error::IonexNodesNotAvailable(gap)) => format!(
            "SIDEREON_IONEX_SLANT_ERROR_KIND_NODES_NOT_AVAILABLE, \
             SIDEREON_IONEX_COVERAGE_ERROR_KIND_NONE, true, {}, {none_refusal}",
            gap_c(Some(**gap))
        ),
        Some(Error::IonexSlantUnavailable(refusal)) => {
            let head = format!(
                "SIDEREON_IONEX_SLANT_ERROR_KIND_SLANT_UNAVAILABLE, \
                 SIDEREON_IONEX_COVERAGE_ERROR_KIND_NONE, false, {empty_gap}"
            );
            let tail = match refusal {
                IonexSlantRefusal::VaryingHeights {
                    map_number,
                    lat_index,
                    lon_index,
                } => format!(
                    "SIDEREON_IONEX_SLANT_REFUSAL_KIND_VARYING_HEIGHTS, {map_number}, \
                     {lat_index}, {lon_index}, false, \
                     SIDEREON_IONEX_MAPPING_DECLARATION_KIND_DECLARED, false, \
                     SIDEREON_IONEX_MAPPING_FUNCTION_KIND_NO_MAPPING"
                ),
                IonexSlantRefusal::HeightNotAvailable {
                    map_number,
                    lat_index,
                    lon_index,
                } => format!(
                    "SIDEREON_IONEX_SLANT_REFUSAL_KIND_HEIGHT_NOT_AVAILABLE, {map_number}, \
                     {lat_index}, {lon_index}, false, \
                     SIDEREON_IONEX_MAPPING_DECLARATION_KIND_DECLARED, false, \
                     SIDEREON_IONEX_MAPPING_FUNCTION_KIND_NO_MAPPING"
                ),
                IonexSlantRefusal::MappingFunction(declaration) => match declaration {
                    IonexMappingDeclaration::Declared(function) => format!(
                        "SIDEREON_IONEX_SLANT_REFUSAL_KIND_MAPPING_FUNCTION, 0, 0, 0, true, \
                         SIDEREON_IONEX_MAPPING_DECLARATION_KIND_DECLARED, true, {}",
                        mapping_function_c(function)
                    ),
                    IonexMappingDeclaration::Absent => {
                        "SIDEREON_IONEX_SLANT_REFUSAL_KIND_MAPPING_FUNCTION, 0, 0, 0, true, \
                         SIDEREON_IONEX_MAPPING_DECLARATION_KIND_ABSENT, false, \
                         SIDEREON_IONEX_MAPPING_FUNCTION_KIND_NO_MAPPING"
                            .to_string()
                    }
                },
                other => panic!("IONEX refusal {other:?} has no C detail in this generator"),
            };
            format!("{head}, {tail}")
        }
        Some(other) => panic!("IONEX failure {other:?} has no C detail in this generator"),
    }
}

/// A SmokeBSlantPin initializer for one slant outcome.
fn slant_pin(result: &Result<IonexSlantDelayEvaluation, Error>) -> String {
    match result {
        Ok(e) => format!(
            "{{ true, SIDEREON_STATUS_OK, {}, {}, {}, {}, {}, {}, {}, {}, {} }}",
            bits(e.delay_m),
            e.status.is_valid(),
            e.status.held.is_some(),
            coverage_c(e.status.held),
            e.status.degraded.is_some(),
            gap_c(e.status.degraded),
            e.status.assumed_mapping.is_some(),
            assumed_c(e.status.assumed_mapping),
            error_c(None)
        ),
        Err(err) => format!(
            "{{ false, {}, UINT64_C(0), false, false, SIDEREON_IONEX_COVERAGE_ERROR_KIND_NONE, \
             false, {}, false, SIDEREON_IONEX_ASSUMED_MAPPING_KIND_NONE, {} }}",
            status_c(err),
            gap_c(None),
            error_c(Some(err))
        ),
    }
}

fn define_slant(name: &str, result: &Result<IonexSlantDelayEvaluation, Error>) {
    println!(
        "static const SmokeBSlantPin {P}_{name} = {};",
        slant_pin(result)
    );
}

fn print_pin_types() {
    println!("/* A missing-node block, a node gap and a slant outcome, field for field");
    println!(" * as the C view reports them. */");
    println!("typedef struct {{");
    println!("    bool has_missing;");
    println!("    size_t map_number;");
    println!("    size_t lat_index;");
    println!("    size_t lon_index;");
    println!("    size_t lon_index_next;");
    println!("    bool missing[4];");
    println!("}} SmokeBMissingPin;");
    println!("typedef struct {{");
    println!("    bool has_gap;");
    println!("    SmokeBMissingPin earlier;");
    println!("    SmokeBMissingPin later;");
    println!("}} SmokeBGapPin;");
    println!("typedef struct {{");
    println!("    bool ok;");
    println!("    SidereonStatus status;");
    println!("    uint64_t delay_bits;");
    println!("    bool is_valid;");
    println!("    bool has_held;");
    println!("    SidereonIonexCoverageErrorKind coverage_error;");
    println!("    bool has_degraded;");
    println!("    SmokeBGapPin gap;");
    println!("    bool has_assumed_mapping;");
    println!("    SidereonIonexAssumedMappingKind assumed_mapping;");
    println!("    SidereonIonexSlantErrorKind error_kind;");
    println!("    SidereonIonexCoverageErrorKind error_coverage;");
    println!("    bool error_has_gap;");
    println!("    SmokeBGapPin error_gap;");
    println!("    SidereonIonexSlantRefusalKind refusal;");
    println!("    size_t refusal_map_number;");
    println!("    size_t refusal_lat_index;");
    println!("    size_t refusal_lon_index;");
    println!("    bool has_mapping_declaration;");
    println!("    SidereonIonexMappingDeclarationKind mapping_declaration;");
    println!("    bool has_mapping_function;");
    println!("    SidereonIonexMappingFunctionKind mapping_function;");
    println!("}} SmokeBSlantPin;");
    println!();
}

// ---- the routes, as src/ionex.rs reaches the engine -----------------------

fn policy(
    coverage: IonexCoveragePolicy,
    missing: IonexMissingNodePolicy,
    mapping: IonexMappingPolicy,
) -> IonexSlantPolicy {
    IonexSlantPolicy::default()
        .with_coverage(coverage)
        .with_missing_nodes(missing)
        .with_mapping(mapping)
}

/// sidereon_ionex_slant_delay_with_policy: the receiver is formed from
/// degrees first, and a refusal there reads as an invalid input.
fn slant_with_policy(
    ionex: &Ionex,
    lat_deg: f64,
    lon_deg: f64,
    az_deg: f64,
    el_deg: f64,
    epoch_s: i64,
    frequency_hz: f64,
    policy: IonexSlantPolicy,
) -> Result<IonexSlantDelayEvaluation, Error> {
    let receiver = Wgs84Geodetic::new(lat_deg * DEG, lon_deg * DEG, 0.0)
        .map_err(|err| Error::InvalidInput(format!("receiver: {err}")))?;
    ionex_slant_delay_with_policy(
        ionex,
        receiver,
        el_deg * DEG,
        az_deg * DEG,
        map_epoch(epoch_s),
        frequency_hz,
        policy,
    )
}

/// src/ionex.rs ionex_epoch_from_whole_second.
fn map_epoch(seconds: i64) -> Instant {
    let (jd_whole, fraction) = split_julian_date_from_j2000_seconds(seconds);
    match JulianDateSplit::new(jd_whole, fraction) {
        Ok(split) => Instant::from_julian_date(TimeScale::Utc, split),
        Err(_) => Instant::from_nanos(TimeScale::Utc, i128::from(seconds) * 1_000_000_000),
    }
}

fn nest(flat: &[f64], present: &[bool]) -> Vec<Vec<Vec<Option<f64>>>> {
    (0..2)
        .map(|m| {
            (0..2)
                .map(|la| {
                    (0..2)
                        .map(|lo| {
                            let i = (m * 2 + la) * 2 + lo;
                            present[i].then_some(flat[i])
                        })
                        .collect()
                })
                .collect()
        })
        .collect()
}

/// sidereon_ionex_header_new: a header declaring no mapping function.
fn undeclared_header() -> IonexHeader {
    let mut header = IonexHeader::new(IonexMappingFunction::CosZ);
    header.mapping_function = None;
    header
}

/// smoke.c fill_ionex_sample_grid's 2x2x2 product: maps at the given whole
/// seconds (UTC), latitudes 40 and -40, longitudes -20 and 20, shell 450 km,
/// base radius 6371 km, exponent -1, no RMS map.
fn grid_product(
    epochs_s: [i64; 2],
    tec: &[f64; 8],
    tec_present: &[bool; 8],
    heights: Option<(&[f64; 8], &[bool; 8])>,
    header: IonexHeader,
) -> Result<Ionex, TecSamplesError> {
    Ionex::from_samples(TecGridSamples {
        map_epochs: epochs_s.iter().map(|s| map_epoch(*s)).collect(),
        lat_nodes_deg: vec![40.0, -40.0],
        lon_nodes_deg: vec![-20.0, 20.0],
        dlat_deg: -80.0,
        dlon_deg: 40.0,
        shell_height_km: 450.0,
        base_radius_km: 6371.0,
        exponent: -1,
        tec_maps: nest(tec, tec_present),
        rms_maps: Vec::new(),
        height_maps: heights.map_or_else(Vec::new, |(h, p)| nest(h, p)),
        header,
    })
}

// ---- golden inputs ---------------------------------------------------------

/// A Python float.hex() string as the core golden JSON states it.
fn hex_float(text: &str) -> f64 {
    let (negative, body) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let value = match body {
        "inf" => f64::INFINITY,
        "nan" => f64::NAN,
        _ => {
            let body = body.strip_prefix("0x").expect("hex float prefix");
            let (mantissa, exponent) = body.split_once('p').expect("hex float exponent");
            let (int_part, frac_part) = mantissa.split_once('.').unwrap_or((mantissa, ""));
            let mut significand: u64 = u64::from_str_radix(int_part, 16).expect("hex int");
            let mut shift: i32 = 0;
            for digit in frac_part.chars() {
                significand = significand * 16 + digit.to_digit(16).expect("hex digit") as u64;
                shift -= 4;
            }
            let exponent: i32 = exponent.parse().expect("hex float exponent value");
            // The significand holds at most 53 bits for these values, so the
            // product is exact.
            (significand as f64) * 2f64.powi(exponent + shift)
        }
    };
    if negative {
        -value
    } else {
        value
    }
}

struct IonexCase {
    lat: f64,
    lon: f64,
    az: f64,
    el: f64,
    epoch_s: i64,
    frequency_hz: f64,
}

fn ionex_cases() -> Vec<IonexCase> {
    let golden: serde_json::Value = serde_json::from_str(&read(&format!(
        "{}/ionex_golden.json",
        valgen::core_fixtures()
    )))
    .expect("ionex golden JSON");
    golden["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .map(|case| {
            let i = &case["inputs"];
            let f = |key: &str| hex_float(i[key].as_str().expect("hex float input"));
            IonexCase {
                lat: f("lat_deg"),
                lon: f("lon_deg"),
                az: f("az_deg"),
                el: f("el_deg"),
                epoch_s: i["epoch_s"].as_i64().expect("epoch_s"),
                frequency_hz: f("frequency_hz"),
            }
        })
        .collect()
}

// ---- sections --------------------------------------------------------------

fn iono_surface(ionex: &Ionex) {
    // Klobuchar with zero coefficients at a second-of-day of 1e9.
    let f_l1 = header_f64s("iono_fixture.h", "KLOB_F_L1_HZ_BITS")[0];
    let params = KlobucharParams {
        alpha: [0.0; 4],
        beta: [0.0; 4],
    };
    let klob = klobuchar_native(&params, 0.0, 0.0, 0.0, 45.0, 1.0e9, f_l1);
    define_bool(&format!("{P}_KLOB_T1E9_OK"), klob.is_ok());
    if let Err(err) = &klob {
        println!("#define {P}_KLOB_T1E9_STATUS {}", status_c(err));
    }

    define_typed(
        &format!("{P}_EPOCH_COUNT"),
        "size_t",
        ionex.map_epochs().len(),
    );
    println!("#define {P}_EXPONENT {}", ionex.exponent());
    define_typed(
        &format!("{P}_LAT_NODE_COUNT"),
        "size_t",
        ionex.lat_nodes_deg().len(),
    );
    define_typed(
        &format!("{P}_LON_NODE_COUNT"),
        "size_t",
        ionex.lon_nodes_deg().len(),
    );

    let cases = ionex_cases();
    let strict = IonexSlantPolicy::default();
    let hold = IonexSlantPolicy::default().with_coverage(IonexCoveragePolicy::Hold);
    let mut default_pins = Vec::new();
    let mut hold_pins = Vec::new();
    for c in &cases {
        let run = |p| {
            slant_with_policy(
                ionex,
                c.lat,
                c.lon,
                c.az,
                c.el,
                c.epoch_s,
                c.frequency_hz,
                p,
            )
        };
        default_pins.push(slant_pin(&run(strict)));
        hold_pins.push(slant_pin(&run(hold)));
    }
    println!("/* Every IONEX golden case under the strict default policy and under the");
    println!(" * hold coverage policy. */");
    println!(
        "static const SmokeBSlantPin {P}_CASE_DEFAULT[] = {{\n    {}\n}};",
        default_pins.join(",\n    ")
    );
    println!(
        "static const SmokeBSlantPin {P}_CASE_HOLD[] = {{\n    {}\n}};",
        hold_pins.join(",\n    ")
    );
    println!();
}

fn warnings_and_header() {
    let bytes = read_bytes(&tests_path("fixtures/ionex/synthetic_2map_7x7.20i"));
    let (ionex, warnings) = Ionex::parse_with_warnings(&bytes).expect("IONEX with warnings");
    define_typed(&format!("{P}_WARNING_COUNT"), "size_t", warnings.len());
    let kinds: Vec<String> = warnings
        .iter()
        .map(|w| c_enum("SIDEREON_IONEX_WARNING_KIND_", w))
        .collect();
    println!(
        "static const SidereonIonexWarningKind {P}_WARNING_KIND[] = {{ {} }};",
        if kinds.is_empty() {
            "0".to_string()
        } else {
            kinds.join(", ")
        }
    );
    let labels: Vec<String> = warnings
        .iter()
        .map(|w| {
            c_string(match w {
                IonexWarning::MissingRecord(label) => label,
                IonexWarning::VersionRecordNotFirst { .. } => "IONEX VERSION / TYPE",
                IonexWarning::EpochMismatch { label, .. } => label,
                IonexWarning::MapCountMismatch { .. } => "# OF MAPS IN FILE",
                IonexWarning::NotANumberValue { kind, .. } => kind,
                IonexWarning::IntervalMismatch { .. } => "INTERVAL",
                IonexWarning::ExponentCarriedIntoMap { kind, .. } => kind,
                _ => "",
            })
        })
        .collect();
    println!(
        "static const char *const {P}_WARNING_LABEL[] = {{ {} }};",
        if labels.is_empty() {
            "\"\"".to_string()
        } else {
            labels.join(", ")
        }
    );
    let messages: Vec<String> = warnings.iter().map(|w| c_string(&w.to_string())).collect();
    println!(
        "static const char *const {P}_WARNING_MESSAGE[] = {{ {} }};",
        if messages.is_empty() {
            "\"\"".to_string()
        } else {
            messages.join(", ")
        }
    );
    let garbage = Ionex::parse_with_warnings(b"not an IONEX file");
    define_bool(&format!("{P}_GARBAGE_OK"), garbage.is_ok());

    let header = ionex.header();
    define_bits(&format!("{P}_HEADER_VERSION_BITS"), header.version);
    let (declaration, function, code) = match &header.mapping_function {
        Some(f) => (
            "SIDEREON_IONEX_MAPPING_DECLARATION_KIND_DECLARED",
            mapping_function_c(f),
            f.code().to_string(),
        ),
        // src/ionex.rs sidereon_ionex_header_get_mapping_declaration writes
        // NO_MAPPING as the function of an absent declaration.
        None => (
            "SIDEREON_IONEX_MAPPING_DECLARATION_KIND_ABSENT",
            "SIDEREON_IONEX_MAPPING_FUNCTION_KIND_NO_MAPPING".to_string(),
            String::new(),
        ),
    };
    println!("#define {P}_HEADER_DECLARATION {declaration}");
    println!("#define {P}_HEADER_FUNCTION {function}");
    define_text(&format!("{P}_HEADER_MAPPING_CODE"), &code);

    // The sample view of the parsed product.
    let samples = ionex.tec_grid_samples();
    let flattened = |maps: &[Vec<Vec<Option<f64>>>]| {
        maps.iter()
            .map(|m| m.iter().map(Vec::len).sum::<usize>())
            .sum::<usize>()
    };
    define_typed(
        &format!("{P}_INFO_EPOCH_COUNT"),
        "size_t",
        samples.map_epochs.len(),
    );
    define_typed(
        &format!("{P}_INFO_LAT_COUNT"),
        "size_t",
        samples.lat_nodes_deg.len(),
    );
    define_typed(
        &format!("{P}_INFO_LON_COUNT"),
        "size_t",
        samples.lon_nodes_deg.len(),
    );
    define_typed(
        &format!("{P}_INFO_TEC_COUNT"),
        "size_t",
        flattened(&samples.tec_maps),
    );
    define_bool(&format!("{P}_INFO_HAS_RMS"), !samples.rms_maps.is_empty());
    define_typed(
        &format!("{P}_INFO_RMS_COUNT"),
        "size_t",
        flattened(&samples.rms_maps),
    );
    define_bool(
        &format!("{P}_INFO_HAS_HEIGHT"),
        !samples.height_maps.is_empty(),
    );
    define_typed(
        &format!("{P}_INFO_HEIGHT_COUNT"),
        "size_t",
        flattened(&samples.height_maps),
    );
    let all_present = samples
        .tec_maps
        .iter()
        .flatten()
        .flatten()
        .all(|v| v.is_some());
    define_bool(&format!("{P}_TEC_ALL_PRESENT"), all_present);
    let nodes = ionex.tec_samples();
    define_typed(&format!("{P}_NODE_COUNT"), "size_t", nodes.len());
    define_bool(
        &format!("{P}_NODES_ALL_VTEC"),
        nodes.iter().all(|n| n.vtec_tecu.is_some()),
    );
    define_bool(
        &format!("{P}_NODES_ANY_RMS"),
        nodes.iter().any(|n| n.rms_tecu.is_some()),
    );
    define_bool(
        &format!("{P}_NODES_ANY_HEIGHT"),
        nodes.iter().any(|n| n.height_offset_km.is_some()),
    );
    define_bool(&format!("{P}_WRITER_OK"), ionex.to_ionex_string().is_ok());
    println!();
}

fn result_list(ionex: &Ionex) {
    let cases = ionex_cases();
    let inside = &cases[0];
    // Row 1 moves the receiver latitude to 95 degrees; row 2 turns the line of
    // sight to azimuth 120.
    let rows = [
        (inside.lat, inside.az),
        (95.0, inside.az),
        (inside.lat, 120.0),
    ];
    let mut accepted = Vec::new();
    let mut marshalled = Vec::new();
    for (lat, az) in rows {
        match Wgs84Geodetic::new(lat * DEG, inside.lon * DEG, 0.0) {
            Ok(receiver) => {
                accepted.push(IonexSlantRequest::new(
                    receiver,
                    inside.el * DEG,
                    az * DEG,
                    map_epoch(inside.epoch_s),
                    inside.frequency_hz,
                ));
                marshalled.push(true);
            }
            Err(_) => marshalled.push(false),
        }
    }
    let mut evaluated =
        ionex_slant_delay_results(ionex, &accepted, IonexSlantPolicy::default()).into_iter();
    let mut pins = Vec::new();
    for ok in marshalled {
        let result = if ok {
            evaluated.next().expect("one result per request")
        } else {
            // src/ionex.rs refused_row: an invalid input the engine never saw.
            Err(Error::InvalidInput("receiver".into()))
        };
        pins.push(slant_pin(&result));
    }
    define_typed(&format!("{P}_ROW_COUNT"), "size_t", pins.len());
    println!(
        "static const SmokeBSlantPin {P}_ROW[] = {{\n    {}\n}};",
        pins.join(",\n    ")
    );

    let declared = slant_with_policy(
        ionex,
        inside.lat,
        inside.lon,
        inside.az,
        inside.el,
        inside.epoch_s,
        F_L1,
        policy(
            IonexCoveragePolicy::Strict,
            IonexMissingNodePolicy::Strict,
            IonexMappingPolicy::Declared,
        ),
    );
    define_slant("DECLARED", &declared);
    println!();
}

fn tec_grid_section() {
    const EPOCHS_NS: [f64; 2] = [0.0, 1000.0];
    const LATS: [f64; 2] = [-10.0, 10.0];
    const LONS: [f64; 2] = [20.0, 60.0];
    const VALUES: [f64; 8] = [1.0, 3.0, 7.0, 15.0, 2.0, 6.0, 14.0, 30.0];
    let grid = TecGrid::new(
        EPOCHS_NS.to_vec(),
        LATS.to_vec(),
        LONS.to_vec(),
        VALUES.iter().map(|v| Some(*v)).collect(),
    )
    .expect("standalone TEC grid");
    let at = |grid: &TecGrid, nanos: i64, lat: f64, lon: f64, p: IonexMissingNodePolicy| {
        grid.vtec_at_pierce_point_with_policy(TecGridEpoch::new(nanos, 0), lon, lat, p)
    };
    let emit = |name: &str,
                r: Result<
        sidereon_core::atmosphere::ionosphere::TecGridEvaluation<f64>,
        TecGridError,
    >| {
        match r {
            Ok(e) => {
                define_bool(&format!("{P}_GRID_{name}_OK"), true);
                define_bits(&format!("{P}_GRID_{name}_VTEC_BITS"), e.value);
                println!(
                    "static const SmokeBGapPin {P}_GRID_{name}_GAP = {};",
                    gap_c(e.degraded)
                );
            }
            Err(err) => {
                define_bool(&format!("{P}_GRID_{name}_OK"), false);
                // src/ionex.rs tec_grid_error_status.
                let status = match &err {
                    TecGridError::AxesTooShort
                    | TecGridError::AxesNotIncreasing
                    | TecGridError::DimensionsOverflow
                    | TecGridError::ValueCountMismatch { .. }
                    | TecGridError::InvalidField { .. } => "SIDEREON_STATUS_INVALID_ARGUMENT",
                    _ => "SIDEREON_STATUS_SOLVE",
                };
                println!("#define {P}_GRID_{name}_STATUS {status}");
                println!(
                    "#define {P}_GRID_{name}_KIND {}",
                    c_enum("SIDEREON_TEC_GRID_ERROR_KIND_", &err)
                );
                let gap = match &err {
                    TecGridError::NodesNotAvailable(g) => Some(*g),
                    _ => None,
                };
                println!(
                    "static const SmokeBGapPin {P}_GRID_{name}_GAP = {};",
                    gap_c(gap)
                );
                if let TecGridError::OutOfBounds { name: axis, value } = &err {
                    let axis_c = match *axis {
                        "timestamp" => "SIDEREON_TEC_GRID_AXIS_EPOCH",
                        "latitude" => "SIDEREON_TEC_GRID_AXIS_LATITUDE",
                        "longitude" => "SIDEREON_TEC_GRID_AXIS_LONGITUDE",
                        _ => "SIDEREON_TEC_GRID_AXIS_UNKNOWN",
                    };
                    println!("#define {P}_GRID_{name}_AXIS {axis_c}");
                    define_bits(&format!("{P}_GRID_{name}_AXIS_VALUE_BITS"), *value);
                }
                if let TecGridError::InvalidField { field, reason } = &err {
                    define_text(&format!("{P}_GRID_{name}_FIELD"), field);
                    define_text(&format!("{P}_GRID_{name}_REASON"), reason);
                }
                define_text(&format!("{P}_GRID_{name}_MESSAGE"), &err.to_string());
            }
        }
    };
    emit(
        "PIERCE",
        at(&grid, 250, 0.0, 30.0, IonexMissingNodePolicy::Strict),
    );
    emit(
        "NODE",
        at(&grid, 0, 10.0, 60.0, IonexMissingNodePolicy::Strict),
    );
    emit(
        "SWAPPED",
        at(&grid, 250, 30.0, 0.0, IonexMissingNodePolicy::Strict),
    );
    emit(
        "NAN_LAT",
        at(&grid, 250, f64::NAN, 30.0, IonexMissingNodePolicy::Strict),
    );

    // Node 0 given as an explicit zero and node 3 as missing.
    let mut sparse_values: Vec<Option<f64>> = VALUES.iter().map(|v| Some(*v)).collect();
    sparse_values[0] = Some(0.0);
    sparse_values[3] = None;
    let sparse = TecGrid::new(
        EPOCHS_NS.to_vec(),
        LATS.to_vec(),
        LONS.to_vec(),
        sparse_values,
    )
    .expect("sparse TEC grid");
    emit(
        "SPARSE_STRICT",
        at(&sparse, 250, 0.0, 30.0, IonexMissingNodePolicy::Strict),
    );
    emit(
        "SPARSE_RENORMALIZE",
        at(&sparse, 250, 0.0, 30.0, IonexMissingNodePolicy::Renormalize),
    );
    println!();
}

/// Build outcomes, slant outcomes and sample views of the hand-built
/// products in smoke.c's IONEX helper checks.
fn built_products() {
    let tec_flat = [10.0; 8];
    let all = [true; 8];
    let epochs = [0, 3600];

    // exercise_ionex_batch_output_zeroing and the other uniform products.
    let uniform = grid_product(epochs, &tec_flat, &all, None, undeclared_header());
    define_bool(&format!("{P}_UNIFORM_BUILD_OK"), uniform.is_ok());

    // exercise_ionex_blank_custom_mapping_code: a header declaring an empty
    // custom code, and an undeclared one.
    let single_layer = policy(
        IonexCoveragePolicy::Strict,
        IonexMissingNodePolicy::Strict,
        IonexMappingPolicy::SingleLayer,
    );
    let mut blank = undeclared_header();
    blank.mapping_function = Some(IonexMappingFunction::Other(String::new()));
    let blank_product = grid_product(epochs, &tec_flat, &all, None, blank).expect("blank code");
    define_slant(
        "BLANK_CODE",
        &slant_with_policy(
            &blank_product,
            0.0,
            0.0,
            0.0,
            85.0,
            1800,
            F_L1,
            single_layer,
        ),
    );
    let bare = grid_product(epochs, &tec_flat, &all, None, undeclared_header()).expect("bare");
    define_slant(
        "BARE",
        &slant_with_policy(&bare, 0.0, 0.0, 0.0, 85.0, 1800, F_L1, single_layer),
    );

    // exercise_ionex_writer_refusal: the writer's outcome for each custom code.
    let codes = ["", "COSZ", "ABCDE", "ABCD"];
    let mut writes = Vec::new();
    let mut statuses = Vec::new();
    let mut messages = Vec::new();
    for code in codes {
        let mut header = undeclared_header();
        header.mapping_function = Some(IonexMappingFunction::Other(code.to_string()));
        let product = grid_product(epochs, &tec_flat, &all, None, header).expect("coded product");
        let written = product.to_ionex_string();
        writes.push(written.is_ok().to_string());
        statuses.push(match &written {
            Ok(_) => "SIDEREON_STATUS_OK",
            Err(err) => status_c(err),
        });
        messages.push(c_string(
            &written.err().map(|e| e.to_string()).unwrap_or_default(),
        ));
    }
    println!(
        "static const bool {P}_WRITER_CODE_OK[4] = {{ {} }};",
        writes.join(", ")
    );
    println!(
        "static const char *const {P}_WRITER_CODE_ERROR[4] = {{ {} }};",
        messages.join(", ")
    );
    println!(
        "static const SidereonStatus {P}_WRITER_CODE_STATUS[4] = {{ {} }};",
        statuses.join(", ")
    );

    // exercise_ionex_height_presence_distinction.
    let zeros = [0.0; 8];
    let none = [false; 8];
    let all_missing = grid_product(
        epochs,
        &tec_flat,
        &all,
        Some((&zeros, &none)),
        undeclared_header(),
    )
    .expect("all-missing heights");
    let s = all_missing.tec_grid_samples();
    define_bool(
        &format!("{P}_ALL_MISSING_HEIGHT_HAS"),
        !s.height_maps.is_empty(),
    );
    define_typed(
        &format!("{P}_ALL_MISSING_HEIGHT_COUNT"),
        "size_t",
        s.height_maps
            .iter()
            .map(|m| m.iter().map(Vec::len).sum::<usize>())
            .sum::<usize>(),
    );
    let presence: Vec<String> = s
        .height_maps
        .iter()
        .flatten()
        .flatten()
        .map(|v| v.is_some().to_string())
        .collect();
    println!(
        "static const bool {P}_ALL_MISSING_HEIGHT_PRESENT[] = {{ {} }};",
        presence.join(", ")
    );
    let without = uniform.expect("uniform").tec_grid_samples();
    define_bool(
        &format!("{P}_NO_HEIGHT_HAS"),
        !without.height_maps.is_empty(),
    );

    // exercise_ionex_missing_nodes_and_heights.
    let tec_ramp = [10.0, 12.0, 14.0, 16.0, 20.0, 22.0, 24.0, 26.0];
    let strict = IonexSlantPolicy::default();
    let renormalize = policy(
        IonexCoveragePolicy::Strict,
        IonexMissingNodePolicy::Renormalize,
        IonexMappingPolicy::SingleLayer,
    );
    let complete =
        grid_product(epochs, &tec_ramp, &all, None, undeclared_header()).expect("complete");
    define_slant(
        "COMPLETE",
        &slant_with_policy(&complete, 0.0, 0.0, 0.0, 85.0, 1800, F_L1, strict),
    );
    let mut first_missing = all;
    first_missing[0] = false;
    let sparse =
        grid_product(epochs, &tec_ramp, &first_missing, None, undeclared_header()).expect("sparse");
    define_slant(
        "SPARSE_STRICT",
        &slant_with_policy(&sparse, 0.0, 0.0, 0.0, 85.0, 1800, F_L1, strict),
    );
    define_slant(
        "SPARSE_RENORMALIZE",
        &slant_with_policy(&sparse, 0.0, 0.0, 0.0, 85.0, 1800, F_L1, renormalize),
    );
    let heighted = grid_product(
        epochs,
        &tec_ramp,
        &all,
        Some((&zeros, &all)),
        undeclared_header(),
    )
    .expect("heighted");
    define_slant(
        "HEIGHTED",
        &slant_with_policy(&heighted, 0.0, 0.0, 0.0, 85.0, 1800, F_L1, strict),
    );
    let mut height_present = all;
    height_present[1] = false;
    let unknown_height = grid_product(
        epochs,
        &tec_ramp,
        &all,
        Some((&zeros, &height_present)),
        undeclared_header(),
    )
    .expect("unknown height");
    define_slant(
        "UNKNOWN_HEIGHT",
        &slant_with_policy(&unknown_height, 0.0, 0.0, 0.0, 85.0, 0, F_L1, strict),
    );

    // exercise_ionex_whole_second_epochs: maps at 2^53 + 1 s and an hour on.
    let far = grid_product(
        [9_007_199_254_740_993, 9_007_199_254_744_593],
        &tec_flat,
        &all,
        None,
        undeclared_header(),
    );
    define_bool(&format!("{P}_FAR_BUILD_OK"), far.is_ok());
    let axis = far.map(|far| far.map_epochs_s()).unwrap_or_default();
    println!(
        "#define {P}_FAR_EPOCH0 INT64_C({})",
        axis.first().copied().unwrap_or(0)
    );
    println!(
        "#define {P}_FAR_EPOCH1 INT64_C({})",
        axis.get(1).copied().unwrap_or(0)
    );
    println!();
}

fn main() {
    let bytes = read_bytes(&tests_path("fixtures/ionex/synthetic_2map_7x7.20i"));
    let ionex = Ionex::parse(&bytes).expect("IONEX fixture");
    header_start(BIN, GUARD);
    print_pin_types();
    iono_surface(&ionex);
    warnings_and_header();
    result_list(&ionex);
    tec_grid_section();
    built_products();
    header_end(GUARD);
}
