#include "sidereon.h"

#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

static int read_file(const char *path, uint8_t **out, size_t *out_len) {
    FILE *file = fopen(path, "rb");
    if (file == NULL || fseek(file, 0, SEEK_END) != 0) {
        if (file != NULL) fclose(file);
        return 0;
    }
    long length = ftell(file);
    if (length < 0 || fseek(file, 0, SEEK_SET) != 0) {
        fclose(file);
        return 0;
    }
    uint8_t *bytes = malloc((size_t)length);
    if (bytes == NULL || fread(bytes, 1, (size_t)length, file) != (size_t)length) {
        free(bytes);
        fclose(file);
        return 0;
    }
    fclose(file);
    *out = bytes;
    *out_len = (size_t)length;
    return 1;
}

static int fail(const char *message, int code) {
    fprintf(stderr, "sp3_accuracy_rounding_smoke: %s\n", message);
    return code;
}

static int check_unbased_pair(const char *a_path, const char *b_path, bool reverse) {
    uint8_t *a_bytes = NULL, *b_bytes = NULL;
    size_t a_len = 0, b_len = 0;
    if (!read_file(a_path, &a_bytes, &a_len) || !read_file(b_path, &b_bytes, &b_len))
        return fail("read unbased pair", 20);
    SidereonSp3 *a = NULL, *b = NULL;
    if (sidereon_sp3_load(a_bytes, a_len, &a) != SIDEREON_STATUS_OK ||
        sidereon_sp3_load(b_bytes, b_len, &b) != SIDEREON_STATUS_OK) {
        free(a_bytes);
        free(b_bytes);
        return fail("parse unbased pair", 21);
    }
    free(a_bytes);
    free(b_bytes);

    SidereonSp3MergeOptions options;
    if (sidereon_sp3_merge_options_init(&options) != SIDEREON_STATUS_OK)
        return fail("initialize unbased merge options", 22);
    options.combine = SIDEREON_SP3_MERGE_COMBINE_PRECEDENCE;
    options.precedence_scope = SIDEREON_SP3_MERGE_PRECEDENCE_SCOPE_CELL;
    options.min_agree = 1;
    options.clock_min_common = 1;
    options.position_tolerance_m = 5.0;
    options.clock_tolerance_s = 50e-9;
    const SidereonSp3 *sources[] = {reverse ? b : a, reverse ? a : b};
    SidereonSp3 *merged = NULL;
    SidereonSp3MergeReport *report = NULL;
    if (sidereon_sp3_merge(sources, 2, &options, &merged, &report) != SIDEREON_STATUS_OK)
        return fail("merge unbased pair", 23);
    sidereon_sp3_merge_report_free(report);

    SidereonSp3RecordAccuracy accuracy = {0};
    if (sidereon_sp3_record_accuracy(merged, "G12", 0, &accuracy) != SIDEREON_STATUS_OK ||
        !accuracy.has_p || accuracy.p.position_sigma_m[0].kind != SIDEREON_SP3_ACCURACY_VALUE_KIND_KNOWN ||
        fabs(accuracy.p.position_sigma_m[0].value - pow(1.25, 13.0) * 1e-3) > 1e-15)
        return fail("unbased pair did not preserve G12 decoded sigma", 24);
    SidereonSp3WriteResult *strict = NULL;
    SidereonSp3WriteOutcome outcome;
    if (sidereon_sp3_to_sp3_text_result(merged, &strict) != SIDEREON_STATUS_OK || strict == NULL ||
        sidereon_sp3_write_result_get_outcome(strict, &outcome) != SIDEREON_STATUS_OK ||
        !outcome.is_ok)
        return fail("unbased pair strict write", 25);
    sidereon_sp3_write_result_free(strict);
    sidereon_sp3_free(merged);
    sidereon_sp3_free(a);
    sidereon_sp3_free(b);
    return 0;
}

