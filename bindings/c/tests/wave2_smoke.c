/*
 * Focused smoke for the unreleased wave-2 C binding surface.
 */
#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sidereon.h"

static int fail(const char *what) {
    char message[512];
    size_t written = sidereon_last_error_message(message, sizeof(message));
    if (written > 0) {
        fprintf(stderr, "FAIL: %s: %s\n", what, message);
    } else {
        fprintf(stderr, "FAIL: %s\n", what);
    }
    return 1;
}

static int require_ok(SidereonStatus status, const char *what) {
    if (status != SIDEREON_STATUS_OK) {
        return fail(what);
    }
    return 0;
}

static bool close_abs(double actual, double expected, double tol) {
    return fabs(actual - expected) <= tol;
}

static double angle_diff_deg(double a, double b) {
    double diff = fmod(a - b + 540.0, 360.0) - 180.0;
    return diff;
}

static uint8_t *read_file(const char *path, size_t *out_len) {
    FILE *fp = fopen(path, "rb");
    if (fp == NULL) {
        return NULL;
    }
    if (fseek(fp, 0, SEEK_END) != 0) {
        fclose(fp);
        return NULL;
    }
    long size = ftell(fp);
    if (size < 0) {
        fclose(fp);
        return NULL;
    }
    rewind(fp);
    uint8_t *data = malloc((size_t)size + 1);
    if (data == NULL) {
        fclose(fp);
        return NULL;
    }
    size_t read = fread(data, 1, (size_t)size, fp);
    fclose(fp);
    if (read != (size_t)size) {
        free(data);
        return NULL;
    }
    data[read] = 0;
    *out_len = read;
    return data;
}

static void set_sat_token(SidereonSatelliteToken *token, const char *text) {
    memset(token->bytes, 0, sizeof(token->bytes));
    memcpy(token->bytes, text, strlen(text));
}

static int test_geodesic(const char *path) {
    FILE *fp = fopen(path, "r");
    if (fp == NULL) {
        return fail("open geodtest row");
    }
    double lat1 = 0.0;
    double lon1 = 0.0;
    double azi1 = 0.0;
    double lat2 = 0.0;
    double lon2 = 0.0;
    double azi2 = 0.0;
    double s12 = 0.0;
    if (fscanf(fp, "%lf %lf %lf %lf %lf %lf %lf", &lat1, &lon1, &azi1, &lat2, &lon2,
               &azi2, &s12) != 7) {
        fclose(fp);
        return fail("parse geodtest row");
    }
    fclose(fp);

    SidereonGeodesicInverseResult inv;
    if (require_ok(sidereon_geodesic_inverse(lat1, lon1, lat2, lon2, &inv),
                   "geodesic inverse") != 0) {
        return 1;
    }
    if (!close_abs(inv.distance_m, s12, 1.0e-8) ||
        fabs(angle_diff_deg(inv.initial_azimuth_deg, azi1)) > 5.0e-13 ||
        fabs(angle_diff_deg(inv.final_azimuth_deg, azi2)) > 5.0e-13) {
        fprintf(stderr, "FAIL: geodesic inverse %.17g %.17g %.17g\n", inv.distance_m,
                inv.initial_azimuth_deg, inv.final_azimuth_deg);
        return 1;
    }

    SidereonGeodesicDirectResult direct;
    if (require_ok(sidereon_geodesic_direct(lat1, lon1, azi1, s12, &direct),
                   "geodesic direct") != 0) {
        return 1;
    }
    if (!close_abs(direct.latitude_deg, lat2, 2.0e-13) ||
        fabs(angle_diff_deg(direct.longitude_deg, lon2)) > 2.0e-13 ||
        fabs(angle_diff_deg(direct.final_azimuth_deg, azi2)) > 5.0e-13) {
        fprintf(stderr, "FAIL: geodesic direct %.17g %.17g %.17g\n", direct.latitude_deg,
                direct.longitude_deg, direct.final_azimuth_deg);
        return 1;
    }
    return 0;
}

