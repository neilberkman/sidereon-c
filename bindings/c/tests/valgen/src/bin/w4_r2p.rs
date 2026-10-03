//! Engine values round2_parity_smoke.c checks, written as
//! tests/w4_r2p_pins.h. Each section builds the inputs the C test passes and
//! calls the sidereon-core function the C route calls.

use sidereon_core::astro::covariance::Covariance6;
use sidereon_core::astro::propagator::{transport_covariance, CovarianceSegment, ProcessNoise};
use sidereon_core::astro::state::CartesianState;
use sidereon_core::astro::time::civil::j2000_seconds;
use sidereon_core::astro::time::{GnssWeekTow, TimeScale};
use sidereon_core::rinex::nav::{
    cnav_ura_ned_m, cnav_ura_nominal_m, BroadcastEphemeris, BroadcastGroupDelayTerm, CnavSignal,
    NavMessage,
};
use valgen::{bits, c_string, core_fixtures, header_end, header_start, read};

const P: &str = "W4_R2P_";
const GUARD: &str = "SIDEREON_W4_R2P_PINS_H";

fn f(name: &str, value: f64) {
    println!("static const uint64_t {P}{name} = {};", bits(value));
}

fn int(name: &str, value: impl std::fmt::Display) {
    println!("#define {P}{name} ({value})");
}

fn flag(name: &str, value: bool) {
    println!("#define {P}{name} {value}");
}

fn text(name: &str, value: &str) {
    println!("static const char {P}{name}[] = {};", c_string(value));
    println!("#define {P}{name}_LEN ((size_t){})", value.len());
}

