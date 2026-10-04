//! tests/w5_ssr_message_pins.h: sidereon-core's decode of the SSR frame and
//! bodies ssr_message_smoke.c holds as hex literals.

#[path = "w5_support/pins.rs"]
mod pins;

use pins::Pins;
use sidereon_core::rtcm::{self as core_rtcm, Message, SsrMessage};
use valgen::{c_string_literals, header_end, header_start, tests_path};

const GUARD: &str = "SIDEREON_W5_SSR_MESSAGE_PINS_H";

fn hex(marker: &str) -> Vec<u8> {
    let text = c_string_literals(&tests_path("ssr_message_smoke.c"), marker);
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).expect("hex byte"))
        .collect()
}

fn info(p: &Pins, name: &str, m: &SsrMessage) {
    p.int(
        &format!("{name}_MESSAGE_NUMBER"),
        i128::from(m.message_number),
    );
    p.variant(
        &format!("{name}_SYSTEM"),
        "SIDEREON_GNSS_SYSTEM_",
        &m.system,
    );
    p.variant(&format!("{name}_KIND"), "SIDEREON_RTCM_SSR_KIND_", &m.kind);
    let h = &m.header;
    p.int(&format!("{name}_EPOCH_TIME_S"), i128::from(h.epoch_time_s));
    p.int(
        &format!("{name}_UPDATE_INTERVAL"),
        i128::from(h.update_interval),
    );
    p.bool(&format!("{name}_MULTIPLE_MESSAGE"), h.multiple_message);
    p.int(&format!("{name}_IOD_SSR"), i128::from(h.iod_ssr));
    p.int(&format!("{name}_PROVIDER_ID"), i128::from(h.provider_id));
    p.int(&format!("{name}_SOLUTION_ID"), i128::from(h.solution_id));
    p.int(
        &format!("{name}_SATELLITE_COUNT"),
        i128::from(h.satellite_count),
    );
    let opt = |value: Option<bool>, field: &str| {
        p.bool(&format!("{name}_HAS_{field}"), value.is_some());
        p.bool(&format!("{name}_{field}"), value.unwrap_or(false));
    };
    opt(h.satellite_reference_datum, "SATELLITE_REFERENCE_DATUM");
    opt(h.dispersive_bias_consistency, "DISPERSIVE_BIAS_CONSISTENCY");
    opt(h.mw_consistency, "MW_CONSISTENCY");
    p.int(&format!("{name}_ORBIT_COUNT"), m.orbit.len() as i128);
    p.int(&format!("{name}_CLOCK_COUNT"), m.clock.len() as i128);
    p.int(&format!("{name}_URA_COUNT"), m.ura.len() as i128);
    p.int(
        &format!("{name}_CODE_BIAS_COUNT"),
        m.code_bias.len() as i128,
    );
    p.int(
        &format!("{name}_PHASE_BIAS_COUNT"),
        m.phase_bias.len() as i128,
    );
}

fn main() {
    header_start("w5_ssr_message", GUARD);
    let p = Pins::new("W5_SSR_MESSAGE");

    let frame = hex("static const char *const combined_frame_hex =");
    let messages = core_rtcm::decode_messages(&frame).expect("combined frame");
    p.comment("sidereon_core::rtcm::decode_messages on combined_frame_hex, and\nSsrMessage::decode on code_body_hex, phase_body_hex and ura_body_hex.");
    p.int("COMBINED_MESSAGE_COUNT", messages.len() as i128);
    let Message::Ssr(combined) = &messages[0] else {
        panic!("combined frame holds an SSR message")
    };
    info(&p, "COMBINED", combined);
    let body = core_rtcm::decode_frame(&frame).expect("combined frame body");
    p.int("COMBINED_BODY_LEN", body.body.len() as i128);
    p.int("COMBINED_FRAME_LEN", body.frame_len as i128);

    let code = SsrMessage::decode(&hex("static const char *const code_body_hex =")).expect("code");
    info(&p, "CODE", &code);
    let record = &code.code_bias[0];
    p.int("CODE_RECORD_SATELLITE_ID", i128::from(record.satellite_id));
    p.int("CODE_RECORD_SIGNAL_COUNT", record.biases.len() as i128);
    let ids: Vec<i128> = record.biases.iter().map(|b| i128::from(b.0)).collect();
    let biases: Vec<i128> = record.biases.iter().map(|b| i128::from(b.1)).collect();
    p.ints("CODE_SIGNAL_IDS", "uint8_t", &ids);
    p.ints("CODE_SIGNAL_BIASES", "int16_t", &biases);

    let phase =
        SsrMessage::decode(&hex("static const char *const phase_body_hex =")).expect("phase");
    info(&p, "PHASE", &phase);
    let record = &phase.phase_bias[0];
    p.int("PHASE_RECORD_SATELLITE_ID", i128::from(record.satellite_id));
    p.int("PHASE_RECORD_YAW_ANGLE", i128::from(record.yaw_angle));
    p.int("PHASE_RECORD_YAW_RATE", i128::from(record.yaw_rate));
    p.int("PHASE_RECORD_SIGNAL_COUNT", record.biases.len() as i128);
    let field = |f: &dyn Fn(&core_rtcm::SsrPhaseBiasSignal) -> i128| -> Vec<i128> {
        record.biases.iter().map(f).collect()
    };
    p.ints(
        "PHASE_SIGNAL_IDS",
        "uint8_t",
        &field(&|s| i128::from(s.signal_id)),
    );
    p.ints(
        "PHASE_SIGNAL_INTEGER_INDICATORS",
        "uint8_t",
        &field(&|s| i128::from(s.integer_indicator)),
    );
    p.ints(
        "PHASE_SIGNAL_WIDE_LANE_INTEGER_INDICATORS",
        "uint8_t",
        &field(&|s| i128::from(s.wide_lane_integer_indicator)),
    );
    p.ints(
        "PHASE_SIGNAL_DISCONTINUITY_COUNTERS",
        "uint8_t",
        &field(&|s| i128::from(s.discontinuity_counter)),
    );
    p.ints(
        "PHASE_SIGNAL_BIASES",
        "int32_t",
        &field(&|s| i128::from(s.bias)),
    );

    let ura = SsrMessage::decode(&hex("static const char *const ura_body_hex =")).expect("ura");
    info(&p, "URA", &ura);
    p.int("URA_RECORD_SATELLITE_ID", i128::from(ura.ura[0].0));
    p.int("URA_RECORD_INDEX", i128::from(ura.ura[0].1));

    p.comment("SsrMessage::decode on the one-byte body {0}.");
    p.outcome("INVALID_BODY", &SsrMessage::decode(&[0u8]));

    header_end(GUARD);
}