static int test_frame_catalog(void) {
    size_t count = 0;
    if (require_ok(sidereon_frame_catalog_count(&count), "frame catalog count") != 0) {
        return 1;
    }
    if (count == 0) {
        return fail("empty frame catalog");
    }

    SidereonHelmertTransform transform;
    if (require_ok(sidereon_frame_catalog_entry(SIDEREON_TERRESTRIAL_FRAME_ITRF2020,
                                                SIDEREON_TERRESTRIAL_FRAME_ETRF2020,
                                                &transform),
                   "frame catalog entry") != 0) {
        return 1;
    }
    if (transform.reference_epoch_year != 2015.0 || transform.provenance[0] == 0) {
        return fail("frame catalog entry metadata");
    }

    SidereonTerrestrialState state = {
        .position = {.position_m = {4027893.6750, 307045.9069, 4919475.1721}},
        .has_velocity = true,
        .velocity = {.velocity_m_per_year = {-0.01361, 0.01686, 0.01024}},
    };
    SidereonTerrestrialState out;
    if (require_ok(sidereon_frame_catalog_transform(&state, SIDEREON_TERRESTRIAL_FRAME_ITRF2020,
                                                    SIDEREON_TERRESTRIAL_FRAME_ETRF2020, 2010.0,
                                                    &out),
                   "frame catalog transform") != 0) {
        return 1;
    }
    const double expected_pos[3] = {4027893.9585, 307045.5550, 4919474.9619};
    const double expected_vel[3] = {-0.00011, 0.00011, 0.00024};
    for (size_t i = 0; i < 3; i++) {
        if (!close_abs(out.position.position_m[i], expected_pos[i], 1.0e-4) ||
            !close_abs(out.velocity.velocity_m_per_year[i], expected_vel[i], 1.0e-5)) {
            return fail("frame catalog transform value");
        }
    }
    if (!out.has_velocity) {
        return fail("frame catalog velocity presence");
    }
    return 0;
}

static int test_egm2008(const char *path) {
    size_t len = 0;
    uint8_t *bytes = read_file(path, &len);
    if (bytes == NULL) {
        return fail("read EGM2008 crop");
    }
    SidereonEgm2008RasterWindow window = {
        .spacing = SIDEREON_EGM2008_GRID_SPACING_TWO_POINT_FIVE_MINUTE,
        .lat_min_deg = 37.0,
        .lon_min_deg = -123.0,
        .n_lat = 25,
        .n_lon = 25,
    };
    SidereonGeoidGrid *grid = NULL;
    SidereonStatus status =
        sidereon_geoid_grid_from_egm2008_raster_window(bytes, len, &window, &grid);
    free(bytes);
    if (require_ok(status, "EGM2008 crop load") != 0) {
        return 1;
    }
    double undulation = 0.0;
    if (require_ok(sidereon_geoid_grid_undulation_deg(grid, 37.774900, -122.419400,
                                                      &undulation),
                   "EGM2008 undulation") != 0) {
        sidereon_geoid_grid_free(grid);
        return 1;
    }
    sidereon_geoid_grid_free(grid);
    if (!close_abs(undulation, -32.163558372373, 1.0e-9)) {
        fprintf(stderr, "FAIL: EGM2008 undulation %.17g\n", undulation);
        return 1;
    }
    return 0;
}

static int test_force_model_phase_b(void) {
    SidereonStatePropagationConfig config;
    if (require_ok(sidereon_state_propagation_config_init(&config), "prop config init") != 0) {
        return 1;
    }
    config.epoch_s = 0.0;
    config.position_km[0] = 7000.0;
    config.position_km[1] = 0.0;
    config.position_km[2] = 0.0;
    config.velocity_km_s[0] = 0.0;
    config.velocity_km_s[1] = 7.5;
    config.velocity_km_s[2] = 1.0;
    config.force_model = SIDEREON_PROPAGATION_FORCE_MODEL_EARTH_PHASE_B;
    config.force_components.spherical_harmonic_max_degree = 4;
    config.force_components.spherical_harmonic_max_order = 4;
    config.max_step_s = 30.0;
    double times[2] = {0.0, 60.0};
    SidereonEphemeris *ephemeris = NULL;
    if (require_ok(sidereon_propagate_state(&config, times, 2, &ephemeris),
                   "phase B propagate") != 0) {
        return 1;
    }
    size_t count = 0;
    int failed = require_ok(sidereon_ephemeris_epoch_count(ephemeris, &count),
                            "phase B epoch count");
    sidereon_ephemeris_free(ephemeris);
    if (failed != 0) {
        return 1;
    }
    if (count != 2) {
        return fail("phase B epoch count value");
    }
    return 0;
}

