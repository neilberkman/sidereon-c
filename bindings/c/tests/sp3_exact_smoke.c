/*
 * Focused C-ABI coverage for exact-SP3 request construction, semantic
 * validation, coverage reporting, and raw declared-header accessors, plus a
 * refused SP3 write and the agreement a clock-only merge reports.
 */
#include "sidereon.h"

#include <math.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sp3_terminal_record_fixture.h"
#include "w5_sp3_exact_pins.h"

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

/* Exact agreement with a pinned engine value (tests/valgen, bin
 * w5_sp3_exact). */
static int same_bits(double value, uint64_t want) {
    uint64_t bits;
    memcpy(&bits, &value, sizeof(bits));
    return bits == want;
}

static int fail(const char *context, int code) {
    char message[512];
    sidereon_last_error_message(message, sizeof(message));
    fprintf(stderr, "FAIL %d: %s: %s\n", code, context, message);
    return code;
}

static int append_format(
    char *buffer, size_t capacity, size_t *length, const char *format, ...) {
    if (*length >= capacity) {
        return 0;
    }
    va_list args;
    va_start(args, format);
    int written = vsnprintf(buffer + *length, capacity - *length, format, args);
    va_end(args);
    if (written < 0 || (size_t)written >= capacity - *length) {
        return 0;
    }
    *length += (size_t)written;
    return 1;
}

static uint8_t *historical_gfz_ultra_sp3(size_t *out_len) {
    const size_t capacity = 160000;
    char *text = (char *)malloc(capacity);
    if (text == NULL) {
        return NULL;
    }
    size_t length = 0;
    if (!append_format(
            text, capacity, &length,
            "#dP%4d %2d %2d %2d %2d %11.8f %7d %-5s%6s%4s %s\n",
            2022, 9, 3, 0, 0, 0.0, 576, "ORBIT", "IGS20", "FIT", "GFZ") ||
        !append_format(
            text, capacity, &length, "## %4d %15.8f %14s %5d %.13f\n",
            2225, 518400.0, "300.00000000", 59825, 0.0) ||
        !append_format(text, capacity, &length, "+    2   G01G02")) {
        free(text);
        return NULL;
    }
    for (size_t index = 2; index < 17; ++index) {
        if (!append_format(text, capacity, &length, "  0")) {
            free(text);
            return NULL;
        }
    }
    if (!append_format(text, capacity, &length, "\n")) {
        free(text);
        return NULL;
    }
    for (size_t line = 1; line < 5; ++line) {
        if (!append_format(text, capacity, &length, "+        ")) {
            free(text);
            return NULL;
        }
        for (size_t index = 0; index < 17; ++index) {
            if (!append_format(text, capacity, &length, "  0")) {
                free(text);
                return NULL;
            }
        }
        if (!append_format(text, capacity, &length, "\n")) {
            free(text);
            return NULL;
        }
    }
    for (size_t line = 0; line < 5; ++line) {
        if (!append_format(text, capacity, &length, "++       ")) {
            free(text);
            return NULL;
        }
        for (size_t index = 0; index < 17; ++index) {
            if (!append_format(text, capacity, &length, "  0")) {
                free(text);
                return NULL;
            }
        }
        if (!append_format(text, capacity, &length, "\n")) {
            free(text);
            return NULL;
        }
    }
    static const char header_tail[] =
        "%c M  cc GPS ccc cccc cccc cccc cccc ccccc ccccc ccccc ccccc\n"
        "%c cc cc ccc ccc cccc cccc cccc cccc ccccc ccccc ccccc ccccc\n"
        "%f  1.2500000  1.025000000  0.00000000000  0.000000000000000\n"
        "%f  0.0000000  0.000000000  0.00000000000  0.000000000000000\n"
        "%i    0    0    0    0      0      0      0      0         0\n"
        "%i    0    0    0    0      0      0      0      0         0\n"
        "/* EXACT VALIDATION TEST FIXTURE\n"
        "/* EXACT VALIDATION TEST FIXTURE\n"
        "/* EXACT VALIDATION TEST FIXTURE\n"
        "/* EXACT VALIDATION TEST FIXTURE\n";
    if (!append_format(text, capacity, &length, "%s", header_tail)) {
        free(text);
        return NULL;
    }

    for (size_t index = 0; index < 576; ++index) {
        size_t offset_s = index * 300;
        size_t day = 3 + offset_s / 86400;
        size_t second_of_day = offset_s % 86400;
        size_t hour = second_of_day / 3600;
        size_t minute = (second_of_day % 3600) / 60;
        size_t second = second_of_day % 60;
        if (!append_format(
                text, capacity, &length,
                "*  %4d %2d %2zu %2zu %2zu %11.8f\n"
                "PG01  15000.000000 -20000.000000   5000.000000    123.456789\n"
                "PG02  16000.000000 -21000.000000   6000.000000    124.456789\n",
                2022, 9, day, hour, minute, (double)second)) {
            free(text);
            return NULL;
        }
    }
    if (!append_format(text, capacity, &length, "EOF\n")) {
        free(text);
        return NULL;
    }
    *out_len = length;
    return (uint8_t *)text;
}

