/* Deterministic smoke coverage for the public SBAS PRN lookup route. */
#include "sidereon.h"

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

static int fail(const char *what) {
    char message[256] = {0};
    sidereon_last_error_message(message, sizeof(message));
    fprintf(stderr, "FAIL: %s (last_error: %s)\n", what, message);
    return 1;
}

static bool last_error_contains(const char *needle) {
    char message[256] = {0};
    size_t written = sidereon_last_error_message(message, sizeof(message));
    return written > 0 && strstr(message, needle) != NULL;
}

int main(void) {
    uint8_t token[8];
    size_t written = 99;
    size_t required = 99;

    /* Frozen engine-produced value: core sbas_prn_to_sat(120) renders S20. */
    memset(token, 0xa5, sizeof(token));
    if (sidereon_sbas_prn_to_satellite_id(
            120, token, sizeof(token), &written, &required) != SIDEREON_STATUS_OK ||
        written != 3 || required != 3 || memcmp(token, "S20", 3) != 0) {
        return fail("mapped PRN 120");
    }
    for (size_t i = 3; i < sizeof(token); ++i) {
        if (token[i] != 0xa5) {
            return fail("mapped PRN wrote past payload");
        }
    }

    /* A query must not write bytes and reports the mapped token length. */
    written = 99;
    required = 99;
    if (sidereon_sbas_prn_to_satellite_id(120, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != 0 || required != 3) {
        return fail("mapped PRN size query");
    }

    /* The core's absent mapping is a successful empty optional-string result. */
    memset(token, 0xa5, sizeof(token));
    written = 99;
    required = 99;
    if (sidereon_sbas_prn_to_satellite_id(
            119, token, sizeof(token), &written, &required) != SIDEREON_STATUS_OK ||
        written != 0 || required != 0) {
        return fail("absent PRN mapping");
    }
    for (size_t i = 0; i < sizeof(token); ++i) {
        if (token[i] != 0xa5) {
            return fail("absent PRN wrote bytes");
        }
    }

    /* A nonzero length still requires a real output buffer, even for the
     * mapped case; counts are initialized before this validation. */
    written = 99;
    required = 99;
    if (sidereon_sbas_prn_to_satellite_id(
            120, NULL, 1, &written, &required) != SIDEREON_STATUS_NULL_POINTER ||
        written != 0 || required != 3 ||
        !last_error_contains("sidereon_sbas_prn_to_satellite_id: null out")) {
        return fail("null output buffer");
    }

    written = 99;
    required = 77;
    if (sidereon_sbas_prn_to_satellite_id(
            120, token, sizeof(token), NULL, &required) != SIDEREON_STATUS_NULL_POINTER ||
        required != 77 ||
        !last_error_contains(
            "sidereon_sbas_prn_to_satellite_id: null out_written")) {
        return fail("null out_written");
    }

    written = 99;
    required = 77;
    if (sidereon_sbas_prn_to_satellite_id(
            120, token, sizeof(token), &written, NULL) != SIDEREON_STATUS_NULL_POINTER ||
        written != 0 ||
        !last_error_contains(
            "sidereon_sbas_prn_to_satellite_id: null out_required")) {
        return fail("null out_required");
    }

    /* A short buffer fails without modifying it, reports the full count, and
     * leaves out_written at zero. */
    memset(token, 0x5a, sizeof(token));
    written = 99;
    required = 99;
    if (sidereon_sbas_prn_to_satellite_id(
            120, token, 2, &written, &required) != SIDEREON_STATUS_INVALID_ARGUMENT ||
        written != 0 || required != 3 || token[0] != 0x5a || token[1] != 0x5a ||
        !last_error_contains("sidereon_sbas_prn_to_satellite_id: out needs room")) {
        return fail("short output buffer");
    }

    puts("sbas_prn_smoke: OK");
    return 0;
}
