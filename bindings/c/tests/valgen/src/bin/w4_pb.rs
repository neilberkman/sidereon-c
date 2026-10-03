//! Engine values phaseb_smoke.c checks, written as tests/w4_pb_pins.h. Each
//! section builds the inputs the C test passes and calls the sidereon-core
//! function the C route calls (src/time.rs, src/signal.rs, src/orbit.rs,
//! src/almanac.rs, src/drag.rs, src/decay.rs, src/sp3.rs, src/dted.rs,
//! src/bias.rs, src/sbas.rs, src/rtcm.rs, src/ssr.rs).

use sidereon_core::astro::anomaly;
use sidereon_core::astro::elements::{coe2rv, ClassicalElements, OrbitType};
use sidereon_core::astro::equinoctial::{self, RetrogradeFactor};
use sidereon_core::astro::frames::transforms::GeodeticStationKm;
use sidereon_core::astro::passes::UtcInstant;
use sidereon_core::astro::relative;
use sidereon_core::astro::state::CartesianState;
use sidereon_core::astro::time::{GnssWeekTow, TimeScale};
use valgen::{
    bits, c_string, c_string_literals, core_fixtures, header_end, header_start, read_bytes,
    tests_path,
};

const P: &str = "W4_PB_";
const GUARD: &str = "SIDEREON_W4_PB_PINS_H";

fn f(name: &str, value: f64) {
    println!("static const uint64_t {P}{name} = {};", bits(value));
}

fn fa(name: &str, values: &[f64]) {
    let joined: Vec<String> = values.iter().map(|v| bits(*v)).collect();
    println!(
        "static const uint64_t {P}{name}[{}] = {{ {} }};",
        values.len(),
        joined.join(", ")
    );
}

fn int(name: &str, value: impl std::fmt::Display) {
    println!("#define {P}{name} ({value})");
}

fn int64(name: &str, value: i64) {
    println!("#define {P}{name} INT64_C({value})");
}

fn flag(name: &str, value: bool) {
    println!("#define {P}{name} {value}");
}

fn text(name: &str, value: &str) {
    println!("static const char {P}{name}[] = {};", c_string(value));
}

/// The C enum constant for a core variant: `prefix` plus the variant's Debug
/// name in upper snake case.
fn variant(prefix: &str, value: impl std::fmt::Debug) -> String {
    let debug = format!("{value:?}");
    let name = debug
        .split(|c: char| c == '(' || c == '{' || c == ' ')
        .next()
        .expect("variant name");
    let mut out = String::from(prefix);
    let mut previous: Option<char> = None;
    for c in name.chars() {
        if c.is_ascii_uppercase()
            && previous.is_some_and(|p| p.is_ascii_lowercase() || p.is_ascii_digit())
        {
            out.push('_');
        }
        out.push(c.to_ascii_uppercase());
        previous = Some(c);
    }
    out
}

fn define(name: &str, value: String) {
    println!("#define {P}{name} {value}");
}

fn labels() {
    println!("/* test_labels: the core label texts. */");
    text("GNSS_GPS_LABEL", sidereon_core::GnssSystem::Gps.as_str());
    text(
        "CARRIER_L1_LABEL",
        sidereon_core::frequencies::CarrierBand::L1.as_str(),
    );
    println!();
}

/// phaseb_smoke.c sample_coe.
fn sample_coe() -> ClassicalElements {
    let a = 7000.0;
    let ecc = 0.01;
    ClassicalElements {
        p: a * (1.0 - ecc * ecc),
        a,
        ecc,
        incl: 0.3,
        raan: 0.2,
        argp: 0.4,
        nu: 0.5,
        arglat: f64::NAN,
        truelon: f64::NAN,
        lonper: f64::NAN,
        orbit_type: OrbitType::EllipticalInclined,
    }
}