static int test_tdm(const char *path) {
    size_t len = 0;
    uint8_t *bytes = read_file(path, &len);
    if (bytes == NULL) {
        return fail("read TDM annex");
    }
    SidereonTdm *tdm = NULL;
    SidereonStatus status = sidereon_tdm_parse_kvn(bytes, len, &tdm);
    free(bytes);
    if (require_ok(status, "TDM parse") != 0) {
        return 1;
    }
    size_t segments = 0;
    size_t records = 0;
    if (require_ok(sidereon_tdm_segment_count(tdm, &segments), "TDM segment count") != 0 ||
        require_ok(sidereon_tdm_record_count(tdm, &records), "TDM record count") != 0) {
        sidereon_tdm_free(tdm);
        return 1;
    }
    if (segments != 2 || records != 20) {
        sidereon_tdm_free(tdm);
        return fail("TDM counts");
    }
    size_t written = 0;
    size_t required = 0;
    if (require_ok(sidereon_tdm_records(tdm, NULL, 0, &written, &required),
                   "TDM records size") != 0) {
        sidereon_tdm_free(tdm);
        return 1;
    }
    if (required != 20) {
        sidereon_tdm_free(tdm);
        return fail("TDM record required count");
    }
    SidereonTdmDataRecord *tdm_records = calloc(required, sizeof(*tdm_records));
    if (tdm_records == NULL) {
        sidereon_tdm_free(tdm);
        return fail("TDM records alloc");
    }
    if (require_ok(sidereon_tdm_records(tdm, tdm_records, required, &written, &required),
                   "TDM records") != 0) {
        free(tdm_records);
        sidereon_tdm_free(tdm);
        return 1;
    }
    if (written != 20 || required != 20 ||
        strcmp(tdm_records[0].keyword, "TRANSMIT_PHASE_CT_1") != 0 ||
        tdm_records[0].observable != SIDEREON_TDM_OBSERVABLE_OTHER) {
        free(tdm_records);
        sidereon_tdm_free(tdm);
        return fail("TDM first record");
    }
    free(tdm_records);

    required = 0;
    written = 0;
    if (require_ok(sidereon_tdm_to_kvn(tdm, NULL, 0, &written, &required), "TDM encode size") !=
        0) {
        sidereon_tdm_free(tdm);
        return 1;
    }
    uint8_t *encoded = malloc(required);
    if (encoded == NULL) {
        sidereon_tdm_free(tdm);
        return fail("TDM encoded alloc");
    }
    if (require_ok(sidereon_tdm_to_kvn(tdm, encoded, required, &written, &required),
                   "TDM encode") != 0) {
        free(encoded);
        sidereon_tdm_free(tdm);
        return 1;
    }
    SidereonTdm *roundtrip = NULL;
    status = sidereon_tdm_parse_kvn(encoded, written, &roundtrip);
    free(encoded);
    sidereon_tdm_free(tdm);
    if (require_ok(status, "TDM roundtrip parse") != 0) {
        return 1;
    }
    size_t records2 = 0;
    int failed = require_ok(sidereon_tdm_record_count(roundtrip, &records2),
                            "TDM roundtrip record count");
    sidereon_tdm_free(roundtrip);
    if (failed != 0) {
        return 1;
    }
    if (records2 != 20) {
        return fail("TDM roundtrip count");
    }
    return 0;
}

