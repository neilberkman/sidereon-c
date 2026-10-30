#include "sidereon.h"

#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

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

static int check_strict_source(const SidereonSp3 *sp3) {
    SidereonSp3WriteResult *result = NULL;
    SidereonSp3WriteOutcome outcome;
    if (sidereon_sp3_to_sp3_text_result(sp3, &result) != SIDEREON_STATUS_OK || result == NULL ||
        sidereon_sp3_write_result_get_outcome(result, &outcome) != SIDEREON_STATUS_OK ||
        !outcome.is_ok) {
        sidereon_sp3_write_result_free(result);
        return 0;
    }
    sidereon_sp3_write_result_free(result);
    return 1;
}

static char *copy_text(const uint8_t *bytes, size_t length) {
    char *text = malloc(length + 1);
    if (text == NULL) return NULL;
    memcpy(text, bytes, length);
    text[length] = '\0';
    return text;
}

static int replace_fixed(char *text, const char *old_value, const char *new_value) {
    size_t length = strlen(old_value);
    if (length != strlen(new_value)) return 0;
    char *at = strstr(text, old_value);
    if (at == NULL) return 0;
    memcpy(at, new_value, length);
    return 1;
}

static int set_record_codes(char *text, const char *prefix, int x, int y, int z, int clock) {
    size_t found = 0;
    for (char *line = text; *line != '\0';) {
        char *end = strchr(line, '\n');
        if (end == NULL) end = line + strlen(line);
        if ((size_t)(end - line) >= 73 && strncmp(line, prefix, strlen(prefix)) == 0) {
            char codes[13];
            int count = snprintf(codes, sizeof(codes), "%2d %2d %2d %3d", x, y, z, clock);
            if (count != 12) return 0;
            memcpy(line + 61, codes, 12);
            found++;
        }
        line = *end == '\0' ? end : end + 1;
    }
    return found == 2;
}

static SidereonSp3 *load_text(char *text) {
    SidereonSp3 *sp3 = NULL;
    if (text != NULL && sidereon_sp3_load((const uint8_t *)text, strlen(text), &sp3) == SIDEREON_STATUS_OK)
        return sp3;
    return NULL;
}

static int check_report_delta(const SidereonSp3WriteResult *result, const SidereonSp3 *source,
                              const char *satellite, size_t epoch, int component,
                              double reported_delta) {
    uint8_t output[8192]; size_t written = 0, required = 0;
    if (sidereon_sp3_write_result_get_text(result, output, sizeof(output), &written, &required) != SIDEREON_STATUS_OK)
        return 0;
    SidereonSp3 *reparsed = NULL;
    if (sidereon_sp3_load(output, written, &reparsed) != SIDEREON_STATUS_OK) return 0;
    SidereonSp3RecordAccuracy before = {0}, after = {0};
    int ok = sidereon_sp3_record_accuracy(source, satellite, epoch, &before) == SIDEREON_STATUS_OK &&
             sidereon_sp3_record_accuracy(reparsed, satellite, epoch, &after) == SIDEREON_STATUS_OK;
    double source_sigma = 0, output_sigma = 0;
    if (ok && component == 0) {
        source_sigma = before.p.position_sigma_m[0].value;
        output_sigma = after.p.position_sigma_m[0].value;
    } else if (ok) {
        source_sigma = before.p.clock_sigma_m.value;
        output_sigma = after.p.clock_sigma_m.value;
    }
    ok = ok && fabs(reported_delta - (output_sigma - source_sigma)) <= 1e-15;
    sidereon_sp3_free(reparsed);
    return ok;
}

static int merge_two(const SidereonSp3 *a, const SidereonSp3 *b, SidereonSp3 **out) {
    SidereonSp3MergeOptions options;
    if (sidereon_sp3_merge_options_init(&options) != SIDEREON_STATUS_OK) return 0;
    options.min_agree = 1;
    options.clock_min_common = 1;
    const SidereonSp3 *sources[] = {a, b};
    SidereonSp3MergeReport *report = NULL;
    int ok = sidereon_sp3_merge(sources, 2, &options, out, &report) == SIDEREON_STATUS_OK;
    sidereon_sp3_merge_report_free(report);
    return ok;
}

