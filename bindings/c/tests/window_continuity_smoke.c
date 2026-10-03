/* Compiled ABI coverage for window-scoped SP3 continuity and nominal issue due times. */
#include "sidereon.h"
#include "w6_window_continuity_pins.h"

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
    *out_len = read;
    return bytes;
}

static int fail(const char *context) {
    char message[512] = {0};
    sidereon_last_error_message(message, sizeof(message));
    fprintf(stderr, "FAIL: %s: %s\n", context, message);
    return 1;
}

static int inject_g01_seam_jump(uint8_t *bytes, size_t length) {
    char *cursor = (char *)bytes;
    char *end = cursor + length;
    int epoch_index = -1;
    size_t shifted = 0;

    while (cursor < end) {
        char *line_end = memchr(cursor, '\n', (size_t)(end - cursor));
        if (line_end == NULL) {
            line_end = end;
        }
        size_t line_len = (size_t)(line_end - cursor);
        if (line_len >= 2 && cursor[0] == '*' && cursor[1] == ' ') {
            epoch_index += 1;
        } else if (epoch_index >= 145 && line_len >= 18 && memcmp(cursor, "PG01", 4) == 0) {
            char field[15] = {0};
            char replacement[15] = {0};
            memcpy(field, cursor + 4, 14);
            double x_km = strtod(field, NULL);
            if (snprintf(replacement, sizeof(replacement), "%14.6f", x_km + 3000.0) != 14) {
                return 0;
            }
            memcpy(cursor + 4, replacement, 14);
            shifted += 1;
        }
        cursor = line_end == end ? end : line_end + 1;
    }
    return shifted > 0;
}

static char *continuity_verdict_json(
    const SidereonSp3 *sp3, double from_j2000_s, double through_j2000_s) {
    size_t written = 99;
    size_t required = 99;
    if (sidereon_sp3_continuity_verdict_json(
            sp3, 0, -1.0, from_j2000_s, through_j2000_s, NULL, 0, &written,
            &required) != SIDEREON_STATUS_OK ||
        written != 0 || required == 0) {
        return NULL;
    }
    char *json = (char *)malloc(required + 1);
    if (json == NULL ||
        sidereon_sp3_continuity_verdict_json(
            sp3, 0, -1.0, from_j2000_s, through_j2000_s, (uint8_t *)json,
            required, &written, &required) != SIDEREON_STATUS_OK ||
        written != required) {
        free(json);
        return NULL;
    }
    json[written] = '\0';
    return json;
}

/* Every expected text and value below is sidereon-core's own result for the
 * seam-injected product, formed into the JSON the C route writes
 * (bindings/c/src/sp3.rs and data_distribution.rs), from tests/valgen
 * (w6_window_continuity). */
static int same_json(const char *json, const char *expected) {
    return json != NULL && strcmp(json, expected) == 0;
}

static uint64_t f64_bits(double value) {
    uint64_t bits = 0;
    memcpy(&bits, &value, sizeof(bits));
    return bits;
}

static char *next_issue_json(void) {
    size_t written = 99;
    size_t required = 99;
    if (sidereon_data_next_issue_due_json(
            "igs_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2026, 8, 4, 2, 59, 59, NULL, 0,
            &written, &required) != SIDEREON_STATUS_OK ||
        written != 0 || required == 0) {
        return NULL;
    }
    char *json = (char *)malloc(required + 1);
    if (json == NULL ||
        sidereon_data_next_issue_due_json(
            "igs_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2026, 8, 4, 2, 59, 59,
            (uint8_t *)json, required, &written, &required) != SIDEREON_STATUS_OK ||
        written != required) {
        free(json);
        return NULL;
    }
    json[written] = '\0';
    return json;
}

