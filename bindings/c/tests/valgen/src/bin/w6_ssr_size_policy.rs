//! inertial_tides_ssr_v2_smoke.c: an SSR correction above RTKLIB's size limit
//! under the strict and lenient policies. sidereon-core's own size tests raise
//! the G30 radial orbit correction of the real 1060 epoch past `MAXECORSSR`
//! (10 m); this is the same record, encoded as an RTCM frame the C store reads,
//! with sidereon-core's results for the exact-query corrected-state route
//! (sidereon_ssr_corrected_state_at_epoch_queries).

#[path = "../w6_shared.rs"]
mod shared;

use shared::*;
use sidereon_core::astro::time::{ExactEpoch, GnssWeekTow, TimeScale};
use sidereon_core::constants::{GPS_EPOCH_TO_J2000_S, SECONDS_PER_WEEK};
use sidereon_core::ephemeris::BroadcastEphemeris;
use sidereon_core::rtcm::{
    Message, SsrClockRecord, SsrHeader, SsrKind, SsrMessage, SsrOrbitRecord, SsrStreamAssembler,
};
use sidereon_core::ssr::{
    MissingCorrectionAction, RegionalPolicy, SsrCorrectedEphemeris, SsrCorrectionSizePolicy,
    SsrCorrectionStore, SsrFallbackPolicy,
};
use sidereon_core::staleness::StalenessPolicy;
use sidereon_core::{GnssSatelliteId, GnssSystem};

/// The core fixture's name, copied byte for byte into tests/fixtures/ssr.
const NAV: &str = "ssr/BRDC00WRD_S_20261820000_G30_G31.rnx";
/// The real 1060 epoch sidereon-core's size tests use.
const WEEK: u32 = 2425;
const TOW_S: f64 = 344_970.0;
/// Radial orbit correction, 0.1 mm: 10.0001 m, just past the 10 m limit.
const RADIAL_RAW: i32 = 100_001;
const STALENESS_S: f64 = 120.0;