fn anomalies() {
    println!("/* test_anomaly_and_equinoctial: sidereon_core::astro::anomaly, elements and");
    println!(" * equinoctial. */");
    let mu = 398600.4418;
    let (ecc, mean) = (0.1, 0.75);
    let e_anom = anomaly::mean_to_eccentric(mean, ecc).expect("M->E");
    f("E_ANOM_BITS", e_anom);
    f(
        "MEAN_ROUND_BITS",
        anomaly::eccentric_to_mean(e_anom, ecc).expect("E->M"),
    );
    let true_anom = anomaly::eccentric_to_true(e_anom, ecc).expect("E->nu");
    f("TRUE_ANOM_BITS", true_anom);
    f(
        "TRUE_ROUND_BITS",
        anomaly::true_to_eccentric(true_anom, ecc).expect("nu->E"),
    );
    let solved = anomaly::solve_kepler(mean, ecc).expect("Kepler");
    f("KEPLER_ANOMALY_BITS", solved.anomaly);
    int("KEPLER_ITERATIONS", solved.iterations);

    let coe = sample_coe();
    let propagated = anomaly::propagate_kepler(&coe, mu, 0.0).expect("propagate Kepler");
    f("PROPAGATED_NU_BITS", propagated.nu);
    let prograde = RetrogradeFactor::Prograde;
    let eq = equinoctial::coe2eq(&coe, prograde).expect("coe->eq");
    f(
        "EQ_ROUND_A_BITS",
        equinoctial::eq2coe(&eq).expect("eq->coe").a,
    );
    let mee = equinoctial::coe2mee(&coe, prograde).expect("coe->mee");
    f(
        "MEE_ROUND_ECC_BITS",
        equinoctial::mee2coe(&mee).expect("mee->coe").ecc,
    );
    let (r, v) = coe2rv(&coe, mu).expect("coe->rv");
    let eq_rv = equinoctial::rv2eq(r, v, mu, prograde).expect("rv->eq");
    let (r2, _) = equinoctial::eq2rv(&eq_rv, mu).expect("eq->rv");
    f("EQ_RV_X_BITS", r2[0]);
    let mee_rv = equinoctial::rv2mee(r, v, mu, prograde).expect("rv->mee");
    let (_, v2) = equinoctial::mee2rv(&mee_rv, mu).expect("mee->rv");
    f("MEE_RV_VY_BITS", v2[1]);
    println!();
}

fn angles_relative() {
    use sidereon_core::astro::angles;
    println!("/* test_angles_and_relative: sidereon_core::astro::angles and relative. */");
    f(
        "SEPARATION_VECTORS_BITS",
        angles::angular_separation([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]).expect("separation"),
    );
    f(
        "SEPARATION_COORDS_BITS",
        angles::angular_separation_coords((0.0, 0.0), (90.0, 0.0)).expect("separation"),
    );
    f(
        "POSITION_ANGLE_BITS",
        angles::position_angle((0.0, 0.0), (90.0, 0.0)).expect("position angle"),
    );
    f(
        "BETA_ANGLE_BITS",
        angles::beta_angle([0.0, 0.0, 1.0], [1.0, 0.0, 0.0]).expect("beta angle"),
    );
    let chief = CartesianState::new(0.0, [7000.0, 0.0, 0.0], [0.0, 7.5, 0.0]);
    let deputy = CartesianState::new(0.0, [7000.0 + 1.0, 0.0 + 2.0, 0.0], [0.0, 7.5 + 0.01, 0.0]);
    let rotation = relative::rtn_to_inertial_rotation(&chief).expect("RTN rotation");
    let flat: Vec<f64> = rotation.iter().flatten().copied().collect();
    fa("RTN_ROTATION_BITS", &flat);
    let rel = relative::relative_state(&chief, &deputy).expect("relative state");
    let recovered = relative::absolute_from_relative(&chief, &rel).expect("absolute");
    fa("RECOVERED_POSITION_BITS", &recovered.position_array());
    let n = relative::mean_motion_circular(7000.0).expect("mean motion");
    f("MEAN_MOTION_CIRCULAR_BITS", n);
    f(
        "MEAN_MOTION_STATE_BITS",
        relative::mean_motion_from_state(&chief).expect("mean motion"),
    );
    let stm = relative::cw_stm(n, 0.0).expect("CW STM");
    f("CW_STM_00_BITS", stm[0][0]);
    let propagated = relative::cw_propagate(&rel, n, 0.0).expect("CW propagate");
    fa("REL_POSITION_BITS", &rel.position_array());
    fa("CW_PROPAGATED_POSITION_BITS", &propagated.position_array());
    println!();
}

