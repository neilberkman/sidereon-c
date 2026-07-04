/*
 * Provenance: C binding smoke for sidereon-core 0.12 public APIs. The inputs
 * come from committed binding fixtures plus the local sidereon-core public test
 * fixtures passed by tests/run_smoke.sh. Each check calls only sidereon.h.
 */
#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sidereon.h"

#ifndef M_PI
#define M_PI 3.14159265358979323846
#endif

static int failures = 0;

static void check(int ok, const char *what) {
    if (!ok) {
        char msg[512];
        size_t n = sidereon_last_error_message(msg, sizeof(msg));
        if (n == 0) {
            msg[0] = '\0';
        }
        fprintf(stderr, "FAIL: %s (last_error: %s)\n", what, msg);
        failures++;
    }
}

static void check_close(double got, double want, double tol, const char *what) {
    check(isfinite(got) && fabs(got - want) <= tol, what);
}

static uint64_t f64_bits(double value) {
    uint64_t bits;
    memcpy(&bits, &value, sizeof(bits));
    return bits;
}

static bool f64_same(double a, double b) {
    return f64_bits(a) == f64_bits(b);
}

static uint8_t *read_file(const char *path, size_t *out_len) {
    FILE *f = fopen(path, "rb");
    if (!f) {
        fprintf(stderr, "FAIL: cannot open %s\n", path);
        failures++;
        return NULL;
    }
    if (fseek(f, 0, SEEK_END) != 0) {
        fclose(f);
        failures++;
        return NULL;
    }
    long size = ftell(f);
    if (size < 0) {
        fclose(f);
        failures++;
        return NULL;
    }
    rewind(f);
    uint8_t *buf = (uint8_t *)malloc((size_t)size + 1);
    if (!buf) {
        fclose(f);
        failures++;
        return NULL;
    }
    size_t got = fread(buf, 1, (size_t)size, f);
    fclose(f);
    if (got != (size_t)size) {
        free(buf);
        failures++;
        return NULL;
    }
    buf[got] = 0;
    *out_len = got;
    return buf;
}

static int hex_nibble(char c) {
    if (c >= '0' && c <= '9') {
        return c - '0';
    }
    if (c >= 'a' && c <= 'f') {
        return 10 + c - 'a';
    }
    if (c >= 'A' && c <= 'F') {
        return 10 + c - 'A';
    }
    return -1;
}

static size_t hex_to_bytes(const char *hex, uint8_t *out, size_t cap) {
    size_t n = strlen(hex);
    if ((n % 2) != 0 || cap < n / 2) {
        return 0;
    }
    for (size_t i = 0; i < n; i += 2) {
        int hi = hex_nibble(hex[i]);
        int lo = hex_nibble(hex[i + 1]);
        if (hi < 0 || lo < 0) {
            return 0;
        }
        out[i / 2] = (uint8_t)((hi << 4) | lo);
    }
    return n / 2;
}

typedef SidereonStatus (*AllanEstimatorFn)(const SidereonAllanSample *samples,
                                           size_t count,
                                           uint32_t series_kind,
                                           double tau0_s,
                                           const size_t *averaging_factors,
                                           size_t averaging_factor_count,
                                           SidereonAllanPoint *out,
                                           size_t len,
                                           size_t *out_written,
                                           size_t *out_required);

static void compare_allan_curve(SidereonAllanDeviationCurves *curves,
                                const SidereonAllanSample *samples,
                                size_t sample_count,
                                const size_t *factors,
                                size_t factor_count,
                                uint32_t estimator,
                                AllanEstimatorFn fn,
                                const char *name) {
    bool present = false;
    check(sidereon_clock_allan_curve_present(curves, estimator, &present) == SIDEREON_STATUS_OK &&
              present,
          name);
    SidereonAllanPoint combined[4];
    SidereonAllanPoint direct[4];
    size_t combined_written = 0;
    size_t combined_required = 0;
    size_t direct_written = 0;
    size_t direct_required = 0;
    check(sidereon_clock_allan_curve(curves, estimator, combined, 4, &combined_written,
                                     &combined_required) == SIDEREON_STATUS_OK &&
              combined_written == factor_count && combined_required == factor_count,
          name);
    check(fn(samples, sample_count, SIDEREON_ALLAN_SERIES_KIND_PHASE_SECONDS, 1.0, factors,
             factor_count, direct, 4, &direct_written, &direct_required) == SIDEREON_STATUS_OK &&
              direct_written == factor_count && direct_required == factor_count,
          name);
    for (size_t i = 0; i < factor_count; i++) {
        check(f64_same(combined[i].tau_s, direct[i].tau_s) &&
                  f64_same(combined[i].deviation, direct[i].deviation) &&
                  combined[i].n == direct[i].n,
              name);
    }
}

