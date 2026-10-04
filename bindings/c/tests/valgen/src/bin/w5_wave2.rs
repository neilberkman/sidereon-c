//! tests/w5_wave2_pins.h: sidereon-core's results for the inputs
//! wave2_smoke.c passes (geodesics, frame catalog, EGM2008 crop, TDM annex,
//! SP3 ECEF orbit fit, SGP4 decay latch, troposphere mapping, eclipse
//! models, w-test constants).

#[path = "w5_support/pins.rs"]
mod pins;

use pins::Pins;
use sidereon_core::astro::events::eclipse::{
    shadow_fraction, shadow_fraction_with_model, EarthShadowModel,
};
use sidereon_core::astro::sgp4::{DecayLatch, MinutesSinceEpoch, OpsMode, Satellite};
use sidereon_core::astro::time::{Instant, JulianDateSplit, TimeScale};
use sidereon_core::astro::tle::TlePolicy;
use sidereon_core::ephemeris::{OrbitFitCovariance, OrbitFitOptions, Sp3};
use sidereon_core::frame::Wgs84Geodetic;
use sidereon_core::frame_catalog::{
    catalog_entry, transform, TerrestrialFrame, TerrestrialPositionM, TerrestrialVelocityMPerYear,
};
use sidereon_core::geoid::{Egm2008GridSpacing, Egm2008RasterWindow, GeoidGrid};
use valgen::{header_end, header_start, read, read_bytes, tests_path};

const GUARD: &str = "SIDEREON_W5_WAVE2_PINS_H";

