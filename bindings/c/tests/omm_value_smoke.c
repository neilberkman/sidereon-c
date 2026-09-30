#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sidereon.h"

static int failures;

static void check(int condition, const char *label) {
    if (!condition) {
        fprintf(stderr, "FAIL: %s\n", label);
        failures++;
    }
}

static uint64_t double_bits(double value) {
    uint64_t bits;
    memcpy(&bits, &value, sizeof(bits));
    return bits;
}

static double julian_day_utc(int year, int month, int day, int hour, int minute, double second) {
    int a = (14 - month) / 12;
    int y = year + 4800 - a;
    int m = month + 12 * a - 3;
    int jdn = day + (153 * m + 2) / 5 + 365 * y + y / 4 - y / 100 + y / 400 - 32045;
    return (double)jdn - 0.5 + ((double)hour * 3600.0 + (double)minute * 60.0 + second) / 86400.0;
}

static uint8_t *read_file(const char *path, size_t *length) {
    FILE *file = fopen(path, "rb");
    if (file == NULL || fseek(file, 0, SEEK_END) != 0) {
        if (file != NULL) {
            fclose(file);
        }
        return NULL;
    }
    long size = ftell(file);
    if (size < 0 || fseek(file, 0, SEEK_SET) != 0) {
        fclose(file);
        return NULL;
    }
    uint8_t *bytes = (uint8_t *)malloc((size_t)size + 1);
    if (bytes == NULL) {
        fclose(file);
        return NULL;
    }
    size_t read = fread(bytes, 1, (size_t)size, file);
    fclose(file);
    if (read != (size_t)size) {
        free(bytes);
        return NULL;
    }
    bytes[read] = 0;
    *length = read;
    return bytes;
}

int main(int argc, char **argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: %s <omm-kvn>\n", argv[0]);
        return 2;
    }
    size_t input_length = 0;
    uint8_t *input = read_file(argv[1], &input_length);
    check(input != NULL, "read OMM fixture");
    if (input == NULL) {
        return 1;
    }

    SidereonOmm *omm = NULL;
    check(sidereon_omm_parse_kvn(input, input_length, &omm) == SIDEREON_STATUS_OK && omm != NULL,
          "parse OMM");
    free(input);
    if (omm == NULL) {
        return 1;
    }

    size_t written = 0;
    size_t required = 0;
    check(sidereon_omm_snapshot_json(omm, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
              written == 0 && required != 0,
          "query complete snapshot size");
    uint8_t *snapshot = (uint8_t *)malloc(required + 1);
    check(snapshot != NULL, "allocate snapshot buffer");
    if (snapshot != NULL) {
        check(sidereon_omm_snapshot_json(omm, snapshot, required, &written, &required) == SIDEREON_STATUS_OK &&
                  written == required,
              "copy complete snapshot");
        snapshot[written] = 0;
        check(strstr((const char *)snapshot, "\"exact_sgp4_epoch\":null") != NULL &&
                  strstr((const char *)snapshot, "\"quantize_tle_derived_fields\":true") != NULL &&
                  strstr((const char *)snapshot, "\"ccsds_omm_vers\":\"2.0\"") != NULL &&
                  strstr((const char *)snapshot, "\"epoch\"") != NULL &&
                  strstr((const char *)snapshot, "\"mean_motion\"") != NULL &&
                  strstr((const char *)snapshot, "\"spacecraft\":null") != NULL &&
                  strstr((const char *)snapshot, "\"comments\":{\"header\":[]") != NULL &&
                  strstr((const char *)snapshot, "\"user_defined\":[]") != NULL,
              "snapshot carries wire fields and non-wire bridge settings");
        SidereonOmm *rebuilt = NULL;
        check(sidereon_omm_from_snapshot_json(snapshot, written, &rebuilt) == SIDEREON_STATUS_OK && rebuilt != NULL,
              "construct from complete detached snapshot");
        if (rebuilt != NULL) {
            size_t rebuilt_required = 0;
            size_t rebuilt_written = 0;
            check(sidereon_omm_snapshot_json(rebuilt, NULL, 0, &rebuilt_written, &rebuilt_required) == SIDEREON_STATUS_OK && rebuilt_required == written,
                  "rebuilt snapshot has same byte length");
            uint8_t *roundtrip = (uint8_t *)malloc(rebuilt_required + 1);
            check(roundtrip != NULL, "allocate rebuilt snapshot");
            if (roundtrip != NULL) {
                check(sidereon_omm_snapshot_json(rebuilt, roundtrip, rebuilt_required, &rebuilt_written, &rebuilt_required) == SIDEREON_STATUS_OK &&
                          rebuilt_written == written && memcmp(snapshot, roundtrip, written) == 0,
                      "constructor preserves every snapshot field");
                free(roundtrip);
            }
            sidereon_omm_free(rebuilt);
        }
        free(snapshot);
    }

    const uint8_t incomplete[] = "{\"unexpected\":true}";
    SidereonOmm *rejected = NULL;
    check(sidereon_omm_from_snapshot_json(incomplete, sizeof(incomplete) - 1, &rejected) == SIDEREON_STATUS_INVALID_ARGUMENT && rejected == NULL,
          "reject unknown and missing snapshot fields without returning a handle");
    rejected = (SidereonOmm *)(uintptr_t)1;
    check(sidereon_omm_from_snapshot_json(NULL, 1, &rejected) == SIDEREON_STATUS_NULL_POINTER && rejected == NULL,
          "clear constructor output before reporting an invalid input slice");

    SidereonOmmElementSet elements = {0};
    check(sidereon_omm_to_element_set(omm, &elements) == SIDEREON_STATUS_OK,
          "invoke core OMM-to-element-set bridge");
    double expected_jd = julian_day_utc(2026, 6, 16, 4, 54, 23.504544);
    double expected_whole = trunc(expected_jd - 2433281.5) + 2433281.5;
    check(elements.epoch_whole == expected_whole &&
              elements.epoch_whole + elements.epoch_fraction == expected_jd &&
              double_bits(elements.epoch_whole) == UINT64_C(0x4142c70bc0000000) &&
              double_bits(elements.epoch_fraction) == UINT64_C(0x3fca2b0c32bc0000) &&
              elements.bstar == 0.0 && elements.mean_motion_dot_present && fabs(elements.mean_motion_dot + 1.2e-7) < 1e-20 &&
              elements.mean_motion_double_dot_present && elements.mean_motion_double_dot == 0.0 &&
              elements.eccentricity == 0.0102442 && elements.argument_of_perigee_deg == 56.9091 &&
              elements.inclination_deg == 55.9944 && elements.mean_anomaly_deg == 304.0464 &&
              elements.mean_motion_rev_per_day == 2.00563771 && elements.right_ascension_deg == 98.6138 &&
              elements.catalog_number_present && elements.catalog_number == 24876 &&
              elements.omm_epoch_days_present &&
              double_bits(elements.omm_epoch_days) == UINT64_C(0x40db458d1586195e),
          "all 15 bridge values match the independent Gregorian-JD calculation and fixture fields");

    sidereon_omm_free(omm);
    if (failures != 0) {
        fprintf(stderr, "omm_value_smoke: %d failure(s)\n", failures);
        return 1;
    }
    puts("omm_value_smoke: OK");
    return 0;
}
