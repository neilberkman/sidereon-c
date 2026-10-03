/* Standalone smoke test for the satellite-constellation C API.
 *
 * Builds a two-satellite constellation from parsed TLE handles, then exercises
 * every new entry point: propagate, visible, look-angle arcs, ground tracks, and
 * passes, plus the count/accessor/free calls. Cross-checks the constellation's
 * batch propagation against the per-satellite sidereon_tle_propagate path (the
 * fleet leading axis must reproduce the single-satellite arc), and frees every
 * handle. Exits 0 only if every step succeeds.
 *
 * Built and run by hand (not part of run_smoke.sh), linking the cdylib + header:
 *   cc -std=c11 -Wall -Wextra -Werror -I../include -I. constellation_smoke.c \
 *      -L<lib_dir> -lsidereon -Wl,-rpath,<lib_dir> -lm -o constellation_smoke
 */

#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sidereon.h"
#include "prop_fixture.h"
/* sidereon-core's results for this fleet, from tests/valgen
 * (w6_constellation). */
#include "w6_constellation_pins.h"

#define SAT_COUNT 2

static uint64_t f64_bits(double value) {
    uint64_t bits = 0;
    memcpy(&bits, &value, sizeof(bits));
    return bits;
}

static int fail(const char *what) {
    char msg[512];
    size_t n = sidereon_last_error_message(msg, sizeof(msg));
    (void)n;
    fprintf(stderr, "FAIL: %s: %s\n", what, msg);
    return 1;
}

