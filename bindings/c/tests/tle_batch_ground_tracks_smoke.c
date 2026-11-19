/* Compiled ABI smoke for batched TLE ground tracks and typed out-of-range
 * datetime errors across passes, coverage, batch look angles, Sun/Moon, and
 * time-scale entry points.
 */
#include "sidereon.h"
#include "prop_fixture.h"

#include <limits.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static uint64_t f64_bits(double value) {
    uint64_t bits = 0;
    memcpy(&bits, &value, sizeof(bits));
    return bits;
}

static int fail_msg(const char *what) {
    char msg[512];
    sidereon_last_error_message(msg, sizeof(msg));
    fprintf(stderr, "tle_batch_ground_tracks_smoke: FAIL: %s: %s\n", what, msg);
    return 1;
}

static int check_last_engine_error_invalid_datetime(const char *what) {
    SidereonEngineErrorInfo info;
    if (sidereon_last_engine_error_info(&info) != SIDEREON_STATUS_OK || info.payload_len == 0) {
        return fail_msg(what);
    }
    char *buf = calloc(info.payload_len + 1, 1);
    if (buf == NULL) {
        return fail_msg(what);
    }
    size_t written = 0, required = 0;
    int rc = 0;
    if (sidereon_last_engine_error_payload((uint8_t *)buf, info.payload_len, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != info.payload_len ||
        strstr(buf, "\"kind\":\"invalid_input\"") == NULL ||
        strstr(buf, "\"field\":\"datetime\"") == NULL ||
        strstr(buf, "\"reason\":\"invalid UTC instant\"") == NULL) {
        rc = fail_msg(what);
    }
    free(buf);
    return rc;
}

static int test_tle_batch_ground_tracks(void) {
    SidereonTle *tle = NULL;
    if (sidereon_tle_load(PROP_TLE_LINE1, PROP_TLE_LINE2, PROP_TLE_OPSMODE, &tle) !=
        SIDEREON_STATUS_OK) {
        return fail_msg("sidereon_tle_load");
    }

    SidereonGroundTrack *single = NULL;
    if (sidereon_tle_ground_track(tle, PROP_EPOCHS_UNIX_US, PROP_EPOCH_COUNT, &single) !=
        SIDEREON_STATUS_OK) {
        sidereon_tle_free(tle);
        return fail_msg("sidereon_tle_ground_track");
    }
    SidereonGeodetic single_vals[PROP_EPOCH_COUNT];
    size_t sw = 0, sr = 0;
    if (sidereon_ground_track_values(single, single_vals, PROP_EPOCH_COUNT, &sw, &sr) !=
            SIDEREON_STATUS_OK ||
        sw != PROP_EPOCH_COUNT) {
        sidereon_ground_track_free(single);
        sidereon_tle_free(tle);
        return fail_msg("sidereon_ground_track_values");
    }

    SidereonTlePair pair = {.line1 = PROP_TLE_LINE1, .line2 = PROP_TLE_LINE2};
    SidereonTlePair pairs[2] = {pair, pair};

    SidereonTleBatchGroundTracks *empty_fleet = NULL;
    if (sidereon_tle_batch_ground_tracks(NULL, 0, PROP_EPOCHS_UNIX_US, PROP_EPOCH_COUNT,
                                         PROP_TLE_OPSMODE, true,
                                         &empty_fleet) != SIDEREON_STATUS_OK ||
        empty_fleet == NULL) {
        sidereon_ground_track_free(single);
        sidereon_tle_free(tle);
        return fail_msg("empty fleet batch ground tracks");
    }
    size_t sats = 99, eps = 99, written = 99, required = 99;
    if (sidereon_tle_batch_ground_tracks_shape(empty_fleet, &sats, &eps) != SIDEREON_STATUS_OK ||
        sats != 0 || eps != PROP_EPOCH_COUNT ||
        sidereon_tle_batch_ground_tracks_values(empty_fleet, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != 0 || required != 0) {
        sidereon_tle_batch_ground_tracks_free(empty_fleet);
        sidereon_ground_track_free(single);
        sidereon_tle_free(tle);
        return fail_msg("empty fleet batch ground tracks shape/values");
    }
    sidereon_tle_batch_ground_tracks_free(empty_fleet);

    SidereonTleBatchGroundTracks *empty_epochs = NULL;
    if (sidereon_tle_batch_ground_tracks(pairs, 2, NULL, 0, PROP_TLE_OPSMODE, false,
                                         &empty_epochs) != SIDEREON_STATUS_OK ||
        empty_epochs == NULL) {
        sidereon_ground_track_free(single);
        sidereon_tle_free(tle);
        return fail_msg("empty epochs batch ground tracks");
    }
    if (sidereon_tle_batch_ground_tracks_shape(empty_epochs, &sats, &eps) != SIDEREON_STATUS_OK ||
        sats != 2 || eps != 0 ||
        sidereon_tle_batch_ground_tracks_values(empty_epochs, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != 0 || required != 0) {
        sidereon_tle_batch_ground_tracks_free(empty_epochs);
        sidereon_ground_track_free(single);
        sidereon_tle_free(tle);
        return fail_msg("empty epochs batch ground tracks shape/values");
    }
    sidereon_tle_batch_ground_tracks_free(empty_epochs);

    for (int mode = 0; mode < 2; mode++) {
        bool parallel = mode == 1;
        SidereonTleBatchGroundTracks *batch = NULL;
        if (sidereon_tle_batch_ground_tracks(pairs, 2, PROP_EPOCHS_UNIX_US, PROP_EPOCH_COUNT,
                                             PROP_TLE_OPSMODE, parallel,
                                             &batch) != SIDEREON_STATUS_OK) {
            sidereon_ground_track_free(single);
            sidereon_tle_free(tle);
            return fail_msg("sidereon_tle_batch_ground_tracks");
        }
        if (sidereon_tle_batch_ground_tracks_shape(batch, &sats, &eps) != SIDEREON_STATUS_OK ||
            sats != 2 || eps != PROP_EPOCH_COUNT) {
            sidereon_tle_batch_ground_tracks_free(batch);
            sidereon_ground_track_free(single);
            sidereon_tle_free(tle);
            return fail_msg("sidereon_tle_batch_ground_tracks_shape");
        }
        if (sidereon_tle_batch_ground_tracks_values(batch, NULL, 0, &written, &required) !=
                SIDEREON_STATUS_OK ||
            written != 0 || required != 2 * PROP_EPOCH_COUNT) {
            sidereon_tle_batch_ground_tracks_free(batch);
            sidereon_ground_track_free(single);
            sidereon_tle_free(tle);
            return fail_msg("sidereon_tle_batch_ground_tracks_values size query");
        }
        SidereonGeodetic short_buf[1];
        short_buf[0].lat_rad = 42.0;
        if (sidereon_tle_batch_ground_tracks_values(batch, short_buf, 1, &written, &required) !=
                SIDEREON_STATUS_INVALID_ARGUMENT ||
            written != 0 || required != 2 * PROP_EPOCH_COUNT || short_buf[0].lat_rad != 42.0) {
            sidereon_tle_batch_ground_tracks_free(batch);
            sidereon_ground_track_free(single);
            sidereon_tle_free(tle);
            return fail_msg("sidereon_tle_batch_ground_tracks_values short buffer");
        }
        SidereonGeodetic full_buf[2 * PROP_EPOCH_COUNT];
        if (sidereon_tle_batch_ground_tracks_values(batch, full_buf, 2 * PROP_EPOCH_COUNT, &written,
                                                    &required) != SIDEREON_STATUS_OK ||
            written != 2 * PROP_EPOCH_COUNT || required != 2 * PROP_EPOCH_COUNT) {
            sidereon_tle_batch_ground_tracks_free(batch);
            sidereon_ground_track_free(single);
            sidereon_tle_free(tle);
            return fail_msg("sidereon_tle_batch_ground_tracks_values full copy");
        }
        for (size_t sat = 0; sat < 2; sat++) {
            for (size_t ep = 0; ep < PROP_EPOCH_COUNT; ep++) {
                SidereonGeodetic got = full_buf[sat * PROP_EPOCH_COUNT + ep];
                SidereonGeodetic want = single_vals[ep];
                if (f64_bits(got.lat_rad) != f64_bits(want.lat_rad) ||
                    f64_bits(got.lon_rad) != f64_bits(want.lon_rad) ||
                    f64_bits(got.height_m) != f64_bits(want.height_m)) {
                    sidereon_tle_batch_ground_tracks_free(batch);
                    sidereon_ground_track_free(single);
                    sidereon_tle_free(tle);
                    return fail_msg("batch ground tracks mismatch single-satellite track");
                }
            }
        }
        sidereon_tle_batch_ground_tracks_free(batch);
    }

    sidereon_tle_batch_ground_tracks_free(NULL);
    sidereon_ground_track_free(single);
    sidereon_tle_free(tle);
    return 0;
}

static int test_out_of_range_instants(void) {
    SidereonTle *tle = NULL;
    if (sidereon_tle_load(PROP_TLE_LINE1, PROP_TLE_LINE2, PROP_TLE_OPSMODE, &tle) !=
        SIDEREON_STATUS_OK) {
        return fail_msg("sidereon_tle_load out-of-range");
    }
    const SidereonTle *tles[1] = {tle};
    SidereonSatelliteConstellation *constellation = NULL;
    if (sidereon_satellite_constellation_build(tles, 1, &constellation) != SIDEREON_STATUS_OK) {
        sidereon_tle_free(tle);
        return fail_msg("sidereon_satellite_constellation_build");
    }

    SidereonGroundStation station = {
        .latitude_deg = PROP_STATION_LATITUDE_DEG,
        .longitude_deg = -0.1278,
        .altitude_m = 0.0,
    };
    int64_t valid_us = 1530619200000000LL;
    int rc = 0;

    /* Pass search: start out of range, and end out of range. */
    int64_t pass_starts[2] = {INT64_MIN, valid_us};
    int64_t pass_ends[2] = {valid_us, INT64_MAX};
    for (int i = 0; i < 2; i++) {
        SidereonPassList *passes = NULL;
        if (sidereon_tle_find_passes(tle, &station, pass_starts[i], pass_ends[i], NULL, &passes) !=
                SIDEREON_STATUS_INVALID_ARGUMENT ||
            passes != NULL) {
            rc = fail_msg("sidereon_tle_find_passes out-of-range status");
            goto cleanup;
        }
        if (check_last_engine_error_invalid_datetime("sidereon_tle_find_passes out-of-range") !=
            0) {
            rc = 1;
            goto cleanup;
        }

        SidereonSatelliteConstellationPasses *cpasses = NULL;
        if (sidereon_satellite_constellation_passes(constellation, &station, pass_starts[i],
                                                    pass_ends[i], NULL,
                                                    &cpasses) != SIDEREON_STATUS_OK) {
            rc = fail_msg("sidereon_satellite_constellation_passes out-of-range");
            goto cleanup;
        }
        size_t pw = 0, pr = 0;
        if (sidereon_satellite_constellation_passes_error_payload(cpasses, 0, NULL, 0, &pw, &pr) !=
                SIDEREON_STATUS_OK ||
            pr == 0) {
            sidereon_satellite_constellation_passes_free(cpasses);
            rc = fail_msg("constellation passes error payload size");
            goto cleanup;
        }
        char *buf = calloc(pr + 1, 1);
        if (buf == NULL ||
            sidereon_satellite_constellation_passes_error_payload(cpasses, 0, (uint8_t *)buf, pr,
                                                                  &pw, &pr) != SIDEREON_STATUS_OK ||
            strstr(buf, "\"field\":\"datetime\"") == NULL ||
            strstr(buf, "\"reason\":\"invalid UTC instant\"") == NULL) {
            free(buf);
            sidereon_satellite_constellation_passes_free(cpasses);
            rc = fail_msg("constellation passes error payload");
            goto cleanup;
        }
        free(buf);
        sidereon_satellite_constellation_passes_free(cpasses);
    }

    /* Coverage grid, constellation look angles, constellation ground tracks, Sun/Moon. */
    int64_t extreme_epochs[2] = {INT64_MIN, INT64_MAX};
    for (int i = 0; i < 2; i++) {
        int64_t bad_epoch = extreme_epochs[i];
        SidereonCoverageGrid *grid = NULL;
        if (sidereon_coverage_look_angles(tles, 1, &station, 1, bad_epoch, &grid) !=
            SIDEREON_STATUS_OK) {
            rc = fail_msg("sidereon_coverage_look_angles out-of-range");
            goto cleanup;
        }
        SidereonCoverageLookAngle cell;
        size_t cw = 0, cr = 0;
        if (sidereon_coverage_grid_look_angle(grid, 0, 0, &cell) != SIDEREON_STATUS_OK || cell.ok ||
            sidereon_coverage_grid_look_angle_error_payload(grid, 0, 0, NULL, 0, &cw, &cr) !=
                SIDEREON_STATUS_OK ||
            cr == 0) {
            sidereon_coverage_grid_free(grid);
            rc = fail_msg("coverage grid out-of-range error size");
            goto cleanup;
        }
        char *cbuf = calloc(cr + 1, 1);
        if (cbuf == NULL ||
            sidereon_coverage_grid_look_angle_error_payload(grid, 0, 0, (uint8_t *)cbuf, cr, &cw,
                                                            &cr) != SIDEREON_STATUS_OK ||
            strstr(cbuf, "\"field\":\"datetime\"") == NULL ||
            strstr(cbuf, "\"reason\":\"invalid UTC instant\"") == NULL) {
            free(cbuf);
            sidereon_coverage_grid_free(grid);
            rc = fail_msg("coverage grid out-of-range error payload");
            goto cleanup;
        }
        free(cbuf);
        sidereon_coverage_grid_free(grid);

        SidereonSatelliteConstellationLookAngles *arcs = NULL;
        if (sidereon_satellite_constellation_look_angle_arcs(
                constellation, &station, &bad_epoch, 1, true, &arcs) != SIDEREON_STATUS_OK) {
            rc = fail_msg("constellation look angles out-of-range");
            goto cleanup;
        }
        size_t aw = 0, ar = 0;
        if (sidereon_satellite_constellation_look_angles_error_payload(arcs, 0, NULL, 0, &aw,
                                                                       &ar) != SIDEREON_STATUS_OK ||
            ar == 0) {
            sidereon_satellite_constellation_look_angles_free(arcs);
            rc = fail_msg("constellation look angles error size");
            goto cleanup;
        }
        char *abuf = calloc(ar + 1, 1);
        if (abuf == NULL ||
            sidereon_satellite_constellation_look_angles_error_payload(
                arcs, 0, (uint8_t *)abuf, ar, &aw, &ar) != SIDEREON_STATUS_OK ||
            strstr(abuf, "\"field\":\"datetime\"") == NULL ||
            strstr(abuf, "\"reason\":\"invalid UTC instant\"") == NULL) {
            free(abuf);
            sidereon_satellite_constellation_look_angles_free(arcs);
            rc = fail_msg("constellation look angles error payload");
            goto cleanup;
        }
        free(abuf);
        sidereon_satellite_constellation_look_angles_free(arcs);

        SidereonSatelliteConstellationGroundTracks *tracks = NULL;
        if (sidereon_satellite_constellation_ground_tracks(constellation, &bad_epoch, 1,
                                                           &tracks) != SIDEREON_STATUS_OK) {
            rc = fail_msg("constellation ground tracks out-of-range");
            goto cleanup;
        }
        size_t gw = 0, gr = 0;
        if (sidereon_satellite_constellation_ground_tracks_error_payload(
                tracks, 0, NULL, 0, &gw, &gr) != SIDEREON_STATUS_OK ||
            gr == 0) {
            sidereon_satellite_constellation_ground_tracks_free(tracks);
            rc = fail_msg("constellation ground tracks error size");
            goto cleanup;
        }
        char *gbuf = calloc(gr + 1, 1);
        if (gbuf == NULL ||
            sidereon_satellite_constellation_ground_tracks_error_payload(
                tracks, 0, (uint8_t *)gbuf, gr, &gw, &gr) != SIDEREON_STATUS_OK ||
            strstr(gbuf, "\"field\":\"datetime\"") == NULL ||
            strstr(gbuf, "\"reason\":\"invalid UTC instant\"") == NULL) {
            free(gbuf);
            sidereon_satellite_constellation_ground_tracks_free(tracks);
            rc = fail_msg("constellation ground tracks error payload");
            goto cleanup;
        }
        free(gbuf);
        sidereon_satellite_constellation_ground_tracks_free(tracks);

        double sun[3] = {0.0, 0.0, 0.0};
        double moon[3] = {0.0, 0.0, 0.0};
        if (sidereon_sun_moon_ecef(bad_epoch, sun, moon) != SIDEREON_STATUS_INVALID_ARGUMENT ||
            check_last_engine_error_invalid_datetime("sidereon_sun_moon_ecef out-of-range") != 0 ||
            sidereon_sun_moon_ecef_batch(&bad_epoch, 1, sun, 3, moon, 3) !=
                SIDEREON_STATUS_INVALID_ARGUMENT ||
            check_last_engine_error_invalid_datetime("sidereon_sun_moon_ecef_batch out-of-range") !=
                0 ||
            sidereon_sun_moon_eci_batch(&bad_epoch, 1, sun, 3, moon, 3) !=
                SIDEREON_STATUS_INVALID_ARGUMENT ||
            check_last_engine_error_invalid_datetime("sidereon_sun_moon_eci_batch out-of-range") !=
                0) {
            rc = 1;
            goto cleanup;
        }
    }

    int32_t extreme_years[4] = {INT32_MIN, -1, 10000, INT32_MAX};
    for (int i = 0; i < 4; i++) {
        SidereonTimeScales ts;
        if (sidereon_timescales_from_utc(extreme_years[i], 1, 1, 0, 0, 0.0, &ts) !=
            SIDEREON_STATUS_INVALID_ARGUMENT) {
            rc = fail_msg("sidereon_timescales_from_utc extreme year");
            goto cleanup;
        }
    }

cleanup:
    sidereon_satellite_constellation_free(constellation);
    sidereon_tle_free(tle);
    return rc;
}

int main(void) {
    if (test_tle_batch_ground_tracks() != 0) {
        return 1;
    }
    if (test_out_of_range_instants() != 0) {
        return 1;
    }
    puts("tle_batch_ground_tracks_smoke OK");
    return 0;
}
