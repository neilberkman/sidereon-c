/*
 * Focused C-ABI coverage for exact-SP3 request construction, semantic
 * validation, coverage reporting, and raw declared-header accessors.
 */
#include "sidereon.h"

#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static uint8_t *read_file(const char *path, size_t *out_len) {
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
    uint8_t *bytes = (uint8_t *)malloc((size_t)size);
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
    *out_len = read;
    return bytes;
}

static int fail(const char *context, int code) {
    char message[512];
    sidereon_last_error_message(message, sizeof(message));
    fprintf(stderr, "FAIL %d: %s: %s\n", code, context, message);
    return code;
}

int main(int argc, char **argv) {
    if (argc != 3) {
        fprintf(stderr, "usage: %s valid.sp3 truncated.sp3\n", argv[0]);
        return 2;
    }

    size_t valid_len = 0;
    uint8_t *valid = read_file(argv[1], &valid_len);
    size_t truncated_len = 0;
    uint8_t *truncated = read_file(argv[2], &truncated_len);
    if (valid == NULL || truncated == NULL) {
        free(valid);
        free(truncated);
        return fail("fixture read", 3);
    }

    struct SidereonExactSp3Request *request = NULL;
    if (sidereon_sp3_exact_request_new(
            2020, 6, 24, NULL, "01D", "15M", "GRGS", &request) !=
            SIDEREON_STATUS_OK ||
        request == NULL) {
        free(valid);
        free(truncated);
        return fail("exact request", 4);
    }

    struct SidereonSp3 *sp3 = NULL;
    enum SidereonExactSp3Coverage coverage = SIDEREON_EXACT_SP3_COVERAGE_INCLUSIVE;
    if (sidereon_sp3_load_exact(valid, valid_len, request, &sp3, &coverage) !=
            SIDEREON_STATUS_OK ||
        sp3 == NULL || coverage != SIDEREON_EXACT_SP3_COVERAGE_HALF_OPEN) {
        sidereon_sp3_exact_request_free(request);
        free(valid);
        free(truncated);
        return fail("exact half-open load", 5);
    }

    size_t parsed_count = 0;
    uint64_t declared_count = 0;
    uint8_t declared_start_present = 0;
    double declared_start = 0.0;
    if (sidereon_sp3_epoch_count(sp3, &parsed_count) != SIDEREON_STATUS_OK ||
        sidereon_sp3_declared_epoch_count(sp3, &declared_count) != SIDEREON_STATUS_OK ||
        sidereon_sp3_declared_start_j2000_seconds(
            sp3, &declared_start_present, &declared_start) != SIDEREON_STATUS_OK ||
        parsed_count != 96 || declared_count != 96 || declared_start_present != 1) {
        sidereon_sp3_free(sp3);
        sidereon_sp3_exact_request_free(request);
        free(valid);
        free(truncated);
        return fail("declared header accessors", 6);
    }

    size_t epochs_written = 0;
    size_t epochs_required = 0;
    if (sidereon_sp3_epochs_j2000_seconds(
            sp3, NULL, 0, &epochs_written, &epochs_required) != SIDEREON_STATUS_OK ||
        epochs_written != 0 || epochs_required != parsed_count) {
        sidereon_sp3_free(sp3);
        sidereon_sp3_exact_request_free(request);
        free(valid);
        free(truncated);
        return fail("epoch size query", 7);
    }
    double *epochs = (double *)malloc(epochs_required * sizeof(double));
    if (epochs == NULL ||
        sidereon_sp3_epochs_j2000_seconds(
            sp3, epochs, epochs_required, &epochs_written, &epochs_required) !=
            SIDEREON_STATUS_OK ||
        epochs_written != parsed_count || fabs(epochs[0] - declared_start) > 1e-9) {
        free(epochs);
        sidereon_sp3_free(sp3);
        sidereon_sp3_exact_request_free(request);
        free(valid);
        free(truncated);
        return fail("declared start matches first epoch", 8);
    }
    free(epochs);

    coverage = SIDEREON_EXACT_SP3_COVERAGE_INCLUSIVE;
    if (sidereon_sp3_validate_exact(sp3, request, &coverage) != SIDEREON_STATUS_OK ||
        coverage != SIDEREON_EXACT_SP3_COVERAGE_HALF_OPEN) {
        sidereon_sp3_free(sp3);
        sidereon_sp3_exact_request_free(request);
        free(valid);
        free(truncated);
        return fail("exact validation", 9);
    }
    sidereon_sp3_free(sp3);
    sidereon_sp3_exact_request_free(request);

    struct SidereonExactSp3Request *bad_request = NULL;
    if (sidereon_sp3_exact_request_new(
            2020, 6, 24, NULL, "01D", "24H", NULL, &bad_request) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        bad_request != NULL) {
        free(valid);
        free(truncated);
        return fail("noncanonical cadence rejection", 10);
    }
    if (sidereon_sp3_exact_request_new(
            2020, 6, 24, NULL, "07D", "15M", NULL, &bad_request) !=
            SIDEREON_STATUS_OK ||
        bad_request == NULL) {
        free(valid);
        free(truncated);
        return fail("official 07D span syntax", 11);
    }
    sidereon_sp3_exact_request_free(bad_request);

    struct SidereonProductIdentity identity;
    if (sidereon_data_product_identity(
            "igs", SIDEREON_PRODUCT_FAMILY_SP3, 2022, 11, 27, NULL, NULL,
            &identity) != SIDEREON_STATUS_OK ||
        sidereon_sp3_exact_request_from_identity(&identity, &request) !=
            SIDEREON_STATUS_OK ||
        request == NULL) {
        free(valid);
        free(truncated);
        return fail("request from SP3 identity", 12);
    }
    sidereon_sp3_exact_request_free(request);
    if (sidereon_data_product_identity(
            "igs", SIDEREON_PRODUCT_FAMILY_RINEX_NAVIGATION, 2022, 11, 27,
            NULL, NULL, &identity) != SIDEREON_STATUS_OK ||
        sidereon_sp3_exact_request_from_identity(&identity, &request) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        request != NULL) {
        free(valid);
        free(truncated);
        return fail("non-SP3 request rejection", 13);
    }

    struct SidereonSp3 *permissive = NULL;
    if (sidereon_sp3_load(truncated, truncated_len, &permissive) != SIDEREON_STATUS_OK ||
        permissive == NULL ||
        sidereon_sp3_epoch_count(permissive, &parsed_count) != SIDEREON_STATUS_OK ||
        sidereon_sp3_declared_epoch_count(permissive, &declared_count) !=
            SIDEREON_STATUS_OK ||
        parsed_count != 11 || declared_count != 96) {
        sidereon_sp3_free(permissive);
        free(valid);
        free(truncated);
        return fail("permissive declared-count evidence", 14);
    }
    if (sidereon_sp3_exact_request_new(
            2026, 4, 30, NULL, "01D", "15M", "IGS", &request) !=
            SIDEREON_STATUS_OK ||
        sidereon_sp3_validate_exact(permissive, request, &coverage) !=
            SIDEREON_STATUS_INVALID_ARGUMENT) {
        sidereon_sp3_free(permissive);
        sidereon_sp3_exact_request_free(request);
        free(valid);
        free(truncated);
        return fail("truncated exact product rejection", 15);
    }
    sidereon_sp3_free(permissive);
    sidereon_sp3_exact_request_free(request);

    static const uint8_t malformed[] = "not an SP3 product";
    if (sidereon_sp3_exact_request_new(
            2020, 6, 24, NULL, "01D", "15M", NULL, &request) !=
            SIDEREON_STATUS_OK || request == NULL) {
        free(valid);
        free(truncated);
        return fail("parse-invalid exact request", 16);
    }
    sp3 = (struct SidereonSp3 *)(uintptr_t)1;
    if (sidereon_sp3_load_exact(
            malformed, sizeof(malformed) - 1, request, &sp3, &coverage) !=
            SIDEREON_STATUS_SP3_PARSE ||
        sp3 != NULL) {
        if (sp3 != (struct SidereonSp3 *)(uintptr_t)1) {
            sidereon_sp3_free(sp3);
        }
        sidereon_sp3_exact_request_free(request);
        free(valid);
        free(truncated);
        return fail("parse-invalid exact product rejection", 17);
    }

    sidereon_sp3_exact_request_free(request);
    sidereon_sp3_exact_request_free(NULL);
    free(valid);
    free(truncated);
    return 0;
}
