#include "sidereon.h"

#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int failures = 0;

static void check(bool condition, const char *message) {
    if (!condition) {
        fprintf(stderr, "rinex_obs_complete_smoke: %s\n", message);
        failures++;
    }
}

static char *copy_change_text(const SidereonRinexObsDowngradeResult *result,
                              size_t change_index,
                              size_t depth,
                              uint32_t field) {
    size_t written = 0, required = 0;
    check(sidereon_rinex_obs_downgrade_result_get_change_text(
              result, change_index, depth, field, NULL, 0, &written, &required) ==
              SIDEREON_STATUS_OK,
          "change text query");
    char *text = calloc(required + 1, 1);
    check(text != NULL, "change text allocation");
    if (text == NULL) {
        return NULL;
    }
    check(sidereon_rinex_obs_downgrade_result_get_change_text(
              result, change_index, depth, field, (uint8_t *)text, required,
              &written, &required) == SIDEREON_STATUS_OK,
          "change text copy");
    check(written == required, "change text exact copy");
    return text;
}

static void exercise_change_payload(const SidereonRinexObsDowngradeResult *result,
                                    size_t index,
                                    size_t depth) {
    SidereonRinexObsDowngradeChange change;
    check(sidereon_rinex_obs_downgrade_result_get_change(result, index, depth,
                                                          &change) ==
              SIDEREON_STATUS_OK,
          "change descriptor");
    for (uint32_t field = 0; field <= 3; field++) {
        char *text = copy_change_text(result, index, depth, field);
        free(text);
    }
    const size_t counts[] = {change.codes_count, change.records_count,
                             change.from_records_count,
                             change.to_records_count};
    for (uint32_t list = 0; list < 4; list++) {
        for (size_t item = 0; item < counts[list]; item++) {
            size_t written = 0, required = 0;
            check(sidereon_rinex_obs_downgrade_result_get_change_list_item(
                      result, index, depth, list, item, NULL, 0, &written,
                      &required) == SIDEREON_STATUS_OK,
                  "change list query");
            uint8_t *text = calloc(required + 1, 1);
            check(text != NULL, "change list allocation");
            if (text != NULL) {
                check(sidereon_rinex_obs_downgrade_result_get_change_list_item(
                          result, index, depth, list, item, text, required,
                          &written, &required) == SIDEREON_STATUS_OK,
                      "change list copy");
                check(written == required, "change list exact copy");
                free(text);
            }
        }
    }
    if (change.has_nested_change) {
        exercise_change_payload(result, index, depth + 1);
    }
}

static SidereonRinexObs *parse_text(const char *text) {
    SidereonRinexObs *obs = NULL;
    check(sidereon_rinex_obs_parse((const uint8_t *)text, strlen(text), &obs) ==
              SIDEREON_STATUS_OK,
          "parse observation text");
    check(obs != NULL, "parse returns handle");
    return obs;
}

static uint8_t *copy_rinex_text(const SidereonRinexObs *obs, size_t *out_len) {
    size_t written = 0, required = 0;
    check(sidereon_rinex_obs_to_rinex_text(obs, NULL, 0, &written, &required) ==
              SIDEREON_STATUS_OK,
          "RINEX text query");
    uint8_t *text = calloc(required + 1, 1);
    check(text != NULL, "RINEX text allocation");
    if (text == NULL) {
        *out_len = 0;
        return NULL;
    }
    check(sidereon_rinex_obs_to_rinex_text(obs, text, required, &written,
                                           &required) == SIDEREON_STATUS_OK,
          "RINEX text copy");
    check(written == required, "RINEX text exact copy");
    *out_len = written;
    return text;
}

static char *read_fixture(const char *path) {
    FILE *file = fopen(path, "rb");
    if (file == NULL) {
        return NULL;
    }
    if (fseek(file, 0, SEEK_END) != 0) {
        fclose(file);
        return NULL;
    }
    long length = ftell(file);
    if (length < 0 || fseek(file, 0, SEEK_SET) != 0) {
        fclose(file);
        return NULL;
    }
    char *text = malloc((size_t)length + 1);
    if (text == NULL) {
        fclose(file);
        return NULL;
    }
    size_t read = fread(text, 1, (size_t)length, file);
    fclose(file);
    if (read != (size_t)length) {
        free(text);
        return NULL;
    }
    text[read] = '\0';
    return text;
}