static const char *terminal_result_class(enum SidereonStatus status, const char *message) {
    if (status == SIDEREON_STATUS_OK) {
        return "accept";
    }
    if (status != SIDEREON_STATUS_INVALID_ARGUMENT) {
        return "unexpected_status";
    }
    if (strstr(message, "malformed EOF record") != NULL) {
        return "malformed_eof_record";
    }
    if (strstr(message, "missing its EOF record") != NULL) {
        return "missing_eof";
    }
    if (strstr(message, "nonblank records after EOF") != NULL) {
        return "trailing_content_after_eof";
    }
    return "unrelated_exact_error";
}

static int run_terminal_record_corpus(
    const uint8_t *valid,
    size_t valid_len,
    const struct SidereonExactSp3Request *request
) {
    static const uint8_t canonical_terminal[] = {'E', 'O', 'F', '\n'};
    if (valid_len < sizeof(canonical_terminal) ||
        memcmp(
            valid + valid_len - sizeof(canonical_terminal),
            canonical_terminal,
            sizeof(canonical_terminal)) != 0) {
        fprintf(stderr, "terminal-record corpus base fixture lacks canonical EOF\\n\n");
        return 0;
    }
    const size_t prefix_len = valid_len - sizeof(canonical_terminal);
    if (SP3_TERMINAL_RECORD_CASE_COUNT != W5_SP3_EXACT_TERMINAL_CASE_COUNT) {
        fprintf(stderr, "terminal-record corpus case count differs from the engine pins\n");
        return 0;
    }

    for (size_t index = 0; index < SP3_TERMINAL_RECORD_CASE_COUNT; index++) {
        const struct Sp3TerminalRecordCase *test_case = &SP3_TERMINAL_RECORD_CASES[index];
        if (test_case->terminal_len > SIZE_MAX - prefix_len) {
            fprintf(stderr, "terminal-record corpus case %s is too large\n", test_case->name);
            return 0;
        }
        const size_t candidate_len = prefix_len + test_case->terminal_len;
        uint8_t *candidate = (uint8_t *)malloc(candidate_len);
        if (candidate == NULL) {
            fprintf(stderr, "terminal-record corpus allocation failed for %s\n", test_case->name);
            return 0;
        }
        memcpy(candidate, valid, prefix_len);
        memcpy(candidate + prefix_len, test_case->terminal_bytes, test_case->terminal_len);

        struct SidereonSp3 *product = (struct SidereonSp3 *)(uintptr_t)1;
        enum SidereonExactSp3Coverage coverage = SIDEREON_EXACT_SP3_COVERAGE_INCLUSIVE;
        enum SidereonStatus status = sidereon_sp3_load_exact(
            candidate, candidate_len, request, &product, &coverage);
        char message[512] = {0};
        if (status != SIDEREON_STATUS_OK) {
            sidereon_last_error_message(message, sizeof(message));
        }
        const char *actual = terminal_result_class(status, message);
        /* The corpus states each case's expected class
         * (fixtures/sp3-terminal-record-v1.json, "expect"); the engine's own
         * outcome for the same bytes is pinned alongside it. */
        int valid_result = strcmp(actual, test_case->expected) == 0 &&
                           strcmp(test_case->name, W5_SP3_EXACT_TERMINAL_NAMES[index]) == 0 &&
                           (status == SIDEREON_STATUS_OK) == W5_SP3_EXACT_TERMINAL_OK[index];
        if (status != SIDEREON_STATUS_OK) {
            valid_result = valid_result &&
                           strstr(message, W5_SP3_EXACT_TERMINAL_ERRORS[index]) != NULL;
        }
        if (status == SIDEREON_STATUS_OK) {
            valid_result = valid_result && product != NULL &&
                           product != (struct SidereonSp3 *)(uintptr_t)1 &&
                           (coverage == SIDEREON_EXACT_SP3_COVERAGE_HALF_OPEN) ==
                               W5_SP3_EXACT_TERMINAL_HALF_OPEN[index];
            if (product != NULL && product != (struct SidereonSp3 *)(uintptr_t)1) {
                sidereon_sp3_free(product);
            }
        } else {
            valid_result = valid_result && product == NULL;
            if (product != NULL && product != (struct SidereonSp3 *)(uintptr_t)1) {
                sidereon_sp3_free(product);
            }
        }
        free(candidate);

        if (!valid_result) {
            fprintf(
                stderr,
                "terminal-record corpus case %s: expected %s, got %s (%s)\n",
                test_case->name,
                test_case->expected,
                actual,
                message);
            return 0;
        }
    }
    return 1;
}