int main(void) {
    /* Parse the committed ISS TLE into two independent handles so the fleet has
     * a stable, known geometry. */
    SidereonTle *tle_a = NULL;
    SidereonTle *tle_b = NULL;
    if (sidereon_tle_load(PROP_TLE_LINE1, PROP_TLE_LINE2, PROP_TLE_OPSMODE, &tle_a) !=
        SIDEREON_STATUS_OK) {
        return fail("sidereon_tle_load A");
    }
    if (sidereon_tle_load(PROP_TLE_LINE1, PROP_TLE_LINE2, PROP_TLE_OPSMODE, &tle_b) !=
        SIDEREON_STATUS_OK) {
        sidereon_tle_free(tle_a);
        return fail("sidereon_tle_load B");
    }

    const SidereonTle *tles[SAT_COUNT] = {tle_a, tle_b};
    SidereonSatelliteConstellation *constellation = NULL;
    if (sidereon_satellite_constellation_build(tles, SAT_COUNT, &constellation) !=
        SIDEREON_STATUS_OK) {
        sidereon_tle_free(tle_a);
        sidereon_tle_free(tle_b);
        return fail("sidereon_satellite_constellation_build");
    }

    int rc = 0;

    size_t sat_count = 0;
    if (sidereon_satellite_constellation_satellite_count(constellation, &sat_count) !=
            SIDEREON_STATUS_OK ||
        sat_count != SAT_COUNT) {
        rc = fail("sidereon_satellite_constellation_satellite_count");
        goto done;
    }

    /* Catalog-number accessor: query size, then fill. */
    size_t id_required = 0;
    if (sidereon_satellite_constellation_catalog_number(constellation, 0, NULL, 0, &id_required) !=
            SIDEREON_STATUS_OK ||
        id_required == 0) {
        rc = fail("sidereon_satellite_constellation_catalog_number size query");
        goto done;
    }
    char id_buf[64];
    if (id_required > sizeof(id_buf) ||
        sidereon_satellite_constellation_catalog_number(constellation, 0, id_buf, sizeof(id_buf),
                                                        &id_required) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_satellite_constellation_catalog_number fill");
        goto done;
    }
    if (id_required != strlen(W6_CONST_CATALOG0) + 1 ||
        strcmp(id_buf, W6_CONST_CATALOG0) != 0) {
        rc = fail("sidereon_satellite_constellation_catalog_number value");
        goto done;
    }
    printf("constellation: %zu satellites, sat[0] catalog=%s\n", sat_count, id_buf);

    /* A small shared epoch grid (one minute apart). */
    const size_t epoch_count = 4;
    int64_t epochs[4];
    int64_t base_us = 1530000000LL * 1000000LL; /* arbitrary unix-us epoch */
    for (size_t j = 0; j < epoch_count; j++) {
        epochs[j] = base_us + (int64_t)j * 60LL * 1000000LL;
    }

    /* Propagate the whole fleet (reuses the batch-propagation handle). */
    SidereonTleBatchPropagation *batch = NULL;
    if (sidereon_satellite_constellation_propagate(constellation, epochs, epoch_count, false,
                                                   &batch) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_satellite_constellation_propagate");
        goto done;
    }
    size_t batch_sats = 0, batch_epochs = 0;
    if (sidereon_tle_batch_propagation_shape(batch, &batch_sats, &batch_epochs) !=
            SIDEREON_STATUS_OK ||
        batch_sats != SAT_COUNT || batch_epochs != epoch_count) {
        rc = fail("sidereon_tle_batch_propagation_shape");
        sidereon_tle_batch_propagation_free(batch);
        goto done;
    }
    SidereonTemeState states[SAT_COUNT * 4];
    size_t written = 0, required = 0;
    if (sidereon_tle_batch_propagation_states(batch, states, SAT_COUNT * epoch_count, &written,
                                              &required) != SIDEREON_STATUS_OK ||
        written != SAT_COUNT * epoch_count) {
        rc = fail("sidereon_tle_batch_propagation_states");
        sidereon_tle_batch_propagation_free(batch);
        goto done;
    }
    for (size_t i = 0; i < SAT_COUNT * epoch_count; i++) {
        for (int k = 0; k < 3; k++) {
            uint64_t p_bits = 0, v_bits = 0;
            memcpy(&p_bits, &states[i].position_km[k], sizeof(p_bits));
            memcpy(&v_bits, &states[i].velocity_km_s[k], sizeof(v_bits));
            if (p_bits != W6_CONST_POSITION_BITS[i * 3 + k] ||
                v_bits != W6_CONST_VELOCITY_BITS[i * 3 + k]) {
                rc = fail("constellation propagation state");
                sidereon_tle_batch_propagation_free(batch);
                goto done;
            }
        }
    }
    sidereon_tle_batch_propagation_free(batch);

    /* Cross-check: fleet axis 0 must match the per-satellite propagate path. */
    SidereonTlePropagation *single = NULL;
    if (sidereon_tle_propagate(tle_a, epochs, epoch_count, &single) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_tle_propagate cross-check");
        goto done;
    }
    SidereonTemeState single_states[4];
    size_t sw = 0, sr = 0;
    if (sidereon_tle_propagation_states(single, single_states, epoch_count, &sw, &sr) !=
            SIDEREON_STATUS_OK ||
        sw != epoch_count) {
        rc = fail("sidereon_tle_propagation_states cross-check");
        sidereon_tle_propagation_free(single);
        goto done;
    }
    for (size_t j = 0; j < epoch_count; j++) {
        for (int k = 0; k < 3; k++) {
            if (states[j].position_km[k] != single_states[j].position_km[k]) {
                rc = fail("fleet axis 0 does not match single-satellite propagate");
                sidereon_tle_propagation_free(single);
                goto done;
            }
        }
    }
    sidereon_tle_propagation_free(single);
    printf("propagate: %zu x %zu states, fleet axis 0 == single-sat arc\n", batch_sats,
           batch_epochs);

    SidereonGroundStation station = {
        .latitude_deg = PROP_STATION_LATITUDE_DEG,
        .longitude_deg = -0.1278,
        .altitude_m = 0.0,
    };

    /* Visible snapshot. */
    SidereonVisibleList *visible = NULL;
    if (sidereon_satellite_constellation_visible(constellation, &station, epochs[0], -90.0,
                                                 &visible) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_satellite_constellation_visible");
        goto done;
    }
    size_t vis_count = 0;
    if (sidereon_visible_list_count(visible, &vis_count) != SIDEREON_STATUS_OK ||
        vis_count != W6_CONST_VISIBLE_COUNT) {
        rc = fail("sidereon_visible_list_count");
        sidereon_visible_list_free(visible);
        goto done;
    }
    sidereon_visible_list_free(visible);
    printf("visible: %zu satellite(s) above the mask\n", vis_count);

    /* Look-angle arcs. */
    SidereonSatelliteConstellationLookAngles *arcs = NULL;
    if (sidereon_satellite_constellation_look_angle_arcs(constellation, &station, epochs,
                                                         epoch_count, false, &arcs) !=
        SIDEREON_STATUS_OK) {
        rc = fail("sidereon_satellite_constellation_look_angle_arcs");
        goto done;
    }
    size_t arc_sats = 0, arc0_len = 0;
    if (sidereon_satellite_constellation_look_angles_satellite_count(arcs, &arc_sats) !=
            SIDEREON_STATUS_OK ||
        arc_sats != SAT_COUNT ||
        sidereon_satellite_constellation_look_angles_arc_len(arcs, 0, &arc0_len) !=
            SIDEREON_STATUS_OK ||
        arc0_len != W6_CONST_LOOK_ARC0_LEN) {
        rc = fail("constellation look-angle arc shape");
        sidereon_satellite_constellation_look_angles_free(arcs);
        goto done;
    }
    SidereonLookAngle looks[SAT_COUNT * 4];
    size_t lw = 0, lr = 0;
    if (sidereon_satellite_constellation_look_angles_values(arcs, looks, SAT_COUNT * epoch_count,
                                                            &lw, &lr) != SIDEREON_STATUS_OK ||
        lw != W6_CONST_LOOK_VALUE_COUNT) {
        rc = fail("sidereon_satellite_constellation_look_angles_values");
        sidereon_satellite_constellation_look_angles_free(arcs);
        goto done;
    }
    for (size_t i = 0; i < lw; i++) {
        if (f64_bits(looks[i].azimuth_deg) != W6_CONST_LOOK_AZIMUTH_DEG_BITS[i] ||
            f64_bits(looks[i].elevation_deg) != W6_CONST_LOOK_ELEVATION_DEG_BITS[i] ||
            f64_bits(looks[i].range_km) != W6_CONST_LOOK_RANGE_KM_BITS[i]) {
            rc = fail("constellation look-angle values");
            sidereon_satellite_constellation_look_angles_free(arcs);
            goto done;
        }
    }
    sidereon_satellite_constellation_look_angles_free(arcs);
    printf("look-angle arcs: %zu sats, arc[0] len=%zu, %zu values\n", arc_sats, arc0_len, lw);

    /* Invalid station geometry is retained as an indexed typed failure while
     * the legacy empty arc remains in its original fleet slot. */
    SidereonGroundStation invalid_station = station;
    invalid_station.latitude_deg = 91.0;
    SidereonSatelliteConstellationLookAngles *invalid_arcs = NULL;
    if (sidereon_satellite_constellation_look_angle_arcs(
            constellation, &invalid_station, epochs, epoch_count, false, &invalid_arcs) !=
        SIDEREON_STATUS_OK) {
        rc = fail("invalid-station constellation look angles");
        goto done;
    }
    size_t invalid_arc_len = 1, payload_written = 123, payload_required = 456;
    if (sidereon_satellite_constellation_look_angles_error_payload(
            NULL, 1, NULL, 0, &payload_written, &payload_required) !=
            SIDEREON_STATUS_NULL_POINTER ||
        payload_written != 0 || payload_required != 0) {
        rc = fail("null look-angle error handle clears counts");
        sidereon_satellite_constellation_look_angles_free(invalid_arcs);
        goto done;
    }
    payload_written = payload_required = 0;
    if (sidereon_satellite_constellation_look_angles_arc_len(invalid_arcs, 1,
                                                            &invalid_arc_len) !=
            SIDEREON_STATUS_OK ||
        invalid_arc_len != 0 ||
        sidereon_satellite_constellation_look_angles_error_payload(
            invalid_arcs, 1, NULL, 0, &payload_written, &payload_required) !=
            SIDEREON_STATUS_OK ||
        payload_written != 0 ||
        payload_required == 0) {
        rc = fail("invalid-station look-angle error size");
        sidereon_satellite_constellation_look_angles_free(invalid_arcs);
        goto done;
    }
    const size_t invalid_error_size = payload_required;
    payload_written = payload_required = 321;
    if (sidereon_satellite_constellation_look_angles_error_payload(
            invalid_arcs, SAT_COUNT, NULL, 0, &payload_written, &payload_required) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        payload_written != 0 || payload_required != 0) {
        rc = fail("out-of-range look-angle error index clears counts");
        sidereon_satellite_constellation_look_angles_free(invalid_arcs);
        goto done;
    }
    SidereonSatelliteConstellationLookAngles *empty_arcs = NULL;
    if (sidereon_satellite_constellation_look_angle_arcs(
            constellation, &station, NULL, 0, false, &empty_arcs) != SIDEREON_STATUS_OK ||
        sidereon_satellite_constellation_look_angles_error_payload(
            empty_arcs, 0, NULL, 0, &payload_written, &payload_required) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        payload_written != 0 || payload_required != 0) {
        rc = fail("successful empty look-angle arc has no error payload");
        sidereon_satellite_constellation_look_angles_free(empty_arcs);
        sidereon_satellite_constellation_look_angles_free(invalid_arcs);
        goto done;
    }
    sidereon_satellite_constellation_look_angles_free(empty_arcs);
    uint8_t *error_payload = calloc(invalid_error_size + 1, 1);
    size_t fill_required = 0;
    if (error_payload == NULL ||
        sidereon_satellite_constellation_look_angles_error_payload(
            invalid_arcs, 1, error_payload, invalid_error_size, &payload_written,
            &fill_required) != SIDEREON_STATUS_OK ||
        fill_required != invalid_error_size ||
        payload_written != invalid_error_size ||
        strstr((const char *)error_payload, "\"kind\":\"invalid_input\"") == NULL ||
        strstr((const char *)error_payload, "\"field\":\"ground_station.latitude_deg\"") == NULL ||
        strstr((const char *)error_payload, "\"reason\":\"out of range\"") == NULL) {
        rc = fail("invalid-station look-angle error payload");
        free(error_payload);
        sidereon_satellite_constellation_look_angles_free(invalid_arcs);
        goto done;
    }
    sidereon_satellite_constellation_look_angles_free(invalid_arcs);
    if (strstr((const char *)error_payload, "\"field\":\"ground_station.latitude_deg\"") == NULL) {
        rc = fail("look-angle error payload survives result free");
        free(error_payload);
        goto done;
    }
    free(error_payload);

    /* Ground tracks. */
    SidereonSatelliteConstellationGroundTracks *tracks = NULL;
    if (sidereon_satellite_constellation_ground_tracks(constellation, epochs, epoch_count,
                                                       &tracks) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_satellite_constellation_ground_tracks");
        goto done;
    }
    size_t trk_sats = 0, trk0_len = 0;
    if (sidereon_satellite_constellation_ground_tracks_satellite_count(tracks, &trk_sats) !=
            SIDEREON_STATUS_OK ||
        trk_sats != SAT_COUNT ||
        sidereon_satellite_constellation_ground_tracks_track_len(tracks, 0, &trk0_len) !=
            SIDEREON_STATUS_OK ||
        trk0_len != W6_CONST_TRACK0_LEN) {
        rc = fail("constellation ground-track shape");
        sidereon_satellite_constellation_ground_tracks_free(tracks);
        goto done;
    }
    SidereonGeodetic geo[SAT_COUNT * 4];
    size_t gw = 0, gr = 0;
    if (sidereon_satellite_constellation_ground_tracks_values(tracks, geo, SAT_COUNT * epoch_count,
                                                              &gw, &gr) != SIDEREON_STATUS_OK ||
        gw != W6_CONST_TRACK_VALUE_COUNT) {
        rc = fail("sidereon_satellite_constellation_ground_tracks_values");
        sidereon_satellite_constellation_ground_tracks_free(tracks);
        goto done;
    }
    for (size_t i = 0; i < gw; i++) {
        if (f64_bits(geo[i].lat_rad) != W6_CONST_TRACK_LAT_RAD_BITS[i] ||
            f64_bits(geo[i].lon_rad) != W6_CONST_TRACK_LON_RAD_BITS[i] ||
            f64_bits(geo[i].height_m) != W6_CONST_TRACK_HEIGHT_M_BITS[i]) {
            rc = fail("constellation ground-track values");
            sidereon_satellite_constellation_ground_tracks_free(tracks);
            goto done;
        }
    }
    sidereon_satellite_constellation_ground_tracks_free(tracks);
    printf("ground tracks: %zu sats, track[0] len=%zu, %zu values\n", trk_sats, trk0_len, gw);

    /* An epoch outside strict UT1 coverage produces a retained SGP4/frame
     * failure, distinct from a successful empty track. */
    const int64_t outside_ut1_epoch = -2208988800000000LL; /* 1900-01-01 */
    SidereonSatelliteConstellationGroundTracks *outside_tracks = NULL;
    if (sidereon_satellite_constellation_ground_tracks(
            constellation, &outside_ut1_epoch, 1, &outside_tracks) != SIDEREON_STATUS_OK) {
        rc = fail("out-of-UT1-range constellation ground tracks");
        goto done;
    }
    payload_written = payload_required = 0;
    if (sidereon_satellite_constellation_ground_tracks_error_payload(
            outside_tracks, 1, NULL, 0, &payload_written, &payload_required) !=
            SIDEREON_STATUS_OK ||
        payload_written != 0 ||
        payload_required == 0) {
        rc = fail("out-of-UT1-range ground-track error size");
        sidereon_satellite_constellation_ground_tracks_free(outside_tracks);
        goto done;
    }
    error_payload = calloc(payload_required + 1, 1);
    if (error_payload == NULL ||
        sidereon_satellite_constellation_ground_tracks_error_payload(
            outside_tracks, 1, error_payload, payload_required, &payload_written,
            &payload_required) != SIDEREON_STATUS_OK ||
        payload_written != payload_required ||
        strstr((const char *)error_payload, "\"kind\":\"frame_transform\"") == NULL ||
        strstr((const char *)error_payload, "\"kind\":\"ut1_outside_coverage\"") == NULL ||
        strstr((const char *)error_payload, "\"reason\":\"before_coverage\"") == NULL) {
        rc = fail("out-of-UT1-range ground-track error payload");
        free(error_payload);
        sidereon_satellite_constellation_ground_tracks_free(outside_tracks);
        goto done;
    }
    free(error_payload);
    sidereon_satellite_constellation_ground_tracks_free(outside_tracks);

    /* Passes over a one-day window. */
    SidereonSatelliteConstellationPasses *passes = NULL;
    int64_t end_us = base_us + 24LL * 3600LL * 1000000LL;
    if (sidereon_satellite_constellation_passes(constellation, &station, base_us, end_us, NULL,
                                                &passes) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_satellite_constellation_passes");
        goto done;
    }
    size_t pass_count = 0;
    if (sidereon_satellite_constellation_passes_count(passes, &pass_count) != SIDEREON_STATUS_OK ||
        pass_count != W6_CONST_PASS_COUNT) {
        rc = fail("sidereon_satellite_constellation_passes_count");
        sidereon_satellite_constellation_passes_free(passes);
        goto done;
    }
    if (pass_count > 0) {
        SidereonFleetPass *rows = calloc(pass_count, sizeof(*rows));
        size_t pw = 0, pr = 0;
        if (rows == NULL ||
            sidereon_satellite_constellation_passes_values(passes, rows, pass_count, &pw, &pr) !=
                SIDEREON_STATUS_OK ||
            pw != pass_count || rows[0].satellite_index >= SAT_COUNT) {
            rc = fail("sidereon_satellite_constellation_passes_values");
            free(rows);
            sidereon_satellite_constellation_passes_free(passes);
            goto done;
        }
        free(rows);
    }
    sidereon_satellite_constellation_passes_free(passes);
    printf("passes: %zu fleet pass(es) over the window\n", pass_count);

    SidereonSatelliteConstellationPasses *invalid_passes = NULL;
    if (sidereon_satellite_constellation_passes(constellation, &invalid_station, base_us, end_us,
                                                NULL, &invalid_passes) != SIDEREON_STATUS_OK) {
        rc = fail("invalid-station constellation passes");
        goto done;
    }
    payload_written = payload_required = 0;
    if (sidereon_satellite_constellation_passes_error_payload(
            invalid_passes, 1, NULL, 0, &payload_written, &payload_required) !=
            SIDEREON_STATUS_OK ||
        payload_written != 0 ||
        payload_required == 0) {
        rc = fail("invalid-station pass error size");
        sidereon_satellite_constellation_passes_free(invalid_passes);
        goto done;
    }
    error_payload = calloc(payload_required + 1, 1);
    if (error_payload == NULL ||
        sidereon_satellite_constellation_passes_error_payload(
            invalid_passes, 1, error_payload, payload_required, &payload_written,
            &payload_required) != SIDEREON_STATUS_OK ||
        payload_written != payload_required ||
        strstr((const char *)error_payload, "\"kind\":\"invalid_input\"") == NULL ||
        strstr((const char *)error_payload, "\"field\":\"ground_station.latitude_deg\"") == NULL ||
        strstr((const char *)error_payload, "\"reason\":\"out of range\"") == NULL) {
        rc = fail("invalid-station pass error payload");
        free(error_payload);
        sidereon_satellite_constellation_passes_free(invalid_passes);
        goto done;
    }
    free(error_payload);
    sidereon_satellite_constellation_passes_free(invalid_passes);

done:
    sidereon_satellite_constellation_free(constellation);
    sidereon_tle_free(tle_a);
    sidereon_tle_free(tle_b);
    if (rc == 0) {
        printf("constellation smoke OK\n");
    }
    return rc;
}