static char *header_details_json(const SidereonRinexObs *obs) {
    size_t written = 99, required = 99;
    check(sidereon_rinex_obs_header_details_json(obs, NULL, 0, &written,
                                                  &required) ==
              SIDEREON_STATUS_OK,
          "header details size query");
    check(written == 0 && required > 2, "header details query counts");
    char *json = calloc(required + 1, 1);
    check(json != NULL, "header details allocation");
    if (json == NULL) {
        return NULL;
    }
    written = 0;
    check(sidereon_rinex_obs_header_details_json(
              obs, (uint8_t *)json, required, &written, &required) ==
              SIDEREON_STATUS_OK,
          "header details copy");
    check(written == required, "header details exact copy");
    return json;
}

static void check_header_detail_fields(const char *path) {
    char *text = read_fixture(path);
    check(text != NULL, "header detail fixture read");
    if (text == NULL) {
        return;
    }
    SidereonRinexObs *obs = parse_text(text);
    free(text);
    if (obs == NULL) {
        return;
    }
    char *json = header_details_json(obs);
    if (json != NULL) {
        const char *keys[] = {
            "\"version\"", "\"approx_position_m\"", "\"antenna_delta_hen_m\"",
            "\"obs_codes\"", "\"declared_obs_codes\"", "\"rinex2_types\"",
            "\"rinex2_system\"", "\"program_run_by_date\"", "\"comments\"",
            "\"marker_number\"", "\"marker_type\"", "\"observer\"", "\"agency\"",
            "\"receiver\"", "\"antenna\"", "\"interval_s\"", "\"time_of_first_obs\"",
            "\"time_of_last_obs\"", "\"n_satellites\"", "\"prn_obs_counts\"",
            "\"phase_shifts\"", "\"scale_factors\"", "\"glonass_slots\"",
            "\"glonass_cod_phs_bis\"", "\"signal_strength_unit\"", "\"leap_seconds\"",
            "\"marker_name\"", "\"unretained_header_labels\""
        };
        for (size_t i = 0; i < sizeof(keys) / sizeof(keys[0]); i++) {
            check(strstr(json, keys[i]) != NULL, "complete header field key");
        }
        if (strstr(path, "ESBC") != NULL) {
            check(strstr(json, "\"marker_type\":\"GEODETIC\"") != NULL,
                  "marker type detail");
            check(strstr(json, "\"signal_strength_unit\":\"DBHZ\"") != NULL,
                  "signal-strength-unit detail");
            check(strstr(json, "\"n_satellites\":0") != NULL,
                  "present zero satellite count");
            check(strstr(json, "\"interval_s\":30.0") != NULL,
                  "interval detail");
            check(strstr(json, "\"marker_number\":\"10118M001\"") != NULL,
                  "marker number detail");
        }
        if (strstr(path, "a7") != NULL || strstr(path, "A7") != NULL) {
            check(strstr(json, "\"rinex2_types\":[\"P1\",\"L1\",\"L2\",\"P2\",\"L5\"]") != NULL,
                  "RINEX 2 type order");
        }
        free(json);
    }
    uint8_t short_buffer[1] = {0};
    size_t written = 77, required = 88;
    check(sidereon_rinex_obs_header_details_json(obs, short_buffer,
                                                  sizeof(short_buffer), &written,
                                                  &required) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              written == 0 && required > sizeof(short_buffer) && short_buffer[0] == 0,
          "header details short buffer reports required size without partial output");
    uint8_t overflow_probe = 0;
    written = 77;
    required = 88;
    check(sidereon_rinex_obs_header_details_json(
              obs, &overflow_probe, SIZE_MAX, &written, &required) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              written == 0 && required > 2 && overflow_probe == 0,
          "header details rejects overflowing capacity before writing");
    sidereon_rinex_obs_free(obs);
}

static void test_header_details(const char *esbc_path, const char *a7_path,
                                const char *wtzz_path) {
    check_header_detail_fields(esbc_path);
    check_header_detail_fields(a7_path);
    check_header_detail_fields(wtzz_path);
    size_t written = 77, required = 88;
    check(sidereon_rinex_obs_header_details_json(NULL, NULL, 0, &written,
                                                  &required) ==
              SIDEREON_STATUS_NULL_POINTER &&
              written == 0 && required == 0,
          "header details null source initializes output counts");
    check(sidereon_rinex_obs_header_details_json(NULL, NULL, 0, NULL, NULL) ==
              SIDEREON_STATUS_NULL_POINTER,
          "header details null counts are rejected");
}