static int check_basis_matrix(const uint8_t *a_bytes, size_t a_len,
                              const uint8_t *b_bytes, size_t b_len) {
    char *a_text = copy_text(a_bytes, a_len), *b_text = copy_text(b_bytes, b_len);
    if (a_text == NULL || b_text == NULL) return fail("copy basis fixtures", 40);
    if (!replace_fixed(b_text, "1.2500000  1.025000000", "2.0000000  2.000000000"))
        return fail("make compatible bases", 41);
    SidereonSp3 *a = load_text(a_text), *b = load_text(b_text), *merged = NULL;
    if (a == NULL || b == NULL || !merge_two(a, b, &merged) || !check_strict_source(merged))
        return fail("compatible-basis merge was not strictly writable", 42);
    sidereon_sp3_free(a); sidereon_sp3_free(b); sidereon_sp3_free(merged);
    free(a_text); free(b_text);

    // The incompatible-basis reverse order must still disclose the
    // conservative change instead of relying on the first source's basis.
    a_text = copy_text(a_bytes, a_len); b_text = copy_text(b_bytes, b_len);
    a = load_text(a_text);
    b = load_text(b_text);
    const SidereonSp3 *reverse[] = {b, a};
    SidereonSp3MergeOptions options;
    SidereonSp3MergeReport *merge_report = NULL;
    if (a == NULL || b == NULL || sidereon_sp3_merge_options_init(&options) != SIDEREON_STATUS_OK)
        return fail("load reverse incompatible pair", 43);
    options.min_agree = 1; options.clock_min_common = 1;
    if (sidereon_sp3_merge(reverse, 2, &options, &merged, &merge_report) != SIDEREON_STATUS_OK)
        return fail("reverse incompatible merge", 44);
    sidereon_sp3_merge_report_free(merge_report);
    SidereonSp3WriteResult *strict = NULL, *rounded = NULL;
    SidereonSp3WriteOutcome outcome;
    SidereonSp3AccuracyRoundingReport report = {0}; bool has_report = false;
    int ok = sidereon_sp3_to_sp3_text_result(merged, &strict) == SIDEREON_STATUS_OK &&
             sidereon_sp3_write_result_get_outcome(strict, &outcome) == SIDEREON_STATUS_OK &&
             !outcome.is_ok && outcome.error.kind == SIDEREON_SP3_WRITE_ERROR_KIND_ACCURACY_NOT_REPRESENTABLE &&
             sidereon_sp3_to_sp3_text_with_accuracy_rounding_result(merged, 0, &rounded) == SIDEREON_STATUS_OK &&
             sidereon_sp3_write_result_get_outcome(rounded, &outcome) == SIDEREON_STATUS_OK && outcome.is_ok &&
             sidereon_sp3_write_result_get_accuracy_rounding_report(rounded, &has_report, &report) == SIDEREON_STATUS_OK &&
             has_report && report.position_sigma_x_m > 0 && report.clock_sigma_m > 0 &&
             check_report_delta(rounded, merged, "G01", 0, 0, report.position_sigma_x_m) &&
             check_report_delta(rounded, merged, "G01", 0, 1, report.clock_sigma_m);
    sidereon_sp3_write_result_free(strict); sidereon_sp3_write_result_free(rounded);
    sidereon_sp3_free(a); sidereon_sp3_free(b); sidereon_sp3_free(merged);
    free(a_text); free(b_text);
    if (!ok) return fail("reverse incompatible merge lost conservative report", 45);
    return 0;
}