fn main() {
    let guard = "SIDEREON_W6_SSR_SIZE_POLICY_PINS_H";
    valgen::header_start("w6_ssr_size_policy", guard);

    let core_nav = valgen::read_bytes(&format!("{}/{NAV}", valgen::core_fixtures()));
    let local_nav = valgen::read_bytes(&valgen::tests_path(&format!("fixtures/{NAV}")));
    assert_eq!(
        core_nav, local_nav,
        "tests/fixtures/{NAV} must be sidereon-core's fixture byte for byte"
    );
    let broadcast =
        BroadcastEphemeris::from_nav(std::str::from_utf8(&core_nav).expect("UTF-8 NAV fixture"))
            .expect("parse NAV fixture");

    let week = GnssWeekTow::new(TimeScale::Gpst, WEEK, TOW_S).expect("SSR week/TOW");
    // The fixture epoch in J2000 seconds, as sidereon-core's size tests form it.
    let t = f64::from(WEEK) * SECONDS_PER_WEEK + TOW_S - GPS_EPOCH_TO_J2000_S;
    let sat = GnssSatelliteId::new(GnssSystem::Gps, 30).expect("G30");
    let iode = broadcast
        .select_record_at(sat, t)
        .expect("G30 broadcast record")
        .issue_of_data
        .expect("G30 issue of data")
        .issue;

    let message = SsrMessage {
        message_number: 1060,
        igs_ssr_version: None,
        system: GnssSystem::Gps,
        kind: SsrKind::CombinedOrbitClock,
        header: SsrHeader {
            epoch_time_s: TOW_S as u32,
            update_interval: 0,
            multiple_message: false,
            iod_ssr: 3,
            provider_id: 9,
            solution_id: 1,
            satellite_reference_datum: Some(false),
            dispersive_bias_consistency: None,
            mw_consistency: None,
            satellite_count: 1,
        },
        orbit: vec![SsrOrbitRecord {
            satellite_id: 30,
            iode,
            iod_crc: None,
            delta_radial: RADIAL_RAW,
            delta_along: 0,
            delta_cross: 0,
            dot_delta_radial: 0,
            dot_delta_along: 0,
            dot_delta_cross: 0,
        }],
        clock: vec![SsrClockRecord {
            satellite_id: 30,
            c0: 0,
            c1: 0,
            c2: 0,
        }],
        code_bias: Vec::new(),
        phase_bias: Vec::new(),
        ura: Vec::new(),
        padding_bits: Vec::new(),
    };
    let frame = Message::Ssr(message)
        .to_frame()
        .expect("encode RTCM 1060 frame");

    // The store sidereon_ssr_store_from_rtcm builds from the frame.
    let mut store = SsrCorrectionStore::new();
    let mut assembler = SsrStreamAssembler::new();
    let mut decoded = assembler.push(&frame);
    decoded.extend(assembler.finish());
    assert_eq!(decoded.len(), 1, "one SSR message in the frame");
    for message in decoded {
        store
            .ingest(&message.expect("decode the encoded frame"), week)
            .expect("ingest RTCM 1060");
    }

    let query = ExactEpoch::from_binary_j2000_seconds(t).expect("finite query epoch");
    let source = |policy| {
        SsrCorrectedEphemeris::new(&broadcast, &store)
            .with_staleness(StalenessPolicy::seconds(STALENESS_S))
            .with_fallback(SsrFallbackPolicy {
                on_missing_correction: MissingCorrectionAction::Decline,
                regional: RegionalPolicy::DeclineRegional,
            })
            .with_correction_size_policy(policy)
    };

    comment("The RTCM 1060 frame: G30 radial orbit correction 10.0001 m.");
    def_bytes("W6_SSR_SIZE_FRAME", &frame);
    def("W6_SSR_SIZE_WEEK", WEEK);
    def_bits("W6_SSR_SIZE_TOW_S_BITS", TOW_S);
    def_bits("W6_SSR_SIZE_QUERY_J2000_S_BITS", t);
    def_bits("W6_SSR_SIZE_STALENESS_S_BITS", STALENESS_S);
    def_str("W6_SSR_SIZE_SATELLITE", "G30");
    def_str("W6_SSR_SIZE_NAV", NAV);

    let strict = source(SsrCorrectionSizePolicy::Strict);
    let refusal = strict.correction_size_refusal_at_epoch_query(sat, &query, &query);
    let strict_state = strict
        .corrected_state_with_group_delay_checked_selected_query(sat, &query, &query)
        .expect("strict exact query");
    comment("Strict: the correction is refused and its size reported.");
    let refusal = refusal.expect("strict refuses the oversized correction");
    def_bool("W6_SSR_SIZE_STRICT_HAS_STATE", strict_state.value.is_some());
    def_bits("W6_SSR_SIZE_ORBIT_M_BITS", refusal.orbit_m);
    def_bits("W6_SSR_SIZE_CLOCK_M_BITS", refusal.clock_m);
    def_bool("W6_SSR_SIZE_EXCEEDS_LIMIT", refusal.exceeds_limit());

    let lenient = source(SsrCorrectionSizePolicy::Lenient);
    let lenient_state = lenient
        .corrected_state_with_group_delay_checked_selected_query(sat, &query, &query)
        .expect("lenient exact query");
    let (position, clock, _) = lenient_state.value.expect("lenient applies the correction");
    let reports = lenient.oversized_corrections();
    assert_eq!(reports.len(), 1, "one oversized correction reported");
    let report = reports[0];
    comment("Lenient: the correction is applied and reported.");
    def_bits_array("W6_SSR_SIZE_LENIENT_POSITION_ECEF_M_BITS", &position);
    def_bits("W6_SSR_SIZE_LENIENT_CLOCK_S_BITS", clock);
    def_bits("W6_SSR_SIZE_REPORT_ORBIT_M_BITS", report.size.orbit_m);
    def_bits("W6_SSR_SIZE_REPORT_CLOCK_M_BITS", report.size.clock_m);
    def(
        "W6_SSR_SIZE_REPORT_PROVIDER_ID",
        report.solution.provider_id,
    );
    def(
        "W6_SSR_SIZE_REPORT_SOLUTION_ID",
        report.solution.solution_id,
    );
    def_bits(
        "W6_SSR_SIZE_REPORT_ORBIT_REF_EPOCH_J2000_S_BITS",
        report.orbit_ref_epoch_j2000_s,
    );
    def_bits(
        "W6_SSR_SIZE_REPORT_CLOCK_REF_EPOCH_J2000_S_BITS",
        report.clock_ref_epoch_j2000_s,
    );
    def_bits("W6_SSR_SIZE_REPORT_T_J2000_S_BITS", report.t_j2000_s);
    valgen::header_end(guard);
}
