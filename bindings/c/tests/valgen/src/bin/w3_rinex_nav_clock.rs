// Print tests/w3_rinex_nav_clock_pins.h: sidereon-core's results for the
// inputs rinex_nav_clock_smoke.c reads (the ESBC mixed NAV file, the synthetic
// clock product and the inline NAV, SBAS and clock texts of the C source).

use sidereon_core::astro::time::{Instant, InstantRepr, TimeScale};
use sidereon_core::ephemeris::BroadcastEphemeris;
use sidereon_core::rinex::clock::{
    civil_to_clock_instant, ClockEpoch, ClockHeaderField, ClockHeaderRecord, ClockPoint,
    ClockRecord, ClockRecordReading, ClockRecordType, ClockTimeSystem, ClockWriteDeparture,
    ClockWriteLeniency, ClockWritePolicy, RinexClock, RinexClockError,
};
use sidereon_core::rinex::nav::{
    encode_nav, parse_glonass_lenient, parse_iono_corrections, parse_leap_seconds, parse_nav,
    parse_nav_lenient, BroadcastRecord, GlonassRecord,
};
use valgen::{bits, c_string, c_string_literals, header_end, header_start, read, tests_path};

const P: &str = "W3NC";
const SOURCE: &str = "rinex_nav_clock_smoke.c";
const NAV_FIXTURE: &str = "fixtures/nav/ESBC00DNK_R_20201770000_01D_MN.rnx";
const CLK_FIXTURE: &str = "fixtures/clk/synthetic_rinex_clock.clk";

/// The SBAS MT2 capture the test logs in both wire forms: the EMS line holds
/// the framed 250-bit form, the RTKLIB line the 226-bit body.
const SBAS_EMS_LINE: &str =
    "120,26,7,1,0,0,1,2,5308DFFC010005FFC00DFFC009FFDFFC001FFDFFDFFFBABBBBBB9BBB83A9CE00\n";
const SBAS_RTKLIB_LINE: &str =
    "2360 259200 120 2 : 5308DFFC010005FFC00DFFC009FFDFFC001FFDFFDFFFBABBBBBB9BBB80\n";

fn source() -> String {
    tests_path(SOURCE)
}

fn inline(marker: &str) -> String {
    c_string_literals(&source(), marker)
}

fn def(name: &str, value: impl std::fmt::Display) {
    println!("#define {P}_{name} {value}");
}

fn def_bits(name: &str, value: f64) {
    def(name, bits(value));
}

fn def_str(name: &str, value: &str) {
    def(name, c_string(value));
}

fn def_bool(name: &str, value: bool) {
    def(name, if value { "true" } else { "false" });
}

fn def_count(name: &str, value: usize) {
    def(name, format!("((size_t)UINT64_C({value}))"));
}

/// The variant name of a Debug rendering: the text before any field list.
fn variant(debug: &str) -> String {
    let cut = debug.find(['{', ' ', '(']).unwrap_or(debug.len());
    debug[..cut].trim().to_string()
}

