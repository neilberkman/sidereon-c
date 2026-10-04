/* Public TimeModelError detail and legacy-message control. */
#include "sidereon.h"

#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int check_tow_refusal(uint32_t which, const char *kind, const char *field,
                             const char *reason, const char *message,
                             const char *operation) {
    SidereonGnssWeekTow input = {
        .system = SIDEREON_TIME_SCALE_GPST,
        .week = UINT32_MAX,
        .tow_s = 0.0,
    };
    SidereonStatus status;
    uint32_t week = 17;
    SidereonGnssWeekTow output;
    if (which == 0) {
        status = sidereon_gnss_week_tow_new(
            SIDEREON_TIME_SCALE_GPST, 1, NAN, &output);
    } else if (which == 1) {
        input.tow_s = 604800.0;
        status = sidereon_gnss_week_tow_normalized(&input, &output);
    } else {
        status = sidereon_gnss_week_tow_unrolled_week(&input, 1, &week);
    }
    SidereonEngineErrorInfo info;
    memset(&info, 0, sizeof info);
    if (status != SIDEREON_STATUS_INVALID_ARGUMENT ||
        sidereon_last_engine_error_info(&info) != SIDEREON_STATUS_OK ||
        info.family != SIDEREON_ENGINE_ERROR_FAMILY_TIME_MODEL) {
        fprintf(stderr, "TimeModelError GNSS week/TOW %s status/family mismatch\n", kind);
        return 1;
    }
    size_t written = 0;
    size_t required = 0;
    if (sidereon_last_engine_error_payload(NULL, 0, &written, &required) != SIDEREON_STATUS_OK ||
        written != 0 || required != info.payload_len) {
        return 1;
    }
    uint8_t *payload = (uint8_t *)malloc(required + 1);
    if (payload == NULL ||
        sidereon_last_engine_error_payload(payload, required, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != required) {
        free(payload);
        return 1;
    }
    payload[written] = 0;
    char expected[512];
    char operation_part[160];
    (void)snprintf(expected, sizeof expected, "\"kind\":\"%s\"", kind);
    (void)snprintf(operation_part, sizeof operation_part,
                   "\"operation\":\"%s\"", operation);
    const char *json = (const char *)payload;
    const int fields_ok = strstr(json, "\"schema_version\":1") != NULL &&
                          strstr(json, "\"family\":\"time_model\"") != NULL &&
                          strstr(json, operation_part) != NULL &&
                          strstr(json, expected) != NULL;
    char field_part[128];
    char reason_part[128];
    char message_part[256];
    (void)snprintf(field_part, sizeof field_part, "\"field\":\"%s\"", field);
    (void)snprintf(reason_part, sizeof reason_part, "\"reason\":\"%s\"", reason);
    (void)snprintf(message_part, sizeof message_part, "\"message\":\"%s\"", message);
    const int complete = fields_ok && strstr(json, field_part) != NULL &&
                         strstr(json, reason_part) != NULL && strstr(json, message_part) != NULL;
    free(payload);
    if (!complete) {
        fprintf(stderr, "TimeModelError GNSS week/TOW %s payload mismatch\n", kind);
        return 1;
    }
    char legacy[512] = {0};
    (void)sidereon_last_error_message(legacy, sizeof legacy);
    char expected_legacy[512];
    (void)snprintf(expected_legacy, sizeof expected_legacy, "%s: %s", operation, message);
    if (strcmp(legacy, expected_legacy) != 0) {
        fprintf(stderr, "TimeModelError GNSS week/TOW %s legacy message mismatch\n", kind);
        return 1;
    }
    return 0;
}

int main(void) {
    SidereonSiderealFilterOptions options;
    if (sidereon_sidereal_filter_options_init(&options) != SIDEREON_STATUS_OK) {
        return 1;
    }
    options.sample_interval_s = NAN;
    const double series[] = {1.0, 2.0};
    SidereonSiderealFilterOutput *output = (SidereonSiderealFilterOutput *)(uintptr_t)1;
    const SidereonStatus status = sidereon_sidereal_filter(
        series, sizeof series / sizeof series[0], 86400.0, &options, &output);
    SidereonEngineErrorInfo info;
    memset(&info, 0, sizeof info);
    if (status != SIDEREON_STATUS_INVALID_ARGUMENT || output != NULL ||
        sidereon_last_engine_error_info(&info) != SIDEREON_STATUS_OK ||
        info.family != SIDEREON_ENGINE_ERROR_FAMILY_TIME_MODEL) {
        fprintf(stderr, "TimeModelError producer status/output/family mismatch\n");
        return 1;
    }

    size_t written = 0;
    size_t required = 0;
    if (sidereon_last_engine_error_payload(NULL, 0, &written, &required) != SIDEREON_STATUS_OK ||
        written != 0 || required != info.payload_len) {
        fprintf(stderr, "TimeModelError payload query mismatch\n");
        return 1;
    }
    uint8_t *payload = (uint8_t *)malloc(required + 1);
    if (payload == NULL ||
        sidereon_last_engine_error_payload(payload, required, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != required) {
        free(payload);
        fprintf(stderr, "TimeModelError payload copy mismatch\n");
        return 1;
    }
    payload[written] = 0;
    const char *json = (const char *)payload;
    const int complete_payload =
        strstr(json, "\"schema_version\":1") != NULL &&
        strstr(json, "\"family\":\"time_model\"") != NULL &&
        strstr(json, "\"operation\":\"sidereon_sidereal_filter\"") != NULL &&
        strstr(json, "\"kind\":\"invalid_input\"") != NULL &&
        strstr(json, "\"message\":\"invalid time model seconds: must be finite\"") != NULL &&
        strstr(json, "\"field\":\"seconds\"") != NULL &&
        strstr(json, "\"reason\":\"must be finite\"") != NULL;
    free(payload);
    if (!complete_payload) {
        fprintf(stderr, "TimeModelError payload fields mismatch\n");
        return 1;
    }

    char message[256] = {0};
    (void)sidereon_last_error_message(message, sizeof message);
    if (strcmp(message,
               "sidereon_sidereal_filter: invalid options.sample_interval_s: "
               "invalid time model seconds: must be finite") != 0) {
        fprintf(stderr, "TimeModelError legacy message mismatch: %s\n", message);
        return 1;
    }
    if (check_tow_refusal(0, "invalid_input", "tow_s", "must be finite",
                          "invalid time model tow_s: must be finite",
                          "sidereon_gnss_week_tow_new") != 0 ||
        check_tow_refusal(1, "invalid_input", "tow_s", "normalized week is out of range",
                          "invalid time model tow_s: normalized week is out of range",
                          "sidereon_gnss_week_tow_normalized") != 0 ||
        check_tow_refusal(2, "invalid_input", "rollovers", "unrolled week is out of range",
                          "invalid time model rollovers: unrolled week is out of range",
                          "sidereon_gnss_week_tow_unrolled_week") != 0) {
        return 1;
    }
    return 0;
}