static int check_exponent_case(const char *a_text, const char *b_text,
                              bool expect_refusal, int report_component) {
    SidereonSp3 *a = load_text((char *)a_text), *b = load_text((char *)b_text), *merged = NULL;
    if (a == NULL || b == NULL || !check_strict_source(a) || !check_strict_source(b) ||
        !merge_two(a, b, &merged))
        return 0;
    if (expect_refusal) {
        SidereonSp3WriteResult *strict = NULL;
        SidereonSp3WriteOutcome strict_outcome;
        int strict_refused = sidereon_sp3_to_sp3_text_result(merged, &strict) == SIDEREON_STATUS_OK &&
                             sidereon_sp3_write_result_get_outcome(strict, &strict_outcome) == SIDEREON_STATUS_OK &&
                             !strict_outcome.is_ok && strict_outcome.error.kind == SIDEREON_SP3_WRITE_ERROR_KIND_ACCURACY_NOT_REPRESENTABLE;
        sidereon_sp3_write_result_free(strict);
        if (!strict_refused) { sidereon_sp3_free(a); sidereon_sp3_free(b); sidereon_sp3_free(merged); return 0; }
    }
    SidereonSp3WriteResult *result = NULL;
    SidereonSp3WriteOutcome outcome;
    int ok = sidereon_sp3_to_sp3_text_with_accuracy_rounding_result(merged, 0, &result) == SIDEREON_STATUS_OK &&
             sidereon_sp3_write_result_get_outcome(result, &outcome) == SIDEREON_STATUS_OK;
    SidereonSp3AccuracyRoundingReport report = {0}; bool has_report = false;
    if (ok && expect_refusal) {
        ok = !outcome.is_ok && outcome.error.kind == SIDEREON_SP3_WRITE_ERROR_KIND_ACCURACY_NOT_REPRESENTABLE;
    } else if (ok) {
        ok = outcome.is_ok && sidereon_sp3_write_result_get_accuracy_rounding_report(
                 result, &has_report, &report) == SIDEREON_STATUS_OK && has_report;
        double value = report_component == 0 ? report.position_sigma_x_m : report.clock_sigma_m;
        ok = ok && value > 0 && check_report_delta(result, merged, "G12", 0, report_component, value);
    }
    sidereon_sp3_write_result_free(result);
    sidereon_sp3_free(a); sidereon_sp3_free(b); sidereon_sp3_free(merged);
    return ok;
}

static int check_exponent_boundaries(const uint8_t *a_bytes, size_t a_len,
                                     const uint8_t *b_bytes, size_t b_len) {
    char *a = copy_text(a_bytes, a_len), *b = copy_text(b_bytes, b_len);
    if (a == NULL || b == NULL) return fail("copy exponent fixtures", 50);
    char *a_low_pos = copy_text(a_bytes, a_len), *b_low_pos = copy_text(b_bytes, b_len);
    char *a_low_clock = copy_text(a_bytes, a_len), *b_low_clock = copy_text(b_bytes, b_len);
    char *a_high_pos = copy_text(a_bytes, a_len), *b_high_pos = copy_text(b_bytes, b_len);
    char *a_high_clock = copy_text(a_bytes, a_len), *b_high_clock = copy_text(b_bytes, b_len);
    if (!a_low_pos || !b_low_pos || !a_low_clock || !b_low_clock || !a_high_pos || !b_high_pos || !a_high_clock || !b_high_clock)
        return fail("allocate exponent fixtures", 51);
    int valid = 1;
    // I2 lower boundary: base 4, exponent -5 must be raised to target -9.
    valid &= set_record_codes(a_low_pos, "PG01", -9, -9, -9, 1);
    valid &= replace_fixed(b_low_pos, "1.2500000  1.025000000", "4.0000000  1.025000000");
    valid &= set_record_codes(b_low_pos, "PG12", -5, -5, -5, 13);
    // I3 lower boundary: base 4, exponent -50 must be raised to target -99.
    valid &= set_record_codes(a_low_clock, "PG01", 1, 1, 1, -99);
    valid &= replace_fixed(b_low_clock, "1.2500000  1.025000000", "1.2500000  4.000000000");
    valid &= set_record_codes(b_low_clock, "PG12", 13, 13, 13, -50);
    // I2 upper: output base 1.25 cannot conservatively encode 2^98.
    valid &= replace_fixed(a_high_pos, "2.0000000  2.000000000", "1.2500000  2.000000000");
    valid &= set_record_codes(a_high_pos, "PG01", 1, 1, 1, 1);
    valid &= replace_fixed(b_high_pos, "1.2500000  1.025000000", "2.0000000  2.000000000");
    valid &= set_record_codes(b_high_pos, "PG12", 98, 98, 98, 13);
    // I3 upper: output base 1.025 cannot conservatively encode 2^998.
    valid &= replace_fixed(a_high_clock, "2.0000000  2.000000000", "2.0000000  1.025000000");
    valid &= set_record_codes(a_high_clock, "PG01", 1, 1, 1, 1);
    valid &= replace_fixed(b_high_clock, "1.2500000  1.025000000", "2.0000000  2.000000000");
    valid &= set_record_codes(b_high_clock, "PG12", 13, 13, 13, 998);
    if (!valid) return fail("rewrite exponent boundary fixtures", 52);
    int ok = check_exponent_case(a_low_pos, b_low_pos, false, 0) &&
             check_exponent_case(a_low_clock, b_low_clock, false, 1) &&
             check_exponent_case(a_high_pos, b_high_pos, true, 0) &&
             check_exponent_case(a_high_clock, b_high_clock, true, 1);
    free(a); free(b); free(a_low_pos); free(b_low_pos); free(a_low_clock); free(b_low_clock);
    free(a_high_pos); free(b_high_pos); free(a_high_clock); free(b_high_clock);
    if (!ok) return fail("I2/I3 exponent boundary behavior", 53);
    return 0;
}