fn observe_almanac(spk: &sidereon_core::astro::Spk) {
    use sidereon_core::astro::almanac::{
        lunar_solar_eclipses, meridian_transits, moon_phases, planetary_events, seasons,
        EphemerisSource, Planet, PlanetaryEventKind, TransitBody,
    };
    use sidereon_core::astro::bodies::{observe, observe_spk_body, ObserveOptions, Target};
    println!("/* test_observe_and_almanac: sidereon_core::astro::bodies and almanac, with");
    println!(" * the core fixture almanac/almanac_de421.spk. */");
    let station = GeodeticStationKm {
        latitude_deg: 51.4779,
        longitude_deg: 0.0,
        altitude_km: 0.0,
    };
    let jan1 = 1735689600000000_i64;
    let feb15 = 1739577600000000_i64;
    let apr1 = 1743465600000000_i64;
    let at = UtcInstant::from_unix_microseconds;
    let sun = observe(&station, at(jan1), Target::Sun, ObserveOptions::default()).expect("Sun");
    f("SUN_APPARENT_RA_BITS", sun.apparent.right_ascension_deg);
    f("SUN_APPARENT_DISTANCE_BITS", sun.apparent.distance_km);
    let moon = observe(&station, at(jan1), Target::Moon, ObserveOptions::default()).expect("Moon");
    f("MOON_AZIMUTH_BITS", moon.horizontal.azimuth_deg);
    let body = observe_spk_body(&station, at(jan1), spk, 4).expect("SPK body");
    f(
        "SPK_BODY_ASTROMETRIC_DISTANCE_BITS",
        body.astrometric.distance_km,
    );

    let analytic = || EphemerisSource::Analytic;
    let season_events = seasons(analytic(), at(jan1), at(apr1), 86400.0, 60.0).expect("seasons");
    int("SEASON_COUNT", season_events.len());
    int64(
        "SEASON0_TIME_UNIX_US",
        season_events[0].time.unix_microseconds(),
    );
    let phases = moon_phases(analytic(), at(jan1), at(feb15), 21600.0, 60.0).expect("phases");
    int("MOON_PHASE_COUNT", phases.len());
    int64(
        "MOON_PHASE0_TIME_UNIX_US",
        phases[0].time.unix_microseconds(),
    );
    let planet = planetary_events(
        EphemerisSource::Spk(spk),
        Planet::Mars,
        PlanetaryEventKind::Opposition,
        at(jan1),
        at(feb15),
        21600.0,
        60.0,
    )
    .expect("planetary events");
    int("PLANETARY_EVENT_COUNT", planet.len());
    let transits = meridian_transits(
        analytic(),
        TransitBody::Sun,
        &station,
        at(jan1),
        at(jan1 + 86400000000),
        3600.0,
        10.0,
    )
    .expect("transits");
    int("TRANSIT_COUNT", transits.len());
    int64(
        "TRANSIT0_TIME_UNIX_US",
        transits[0].time.unix_microseconds(),
    );
    let eclipses =
        lunar_solar_eclipses(analytic(), at(jan1), at(apr1), 86400.0, 60.0).expect("eclipses");
    int("ECLIPSE_COUNT", eclipses.len());
    println!();
}

