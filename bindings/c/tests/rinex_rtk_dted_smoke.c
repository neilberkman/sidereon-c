/*
 * G07/G11 C ABI smoke test.
 *
 * argv: <sp3> <wtzr_obs> <wtzz_obs> <dted_tile_root>
 */
#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sidereon.h"

static int failures = 0;

/* Frozen from the public 120-epoch WTZR/WTZZ + matching SP3 fixtures and the
 * two public DTED mini tiles. Core-produced float64 fields are checked by
 * their IEEE-754 bit pattern; no binding-side numerical model is involved. */

static uint64_t f64_bits(double value) {
    uint64_t bits = 0;
    memcpy(&bits, &value, sizeof(bits));
    return bits;
}

static uint64_t fnv1a64(const uint8_t *bytes, size_t length) {
    uint64_t hash = UINT64_C(14695981039346656037);
    for (size_t i = 0; i < length; i++) {
        hash ^= bytes[i];
        hash *= UINT64_C(1099511628211);
    }
    return hash;
}

static void check(int ok, const char *what) {
    if (!ok) {
        char message[512];
        size_t message_len = sidereon_last_error_message(message, sizeof(message));
        if (message_len == 0) {
            message[0] = '\0';
        }
        /* The last error is sticky and may come from an earlier call, so it is
           reported as context rather than as the cause of this failure. */
        if (message[0] != '\0') {
            fprintf(stderr, "FAIL: %s (last ABI error, may predate this check: %s)\n",
                    what, message);
        } else {
            fprintf(stderr, "FAIL: %s\n", what);
        }
        failures++;
    }
}

static void check_bits(double actual, uint64_t expected, const char *what) {
    check(f64_bits(actual) == expected, what);
}

static uint8_t *read_file(const char *path, size_t *out_len) {
    FILE *file = fopen(path, "rb");
    if (!file) {
        fprintf(stderr, "FAIL: cannot open %s\n", path);
        failures++;
        return NULL;
    }
    if (fseek(file, 0, SEEK_END) != 0) {
        fclose(file);
        failures++;
        return NULL;
    }
    long size = ftell(file);
    if (size < 0) {
        fclose(file);
        failures++;
        return NULL;
    }
    rewind(file);
    size_t allocation = (size_t)size == 0 ? 1 : (size_t)size;
    uint8_t *bytes = (uint8_t *)malloc(allocation);
    if (!bytes) {
        fclose(file);
        failures++;
        return NULL;
    }
    size_t read_count = fread(bytes, 1, (size_t)size, file);
    fclose(file);
    if (read_count != (size_t)size) {
        free(bytes);
        failures++;
        return NULL;
    }
    *out_len = read_count;
    return bytes;
}

static SidereonSp3 *load_sp3(const char *path) {
    size_t length = 0;
    uint8_t *bytes = read_file(path, &length);
    if (!bytes) {
        return NULL;
    }
    SidereonSp3 *sp3 = NULL;
    check(sidereon_sp3_load(bytes, length, &sp3) == SIDEREON_STATUS_OK && sp3 != NULL,
          "sp3 load");
    free(bytes);
    return sp3;
}

static SidereonRinexObs *load_obs(const char *path) {
    size_t length = 0;
    uint8_t *bytes = read_file(path, &length);
    if (!bytes) {
        return NULL;
    }
    SidereonRinexObs *obs = NULL;
    check(sidereon_rinex_obs_parse(bytes, length, &obs) == SIDEREON_STATUS_OK && obs != NULL,
          "RINEX observation parse");
    free(bytes);
    return obs;
}

