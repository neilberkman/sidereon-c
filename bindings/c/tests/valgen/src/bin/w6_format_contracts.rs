//! format_contracts_smoke.c: sidereon-core's reading of the Wettzell ANTEX
//! trim and of the inline ANTEX and BLQ texts that test holds, its refusals,
//! the SBAS PRN window and the RTCM MSM encode refusal, as the C routes
//! report them (bindings/c/src/antex.rs, antenna.rs, blq.rs, sbas.rs,
//! rtcm.rs).

#[path = "../w6_shared.rs"]
mod shared;

use shared::*;
use sidereon_core::antex::{Antex, AntexDateTime, AntexError, PcvType, SecondFraction};
use sidereon_core::rtcm::{Message, MsmHeader, MsmKind, MsmMessage, MsmSatellite};
use sidereon_core::sbas::{sat_to_sbas_prn, SbasCorrectionStore};
use sidereon_core::tides::{
    parse_ocean_loading_blq_blocks, write_ocean_loading_blq_blocks, OceanLoadingBlqBlock,
    OceanLoadingBlqCommentPlacement, TideError,
};
use sidereon_core::{GnssSatelliteId, GnssSystem};
use valgen::{read, tests_path};

const SOURCE: &str = "format_contracts_smoke.c";

fn antex_error_kind(err: &AntexError) -> String {
    c_enum("SIDEREON_ANTEX_ERROR_KIND", err)
}

fn date(name: &str, value: &AntexDateTime) {
    def(&format!("{name}_YEAR"), value.year);
    def(&format!("{name}_MONTH"), value.month);
    def(&format!("{name}_DAY"), value.day);
    def(&format!("{name}_FRACTION_DIGITS"), value.fraction.digits());
    def(&format!("{name}_FRACTION_SCALE"), value.fraction.scale());
}

fn placement(name: &str, value: OceanLoadingBlqCommentPlacement) {
    let (label, row) = match value {
        OceanLoadingBlqCommentPlacement::BeforeStation => ("BEFORE_STATION", 0),
        OceanLoadingBlqCommentPlacement::BeforeRow(row) => ("BEFORE_ROW", row),
        // sidereon_blq_blocks_comment writes row 0 for a placement after the
        // rows.
        OceanLoadingBlqCommentPlacement::AfterRows => ("AFTER_ROWS", 0),
    };
    def(
        &format!("{name}_PLACEMENT"),
        format!("SIDEREON_BLQ_COMMENT_PLACEMENT_{label}"),
    );
    def_size(&format!("{name}_ROW"), row);
}

fn rtcm_status(err: &sidereon_core::Error) -> &'static str {
    use sidereon_core::Error as E;
    match err {
        E::InvalidInput(_) | E::RtcmEncode(_) | E::RtcmConversion(_) | E::SbasEncode(_) => {
            "SIDEREON_STATUS_INVALID_ARGUMENT"
        }
        E::Parse(_) => "SIDEREON_STATUS_SP3_PARSE",
        E::Ut1OutsideCoverage(_) => "SIDEREON_STATUS_UT1_OUTSIDE_COVERAGE",
        _ => "SIDEREON_STATUS_SOLVE",
    }
}