fn drag_decay() {
    use sidereon_core::astro::forces::drag::{DragParameters, SpaceWeather};
    use sidereon_core::astro::forces::ForceModel;
    use sidereon_core::astro::propagator::{
        estimate_decay, DecayConfig, IntegratorKind, PropagationContext, PropagationForceModel,
    };
    println!("/* test_drag_decay: sidereon_core::astro::forces::drag and");
    println!(" * propagator::estimate_decay, with the configuration the binding forms. */");
    let weather = SpaceWeather::default();
    let drag = DragParameters::from_area_mass(2.2, 20.0, 100.0, weather, 90.0).expect("drag");
    f("DRAG_BC_FACTOR_BITS", drag.bc_factor_m2_kg());
    // The C struct round trip (drag_parameters_to_c / drag_parameters_from_c).
    let drag = DragParameters::from_bc_factor_m2_kg(
        drag.bc_factor_m2_kg(),
        drag.space_weather(),
        drag.cutoff_altitude_km(),
    )
    .expect("drag round trip");
    let state = CartesianState::new(0.0, [6778.0, 0.0, 0.0], [0.0, 7.67, 0.0]);
    let accel = drag
        .to_force()
        .acceleration(&state, &PropagationContext::default())
        .expect("drag acceleration");
    fa("DRAG_ACCEL_BITS", accel.as_slice());
    // sidereon_decay_config_init then decay_config_from_c, with the test's
    // drag and a 500 km reentry altitude.
    let defaults = DecayConfig::new(drag);
    let mut options = defaults.options;
    options.dense_output = false;
    let config = DecayConfig::new(drag)
        .with_force_model(PropagationForceModel::TwoBodyJ2)
        .with_mu_km3_s2(defaults.mu_km3_s2)
        .with_integrator(IntegratorKind::Dp54)
        .with_options(options)
        .with_reentry_altitude_km(500.0)
        .with_scan_step_s(defaults.scan_step_s)
        .with_crossing_tolerance_s(defaults.crossing_tolerance_s)
        .with_max_duration_s(defaults.max_duration_s)
        .with_max_scan_samples(defaults.max_scan_samples);
    let estimate = estimate_decay(state, &config).expect("decay estimate");
    f("DECAY_TIME_BITS", estimate.time_to_decay_s);
    println!();
}

fn ephemeris_sample() {
    use sidereon_core::ephemeris::{sample, Sp3, Sp3InterpolationOptions};
    println!("/* test_ephemeris_sample: sidereon_core::ephemeris::sample on the SP3 the C");
    println!(" * test loads (fixtures/sp3/GRG0MGXFIN_20201760000_01D_15M_ORB.SP3). */");
    let sp3 = Sp3::parse(&read_bytes(&tests_path(
        "fixtures/sp3/GRG0MGXFIN_20201760000_01D_15M_ORB.SP3",
    )))
    .expect("SP3")
    .with_interpolation_options(Sp3InterpolationOptions::default());
    let epochs = sp3.epochs_j2000_seconds();
    int("SP3_EPOCH_COUNT", epochs.len());
    let t = epochs[0];
    f("SP3_EPOCH0_BITS", t);
    let rows = sample(&sp3, &["G01".parse().expect("G01")], t, t, 60.0).expect("sample");
    int("SAMPLE_ROW_COUNT", rows.len());
    define(
        "SAMPLE_ROW0_STATUS",
        variant("SIDEREON_EPHEMERIS_SAMPLE_STATUS_", rows[0].status),
    );
    flag(
        "SAMPLE_ROW0_HAS_POSITION",
        rows[0].position_ecef_m.is_some(),
    );
    fa(
        "SAMPLE_ROW0_POSITION_BITS",
        &rows[0].position_ecef_m.unwrap_or([0.0; 3]),
    );
    println!();
}

fn terrain(root: &str) {
    use sidereon_core::terrain::{DtedLookupOptions, DtedTerrain, DtedTile};
    println!("/* test_terrain: sidereon_core::terrain on the core fixture dted/tiles. */");
    let options = DtedLookupOptions::default();
    define(
        "DTED_DEFAULT_INTERPOLATION",
        variant("SIDEREON_DTED_INTERPOLATION_", options.interpolation),
    );
    let mut terrain = DtedTerrain::new(root);
    f(
        "DTED_HEIGHT_BITS",
        terrain
            .height_m_with_options(-106.5, 36.5, options)
            .expect("DTED height"),
    );
    let tile = DtedTile::from_path(format!("{root}/n36_w107_1arc_v3.dt2")).expect("DTED tile");
    int(
        "DTED_TILE_ELEVATION",
        tile.get_elevation(-106.5, 36.5).expect("tile elevation"),
    );
    println!();
}