static void check_single_arc(const SidereonRtkRinexArc *arc) {
    size_t epoch_count = 0;
    size_t skipped_count = 0;
    check(sidereon_rtk_rinex_arc_epoch_count(arc, &epoch_count) == SIDEREON_STATUS_OK &&
              epoch_count == 120,
          "single RINEX arc epoch count");
    check(sidereon_rtk_rinex_arc_skipped_epoch_count(arc, &skipped_count) == SIDEREON_STATUS_OK &&
              skipped_count == 0,
          "single RINEX arc skipped count");
    printf("single RINEX arc: %zu epochs, %zu skipped\n", epoch_count, skipped_count);

    if (epoch_count != 120) {
        return;
    }

    SidereonRtkArcEpochOutMetadata metadata;
    memset(&metadata, 0, sizeof(metadata));
    check(sidereon_rtk_rinex_arc_epoch_metadata(arc, 0, &metadata) == SIDEREON_STATUS_OK,
          "single RINEX arc epoch metadata");
    check(metadata.base_count == 10 && metadata.rover_count == 10 &&
              metadata.satellite_position_count == 10 &&
              metadata.base_satellite_position_count == 10 &&
              metadata.rover_satellite_position_count == 10 && !metadata.has_velocity_mps &&
              metadata.has_prediction_time,
          "single RINEX arc frozen metadata");
    check_bits(metadata.prediction_time_s, UINT64_C(0x41c342fe60000000),
          "single RINEX arc prediction time");

    size_t written = 0;
    size_t required = 0;
    check(sidereon_rtk_rinex_arc_epoch_base_observations(
              arc, 0, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
              written == 0 && required == 10,
          "single RINEX base observation query");
    SidereonRtkArcObservationOut observations[10] = {0};
    SidereonStatus status = sidereon_rtk_rinex_arc_epoch_base_observations(
        arc, 0, observations, 10, &written, &required);
    check(status == SIDEREON_STATUS_OK && written == 10 && required == 10,
              "single RINEX base observation fill");
    if (status == SIDEREON_STATUS_OK && written == 10) {
        check(strcmp(observations[0].sat_id.bytes, "G05") == 0 &&
                  strcmp(observations[0].ambiguity_id.bytes, "G05") == 0 &&
                  observations[0].has_lli && observations[0].lli == 0,
              "single RINEX base observation identity");
        check_bits(observations[0].code_m, UINT64_C(0x4173c903ad604189),
                   "single RINEX base code");
        check_bits(observations[0].phase_m, UINT64_C(0x4173c90400aae46b),
                   "single RINEX base phase");
    }

    written = 0;
    required = 0;
    check(sidereon_rtk_rinex_arc_epoch_rover_observations(
              arc, 0, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
              written == 0 && required == 10,
          "single RINEX rover observation query");
    SidereonRtkArcObservationOut rover_observations[10] = {0};
    status = sidereon_rtk_rinex_arc_epoch_rover_observations(
        arc, 0, rover_observations, 10, &written, &required);
    check(status == SIDEREON_STATUS_OK && written == 10 && required == 10,
          "single RINEX rover observation fill");
    if (status == SIDEREON_STATUS_OK && written == 10) {
        check(strcmp(rover_observations[0].sat_id.bytes, "G05") == 0 &&
                  strcmp(rover_observations[0].ambiguity_id.bytes, "G05") == 0 &&
                  !rover_observations[0].has_lli && rover_observations[0].lli == 0,
              "single RINEX rover observation identity");
        check_bits(rover_observations[0].code_m, UINT64_C(0x4173d55e54189375),
                   "single RINEX rover code");
        check_bits(rover_observations[0].phase_m, UINT64_C(0x4173d55ebaaf6717),
                   "single RINEX rover phase");
    }

    written = 0;
    required = 0;
    check(sidereon_rtk_rinex_arc_epoch_base_satellite_positions(
              arc, 0, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
              written == 0 && required == 10,
          "single RINEX base position query");
    SidereonRtkArcPositionOut base_positions[10] = {0};
    status = sidereon_rtk_rinex_arc_epoch_base_satellite_positions(
        arc, 0, base_positions, 10, &written, &required);
    check(status == SIDEREON_STATUS_OK && written == 10 && required == 10,
          "single RINEX base position fill");
    if (status == SIDEREON_STATUS_OK && written == 10) {
        check(strcmp(base_positions[0].id.bytes, "G05") == 0,
              "single RINEX base position identity");
        check_bits(base_positions[0].pos[0], UINT64_C(0x41737544d632d2c6),
                   "single RINEX base transmit-time position x");
        check_bits(base_positions[0].pos[1], UINT64_C(0xc151590276ce05e1),
                   "single RINEX base transmit-time position y");
        check_bits(base_positions[0].pos[2], UINT64_C(0x416f3456ee5600c0),
                   "single RINEX base transmit-time position z");
    }

    written = 0;
    required = 0;
    check(sidereon_rtk_rinex_arc_epoch_rover_satellite_positions(
              arc, 0, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
              written == 0 && required == 10,
          "single RINEX rover position query");
    SidereonRtkArcPositionOut rover_positions[10] = {0};
    status = sidereon_rtk_rinex_arc_epoch_rover_satellite_positions(
        arc, 0, rover_positions, 10, &written, &required);
    check(status == SIDEREON_STATUS_OK && written == 10 && required == 10,
          "single RINEX rover position fill");
    if (status == SIDEREON_STATUS_OK && written == 10) {
        check(strcmp(rover_positions[0].id.bytes, "G05") == 0,
              "single RINEX rover position identity");
        check_bits(rover_positions[0].pos[0], UINT64_C(0x41737544d118bf58),
                   "single RINEX rover transmit-time position x");
        check_bits(rover_positions[0].pos[1], UINT64_C(0xc151590280f42387),
                   "single RINEX rover transmit-time position y");
        check_bits(rover_positions[0].pos[2], UINT64_C(0x416f3456f972c24e),
                   "single RINEX rover transmit-time position z");
    }

    written = 0;
    required = 0;
    check(sidereon_rtk_rinex_arc_epoch_satellite_positions(
              arc, 0, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
              written == 0 && required == 10,
          "single RINEX position query");
    SidereonRtkArcPositionOut positions[10] = {0};
    status = sidereon_rtk_rinex_arc_epoch_satellite_positions(
        arc, 0, positions, 10, &written, &required);
    check(status == SIDEREON_STATUS_OK && written == 10 && required == 10,
          "single RINEX position fill");
    if (status == SIDEREON_STATUS_OK && written == 10) {
        check(strcmp(positions[0].id.bytes, "G05") == 0,
              "single RINEX position identity");
        check_bits(positions[0].pos[0], UINT64_C(0x4173754cfed0e560),
                   "single RINEX position x");
        check_bits(positions[0].pos[1], UINT64_C(0xc15158f23c083127),
                   "single RINEX position y");
        check_bits(positions[0].pos[2], UINT64_C(0x416f344529168729),
                   "single RINEX position z");
    }

    written = 0;
    required = 0;
    check(sidereon_rtk_rinex_arc_wavelengths_m(
              arc, NULL, 0, &written, &required) == SIDEREON_STATUS_OK && written == 0 &&
              required == 11,
          "single RINEX wavelength query");
    SidereonRtkMapValue wavelengths[11] = {0};
    status = sidereon_rtk_rinex_arc_wavelengths_m(arc, wavelengths, 11, &written, &required);
    check(status == SIDEREON_STATUS_OK && written == 11 && required == 11,
          "single RINEX wavelength fill");
    if (status == SIDEREON_STATUS_OK && written == 11) {
        check(strcmp(wavelengths[0].id.bytes, "G05") == 0,
              "single RINEX wavelength identity");
        check_bits(wavelengths[0].value, UINT64_C(0x3fc85b8b06a70079),
                   "single RINEX wavelength value");
    }

    written = 0;
    required = 0;
    check(sidereon_rtk_rinex_arc_offsets_m(
              arc, NULL, 0, &written, &required) == SIDEREON_STATUS_OK && written == 0 &&
              required == 11,
          "single RINEX offset query");
    SidereonRtkMapValue offsets[11] = {0};
    status = sidereon_rtk_rinex_arc_offsets_m(arc, offsets, 11, &written, &required);
    check(status == SIDEREON_STATUS_OK && written == 11 && required == 11,
          "single RINEX offset fill");
    if (status == SIDEREON_STATUS_OK && written == 11) {
        check(strcmp(offsets[0].id.bytes, "G05") == 0,
              "single RINEX offset identity");
        check_bits(offsets[0].value, UINT64_C(0x0000000000000000),
                   "single RINEX offset value");
    }
}

static void check_dual_arc(const SidereonRtkRinexDualFrequencyArc *arc) {
    size_t epoch_count = 0;
    size_t skipped_count = 0;
    check(sidereon_rtk_rinex_dual_frequency_arc_epoch_count(arc, &epoch_count) ==
              SIDEREON_STATUS_OK &&
              epoch_count == 120,
          "dual RINEX arc epoch count");
    check(sidereon_rtk_rinex_dual_frequency_arc_skipped_epoch_count(arc, &skipped_count) ==
              SIDEREON_STATUS_OK && skipped_count == 0,
          "dual RINEX arc skipped count");
    printf("dual RINEX arc: %zu epochs, %zu skipped\n", epoch_count, skipped_count);

    if (epoch_count != 120) {
        return;
    }

    SidereonRtkRinexDualFrequencyArcEpochOutMetadata metadata;
    memset(&metadata, 0, sizeof(metadata));
    check(sidereon_rtk_rinex_dual_frequency_arc_epoch_metadata(arc, 0, &metadata) ==
              SIDEREON_STATUS_OK,
          "dual RINEX arc epoch metadata");
    check(metadata.observation_count == 10 && metadata.satellite_position_count == 10 &&
              metadata.base_satellite_position_count == 10 &&
              metadata.rover_satellite_position_count == 10 && !metadata.has_velocity_mps &&
              metadata.has_gap_time_s && metadata.has_prediction_time,
          "dual RINEX arc frozen metadata flags and counts");
    check_bits(metadata.jd_whole, UINT64_C(0x4142c2c8c0000000),
               "dual RINEX Julian day whole");
    check_bits(metadata.jd_fraction, UINT64_C(0x0000000000000000),
               "dual RINEX Julian day fraction");
    check_bits(metadata.gap_time_s, UINT64_C(0x41c342fe60000000),
               "dual RINEX gap time");
    check_bits(metadata.prediction_time_s, UINT64_C(0x41c342fe60000000),
               "dual RINEX prediction time");

    size_t written = 0;
    size_t required = 0;
    static const char expected_sort_key[] = "2020-06-25T00:00:0.000000000";
    check(sidereon_rtk_rinex_dual_frequency_arc_epoch_sort_key(
              arc, 0, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
              written == 0 && required == sizeof(expected_sort_key) - 1,
          "dual RINEX sort-key query");
    uint8_t sort_key[sizeof(expected_sort_key) - 1] = {0};
    SidereonStatus status = sidereon_rtk_rinex_dual_frequency_arc_epoch_sort_key(
        arc, 0, sort_key, sizeof(sort_key), &written, &required);
    check(status == SIDEREON_STATUS_OK && written == sizeof(sort_key) &&
              required == sizeof(sort_key) &&
              memcmp(sort_key, expected_sort_key, sizeof(sort_key)) == 0,
          "dual RINEX frozen sort key");

    written = 0;
    required = 0;
    check(sidereon_rtk_rinex_dual_frequency_arc_epoch_observations(
              arc, 0, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
              written == 0 && required == 10,
          "dual RINEX observation query");
    SidereonRtkDualFrequencySatelliteObservationOut observations[10] = {0};
    status = sidereon_rtk_rinex_dual_frequency_arc_epoch_observations(
        arc, 0, observations, 10, &written, &required);
    check(status == SIDEREON_STATUS_OK && written == 10 && required == 10,
          "dual RINEX observation fill");
    if (status == SIDEREON_STATUS_OK && written == 10) {
        check(strcmp(observations[0].sat_id.bytes, "G05") == 0 &&
                  strcmp(observations[0].base.ambiguity_id.bytes, "G05") == 0 &&
                  strcmp(observations[0].rover.ambiguity_id.bytes, "G05") == 0 &&
                  observations[0].base.has_lli1 && observations[0].base.lli1 == 0 &&
                  observations[0].base.has_lli2 && observations[0].base.lli2 == 0,
              "dual RINEX observation identity");
        check_bits(observations[0].base.p1_m, UINT64_C(0x4173c903ad604189),
                   "dual RINEX base P1");
        check_bits(observations[0].base.p2_m, UINT64_C(0x4173c903949374bc),
                   "dual RINEX base P2");
        check_bits(observations[0].base.phi1_cycles, UINT64_C(0x4199fe358e52f1aa),
                   "dual RINEX base phase 1");
        check_bits(observations[0].base.phi2_cycles, UINT64_C(0x419441197de76c8b),
                   "dual RINEX base phase 2");
        check_bits(observations[0].base.f1_hz, UINT64_C(0x41d779c018000000),
                   "dual RINEX base frequency 1");
        check_bits(observations[0].base.f2_hz, UINT64_C(0x41d24aec20000000),
                   "dual RINEX base frequency 2");
        check_bits(observations[0].rover.p1_m, UINT64_C(0x4173d55e54189375),
                   "dual RINEX rover P1");
        check_bits(observations[0].rover.p2_m, UINT64_C(0x4173d55e285a1cac),
                   "dual RINEX rover P2");
        check_bits(observations[0].rover.phi1_cycles, UINT64_C(0x419a0e709db95810),
                   "dual RINEX rover phase 1");
        check_bits(observations[0].rover.phi2_cycles, UINT64_C(0x41944dbeea49ba5e),
                   "dual RINEX rover phase 2");
    }

    written = 0;
    required = 0;
    check(sidereon_rtk_rinex_dual_frequency_arc_epoch_base_satellite_positions(
              arc, 0, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
              written == 0 && required == 10,
          "dual RINEX base position query");
    written = 0;
    required = 0;
    check(sidereon_rtk_rinex_dual_frequency_arc_epoch_rover_satellite_positions(
              arc, 0, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
              written == 0 && required == 10,
          "dual RINEX rover position query");
    SidereonRtkArcPositionOut positions[10] = {0};
    status = sidereon_rtk_rinex_dual_frequency_arc_epoch_satellite_positions(
        arc, 0, positions, 10, &written, &required);
    check(status == SIDEREON_STATUS_OK && written == 10 && required == 10,
          "dual RINEX position fill");
    if (status == SIDEREON_STATUS_OK && written == 10) {
        check(strcmp(positions[0].id.bytes, "G05") == 0,
              "dual RINEX position identity");
        check_bits(positions[0].pos[0], UINT64_C(0x4173754cfed0e560),
                   "dual RINEX position x");
        check_bits(positions[0].pos[1], UINT64_C(0xc15158f23c083127),
                   "dual RINEX position y");
        check_bits(positions[0].pos[2], UINT64_C(0x416f344529168729),
                   "dual RINEX position z");
    }
}

static void check_dted_store(const char *tile_root) {
    char tile_w107[1024];
    char tile_w106[1024];
    int path_result = snprintf(tile_w107, sizeof(tile_w107), "%s/n36_w107_1arc_v3.dt2", tile_root);
    check(path_result > 0 && (size_t)path_result < sizeof(tile_w107), "DTED w107 path");
    path_result = snprintf(tile_w106, sizeof(tile_w106), "%s/n36_w106_1arc_v3.dt2", tile_root);
    check(path_result > 0 && (size_t)path_result < sizeof(tile_w106), "DTED w106 path");

    SidereonDtedTileListEntry entries[2] = {
        {{36, -107}, tile_w107},
        {{36, -106}, tile_w106},
    };
    size_t written = 0;
    size_t required = 0;
    check(sidereon_dted_tile_list_to_mmap_store(
              entries, 2, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
              written == 0 && required == 8242,
          "DTED tile-list query");
    uint8_t *list_bytes = (uint8_t *)malloc(required);
    check(list_bytes != NULL, "DTED tile-list allocation");
    if (!list_bytes) {
        return;
    }
    check(sidereon_dted_tile_list_to_mmap_store(
              entries, 2, list_bytes, required, &written, &required) == SIDEREON_STATUS_OK &&
              written == 8242 && required == 8242,
          "DTED tile-list fill");
    check(fnv1a64(list_bytes, required) == UINT64_C(0xff514a676a94d479),
          "DTED tile-list frozen FNV-1a-64");

    size_t repeat_written = 0;
    size_t repeat_required = 0;
    check(sidereon_dted_tile_list_to_mmap_store(
              entries, 2, NULL, 0, &repeat_written, &repeat_required) == SIDEREON_STATUS_OK &&
              repeat_written == 0 && repeat_required == 8242,
          "DTED deterministic query");
    uint8_t *repeat_bytes = (uint8_t *)malloc(repeat_required);
    check(repeat_bytes != NULL, "DTED repeat allocation");
    if (repeat_bytes) {
        check(sidereon_dted_tile_list_to_mmap_store(
                  entries, 2, repeat_bytes, repeat_required, &repeat_written, &repeat_required) ==
                  SIDEREON_STATUS_OK &&
                  repeat_written == required && memcmp(list_bytes, repeat_bytes, required) == 0,
              "DTED deterministic bytes");
        free(repeat_bytes);
    }

    /* POSIX leaves TMPDIR unset on many systems; /tmp is the standard fallback. */
    const char *tmpdir = getenv("TMPDIR");
    if (!tmpdir || !*tmpdir) {
        tmpdir = "/tmp";
    }
    char output_path[1024];
    path_result = snprintf(output_path, sizeof(output_path), "%s/sidereon-g11-smoke.store", tmpdir);
    check(path_result > 0 && (size_t)path_result < sizeof(output_path), "DTED output path");
    char missing_parent_path[1024];
    path_result = snprintf(missing_parent_path, sizeof(missing_parent_path),
                           "%s/sidereon-g11-smoke-missing-parent", tmpdir);
    check(path_result > 0 && (size_t)path_result < sizeof(missing_parent_path),
          "DTED missing-parent path");
    char missing_output_path[1024];
    path_result = snprintf(missing_output_path, sizeof(missing_output_path),
                           "%s/sidereon-g11-smoke-missing-parent/store", tmpdir);
    check(path_result > 0 && (size_t)path_result < sizeof(missing_output_path),
          "DTED missing-parent output path");
    remove(output_path);
    remove(missing_output_path);
    remove(missing_parent_path);
    check(sidereon_write_dted_tile_list_to_mmap_store(entries, 2, output_path) ==
              SIDEREON_STATUS_OK,
          "DTED tile-list writer");
    check(sidereon_write_dted_tile_list_to_mmap_store(entries, 2, output_path) ==
              SIDEREON_STATUS_OK,
          "DTED tile-list deterministic overwrite");
    size_t written_file = 0;
    uint8_t *file_bytes = read_file(output_path, &written_file);
    check(file_bytes != NULL && written_file == required &&
              (!file_bytes || memcmp(list_bytes, file_bytes, required) == 0),
          "DTED writer bytes equal conversion");
    free(file_bytes);
    remove(output_path);

    check(sidereon_write_dted_tile_list_to_mmap_store(entries, 2, missing_output_path) ==
              SIDEREON_STATUS_INVALID_ARGUMENT,
          "DTED writer I/O error status");
    SidereonTerrainStoreError io_error;
    memset(&io_error, 0, sizeof(io_error));
    check(sidereon_last_terrain_store_error(&io_error) == SIDEREON_STATUS_OK &&
              io_error.kind == SIDEREON_TERRAIN_STORE_ERROR_KIND_IO,
          "DTED writer I/O typed error");
    remove(missing_output_path);
    remove(missing_parent_path);

    size_t tree_written = 0;
    size_t tree_required = 0;
    check(sidereon_dted_tree_to_mmap_store(
              tile_root, NULL, 0, &tree_written, &tree_required) == SIDEREON_STATUS_OK &&
              tree_written == 0 && tree_required == 8242,
          "DTED tree/list size equivalence");
    uint8_t *tree_bytes = (uint8_t *)malloc(tree_required);
    check(tree_bytes != NULL, "DTED tree allocation");
    if (tree_bytes) {
        check(sidereon_dted_tree_to_mmap_store(
                  tile_root, tree_bytes, tree_required, &tree_written, &tree_required) ==
                  SIDEREON_STATUS_OK &&
                  tree_written == 8242 && tree_required == 8242 &&
                      memcmp(list_bytes, tree_bytes, 8242) == 0,
              "DTED tree/list bytes equal");
        free(tree_bytes);
    }

    SidereonDtedTileListEntry mismatch = entries[0];
    mismatch.tile_id.lat_index = 35;
    written = 0;
    required = 0;
    check(sidereon_dted_tile_list_to_mmap_store(
              &mismatch, 1, NULL, 0, &written, &required) == SIDEREON_STATUS_INVALID_ARGUMENT,
          "DTED tile-id mismatch status");
    SidereonTerrainStoreError terrain_error;
    memset(&terrain_error, 0, sizeof(terrain_error));
    check(sidereon_last_terrain_store_error(&terrain_error) == SIDEREON_STATUS_OK &&
              terrain_error.kind == SIDEREON_TERRAIN_STORE_ERROR_KIND_TILE_ID_MISMATCH,
          "DTED tile-id mismatch typed error");
    check(sidereon_dted_tile_list_to_mmap_store(
              NULL, 1, NULL, 0, &written, &required) == SIDEREON_STATUS_NULL_POINTER,
          "DTED null entry pointer status");

    free(list_bytes);
}

int main(int argc, char **argv) {
    if (argc != 5) {
        fprintf(stderr, "usage: %s <sp3> <wtzr_obs> <wtzz_obs> <dted_tile_root>\n", argv[0]);
        return 2;
    }

    SidereonSp3 *sp3 = load_sp3(argv[1]);
    SidereonRinexObs *base_obs = load_obs(argv[2]);
    SidereonRinexObs *rover_obs = load_obs(argv[3]);
    if (sp3 && base_obs && rover_obs) {
        SidereonRtkRinexArcOptions single_options;
        check(sidereon_rtk_rinex_arc_options_init(&single_options) == SIDEREON_STATUS_OK,
              "single RINEX option init");
        single_options.has_max_epochs = true;
        single_options.max_epochs = 120;
        single_options.include_prediction_time = true;
        SidereonRtkRinexArc *single_arc = NULL;
        check(sidereon_build_rinex_rtk_arc(
                  sp3, base_obs, rover_obs, &single_options, &single_arc) == SIDEREON_STATUS_OK &&
                  single_arc != NULL,
              "single RINEX arc build");
        if (single_arc) {
            check_single_arc(single_arc);
            sidereon_rtk_rinex_arc_free(single_arc);
        }

        SidereonRtkRinexDualArcOptions dual_options;
        check(sidereon_rtk_rinex_dual_arc_options_init(&dual_options) == SIDEREON_STATUS_OK,
              "dual RINEX option init");
        dual_options.has_max_epochs = true;
        dual_options.max_epochs = 120;
        dual_options.include_prediction_time = true;
        SidereonRtkRinexDualFrequencyArc *dual_arc = NULL;
        check(sidereon_build_dual_frequency_rinex_rtk_arc(
                  sp3, base_obs, rover_obs, &dual_options, &dual_arc) == SIDEREON_STATUS_OK &&
                  dual_arc != NULL,
              "dual RINEX arc build");
        if (dual_arc) {
            check_dual_arc(dual_arc);
            sidereon_rtk_rinex_dual_frequency_arc_free(dual_arc);
        }

        single_options.min_common_satellites = 0;
        single_arc = NULL;
        check(sidereon_build_rinex_rtk_arc(
                  sp3, base_obs, rover_obs, &single_options, &single_arc) ==
                  SIDEREON_STATUS_INVALID_ARGUMENT &&
                  single_arc == NULL,
              "single RINEX invalid option");
        check(sidereon_build_rinex_rtk_arc(
                  NULL, base_obs, rover_obs, &single_options, &single_arc) ==
                  SIDEREON_STATUS_NULL_POINTER,
              "single RINEX null input");
        sidereon_rtk_rinex_arc_free(NULL);
        sidereon_rtk_rinex_dual_frequency_arc_free(NULL);
    }

    if (sp3) {
        sidereon_sp3_free(sp3);
    }
    if (base_obs) {
        sidereon_rinex_obs_free(base_obs);
    }
    if (rover_obs) {
        sidereon_rinex_obs_free(rover_obs);
    }

    check_dted_store(argv[4]);
    if (failures != 0) {
        fprintf(stderr, "rinex_rtk_dted_smoke: %d failure(s)\n", failures);
        return 1;
    }
    return 0;
}
