// smoke.c exercise_ppp_surface: the float and fixed solve metadata and
// ambiguity values ppp_fixture.h does not carry, the VMF1-mapping solve and
// the engine's refusal of non-ascending VMF samples. The inputs are the
// committed fixtures/ppp_esbc.json, built into sidereon-core types as
// src/ppp_fixture_parity.rs builds them (that module checks the C routes
// return the engine's result bit for bit on these inputs).

#[path = "smoke_a_support/mod.rs"]
mod support;

use sidereon_core::atmosphere::troposphere::Met;
use sidereon_core::ephemeris::{Sp3, Sp3InterpolationOptions};
use sidereon_core::ppp_corrections::CivilDateTime;
use sidereon_core::precise_positioning::{
    solve_fixed_from_float, solve_float_epochs, FixedAmbiguityOptions, FixedSolveConfig,
    FloatEpoch, FloatObservation, FloatSolveConfig, FloatSolveOptions, FloatState,
    MeasurementWeights, PcvSample, RangeCorrections, ReceiverAntennaFrequency,
    ReceiverAntennaOptions, TropoMapping, TroposphereOptions, VmfSiteSample, VmfSiteSeries,
};
use sidereon_core::GnssSatelliteId;
use std::collections::BTreeMap;
use std::str::FromStr;
use support::*;
use valgen::{header_end, header_start, read, read_bytes, tests_path};

const GUARD: &str = "SIDEREON_SMOKE_A_PPP_PINS_H";

/// A JSON value whose numbers keep their source text, so each reads as the
/// double Python's json module (which writes ppp_fixture.h) reads.
#[derive(Debug)]
enum Json {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    fn get(&self, key: &str) -> &Json {
        match self {
            Json::Object(fields) => fields
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value)
                .unwrap_or_else(|| panic!("fixture has no key {key:?}")),
            other => panic!("fixture value is not an object: {other:?}"),
        }
    }

    fn f64(&self) -> f64 {
        match self {
            Json::Number(text) => text
                .parse::<f64>()
                .unwrap_or_else(|err| panic!("fixture number {text:?}: {err}")),
            other => panic!("fixture value is not a number: {other:?}"),
        }
    }

    fn i64(&self) -> i64 {
        match self {
            Json::Number(text) => text
                .parse::<i64>()
                .unwrap_or_else(|err| panic!("fixture integer {text:?}: {err}")),
            other => panic!("fixture value is not a number: {other:?}"),
        }
    }

    fn bool(&self) -> bool {
        match self {
            Json::Bool(value) => *value,
            other => panic!("fixture value is not a boolean: {other:?}"),
        }
    }

    fn str(&self) -> &str {
        match self {
            Json::String(value) => value,
            other => panic!("fixture value is not a string: {other:?}"),
        }
    }

    fn array(&self) -> &[Json] {
        match self {
            Json::Array(values) => values,
            other => panic!("fixture value is not an array: {other:?}"),
        }
    }

    fn object(&self) -> &[(String, Json)] {
        match self {
            Json::Object(fields) => fields,
            other => panic!("fixture value is not an object: {other:?}"),
        }
    }
}

