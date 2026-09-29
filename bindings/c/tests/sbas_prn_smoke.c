/* Deterministic smoke coverage for the public SBAS PRN lookup route. */
#include "sidereon.h"
#include "w6_sbas_prn_pins.h"

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

static int fail(const char *what) {
    /* The last error is sticky and may come from an earlier call, so it is
       reported as context rather than as the cause of this failure. */
    char message[256] = {0};
    sidereon_last_error_message(message, sizeof(message));
    if (message[0] != '\0') {
        fprintf(stderr, "FAIL: %s (last ABI error, may predate this check: %s)\n", what,
                message);
    } else {
        fprintf(stderr, "FAIL: %s\n", what);
    }
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

    /* The token sidereon_core::sbas::sbas_prn_to_sat(120) names, from
     * tests/valgen (w6_sbas_prn). */
    memset(token, 0xa5, sizeof(token));
    if (sidereon_sbas_prn_to_satellite_id(
            120, token, sizeof(token), &written, &required) != SIDEREON_STATUS_OK ||
        written != W6_SBAS_PRN_120_TOKEN_LEN || required != W6_SBAS_PRN_120_TOKEN_LEN ||
        memcmp(token, W6_SBAS_PRN_120_TOKEN, W6_SBAS_PRN_120_TOKEN_LEN) != 0) {
        return fail("mapped PRN 120");
    }
    for (size_t i = W6_SBAS_PRN_120_TOKEN_LEN; i < sizeof(token); ++i) {
        if (token[i] != 0xa5) {
            return fail("mapped PRN wrote past payload");
        }
    }

    /* A query must not write bytes and reports the mapped token length. */
    written = 99;
    required = 99;
    if (sidereon_sbas_prn_to_satellite_id(120, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != 0 || required != W6_SBAS_PRN_120_TOKEN_LEN) {
        return fail("mapped PRN size query");
    }

    /* The core names no satellite for PRN 119 (tests/valgen, w6_sbas_prn);
     * the binding reports an absent mapping as a successful empty result. */
    if (W6_SBAS_PRN_119_MAPPED) {
        return fail("sbas_prn_to_sat(119) maps a satellite; this check needs another PRN");
    }
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
        written != 0 || required != W6_SBAS_PRN_120_TOKEN_LEN ||
        !last_error_contains("sidereon_sbas_prn_to_satellite_id: null out")) {
        return fail("null output buffer");
    }

    /* Both counts are cleared before either is validated, so a null
     * out_written still leaves the writable out_required at zero rather than
     * the value the caller left in it. */
    written = 99;
    required = 77;
    if (sidereon_sbas_prn_to_satellite_id(
            120, token, sizeof(token), NULL, &required) != SIDEREON_STATUS_NULL_POINTER ||
        required != 0 ||
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
            120, token, W6_SBAS_PRN_120_TOKEN_LEN - 1, &written, &required) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        written != 0 || required != W6_SBAS_PRN_120_TOKEN_LEN || token[0] != 0x5a ||
        token[1] != 0x5a ||
        !last_error_contains("sidereon_sbas_prn_to_satellite_id: out needs room")) {
        return fail("short output buffer");
    }

    puts("sbas_prn_smoke: OK");
    return 0;
}
