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

#include "sidereon.h"
#include "prop_fixture.h"

#define SAT_COUNT 2

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
    if (!isfinite(states[0].position_km[0]) || !isfinite(states[0].velocity_km_s[2])) {
        rc = fail("constellation propagation produced non-finite state");
        sidereon_tle_batch_propagation_free(batch);
        goto done;
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
    if (sidereon_visible_list_count(visible, &vis_count) != SIDEREON_STATUS_OK) {
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
        arc0_len != epoch_count) {
        rc = fail("constellation look-angle arc shape");
        sidereon_satellite_constellation_look_angles_free(arcs);
        goto done;
    }
    SidereonLookAngle looks[SAT_COUNT * 4];
    size_t lw = 0, lr = 0;
    if (sidereon_satellite_constellation_look_angles_values(arcs, looks, SAT_COUNT * epoch_count,
                                                            &lw, &lr) != SIDEREON_STATUS_OK ||
        lw != SAT_COUNT * epoch_count) {
        rc = fail("sidereon_satellite_constellation_look_angles_values");
        sidereon_satellite_constellation_look_angles_free(arcs);
        goto done;
    }
    sidereon_satellite_constellation_look_angles_free(arcs);
    printf("look-angle arcs: %zu sats, arc[0] len=%zu, %zu values\n", arc_sats, arc0_len, lw);

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
        trk0_len != epoch_count) {
        rc = fail("constellation ground-track shape");
        sidereon_satellite_constellation_ground_tracks_free(tracks);
        goto done;
    }
    SidereonGeodetic geo[SAT_COUNT * 4];
    size_t gw = 0, gr = 0;
    if (sidereon_satellite_constellation_ground_tracks_values(tracks, geo, SAT_COUNT * epoch_count,
                                                              &gw, &gr) != SIDEREON_STATUS_OK ||
        gw != SAT_COUNT * epoch_count) {
        rc = fail("sidereon_satellite_constellation_ground_tracks_values");
        sidereon_satellite_constellation_ground_tracks_free(tracks);
        goto done;
    }
    sidereon_satellite_constellation_ground_tracks_free(tracks);
    printf("ground tracks: %zu sats, track[0] len=%zu, %zu values\n", trk_sats, trk0_len, gw);

    /* Passes over a one-day window. */
    SidereonSatelliteConstellationPasses *passes = NULL;
    int64_t end_us = base_us + 24LL * 3600LL * 1000000LL;
    if (sidereon_satellite_constellation_passes(constellation, &station, base_us, end_us, NULL,
                                                &passes) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_satellite_constellation_passes");
        goto done;
    }
    size_t pass_count = 0;
    if (sidereon_satellite_constellation_passes_count(passes, &pass_count) != SIDEREON_STATUS_OK) {
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

done:
    sidereon_satellite_constellation_free(constellation);
    sidereon_tle_free(tle_a);
    sidereon_tle_free(tle_b);
    if (rc == 0) {
        printf("constellation smoke OK\n");
    }
    return rc;
}