static int check_sentinel_and_invalid_base(const uint8_t *a_bytes, size_t a_len,
                                           const uint8_t *b_bytes, size_t b_len) {
    char *sentinel_text = copy_text(b_bytes, b_len);
    char *invalid_text = copy_text(b_bytes, b_len);
    if (sentinel_text == NULL || invalid_text == NULL) return fail("copy sentinel fixtures", 54);
    if (!set_record_codes(sentinel_text, "PG12", 99, 99, 99, 999) ||
        !replace_fixed(invalid_text, "1.2500000  1.025000000", "0.0000000  1.025000000"))
        return fail("rewrite sentinel fixtures", 55);
    SidereonSp3 *sentinel = load_text(sentinel_text), *invalid = load_text(invalid_text);
    if (sentinel == NULL || invalid == NULL) return fail("parse sentinel fixtures", 56);
    SidereonSp3WriteResult *result = NULL;
    SidereonSp3WriteOutcome outcome;
    if (sidereon_sp3_to_sp3_text_with_accuracy_rounding_result(sentinel, 0, &result) != SIDEREON_STATUS_OK ||
        sidereon_sp3_write_result_get_outcome(result, &outcome) != SIDEREON_STATUS_OK || !outcome.is_ok)
        return fail("sentinel conservative write", 57);
    size_t written = 0, required = 0;
    uint8_t output[8192];
    if (sidereon_sp3_write_result_get_text(result, output, sizeof(output), &written, &required) != SIDEREON_STATUS_OK)
        return fail("copy sentinel output", 59);
    SidereonSp3 *reparsed = NULL;
    if (sidereon_sp3_load(output, written, &reparsed) != SIDEREON_STATUS_OK)
        return fail("reparse sentinel output", 60);
    sidereon_sp3_write_result_free(result); result = NULL;
    for (size_t epoch = 0; epoch < 2; ++epoch) {
        SidereonSp3RecordAccuracy accuracy = {0};
        if (sidereon_sp3_record_accuracy(reparsed, "G12", epoch, &accuracy) != SIDEREON_STATUS_OK ||
            accuracy.p.position_sigma_m[0].kind != SIDEREON_SP3_ACCURACY_VALUE_KIND_TOO_LARGE ||
            accuracy.p.clock_sigma_m.kind != SIDEREON_SP3_ACCURACY_VALUE_KIND_TOO_LARGE)
            return fail("sentinel meaning changed during conversion", 61);
    }
    sidereon_sp3_free(reparsed); reparsed = NULL;
    SidereonSp3RecordAccuracy invalid_accuracy = {0};
    if (sidereon_sp3_record_accuracy(invalid, "G12", 0, &invalid_accuracy) != SIDEREON_STATUS_OK ||
        invalid_accuracy.p.position_sigma_m[0].kind != SIDEREON_SP3_ACCURACY_VALUE_KIND_INVALID_BASE)
        return fail("invalid base was not retained", 62);
    SidereonSp3AccuracyRoundingReport invalid_report = {0}; bool has_invalid_report = false;
    if (sidereon_sp3_to_sp3_text_with_accuracy_rounding_result(invalid, 0, &result) != SIDEREON_STATUS_OK ||
        sidereon_sp3_write_result_get_outcome(result, &outcome) != SIDEREON_STATUS_OK || !outcome.is_ok ||
        sidereon_sp3_write_result_get_accuracy_rounding_report(result, &has_invalid_report, &invalid_report) != SIDEREON_STATUS_OK ||
        !has_invalid_report || invalid_report.position_sigma_x_m != 0 || invalid_report.clock_sigma_m != 0)
        return fail("standalone invalid base was not preserved", 63);
    if (sidereon_sp3_write_result_get_text(result, output, sizeof(output), &written, &required) != SIDEREON_STATUS_OK ||
        sidereon_sp3_load(output, written, &reparsed) != SIDEREON_STATUS_OK ||
        sidereon_sp3_record_accuracy(reparsed, "G12", 0, &invalid_accuracy) != SIDEREON_STATUS_OK ||
        invalid_accuracy.p.position_sigma_m[0].kind != SIDEREON_SP3_ACCURACY_VALUE_KIND_INVALID_BASE)
        return fail("standalone invalid-base meaning changed", 64);
    if (sidereon_sp3_record_accuracy(invalid, "G12", 0, &invalid_accuracy) != SIDEREON_STATUS_OK ||
        invalid_accuracy.p.position_sigma_m[0].kind != SIDEREON_SP3_ACCURACY_VALUE_KIND_INVALID_BASE)
        return fail("standalone invalid-base input changed", 69);
    sidereon_sp3_write_result_free(result); result = NULL;
    char *known_text = copy_text(a_bytes, a_len);
    SidereonSp3 *known = load_text(known_text);
    if (known == NULL) return fail("parse known-base merge source", 65);
    for (int reverse = 0; reverse <= 1; ++reverse) {
        const SidereonSp3 *sources[] = {reverse ? known : invalid, reverse ? invalid : known};
        SidereonSp3 *merged = NULL;
        SidereonSp3MergeReport *merge_report = NULL;
        SidereonSp3MergeOptions options;
        if (sidereon_sp3_merge_options_init(&options) != SIDEREON_STATUS_OK)
            return fail("initialize InvalidBase merge", 66);
        options.min_agree = 1; options.clock_min_common = 1;
        if (sidereon_sp3_merge(sources, 2, &options, &merged, &merge_report) != SIDEREON_STATUS_OK)
            return fail("merge InvalidBase source", 67);
        sidereon_sp3_merge_report_free(merge_report);
        if (sidereon_sp3_to_sp3_text_with_accuracy_rounding_result(merged, 0, &result) != SIDEREON_STATUS_OK ||
            sidereon_sp3_write_result_get_outcome(result, &outcome) != SIDEREON_STATUS_OK || outcome.is_ok ||
            outcome.error.kind != SIDEREON_SP3_WRITE_ERROR_KIND_ACCURACY_NOT_REPRESENTABLE)
            return fail("mixed invalid/known bases were silently converted", 68);
        sidereon_sp3_write_result_free(result); result = NULL;
        sidereon_sp3_free(merged);
    }
    sidereon_sp3_free(known); free(known_text);
    sidereon_sp3_free(sentinel); sidereon_sp3_free(invalid);
    sidereon_sp3_free(reparsed); free(sentinel_text); free(invalid_text);
    return 0;
}