fn main() {
    header_start("w5_wave2", GUARD);
    let p = Pins::new("W5_WAVE2");

    // Geodesic inverse and direct on the one GeographicLib test row the
    // program reads (fixtures/geodesic/geodtest_one.dat).
    let row: Vec<f64> = read(&tests_path("fixtures/geodesic/geodtest_one.dat"))
        .split_whitespace()
        .take(7)
        .map(|v| v.parse::<f64>().expect("geodtest value"))
        .collect();
    let (lat1, lon1, azi1, lat2, lon2, _azi2, s12) =
        (row[0], row[1], row[2], row[3], row[4], row[5], row[6]);
    let inverse = sidereon_core::geodesic::geodesic_inverse(lat1, lon1, lat2, lon2)
        .expect("geodesic inverse");
    p.comment("sidereon_core::geodesic::geodesic_inverse and geodesic_direct on geodtest_one.dat.");
    p.f64("GEODESIC_INVERSE_DISTANCE_M_BITS", inverse.0);
    p.f64("GEODESIC_INVERSE_INITIAL_AZIMUTH_DEG_BITS", inverse.1);
    p.f64("GEODESIC_INVERSE_FINAL_AZIMUTH_DEG_BITS", inverse.2);
    let direct =
        sidereon_core::geodesic::geodesic_direct(lat1, lon1, azi1, s12).expect("geodesic direct");
    p.f64("GEODESIC_DIRECT_LATITUDE_DEG_BITS", direct.0);
    p.f64("GEODESIC_DIRECT_LONGITUDE_DEG_BITS", direct.1);
    p.f64("GEODESIC_DIRECT_FINAL_AZIMUTH_DEG_BITS", direct.2);

    // Frame catalog: ITRF2020 -> ETRF2020 entry and one station transform.
    let entry =
        catalog_entry(TerrestrialFrame::Itrf2020, TerrestrialFrame::Etrf2020).expect("entry");
    p.comment("sidereon_core::frame_catalog::catalog_entry(ITRF2020, ETRF2020) and transform.");
    p.f64(
        "FRAME_ENTRY_REFERENCE_EPOCH_YEAR_BITS",
        entry.reference_epoch_year,
    );
    p.bool("FRAME_ENTRY_HAS_PROVENANCE", !entry.provenance.is_empty());
    let state = transform(
        TerrestrialPositionM::from_array([4027893.6750, 307045.9069, 4919475.1721])
            .expect("position"),
        Some(
            TerrestrialVelocityMPerYear::from_array([-0.01361, 0.01686, 0.01024])
                .expect("velocity"),
        ),
        TerrestrialFrame::Itrf2020,
        TerrestrialFrame::Etrf2020,
        2010.0,
    )
    .expect("frame transform");
    p.f64s(
        "FRAME_TRANSFORM_POSITION_M_BITS",
        &state.position.as_array(),
    );
    p.bool("FRAME_TRANSFORM_HAS_VELOCITY", state.velocity.is_some());
    p.f64s(
        "FRAME_TRANSFORM_VELOCITY_M_PER_YEAR_BITS",
        &state.velocity.expect("velocity").as_array(),
    );

    // EGM2008 2.5-minute crop.
    let crop = read_bytes(&tests_path("fixtures/geoid/egm2008_25_norcal_crop.bin"));
    let window =
        Egm2008RasterWindow::new(Egm2008GridSpacing::TwoPointFiveMinute, 37.0, -123.0, 25, 25)
            .expect("window");
    let grid = GeoidGrid::from_egm2008_raster_window(&crop, window).expect("EGM2008 crop");
    p.comment("GeoidGrid::from_egm2008_raster_window then undulation_deg(37.7749, -122.4194).");
    p.f64(
        "EGM2008_UNDULATION_M_BITS",
        grid.undulation_deg(37.774900, -122.419400),
    );

    // TDM annex E-18.
    let tdm =
        sidereon_core::astro::tdm::parse_kvn(&read(&tests_path("fixtures/tdm/annex_e_18.kvn")))
            .expect("TDM parse");
    let records: Vec<_> = tdm
        .segments
        .iter()
        .flat_map(|segment| segment.data.records.iter())
        .collect();
    p.comment("sidereon_core::astro::tdm::parse_kvn(annex_e_18.kvn).");
    p.int("TDM_SEGMENT_COUNT", tdm.segments.len() as i128);
    p.int("TDM_RECORD_COUNT", records.len() as i128);
    p.variant(
        "TDM_FIRST_OBSERVABLE",
        "SIDEREON_TDM_OBSERVABLE_",
        &records[0].observable,
    );

    // SP3 ECEF precise-orbit fit of G01 on the prior-day GAP SP3 under the
    // options sidereon_orbit_fit_options_init writes (the core defaults).
    let sp3 = Sp3::parse(&read_bytes(&tests_path(
        "fixtures/sp3/GAP_G01_20201760000_15M.sp3",
    )))
    .expect("GAP SP3")
    .with_interpolation_options(Default::default());
    let orientation = sidereon_core::astro::frames::TdbEarthOrientationProvider::new();
    let report = sidereon_core::ephemeris::fit_sp3_ecef_precise_orbit(
        &sp3,
        "G01".parse().expect("G01"),
        &orientation,
        &OrbitFitOptions::default(),
    )
    .expect("ECEF fit");
    let fit = report.fits.values().next().expect("one fit");
    p.comment("sidereon_core::ephemeris::fit_sp3_ecef_precise_orbit(G01), default options.");
    p.int("ECEF_FIT_COUNT", report.fits.len() as i128);
    p.variant_named(
        "ECEF_FIT_COVARIANCE_KIND",
        "SIDEREON_ORBIT_FIT_COVARIANCE_KIND_",
        match fit.covariance {
            OrbitFitCovariance::Estimated { .. } => "Estimated",
            OrbitFitCovariance::Unbounded => "Unbounded",
        },
    );

    // SGP4 decay latch on the decaying 28872 TLE.
    let (satellite, _) = Satellite::from_tle_with_policy(
        "1 28872U 05037B   05333.02012661  .25992681  00000-0  24476-3 0  1534",
        "2 28872  96.4736 157.9986 0303955 244.0492 110.6523 16.46015938 10708",
        OpsMode::Improved,
        TlePolicy::Strict,
    )
    .expect("decay TLE");
    let mut latch = DecayLatch::new();
    let empty_latch_1450 =
        satellite.propagate_with_decay_latch(MinutesSinceEpoch(1450.0), &mut latch);
    latch.clear();
    let at_1440 = satellite.propagate_with_decay_latch(MinutesSinceEpoch(1440.0), &mut latch);
    let first = latch.first_failing_epoch();
    let after_1450 = satellite.propagate_with_decay_latch(MinutesSinceEpoch(1450.0), &mut latch);
    p.comment("Satellite::propagate_with_decay_latch: 1450 min on an empty latch, then 1440\nand 1450 min on a cleared latch.");
    p.outcome("DECAY_EMPTY_1450", &empty_latch_1450);
    p.outcome("DECAY_1440", &at_1440);
    p.bool("DECAY_HAS_FIRST_FAILING_EPOCH", first.is_some());
    p.f64(
        "DECAY_FIRST_FAILING_EPOCH_MIN_BITS",
        first.map_or(0.0, |e| e.0),
    );
    p.outcome("DECAY_AFTER_1450", &after_1450);

    // Troposphere mapping at one degree elevation.
    let pi = std::f64::consts::PI;
    let tropo = sidereon_core::atmosphere::troposphere::tropo_mapping(
        sidereon_core::atmosphere::troposphere::MappingModel::Niell,
        pi / 180.0,
        Wgs84Geodetic::new(0.7, -1.2, 20.0).expect("receiver"),
        Instant::from_julian_date(
            TimeScale::Utc,
            JulianDateSplit::new(2451545.0, 0.5).expect("jd"),
        ),
    );
    p.comment("sidereon_core::atmosphere::troposphere::tropo_mapping(Niell) at 1 degree.");
    p.outcome("TROPO_LOW_ELEVATION", &tropo);
    p.f64(
        "TROPO_MIN_ELEVATION_RAD_BITS",
        sidereon_core::atmosphere::troposphere::NIELL_MIN_MAPPING_ELEVATION_RAD,
    );

    // Eclipse fractions.
    let sat = [7000.0, 0.0, 0.0];
    let sun = [149597870.7, 0.0, 0.0];
    p.comment("sidereon_core::astro::events::eclipse shadow fractions.");
    p.f64(
        "ECLIPSE_LEGACY_BITS",
        shadow_fraction(sat, sun).expect("legacy"),
    );
    p.f64(
        "ECLIPSE_SPHERICAL_BITS",
        shadow_fraction_with_model(sat, sun, EarthShadowModel::Spherical).expect("spherical"),
    );
    p.f64(
        "ECLIPSE_OBLATE_BITS",
        shadow_fraction_with_model(sat, sun, EarthShadowModel::Wgs84Oblate).expect("oblate"),
    );

    // Baarda w-test constants.
    let w = sidereon_core::quality::wtest_noncentrality_components(0.001, 0.20).expect("w-test");
    p.comment("sidereon_core::quality::wtest_noncentrality_components(0.001, 0.20).");
    p.f64("WTEST_DELTA0_BITS", w.delta0);
    p.f64("WTEST_LAMBDA0_BITS", w.lambda0);

    header_end(GUARD);
}