/// CamelCase to SCREAMING_SNAKE: an underscore before an upper-case letter
/// that follows a lower-case letter or a digit.
fn snake(name: &str) -> String {
    let mut out = String::new();
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

/// The C constant the binding writes for an engine enum value: the binding's
/// constants are the engine variant names under the C enum's prefix.
fn c_const(prefix: &str, value: &impl std::fmt::Debug) -> String {
    format!("{prefix}_{}", snake(&variant(&format!("{value:?}"))))
}

fn time_scale(scale: TimeScale) -> String {
    c_const("SIDEREON_TIME_SCALE", &scale)
}

fn opt_bits(value: Option<f64>) -> (bool, String) {
    (value.is_some(), bits(value.unwrap_or(0.0)))
}

fn nav_record_type() {
    println!("typedef struct W3NcNavRecord {{");
    println!("    const char *sat;");
    println!("    uint32_t message;");
    println!("    bool has_issue;");
    println!("    uint32_t issue;");
    println!("    uint32_t issue_message;");
    println!("    uint32_t week;");
    println!("    uint32_t toe_system;");
    println!("    uint32_t toe_week;");
    println!("    uint64_t toe_tow_s;");
    println!("    uint32_t toc_system;");
    println!("    uint32_t toc_week;");
    println!("    uint64_t toc_tow_s;");
    println!("    /* sqrt_a, e, m0, delta_n, omega0, i0, omega, omega_dot, idot, cuc, cus,");
    println!("     * crc, crs, cic, cis, toe_sow. */");
    println!("    uint64_t elements[16];");
    println!("    /* af0, af1, af2, toc_sow. */");
    println!("    uint64_t clock[4];");
    println!("    uint64_t sv_health;");
    println!("    bool has_sv_accuracy_m;");
    println!("    uint64_t sv_accuracy_m;");
    println!("    bool has_fit_interval_s;");
    println!("    uint64_t fit_interval_s;");
    println!("    /* gps_tgd, galileo_bgd_e5a_e1, galileo_bgd_e5b_e1, beidou_tgd1,");
    println!("     * beidou_tgd2, cnav_isc_l1ca, cnav_isc_l2c, cnav_isc_l5i5,");
    println!("     * cnav_isc_l5q5, cnav_isc_l1cd, cnav_isc_l1cp. */");
    println!("    bool has_group_delay[11];");
    println!("    uint64_t group_delay[11];");
    println!("    bool cnav_present;");
    println!("}} W3NcNavRecord;");
    println!();
    println!("typedef struct W3NcGlonassRecord {{");
    println!("    const char *sat;");
    println!("    int32_t freq_channel;");
    println!("    int32_t stated_freq_channel;");
    println!("    uint64_t toe_utc_j2000_s;");
    println!("    uint64_t epoch_utc_j2000_s;");
    println!("    uint64_t pos_m[3];");
    println!("    uint64_t vel_m_s[3];");
    println!("    uint64_t acc_m_s2[3];");
    println!("    uint64_t clk_bias;");
    println!("    uint64_t gamma_n;");
    println!("    uint64_t sv_health;");
    println!("    /* message_frame_time_s, age_days, status_flags,");
    println!("     * l1_l2_group_delay_field_s, urai, health_flags. */");
    println!("    bool has_optional[6];");
    println!("    uint64_t optional[6];");
    println!("}} W3NcGlonassRecord;");
    println!();
}

fn nav_message(message: impl std::fmt::Debug) -> String {
    c_const("SIDEREON_NAV_MESSAGE", &message)
}

fn emit_nav_record(name: &str, record: &BroadcastRecord) {
    let e = &record.elements;
    let elements = [
        e.sqrt_a,
        e.e,
        e.m0,
        e.delta_n,
        e.omega0,
        e.i0,
        e.omega,
        e.omega_dot,
        e.idot,
        e.cuc,
        e.cus,
        e.crc,
        e.crs,
        e.cic,
        e.cis,
        e.toe_sow,
    ];
    let c = &record.clock;
    let clock = [c.af0, c.af1, c.af2, c.toc_sow];
    let g = &record.group_delays;
    let delays = [
        g.gps_tgd_s,
        g.galileo_bgd_e5a_e1_s,
        g.galileo_bgd_e5b_e1_s,
        g.beidou_tgd1_s,
        g.beidou_tgd2_s,
        g.cnav_isc_l1ca_s,
        g.cnav_isc_l2c_s,
        g.cnav_isc_l5i5_s,
        g.cnav_isc_l5q5_s,
        g.cnav_isc_l1cd_s,
        g.cnav_isc_l1cp_s,
    ];
    let (has_accuracy, accuracy) = opt_bits(record.sv_accuracy_m);
    let (has_fit, fit) = opt_bits(record.fit_interval_s);
    let join = |values: &[f64]| {
        values
            .iter()
            .map(|v| bits(*v))
            .collect::<Vec<_>>()
            .join(", ")
    };
    println!("static const W3NcNavRecord {P}_{name} = {{");
    println!("    {},", c_string(&record.satellite_id.to_string()));
    println!("    {},", nav_message(record.message));
    println!("    {},", record.issue_of_data.is_some());
    println!(
        "    {},",
        record.issue_of_data.map_or(0, |issue| issue.issue)
    );
    println!(
        "    {},",
        record
            .issue_of_data
            .map_or("0".to_string(), |issue| nav_message(issue.message))
    );
    println!("    {},", record.week);
    println!("    {},", time_scale(record.toe.system));
    println!("    {},", record.toe.week);
    println!("    {},", bits(record.toe.tow_s));
    println!("    {},", time_scale(record.toc.system));
    println!("    {},", record.toc.week);
    println!("    {},", bits(record.toc.tow_s));
    println!("    {{ {} }},", join(&elements));
    println!("    {{ {} }},", join(&clock));
    println!("    {},", bits(record.sv_health));
    println!("    {has_accuracy},");
    println!("    {accuracy},");
    println!("    {has_fit},");
    println!("    {fit},");
    println!(
        "    {{ {} }},",
        delays
            .iter()
            .map(|d| d.is_some().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!(
        "    {{ {} }},",
        delays
            .iter()
            .map(|d| bits(d.unwrap_or(0.0)))
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("    {},", record.cnav.is_some());
    println!("}};");
}

fn emit_glonass_record(name: &str, record: &GlonassRecord) {
    let optional = [
        record.message_frame_time_s,
        record.age_days,
        record.status_flags,
        record.l1_l2_group_delay_field_s,
        record.urai,
        record.health_flags,
    ];
    let three = |v: [f64; 3]| format!("{{ {}, {}, {} }}", bits(v[0]), bits(v[1]), bits(v[2]));
    println!("static const W3NcGlonassRecord {P}_{name} = {{");
    println!("    {},", c_string(&record.satellite_id.to_string()));
    println!("    {},", record.freq_channel);
    println!("    {},", record.stated_freq_channel);
    println!("    {},", bits(record.toe_utc_j2000_s));
    println!("    {},", bits(record.epoch_utc_j2000_s));
    println!("    {},", three(record.pos_m));
    println!("    {},", three(record.vel_m_s));
    println!("    {},", three(record.acc_m_s2));
    println!("    {},", bits(record.clk_bias));
    println!("    {},", bits(record.gamma_n));
    println!("    {},", bits(record.sv_health));
    println!(
        "    {{ {} }},",
        optional
            .iter()
            .map(|v| v.is_some().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!(
        "    {{ {} }},",
        optional
            .iter()
            .map(|v| bits(v.unwrap_or(0.0)))
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("}};");
}

fn nav_section() {
    let nav = read(&tests_path(NAV_FIXTURE));
    let records = parse_nav(&nav).expect("ESBC NAV parses strictly");
    println!("/* The ESBC mixed NAV file ({NAV_FIXTURE}). */");
    def_count("RAW_COUNT", records.len());
    emit_nav_record("RAW0", &records[0]);

    let encoded = encode_nav(&records[..1]).expect("encode the first raw record");
    let header_line = &encoded[..encoded.find('\n').expect("encoded header line") + 1];
    def_str("ENCODED_HEADER_LINE", header_line);
    let reparsed = parse_nav(&encoded).expect("encoded NAV reparses");
    def_count("REENCODED_COUNT", reparsed.len());
    assert!(
        reparsed[0] == records[0],
        "the encoded first record reads back unchanged"
    );

    let lenient = parse_nav_lenient(&nav).expect("lenient NAV parse");
    def_count("LENIENT_RECORD_COUNT", lenient.records.len());
    def_count("LENIENT_SKIPPED_COUNT", lenient.skipped.len());
    assert!(lenient.records[0] == records[0], "lenient first record");

    // The test's malformed copy: the first "C05 2020" becomes "C05 XXXX".
    let at = nav.find("C05 2020").expect("C05 2020 in the NAV file");
    let mut bad = nav.clone();
    bad.replace_range(at + 4..at + 8, "XXXX");
    let bad_lenient = parse_nav_lenient(&bad).expect("lenient malformed NAV parse");
    def_count("BAD_LENIENT_RECORD_COUNT", bad_lenient.records.len());
    def_count("BAD_LENIENT_SKIPPED_COUNT", bad_lenient.skipped.len());
    def_str("BAD_SKIPPED0_SATELLITE", &bad_lenient.skipped[0].satellite);
    def_str("BAD_SKIPPED0_MESSAGE", &bad_lenient.skipped[0].message);
    // sidereon_parse_rinex_nav_records maps a parse_nav refusal to
    // SIDEREON_STATUS_INVALID_ARGUMENT (src/rinex.rs).
    def(
        "BAD_STRICT_STATUS",
        if parse_nav(&bad).is_err() {
            "SIDEREON_STATUS_INVALID_ARGUMENT"
        } else {
            "SIDEREON_STATUS_OK"
        },
    );

    let iono = parse_iono_corrections(&nav).expect("NAV ionosphere corrections");
    def_bool("IONO_GPS_PRESENT", iono.gps.is_some());
    def_bool("IONO_GALILEO_PRESENT", iono.galileo.is_some());
    def_bool("IONO_BEIDOU_PRESENT", iono.beidou.is_some());
    let gps = iono.gps.expect("GPS Klobuchar");
    for i in 0..4 {
        def_bits(&format!("IONO_GPS_ALPHA{i}"), gps.alpha[i]);
        def_bits(&format!("IONO_GPS_BETA{i}"), gps.beta[i]);
    }
    let galileo = iono.galileo.expect("Galileo NeQuick");
    def_bits("IONO_GALILEO_AI0", galileo.ai0);
    def_bits("IONO_GALILEO_AI1", galileo.ai1);
    def_bits("IONO_GALILEO_AI2", galileo.ai2);
    let leap = parse_leap_seconds(&nav).expect("NAV leap seconds");
    def_bool("LEAP_PRESENT", leap.is_some());
    def_bits("LEAP_SECONDS", leap.unwrap_or(0.0));
    let empty = inline("const uint8_t empty_header[] =");
    let empty_leap = parse_leap_seconds(&empty).expect("empty-header leap seconds");
    def_bool("EMPTY_HEADER_LEAP_PRESENT", empty_leap.is_some());

    let glonass = inline("static const char *glonass_fixture(void) {");
    let parsed = parse_glonass_lenient(&glonass).expect("GLONASS fixture");
    def_count("GLONASS_COUNT", parsed.records.len());
    emit_glonass_record("GLONASS0", &parsed.records[0]);
    let extended = inline("static const char *glonass_extended_fixture(void) {");
    let extended = parse_glonass_lenient(&extended).expect("extended GLONASS fixture");
    def_count("EXTENDED_GLONASS_COUNT", extended.records.len());
    def_count("EXTENDED_GLONASS_SKIPPED_COUNT", extended.skipped.len());
    emit_glonass_record("EXTENDED_GLONASS0", &extended.records[0]);

    let combined = format!("{nav}{glonass}");
    let store = BroadcastEphemeris::from_nav(&combined).expect("broadcast store");
    def_count("STORE_RECORD_COUNT", store.records().len());
    def_count("STORE_GLONASS_COUNT", store.glonass_records().len());
    emit_nav_record("STORE0", &store.records()[0]);
    emit_glonass_record("STORE_GLONASS0", &store.glonass_records()[0]);
    let channels = store.glonass_frequency_channels();
    def_count("STORE_CHANNEL_COUNT", channels.len());
    let (slot, channel) = channels.iter().next().expect("one frequency channel");
    def("STORE_CHANNEL0_SLOT", slot);
    def("STORE_CHANNEL0_CHANNEL", channel);
    let store_iono = store.iono_corrections();
    def_bool("STORE_IONO_GPS_PRESENT", store_iono.gps.is_some());
    def_bool("STORE_IONO_GALILEO_PRESENT", store_iono.galileo.is_some());
    let store_leap = parse_leap_seconds(&combined).expect("store leap seconds");
    def_bool("STORE_LEAP_PRESENT", store_leap.is_some());
    def_bits("STORE_LEAP_SECONDS", store_leap.unwrap_or(0.0));
    println!();
}

fn sbas_section() {
    let ems = sidereon_core::sbas::parse_ems_lines(SBAS_EMS_LINE).expect("EMS line");
    let rtklib = sidereon_core::sbas::parse_rtklib_lines(SBAS_RTKLIB_LINE).expect("RTKLIB line");
    println!("/* The SBAS MT2 capture, logged in the EMS and RTKLIB text forms. */");
    def_str("SBAS_EMS_LINE", SBAS_EMS_LINE);
    def_str("SBAS_RTKLIB_LINE", SBAS_RTKLIB_LINE);
    def_count("SBAS_EMS_COUNT", ems.len());
    def_count("SBAS_RTKLIB_COUNT", rtklib.len());
    for (name, block) in [("SBAS_EMS0", &ems[0]), ("SBAS_RTKLIB0", &rtklib[0])] {
        def_str(&format!("{name}_SAT"), &block.satellite_id.to_string());
        def(
            &format!("{name}_EPOCH_SYSTEM"),
            time_scale(block.epoch.system),
        );
        def(&format!("{name}_EPOCH_WEEK"), block.epoch.week);
        def_bits(&format!("{name}_EPOCH_TOW_S"), block.epoch.tow_s);
        def(
            &format!("{name}_FORM"),
            c_const("SIDEREON_SBAS_WIRE_FORM", &block.form),
        );
        def_count(&format!("{name}_BYTE_COUNT"), block.bytes.len());
        def_bool(
            &format!("{name}_HAS_DECLARED_MESSAGE_TYPE"),
            block.declared_message_type.is_some(),
        );
        def(
            &format!("{name}_DECLARED_MESSAGE_TYPE"),
            block.declared_message_type.unwrap_or(0),
        );
        def_bool(
            &format!("{name}_HAS_MESSAGE_TYPE"),
            block.message_type().is_some(),
        );
        def(
            &format!("{name}_MESSAGE_TYPE"),
            block.message_type().unwrap_or(0),
        );
    }
    println!(
        "static const uint8_t {P}_SBAS_EMS0_BYTES[] = {{ {} }};",
        ems[0]
            .bytes
            .iter()
            .map(|b| format!("0x{b:02X}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!();
}

/// The C reading the binding writes (clock_record_reading_to_c).
fn record_reading(reading: ClockRecordReading) -> String {
    use sidereon_core::rinex::clock::ClockLayout;
    let name = match reading {
        ClockRecordReading::Columns(ClockLayout::V300) => "COLUMNS_V300",
        ClockRecordReading::Columns(ClockLayout::V304) => "COLUMNS_V304",
        ClockRecordReading::Whitespace => "WHITESPACE",
        ClockRecordReading::Edited => "EDITED",
        _ => "UNKNOWN",
    };
    format!("SIDEREON_CLOCK_RECORD_READING_{name}")
}

fn emit_clock_epoch(name: &str, epoch: &Instant) {
    def(&format!("{name}_SCALE"), time_scale(epoch.scale));
    match epoch.repr {
        InstantRepr::JulianDate(jd) => {
            def(&format!("{name}_REPRESENTATION"), 0);
            def_bits(&format!("{name}_JD_WHOLE"), jd.jd_whole);
            def_bits(&format!("{name}_JD_FRACTION"), jd.fraction);
        }
        InstantRepr::Nanos(_) => panic!("{name}: the test reads a Julian-date epoch"),
    }
}

fn emit_clock_record(name: &str, record: &ClockRecord) {
    def(
        &format!("{name}_TYPE"),
        c_const("SIDEREON_CLOCK_RECORD_TYPE", &record.record_type()),
    );
    def_bool(
        &format!("{name}_HAS_SATELLITE"),
        record.satellite().is_some(),
    );
    def_str(
        &format!("{name}_SATELLITE"),
        record.satellite().unwrap_or(""),
    );
    def_str(&format!("{name}_NAME"), record.name());
    let civil = record.civil_epoch();
    def(&format!("{name}_YEAR"), civil.year);
    def(&format!("{name}_MONTH"), civil.month);
    def(&format!("{name}_DAY"), civil.day);
    def(&format!("{name}_HOUR"), civil.hour);
    def(&format!("{name}_MINUTE"), civil.minute);
    def_bits(&format!("{name}_SECOND"), civil.second);
    def_bool(&format!("{name}_HAS_EPOCH"), record.epoch().is_some());
    if let Some(epoch) = record.epoch() {
        def(&format!("{name}_EPOCH_SCALE"), time_scale(epoch.scale));
    }
    def_count(&format!("{name}_VALUE_COUNT"), record.values().len());
    for (i, value) in record.values().iter().enumerate() {
        def_bits(&format!("{name}_VALUE{i}"), *value);
    }
    def_count(
        &format!("{name}_SURPLUS_COUNT"),
        record.surplus_values().len(),
    );
    for (i, surplus) in record.surplus_values().iter().enumerate() {
        def_count(&format!("{name}_SURPLUS{i}_POSITION"), surplus.position);
        def_bits(&format!("{name}_SURPLUS{i}_VALUE"), surplus.value);
    }
    def_bool(&format!("{name}_HAS_LINE"), record.line().is_some());
    def_count(&format!("{name}_LINE"), record.line().unwrap_or(0));
    def_count(&format!("{name}_LINE_COUNT"), record.line_count());
    def(&format!("{name}_READING"), record_reading(record.reading()));
    def_bool(
        &format!("{name}_HAS_CONTINUATION_READING"),
        record.continuation_reading().is_some(),
    );
}

fn emit_error(name: &str, err: &RinexClockError) {
    def(
        &format!("{name}_KIND"),
        c_const("SIDEREON_RINEX_CLOCK_ERROR_KIND", err),
    );
    match err {
        RinexClockError::InvalidInput { field, .. } | RinexClockError::BadField { field, .. } => {
            def_str(&format!("{name}_FIELD"), field);
        }
        RinexClockError::UnsupportedTimeScale { scale } => {
            def(&format!("{name}_TIME_SCALE"), time_scale(*scale));
        }
        _ => {}
    }
}

fn header_parts(record: &ClockHeaderRecord) -> Vec<String> {
    match record.field() {
        Some(ClockHeaderField::TypesOfData { types, .. }) => types.clone(),
        _ => Vec::new(),
    }
}

fn clock_a18_section() {
    let text = inline("static const char clock_304_a18[] =");
    let mut clock = RinexClock::parse(&text).expect("A18 parses");
    assert_eq!(
        clock.to_rinex_string().expect("A18 writes"),
        text,
        "A18 restated byte for byte"
    );
    println!("/* RINEX clock 3.04 Table A18 example (clock_304_a18 in the C source). */");
    def_bool("A18_HAS_VERSION", clock.version().is_some());
    def_bits("A18_VERSION", clock.version().unwrap_or(0.0));
    def_bool("A18_HAS_LAYOUT", clock.layout().is_some());
    def(
        "A18_LAYOUT",
        c_const("SIDEREON_CLOCK_LAYOUT", &clock.layout().expect("layout")),
    );
    def_bool("A18_HAS_TIME_SYSTEM", clock.time_system().is_some());
    def(
        "A18_TIME_SYSTEM",
        c_const(
            "SIDEREON_CLOCK_TIME_SYSTEM",
            &clock.time_system().expect("time system"),
        ),
    );
    def(
        "A18_TIME_SYSTEM_STATUS",
        c_const(
            "SIDEREON_CLOCK_TIME_SYSTEM_STATUS",
            clock.time_system_status(),
        ),
    );
    def_bool("A18_HAS_TIME_SCALE", clock.time_scale().is_some());
    def(
        "A18_TIME_SCALE",
        time_scale(clock.time_scale().expect("time scale")),
    );
    def_count("A18_HEADER_RECORD_COUNT", clock.header_records().len());
    def_count("A18_RECORD_COUNT", clock.record_count());
    def_count("A18_SERIES_COUNT", clock.series().len());
    def_count(
        "A18_SAMPLE_COUNT",
        clock.series().values().map(Vec::len).sum::<usize>(),
    );
    def_count("A18_SKIPPED_COUNT", clock.skipped_records().len());
    def_count("A18_DIAGNOSTIC_COUNT", clock.diagnostics().len());
    def_count("A18_NOTICE_COUNT", clock.notices().len());
    for (i, notice) in clock.notices().iter().enumerate() {
        def(
            &format!("A18_NOTICE{i}_KIND"),
            c_const("SIDEREON_CLOCK_NOTICE_KIND", notice),
        );
        use sidereon_core::rinex::clock::RinexClockNotice as N;
        match notice {
            N::HeaderRecordNonconforming { line }
            | N::HeaderRecordUninterpreted { line }
            | N::HeaderRecordUnknownLabel { line } => {
                def(&format!("A18_NOTICE{i}_HAS_LINE"), "true");
                def_count(&format!("A18_NOTICE{i}_LINE"), *line);
            }
            N::TimeSystemDefaulted { system } | N::TimeSystemWithoutScale { system } => {
                def(&format!("A18_NOTICE{i}_HAS_TIME_SYSTEM"), "true");
                def(
                    &format!("A18_NOTICE{i}_TIME_SYSTEM"),
                    c_const("SIDEREON_CLOCK_TIME_SYSTEM", system),
                );
            }
            _ => {}
        }
    }
    for (i, skip) in clock.skipped_records().iter().enumerate() {
        def_count(&format!("A18_SKIP{i}_LINE"), skip.line);
        let record_type = ClockRecordType::from_code(&skip.record_type).expect("record type");
        def(
            &format!("A18_SKIP{i}_TYPE"),
            c_const("SIDEREON_CLOCK_RECORD_TYPE", &record_type),
        );
    }
    let records: Vec<ClockRecord> = clock.records().collect();
    def_count("A18_RECORDS_LEN", records.len());
    emit_clock_record("A18_RECORD2", &records[2]);
    let header = clock.header_records();
    let leap = &header[4];
    def(
        "A18_HEADER4_FIELD_KIND",
        c_const(
            "SIDEREON_CLOCK_HEADER_FIELD_KIND",
            leap.field().expect("header 4 field"),
        ),
    );
    match leap.field() {
        Some(ClockHeaderField::LeapSecondsGnss(value))
        | Some(ClockHeaderField::LeapSeconds(value)) => {
            def("A18_HEADER4_HAS_INTEGER", "true");
            def("A18_HEADER4_INTEGER", value);
        }
        other => panic!("header record 4 reads {other:?}"),
    }
    def_bool("A18_HEADER4_HAS_LINE", leap.line().is_some());
    def_count("A18_HEADER4_LINE", leap.line().unwrap_or(0));
    def_count("A18_HEADER4_LABEL_COLUMN", leap.label_column());
    def_str("A18_HEADER4_LABEL", leap.label());
    let types = &header[5];
    def(
        "A18_HEADER5_FIELD_KIND",
        c_const(
            "SIDEREON_CLOCK_HEADER_FIELD_KIND",
            types.field().expect("header 5 field"),
        ),
    );
    match types.field() {
        Some(ClockHeaderField::TypesOfData { count, .. }) => {
            def("A18_HEADER5_HAS_COUNT", "true");
            def_count("A18_HEADER5_COUNT", *count);
        }
        other => panic!("header record 5 reads {other:?}"),
    }
    let parts = header_parts(types);
    def_count("A18_HEADER5_TEXT_PART_COUNT", parts.len());
    def_str("A18_HEADER5_PART1", &parts[1]);

    clock
        .set_time_system(ClockTimeSystem::Gps)
        .expect("declare GPS time");
    def_str(
        "A18_DECLARED_TEXT",
        &clock.to_rinex_string().expect("declared A18 writes"),
    );
    def(
        "A18_DECLARED_TIME_SYSTEM_STATUS",
        c_const(
            "SIDEREON_CLOCK_TIME_SYSTEM_STATUS",
            clock.time_system_status(),
        ),
    );
    def_count(
        "A18_DECLARED_HEADER_RECORD_COUNT",
        clock.header_records().len(),
    );
    println!();
}

/// The inputs check_clock_surplus_and_edits passes, as the C source states
/// them.
const SURPLUS_BIAS_ONLY: [f64; 1] = [0.2e-3];
const SURPLUS_BIAS_AND_SIGMA: [f64; 2] = [0.170710878415e-3, 5.556437046250e-12];
const SURPLUS_INSERT_EPOCH: ClockEpoch = ClockEpoch {
    year: 2026,
    month: 9,
    day: 17,
    hour: 0,
    minute: 0,
    second: 30.0,
};
const SURPLUS_INSERT_VALUES: [f64; 1] = [1.0e-8];

fn clock_surplus_section() {
    let text = inline("static const char clock_200_surplus[] =");
    let mut clock = RinexClock::parse(&text).expect("surplus product parses");
    println!("/* The EMR0OPSRAP lines (clock_200_surplus in the C source), and the edits");
    println!(" * check_clock_surplus_and_edits makes. */");
    println!(
        "static const double {P}_SURPLUS_BIAS_ONLY[1] = {{ {:e} }};",
        SURPLUS_BIAS_ONLY[0]
    );
    println!(
        "static const double {P}_SURPLUS_BIAS_AND_SIGMA[2] = {{ {:e}, {:e} }};",
        SURPLUS_BIAS_AND_SIGMA[0], SURPLUS_BIAS_AND_SIGMA[1]
    );
    println!(
        "static const double {P}_SURPLUS_INSERT_VALUES[1] = {{ {:e} }};",
        SURPLUS_INSERT_VALUES[0]
    );
    def(
        "SURPLUS_INSERT_EPOCH_INIT",
        format!(
            "{{ {}, {}, {}, {}, {}, {:?} }}",
            SURPLUS_INSERT_EPOCH.year,
            SURPLUS_INSERT_EPOCH.month,
            SURPLUS_INSERT_EPOCH.day,
            SURPLUS_INSERT_EPOCH.hour,
            SURPLUS_INSERT_EPOCH.minute,
            SURPLUS_INSERT_EPOCH.second
        ),
    );
    def_count("SURPLUS_NOTICE_COUNT", clock.notices().len());
    let notice = &clock.notices()[0];
    def(
        "SURPLUS_NOTICE0_KIND",
        c_const("SIDEREON_CLOCK_NOTICE_KIND", notice),
    );
    match notice {
        sidereon_core::rinex::clock::RinexClockNotice::SurplusValues {
            records,
            first_line,
        } => {
            def("SURPLUS_NOTICE0_HAS_RECORDS", "true");
            def_count("SURPLUS_NOTICE0_RECORDS", *records);
            def_count("SURPLUS_NOTICE0_FIRST_LINE", *first_line);
        }
        other => panic!("surplus notice reads {other:?}"),
    }
    let records: Vec<ClockRecord> = clock.records().collect();
    emit_clock_record("SURPLUS_RECORD1", &records[1]);
    let g01 = &clock.series()["G01"];
    def_count(
        "SURPLUS_G01_ADDITIONAL_COUNT",
        g01[0].additional_values.len(),
    );

    let refused = clock
        .set_record_values(1, SURPLUS_BIAS_ONLY.to_vec())
        .expect_err("the surplus-dropping edit is refused");
    emit_error("SURPLUS_DROP_ERROR", &refused);
    assert_eq!(
        clock.to_rinex_string().expect("unchanged product writes"),
        text,
        "a refused edit leaves the product unchanged"
    );
    clock
        .set_record_values(1, SURPLUS_BIAS_AND_SIGMA.to_vec())
        .expect("restating the sigma as a declared value");
    let inserted = ClockRecord::new(
        ClockRecordType::Ar,
        "WTZR",
        SURPLUS_INSERT_EPOCH,
        SURPLUS_INSERT_VALUES.to_vec(),
    )
    .expect("receiver record");
    clock
        .insert_record(2, inserted)
        .expect("insert receiver record");
    let removed = clock.remove_record(0).expect("remove the first record");
    emit_clock_record("SURPLUS_REMOVED", &removed);
    let written = clock.to_rinex_string().expect("edited product writes");
    let reread = RinexClock::parse(&written).expect("edited product reads back");
    def_count("SURPLUS_REREAD_RECORD_COUNT", reread.record_count());
    let rows: Vec<ClockRecord> = reread.records().collect();
    emit_clock_record("SURPLUS_REREAD0", &rows[0]);
    emit_clock_record("SURPLUS_REREAD1", &rows[1]);
    println!();
}

fn epoch_at(scale: TimeScale, y: i32, mo: u8, d: u8, h: u8, mi: u8, s: f64) -> Instant {
    civil_to_clock_instant(scale, y, mo, d, h, mi, s).expect("civil clock epoch")
}

fn point(epoch: Instant, bias_s: f64) -> ClockPoint {
    ClockPoint::new(epoch, bias_s, Vec::new())
}

fn clock_built_section() {
    println!("/* The products check_clock_built_products builds from points. */");
    let utc = RinexClock::from_clock_points(
        TimeScale::Utc,
        vec![(
            "G01".to_string(),
            vec![
                point(epoch_at(TimeScale::Utc, 2016, 12, 31, 23, 59, 59.0), 1.0e-6),
                point(epoch_at(TimeScale::Utc, 2017, 1, 1, 0, 0, 0.0), 3.0e-6),
            ],
        )],
    )
    .expect("UTC product");
    let leap = ClockEpoch {
        year: 2016,
        month: 12,
        day: 31,
        hour: 23,
        minute: 59,
        second: 60.0,
    };
    let bias = utc.clock_s("G01", leap).expect("leap-second query");
    def_bool("UTC_LEAP_AVAILABLE", bias.is_some());
    def_bits("UTC_LEAP_BIAS", bias.unwrap_or(0.0));
    def_bool(
        "GPS_LEAP_LABEL_AVAILABLE",
        civil_to_clock_instant(TimeScale::Gpst, 2016, 12, 31, 23, 59, 60.0).is_some(),
    );
    def(
        "UTC_TIME_SYSTEM",
        c_const(
            "SIDEREON_CLOCK_TIME_SYSTEM",
            &utc.time_system().expect("UTC product time system"),
        ),
    );
    def(
        "UTC_TIME_SYSTEM_STATUS",
        c_const(
            "SIDEREON_CLOCK_TIME_SYSTEM_STATUS",
            utc.time_system_status(),
        ),
    );
    def_count("UTC_HEADER_RECORD_COUNT", utc.header_records().len());
    def_count("UTC_RECORD_COUNT", utc.record_count());

    let mut off_grid = epoch_at(TimeScale::Gpst, 2020, 1, 1, 0, 0, 30.0);
    match &mut off_grid.repr {
        InstantRepr::JulianDate(jd) => jd.fraction += 1.0e-12,
        InstantRepr::Nanos(_) => panic!("the test offsets a Julian-date fraction"),
    }
    let gps = RinexClock::from_clock_points(
        TimeScale::Gpst,
        vec![(
            "G01".to_string(),
            vec![
                point(epoch_at(TimeScale::Gpst, 2020, 1, 1, 0, 0, 0.0), 1.0e-6),
                point(off_grid, 2.0e-6),
            ],
        )],
    )
    .expect("GPS product");
    let strict = gps
        .to_rinex_string_with_policy(ClockWritePolicy::strict())
        .expect_err("the strict writer refuses an off-grid epoch");
    emit_error("OFF_GRID_STRICT_ERROR", &strict);
    let (_, departures) = gps
        .to_rinex_string_with_policy(
            ClockWritePolicy::strict().with_nearest_microsecond_epochs(ClockWriteLeniency::Allow),
        )
        .expect("the lenient writer writes the off-grid epoch");
    def_count("OFF_GRID_DEPARTURE_COUNT", departures.len());
    match &departures[0] {
        ClockWriteDeparture::EpochAtNearestMicrosecond {
            record,
            epoch,
            name,
            written,
            ..
        } => {
            def(
                "OFF_GRID_DEPARTURE0_KIND",
                c_const("SIDEREON_CLOCK_WRITE_DEPARTURE_KIND", &departures[0]),
            );
            def_count("OFF_GRID_DEPARTURE0_RECORD", *record);
            def_bool("OFF_GRID_DEPARTURE0_HAS_EPOCH", epoch.is_some());
            def_str("OFF_GRID_DEPARTURE0_NAME", name);
            def_str("OFF_GRID_DEPARTURE0_WRITTEN", written);
        }
        other => panic!("departure reads {other:?}"),
    }

    let mut glonass_epoch = epoch_at(TimeScale::Gpst, 2020, 1, 1, 0, 0, 0.0);
    glonass_epoch.scale = TimeScale::Glonasst;
    let glonass = RinexClock::from_clock_points(
        TimeScale::Glonasst,
        vec![("R01".to_string(), vec![point(glonass_epoch, 1.0e-6)])],
    )
    .expect("GLONASS system time product");
    let refused = glonass
        .to_rinex_string_with_policy(ClockWritePolicy::strict())
        .expect_err("GLONASS system time has no RINEX clock time system");
    emit_error("GLONASS_WRITE_ERROR", &refused);
    println!();
}

fn clock_lossy_section() {
    let text = read(&tests_path(CLK_FIXTURE));
    let clock = RinexClock::parse_lossy(&text);
    println!("/* The synthetic clock product ({CLK_FIXTURE}), read lossily. */");
    let series = clock.series();
    def_count("CLK_SERIES_COUNT", series.len());
    def_count(
        "CLK_SAMPLE_COUNT",
        series.values().map(Vec::len).sum::<usize>(),
    );
    for (i, satellite) in series.keys().enumerate() {
        def_str(&format!("CLK_SATELLITE{i}"), satellite);
    }
    let g05 = &series["G05"];
    def_count("CLK_G05_COUNT", g05.len());
    for (i, sample) in g05.iter().enumerate() {
        emit_clock_epoch(&format!("CLK_G05_{i}_EPOCH"), &sample.epoch);
        def_bits(&format!("CLK_G05_{i}_BIAS"), sample.bias_s);
        def_count(
            &format!("CLK_G05_{i}_ADDITIONAL_COUNT"),
            sample.additional_values.len(),
        );
        for (j, value) in sample.additional_values.iter().enumerate() {
            def_bits(&format!("CLK_G05_{i}_ADDITIONAL{j}"), *value);
        }
    }
    def_bool("CLK_HAS_G99", series.contains_key("G99"));

    let malformed = inline("static const char malformed_as_clock[] =");
    let lossy = RinexClock::parse_lossy(&malformed);
    def_count(
        "MALFORMED_AS_LOSSY_SAMPLE_COUNT",
        lossy.series().values().map(Vec::len).sum::<usize>(),
    );
    // sidereon_rinex_clock_parse maps a RinexClock::parse refusal to
    // SIDEREON_STATUS_INVALID_ARGUMENT (src/rinex_clock.rs).
    def(
        "MALFORMED_AS_STRICT_STATUS",
        if RinexClock::parse(&malformed).is_err() {
            "SIDEREON_STATUS_INVALID_ARGUMENT"
        } else {
            "SIDEREON_STATUS_OK"
        },
    );
    println!();
}

fn main() {
    let guard = "SIDEREON_W3_RINEX_NAV_CLOCK_PINS_H";
    header_start("w3_rinex_nav_clock", guard);
    nav_record_type();
    nav_section();
    sbas_section();
    clock_a18_section();
    clock_surplus_section();
    clock_built_section();
    clock_lossy_section();
    header_end(guard);
}