static int test_ecef_fit(const char *path) {
    size_t len = 0;
    uint8_t *bytes = read_file(path, &len);
    if (bytes == NULL) {
        return fail("read ECEF SP3");
    }
    SidereonSp3 *sp3 = NULL;
    SidereonStatus status = sidereon_sp3_load(bytes, len, &sp3);
    free(bytes);
    if (require_ok(status, "load ECEF SP3") != 0) {
        return 1;
    }
    SidereonOrbitFitOptions options;
    if (require_ok(sidereon_orbit_fit_options_init(&options), "orbit fit options") != 0) {
        sidereon_sp3_free(sp3);
        return 1;
    }
    SidereonOrbitFitReport *report = NULL;
    status = sidereon_fit_sp3_ecef_precise_orbit(sp3, "G01", &options, &report);
    if (require_ok(status, "ECEF SP3 fit") != 0) {
        sidereon_sp3_free(sp3);
        return 1;
    }
    SidereonOrbitFitSolution fit;
    size_t written = 0;
    size_t required = 0;
    int failed = require_ok(sidereon_orbit_fit_report_fits(report, &fit, 1, &written, &required),
                            "ECEF fit report");
    sidereon_orbit_fit_report_free(report);
    if (failed != 0) {
        sidereon_sp3_free(sp3);
        return 1;
    }
    if (written != 1 || required != 1) {
        sidereon_sp3_free(sp3);
        return fail("ECEF fit report count");
    }
    if (fit.covariance.kind != SIDEREON_ORBIT_FIT_COVARIANCE_KIND_ESTIMATED &&
        fit.covariance.kind != SIDEREON_ORBIT_FIT_COVARIANCE_KIND_UNBOUNDED) {
        sidereon_sp3_free(sp3);
        return fail("ECEF fit covariance tag");
    }

    SidereonSatelliteToken token;
    set_sat_token(&token, "G01");
    report = NULL;
    if (require_ok(sidereon_fit_sp3_ecef_precise_orbits(sp3, &token, 1, &options, &report),
                   "ECEF SP3 selected fits") != 0) {
        sidereon_sp3_free(sp3);
        return 1;
    }
    sidereon_orbit_fit_report_free(report);

    report = NULL;
    if (require_ok(sidereon_fit_all_sp3_ecef_precise_orbits(sp3, &options, &report),
                   "ECEF SP3 all fits") != 0) {
        sidereon_sp3_free(sp3);
        return 1;
    }
    sidereon_orbit_fit_report_free(report);
    sidereon_sp3_free(sp3);
    return 0;
}

static int test_decay_latch(void) {
    const char *line1 =
        "1 28872U 05037B   05333.02012661  .25992681  00000-0  24476-3 0  1534";
    const char *line2 =
        "2 28872  96.4736 157.9986 0303955 244.0492 110.6523 16.46015938 10708";
    SidereonTle *tle = NULL;
    if (require_ok(sidereon_tle_load(line1, line2, SIDEREON_TLE_OPS_MODE_IMPROVED, &tle),
                   "decay TLE load") != 0) {
        return 1;
    }
    SidereonSgp4DecayLatch *latch = NULL;
    if (require_ok(sidereon_sgp4_decay_latch_new(&latch), "decay latch new") != 0) {
        sidereon_tle_free(tle);
        return 1;
    }
    SidereonTemeState state;
    if (require_ok(sidereon_tle_propagate_with_decay_latch(tle, 1450.0, latch, &state),
                   "empty latch later raw state") != 0) {
        sidereon_sgp4_decay_latch_free(latch);
        sidereon_tle_free(tle);
        return 1;
    }
    if (require_ok(sidereon_sgp4_decay_latch_clear(latch), "decay latch clear") != 0) {
        sidereon_sgp4_decay_latch_free(latch);
        sidereon_tle_free(tle);
        return 1;
    }
    if (sidereon_tle_propagate_with_decay_latch(tle, 1440.0, latch, &state) ==
        SIDEREON_STATUS_OK) {
        sidereon_sgp4_decay_latch_free(latch);
        sidereon_tle_free(tle);
        return fail("decay latch first failure");
    }
    bool has_epoch = false;
    double first_epoch = 0.0;
    if (require_ok(sidereon_sgp4_decay_latch_first_failing_epoch(latch, &has_epoch,
                                                                 &first_epoch),
                   "decay latch first epoch") != 0) {
        sidereon_sgp4_decay_latch_free(latch);
        sidereon_tle_free(tle);
        return 1;
    }
    if (!has_epoch || first_epoch != 1440.0) {
        sidereon_sgp4_decay_latch_free(latch);
        sidereon_tle_free(tle);
        return fail("decay latch first epoch value");
    }
    if (sidereon_tle_propagate_with_decay_latch(tle, 1450.0, latch, &state) ==
        SIDEREON_STATUS_OK) {
        sidereon_sgp4_decay_latch_free(latch);
        sidereon_tle_free(tle);
        return fail("decay latch later epoch");
    }
    sidereon_sgp4_decay_latch_free(latch);
    sidereon_tle_free(tle);
    return 0;
}