/* An SP3-c product whose first %f base, 1.25000001, carries more digits than
 * the F10.7 field states. The reader keeps the value the source spelled; the
 * writer cannot restate it and refuses. */
static const char FINE_BASE_SP3[] =
    "#cP2020  6 24  0  0  0.00000000       2 ORBIT IGS14 FIT  TST\n"
    "## 2111 432000.00000000   900.00000000 59024 0.0000000000000\n"
    "+    2   G01G02  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0\n"
    "++         0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0\n"
    "%c G  cc GPS ccc cccc cccc cccc cccc ccccc ccccc ccccc ccccc\n"
    "%c cc cc ccc ccc cccc cccc cccc cccc ccccc ccccc ccccc ccccc\n"
    "%f 1.25000001  1.025000000  0.00000000000  0.000000000000000\n"
    "%f  0.0000000  0.000000000  0.00000000000  0.000000000000000\n"
    "%i    0    0    0    0      0      0      0      0         0\n"
    "%i    0    0    0    0      0      0      0      0         0\n"
    "/* TEST SP3-c FIXTURE\n"
    "*  2020  6 24  0  0  0.00000000\n"
    "PG01  15000.000000 -20000.000000   5000.000000    123.456789\n"
    "PG02  -1234.567890   2345.678901  -3456.789012 999999.999999\n"
    "*  2020  6 24  0 15  0.00000000\n"
    "PG01  15100.000000 -20100.000000   5100.000000   -987.654321              E\n"
    "PG02      0.000000      0.000000      0.000000    100.000000\n"
    "EOF\n";

/* One epoch whose only record carries a clock and no orbit. */
static const char CLOCK_ONLY_SP3[] =
    "#cP2020  6 24  0  0  0.00000000       1 ORBIT IGS14 FIT  TST\n"
    "## 2111 259200.00000000   900.00000000 59024 0.0000000000000\n"
    "+    1   G01  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0\n"
    "++         5  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0\n"
    "%c G  cc GPS ccc cccc cccc cccc cccc ccccc ccccc ccccc ccccc\n"
    "%c cc cc ccc ccc cccc cccc cccc cccc ccccc ccccc ccccc ccccc\n"
    "%f  1.2500000  1.025000000  0.00000000000  0.000000000000000\n"
    "%f  0.0000000  0.000000000  0.00000000000  0.000000000000000\n"
    "%i    0    0    0    0      0      0      0      0         0\n"
    "%i    0    0    0    0      0      0      0      0         0\n"
    "*  2020  6 24  0  0  0.00000000\n"
    "PG01      0.000000      0.000000      0.000000    123.456789\n"
    "EOF\n";