struct JsonReader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl JsonReader<'_> {
    fn parse(text: &str) -> Json {
        let mut reader = JsonReader {
            bytes: text.as_bytes(),
            pos: 0,
        };
        let value = reader.value();
        reader.skip_whitespace();
        assert_eq!(reader.pos, reader.bytes.len(), "trailing text in fixture");
        value
    }

    fn skip_whitespace(&mut self) {
        while self
            .bytes
            .get(self.pos)
            .is_some_and(|b| matches!(b, b' ' | b'\t' | b'\n' | b'\r'))
        {
            self.pos += 1;
        }
    }

    fn peek(&mut self) -> u8 {
        self.skip_whitespace();
        *self
            .bytes
            .get(self.pos)
            .unwrap_or_else(|| panic!("fixture ends early"))
    }

    fn expect(&mut self, byte: u8) {
        assert_eq!(
            self.peek(),
            byte,
            "fixture byte {} is not {:?}",
            self.pos,
            byte as char
        );
        self.pos += 1;
    }

    fn literal(&mut self, word: &str) {
        assert!(
            self.bytes[self.pos..].starts_with(word.as_bytes()),
            "fixture byte {} is not {word}",
            self.pos
        );
        self.pos += word.len();
    }

    fn value(&mut self) -> Json {
        match self.peek() {
            b'{' => {
                self.pos += 1;
                let mut fields = Vec::new();
                if self.peek() == b'}' {
                    self.pos += 1;
                    return Json::Object(fields);
                }
                loop {
                    let key = self.string();
                    self.expect(b':');
                    fields.push((key, self.value()));
                    match self.peek() {
                        b',' => self.pos += 1,
                        b'}' => {
                            self.pos += 1;
                            return Json::Object(fields);
                        }
                        other => panic!("fixture byte {} is {:?}", self.pos, other as char),
                    }
                }
            }
            b'[' => {
                self.pos += 1;
                let mut values = Vec::new();
                if self.peek() == b']' {
                    self.pos += 1;
                    return Json::Array(values);
                }
                loop {
                    values.push(self.value());
                    match self.peek() {
                        b',' => self.pos += 1,
                        b']' => {
                            self.pos += 1;
                            return Json::Array(values);
                        }
                        other => panic!("fixture byte {} is {:?}", self.pos, other as char),
                    }
                }
            }
            b'"' => Json::String(self.string()),
            b't' => {
                self.literal("true");
                Json::Bool(true)
            }
            b'f' => {
                self.literal("false");
                Json::Bool(false)
            }
            b'n' => {
                self.literal("null");
                Json::Null
            }
            _ => {
                let start = self.pos;
                while self
                    .bytes
                    .get(self.pos)
                    .is_some_and(|b| matches!(b, b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E'))
                {
                    self.pos += 1;
                }
                assert!(self.pos > start, "fixture byte {start} starts no value");
                Json::Number(
                    std::str::from_utf8(&self.bytes[start..self.pos])
                        .expect("ASCII number")
                        .to_owned(),
                )
            }
        }
    }

    fn string(&mut self) -> String {
        self.expect(b'"');
        let mut out = String::new();
        loop {
            let byte = self.bytes[self.pos];
            self.pos += 1;
            match byte {
                b'"' => return out,
                b'\\' => {
                    let escaped = self.bytes[self.pos];
                    self.pos += 1;
                    out.push(match escaped {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'n' => '\n',
                        b't' => '\t',
                        other => panic!("fixture escape \\{} is not handled", other as char),
                    });
                }
                _ => {
                    // Multi-byte UTF-8 passes through byte by byte.
                    let start = self.pos - 1;
                    let mut end = self.pos;
                    while end < self.bytes.len() && (self.bytes[end] & 0xC0) == 0x80 {
                        end += 1;
                    }
                    out.push_str(std::str::from_utf8(&self.bytes[start..end]).expect("UTF-8"));
                    self.pos = end;
                }
            }
        }
    }
}

const ZERO_PCV_ZENITHS_DEG: [f64; 2] = [0.0, 90.0];

fn sorted_f64_map(value: &Json) -> Vec<(String, f64)> {
    let mut out: Vec<(String, f64)> = value
        .object()
        .iter()
        .map(|(key, value)| (key.clone(), value.f64()))
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn epochs(fixture: &Json) -> Vec<FloatEpoch> {
    fixture
        .get("epochs")
        .array()
        .iter()
        .map(|epoch| {
            let civil = epoch.get("civil");
            FloatEpoch {
                epoch: CivilDateTime {
                    year: i32::try_from(civil.get("year").i64()).expect("year"),
                    month: u8::try_from(civil.get("month").i64()).expect("month"),
                    day: u8::try_from(civil.get("day").i64()).expect("day"),
                    hour: u8::try_from(civil.get("hour").i64()).expect("hour"),
                    minute: u8::try_from(civil.get("minute").i64()).expect("minute"),
                    second: civil.get("second").f64(),
                },
                jd_whole: epoch.get("jd_whole").f64(),
                jd_fraction: epoch.get("jd_fraction").f64(),
                t_rx_j2000_s: epoch.get("t_rx_j2000_s").f64(),
                observations: epoch
                    .get("observations")
                    .array()
                    .iter()
                    .map(|obs| {
                        let sat = GnssSatelliteId::from_str(obs.get("satellite_id").str())
                            .expect("satellite token");
                        FloatObservation {
                            sat,
                            satellite_id: sat.to_string(),
                            ambiguity_id: obs.get("ambiguity_id").str().to_owned(),
                            code_m: obs.get("code_m").f64(),
                            phase_m: obs.get("phase_m").f64(),
                            freq1_hz: obs.get("freq1_hz").f64(),
                            freq2_hz: obs.get("freq2_hz").f64(),
                            glonass_channel: None,
                            signals: None,
                        }
                    })
                    .collect(),
            }
        })
        .collect()
}

fn initial_state(fixture: &Json) -> FloatState {
    let state = fixture.get("initial_state");
    let position = state.get("position_m").array();
    FloatState {
        position_m: [position[0].f64(), position[1].f64(), position[2].f64()],
        clocks_m: state
            .get("clocks_m")
            .array()
            .iter()
            .map(Json::f64)
            .collect(),
        ambiguities_m: state
            .get("ambiguities_m")
            .array()
            .iter()
            .map(|pair| {
                let pair = pair.array();
                (pair[0].str().to_owned(), pair[1].f64())
            })
            .collect(),
        ztd_m: state.get("ztd_m").f64(),
        tropo_gradient_north_m: 0.0,
        tropo_gradient_east_m: 0.0,
        residual_ionosphere_m: BTreeMap::new(),
    }
}

fn weights(config: &Json) -> MeasurementWeights {
    let weights = config.get("weights");
    MeasurementWeights {
        code: weights.get("code").f64(),
        phase: weights.get("phase").f64(),
        elevation_weighting: weights.get("elevation_weighting").bool(),
    }
}

fn tropo(config: &Json, mapping: TropoMapping) -> TroposphereOptions {
    let tropo = config.get("tropo");
    let met = Met::new(
        tropo.get("pressure_hpa").f64(),
        tropo.get("temperature_k").f64(),
        tropo.get("relative_humidity").f64(),
    )
    .expect("fixture meteorology");
    let mut options = TroposphereOptions::new(met);
    options.enabled = tropo.get("enabled").bool();
    options.estimate_ztd = tropo.get("estimate_ztd").bool();
    options.estimate_tropo_gradients = false;
    options.mapping = mapping;
    options
}

fn options(config: &Json) -> FloatSolveOptions {
    let opts = config.get("opts");
    let mut options = FloatSolveOptions::default();
    options.max_iterations = usize::try_from(opts.get("max_iterations").i64()).expect("iterations");
    options.position_tolerance_m = opts.get("position_tolerance_m").f64();
    options.clock_tolerance_m = opts.get("clock_tolerance_m").f64();
    options.ambiguity_tolerance_m = opts.get("ambiguity_tolerance_m").f64();
    options.ztd_tolerance_m = opts.get("ztd_tolerance_m").f64();
    options
}

/// smoke.c configure_ppp_zero_receiver_antenna: zero offsets and a zero
/// no-azimuth pattern on L1 and L2.
fn zero_antenna_corrections() -> RangeCorrections {
    let frequency = |label: &str| ReceiverAntennaFrequency {
        label: label.to_owned(),
        pco_m: [0.0; 3],
        pcv_samples: ZERO_PCV_ZENITHS_DEG
            .iter()
            .map(|zenith_deg| PcvSample {
                azimuth_deg: None,
                zenith_deg: *zenith_deg,
                value_m: 0.0,
            })
            .collect(),
    };
    RangeCorrections {
        receiver_antenna: Some(ReceiverAntennaOptions::new(
            "L1".to_owned(),
            1_575_420_000.0,
            "L2".to_owned(),
            1_227_600_000.0,
            vec![frequency("L1"), frequency("L2")],
        )),
        sat_clock_relativity: false,
        satellite_clock: None,
        ppp: Default::default(),
    }
}

fn float_config(fixture: &Json, mapping: TropoMapping) -> FloatSolveConfig {
    let config = fixture.get("config");
    FloatSolveConfig::new(
        weights(config),
        tropo(config, mapping),
        zero_antenna_corrections(),
        options(config),
        None,
        config.get("residual_screen").bool(),
        false,
    )
}

fn fixed_config(fixture: &Json) -> FixedSolveConfig {
    let config = fixture.get("fixed_config");
    let ambiguity_json = config.get("ambiguity");
    let mut ambiguity = FixedAmbiguityOptions::new(ambiguity_json.get("ratio_threshold").f64());
    ambiguity.wavelengths_m = sorted_f64_map(ambiguity_json.get("wavelengths_m"))
        .into_iter()
        .collect();
    ambiguity.offsets_m = sorted_f64_map(ambiguity_json.get("offsets_m"))
        .into_iter()
        .collect();
    FixedSolveConfig::new(
        weights(config),
        tropo(config, TropoMapping::Niell),
        zero_antenna_corrections(),
        options(config),
        None,
        ambiguity,
        false,
    )
}

/// smoke.c's VMF1 site series: two samples bracketing the arc.
fn vmf_samples(unsorted: bool) -> Vec<VmfSiteSample> {
    vec![
        VmfSiteSample {
            mjd: 59025.0,
            ah: 0.00123,
            aw: 0.00055,
        },
        VmfSiteSample {
            mjd: if unsorted { 59024.0 } else { 59025.5 },
            ah: 0.00124,
            aw: 0.00056,
        },
    ]
}

fn main() {
    let fixture = JsonReader::parse(&read(&tests_path("fixtures/ppp_esbc.json")));
    let sp3_path = tests_path(&format!("fixtures/sp3/{}", fixture.get("sp3_file").str()));
    let sp3 = Sp3::parse(&read_bytes(&sp3_path))
        .expect("parse PPP SP3")
        .with_interpolation_options(Sp3InterpolationOptions::default());
    let epochs = epochs(&fixture);

    let float = solve_float_epochs(
        &sp3,
        &epochs,
        initial_state(&fixture),
        float_config(&fixture, TropoMapping::Niell),
    )
    .expect("PPP float solve");
    let fixed = solve_fixed_from_float(&sp3, &epochs, float.clone(), fixed_config(&fixture))
        .expect("PPP fixed solve");
    let vmf_series = VmfSiteSeries::new(&vmf_samples(false)).expect("VMF1 series");
    let vmf = solve_float_epochs(
        &sp3,
        &epochs,
        initial_state(&fixture),
        float_config(&fixture, TropoMapping::Vmf1(vmf_series)),
    )
    .expect("PPP VMF1 float solve");
    let unsorted = VmfSiteSeries::new(&vmf_samples(true));

    header_start("smoke_a_ppp", GUARD);
    comment("solve_float_epochs of fixtures/ppp_esbc.json under the smoke's float config.");
    def_usize("SMOKE_A_PPP_FLOAT_ITERATIONS", float.iterations);
    def_bool("SMOKE_A_PPP_FLOAT_CONVERGED", float.converged);
    def_variant("SMOKE_A_PPP_FLOAT_STATUS", &float.status);
    def_bool(
        "SMOKE_A_PPP_FLOAT_HAS_ZTD_RESIDUAL",
        float.ztd_residual_m.is_some(),
    );
    def_bits("SMOKE_A_PPP_FLOAT_CODE_RMS_BITS", float.code_rms_m);
    def_bits("SMOKE_A_PPP_FLOAT_PHASE_RMS_BITS", float.phase_rms_m);
    def_bits("SMOKE_A_PPP_FLOAT_WEIGHTED_RMS_BITS", float.weighted_rms_m);
    def_usize(
        "SMOKE_A_PPP_FLOAT_AMBIGUITY_COUNT",
        float.ambiguities_m.len(),
    );
    def_usize("SMOKE_A_PPP_FLOAT_RESIDUAL_COUNT", float.residuals_m.len());
    def_usize("SMOKE_A_PPP_FLOAT_USED_SAT_COUNT", float.used_sats.len());
    let ambiguities: Vec<f64> = float.ambiguities_m.values().copied().collect();
    def_bits_array("SMOKE_A_PPP_FLOAT_AMBIGUITY_BITS", &ambiguities);

    comment("solve_fixed_from_float of that float solution under the fixed config.");
    def_usize("SMOKE_A_PPP_FIXED_ITERATIONS", fixed.iterations);
    def_bool("SMOKE_A_PPP_FIXED_CONVERGED", fixed.converged);
    def_variant("SMOKE_A_PPP_FIXED_STATUS", &fixed.status);
    def_bool(
        "SMOKE_A_PPP_FIXED_HAS_ZTD_RESIDUAL",
        fixed.ztd_residual_m.is_some(),
    );
    def_bits("SMOKE_A_PPP_FIXED_CODE_RMS_BITS", fixed.code_rms_m);
    def_bits("SMOKE_A_PPP_FIXED_PHASE_RMS_BITS", fixed.phase_rms_m);
    def_bits("SMOKE_A_PPP_FIXED_WEIGHTED_RMS_BITS", fixed.weighted_rms_m);
    def_usize("SMOKE_A_PPP_FIXED_RESIDUAL_COUNT", fixed.residuals_m.len());
    def_usize("SMOKE_A_PPP_FIXED_USED_SAT_COUNT", fixed.used_sats.len());
    def_bits(
        "SMOKE_A_PPP_FIXED_INTEGER_BEST_SCORE_BITS",
        fixed.integer.integer_best_score,
    );
    def_bool(
        "SMOKE_A_PPP_FIXED_HAS_INTEGER_SECOND_BEST_SCORE",
        fixed.integer.integer_second_best_score.is_some(),
    );
    def_bits(
        "SMOKE_A_PPP_FIXED_INTEGER_SECOND_BEST_SCORE_BITS",
        fixed.integer.integer_second_best_score.unwrap_or(0.0),
    );

    comment("The same float solve with the smoke's VMF1 site series.");
    def_bits_array("SMOKE_A_PPP_VMF1_POSITION_BITS", &vmf.position_m);

    comment("VmfSiteSeries::new of the smoke's non-ascending samples.");
    match unsorted {
        Ok(_) => {
            def_bool("SMOKE_A_PPP_VMF_UNSORTED_REFUSED", false);
            def_str("SMOKE_A_PPP_VMF_UNSORTED_ERROR_TEXT", "");
        }
        Err(err) => {
            def_bool("SMOKE_A_PPP_VMF_UNSORTED_REFUSED", true);
            def_str("SMOKE_A_PPP_VMF_UNSORTED_ERROR_TEXT", &err.to_string());
        }
    }
    header_end(GUARD);
}