fn lookup(prefix: &str, value: &sidereon_core::bias::BiasLookup) {
    use sidereon_core::bias::BiasLookup;
    define(
        &format!("{prefix}_STATUS"),
        variant("SIDEREON_BIAS_LOOKUP_STATUS_", value),
    );
    if let BiasLookup::Available { value, records, .. } = value {
        f(&format!("{prefix}_VALUE_BITS"), *value);
        int(&format!("{prefix}_RECORD_COUNT"), records.len());
        int(&format!("{prefix}_RECORD0"), records[0]);
    }
}

fn biases(core: &str) {
    use sidereon_core::bias::{
        bias_epoch_instant, BiasDeparture, BiasEpoch, BiasNotice, BiasReadPolicy, BiasSet,
        CodeDcbOptions,
    };
    println!("/* test_biases: sidereon_core::bias on the test's EDGE_BIA text and the core");
    println!(" * fixture bias/P1C1_RINEX.DCB. */");
    let edge = c_string_literals(
        &tests_path("phaseb_smoke.c"),
        "static const uint8_t EDGE_BIA[] =",
    );
    flag(
        "EDGE_STRICT_OK",
        BiasSet::parse_bias_sinex(edge.as_bytes()).is_ok(),
    );
    let set = BiasSet::parse_bias_sinex_with_policy(edge.as_bytes(), BiasReadPolicy::Lenient)
        .expect("lenient Bias-SINEX")
        .value;
    int("EDGE_RECORD_COUNT", set.records().len());
    int("EDGE_NOTICE_COUNT", set.notices().len());
    let mut missing_footer = false;
    let mut suffix_line = 0;
    let mut header_layout_reason_len = 0;
    for notice in set.notices() {
        if let BiasNotice::Departure(departure) = notice {
            match departure {
                BiasDeparture::MissingFooter => missing_footer = true,
                BiasDeparture::BlockStartSuffix { line } => suffix_line = *line,
                BiasDeparture::HeaderLayout { reason } => {
                    header_layout_reason_len = reason.to_string().len()
                }
                _ => {}
            }
        }
    }
    flag("EDGE_MISSING_FOOTER", missing_footer);
    flag("EDGE_HAS_BLOCK_START_SUFFIX", suffix_line != 0);
    int("EDGE_HEADER_LAYOUT_REASON_LEN", header_layout_reason_len);
    define("EDGE_MODE", variant("SIDEREON_BIAS_MODE_", set.mode()));
    let scale = set.time_scale();
    flag("EDGE_HAS_TIME_SCALE", scale.is_some());
    define(
        "EDGE_TIME_SCALE",
        variant("SIDEREON_TIME_SCALE_", scale.expect("time scale")),
    );
    let instant = bias_epoch_instant(
        BiasEpoch::new(2020, 1, 43200).expect("epoch"),
        scale.expect("time scale"),
    )
    .expect("instant");
    let g01 = "G01".parse().expect("G01");
    let g02 = "G02".parse().expect("G02");
    let osb = set.code_osb_seconds(g01, "C1C", instant);
    lookup("EDGE_CODE_OSB", &osb);
    if let sidereon_core::bias::BiasLookup::Available { records, .. } = &osb {
        let record = &set.records()[records[0]];
        flag("EDGE_CODE_OSB_RECORD_HAS_SLOPE", record.slope.is_some());
        define(
            "EDGE_CODE_OSB_RECORD_FAMILY",
            variant("SIDEREON_BIAS_OBSERVABLE_FAMILY_", record.family),
        );
        define(
            "EDGE_CODE_OSB_RECORD_UNIT",
            variant("SIDEREON_BIAS_UNIT_", record.unit),
        );
        flag("EDGE_CODE_OSB_RECORD_HAS_LINE", record.line.is_some());
    }
    lookup(
        "EDGE_PHASE_OSB",
        &set.phase_osb_cycles(g01, "L1C", instant, None),
    );
    lookup(
        "EDGE_CODE_DSB",
        &set.code_dsb_seconds(g01, "C1C", "C1W", instant),
    );
    lookup(
        "EDGE_CODE_OSB_G02_C1C",
        &set.code_osb_seconds(g02, "C1C", instant),
    );
    lookup(
        "EDGE_CODE_OSB_G02_C1W",
        &set.code_osb_seconds(g02, "C1W", instant),
    );

    let options = CodeDcbOptions::new(
        ("P1".to_string(), "C1".to_string()),
        2026,
        6,
        TimeScale::Gpst,
    );
    let dcb_bytes = read_bytes(&format!("{core}/bias/P1C1_RINEX.DCB"));
    let dcb = BiasSet::parse_code_dcb(&dcb_bytes, Some(options.clone()))
        .expect("CODE DCB")
        .value;
    let dcb_scale = dcb.time_scale().expect("DCB time scale");
    let dcb_instant = bias_epoch_instant(BiasEpoch::new(2026, 153, 0).expect("epoch"), dcb_scale)
        .expect("instant");
    lookup(
        "DCB_CODE_DSB",
        &dcb.code_dsb_seconds(g01, "C1W", "C1C", dcb_instant),
    );
    flag(
        "DCB_STRICT_OK",
        BiasSet::parse_code_dcb_with_policy(&dcb_bytes, Some(options), BiasReadPolicy::Strict)
            .is_ok(),
    );

    // The gzip product the C test loads with sidereon_bias_sinex_load_with_policy
    // under the lenient policy; the binding's reader decompresses every gzip
    // member, as MultiGzDecoder does.
    let gz = read_bytes(&format!(
        "{core}/bias/COD0OPSFIN_20261330000_01D_01D_OSB.BIA.gz"
    ));
    let mut text = Vec::new();
    std::io::Read::read_to_end(&mut flate2::read::MultiGzDecoder::new(&gz[..]), &mut text)
        .expect("decompress BIA.gz");
    let lossy = BiasSet::parse_bias_sinex_with_policy(&text, BiasReadPolicy::Lenient)
        .expect("lenient BIA.gz")
        .value;
    int("GZ_RECORD_COUNT", lossy.records().len());
    int("GZ_NOTICE_COUNT", lossy.notices().len());
    define("GZ_MODE", variant("SIDEREON_BIAS_MODE_", lossy.mode()));
    println!();
}