static int test_tropo_and_eclipse(void) {
    const double pi = 3.14159265358979323846264338327950288;
    SidereonGeodetic receiver = {.lat_rad = 0.7, .lon_rad = -1.2, .height_m = 20.0};
    SidereonMappingFactors mapping;
    SidereonTropoMappingError error;
    SidereonStatus status = sidereon_tropo_mapping_factors_checked(
        pi / 180.0, receiver, SIDEREON_TIME_SCALE_UTC, 2451545.0, 0.5, &mapping, &error);
    if (status != SIDEREON_STATUS_INVALID_ARGUMENT ||
        error.kind != SIDEREON_TROPO_MAPPING_ERROR_KIND_LOW_ELEVATION ||
        !close_abs(error.min_elevation_rad, 3.0 * pi / 180.0, 1.0e-16)) {
        return fail("troposphere low elevation typed error");
    }

    const double sat[3] = {7000.0, 0.0, 0.0};
    const double sun[3] = {149597870.7, 0.0, 0.0};
    double legacy = 0.0;
    double spherical = 0.0;
    double oblate = 0.0;
    if (require_ok(sidereon_eclipse_shadow_fraction(sat, sun, &legacy),
                   "legacy eclipse fraction") != 0 ||
        require_ok(sidereon_eclipse_shadow_fraction_with_model(
                       sat, sun, SIDEREON_EARTH_SHADOW_MODEL_SPHERICAL, &spherical),
                   "spherical eclipse fraction") != 0 ||
        require_ok(sidereon_eclipse_shadow_fraction_with_model(
                       sat, sun, SIDEREON_EARTH_SHADOW_MODEL_WGS84_OBLATE, &oblate),
                   "oblate eclipse fraction") != 0) {
        return 1;
    }
    if (legacy != spherical || !isfinite(oblate)) {
        return fail("eclipse model fractions");
    }
    return 0;
}

static int test_reliability_components(void) {
    SidereonWTestNoncentrality constants;
    if (require_ok(sidereon_wtest_noncentrality(0.001, 0.20, &constants),
                   "w-test noncentrality") != 0) {
        return 1;
    }
    if (constants.delta0 != 4.132147965064809 ||
        constants.lambda0 != 17.074646805189243) {
        return fail("w-test core constants");
    }
    return 0;
}

int main(int argc, char **argv) {
    if (argc != 5) {
        fprintf(stderr, "usage: %s geodtest egm2008_crop tdm_annex ecef_sp3\n", argv[0]);
        return 2;
    }
    if (test_geodesic(argv[1]) != 0) {
        return 1;
    }
    if (test_frame_catalog() != 0) {
        return 1;
    }
    if (test_egm2008(argv[2]) != 0) {
        return 1;
    }
    if (test_force_model_phase_b() != 0) {
        return 1;
    }
    if (test_tdm(argv[3]) != 0) {
        return 1;
    }
    if (test_ecef_fit(argv[4]) != 0) {
        return 1;
    }
    if (test_decay_latch() != 0) {
        return 1;
    }
    if (test_tropo_and_eclipse() != 0) {
        return 1;
    }
    if (test_reliability_components() != 0) {
        return 1;
    }
    return 0;
}