fn main() {
    let guard = "SIDEREON_W6_FORMAT_CONTRACTS_PINS_H";
    valgen::header_start("w6_format_contracts", guard);
    let source = tests_path(SOURCE);

    // The Wettzell ANTEX trim run_smoke.sh passes.
    let antex =
        Antex::parse(&read(&tests_path("fixtures/antex/igs20_wettzell_trim.atx"))).expect("ANTEX");
    let header = &antex.header;
    let version = header.version.as_ref();
    comment("ANTEX header.");
    def_bool("W6_FC_HAS_VERSION", version.is_some());
    def_bits(
        "W6_FC_VERSION_BITS",
        version.map_or(f64::NAN, |v| v.version),
    );
    let system = version.and_then(|v| v.system);
    def_bool("W6_FC_HAS_SYSTEM", system.is_some());
    def("W6_FC_SYSTEM", system.map_or(0, u32::from));
    let pcv = header.pcv_type.as_ref();
    def_bool("W6_FC_HAS_PCV_TYPE", pcv.is_some());
    def(
        "W6_FC_PCV_TYPE",
        match pcv.map(|p| p.pcv_type) {
            Some(PcvType::Absolute) | None => "SIDEREON_ANTEX_PCV_TYPE_ABSOLUTE",
            Some(PcvType::Relative) => "SIDEREON_ANTEX_PCV_TYPE_RELATIVE",
        },
    );
    def_bool(
        "W6_FC_HAS_REFERENCE_ANTENNA",
        pcv.is_some_and(|p| p.reference_antenna().is_some()),
    );
    def_size("W6_FC_HEADER_COMMENTS", header.comments.len());
    def_bool("W6_FC_END_OF_HEADER", header.end_of_header);
    let blocks: Vec<_> = antex.antenna_blocks().collect();
    def_size("W6_FC_BLOCK_COUNT", blocks.len());

    comment("Block 9, the block the test reads as the receiver antenna.");
    let receiver = blocks[9];
    def(
        "W6_FC_RECEIVER_KIND",
        c_enum("SIDEREON_ANTENNA_KIND", &receiver.kind),
    );
    def_bool("W6_FC_RECEIVER_HAS_DAZI", receiver.dazi_deg.is_some());
    def_bits(
        "W6_FC_RECEIVER_DAZI_BITS",
        receiver.dazi_deg.unwrap_or(f64::NAN),
    );
    def_bool("W6_FC_RECEIVER_HAS_GRID", receiver.zenith_grid.is_some());
    def_bits(
        "W6_FC_RECEIVER_ZENITH_END_BITS",
        receiver.zenith_grid.map_or(f64::NAN, |g| g.end_deg),
    );
    def_size("W6_FC_RECEIVER_FREQUENCIES", receiver.frequencies.len());
    def_size("W6_FC_RECEIVER_CALIBRATIONS", receiver.calibrations.len());
    def_size("W6_FC_RECEIVER_COMMENTS", receiver.comments.len());
    def_bool(
        "W6_FC_RECEIVER_HAS_VALID_FROM",
        receiver.valid_from.is_some(),
    );
    def_str(
        "W6_FC_RECEIVER_FREQUENCY5",
        &receiver.frequencies[5].frequency,
    );
    def_str("W6_FC_RECEIVER_METHOD", &receiver.calibrations[0].method);

    comment("G05 at 2020-01-01T00:00:00 GPST.");
    let epoch = AntexDateTime::new_with_fraction(
        2020,
        1,
        1,
        0,
        0,
        0,
        SecondFraction::new(0, 0).expect("zero fraction"),
    )
    .expect("epoch");
    let satellite = antex.satellite_antenna("G05", epoch);
    def_bool("W6_FC_G05_FOUND", satellite.is_some());
    let satellite = satellite.expect("G05 antenna");
    def(
        "W6_FC_G05_KIND",
        c_enum("SIDEREON_ANTENNA_KIND", &satellite.kind),
    );
    def_bool("W6_FC_G05_HAS_VALID_FROM", satellite.valid_from.is_some());
    date(
        "W6_FC_G05_VALID_FROM",
        &satellite.valid_from.expect("valid from"),
    );
    let leap = AntexDateTime::new_with_fraction(
        2016,
        12,
        31,
        23,
        59,
        60,
        SecondFraction::new(0, 0).expect("zero fraction"),
    );
    comment("2016-12-31T23:59:60 as an ANTEX date-time: refused (true) and its kind.");
    def_bool("W6_FC_LEAP_REFUSED", leap.is_err());
    def(
        "W6_FC_LEAP_KIND",
        leap.err()
            .map_or("SIDEREON_ANTEX_ERROR_KIND_NONE".to_string(), |e| {
                antex_error_kind(&e)
            }),
    );

    comment("The inline ANTEX with two G01 sections.");
    let ambiguous = Antex::parse(&c_literal(&source, "static const char ambiguous_antex[] ="))
        .expect("ambiguous ANTEX parses");
    let block = ambiguous.antenna_blocks().next().expect("block");
    def_size("W6_FC_AMBIGUOUS_FREQUENCIES", block.frequencies.len());
    def_bool(
        "W6_FC_AMBIGUOUS_HAS_COUNT_RECORD",
        block.has_frequency_count,
    );
    let err = block.pco("G01").expect_err("ambiguous frequency refused");
    def("W6_FC_AMBIGUOUS_KIND", antex_error_kind(&err));
    match &err {
        AntexError::AmbiguousFrequency {
            frequency,
            sections,
            ..
        } => {
            def("W6_FC_AMBIGUOUS_SECTIONS", sections);
            def_str("W6_FC_AMBIGUOUS_FREQUENCY", frequency);
        }
        other => panic!("unexpected ANTEX refusal {other:?}"),
    }

    comment("The inline ANTEX whose DAZI value does not read.");
    let bad = Antex::parse(&c_literal(&source, "static const char bad_dazi_antex[] ="))
        .expect_err("bad DAZI refused");
    def("W6_FC_BAD_DAZI_KIND", antex_error_kind(&bad));
    match &bad {
        AntexError::InvalidField {
            antenna_id,
            record,
            value,
            ..
        } => {
            def_bool("W6_FC_BAD_DAZI_HAS_ANTENNA_ID", antenna_id.is_some());
            def_str(
                "W6_FC_BAD_DAZI_ANTENNA_ID",
                antenna_id.as_deref().unwrap_or(""),
            );
            def_str("W6_FC_BAD_DAZI_RECORD", record);
            def_str("W6_FC_BAD_DAZI_VALUE", value);
        }
        other => panic!("unexpected ANTEX refusal {other:?}"),
    }

    // BLQ.
    let blq = parse_ocean_loading_blq_blocks(&c_literal(&source, "static const char onsa_blq[] ="))
        .expect("BLQ parses");
    comment("The inline ONSA BLQ block.");
    def_size("W6_FC_BLQ_BLOCKS", blq.len());
    let onsa = &blq[0];
    def_size("W6_FC_BLQ_COMMENTS", onsa.comments.len());
    placement("W6_FC_BLQ_COMMENT1", onsa.comments[1].placement.clone());
    def_str("W6_FC_BLQ_COMMENT1_LINE", &onsa.comments[1].line);
    placement("W6_FC_BLQ_COMMENT2", onsa.comments[2].placement.clone());
    def_str("W6_FC_BLQ_STATION", &onsa.station);
    let amplitude: Vec<f64> = onsa
        .coefficients
        .amplitude_m
        .iter()
        .flatten()
        .copied()
        .collect();
    let phase: Vec<f64> = onsa
        .coefficients
        .phase_deg
        .iter()
        .flatten()
        .copied()
        .collect();
    def_bits_array("W6_FC_BLQ_AMPLITUDE_BITS", &amplitude);
    def_bits_array("W6_FC_BLQ_PHASE_BITS", &phase);
    comment("Writing a block whose station is $ONSA.");
    let refused = write_ocean_loading_blq_blocks(&[OceanLoadingBlqBlock {
        station: "$ONSA".to_string(),
        coefficients: onsa.coefficients.clone(),
        comments: Vec::new(),
    }])
    .expect_err("comment-like station refused");
    match &refused {
        TideError::BlqWrite { block, kind } => {
            def("W6_FC_BLQ_REFUSED_BLOCK", block);
            def(
                "W6_FC_BLQ_REFUSED_KIND",
                c_enum("SIDEREON_BLQ_WRITE_ERROR_KIND", kind),
            );
        }
        other => panic!("unexpected BLQ refusal {other:?}"),
    }

    // SBAS.
    comment("sat_to_sbas_prn for S20 and S99; an empty store's unassigned mask for S20.");
    let s20: GnssSatelliteId = "S20".parse().expect("S20");
    let s99: GnssSatelliteId = "S99".parse().expect("S99");
    let prn = sat_to_sbas_prn(s20);
    def_bool("W6_FC_S20_PRESENT", prn.is_some());
    def("W6_FC_S20_PRN", prn.unwrap_or(0));
    def_bool("W6_FC_S99_PRESENT", sat_to_sbas_prn(s99).is_some());
    def_bool(
        "W6_FC_S20_UNASSIGNED_PRESENT",
        SbasCorrectionStore::new()
            .unassigned_mask_corrections(s20)
            .is_some(),
    );

    // RTCM: 1077 with one satellite, id 65, no signals.
    let signal_mask = sidereon_core::rtcm::msm_signal_mask(&[]);
    let message = Message::Msm(MsmMessage {
        message_number: 1077,
        system: GnssSystem::Gps,
        kind: MsmKind::Msm7,
        header: MsmHeader {
            reference_station_id: 0,
            epoch_time: 0,
            multiple_message: false,
            iods: 0,
            reserved: 0,
            clock_steering: 0,
            external_clock: 0,
            divergence_free_smoothing: false,
            smoothing_interval: 0,
        },
        signal_mask,
        satellites: vec![MsmSatellite {
            id: 65,
            rough_range_ms: Some(70),
            rough_range_mod1: 0,
            extended_info: Some(0),
            rough_phase_range_rate_m_s: Some(0),
        }],
        signals: Vec::new(),
        trailing_bits: Vec::new(),
    });
    comment("RTCM MSM7 with satellite id 65: encode and frame statuses.");
    def(
        "W6_FC_MSM_ENCODE_STATUS",
        message
            .encode()
            .err()
            .map_or("SIDEREON_STATUS_OK", |e| rtcm_status(&e)),
    );
    def(
        "W6_FC_MSM_FRAME_STATUS",
        message
            .to_frame()
            .err()
            .map_or("SIDEREON_STATUS_OK", |e| rtcm_status(&e)),
    );
    valgen::header_end(guard);
}