static void test_clock_stability(void) {
    int start = failures;
    SidereonAllanSample samples[12];
    for (size_t i = 0; i < 12; i++) {
        samples[i].has_value = true;
        samples[i].value = 1.0e-9 * (double)((i + 1) * (i + 3));
    }
    const size_t factors[2] = {1, 2};

    SidereonAllanOptions options;
    if (sidereon_clock_allan_options_init(&options) != SIDEREON_STATUS_OK) {
        check(0, "clock options init");
        return;
    }
    options.estimators.adev = true;
    options.estimators.overlapping_adev = true;
    options.estimators.mdev = true;
    options.estimators.hdev = true;
    options.estimators.tdev = true;
    options.tau_grid = SIDEREON_ALLAN_TAU_GRID_EXPLICIT;
    options.gap_policy = SIDEREON_ALLAN_GAP_POLICY_REJECT;
    options.averaging_factors = factors;
    options.averaging_factor_count = 2;

    SidereonAllanDeviationCurves *curves = NULL;
    check(sidereon_clock_compute_allan_deviations(samples, 12,
                                                  SIDEREON_ALLAN_SERIES_KIND_PHASE_SECONDS, 1.0,
                                                  &options, &curves) == SIDEREON_STATUS_OK &&
              curves != NULL,
          "clock compute Allan curves");
    if (curves == NULL) {
        return;
    }

    compare_allan_curve(curves, samples, 12, factors, 2, SIDEREON_ALLAN_ESTIMATOR_ADEV,
                        sidereon_clock_allan_deviation, "clock ADEV parity");
    compare_allan_curve(curves, samples, 12, factors, 2,
                        SIDEREON_ALLAN_ESTIMATOR_OVERLAPPING_ADEV,
                        sidereon_clock_overlapping_adev, "clock OADEV parity");
    compare_allan_curve(curves, samples, 12, factors, 2, SIDEREON_ALLAN_ESTIMATOR_MDEV,
                        sidereon_clock_modified_adev, "clock MDEV parity");
    compare_allan_curve(curves, samples, 12, factors, 2, SIDEREON_ALLAN_ESTIMATOR_HDEV,
                        sidereon_clock_hadamard_deviation, "clock HDEV parity");
    compare_allan_curve(curves, samples, 12, factors, 2, SIDEREON_ALLAN_ESTIMATOR_TDEV,
                        sidereon_clock_time_deviation, "clock TDEV parity");
    sidereon_clock_allan_deviation_curves_free(curves);

    if (failures == start) {
        printf("clock_stability_smoke: OK (5 estimators, 2 taus)\n");
    }
}

static void test_terrain_batch(const char *dted_root) {
    int start = failures;
    SidereonDtedLookupOptions options;
    SidereonDtedTerrain *terrain = NULL;
    check(sidereon_dted_lookup_options_init(&options) == SIDEREON_STATUS_OK,
          "terrain options init");
    check(sidereon_dted_terrain_new(dted_root, &terrain) == SIDEREON_STATUS_OK && terrain != NULL,
          "terrain new");
    if (terrain == NULL) {
        return;
    }

    SidereonLonLatDeg points[3] = {{-106.5, 36.5}, {-106.25, 36.75}, {-106.5, NAN}};
    SidereonDtedHeightResult batch[3];
    memset(batch, 0, sizeof(batch));
    check(sidereon_dted_terrain_height_batch_m(terrain, points, 3, &options, batch) ==
              SIDEREON_STATUS_OK,
          "terrain height batch");
    for (size_t i = 0; i < 2; i++) {
        double scalar = 0.0;
        check(sidereon_dted_terrain_height_m_with_options(
                  terrain, points[i].lon_deg, points[i].lat_deg, &options, &scalar) ==
                  SIDEREON_STATUS_OK,
              "terrain scalar reference");
        check(batch[i].status == SIDEREON_STATUS_OK && batch[i].has_height_m &&
                  f64_same(batch[i].height_m, scalar),
              "terrain batch scalar parity");
    }
    check(batch[2].status == SIDEREON_STATUS_INVALID_ARGUMENT && !batch[2].has_height_m,
          "terrain batch per-point error");
    sidereon_dted_terrain_free(terrain);

    if (failures == start) {
        printf("terrain_batch_smoke: OK (3 points)\n");
    }
}

