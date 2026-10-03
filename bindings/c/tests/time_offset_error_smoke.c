/* Public controls for every core TimeOffsetError variant. */
#include "sidereon.h"

#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef SidereonStatus (*offset_call)(double *out);

static SidereonStatus epoch_required(double *out) {
    return sidereon_timescale_offset_s(SIDEREON_TIME_SCALE_GPST, SIDEREON_TIME_SCALE_UTC, out);
}

static SidereonStatus epoch_required_glonasst(double *out) {
    return sidereon_timescale_offset_s(
        SIDEREON_TIME_SCALE_GPST, SIDEREON_TIME_SCALE_GLONASST, out);
}

static SidereonStatus unsupported_tcg(double *out) {
    return sidereon_timescale_offset_s(SIDEREON_TIME_SCALE_GPST, SIDEREON_TIME_SCALE_TCG, out);
}

static SidereonStatus unsupported_tcb(double *out) {
    return sidereon_timescale_offset_s(SIDEREON_TIME_SCALE_GPST, SIDEREON_TIME_SCALE_TCB, out);
}

static SidereonStatus unsupported_tdb(double *out) {
    return sidereon_timescale_offset_s(SIDEREON_TIME_SCALE_GPST, SIDEREON_TIME_SCALE_TDB, out);
}

static SidereonStatus non_finite_epoch_utc(double *out) {
    return sidereon_timescale_offset_at_s(
        SIDEREON_TIME_SCALE_UTC, SIDEREON_TIME_SCALE_TAI, NAN, out);
}

static SidereonStatus non_finite_epoch_glonasst(double *out) {
    return sidereon_timescale_offset_at_s(
        SIDEREON_TIME_SCALE_GLONASST, SIDEREON_TIME_SCALE_TAI, INFINITY, out);
}

static int check_case(offset_call call, const char *kind, const char *scale,
                      const char *core_message, const char *operation) {
    double offset = 123.0;
    if (call(&offset) != SIDEREON_STATUS_INVALID_ARGUMENT || offset != 0.0) {
        fprintf(stderr, "TimeOffsetError %s status/output mismatch\n", kind);
        return 1;
    }
    SidereonEngineErrorInfo info;
    memset(&info, 0, sizeof info);
    if (sidereon_last_engine_error_info(&info) != SIDEREON_STATUS_OK ||
        info.family != SIDEREON_ENGINE_ERROR_FAMILY_TIME_OFFSET) {
        fprintf(stderr, "TimeOffsetError %s family mismatch\n", kind);
        return 1;
    }
    size_t written = 0;
    size_t required = 0;
    if (sidereon_last_engine_error_payload(NULL, 0, &written, &required) != SIDEREON_STATUS_OK ||
        written != 0 || required != info.payload_len) {
        fprintf(stderr, "TimeOffsetError %s payload query mismatch\n", kind);
        return 1;
    }
    uint8_t *payload = (uint8_t *)malloc(required + 1);
    if (payload == NULL ||
        sidereon_last_engine_error_payload(payload, required, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != required) {
        free(payload);
        fprintf(stderr, "TimeOffsetError %s payload copy mismatch\n", kind);
        return 1;
    }
    payload[written] = 0;
    char kind_field[96];
    (void)snprintf(kind_field, sizeof kind_field, "\"kind\":\"%s\"", kind);
    char message_field[512];
    (void)snprintf(message_field, sizeof message_field, "\"message\":\"%s\"", core_message);
    char scale_field[64];
    (void)snprintf(scale_field, sizeof scale_field, "\"scale\":\"%s\"", scale);
    const char *json = (const char *)payload;
    char operation_field[160];
    (void)snprintf(operation_field, sizeof operation_field,
                   "\"operation\":\"%s\"", operation);
    const int complete_payload =
        strstr(json, "\"schema_version\":1") != NULL &&
        strstr(json, "\"family\":\"time_offset\"") != NULL &&
        strstr(json, operation_field) != NULL && strstr(json, kind_field) != NULL &&
        strstr(json, message_field) != NULL && strstr(json, scale_field) != NULL;
    free(payload);
    if (!complete_payload) {
        fprintf(stderr, "TimeOffsetError %s payload fields mismatch\n", kind);
        return 1;
    }
    char expected_legacy[640];
    (void)snprintf(expected_legacy, sizeof expected_legacy, "%s: %s", operation, core_message);
    char legacy[640] = {0};
    (void)sidereon_last_error_message(legacy, sizeof legacy);
    if (strcmp(legacy, expected_legacy) != 0) {
        fprintf(stderr, "TimeOffsetError %s legacy message mismatch: %s\n", kind, legacy);
        return 1;
    }
    return 0;
}

int main(void) {
    if (check_case(epoch_required, "epoch_required", "UTC",
                   "time-scale UTC is UTC-based; its offset is epoch-dependent, use timescale_offset_at_s",
                   "sidereon_timescale_offset_s") != 0 ||
        check_case(epoch_required_glonasst, "epoch_required", "GLONASST",
                   "time-scale GLONASST is UTC-based; its offset is epoch-dependent, use timescale_offset_at_s",
                   "sidereon_timescale_offset_s") != 0 ||
        check_case(unsupported_tcg, "unsupported", "TCG",
                   "time-scale TCG has no fixed/constant offset; resolve it through TimeScales",
                   "sidereon_timescale_offset_s") != 0 ||
        check_case(unsupported_tdb, "unsupported", "TDB",
                   "time-scale TDB has no fixed/constant offset; resolve it through TimeScales",
                   "sidereon_timescale_offset_s") != 0 ||
        check_case(unsupported_tcb, "unsupported", "TCB",
                   "time-scale TCB has no fixed/constant offset; resolve it through TimeScales",
                   "sidereon_timescale_offset_s") != 0 ||
        check_case(non_finite_epoch_utc, "non_finite_epoch", "UTC",
                   "utc_jd must be finite to resolve leap seconds for scale UTC",
                   "sidereon_timescale_offset_at_s") != 0) {
        return 1;
    }
    return check_case(non_finite_epoch_glonasst, "non_finite_epoch", "GLONASST",
                      "utc_jd must be finite to resolve leap seconds for scale GLONASST",
                      "sidereon_timescale_offset_at_s");
}
