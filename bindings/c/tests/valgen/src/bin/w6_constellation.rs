//! constellation_smoke.c: sidereon-core's catalog number, TEME states,
//! visibility and pass count for the two-satellite fleet built from the
//! committed ISS TLE, at the epochs and station that test uses.

#[path = "../w6_shared.rs"]
mod shared;

use shared::*;
use sidereon_core::astro::passes::{
    find_passes_for_satellite, ground_track, look_angle_batch_serial, propagate_teme_batch_serial,
    visible_from_satellites, GroundStation, PassFinderOptions, UtcInstant,
};
use sidereon_core::astro::sgp4::{OpsMode, Satellite};
use sidereon_core::astro::tle::{parse_with_policy, TlePolicy};
use valgen::tests_path;

fn main() {
    let guard = "SIDEREON_W6_CONSTELLATION_PINS_H";
    valgen::header_start("w6_constellation", guard);
    let header = tests_path("prop_fixture.h");
    let line1 = header_define_string(&header, "PROP_TLE_LINE1");
    let line2 = header_define_string(&header, "PROP_TLE_LINE2");
    let elements = parse_with_policy(&line1, &line2, TlePolicy::Strict)
        .expect("TLE")
        .elements;
    let (satellite, _) =
        Satellite::from_tle_with_policy(&line1, &line2, OpsMode::Afspc, TlePolicy::Strict)
            .expect("ISS TLE");
    let satellites = vec![satellite.clone(), satellite];
    let ids = vec![
        elements.catalog_number.clone(),
        elements.catalog_number.clone(),
    ];
    comment("Catalog number of satellite 0.");
    def_str("W6_CONST_CATALOG0", &ids[0]);

    // The test's grid: 1530000000 s plus 0, 60, 120 and 180 s.
    let base_us: i64 = 1530000000 * 1000000;
    let epochs: Vec<UtcInstant> = (0..4)
        .map(|j| UtcInstant::from_unix_microseconds(base_us + j * 60 * 1000000))
        .collect();
    let arcs = propagate_teme_batch_serial(&satellites, &epochs);
    let mut positions = Vec::new();
    let mut velocities = Vec::new();
    for arc in &arcs {
        for prediction in arc.as_ref().expect("propagated arc") {
            positions.extend(prediction.position);
            velocities.extend(prediction.velocity);
        }
    }
    comment("Fleet TEME states, satellite-major, km and km/s.");
    def_bits_array("W6_CONST_POSITION_BITS", &positions);
    def_bits_array("W6_CONST_VELOCITY_BITS", &velocities);

    let station = GroundStation {
        latitude_deg: header_define_f64(&header, "PROP_STATION_LATITUDE_DEG"),
        longitude_deg: -0.1278,
        altitude_m: 0.0,
    };
    let visible = visible_from_satellites(
        &satellites,
        &ids,
        station,
        UtcInstant::from_unix_microseconds(base_us),
        -90.0,
    )
    .expect("visible");
    comment("Satellites above -90 deg at the first epoch.");
    def_size("W6_CONST_VISIBLE_COUNT", visible.len());

    // sidereon_satellite_constellation_look_angle_arcs (serial) and
    // sidereon_satellite_constellation_ground_tracks: one arc per satellite,
    // flattened satellite-major; a satellite whose arc fails is empty.
    let looks = look_angle_batch_serial(&satellites, station, &epochs);
    let mut azimuth = Vec::new();
    let mut elevation = Vec::new();
    let mut range = Vec::new();
    let mut arc_lens = Vec::new();
    for arc in &looks {
        let arc = arc.clone().unwrap_or_default();
        arc_lens.push(arc.len());
        for look in &arc {
            azimuth.push(look.azimuth_deg);
            elevation.push(look.elevation_deg);
            range.push(look.range_km);
        }
    }
    comment("Look-angle arcs: arc 0 length and every value, satellite-major.");
    def_size("W6_CONST_LOOK_ARC0_LEN", arc_lens[0]);
    def_size("W6_CONST_LOOK_VALUE_COUNT", azimuth.len());
    def_bits_array("W6_CONST_LOOK_AZIMUTH_DEG_BITS", &azimuth);
    def_bits_array("W6_CONST_LOOK_ELEVATION_DEG_BITS", &elevation);
    def_bits_array("W6_CONST_LOOK_RANGE_KM_BITS", &range);

    let mut lat = Vec::new();
    let mut lon = Vec::new();
    let mut height = Vec::new();
    let mut track_lens = Vec::new();
    for satellite in &satellites {
        let track = ground_track(satellite, &epochs).unwrap_or_default();
        track_lens.push(track.len());
        for point in &track {
            lat.push(point.lat_rad);
            lon.push(point.lon_rad);
            height.push(point.height_m);
        }
    }
    comment("Ground tracks: track 0 length and every point, satellite-major.");
    def_size("W6_CONST_TRACK0_LEN", track_lens[0]);
    def_size("W6_CONST_TRACK_VALUE_COUNT", lat.len());
    def_bits_array("W6_CONST_TRACK_LAT_RAD_BITS", &lat);
    def_bits_array("W6_CONST_TRACK_LON_RAD_BITS", &lon);
    def_bits_array("W6_CONST_TRACK_HEIGHT_M_BITS", &height);

    let end_us = base_us + 24 * 3600 * 1000000;
    let mut pass_count = 0;
    for satellite in &satellites {
        if let Ok(passes) = find_passes_for_satellite(
            satellite,
            station,
            UtcInstant::from_unix_microseconds(base_us),
            UtcInstant::from_unix_microseconds(end_us),
            PassFinderOptions::default(),
        ) {
            pass_count += passes.len();
        }
    }
    comment("Fleet passes over one day with the default pass-finder options.");
    def_size("W6_CONST_PASS_COUNT", pass_count);
    valgen::header_end(guard);
}