static void test_header_routes_and_skips(void) {
    char text[1024];
    int n = snprintf(
        text, sizeof(text),
        "     3.05           OBSERVATION DATA    R                   RINEX VERSION / TYPE\n"
        "%-60s%-20s\n"
        "%-60s%-20s\n"
        "%-60s%-20s\n",
        "R    1 L1C", "SYS / # / OBS TYPES",
        "R L1C  0.25000  03 R01 R28 R00", "SYS / PHASE SHIFT", "",
        "END OF HEADER");
    check(n > 0 && (size_t)n < sizeof(text), "skip fixture formatting");
    SidereonRinexObs *obs = parse_text(text);
    if (obs == NULL) {
        return;
    }
    size_t skipped = 0;
    check(sidereon_rinex_obs_skipped_records(obs, &skipped) ==
              SIDEREON_STATUS_OK,
          "skipped-record route");
    check(skipped == 1, "R00 counted as skipped while R28 remains representable");

    size_t written = 99, required = 99;
    check(sidereon_rinex_obs_header_timeline(obs, NULL, 0, &written,
                                             &required) == SIDEREON_STATUS_OK,
          "timeline query");
    check(written == 0 && required == 1, "file-only timeline has one segment");
    SidereonRinexObsHeaderSegment segment;
    check(sidereon_rinex_obs_header_timeline(obs, &segment, 1, &written,
                                             &required) == SIDEREON_STATUS_OK,
          "timeline copy");
    check(written == 1 && required == 1 && segment.first_epoch_index == 0,
          "timeline starts with detached file header at zero");
    check(segment.header.version == 3.05, "timeline copied header version");

    SidereonRinexObsHeader header;
    check(sidereon_rinex_obs_header_at(obs, 0, &header) ==
              SIDEREON_STATUS_INVALID_ARGUMENT,
          "header-at rejects index equal to zero epoch count");
    sidereon_rinex_obs_free(obs);
}

static void test_downgrade(void) {
    static const char text[] =
        "     3.02           OBSERVATION DATA    C                   RINEX VERSION / TYPE\n"
        "C    2 C1I L1I                                              SYS / # / OBS TYPES\n"
        "                                                            END OF HEADER\n"
        "> 2020 06 24 00 00  0.0000000  0  1\n"
        "C01  22000000.000 7        10.00015\n"
        "> 2020 06 24 00 00 30.0000000  6  1\n"
        "C01                       100.000  \n";
    SidereonRinexObs *source = parse_text(text);
    if (source == NULL) {
        return;
    }
    size_t first_len = 0, second_len = 0;
    uint8_t *first_text = copy_rinex_text(source, &first_len);
    SidereonRinexObs *reparsed =
        first_text == NULL ? NULL : parse_text((const char *)first_text);
    uint8_t *second_text =
        reparsed == NULL ? NULL : copy_rinex_text(reparsed, &second_len);
    check(first_text != NULL && second_text != NULL && first_len == second_len &&
              memcmp(first_text, second_text, first_len) == 0,
          "parse-write equality is byte-stable through the public C routes");
    free(first_text);
    free(second_text);
    sidereon_rinex_obs_free(reparsed);

    size_t epochs = 0;
    check(sidereon_rinex_obs_epoch_count(source, &epochs) == SIDEREON_STATUS_OK &&
              epochs == 2,
          "downgrade fixture epoch count");
    SidereonRinexObsHeader at;
    check(sidereon_rinex_obs_header_at(source, 0, &at) == SIDEREON_STATUS_OK &&
              at.version == 3.02,
          "header-at first epoch");
    check(sidereon_rinex_obs_header_at(source, epochs, &at) ==
              SIDEREON_STATUS_INVALID_ARGUMENT,
          "header-at rejects epoch count");

    SidereonRinexObsDowngradeResult *result = NULL;
    check(sidereon_rinex_obs_downgrade_to_rinex2(source, 2.11, &result) ==
              SIDEREON_STATUS_OK &&
              result != NULL,
          "downgrade returns owned result");
    SidereonRinexObsDowngradeOutcome outcome;
    check(sidereon_rinex_obs_downgrade_result_get_outcome(result, &outcome) ==
              SIDEREON_STATUS_OK &&
              outcome.is_ok && outcome.status == SIDEREON_STATUS_OK,
          "downgrade success outcome");
    check(outcome.change_count == 2, "downgrade reports exactly two renames");
    const char *expected_from[] = {"C1I", "L1I"};
    const char *expected_to[] = {"C2I", "L2I"};
    for (size_t i = 0; i < outcome.change_count; i++) {
        SidereonRinexObsDowngradeChange change;
        check(sidereon_rinex_obs_downgrade_result_get_change(result, i, 0,
                                                              &change) ==
                  SIDEREON_STATUS_OK,
              "ordered downgrade change");
        check(change.kind ==
                  SIDEREON_RINEX_OBS_DOWNGRADE_CHANGE_KIND_CODE_RENAMED,
              "ordered change is code rename");
        char *from = copy_change_text(
            result, i, 0, SIDEREON_RINEX_OBS_DOWNGRADE_TEXT_FIELD_FROM);
        char *to = copy_change_text(
            result, i, 0, SIDEREON_RINEX_OBS_DOWNGRADE_TEXT_FIELD_TO);
        check(from != NULL && strcmp(from, expected_from[i]) == 0,
              "ordered rename source");
        check(to != NULL && strcmp(to, expected_to[i]) == 0,
              "ordered rename target");
        free(from);
        free(to);
        exercise_change_payload(result, i, 0);
    }

    SidereonRinexObs *downgraded = NULL;
    check(sidereon_rinex_obs_downgrade_result_take_obs(result, &downgraded) ==
              SIDEREON_STATUS_OK &&
              downgraded != NULL,
          "take fresh downgraded product");
    SidereonRinexObs *second = (SidereonRinexObs *)(uintptr_t)1;
    check(sidereon_rinex_obs_downgrade_result_take_obs(result, &second) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              second == NULL,
          "take is one-shot and initializes output");
    sidereon_rinex_obs_downgrade_result_free(result);
    sidereon_rinex_obs_free(source);

    SidereonRinexObsHeader downgraded_header;
    check(sidereon_rinex_obs_header(downgraded, &downgraded_header) ==
              SIDEREON_STATUS_OK &&
              downgraded_header.version == 2.11,
          "taken product outlives source and result");
    sidereon_rinex_obs_free(downgraded);
}