static int run_sp3_writer_refusal(void) {
    struct SidereonSp3 *sp3 = NULL;
    if (sidereon_sp3_load((const uint8_t *)FINE_BASE_SP3, sizeof(FINE_BASE_SP3) - 1, &sp3) !=
            SIDEREON_STATUS_OK ||
        sp3 == NULL) {
        return fail("fine-base SP3 load", 30);
    }
    size_t written = 99;
    size_t required = 99;
    if (sidereon_sp3_to_sp3_text(sp3, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        written != 0 || required != 0) {
        sidereon_sp3_free(sp3);
        return fail("SP3 writer refuses a base its field cannot state", 31);
    }

    struct SidereonSp3WriteResult *result = NULL;
    if (sidereon_sp3_to_sp3_text_result(sp3, &result) != SIDEREON_STATUS_OK || result == NULL) {
        sidereon_sp3_free(sp3);
        return fail("SP3 writer result route", 32);
    }
    sidereon_sp3_free(sp3);

    struct SidereonSp3WriteOutcome outcome;
    /* The engine refuses the write with PrecisionNotRepresentable and these
     * fields (pinned); the binding's typed mirror carries them
     * (src/sp3.rs, sp3_write_refusal). */
    int ok = sidereon_sp3_write_result_get_outcome(result, &outcome) == SIDEREON_STATUS_OK &&
             !W5_SP3_EXACT_FINE_BASE_WRITE_OK &&
             strcmp(W5_SP3_EXACT_FINE_BASE_WRITE_ERROR_VARIANT, "PrecisionNotRepresentable") == 0 &&
             !outcome.is_ok && outcome.status == SIDEREON_STATUS_INVALID_ARGUMENT &&
             outcome.error.kind == SIDEREON_SP3_WRITE_ERROR_KIND_PRECISION_NOT_REPRESENTABLE &&
             outcome.error.has_field && !outcome.error.has_text_value &&
             outcome.error.has_columns && outcome.error.columns == W5_SP3_EXACT_FINE_BASE_COLUMNS &&
             outcome.error.has_decimals &&
             outcome.error.decimals == W5_SP3_EXACT_FINE_BASE_DECIMALS &&
             outcome.error.has_number &&
             same_bits(outcome.error.number, W5_SP3_EXACT_FINE_BASE_NUMBER_BITS) &&
             !outcome.error.has_sat_id && !outcome.error.has_epoch_index &&
             isnan(outcome.error.stored);
    char field[32] = {0};
    ok = ok &&
         sidereon_sp3_write_result_get_field(result, (uint8_t *)field, sizeof(field) - 1,
                                             &written, &required) == SIDEREON_STATUS_OK &&
         written == required && strcmp(field, W5_SP3_EXACT_FINE_BASE_FIELD) == 0;
    written = 99;
    required = 99;
    ok = ok &&
         sidereon_sp3_write_result_get_text(result, NULL, 0, &written, &required) ==
             SIDEREON_STATUS_INVALID_ARGUMENT &&
         written == 0 && required == 0;
    ok = ok &&
         sidereon_sp3_write_result_get_message(result, NULL, 0, &written, &required) ==
             SIDEREON_STATUS_OK &&
         required > 0;
    sidereon_sp3_write_result_free(result);
    sidereon_sp3_write_result_free(NULL);
    return ok ? 0 : fail("SP3 writer refusal is typed with every field", 33);
}

static int run_clock_only_merge_agreement(void) {
    struct SidereonSp3 *sp3 = NULL;
    if (sidereon_sp3_load((const uint8_t *)CLOCK_ONLY_SP3, sizeof(CLOCK_ONLY_SP3) - 1, &sp3) !=
            SIDEREON_STATUS_OK ||
        sp3 == NULL) {
        return fail("clock-only SP3 load", 34);
    }
    struct SidereonSp3MergeOptions options;
    if (sidereon_sp3_merge_options_init(&options) != SIDEREON_STATUS_OK) {
        sidereon_sp3_free(sp3);
        return fail("merge options init", 35);
    }
    options.min_agree = 1;
    options.clock_min_common = 1;
    const struct SidereonSp3 *sources[2] = {sp3, sp3};
    struct SidereonSp3 *merged = NULL;
    struct SidereonSp3MergeReport *report = NULL;
    if (sidereon_sp3_merge(sources, 2, &options, &merged, &report) != SIDEREON_STATUS_OK) {
        sidereon_sp3_free(sp3);
        return fail("clock-only merge", 36);
    }

    size_t count = 0;
    struct SidereonSp3EpochAgreement agreement;
    struct SidereonSp3AgreementSummary summary;
    /* No accepted cell carries an orbit, so no position spread exists. The two
     * clocks agree exactly, which is a measured zero. */
    int ok = sidereon_sp3_merge_report_epoch_agreement_count(report, &count) ==
                 SIDEREON_STATUS_OK &&
             count == W5_SP3_EXACT_MERGE_EPOCH_AGREEMENT_COUNT &&
             sidereon_sp3_merge_report_epoch_agreement(report, 0, &agreement) ==
                 SIDEREON_STATUS_OK &&
             agreement.satellites == W5_SP3_EXACT_MERGE_SATELLITES &&
             agreement.position_rms_present == W5_SP3_EXACT_MERGE_POSITION_RMS_PRESENT &&
             agreement.position_max_present == W5_SP3_EXACT_MERGE_POSITION_MAX_PRESENT &&
             agreement.clock_rms_present == W5_SP3_EXACT_MERGE_CLOCK_RMS_PRESENT &&
             same_bits(agreement.clock_rms_s, W5_SP3_EXACT_MERGE_CLOCK_RMS_S_BITS) &&
             agreement.clock_max_present == W5_SP3_EXACT_MERGE_CLOCK_MAX_PRESENT &&
             same_bits(agreement.clock_max_s, W5_SP3_EXACT_MERGE_CLOCK_MAX_S_BITS) &&
             /* An absent value reads as NaN (src/sp3.rs, sp3_epoch_agreement_to_c). */
             (agreement.position_rms_present || isnan(agreement.position_rms_m)) &&
             (agreement.position_max_present || isnan(agreement.position_max_m));
    ok = ok &&
         sidereon_sp3_merge_report_agreement_summary(report, &summary) == SIDEREON_STATUS_OK &&
         summary.position_rms_present == W5_SP3_EXACT_SUMMARY_POSITION_RMS_PRESENT &&
         summary.position_max_present == W5_SP3_EXACT_SUMMARY_POSITION_MAX_PRESENT &&
         (summary.position_rms_present || isnan(summary.position_rms_m)) &&
         (summary.position_max_present || isnan(summary.position_max_m)) &&
         summary.clock_rms_present == W5_SP3_EXACT_SUMMARY_CLOCK_RMS_PRESENT &&
         same_bits(summary.clock_rms_s, W5_SP3_EXACT_SUMMARY_CLOCK_RMS_S_BITS) &&
         summary.clock_max_present == W5_SP3_EXACT_SUMMARY_CLOCK_MAX_PRESENT &&
         same_bits(summary.clock_max_s, W5_SP3_EXACT_SUMMARY_CLOCK_MAX_S_BITS);
    sidereon_sp3_merge_report_free(report);
    sidereon_sp3_free(merged);
    sidereon_sp3_free(sp3);
    return ok ? 0 : fail("clock-only merge reports position agreement absent", 37);
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
        sp3 == NULL || !W5_SP3_EXACT_GRG_LOAD_OK || coverage != W5_SP3_EXACT_GRG_LOAD_COVERAGE) {
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
        parsed_count != W5_SP3_EXACT_GRG_EPOCH_COUNT ||
        declared_count != W5_SP3_EXACT_GRG_DECLARED_EPOCH_COUNT ||
        (declared_start_present == 1) != W5_SP3_EXACT_GRG_DECLARED_START_PRESENT ||
        !same_bits(declared_start, W5_SP3_EXACT_GRG_DECLARED_START_J2000_S_BITS)) {
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
        epochs_written != parsed_count ||
        !same_bits(epochs[0], W5_SP3_EXACT_GRG_FIRST_EPOCH_J2000_S_BITS)) {
        free(epochs);
        sidereon_sp3_free(sp3);
        sidereon_sp3_exact_request_free(request);
        free(valid);
        free(truncated);
        return fail("declared start matches first epoch", 8);
    }
    free(epochs);

    coverage = SIDEREON_EXACT_SP3_COVERAGE_INCLUSIVE;
    if (!W5_SP3_EXACT_GRG_VALIDATE_OK ||
        sidereon_sp3_validate_exact(sp3, request, &coverage) != SIDEREON_STATUS_OK ||
        coverage != W5_SP3_EXACT_GRG_VALIDATE_COVERAGE) {
        sidereon_sp3_free(sp3);
        sidereon_sp3_exact_request_free(request);
        free(valid);
        free(truncated);
        return fail("exact validation", 9);
    }
    sidereon_sp3_free(sp3);
    if (!run_terminal_record_corpus(valid, valid_len, request)) {
        sidereon_sp3_exact_request_free(request);
        free(valid);
        free(truncated);
        return fail("shared terminal-record corpus", 18);
    }
    sidereon_sp3_exact_request_free(request);

    struct SidereonExactSp3Request *bad_request = NULL;
    /* Each request's acceptance is the engine's (ExactSp3Request::new and
     * from_identity, pinned); the binding reports a refusal as
     * INVALID_ARGUMENT (src/sp3.rs, map_exact_sp3_error). */
    if (W5_SP3_EXACT_REQUEST_24H_OK ||
        sidereon_sp3_exact_request_new(
            2020, 6, 24, NULL, "01D", "24H", NULL, &bad_request) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        bad_request != NULL) {
        free(valid);
        free(truncated);
        return fail("noncanonical cadence rejection", 10);
    }
    if (!W5_SP3_EXACT_REQUEST_07D_OK ||
        sidereon_sp3_exact_request_new(
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
        !W5_SP3_EXACT_REQUEST_IGS_SP3_IDENTITY_OK ||
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
        W5_SP3_EXACT_REQUEST_IGS_NAV_IDENTITY_OK ||
        sidereon_sp3_exact_request_from_identity(&identity, &request) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        request != NULL) {
        free(valid);
        free(truncated);
        return fail("non-SP3 request rejection", 13);
    }

    size_t historical_len = 0;
    uint8_t *historical = historical_gfz_ultra_sp3(&historical_len);
    if (historical == NULL ||
        sidereon_data_product_identity(
            "gfz_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2022, 9, 4, "05M",
            "0000", &identity) != SIDEREON_STATUS_OK ||
        sidereon_sp3_exact_request_from_identity(&identity, &request) !=
            SIDEREON_STATUS_OK ||
        request == NULL) {
        free(historical);
        free(valid);
        free(truncated);
        return fail("historical GFZ request from identity", 19);
    }
    sp3 = NULL;
    coverage = SIDEREON_EXACT_SP3_COVERAGE_INCLUSIVE;
    if (sidereon_sp3_load_exact(
            historical, historical_len, request, &sp3, &coverage) !=
            SIDEREON_STATUS_OK ||
        sp3 == NULL || !W5_SP3_EXACT_GFZ_IDENTITY_LOAD_OK ||
        coverage != W5_SP3_EXACT_GFZ_IDENTITY_LOAD_COVERAGE) {
        sidereon_sp3_free(sp3);
        sidereon_sp3_exact_request_free(request);
        free(historical);
        free(valid);
        free(truncated);
        return fail("historical GFZ cataloged content start", 20);
    }
    sidereon_sp3_free(sp3);
    sidereon_sp3_exact_request_free(request);

    if (sidereon_sp3_exact_request_new(
            2022, 9, 4, "0000", "02D", "05M", "GFZ", &request) !=
            SIDEREON_STATUS_OK ||
        request == NULL) {
        free(historical);
        free(valid);
        free(truncated);
        return fail("historical GFZ literal request", 21);
    }
    sp3 = (struct SidereonSp3 *)(uintptr_t)1;
    if (W5_SP3_EXACT_GFZ_LITERAL_LOAD_OK ||
        strcmp(W5_SP3_EXACT_GFZ_LITERAL_LOAD_ERROR_VARIANT, "Parse") == 0 ||
        sidereon_sp3_load_exact(
            historical, historical_len, request, &sp3, &coverage) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        sp3 != NULL) {
        if (sp3 != NULL && sp3 != (struct SidereonSp3 *)(uintptr_t)1) {
            sidereon_sp3_free(sp3);
        }
        sidereon_sp3_exact_request_free(request);
        free(historical);
        free(valid);
        free(truncated);
        return fail("literal filename epoch remains strict", 22);
    }
    sidereon_sp3_exact_request_free(request);
    free(historical);

    struct SidereonSp3 *permissive = NULL;
    if (sidereon_sp3_load(truncated, truncated_len, &permissive) != SIDEREON_STATUS_OK ||
        permissive == NULL ||
        sidereon_sp3_epoch_count(permissive, &parsed_count) != SIDEREON_STATUS_OK ||
        sidereon_sp3_declared_epoch_count(permissive, &declared_count) !=
            SIDEREON_STATUS_OK ||
        parsed_count != W5_SP3_EXACT_TRUNCATED_EPOCH_COUNT ||
        declared_count != W5_SP3_EXACT_TRUNCATED_DECLARED_EPOCH_COUNT) {
        sidereon_sp3_free(permissive);
        free(valid);
        free(truncated);
        return fail("permissive declared-count evidence", 14);
    }
    if (sidereon_sp3_exact_request_new(
            2026, 4, 30, NULL, "01D", "15M", "IGS", &request) !=
            SIDEREON_STATUS_OK ||
        W5_SP3_EXACT_TRUNCATED_VALIDATE_OK ||
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
    /* The engine refuses the text as a parse error; the binding reports a
     * parse refusal as SP3_PARSE (src/sp3.rs, map_exact_sp3_error). */
    if (W5_SP3_EXACT_MALFORMED_LOAD_OK ||
        strcmp(W5_SP3_EXACT_MALFORMED_LOAD_ERROR_VARIANT, "Parse") != 0 ||
        sidereon_sp3_load_exact(
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

    int rc = run_sp3_writer_refusal();
    if (rc != 0) {
        return rc;
    }
    return run_clock_only_merge_agreement();
}