int main(int argc, char **argv) {
    if (argc != 5) return fail("expected accuracy A/B and unbased A/B paths", 1);
    uint8_t *a_bytes = NULL, *b_bytes = NULL;
    size_t a_len = 0, b_len = 0;
    if (!read_file(argv[1], &a_bytes, &a_len) || !read_file(argv[2], &b_bytes, &b_len))
        return fail("read fixture", 2);
    SidereonSp3 *a = NULL, *b = NULL, *merged = NULL;
    if (sidereon_sp3_load(a_bytes, a_len, &a) != SIDEREON_STATUS_OK ||
        sidereon_sp3_load(b_bytes, b_len, &b) != SIDEREON_STATUS_OK)
        return fail("parse source fixtures", 3);
    free(a_bytes);
    free(b_bytes);

    SidereonSp3MergeOptions options;
    if (sidereon_sp3_merge_options_init(&options) != SIDEREON_STATUS_OK)
        return fail("initialize merge options", 4);
    options.min_agree = 1;
    options.clock_min_common = 1;
    const SidereonSp3 *sources[] = {a, b};
    SidereonSp3MergeReport *merge_report = NULL;
    if (sidereon_sp3_merge(sources, 2, &options, &merged, &merge_report) != SIDEREON_STATUS_OK)
        return fail("merge fixtures", 5);
    sidereon_sp3_merge_report_free(merge_report);
    sidereon_sp3_free(a);
    sidereon_sp3_free(b);

    SidereonSp3WriteResult *strict = NULL;
    if (sidereon_sp3_to_sp3_text_result(merged, &strict) != SIDEREON_STATUS_OK || strict == NULL)
        return fail("obtain strict merged result", 6);
    SidereonSp3WriteOutcome outcome;
    if (sidereon_sp3_write_result_get_outcome(strict, &outcome) != SIDEREON_STATUS_OK ||
        outcome.is_ok || outcome.error.kind != SIDEREON_SP3_WRITE_ERROR_KIND_ACCURACY_NOT_REPRESENTABLE)
        return fail("strict merged write did not retain typed accuracy refusal", 7);
    sidereon_sp3_write_result_free(strict);

    SidereonSp3WriteResult *rounded = NULL;
    if (sidereon_sp3_to_sp3_text_with_accuracy_rounding_result(
            merged, SIDEREON_SP3_ACCURACY_ROUNDING_POLICY_CONSERVATIVE, &rounded) !=
            SIDEREON_STATUS_OK || rounded == NULL)
        return fail("call conservative accuracy writer", 8);
    if (sidereon_sp3_write_result_get_outcome(rounded, &outcome) != SIDEREON_STATUS_OK ||
        !outcome.is_ok)
        return fail("conservative accuracy writer refused", 9);
    bool has_accuracy = false;
    SidereonSp3AccuracyRoundingReport accuracy = {0};
    if (sidereon_sp3_write_result_get_accuracy_rounding_report(
            rounded, &has_accuracy, &accuracy) != SIDEREON_STATUS_OK || !has_accuracy)
        return fail("read accuracy report", 10);
    const double source_sigma = pow(1.25, 13.0) * 1e-3;
    const double expected_delta = 0.032 - source_sigma;
    const double source_velocity_sigma = pow(1.25, 13.0) * 1e-7;
    const double expected_velocity_delta = 32e-7 - source_velocity_sigma;
    const double source_clock_sigma = pow(1.025, 13.0) * 299792458e-12;
    const double expected_clock_delta = 2.0 * 299792458e-12 - source_clock_sigma;
    const double source_rate_sigma = pow(1.025, 13.0) * 299792458e-16;
    const double expected_rate_delta = 2.0 * 299792458e-16 - source_rate_sigma;
    if (fabs(accuracy.position_sigma_x_m - expected_delta) > 1e-15 ||
        fabs(accuracy.position_sigma_y_m - expected_delta) > 1e-15 ||
        fabs(accuracy.position_sigma_z_m - expected_delta) > 1e-15 ||
        fabs(accuracy.velocity_sigma_x_m_s - expected_velocity_delta) > 1e-15 ||
        fabs(accuracy.velocity_sigma_y_m_s - expected_velocity_delta) > 1e-15 ||
        fabs(accuracy.velocity_sigma_z_m_s - expected_velocity_delta) > 1e-15 ||
        fabs(accuracy.clock_sigma_m - expected_clock_delta) > 1e-15 ||
        fabs(accuracy.clock_rate_sigma_m_s - expected_rate_delta) > 1e-15)
        return fail("accuracy report differs from independent exponent calculation", 11);

    SidereonSp3RoundingReport records = {0};
    bool has_records = false;
    if (sidereon_sp3_write_result_get_rounding_report(rounded, &has_records, &records) !=
            SIDEREON_STATUS_OK || !has_records)
        return fail("legacy record report missing on detailed result", 12);

    SidereonSp3WriteResult *invalid = (SidereonSp3WriteResult *)(uintptr_t)1;
    if (sidereon_sp3_to_sp3_text_with_accuracy_rounding_result(merged, 99, &invalid) !=
            SIDEREON_STATUS_INVALID_ARGUMENT || invalid != NULL)
        return fail("unknown policy was not rejected before conversion", 13);

    sidereon_sp3_write_result_free(rounded);
    sidereon_sp3_free(merged);
    if (check_unbased_pair(argv[3], argv[4], false) != 0 ||
        check_unbased_pair(argv[3], argv[4], true) != 0)
        return 14;
    puts("sp3_accuracy_rounding_smoke: all checks passed");
    return 0;
}