static int check_direct_rounding(const SidereonSp3 *sp3) {
    SidereonSp3WriteResult *result = NULL;
    SidereonSp3WriteOutcome outcome;
    SidereonSp3AccuracyRoundingReport report = {0};
    bool has_report = false;
    int ok = sidereon_sp3_to_sp3_text_with_accuracy_rounding_result(
                 sp3, SIDEREON_SP3_ACCURACY_ROUNDING_POLICY_CONSERVATIVE, &result) ==
                 SIDEREON_STATUS_OK &&
             result != NULL &&
             sidereon_sp3_write_result_get_outcome(result, &outcome) == SIDEREON_STATUS_OK &&
             outcome.is_ok &&
             sidereon_sp3_write_result_get_accuracy_rounding_report(
                 result, &has_report, &report) == SIDEREON_STATUS_OK &&
             has_report && report.position_sigma_x_m == 0 && report.position_sigma_y_m == 0 &&
             report.position_sigma_z_m == 0 && report.velocity_sigma_x_m_s == 0 &&
             report.velocity_sigma_y_m_s == 0 && report.velocity_sigma_z_m_s == 0 &&
             report.clock_sigma_m == 0 && report.clock_rate_sigma_m_s == 0;
    sidereon_sp3_write_result_free(result);
    return ok;
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
    if (!check_strict_source(a) || !check_strict_source(b))
        return fail("source strict writer", 30);

    SidereonSp3MergeOptions options;
    if (sidereon_sp3_merge_options_init(&options) != SIDEREON_STATUS_OK)
        return fail("initialize unbased merge options", 22);
    options.combine = SIDEREON_SP3_MERGE_COMBINE_PRECEDENCE;
    options.precedence_scope = SIDEREON_SP3_MERGE_PRECEDENCE_SCOPE_CELL;
    options.min_agree = 1;
    options.clock_min_common = 1;
    options.position_tolerance_m = 5.0;
    options.clock_tolerance_s = 50e-9;
    options.outlier_reject_enabled = 1;
    options.outlier_reject_position_tolerance_m = 5.0;
    options.outlier_reject_clock_tolerance_s = 50e-9;
    options.target_epoch_interval_s_enabled = 1;
    options.target_epoch_interval_s = 300.0;
    uint32_t gps = SIDEREON_GNSS_SYSTEM_GPS;
    options.systems = &gps;
    options.system_count = 1;
    const SidereonSp3 *sources[] = {reverse ? b : a, reverse ? a : b};
    SidereonSp3 *merged = NULL;
    SidereonSp3MergeReport *report = NULL;
    if (sidereon_sp3_merge(sources, 2, &options, &merged, &report) != SIDEREON_STATUS_OK)
        return fail("merge unbased pair", 23);
    sidereon_sp3_merge_report_free(report);

    for (size_t epoch = 0; epoch < 2; ++epoch) {
        SidereonSp3RecordAccuracy accuracy = {0};
        if (sidereon_sp3_record_accuracy(merged, "G12", epoch, &accuracy) != SIDEREON_STATUS_OK ||
            !accuracy.has_p || accuracy.p.position_sigma_m[0].kind != SIDEREON_SP3_ACCURACY_VALUE_KIND_KNOWN ||
            fabs(accuracy.p.position_sigma_m[0].value - pow(1.25, 13.0) * 1e-3) > 1e-15)
            return fail("unbased pair did not preserve G12 decoded sigma", 24);
        if (sidereon_sp3_record_accuracy(merged, "G01", epoch, &accuracy) != SIDEREON_STATUS_OK ||
            !accuracy.has_p || accuracy.p.position_sigma_m[0].kind != SIDEREON_SP3_ACCURACY_VALUE_KIND_UNKNOWN)
            return fail("unbased pair did not preserve G01 unknown accuracy", 26);
    }
    SidereonSp3WriteResult *strict = NULL;
    SidereonSp3WriteOutcome outcome;
    if (sidereon_sp3_to_sp3_text_result(merged, &strict) != SIDEREON_STATUS_OK || strict == NULL ||
        sidereon_sp3_write_result_get_outcome(strict, &outcome) != SIDEREON_STATUS_OK ||
        !outcome.is_ok)
        return fail("unbased pair strict write", 25);
    sidereon_sp3_write_result_free(strict);
    sidereon_sp3_free(merged);
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
    if (!check_strict_source(a) || !check_strict_source(b))
        return fail("accuracy source strict writer", 32);
    if (!check_direct_rounding(b))
        return fail("direct all-field accuracy rounding did not preserve the source grid", 33);
    if (check_basis_matrix(a_bytes, a_len, b_bytes, b_len) != 0 ||
        check_exponent_boundaries(a_bytes, a_len, b_bytes, b_len) != 0 ||
        check_sentinel_and_invalid_base(a_bytes, a_len, b_bytes, b_len) != 0)
        return 34;

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
    const double source_clock_sigma = pow(1.025, 13.0) * 299792458e-12;
    const double expected_clock_delta = 2.0 * 299792458e-12 - source_clock_sigma;
    if (fabs(accuracy.position_sigma_x_m - expected_delta) > 1e-15 ||
        fabs(accuracy.position_sigma_y_m - expected_delta) > 1e-15 ||
        fabs(accuracy.position_sigma_z_m - expected_delta) > 1e-15 ||
        accuracy.velocity_sigma_x_m_s != 0 || accuracy.velocity_sigma_y_m_s != 0 ||
        accuracy.velocity_sigma_z_m_s != 0 ||
        fabs(accuracy.clock_sigma_m - expected_clock_delta) > 1e-15 ||
        accuracy.clock_rate_sigma_m_s != 0)
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
    sidereon_sp3_free(a);
    sidereon_sp3_free(b);
    if (check_unbased_pair(argv[3], argv[4], false) != 0 ||
        check_unbased_pair(argv[3], argv[4], true) != 0)
        return 14;
    puts("sp3_accuracy_rounding_smoke: all checks passed");
    return 0;
}
