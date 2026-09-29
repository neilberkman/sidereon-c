//! The committed ESBC PPP fixture (`tests/fixtures/ppp_esbc.json`) solved two
//! ways: through the C entry points, from the same C structs the smoke program
//! builds, and through `sidereon_core::precise_positioning` directly, from engine
//! types built here without the binding's conversion code. The two must agree
//! bit for bit, and the engine result must equal the fixture's `expected` block.
//!
//! `write_ppp_esbc_expected` is the generator for that block. The block names
//! the sidereon-core revision it was written from, `tests/CORE_REVISION`. It is ignored by
//! default; run it from the binding root with
//!
//! ```text
//! cargo test --lib ppp_fixture_parity::write_ppp_esbc_expected -- --ignored --exact
//! python3 tests/gen_ppp_fixture_header.py
//! ```
//!
//! The first command rewrites only the `expected` block of `ppp_esbc.json` from
//! the engine; the second transcribes the whole fixture into `ppp_fixture.h`.
//!
//! The fixture is read with the small parser below rather than serde_json:
//! serde_json without its `float_roundtrip` feature may round a decimal to a
//! neighbouring double, and the inputs here have to be the doubles Python's
//! `json` module (which writes `ppp_fixture.h`) reads.

use super::*;
use sidereon_core::precise_positioning::{solve_fixed_from_float, solve_float_epochs};
use std::mem::MaybeUninit;
use std::path::PathBuf;

const FIXTURE: &str = "tests/fixtures/ppp_esbc.json";
const SP3_DIR: &str = "tests/fixtures/sp3";
const ANTENNA_FREQ1_LABEL: &str = "L1";
const ANTENNA_FREQ1_HZ: f64 = 1_575_420_000.0;
const ANTENNA_FREQ2_LABEL: &str = "L2";
const ANTENNA_FREQ2_HZ: f64 = 1_227_600_000.0;
const ZERO_PCV_ZENITHS_DEG: [f64; 2] = [0.0, 90.0];

// ---------------------------------------------------------------------------
// Fixture text

/// A JSON value whose numbers keep their source text.
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
            // str::parse rounds correctly, as Python's float() does.
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

fn manifest_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn load_fixture() -> (String, Json) {
    let text = std::fs::read_to_string(manifest_path(FIXTURE)).expect("read PPP fixture");
    let json = JsonReader::parse(&text);
    (text, json)
}

fn sp3_bytes(fixture: &Json) -> Vec<u8> {
    let name = fixture.get("sp3_file").str();
    std::fs::read(manifest_path(SP3_DIR).join(name)).expect("read PPP SP3")
}