int main(int argc, char **argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: %s <daily-sp3>\n", argv[0]);
        return 2;
    }

    int rc = 1;
    size_t byte_len = 0;
    uint8_t *bytes = read_file(argv[1], &byte_len);
    SidereonSp3 *sp3 = NULL;
    SidereonSp3 *merged = NULL;
    SidereonSp3MergeReport *report = NULL;
    double *epochs = NULL;
    char *json = NULL;

    if (bytes == NULL || !inject_g01_seam_jump(bytes, byte_len)) {
        rc = fail("prepare seam-injected SP3 fixture");
        goto cleanup;
    }
    if (sidereon_sp3_load(bytes, byte_len, &sp3) != SIDEREON_STATUS_OK) {
        rc = fail("load seam-injected SP3 fixture");
        goto cleanup;
    }

    size_t epoch_count = 0;
    size_t written = 0;
    size_t required = 0;
    if (sidereon_sp3_epoch_count(sp3, &epoch_count) != SIDEREON_STATUS_OK || epoch_count <= 145) {
        rc = fail("read SP3 epoch count");
        goto cleanup;
    }
    epochs = (double *)malloc(epoch_count * sizeof(double));
    if (epochs == NULL ||
        sidereon_sp3_epochs_j2000_seconds(
            sp3, epochs, epoch_count, &written, &required) != SIDEREON_STATUS_OK ||
        written != epoch_count || required != epoch_count) {
        rc = fail("copy SP3 epoch axis");
        goto cleanup;
    }
    double seam = epochs[144];

    double before_s = NAN;
    double after_s = NAN;
    if (sidereon_sp3_stencil_extent(sp3, &before_s, &after_s) != SIDEREON_STATUS_OK ||
        f64_bits(before_s) != W6_WINDOW_STENCIL_BEFORE_S_BITS ||
        f64_bits(after_s) != W6_WINDOW_STENCIL_AFTER_S_BITS) {
        rc = fail("derive stencil extent");
        goto cleanup;
    }

    json = continuity_verdict_json(sp3, epochs[24], epochs[72]);
    if (!same_json(json, W6_WINDOW_VERDICT_INSIDE_DAY_JSON)) {
        rc = fail("inside-one-day continuity mapping");
        goto cleanup;
    }
    free(json);
    json = continuity_verdict_json(sp3, seam - 600.0, seam + 600.0);
    if (!same_json(json, W6_WINDOW_VERDICT_STRADDLING_JSON)) {
        rc = fail("straddling continuity mapping");
        goto cleanup;
    }
    free(json);
    json = continuity_verdict_json(sp3, seam - 7200.0, seam - after_s);
    if (!same_json(json, W6_WINDOW_VERDICT_STENCIL_BOUNDARY_JSON)) {
        rc = fail("stencil-boundary continuity mapping");
        goto cleanup;
    }
    free(json);
    json = continuity_verdict_json(sp3, seam - 7200.0, seam - after_s - 0.001);
    if (!same_json(json, W6_WINDOW_VERDICT_OUTSIDE_STENCIL_JSON)) {
        rc = fail("outside-stencil continuity mapping");
        goto cleanup;
    }
    free(json);
    json = NULL;

    SidereonSp3MergeOptions options;
    if (sidereon_sp3_merge_options_init(&options) != SIDEREON_STATUS_OK) {
        rc = fail("initialize merge options");
        goto cleanup;
    }
    options.min_agree = 1;
    options.clock_min_common = 1;
    const SidereonSp3 *sources[1] = {sp3};
    if (sidereon_sp3_merge(sources, 1, &options, &merged, &report) != SIDEREON_STATUS_OK) {
        rc = fail("merge fixture for optional verdict");
        goto cleanup;
    }
    written = 99;
    required = 99;
    if (sidereon_sp3_merge_report_continuity_verdict_json(
            report, epochs[24], epochs[72], NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != 0 || required != strlen(W6_WINDOW_MERGE_VERDICT_JSON)) {
        rc = fail("query optional merge continuity verdict");
        goto cleanup;
    }
    uint8_t merge_json[sizeof(W6_WINDOW_MERGE_VERDICT_JSON)];
    if (sidereon_sp3_merge_report_continuity_verdict_json(
            report, epochs[24], epochs[72], merge_json, sizeof(merge_json), &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != required ||
        memcmp(merge_json, W6_WINDOW_MERGE_VERDICT_JSON, written) != 0) {
        rc = fail("copy optional merge continuity verdict");
        goto cleanup;
    }

    json = next_issue_json();
    if (!same_json(json, W6_WINDOW_NEXT_ISSUE_JSON)) {
        rc = fail("map next nominal issue");
        goto cleanup;
    }
    free(json);
    json = NULL;

    /* sidereon-core refuses the wum_nrt nominal schedule; the binding maps a
     * catalog refusal to INVALID_ARGUMENT (map_error in data_distribution.rs). */
    if (!W6_WINDOW_WUM_NRT_NEXT_ISSUE_REFUSED) {
        rc = fail("sidereon-core gives a wum_nrt nominal issue");
        goto cleanup;
    }
    written = 99;
    required = 99;
    if (sidereon_data_next_issue_due_json(
            "wum_nrt", SIDEREON_PRODUCT_FAMILY_SP3, 2026, 8, 4, 0, 0, 0, NULL, 0,
            &written, &required) != SIDEREON_STATUS_INVALID_ARGUMENT ||
        written != 0 || required != 0) {
        rc = fail("reject unsupported nominal schedule");
        goto cleanup;
    }

    puts("window continuity + next-issue ABI smoke passed");
    rc = 0;

cleanup:
    free(json);
    free(epochs);
    sidereon_sp3_merge_report_free(report);
    sidereon_sp3_free(merged);
    sidereon_sp3_free(sp3);
    free(bytes);
    return rc;
}