static void test_mmap_terrain_store(const char *dted_root) {
    int start = failures;
    SidereonDtedLookupOptions options;
    check(sidereon_dted_lookup_options_init(&options) == SIDEREON_STATUS_OK,
          "mmap terrain options init");

    size_t written = 0;
    size_t required = 0;
    check(sidereon_dted_tree_to_mmap_store(dted_root, NULL, 0, &written, &required) ==
                  SIDEREON_STATUS_OK &&
              written == 0 && required > 0,
          "mmap terrain store sizing");
    if (required == 0) {
        return;
    }
    const size_t store_len = required;

    uint8_t *store = (uint8_t *)malloc(store_len);
    check(store != NULL, "mmap terrain store allocation");
    if (store == NULL) {
        return;
    }
    check(sidereon_dted_tree_to_mmap_store(dted_root, store, store_len, &written, &required) ==
                  SIDEREON_STATUS_OK &&
              written == store_len && required == store_len,
          "mmap terrain store build");

    uint64_t byte_checksum = 0;
    check(sidereon_terrain_store_checksum64(store, store_len, &byte_checksum) ==
              SIDEREON_STATUS_OK,
          "mmap terrain store checksum bytes");

    SidereonMmapTerrain *mmap = NULL;
    check(sidereon_mmap_terrain_from_bytes(store, store_len, &mmap) == SIDEREON_STATUS_OK &&
              mmap != NULL,
          "mmap terrain from bytes");
    SidereonMmapTerrain *mmap_vec = NULL;
    check(sidereon_mmap_terrain_from_vec(store, store_len, &mmap_vec) == SIDEREON_STATUS_OK &&
              mmap_vec != NULL,
          "mmap terrain from vec");
    sidereon_mmap_terrain_free(mmap_vec);

    const char *store_path = "sidereon_c_terrain_store_test.bin";
    remove(store_path);
    check(sidereon_write_dted_tree_to_mmap_store(dted_root, store_path) == SIDEREON_STATUS_OK,
          "mmap terrain write store");
    SidereonMmapTerrain *mmap_path = NULL;
    check(sidereon_mmap_terrain_from_path(store_path, &mmap_path) == SIDEREON_STATUS_OK &&
              mmap_path != NULL,
          "mmap terrain from path");
    sidereon_mmap_terrain_free(mmap_path);
    remove(store_path);

    if (mmap == NULL) {
        free(store);
        return;
    }

    uint64_t handle_checksum = 0;
    check(sidereon_mmap_terrain_checksum64(mmap, &handle_checksum) == SIDEREON_STATUS_OK &&
              handle_checksum == byte_checksum,
          "mmap terrain checksum handle");

    SidereonVerticalDatum datum = 0;
    check(sidereon_mmap_terrain_vertical_datum(mmap, &datum) == SIDEREON_STATUS_OK &&
              datum == SIDEREON_VERTICAL_DATUM_EGM96_MSL_ORTHOMETRIC,
          "mmap terrain vertical datum");

    written = 0;
    required = 0;
    check(sidereon_mmap_terrain_tile_index(mmap, NULL, 0, &written, &required) ==
                  SIDEREON_STATUS_OK &&
              written == 0 && required > 0,
          "mmap terrain tile index sizing");
    SidereonTerrainStoreTileIndex *index =
        (SidereonTerrainStoreTileIndex *)calloc(required, sizeof(*index));
    check(index != NULL, "mmap terrain tile index allocation");
    if (index != NULL) {
        check(sidereon_mmap_terrain_tile_index(mmap, index, required, &written, &required) ==
                      SIDEREON_STATUS_OK &&
                  written == required && index[0].lon_count > 1 && index[0].lat_count > 1 &&
                  index[0].vertical_datum == SIDEREON_VERTICAL_DATUM_EGM96_MSL_ORTHOMETRIC,
              "mmap terrain tile index copy");
        free(index);
    }

    size_t roundtrip_required = 0;
    check(sidereon_mmap_terrain_to_bytes(mmap, NULL, 0, &written, &roundtrip_required) ==
                  SIDEREON_STATUS_OK &&
              roundtrip_required == store_len,
          "mmap terrain to_bytes sizing");
    uint8_t *roundtrip = (uint8_t *)malloc(roundtrip_required);
    check(roundtrip != NULL, "mmap terrain to_bytes allocation");
    if (roundtrip != NULL) {
        check(sidereon_mmap_terrain_to_bytes(mmap, roundtrip, roundtrip_required, &written,
                                             &roundtrip_required) == SIDEREON_STATUS_OK &&
                  written == store_len && roundtrip_required == store_len &&
                  memcmp(roundtrip, store, store_len) == 0,
              "mmap terrain to_bytes parity");
        free(roundtrip);
    }

    SidereonDtedTerrain *terrain = NULL;
    check(sidereon_dted_terrain_new(dted_root, &terrain) == SIDEREON_STATUS_OK && terrain != NULL,
          "mmap terrain DTED reference");
    SidereonLonLatDeg points[3] = {{-106.5, 36.5}, {-106.25, 36.75}, {-106.5, NAN}};
    for (size_t i = 0; i < 2 && terrain != NULL; i++) {
        double reference = 0.0;
        SidereonOrthometricHeightM scalar = {0.0};
        check(sidereon_dted_terrain_height_m_with_options(
                  terrain, points[i].lon_deg, points[i].lat_deg, &options, &reference) ==
                  SIDEREON_STATUS_OK,
              "mmap terrain DTED scalar");
        check(sidereon_mmap_terrain_orthometric_height_m_with_options(
                  mmap, points[i].lon_deg, points[i].lat_deg, &options, &scalar) ==
                  SIDEREON_STATUS_OK,
              "mmap terrain orthometric scalar");
        check(f64_same(scalar.value_m, reference), "mmap terrain scalar parity");
    }

    SidereonTerrainHeightResult batch[3];
    memset(batch, 0, sizeof(batch));
    check(sidereon_mmap_terrain_orthometric_height_batch(mmap, points, 3, &options, batch) ==
              SIDEREON_STATUS_OK,
          "mmap terrain orthometric batch");
    check(batch[0].status == SIDEREON_STATUS_OK && batch[0].has_orthometric_height_m,
          "mmap terrain batch first result");
    check(batch[2].status == SIDEREON_STATUS_INVALID_ARGUMENT &&
              !batch[2].has_orthometric_height_m,
          "mmap terrain batch per-point error");

    SidereonEllipsoidalHeightM ellipsoidal = {0.0};
    check(sidereon_mmap_terrain_ellipsoidal_height_m_with_model(
              mmap, -106.5, 36.5, &options, SIDEREON_TERRAIN_GEOID_MODEL_EGM96_ONE_DEGREE, NULL,
              &ellipsoidal) == SIDEREON_STATUS_OK &&
              isfinite(ellipsoidal.value_m),
          "mmap terrain ellipsoidal one-degree");

    char missing_dac[512];
    int missing_len = snprintf(missing_dac, sizeof(missing_dac), "%s/WW15MGH.DAC", dted_root);
    check(missing_len > 0 && (size_t)missing_len < sizeof(missing_dac),
          "mmap terrain missing DAC path");
    SidereonEgm96FifteenMinuteGeoid *geoid = NULL;
    check(sidereon_egm96_15m_geoid_from_ww15mgh_dac_path(missing_dac, &geoid) ==
                  SIDEREON_STATUS_INVALID_ARGUMENT &&
              geoid == NULL,
          "mmap terrain missing 15-minute geoid");
    SidereonTerrainDatumError datum_error;
    memset(&datum_error, 0, sizeof(datum_error));
    check(sidereon_last_terrain_datum_error(&datum_error) == SIDEREON_STATUS_OK &&
              datum_error.kind == SIDEREON_TERRAIN_DATUM_ERROR_KIND_MISSING_EGM96_DAC &&
              strstr((const char *)datum_error.path, "WW15MGH.DAC") != NULL &&
              strstr((const char *)datum_error.remediation, "WW15MGH.DAC") != NULL,
          "mmap terrain missing DAC typed error");
    sidereon_egm96_15m_geoid_free(geoid);

    sidereon_dted_terrain_free(terrain);
    sidereon_mmap_terrain_free(mmap);
    free(store);

    if (failures == start) {
        printf("mmap_terrain_store_smoke: OK (DTED parity, typed missing DAC)\n");
    }
}