fn sorted_f64_map(value: &Json) -> Vec<(String, f64)> {
    let mut out: Vec<(String, f64)> = value
        .object()
        .iter()
        .map(|(key, value)| (key.clone(), value.f64()))
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn initial_ambiguity_pairs(fixture: &Json) -> Vec<(String, f64)> {
    fixture
        .get("initial_state")
        .get("ambiguities_m")
        .array()
        .iter()
        .map(|pair| {
            let pair = pair.array();
            assert_eq!(pair.len(), 2, "initial ambiguity is an [id, value] pair");
            (pair[0].str().to_owned(), pair[1].f64())
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Engine inputs, built from engine types

fn core_sp3(fixture: &Json) -> Sp3 {
    Sp3::parse(&sp3_bytes(fixture))
        .expect("parse PPP SP3")
        .with_interpolation_options(Sp3InterpolationOptions::default())
}

fn core_epochs(fixture: &Json) -> Vec<PppFloatEpoch> {
    fixture
        .get("epochs")
        .array()
        .iter()
        .map(|epoch| {
            let civil = epoch.get("civil");
            PppFloatEpoch {
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
                        let token = obs.get("satellite_id").str();
                        let sat = GnssSatelliteId::from_str(token).expect("satellite token");
                        PppFloatObservation {
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

fn core_initial_state(fixture: &Json) -> PppFloatStateInner {
    let state = fixture.get("initial_state");
    let position = state.get("position_m").array();
    assert_eq!(position.len(), 3);
    PppFloatStateInner {
        position_m: [position[0].f64(), position[1].f64(), position[2].f64()],
        clocks_m: state
            .get("clocks_m")
            .array()
            .iter()
            .map(Json::f64)
            .collect(),
        ambiguities_m: initial_ambiguity_pairs(fixture).into_iter().collect(),
        ztd_m: state.get("ztd_m").f64(),
        tropo_gradient_north_m: 0.0,
        tropo_gradient_east_m: 0.0,
        residual_ionosphere_m: BTreeMap::new(),
    }
}

fn core_weights(config: &Json) -> PppMeasurementWeightsInner {
    let weights = config.get("weights");
    PppMeasurementWeightsInner {
        code: weights.get("code").f64(),
        phase: weights.get("phase").f64(),
        elevation_weighting: weights.get("elevation_weighting").bool(),
    }
}

fn core_tropo(config: &Json) -> PppTroposphereOptionsInner {
    let tropo = config.get("tropo");
    assert!(
        tropo.get("enabled").bool(),
        "the ESBC arc is troposphere-corrected"
    );
    let met = Met::new(
        tropo.get("pressure_hpa").f64(),
        tropo.get("temperature_k").f64(),
        tropo.get("relative_humidity").f64(),
    )
    .expect("fixture meteorology");
    let mut options = PppTroposphereOptionsInner::new(met);
    options.enabled = true;
    options.estimate_ztd = tropo.get("estimate_ztd").bool();
    options.estimate_tropo_gradients = false;
    options.mapping = PppTropoMapping::Niell;
    options
}

fn core_options(config: &Json) -> PppFloatSolveOptions {
    let opts = config.get("opts");
    let mut options = PppFloatSolveOptions::default();
    options.max_iterations = usize::try_from(opts.get("max_iterations").i64()).expect("iterations");
    options.position_tolerance_m = opts.get("position_tolerance_m").f64();
    options.clock_tolerance_m = opts.get("clock_tolerance_m").f64();
    options.ambiguity_tolerance_m = opts.get("ambiguity_tolerance_m").f64();
    options.ztd_tolerance_m = opts.get("ztd_tolerance_m").f64();
    options
}

/// The receiver antenna the smoke passes: zero offsets and a zero no-azimuth
/// pattern on both frequencies.
fn core_zero_antenna_corrections() -> RangeCorrections {
    let frequency = |label: &str| PppReceiverAntennaFrequencyInner {
        label: label.to_owned(),
        pco_m: [0.0; 3],
        pcv_samples: ZERO_PCV_ZENITHS_DEG
            .iter()
            .map(|zenith_deg| PppPcvSample {
                azimuth_deg: None,
                zenith_deg: *zenith_deg,
                value_m: 0.0,
            })
            .collect(),
    };
    RangeCorrections {
        receiver_antenna: Some(PppReceiverAntennaOptionsInner::new(
            ANTENNA_FREQ1_LABEL.to_owned(),
            ANTENNA_FREQ1_HZ,
            ANTENNA_FREQ2_LABEL.to_owned(),
            ANTENNA_FREQ2_HZ,
            vec![
                frequency(ANTENNA_FREQ1_LABEL),
                frequency(ANTENNA_FREQ2_LABEL),
            ],
        )),
        sat_clock_relativity: false,
        satellite_clock: None,
        ppp: Default::default(),
    }
}

fn core_float_config(fixture: &Json) -> PppFloatSolveConfigInner {
    let config = fixture.get("config");
    PppFloatSolveConfigInner::new(
        core_weights(config),
        core_tropo(config),
        core_zero_antenna_corrections(),
        core_options(config),
        None,
        config.get("residual_screen").bool(),
        false,
    )
}

fn core_fixed_config(fixture: &Json) -> PppFixedSolveConfigInner {
    let config = fixture.get("fixed_config");
    let ambiguity_json = config.get("ambiguity");
    let ratio_threshold = ambiguity_json.get("ratio_threshold").f64();
    let mut ambiguity = PppFixedAmbiguityOptionsInner::new(ratio_threshold);
    ambiguity.wavelengths_m = sorted_f64_map(ambiguity_json.get("wavelengths_m"))
        .into_iter()
        .collect();
    ambiguity.offsets_m = sorted_f64_map(ambiguity_json.get("offsets_m"))
        .into_iter()
        .collect();
    PppFixedSolveConfigInner::new(
        core_weights(config),
        core_tropo(config),
        core_zero_antenna_corrections(),
        core_options(config),
        None,
        ambiguity,
        false,
    )
}

struct CoreSolutions {
    float: PppFloatSolutionInner,
    fixed: PppFixedSolutionInner,
}

fn core_solutions(fixture: &Json) -> CoreSolutions {
    let sp3 = core_sp3(fixture);
    let epochs = core_epochs(fixture);
    let float = solve_float_epochs(
        &sp3,
        &epochs,
        core_initial_state(fixture),
        core_float_config(fixture),
    )
    .expect("engine PPP float solve");
    let fixed = solve_fixed_from_float(&sp3, &epochs, float.clone(), core_fixed_config(fixture))
        .expect("engine PPP fixed solve");
    CoreSolutions { float, fixed }
}

// ---------------------------------------------------------------------------
// C inputs, built the way tests/smoke.c builds them

fn c_string(text: &str) -> CString {
    CString::new(text).expect("fixture text has no NUL")
}

/// Owned storage behind the raw pointers of the C configs.
struct CInputs {
    _strings: Vec<CString>,
    _observations: Vec<SidereonPppObservation>,
    epochs: Vec<SidereonPppEpoch>,
    clocks: Vec<f64>,
    initial_ambiguities: Vec<SidereonPppFloatMapEntry>,
    wavelengths: Vec<SidereonPppFloatMapEntry>,
    offsets: Vec<SidereonPppFloatMapEntry>,
    _zero_pcv: Box<[SidereonReceiverAntennaNoaziPcvSample; 2]>,
    antenna: Box<SidereonPppReceiverAntennaOptions>,
}

impl CInputs {
    fn new(fixture: &Json) -> Self {
        let mut strings = Vec::new();
        let mut intern = |text: &str| -> *const c_char {
            let owned = c_string(text);
            // A CString's heap buffer does not move when the CString does.
            let ptr = owned.as_ptr();
            strings.push(owned);
            ptr
        };

        let raw_epochs = fixture.get("epochs").array();
        let mut observations = Vec::new();
        let mut spans = Vec::with_capacity(raw_epochs.len());
        for epoch in raw_epochs {
            let start = observations.len();
            for obs in epoch.get("observations").array() {
                observations.push(SidereonPppObservation {
                    sat_id: intern(obs.get("satellite_id").str()),
                    ambiguity_id: intern(obs.get("ambiguity_id").str()),
                    code_m: obs.get("code_m").f64(),
                    phase_m: obs.get("phase_m").f64(),
                    freq1_hz: obs.get("freq1_hz").f64(),
                    freq2_hz: obs.get("freq2_hz").f64(),
                    code1_signal: ptr::null(),
                    code2_signal: ptr::null(),
                    phase1_signal: ptr::null(),
                    phase2_signal: ptr::null(),
                    has_glonass_channel: false,
                    glonass_channel: 0,
                });
            }
            spans.push((start, observations.len() - start));
        }
        let epochs = raw_epochs
            .iter()
            .zip(&spans)
            .map(|(epoch, (start, count))| {
                let civil = epoch.get("civil");
                SidereonPppEpoch {
                    civil: SidereonPppCivilDateTime {
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
                    // observations is not resized after this point.
                    observations: observations[*start..].as_ptr(),
                    observation_count: *count,
                }
            })
            .collect();

        let clocks = fixture
            .get("initial_state")
            .get("clocks_m")
            .array()
            .iter()
            .map(Json::f64)
            .collect();
        let mut map_entries = |pairs: Vec<(String, f64)>| -> Vec<SidereonPppFloatMapEntry> {
            pairs
                .into_iter()
                .map(|(id, value)| SidereonPppFloatMapEntry {
                    id: intern(&id),
                    value,
                })
                .collect()
        };
        let initial_ambiguities = map_entries(initial_ambiguity_pairs(fixture));
        let ambiguity = fixture.get("fixed_config").get("ambiguity");
        let wavelengths = map_entries(sorted_f64_map(ambiguity.get("wavelengths_m")));
        let offsets = map_entries(sorted_f64_map(ambiguity.get("offsets_m")));

        let zero_pcv = Box::new(ZERO_PCV_ZENITHS_DEG.map(|zenith_deg| {
            SidereonReceiverAntennaNoaziPcvSample {
                zenith_deg,
                value_m: 0.0,
            }
        }));
        let calibration = SidereonReceiverAntennaCalibration {
            pco_neu_m: [0.0; 3],
            noazi_pcv_m: zero_pcv.as_ptr(),
            noazi_pcv_count: zero_pcv.len(),
            azimuth_pcv_m: ptr::null(),
            azimuth_pcv_count: 0,
        };
        let antenna = Box::new(SidereonPppReceiverAntennaOptions {
            freq1_label: intern(ANTENNA_FREQ1_LABEL),
            freq1_hz: ANTENNA_FREQ1_HZ,
            freq1: calibration,
            freq2_label: intern(ANTENNA_FREQ2_LABEL),
            freq2_hz: ANTENNA_FREQ2_HZ,
            freq2: calibration,
        });

        CInputs {
            _strings: strings,
            _observations: observations,
            epochs,
            clocks,
            initial_ambiguities,
            wavelengths,
            offsets,
            _zero_pcv: zero_pcv,
            antenna,
        }
    }
}

fn c_init<T>(init: unsafe extern "C" fn(*mut T) -> SidereonStatus) -> T {
    let mut value = MaybeUninit::<T>::uninit();
    assert_eq!(unsafe { init(value.as_mut_ptr()) }, SidereonStatus::Ok);
    unsafe { value.assume_init() }
}

fn c_weights(config: &Json) -> SidereonPppMeasurementWeights {
    let weights = config.get("weights");
    let mut out = c_init(sidereon_ppp_measurement_weights_init);
    out.code = weights.get("code").f64();
    out.phase = weights.get("phase").f64();
    out.elevation_weighting = weights.get("elevation_weighting").bool();
    out
}

fn c_tropo(config: &Json) -> SidereonPppTroposphereOptions {
    let tropo = config.get("tropo");
    let mut out = c_init(sidereon_ppp_troposphere_options_init);
    out.enabled = tropo.get("enabled").bool();
    out.estimate_ztd = tropo.get("estimate_ztd").bool();
    out.pressure_hpa = tropo.get("pressure_hpa").f64();
    out.temperature_k = tropo.get("temperature_k").f64();
    out.relative_humidity = tropo.get("relative_humidity").f64();
    out
}

fn c_options(config: &Json) -> SidereonPppFloatOptions {
    let opts = config.get("opts");
    let mut out = c_init(sidereon_ppp_float_options_init);
    out.max_iterations = usize::try_from(opts.get("max_iterations").i64()).expect("iterations");
    out.position_tolerance_m = opts.get("position_tolerance_m").f64();
    out.clock_tolerance_m = opts.get("clock_tolerance_m").f64();
    out.ambiguity_tolerance_m = opts.get("ambiguity_tolerance_m").f64();
    out.ztd_tolerance_m = opts.get("ztd_tolerance_m").f64();
    out
}

fn c_corrections(inputs: &CInputs) -> SidereonPppRangeCorrections {
    let mut out = c_init(sidereon_ppp_range_corrections_init);
    out.receiver_antenna = &*inputs.antenna;
    out
}

fn c_float_config(fixture: &Json, inputs: &CInputs) -> SidereonPppFloatConfig {
    let config = fixture.get("config");
    let position = fixture.get("initial_state").get("position_m").array();
    SidereonPppFloatConfig {
        epochs: inputs.epochs.as_ptr(),
        epoch_count: inputs.epochs.len(),
        initial_state: SidereonPppFloatState {
            position_m: [position[0].f64(), position[1].f64(), position[2].f64()],
            clocks_m: inputs.clocks.as_ptr(),
            clock_count: inputs.clocks.len(),
            ambiguities_m: inputs.initial_ambiguities.as_ptr(),
            ambiguity_count: inputs.initial_ambiguities.len(),
            ztd_m: fixture.get("initial_state").get("ztd_m").f64(),
            tropo_gradient_north_m: 0.0,
            tropo_gradient_east_m: 0.0,
        },
        weights: c_weights(config),
        tropo: c_tropo(config),
        corrections: c_corrections(inputs),
        options: c_options(config),
        has_elevation_cutoff_deg: false,
        elevation_cutoff_deg: 0.0,
        residual_screen: config.get("residual_screen").bool(),
    }
}

fn c_fixed_config(fixture: &Json, inputs: &CInputs) -> SidereonPppFixedConfig {
    let config = fixture.get("fixed_config");
    let mut ambiguity = c_init(sidereon_ppp_fixed_ambiguity_options_init);
    ambiguity.wavelengths_m = inputs.wavelengths.as_ptr();
    ambiguity.wavelength_count = inputs.wavelengths.len();
    ambiguity.offsets_m = inputs.offsets.as_ptr();
    ambiguity.offset_count = inputs.offsets.len();
    ambiguity.ratio_threshold = config.get("ambiguity").get("ratio_threshold").f64();
    SidereonPppFixedConfig {
        epochs: inputs.epochs.as_ptr(),
        epoch_count: inputs.epochs.len(),
        weights: c_weights(config),
        tropo: c_tropo(config),
        corrections: c_corrections(inputs),
        options: c_options(config),
        has_elevation_cutoff_deg: false,
        elevation_cutoff_deg: 0.0,
        ambiguity,
    }
}

fn c_last_error() -> String {
    LAST_ERROR.with(|slot| {
        slot.borrow()
            .as_ref()
            .map(|message| message.to_string_lossy().into_owned())
            .unwrap_or_default()
    })
}

fn c_id(id: &SidereonPppId) -> String {
    unsafe { CStr::from_ptr(id.bytes.as_ptr()) }
        .to_str()
        .expect("UTF-8 id")
        .to_owned()
}

fn c_copy<T>(
    what: &str,
    copy: impl Fn(*mut T, usize, *mut usize, *mut usize) -> SidereonStatus,
) -> Vec<T> {
    let mut written = 0usize;
    let mut required = 0usize;
    assert_eq!(
        copy(ptr::null_mut(), 0, &mut written, &mut required),
        SidereonStatus::Ok,
        "{what} size query: {}",
        c_last_error()
    );
    let mut out = Vec::with_capacity(required);
    assert_eq!(
        copy(out.as_mut_ptr(), required, &mut written, &mut required),
        SidereonStatus::Ok,
        "{what}: {}",
        c_last_error()
    );
    assert_eq!(written, required, "{what} copied every row");
    unsafe { out.set_len(written) };
    out
}

fn c_position(what: &str, copy: impl Fn(*mut f64, usize) -> SidereonStatus) -> [f64; 3] {
    let mut out = [0.0; 3];
    assert_eq!(
        copy(out.as_mut_ptr(), out.len()),
        SidereonStatus::Ok,
        "{what}: {}",
        c_last_error()
    );
    out
}

// ---------------------------------------------------------------------------
// Comparisons

fn bits3(values: [f64; 3]) -> [u64; 3] {
    values.map(f64::to_bits)
}

fn bits_vec(values: &[f64]) -> Vec<u64> {
    values.iter().map(|value| value.to_bits()).collect()
}

fn bits_map(values: &BTreeMap<String, f64>) -> Vec<(String, u64)> {
    values
        .iter()
        .map(|(id, value)| (id.clone(), value.to_bits()))
        .collect()
}

fn integer_status_name(status: PppIntegerStatusInner) -> &'static str {
    match status {
        PppIntegerStatusInner::Fixed => "Fixed",
        PppIntegerStatusInner::NotFixed => "NotFixed",
    }
}

#[test]
fn c_ppp_matches_engine_bit_for_bit() {
    let (_, fixture) = load_fixture();
    let core = core_solutions(&fixture);

    let sp3_bytes = sp3_bytes(&fixture);
    let inputs = CInputs::new(&fixture);
    let float_config = c_float_config(&fixture, &inputs);
    let fixed_config = c_fixed_config(&fixture, &inputs);
    unsafe {
        let mut sp3: *mut SidereonSp3 = ptr::null_mut();
        assert_eq!(
            sidereon_sp3_load(sp3_bytes.as_ptr(), sp3_bytes.len(), &mut sp3),
            SidereonStatus::Ok,
            "{}",
            c_last_error()
        );
        let mut float: *mut SidereonPppFloatSolution = ptr::null_mut();
        assert_eq!(
            sidereon_solve_ppp_float(sp3, &float_config, &mut float),
            SidereonStatus::Ok,
            "{}",
            c_last_error()
        );
        let mut fixed: *mut SidereonPppFixedSolution = ptr::null_mut();
        assert_eq!(
            sidereon_solve_ppp_fixed(sp3, float, &fixed_config, &mut fixed),
            SidereonStatus::Ok,
            "{}",
            c_last_error()
        );

        // Float: position and ambiguities through the C getters; the per-epoch
        // clocks have no C getter, so they are read from the returned handle.
        let position = c_position("float position", |out, len| {
            sidereon_ppp_float_solution_position(float, out, len)
        });
        assert_eq!(
            bits3(position),
            bits3(core.float.position_m),
            "float position"
        );
        assert_eq!(
            bits_vec(&(*float).inner.epoch_clocks_m),
            bits_vec(&core.float.epoch_clocks_m),
            "float epoch clocks"
        );
        let ambiguities = c_copy("float ambiguities", |out, len, written, required| {
            sidereon_ppp_float_solution_ambiguities(float, out, len, written, required)
        });
        let c_ambiguities: BTreeMap<String, f64> = ambiguities
            .iter()
            .map(|row: &SidereonPppAmbiguity| (c_id(&row.id), row.value_m))
            .collect();
        assert_eq!(c_ambiguities.len(), ambiguities.len(), "unique float ids");
        assert_eq!(
            bits_map(&c_ambiguities),
            bits_map(&core.float.ambiguities_m),
            "float ambiguities"
        );

        // Fixed: position, embedded float position, clocks, integer metadata
        // and fixed ambiguities.
        let fixed_position = c_position("fixed position", |out, len| {
            sidereon_ppp_fixed_solution_position(fixed, out, len)
        });
        assert_eq!(
            bits3(fixed_position),
            bits3(core.fixed.position_m),
            "fixed position"
        );
        let fixed_float_position = c_position("fixed float position", |out, len| {
            sidereon_ppp_fixed_solution_float_position(fixed, out, len)
        });
        assert_eq!(
            bits3(fixed_float_position),
            bits3(core.fixed.float_solution.position_m),
            "fixed embedded float position"
        );
        assert_eq!(
            bits_vec(&(*fixed).inner.epoch_clocks_m),
            bits_vec(&core.fixed.epoch_clocks_m),
            "fixed epoch clocks"
        );
        let mut metadata = MaybeUninit::<SidereonPppFixedMetadata>::uninit();
        assert_eq!(
            sidereon_ppp_fixed_solution_metadata(fixed, metadata.as_mut_ptr()),
            SidereonStatus::Ok,
            "{}",
            c_last_error()
        );
        let metadata = metadata.assume_init();
        assert_eq!(
            metadata.integer_ratio.to_bits(),
            core.fixed.integer.integer_ratio.to_bits(),
            "integer ratio"
        );
        assert_eq!(
            metadata.integer_candidates, core.fixed.integer.integer_candidates,
            "integer candidates"
        );
        let expected_status = match core.fixed.integer.integer_status {
            PppIntegerStatusInner::Fixed => SidereonPppIntegerStatus::Fixed,
            PppIntegerStatusInner::NotFixed => SidereonPppIntegerStatus::NotFixed,
        };
        assert_eq!(metadata.integer_status, expected_status, "integer status");
        let fixed_rows = c_copy("fixed ambiguities", |out, len, written, required| {
            sidereon_ppp_fixed_solution_fixed_ambiguities(fixed, out, len, written, required)
        });
        let c_cycles: BTreeMap<String, i64> = fixed_rows
            .iter()
            .map(|row: &SidereonPppFixedAmbiguity| (c_id(&row.id), row.cycles))
            .collect();
        let c_fixed_m: BTreeMap<String, f64> = fixed_rows
            .iter()
            .map(|row| (c_id(&row.id), row.value_m))
            .collect();
        assert_eq!(c_cycles.len(), fixed_rows.len(), "unique fixed ids");
        assert_eq!(
            c_cycles, core.fixed.fixed_ambiguities_cycles,
            "fixed ambiguity cycles"
        );
        assert_eq!(
            bits_map(&c_fixed_m),
            bits_map(&core.fixed.fixed_ambiguities_m),
            "fixed ambiguities in meters"
        );

        sidereon_ppp_fixed_solution_free(fixed);
        sidereon_ppp_float_solution_free(float);
        sidereon_sp3_free(sp3);
    }
}

#[test]
fn c_ppp_fixed_compat_error_mapping_and_live_retention() {
    use crate::engine_error::{
        clear_engine_error, snapshot_engine_error_for_test, SidereonEngineErrorFamily,
    };
    use serde_json::Value;

    clear_engine_error();
    let (_, fixture) = load_fixture();
    let sp3_data = sp3_bytes(&fixture);
    let inputs = CInputs::new(&fixture);
    let float_config = c_float_config(&fixture, &inputs);
    let fixed_config = c_fixed_config(&fixture, &inputs);

    let seed_public_refusal = || {
        let invalid = b"INVALID SP3";
        let mut refused = ptr::null_mut();
        assert_eq!(
            unsafe { crate::sp3::sidereon_sp3_load(invalid.as_ptr(), invalid.len(), &mut refused) },
            SidereonStatus::Sp3Parse
        );
        assert!(refused.is_null());
        let (info, payload) = snapshot_engine_error_for_test().expect("SP3 refusal recorded");
        assert_eq!(info.family, SidereonEngineErrorFamily::Facade);
        assert!(info.payload_len > 0);
        assert!(!payload.is_empty());
        (info, payload)
    };

    unsafe {
        let mut sp3: *mut SidereonSp3 = ptr::null_mut();
        assert_eq!(
            sidereon_sp3_load(sp3_data.as_ptr(), sp3_data.len(), &mut sp3),
            SidereonStatus::Ok,
            "{}",
            c_last_error()
        );
        assert!(!sp3.is_null());

        // The public float producer clears a populated generic error slot and
        // yields the real float handle consumed by both fixed controls.
        let _ = seed_public_refusal();
        let mut float: *mut SidereonPppFloatSolution = ptr::null_mut();
        assert_eq!(
            sidereon_solve_ppp_float(sp3, &float_config, &mut float),
            SidereonStatus::Ok,
            "{}",
            c_last_error()
        );
        assert!(!float.is_null());
        assert!(snapshot_engine_error_for_test().is_none());

        // Prove a valid fixed solve succeeds and resets a second populated
        // error slot before exercising the one-wavelength compatibility error.
        let _ = seed_public_refusal();
        let mut fixed: *mut SidereonPppFixedSolution = ptr::null_mut();
        assert_eq!(
            sidereon_solve_ppp_fixed(sp3, float, &fixed_config, &mut fixed),
            SidereonStatus::Ok,
            "{}",
            c_last_error()
        );
        assert!(!fixed.is_null());
        assert!(snapshot_engine_error_for_test().is_none());

        assert!(!inputs.wavelengths.is_empty());
        let mut nan_wavelengths = inputs.wavelengths.clone();
        let original_wavelength = nan_wavelengths[0].value;
        assert!(original_wavelength.is_finite() && original_wavelength > 0.0);
        let chosen_nan = f64::from_bits(0x7ff8_0000_0000_0042);
        assert_eq!(chosen_nan.to_bits(), 0x7ff8_0000_0000_0042);
        nan_wavelengths[0].value = chosen_nan;
        for (index, (original, changed)) in
            inputs.wavelengths.iter().zip(&nan_wavelengths).enumerate()
        {
            assert_eq!(original.id, changed.id);
            if index == 0 {
                assert_eq!(changed.value.to_bits(), chosen_nan.to_bits());
            } else {
                assert_eq!(changed.value.to_bits(), original.value.to_bits());
            }
        }
        let mut nan_fixed_config = c_fixed_config(&fixture, &inputs);
        nan_fixed_config.ambiguity.wavelengths_m = nan_wavelengths.as_ptr();

        let mut refused_fixed: *mut SidereonPppFixedSolution = ptr::null_mut();
        assert_eq!(
            sidereon_solve_ppp_fixed(sp3, float, &nan_fixed_config, &mut refused_fixed),
            SidereonStatus::InvalidArgument
        );
        assert!(refused_fixed.is_null());
        let (base_info, base_payload) =
            snapshot_engine_error_for_test().expect("typed PPP fixed refusal recorded");
        assert_eq!(base_info.family, SidereonEngineErrorFamily::Facade);
        assert!(base_info.payload_len > 0);
        assert!(!base_payload.is_empty());
        let payload: Value = serde_json::from_str(&base_payload).expect("valid engine JSON");
        assert_eq!(payload["schema_version"], 1);
        assert_eq!(payload["family"], "facade");
        assert_eq!(payload["operation"], "sidereon_solve_ppp_fixed");
        assert_eq!(payload["error"]["kind"], "ppp_fixed");
        assert_eq!(payload["error"]["fields"]["cause"]["kind"], "float");
        assert_eq!(
            payload["error"]["fields"]["cause"]["fields"]["cause"]["kind"],
            "invalid_input"
        );
        assert_eq!(
            payload["error"]["fields"]["cause"]["fields"]["cause"]["fields"],
            serde_json::json!({
                "field": "ppp fixed ambiguity wavelength_m",
                "reason": "not finite"
            })
        );
        let mut legacy_buf = vec![0 as std::os::raw::c_char; 512];
        let needed = crate::sidereon_last_error_message(legacy_buf.as_mut_ptr(), legacy_buf.len());
        assert!(needed > 0);
        let legacy = CStr::from_ptr(legacy_buf.as_ptr())
            .to_str()
            .expect("UTF-8 legacy diagnostic");
        assert!(legacy.contains("ppp fixed ambiguity wavelength_m"));
        assert!(legacy.contains("not finite"));

        let mut fixed_position = [0.0; 3];
        assert_eq!(
            sidereon_ppp_fixed_solution_position(
                fixed,
                fixed_position.as_mut_ptr(),
                fixed_position.len(),
            ),
            SidereonStatus::Ok
        );
        let (after_fixed_getter_info, after_fixed_getter_payload) =
            snapshot_engine_error_for_test().expect("PPP error retained across fixed getter");
        assert_eq!(after_fixed_getter_info.family, base_info.family);
        assert_eq!(after_fixed_getter_info.payload_len, base_info.payload_len);
        assert_eq!(
            after_fixed_getter_payload.as_bytes(),
            base_payload.as_bytes()
        );

        let mut float_position = [0.0; 3];
        assert_eq!(
            sidereon_ppp_float_solution_position(
                float,
                float_position.as_mut_ptr(),
                float_position.len(),
            ),
            SidereonStatus::Ok
        );
        let (after_float_getter_info, after_float_getter_payload) =
            snapshot_engine_error_for_test().expect("PPP error retained across float getter");
        assert_eq!(after_float_getter_info.family, base_info.family);
        assert_eq!(after_float_getter_info.payload_len, base_info.payload_len);
        assert_eq!(
            after_float_getter_payload.as_bytes(),
            base_payload.as_bytes()
        );

        sidereon_ppp_fixed_solution_free(fixed);
        let (after_fixed_free_info, after_fixed_free_payload) =
            snapshot_engine_error_for_test().expect("PPP error retained across fixed free");
        assert_eq!(after_fixed_free_info.family, base_info.family);
        assert_eq!(after_fixed_free_info.payload_len, base_info.payload_len);
        assert_eq!(after_fixed_free_payload.as_bytes(), base_payload.as_bytes());

        sidereon_ppp_float_solution_free(float);
        let (after_float_free_info, after_float_free_payload) =
            snapshot_engine_error_for_test().expect("PPP error retained across float free");
        assert_eq!(after_float_free_info.family, base_info.family);
        assert_eq!(after_float_free_info.payload_len, base_info.payload_len);
        assert_eq!(after_float_free_payload.as_bytes(), base_payload.as_bytes());

        sidereon_sp3_free(sp3);
    }
    clear_engine_error();
}

#[test]
fn engine_ppp_matches_committed_fixture() {
    let (_, fixture) = load_fixture();
    let core = core_solutions(&fixture);
    let expected = fixture.get("expected");
    let expected3 = |key: &str| -> [u64; 3] {
        let values = expected.get(key).array();
        assert_eq!(values.len(), 3, "{key}");
        [values[0].f64(), values[1].f64(), values[2].f64()].map(f64::to_bits)
    };
    let stale = "the engine result differs from tests/fixtures/ppp_esbc.json; regenerate it \
                 with the commands in src/ppp_fixture_parity.rs";
    assert_eq!(
        bits3(core.float.position_m),
        expected3("position_m"),
        "float position: {stale}"
    );
    assert_eq!(
        bits3(core.fixed.position_m),
        expected3("fixed_position_m"),
        "fixed position: {stale}"
    );
    assert_eq!(
        bits3(core.fixed.float_solution.position_m),
        expected3("fixed_float_position_m"),
        "fixed embedded float position: {stale}"
    );
    assert_eq!(
        integer_status_name(core.fixed.integer.integer_status),
        expected.get("fixed_integer_status").str(),
        "integer status: {stale}"
    );
    assert_eq!(
        core.fixed.integer.integer_ratio.to_bits(),
        expected.get("fixed_integer_ratio").f64().to_bits(),
        "integer ratio: {stale}"
    );
    assert_eq!(
        i64::try_from(core.fixed.integer.integer_candidates).expect("candidates"),
        expected.get("fixed_integer_candidates").i64(),
        "integer candidates: {stale}"
    );
    let expected_cycles: BTreeMap<String, i64> = expected
        .get("fixed_ambiguities_cycles")
        .object()
        .iter()
        .map(|(id, value)| (id.clone(), value.i64()))
        .collect();
    assert_eq!(
        core.fixed.fixed_ambiguities_cycles, expected_cycles,
        "fixed ambiguity cycles: {stale}"
    );
    let expected_fixed_m: BTreeMap<String, f64> =
        sorted_f64_map(expected.get("fixed_ambiguities_m"))
            .into_iter()
            .collect();
    assert_eq!(
        bits_map(&core.fixed.fixed_ambiguities_m),
        bits_map(&expected_fixed_m),
        "fixed ambiguities in meters: {stale}"
    );
}

// ---------------------------------------------------------------------------
// Generator

/// Shortest text that reads back as the same double. Rust's `{:?}` for f64
/// prints the shortest round-tripping digits, as Python's `repr` does.
fn json_f64(value: f64) -> String {
    assert!(value.is_finite(), "JSON has no non-finite numbers");
    let text = format!("{value:?}");
    assert_eq!(text.parse::<f64>().map(f64::to_bits), Ok(value.to_bits()));
    text
}

fn json_key(key: &str) -> String {
    assert!(
        !key.contains(['"', '\\']) && key.chars().all(|c| !c.is_control()),
        "id {key:?} would need escaping"
    );
    format!("\"{key}\"")
}

fn json_array3(values: [f64; 3]) -> String {
    format!(
        "[\n      {},\n      {},\n      {}\n    ]",
        json_f64(values[0]),
        json_f64(values[1]),
        json_f64(values[2])
    )
}

fn json_object(rows: Vec<(String, String)>) -> String {
    let body: Vec<String> = rows
        .into_iter()
        .map(|(key, value)| format!("      {}: {value}", json_key(&key)))
        .collect();
    format!("{{\n{}\n    }}", body.join(",\n"))
}

/// The `expected` block, laid out as the rest of the fixture is (two-space
/// indent, keys in the order `gen_ppp_fixture_header.py` documents).
fn expected_block(core: &CoreSolutions) -> String {
    let fixed = &core.fixed;
    let fields = [
        (
            "core_revision",
            format!("\"{}\"", include_str!("../tests/CORE_REVISION").trim()),
        ),
        ("position_m", json_array3(core.float.position_m)),
        ("fixed_position_m", json_array3(fixed.position_m)),
        (
            "fixed_float_position_m",
            json_array3(fixed.float_solution.position_m),
        ),
        (
            "fixed_integer_status",
            format!("\"{}\"", integer_status_name(fixed.integer.integer_status)),
        ),
        ("fixed_integer_ratio", json_f64(fixed.integer.integer_ratio)),
        (
            "fixed_integer_candidates",
            fixed.integer.integer_candidates.to_string(),
        ),
        (
            "fixed_ambiguities_cycles",
            json_object(
                fixed
                    .fixed_ambiguities_cycles
                    .iter()
                    .map(|(id, cycles)| (id.clone(), cycles.to_string()))
                    .collect(),
            ),
        ),
        (
            "fixed_ambiguities_m",
            json_object(
                fixed
                    .fixed_ambiguities_m
                    .iter()
                    .map(|(id, value)| (id.clone(), json_f64(*value)))
                    .collect(),
            ),
        ),
    ];
    let body: Vec<String> = fields
        .into_iter()
        .map(|(key, value)| format!("    {}: {value}", json_key(key)))
        .collect();
    format!("  \"expected\": {{\n{}\n  }}\n}}\n", body.join(",\n"))
}

#[test]
#[ignore = "generator: rewrites the expected block of tests/fixtures/ppp_esbc.json"]
fn write_ppp_esbc_expected() {
    let (text, fixture) = load_fixture();
    // `expected` is the fixture's last key; everything before it is kept byte
    // for byte.
    let marker = "\n  \"expected\": {";
    let start = text.rfind(marker).expect("fixture has an expected block") + 1;
    let core = core_solutions(&fixture);
    let rewritten = format!("{}{}", &text[..start], expected_block(&core));

    // The rewritten file must read back as the same inputs and exactly the
    // engine result.
    let reread = JsonReader::parse(&rewritten);
    for key in [
        "source",
        "sp3_file",
        "epochs",
        "initial_state",
        "config",
        "fixed_config",
    ] {
        assert_eq!(
            format!("{:?}", reread.get(key)),
            format!("{:?}", fixture.get(key)),
            "{key} unchanged"
        );
    }
    let expected = reread.get("expected");
    assert_eq!(
        expected.get("position_m").array()[1].f64().to_bits(),
        core.float.position_m[1].to_bits()
    );

    std::fs::write(manifest_path(FIXTURE), rewritten).expect("write PPP fixture");
    eprintln!(
        "wrote expected block of {FIXTURE}: float position bits {:016x?}",
        bits3(core.float.position_m)
    );
}