static void test_downgrade_refusals(void) {
    static const char text[] =
        "     3.02           OBSERVATION DATA    G                   RINEX VERSION / TYPE\n"
        "G    1 C1C                                                  SYS / # / OBS TYPES\n"
        "                                                            END OF HEADER\n";
    SidereonRinexObs *source = parse_text(text);
    if (source == NULL) {
        return;
    }
    const double versions[] = {1.99, 3.0, NAN, INFINITY, -INFINITY};
    for (size_t i = 0; i < sizeof(versions) / sizeof(versions[0]); i++) {
        SidereonRinexObsDowngradeResult *result = NULL;
        check(sidereon_rinex_obs_downgrade_to_rinex2(source, versions[i],
                                                      &result) ==
                  SIDEREON_STATUS_OK &&
                  result != NULL,
              "invalid target returns typed result");
        SidereonRinexObsDowngradeOutcome outcome;
        check(sidereon_rinex_obs_downgrade_result_get_outcome(result, &outcome) ==
                  SIDEREON_STATUS_OK &&
                  !outcome.is_ok &&
                  outcome.status == SIDEREON_STATUS_INVALID_ARGUMENT &&
                  outcome.error.kind ==
                      SIDEREON_RINEX_OBS_WRITE_ERROR_KIND_NOT_VERSION_TWO &&
                  outcome.error.has_version && outcome.change_count == 0,
              "invalid target keeps typed NotVersionTwo refusal");
        SidereonRinexObs *none = (SidereonRinexObs *)(uintptr_t)1;
        check(sidereon_rinex_obs_downgrade_result_take_obs(result, &none) ==
                  SIDEREON_STATUS_INVALID_ARGUMENT &&
                  none == NULL,
              "refusal has no product");
        sidereon_rinex_obs_downgrade_result_free(result);
    }
    SidereonRinexObsHeader header;
    check(sidereon_rinex_obs_header(source, &header) == SIDEREON_STATUS_OK &&
              header.version == 3.02,
          "refusals leave source unchanged and usable");

    SidereonRinexObsDowngradeResult *sentinel =
        (SidereonRinexObsDowngradeResult *)(uintptr_t)1;
    check(sidereon_rinex_obs_downgrade_to_rinex2(NULL, 2.11, &sentinel) ==
              SIDEREON_STATUS_NULL_POINTER &&
              sentinel == NULL,
          "null source initializes result output");
    check(sidereon_rinex_obs_downgrade_to_rinex2(source, 2.11, NULL) ==
              SIDEREON_STATUS_NULL_POINTER,
          "null result output rejected");
    check(sidereon_rinex_obs_header_at(NULL, 0, &header) ==
              SIDEREON_STATUS_NULL_POINTER,
          "null header source rejected");
    check(sidereon_rinex_obs_header_at(source, 0, NULL) ==
              SIDEREON_STATUS_NULL_POINTER,
          "null header output rejected");
    sidereon_rinex_obs_free(source);
}

int main(int argc, char **argv) {
    if (argc == 4) {
        test_header_details(argv[1], argv[2], argv[3]);
    } else {
        check(false, "header detail fixture arguments");
    }
    test_header_routes_and_skips();
    test_downgrade();
    test_downgrade_refusals();
    if (failures != 0) {
        fprintf(stderr, "rinex_obs_complete_smoke: %d failure(s)\n", failures);
        return 1;
    }
    puts("rinex_obs_complete_smoke: ok");
    return 0;
}