static bool copy_doubles(SidereonStatus (*fn)(const SidereonIonex *,
                                              double *,
                                              size_t,
                                              size_t *,
                                              size_t *),
                         const SidereonIonex *ionex,
                         double **out,
                         size_t *out_count,
                         const char *what) {
    size_t written = 0;
    size_t required = 0;
    if (fn(ionex, NULL, 0, &written, &required) != SIDEREON_STATUS_OK || written != 0) {
        check(0, what);
        return false;
    }
    double *values = NULL;
    if (required > 0) {
        values = (double *)calloc(required, sizeof(*values));
        if (!values) {
            check(0, what);
            return false;
        }
    }
    if (fn(ionex, values, required, &written, &required) != SIDEREON_STATUS_OK ||
        written != required) {
        free(values);
        check(0, what);
        return false;
    }
    *out = values;
    *out_count = required;
    return true;
}

static void test_ionex_samples(const char *ionex_path) {
    int start = failures;
    size_t len = 0;
    uint8_t *bytes = read_file(ionex_path, &len);
    if (!bytes) {
        return;
    }

    SidereonIonex *ionex = NULL;
    SidereonIonex *from_grid = NULL;
    SidereonIonex *from_nodes = NULL;
    double *epochs = NULL;
    double *lats = NULL;
    double *lons = NULL;
    double *tec = NULL;
    double *rms = NULL;
    SidereonTecSample *samples = NULL;

    check(sidereon_ionex_parse(bytes, len, &ionex) == SIDEREON_STATUS_OK && ionex != NULL,
          "IONEX parse");
    free(bytes);
    bytes = NULL;
    if (ionex == NULL) {
        goto cleanup;
    }

    SidereonTecGridSamplesInfo info;
    check(sidereon_ionex_tec_grid_samples_info(ionex, &info) == SIDEREON_STATUS_OK,
          "IONEX sample info");
    size_t epoch_count = 0;
    size_t lat_count = 0;
    size_t lon_count = 0;
    size_t tec_count = 0;
    size_t rms_count = 0;
    check(copy_doubles(sidereon_ionex_tec_grid_samples_epochs_j2000_s, ionex, &epochs,
                       &epoch_count, "IONEX sample epochs"),
          "IONEX sample epochs copied");
    check(copy_doubles(sidereon_ionex_lat_nodes_deg, ionex, &lats, &lat_count, "IONEX lat nodes"),
          "IONEX lat nodes copied");
    check(copy_doubles(sidereon_ionex_lon_nodes_deg, ionex, &lons, &lon_count, "IONEX lon nodes"),
          "IONEX lon nodes copied");
    check(copy_doubles(sidereon_ionex_tec_grid_samples_tec_maps_tecu, ionex, &tec, &tec_count,
                       "IONEX TEC maps"),
          "IONEX TEC maps copied");
    check(copy_doubles(sidereon_ionex_tec_grid_samples_rms_maps_tecu, ionex, &rms, &rms_count,
                       "IONEX RMS maps"),
          "IONEX RMS maps copied");
    check(epoch_count == info.map_epoch_count && lat_count == info.lat_node_count &&
              lon_count == info.lon_node_count && tec_count == info.tec_map_value_count &&
              rms_count == info.rms_map_value_count,
          "IONEX sample counts");

    SidereonTecGridSamples grid;
    memset(&grid, 0, sizeof(grid));
    grid.time_scale = SIDEREON_TIME_SCALE_UTC;
    grid.map_epochs_j2000_s = epochs;
    grid.map_epoch_count = epoch_count;
    grid.lat_nodes_deg = lats;
    grid.lat_node_count = lat_count;
    grid.lon_nodes_deg = lons;
    grid.lon_node_count = lon_count;
    grid.dlat_deg = info.dlat_deg;
    grid.dlon_deg = info.dlon_deg;
    grid.shell_height_km = info.shell_height_km;
    grid.base_radius_km = info.base_radius_km;
    grid.exponent = info.exponent;
    grid.tec_maps_tecu = tec;
    grid.tec_map_value_count = tec_count;
    grid.has_rms_maps = info.has_rms_maps;
    grid.rms_maps_tecu = rms;
    grid.rms_map_value_count = rms_count;
    check(sidereon_ionex_from_tec_grid_samples(&grid, &from_grid) == SIDEREON_STATUS_OK &&
              from_grid != NULL,
          "IONEX from grid samples");

    size_t written = 0;
    size_t required = 0;
    check(sidereon_ionex_tec_samples(ionex, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
              written == 0 && required == tec_count,
          "IONEX node sample count");
    samples = (SidereonTecSample *)calloc(required, sizeof(*samples));
    if (!samples) {
        check(0, "IONEX node sample allocation");
        goto cleanup;
    }
    check(sidereon_ionex_tec_samples(ionex, samples, required, &written, &required) ==
                  SIDEREON_STATUS_OK &&
              written == required,
          "IONEX node samples");
    check(sidereon_ionex_from_tec_samples(samples, required, info.shell_height_km,
                                          info.base_radius_km, info.exponent, &from_nodes) ==
                  SIDEREON_STATUS_OK &&
              from_nodes != NULL,
          "IONEX from node samples");

    if (from_grid != NULL && from_nodes != NULL && epoch_count > 0 && lat_count > 0 &&
        lon_count > 0) {
        const double rx_lat = lats[lat_count / 2];
        const double rx_lon = lons[lon_count / 2];
        const int64_t epoch = (int64_t)llround(epochs[0]);
        double original_delay = 0.0;
        double grid_delay = 0.0;
        double nodes_delay = 0.0;
        check(sidereon_ionex_slant_delay(ionex, rx_lat, rx_lon, 120.0, 90.0, epoch,
                                          1575.42e6, &original_delay) == SIDEREON_STATUS_OK,
              "IONEX original delay");
        check(sidereon_ionex_slant_delay(from_grid, rx_lat, rx_lon, 120.0, 90.0, epoch,
                                          1575.42e6, &grid_delay) == SIDEREON_STATUS_OK,
              "IONEX grid delay");
        check(sidereon_ionex_slant_delay(from_nodes, rx_lat, rx_lon, 120.0, 90.0, epoch,
                                          1575.42e6, &nodes_delay) == SIDEREON_STATUS_OK,
              "IONEX node delay");
        check(f64_same(original_delay, grid_delay) && f64_same(original_delay, nodes_delay),
              "IONEX sample delay parity");
    }

cleanup:
    sidereon_ionex_free(from_nodes);
    sidereon_ionex_free(from_grid);
    sidereon_ionex_free(ionex);
    free(bytes);
    free(epochs);
    free(lats);
    free(lons);
    free(tec);
    free(rms);
    free(samples);
    if (failures == start) {
        printf("ionex_samples_smoke: OK (%zu TEC samples)\n", tec_count);
    }
}

static void test_sbas_decode(void) {
    int start = failures;
    const char *hex = "5366819010029EE7ED83018202819BBE1A08BF8008FFA00000004066C0";
    uint8_t body[32];
    size_t body_len = hex_to_bytes(hex, body, sizeof(body));
    check(body_len == 29, "SBAS vector decode");

    SidereonSbasBlock *block = NULL;
    check(sidereon_sbas_block_decode(body, body_len, SIDEREON_SBAS_WIRE_FORM_BODY226, &block) ==
                  SIDEREON_STATUS_OK &&
              block != NULL,
          "SBAS block decode");
    if (block == NULL) {
        return;
    }

    SidereonSbasMessageInfo info;
    check(sidereon_sbas_block_info(block, &info) == SIDEREON_STATUS_OK &&
              info.kind == SIDEREON_SBAS_MESSAGE_KIND_LONG_TERM_CORRECTIONS &&
              info.message_type == 25 && info.long_term_count == 2,
          "SBAS long-term info");

    const uint8_t expected_index[2] = {16, 31};
    const uint8_t expected_iode[2] = {50, 13};
    const int32_t expected_delta[2][3] = {{16, 20, -71}, {34, -16, 8}};
    const int32_t expected_rate[2][3] = {{6, 3, 4}, {0, 0, 0}};
    const int32_t expected_af0[2] = {-37, -3};
    const int32_t expected_af1[2] = {5, 2};
    for (size_t half_index = 0; half_index < 2; half_index++) {
        bool present = false;
        SidereonSbasLongTermHalfInfo half;
        check(sidereon_sbas_block_long_term_half_info(block, half_index, &present, &half) ==
                  SIDEREON_STATUS_OK &&
                  present && half.velocity_code && half.iodp == 3 && half.record_count == 1,
              "SBAS long-term half info");
        SidereonSbasLongTermRecord records[2];
        size_t written = 0;
        size_t required = 0;
        check(sidereon_sbas_block_long_term_records(block, half_index, records, 2, &written,
                                                    &required) == SIDEREON_STATUS_OK &&
                  written == 1 && required == 1,
              "SBAS long-term records");
        check(records[0].monitored_index == expected_index[half_index] &&
                  records[0].iode == expected_iode[half_index] &&
                  records[0].delta_x == expected_delta[half_index][0] &&
                  records[0].delta_y == expected_delta[half_index][1] &&
                  records[0].delta_z == expected_delta[half_index][2] &&
                  records[0].delta_x_rate == expected_rate[half_index][0] &&
                  records[0].delta_y_rate == expected_rate[half_index][1] &&
                  records[0].delta_z_rate == expected_rate[half_index][2] &&
                  records[0].delta_a_f0 == expected_af0[half_index] &&
                  records[0].delta_a_f1 == expected_af1[half_index] &&
                  records[0].has_time_of_day_s && records[0].time_of_day_s == 102,
              "SBAS long-term record vector");
    }

    uint8_t encoded[32];
    size_t encoded_written = 0;
    size_t encoded_required = 0;
    check(sidereon_sbas_block_encode(block, encoded, sizeof(encoded), &encoded_written,
                                     &encoded_required) == SIDEREON_STATUS_OK &&
              encoded_written == body_len && encoded_required == body_len &&
              memcmp(encoded, body, body_len) == 0,
          "SBAS encode roundtrip");
    sidereon_sbas_block_free(block);

    if (failures == start) {
        printf("sbas_decode_smoke: OK (2 long-term records)\n");
    }
}

static void test_araim(void) {
    int start = failures;
    SidereonAraimRow rows[10] = {
        {"G01", {0.0966, -0.0225, -0.9951}, SIDEREON_GNSS_SYSTEM_GPS, M_PI / 2.0},
        {"G02", {0.2612, -0.6750, 0.6900}, SIDEREON_GNSS_SYSTEM_GPS, M_PI / 2.0},
        {"G03", {0.7477, -0.0723, 0.6601}, SIDEREON_GNSS_SYSTEM_GPS, M_PI / 2.0},
        {"G04", {0.2269, 0.9398, -0.2553}, SIDEREON_GNSS_SYSTEM_GPS, M_PI / 2.0},
        {"G05", {0.2877, 0.5907, 0.7539}, SIDEREON_GNSS_SYSTEM_GPS, M_PI / 2.0},
        {"E01", {0.9455, 0.3236, 0.0354}, SIDEREON_GNSS_SYSTEM_GALILEO, M_PI / 2.0},
        {"E02", {0.5957, 0.6748, -0.4356}, SIDEREON_GNSS_SYSTEM_GALILEO, M_PI / 2.0},
        {"E03", {0.7075, -0.0938, 0.7004}, SIDEREON_GNSS_SYSTEM_GALILEO, M_PI / 2.0},
        {"E04", {0.7709, -0.5571, -0.3088}, SIDEREON_GNSS_SYSTEM_GALILEO, M_PI / 2.0},
        {"E05", {0.2780, -0.6622, -0.6958}, SIDEREON_GNSS_SYSTEM_GALILEO, M_PI / 2.0},
    };
    uint32_t clock_systems[2] = {SIDEREON_GNSS_SYSTEM_GPS, SIDEREON_GNSS_SYSTEM_GALILEO};
    SidereonAraimGeometry geometry;
    memset(&geometry, 0, sizeof(geometry));
    geometry.rows = rows;
    geometry.row_count = 10;
    geometry.receiver.lat_rad = 0.0;
    geometry.receiver.lon_rad = 0.0;
    geometry.receiver.height_m = 0.0;
    geometry.clock_systems = clock_systems;
    geometry.clock_system_count = 2;

    SidereonAraimSatelliteIsmModel model = {
        .sigma_ura_m = 0.75,
        .sigma_ure_m = 0.5,
        .has_effective_sigma_int_m = false,
        .effective_sigma_int_m = 0.0,
        .has_effective_sigma_acc_m = false,
        .effective_sigma_acc_m = 0.0,
        .b_nom_m = 0.5,
        .p_sat = 1.0e-5,
    };
    SidereonAraimConstellationIsm constellations[2] = {
        {SIDEREON_GNSS_SYSTEM_GPS, 1.0e-4, model},
        {SIDEREON_GNSS_SYSTEM_GALILEO, 1.0e-4, model},
    };
    SidereonAraimSatelliteIsm satellites[10] = {
        {"G01", 0.75, 0.5, true, sqrt(3.8865), true, sqrt(3.5740), 0.5, 1.0e-5},
        {"G02", 0.75, 0.5, true, sqrt(1.4377), true, sqrt(1.1252), 0.5, 1.0e-5},
        {"G03", 0.75, 0.5, true, sqrt(0.8604), true, sqrt(0.5479), 0.5, 1.0e-5},
        {"G04", 0.75, 0.5, true, sqrt(1.6383), true, sqrt(1.3258), 0.5, 1.0e-5},
        {"G05", 0.75, 0.5, true, sqrt(1.3229), true, sqrt(1.0104), 0.5, 1.0e-5},
        {"E01", 0.75, 0.5, true, sqrt(0.8434), true, sqrt(0.5309), 0.5, 1.0e-5},
        {"E02", 0.75, 0.5, true, sqrt(0.8963), true, sqrt(0.5838), 0.5, 1.0e-5},
        {"E03", 0.75, 0.5, true, sqrt(0.8669), true, sqrt(0.5544), 0.5, 1.0e-5},
        {"E04", 0.75, 0.5, true, sqrt(0.8573), true, sqrt(0.5448), 0.5, 1.0e-5},
        {"E05", 0.75, 0.5, true, sqrt(1.3616), true, sqrt(1.0491), 0.5, 1.0e-5},
    };
    SidereonAraimIsm ism;
    memset(&ism, 0, sizeof(ism));
    ism.constellations = constellations;
    ism.constellation_count = 2;
    ism.satellites = satellites;
    ism.satellite_count = 10;

    SidereonAraimIntegrityAllocation allocation;
    check(sidereon_araim_allocation_lpv_200(&allocation) == SIDEREON_STATUS_OK,
          "ARAIM allocation init");

    SidereonAraimResult *result = NULL;
    check(sidereon_araim(&geometry, &ism, &allocation, &result) == SIDEREON_STATUS_OK &&
              result != NULL,
          "ARAIM solve");
    if (result == NULL) {
        return;
    }

    SidereonAraimSummary summary;
    check(sidereon_araim_result_summary(result, &summary) == SIDEREON_STATUS_OK,
          "ARAIM summary");
    check(summary.availability && summary.fault_mode_count == 13,
          "ARAIM summary status");
    check_close(summary.vpl_m, 19.2, 0.05, "ARAIM VPL published reference");
    check_close(summary.hpl_m, 14.5, 0.05, "ARAIM HPL published reference");
    check_close(summary.emt_m, 7.8, 0.05, "ARAIM EMT published reference");
    check_close(summary.sigma_acc_v_m, 1.47, 0.02, "ARAIM vertical sigma published reference");

    SidereonAraimFaultMode modes[13];
    size_t written = 0;
    size_t required = 0;
    check(sidereon_araim_result_fault_modes(result, modes, 13, &written, &required) ==
                  SIDEREON_STATUS_OK &&
              written == 13 && required == 13,
          "ARAIM fault modes");
    check(modes[0].monitorable && modes[0].excluded_count == 0 &&
              !modes[0].has_excluded_constellation,
          "ARAIM fault-free mode");
    sidereon_araim_result_free(result);

    if (failures == start) {
        printf("araim_smoke: OK (WG-C Appendix D reference)\n");
    }
}

static void test_astro_angles(void) {
    int start = failures;
    double sep = 0.0;
    double pa = 0.0;
    double north = 0.0;
    check(sidereon_angular_separation_coords_deg(0.0, 0.0, 90.0, 0.0, &sep) ==
              SIDEREON_STATUS_OK,
          "angle separation coords");
    check(sidereon_position_angle_deg(0.0, 0.0, 90.0, 0.0, &pa) == SIDEREON_STATUS_OK,
          "position angle east");
    check(sidereon_position_angle_deg(0.0, 0.0, 0.0, 10.0, &north) == SIDEREON_STATUS_OK,
          "position angle north");
    check_close(sep, 90.0, 1.0e-12, "angle separation reference");
    check_close(pa, 90.0, 1.0e-12, "position angle east reference");
    check_close(north, 0.0, 1.0e-12, "position angle north reference");

    if (failures == start) {
        printf("astro_angles_smoke: OK (2 position-angle vectors)\n");
    }
}

int main(int argc, char **argv) {
    if (argc < 3) {
        fprintf(stderr, "usage: %s <ionex> <dted_root>\n", argv[0]);
        return 2;
    }

    test_clock_stability();
    test_terrain_batch(argv[2]);
    test_mmap_terrain_store(argv[2]);
    test_ionex_samples(argv[1]);
    test_sbas_decode();
    test_araim();
    test_astro_angles();

    if (failures != 0) {
        fprintf(stderr, "core012_smoke: FAIL (%d failures)\n", failures);
        return 1;
    }
    printf("core012_smoke: OK\n");
    return 0;
}