/// FNV-1a 64 of `data`, the hash the C tests recompute over returned bytes.
fn fnv1a64(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in data {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn hash(name: &str, data: &[u8]) {
    println!("#define {P}{name} UINT64_C({:#018x})", fnv1a64(data));
}

fn bytes_text(name: &str, value: &[u8]) {
    text(name, std::str::from_utf8(value).expect("ASCII text"));
}

/// The C enum constant for a core variant: `prefix` plus the variant's Debug
/// name in upper snake case (QzssCnav2 -> QZSS_CNAV2).
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

fn covariance() {
    let mut p0 = [[0.0; 6]; 6];
    let diag = [1.0, 2.0, 3.0, 0.01, 0.02, 0.03];
    for (i, value) in diag.iter().enumerate() {
        p0[i][i] = *value;
    }
    let mut stm = [[0.0; 6]; 6];
    for (i, row) in stm.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    let segment = CovarianceSegment {
        stm,
        dt_seconds: 10.0,
        q_rotation_state: CartesianState::new(0.0, [7000.0, 0.0, 0.0], [0.0, 7.5, 0.0]),
    };
    let noise = ProcessNoise::RtnAccelerationPsd {
        q_radial_km2_s3: 1.0e-6,
        q_transverse_km2_s3: 2.0e-6,
        q_normal_km2_s3: 3.0e-6,
    };
    let out = transport_covariance(
        Covariance6::try_from_matrix(p0).expect("P0"),
        &[segment],
        noise,
    )
    .expect("transport covariance");
    println!("/* test_covariance: sidereon_core::astro::propagator::transport_covariance. */");
    int("COV_TRANSPORT_COUNT", out.len());
    let m = out[1].as_matrix();
    let values: Vec<String> = m.iter().flatten().map(|v| bits(*v)).collect();
    println!(
        "static const uint64_t {P}COV_TRANSPORT_1_BITS[36] = {{ {} }};",
        values.join(", ")
    );
    println!();
}

fn cnav(core: &str) {
    let text_nav = read(&format!(
        "{core}/nav/BRD400DLR_S_20261800000_01H_MN_trim.rnx"
    ));
    let nav = BroadcastEphemeris::from_nav(&text_nav).expect("CNAV NAV");
    let records = nav.records();
    println!("/* test_cnav: sidereon_core::rinex::nav::BroadcastEphemeris::from_nav of");
    println!(" * nav/BRD400DLR_S_20261800000_01H_MN_trim.rnx and its records. */");
    int("CNAV_RECORD_COUNT", records.len());
    let j02: sidereon_core::GnssSatelliteId = "J02".parse().expect("J02");
    let find = |message: NavMessage| {
        records
            .iter()
            .position(|r| r.satellite_id == j02 && r.message == message)
            .expect("J02 record")
    };
    let cnav_index = find(NavMessage::QzssCnav);
    let cnav2_index = find(NavMessage::QzssCnav2);
    let r = &records[cnav_index];
    let c = r.cnav.expect("J02 CNAV parameters");
    flag("J02_CNAV_HAS_ISSUE", r.issue_of_data.is_some());
    int("J02_CNAV_WEEK", r.week);
    int("J02_CNAV_TOE_WEEK", r.toe.week);
    f("J02_CNAV_TOE_TOW_S_BITS", r.toe.tow_s);
    flag("J02_CNAV_PRESENT", r.cnav.is_some());
    int("J02_CNAV_URA_ED_INDEX", c.ura_ed_index);
    int("J02_CNAV_URA_NED0_INDEX", c.ura_ned0_index);
    int("J02_CNAV_URA_NED1_INDEX", c.ura_ned1_index);
    int("J02_CNAV_URA_NED2_INDEX", c.ura_ned2_index);
    f("J02_CNAV_ADOT_BITS", c.adot_m_s);
    f("J02_CNAV_DN0_DOT_BITS", c.delta_n0_dot_rad_s2);
    let ura0 = cnav_ura_nominal_m(0);
    flag("CNAV_URA_NOMINAL_0_PRESENT", ura0.is_some());
    f("CNAV_URA_NOMINAL_0_BITS", ura0.unwrap_or(0.0));
    // The C test queries the NED URA at week 2425, 86400 s.
    let ned = cnav_ura_ned_m(
        &c,
        GnssWeekTow {
            system: TimeScale::Gpst,
            week: 2425,
            tow_s: 86400.0,
        },
    );
    flag("J02_CNAV_URA_NED_PRESENT", ned.is_some());
    f("J02_CNAV_URA_NED_BITS", ned.unwrap_or(0.0));
    let isc = r.group_delays.get(BroadcastGroupDelayTerm::CnavIscL2C);
    flag("J02_CNAV_ISC_L2C_PRESENT", isc.is_some());
    f("J02_CNAV_ISC_L2C_BITS", isc.unwrap_or(0.0));
    let corr = r
        .group_delays
        .cnav_single_frequency_correction_s(CnavSignal::L1Ca);
    flag("J02_CNAV_L1CA_CORRECTION_PRESENT", corr.is_some());
    f("J02_CNAV_L1CA_CORRECTION_BITS", corr.unwrap_or(0.0));

    let r2 = &records[cnav2_index];
    let c2 = r2.cnav.expect("J02 CNAV2 parameters");
    flag("J02_CNAV2_HAS_ISSUE", r2.issue_of_data.is_some());
    flag("J02_CNAV2_PRESENT", r2.cnav.is_some());
    f("J02_CNAV2_TRANSMISSION_SOW_BITS", c2.transmission_time_sow);
    let isc2 = r2.group_delays.get(BroadcastGroupDelayTerm::CnavIscL1Cp);
    flag("J02_CNAV2_ISC_L1CP_PRESENT", isc2.is_some());
    f("J02_CNAV2_ISC_L1CP_BITS", isc2.unwrap_or(0.0));
    let corr2 = r2
        .group_delays
        .cnav_single_frequency_correction_s(CnavSignal::L1Cp);
    flag("J02_CNAV2_L1CP_CORRECTION_PRESENT", corr2.is_some());
    f("J02_CNAV2_L1CP_CORRECTION_BITS", corr2.unwrap_or(0.0));
    // select_by_issue for J02, issue 288, QZSS CNAV2, 2026-06-29 00:00:00.
    let selected = nav.select_by_issue_at(
        j02,
        sidereon_core::ephemeris::BroadcastIssue {
            issue: 288,
            message: NavMessage::QzssCnav2,
        },
        NavMessage::QzssCnav2,
        j2000_seconds(2026, 6, 29, 0, 0, 0.0),
    );
    flag("J02_CNAV2_SELECT_BY_ISSUE_PRESENT", selected.is_some());
    println!();
}

fn qc(core: &str) {
    use sidereon_core::rinex::observations::RinexObs;
    let obs_text = read(&format!(
        "{core}/obs/ESBC00DNK_R_20201770000_01D_30S_MO_120epoch.rnx"
    ));
    let obs = RinexObs::parse(&obs_text).expect("QC observation file");
    let report = sidereon_core::observation_qc::observation_qc_with_options(
        &obs,
        sidereon_core::observation_qc::ObservationQcOptions::default(),
    )
    .expect("observation QC");
    println!("/* test_qc: sidereon_core::observation_qc::observation_qc_with_options under the");
    println!(" * default options (sidereon_observation_qc_options_init). */");
    int("QC_TOTAL_EPOCH_RECORDS", report.total_epoch_records);
    int("QC_OBSERVATION_EPOCHS", report.observation_epochs);
    int("QC_EVENT_RECORDS", report.event_records);
    int("QC_SKIPPED_RECORDS", report.skipped_records);
    flag("QC_HAS_INTERVAL_S", report.interval_s.is_some());
    f("QC_INTERVAL_S_BITS", report.interval_s.unwrap_or(0.0));
    int("QC_MISSING_EPOCHS", report.missing_epochs);
    int("QC_DATA_GAP_COUNT", report.data_gaps.len());
    int("QC_CLOCK_JUMP_COUNT", report.clock_jumps.len());
    flag(
        "QC_HAS_OBSERVATIONS_PER_SLIP",
        report.cycle_slips.observations_per_slip.is_some(),
    );
    f(
        "QC_OBSERVATIONS_PER_SLIP_BITS",
        report.cycle_slips.observations_per_slip.unwrap_or(0.0),
    );
    for row in &report.cycle_slips.by_system {
        let name = format!("{:?}", row.system).to_uppercase();
        flag(
            &format!("QC_SLIPS_{name}_HAS_OBSERVATIONS_PER_SLIP"),
            row.observations_per_slip.is_some(),
        );
        f(
            &format!("QC_SLIPS_{name}_OBSERVATIONS_PER_SLIP_BITS"),
            row.observations_per_slip.unwrap_or(0.0),
        );
    }
    for row in &report.multipath.systems {
        let name = format!("{:?}", row.system).to_uppercase();
        flag(&format!("QC_MP_{name}_HAS_MP1"), row.mp1.is_some());
        flag(&format!("QC_MP_{name}_HAS_MP2"), row.mp2.is_some());
    }
    let g08 = report
        .multipath
        .satellites
        .iter()
        .find(|row| row.satellite.to_string() == "G08")
        .expect("G08 multipath row");
    flag("QC_MP_G08_HAS_MP1", g08.mp1.is_some());
    flag("QC_MP_G08_HAS_MP2", g08.mp2.is_some());
    // The rendered reports are compared by length and FNV-1a 64 of their bytes.
    let text_report = sidereon_core::observation_qc::render_text(&report);
    let html_report = sidereon_core::observation_qc::render_html(&report);
    // sidereon_observation_qc_to_json serializes the report with serde_json.
    let json_report = serde_json::to_string(&report).expect("QC JSON");
    int("QC_RENDER_TEXT_LEN", text_report.len());
    hash("QC_RENDER_TEXT_FNV1A64", text_report.as_bytes());
    int("QC_RENDER_HTML_LEN", html_report.len());
    hash("QC_RENDER_HTML_FNV1A64", html_report.as_bytes());
    int("QC_TO_JSON_LEN", json_report.len());
    hash("QC_TO_JSON_FNV1A64", json_report.as_bytes());
    println!();

    let crx = read(&format!(
        "{core}/obs/ESBC00DNK_R_20201770000_01D_30S_MO_trim.crx"
    ));
    let lint = sidereon_core::rinex::qc::lint_obs_text(&crx);
    use sidereon_core::rinex::qc::Severity;
    println!("/* test_qc: sidereon_core::rinex::qc::lint_obs_text and repair_obs_text of");
    println!(" * obs/ESBC00DNK_R_20201770000_01D_30S_MO_trim.crx. */");
    int("LINT_FINDING_COUNT", lint.findings.len());
    int("LINT_ERROR_COUNT", lint.count(Severity::Error));
    int("LINT_WARNING_COUNT", lint.count(Severity::Warning));
    flag("LINT_DECODED_FROM_CRINEX", lint.decoded_from_crinex);
    let mut options = sidereon_core::rinex::qc::RepairOptions::default();
    options.set_interval = true;
    options.set_time_of_last_obs = true;
    options.set_obs_counts = true;
    options.drop_empty_records = true;
    options.drop_unsupported = true;
    let repair = sidereon_core::rinex::qc::repair_obs_text(&crx, &options).expect("repair");
    int(
        "REPAIR_REMAINING_FINDING_COUNT",
        repair.remaining.findings.len(),
    );
    flag("REPAIR_DECODED_FROM_CRINEX", repair.decoded_from_crinex);
    let crinex = sidereon_core::rinex::qc::repair_obs_to_crinex_string(&repair)
        .expect("repaired CRINEX text");
    int("REPAIR_CRINEX_LEN", crinex.len());
    println!();
}

fn geoid() {
    let points = [(0.0, 0.0), (0.0, 80.0), (60.0, -30.0)];
    let values = sidereon_core::geoid::egm96_undulations_deg(&points);
    println!("/* test_geoid: sidereon_core::geoid EGM96 routes. */");
    int("EGM96_POINT_COUNT", values.len());
    for (i, value) in values.iter().enumerate() {
        f(&format!("EGM96_UNDULATION_{i}_BITS"), *value);
    }
    let orthometric = sidereon_core::geoid::egm96_orthometric_height_m(100.0, 0.0, 0.0);
    f("EGM96_ORTHOMETRIC_BITS", orthometric);
    f(
        "EGM96_ELLIPSOIDAL_BITS",
        sidereon_core::geoid::egm96_ellipsoidal_height_m(orthometric, 0.0, 0.0),
    );
    println!();
}

fn nmea() {
    let text_nmea = "$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47\r\n\
        $GPGGA,123520,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*4D\r\n";
    let parsed = sidereon_core::nmea::parse_nmea(text_nmea.as_bytes());
    let epochs = sidereon_core::nmea::group_epochs(&parsed.value);
    let mut skips = parsed.diagnostics.skips.len();
    let mut warnings = parsed.diagnostics.warnings.len();
    for epoch in &epochs {
        skips += epoch.diagnostics.skips.len();
        warnings += epoch.diagnostics.warnings.len();
    }
    println!("/* test_nmea: sidereon_core::nmea::parse_nmea, group_epochs, NmeaAccumulator");
    println!(" * and write_gga. */");
    int("NMEA_SENTENCE_COUNT", parsed.value.sentences.len());
    int("NMEA_EPOCH_COUNT", epochs.len());
    int("NMEA_SKIP_COUNT", skips);
    int("NMEA_WARNING_COUNT", warnings);
    let first = &epochs[0];
    let position = first.position();
    flag("NMEA_EPOCH0_HAS_POSITION", position.is_some());
    int("NMEA_EPOCH0_SENTENCE_COUNT", first.sentence_count);
    int(
        "NMEA_EPOCH0_USED_SATELLITE_COUNT",
        first.used_satellites().count(),
    );
    let position = position.expect("first epoch position");
    f("NMEA_EPOCH0_LAT_RAD_BITS", position.lat_rad);
    f("NMEA_EPOCH0_LON_RAD_BITS", position.lon_rad);
    f("NMEA_EPOCH0_HEIGHT_M_BITS", position.height_m);

    // The C test pushes the first 80 bytes, then the rest, then finishes.
    let mut acc = sidereon_core::nmea::NmeaAccumulator::new();
    let bytes = text_nmea.as_bytes();
    let mut sentence_count = 0;
    let mut epoch_count = 0;
    for chunk in [&bytes[..80], &bytes[80..]] {
        let output = acc.push_bytes(chunk);
        sentence_count += output.sentences.len();
        epoch_count += output.snapshots.len();
    }
    if acc.finish().is_some() {
        epoch_count += 1;
    }
    int("NMEA_ACC_SENTENCE_COUNT", sentence_count);
    int("NMEA_ACC_EPOCH_COUNT", epoch_count);

    let lat = 48.1173 * std::f64::consts::PI / 180.0;
    let lon = 11.516666666666667 * std::f64::consts::PI / 180.0;
    let position = sidereon_core::frame::Wgs84Geodetic::new(lat, lon, 592.3).expect("position");
    let time = sidereon_core::nmea::NmeaTime::from_seconds_of_day_floor_centis(
        12.0 * 3600.0 + 35.0 * 60.0 + 19.0,
    )
    .expect("GGA time");
    let gga = sidereon_core::nmea::Gga::vrs_position(
        position,
        time,
        sidereon_core::nmea::GgaQuality::GpsSps,
        8,
        0.9,
        3,
    )
    .expect("GGA");
    let sentence =
        sidereon_core::nmea::write_gga(sidereon_core::nmea::NmeaTalker::parse("GP"), &gga)
            .expect("write GGA");
    text("NMEA_GGA", &sentence);
    println!();
}

fn space_weather(core: &str) {
    use sidereon_core::astro::space_weather::parse;
    let bytes = valgen::read_bytes(&format!("{core}/space_weather/SW-All-20260702-trim.csv"));
    let parsed = parse(&bytes).expect("space-weather CSV");
    let table = &parsed.value;
    println!("/* test_space_weather: sidereon_core::astro::space_weather::parse of");
    println!(" * space_weather/SW-All-20260702-trim.csv. */");
    int("SW_DAY_COUNT", table.days().len());
    int("SW_MONTHLY_COUNT", table.monthly().len());
    int("SW_SKIP_COUNT", parsed.diagnostics.skips.len());
    int("SW_WARNING_COUNT", parsed.diagnostics.warnings.len());
    let coverage = table.coverage();
    flag(
        "SW_HAS_LAST_OBSERVED",
        coverage.last_observed_j2000_s.is_some(),
    );
    flag(
        "SW_HAS_LAST_DAILY_PREDICTED",
        coverage.last_daily_predicted_j2000_s.is_some(),
    );
    let sample = table
        .sample_at(j2000_seconds(2026, 7, 1, 12, 0, 0.0))
        .expect("observed sample");
    f("SW_SAMPLE_F107_BITS", sample.space_weather.f107);
    f("SW_SAMPLE_F107A_BITS", sample.space_weather.f107a);
    f("SW_SAMPLE_AP_BITS", sample.space_weather.ap);
    println!(
        "#define {P}SW_SAMPLE_CLASS {}",
        variant("SW_CLASS_", sample.class)
    );
    flag("SW_SAMPLE_AP_DEFAULTED", sample.ap_defaulted);
    let ap = table
        .ap_array_at(j2000_seconds(2003, 10, 31, 13, 0, 0.0))
        .expect("AP array");
    let values: Vec<String> = ap.iter().map(|v| bits(*v)).collect();
    println!(
        "static const uint64_t {P}SW_AP_ARRAY_BITS[7] = {{ {} }};",
        values.join(", ")
    );
    let day = table.day(2026, 7, 1);
    flag("SW_DAY_PRESENT", day.is_some());
    let day = day.expect("2026-07-01 row");
    flag("SW_DAY_HAS_AP_AVG", day.ap_avg.is_some());
    int("SW_DAY_AP_AVG", day.ap_avg.unwrap_or(0));
    flag("SW_DAY_HAS_F107_OBS", day.f107_obs.is_some());
    f("SW_DAY_F107_OBS_BITS", day.f107_obs.unwrap_or(0.0));
    println!();
}

fn ntrip() {
    use sidereon_core::ntrip::{
        parse_sourcetable, GgaPosition, NtripClientMachine, NtripConfig, NtripCredentials,
        NtripEvent, NtripVersion,
    };
    println!("/* test_ntrip: sidereon_core::ntrip. */");
    let mut config = NtripConfig::default();
    config.host = "caster.example.test".to_string();
    config.port = 2101;
    config.mountpoint = "MOUNT".to_string();
    config.version = NtripVersion::Rev2;
    config.credentials = Some(NtripCredentials {
        username: "user".to_string(),
        password: "pass".to_string(),
    });
    config.user_agent_product = "sidereon-test/0".to_string();
    config.gga_interval_s = None;
    let request = config.request_bytes().expect("request");
    bytes_text("NTRIP_REQUEST", &request);

    config.version = NtripVersion::Rev1;
    config.credentials = None;
    config.gga_interval_s = Some(10.0);
    let mut machine = NtripClientMachine::new(config);
    let machine_request = machine.connection_request().expect("machine request");
    bytes_text("NTRIP_MACHINE_REQUEST", &machine_request);
    let events = machine.push(b"ICY 200 OK\r\n\r\nabc");
    int("NTRIP_EVENT_COUNT", events.len());
    for (i, event) in events.iter().enumerate() {
        println!(
            "#define {P}NTRIP_EVENT{i}_KIND {}",
            variant("NTRIP_EVENT_", event)
        );
        match event {
            NtripEvent::Connected(handshake) => println!(
                "#define {P}NTRIP_EVENT{i}_VERSION {}",
                variant("NTRIP_VERSION_", handshake.version)
            ),
            NtripEvent::Payload(payload) => {
                bytes_text(&format!("NTRIP_EVENT{i}_PAYLOAD"), payload);
            }
            _ => {}
        }
    }
    println!(
        "#define {P}NTRIP_STATE {}",
        variant("NTRIP_STATE_", machine.state())
    );
    let position = GgaPosition {
        lat_deg: 40.0,
        lon_deg: -105.0,
        height_m: 1600.0,
        fix_quality: 1,
        num_satellites: 10,
        hdop: 1.0,
    };
    let gga = machine
        .try_gga_message(5.0, &position, 3661.239)
        .expect("GGA message");
    flag("NTRIP_GGA_PRESENT", gga.is_some());
    bytes_text("NTRIP_GGA", &gga.unwrap_or_default());

    let table_text = "STR;MOUNT;ID;RTCM 3;1004(1);2;GPS;NET;USA;40.1;-105.2;1;0;gen;none;B;N;9600;misc;with;semis\r\n\
        CAS;caster.example.test;2101;Caster;Op;0;USA;40.0;-105.0;backup.example.test;2102;cas misc\r\n\
        NET;NET;Op;D;Y;https://net;https://str;https://reg;net misc\r\n\
        ENDSOURCETABLE\r\n";
    let table = parse_sourcetable(table_text).expect("sourcetable");
    int("NTRIP_SOURCETABLE_RECORD_COUNT", table.records.len());
    int("NTRIP_SOURCETABLE_STREAM_COUNT", table.streams().count());
    let stream = table.streams().next().expect("stream");
    text("NTRIP_STREAM_MOUNTPOINT", &stream.mountpoint);
    let lat = stream.lat_deg.value().copied();
    flag("NTRIP_STREAM_HAS_LAT_DEG", lat.is_some());
    f("NTRIP_STREAM_LAT_DEG_BITS", lat.unwrap_or(0.0));
    let bitrate = stream.bitrate.value().copied();
    flag("NTRIP_STREAM_HAS_BITRATE", bitrate.is_some());
    int("NTRIP_STREAM_BITRATE", bitrate.unwrap_or(0));
    int(
        "NTRIP_SOURCETABLE_TEXT_LEN",
        table.to_text().expect("sourcetable text").len(),
    );
    println!();
}

fn main() {
    let core = core_fixtures();
    header_start("w4_r2p", GUARD);
    covariance();
    cnav(&core);
    qc(&core);
    geoid();
    nmea();
    space_weather(&core);
    ntrip();
    header_end(GUARD);
}