fn sbas() {
    use sidereon_core::sbas::{SbasBlock, SbasCorrectionStore, SbasWireForm};
    println!("/* test_sbas: sidereon_core::sbas decode and correction store. */");
    let hex = "5366819010029EE7ED83018202819BBE1A08BF8008FFA00000004066C0";
    let body: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("hex"))
        .collect();
    let block = SbasBlock::decode(&body, SbasWireForm::Body226).expect("SBAS block");
    define(
        "SBAS_KIND",
        variant("SIDEREON_SBAS_MESSAGE_KIND_", &block.message),
    );
    let long_term_count: usize = match &block.message {
        sidereon_core::sbas::SbasMessage::LongTermCorrections(m) => {
            m.halves.iter().map(|h| h.records.len()).sum()
        }
        sidereon_core::sbas::SbasMessage::MixedCorrections(m) => m.long_term.records.len(),
        _ => 0,
    };
    int("SBAS_LONG_TERM_COUNT", long_term_count);
    let mut store = SbasCorrectionStore::new();
    let epoch = GnssWeekTow {
        system: TimeScale::Gpst,
        week: 2400,
        tow_s: 20.0,
    };
    flag(
        "SBAS_INGEST_OK",
        store
            .ingest(&block.message, "S20".parse().expect("S20"), epoch)
            .is_ok(),
    );
    flag(
        "SBAS_PREFERRED_GEO_PRESENT",
        !store.ready_geos(0.0).is_empty(),
    );
    println!();
}

fn ssr() {
    use sidereon_core::rtcm::{decode_messages, Message, RtcmPolicy, SsrStreamAssembler};
    use sidereon_core::ssr::{SsrCorrectionStore, SsrNavigationMessage};
    println!("/* test_ssr: sidereon_core::rtcm and ssr on the test's RTCM frame. */");
    // The RTCM 1060 frame test_ssr decodes (phaseb_smoke.c test_ssr).
    let hex = "d3003c4245438a3040000827968003270026dffea30000f7fff6ffff0000530000000000003e87fff8effc94002c7ffff57fffc80003004128000000000000625cf0";
    let frame: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("hex"))
        .collect();
    let messages = decode_messages(&frame).expect("RTCM frame");
    int("RTCM_MESSAGE_COUNT", messages.len());
    define(
        "RTCM_MESSAGE0_KIND",
        variant("SIDEREON_RTCM_MESSAGE_KIND_", &messages[0]),
    );
    int("RTCM_MESSAGE0_NUMBER", messages[0].message_number());
    let Message::Ssr(message) = &messages[0] else {
        panic!("frame is not an SSR message");
    };
    define("SSR_KIND", variant("SIDEREON_RTCM_SSR_KIND_", message.kind));
    int("SSR_ORBIT_COUNT", message.orbit.len());
    int("SSR_CLOCK_COUNT", message.clock.len());

    let week = GnssWeekTow {
        system: TimeScale::Gpst,
        week: 2425,
        tow_s: 344970.0,
    };
    // sidereon::ssr_store_from_rtcm_strict, as the binding calls it.
    let strict = |bytes: &[u8]| -> Result<SsrCorrectionStore, String> {
        let mut store = SsrCorrectionStore::new();
        let mut assembler = SsrStreamAssembler::new();
        let mut decoded = assembler.push(bytes);
        decoded.extend(assembler.finish());
        for decoded in decoded {
            let message = decoded.map_err(|e| e.to_string())?;
            store.ingest(&message, week).map_err(|e| e.to_string())?;
        }
        if assembler.diagnostics().resync_bytes > 0 {
            return Err("bytes outside CRC-valid frames".to_string());
        }
        Ok(store)
    };
    let store = strict(&frame).expect("strict SSR store");
    let g30 = "G30".parse().expect("G30");
    let orbit = store.orbit(g30);
    flag("SSR_G30_ORBIT_PRESENT", orbit.is_some());
    f(
        "SSR_G30_RADIAL_BITS",
        orbit.map_or(0.0, |orbit| orbit.radial_m),
    );
    let clock = store.clock(g30);
    flag("SSR_G30_CLOCK_PRESENT", clock.is_some());
    f("SSR_G30_C0_BITS", clock.map_or(0.0, |clock| clock.c0_m));
    flag(
        "SSR_STRICT_PARTIAL_OK",
        strict(&frame[..frame.len() - 1]).is_ok(),
    );

    // sidereon::ssr_store_from_rtcm, the lenient reading route.
    let mut reading = SsrCorrectionStore::new();
    let mut assembler = SsrStreamAssembler::with_policy(RtcmPolicy::Lenient);
    let mut decoded = assembler.push(&frame);
    let trailing = assembler.retained_len();
    decoded.extend(assembler.finish());
    let mut refusals = 0;
    for message in decoded.into_iter().flatten() {
        if reading.ingest(&message, week).is_err() {
            refusals += 1;
        }
    }
    int("SSR_READING_TRAILING", trailing);
    int("SSR_READING_REFUSALS", refusals);
    let orbit = reading.orbit(g30);
    flag("SSR_READING_G30_ORBIT_PRESENT", orbit.is_some());
    flag(
        "SSR_READING_G30_HAS_NAV_MESSAGE",
        orbit.is_some_and(|orbit| matches!(orbit.nav_message, SsrNavigationMessage::Has(_))),
    );
    println!();
}

fn main() {
    let core = core_fixtures();
    let spk = sidereon_core::astro::Spk::from_bytes(&read_bytes(&format!(
        "{core}/almanac/almanac_de421.spk"
    )))
    .expect("SPK");
    header_start("w4_pb", GUARD);
    labels();
    anomalies();
    angles_relative();
    observe_almanac(&spk);
    drag_decay();
    ephemeris_sample();
    terrain(&format!("{core}/dted/tiles"));
    biases(&core);
    sbas();
    ssr();
    header_end(GUARD);
}
