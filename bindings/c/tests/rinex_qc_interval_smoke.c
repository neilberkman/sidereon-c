/*
 * Regression coverage for an INTERVAL value of zero, which RINEX permits to
 * mean that the optional cadence metadata is unavailable. Source metadata is
 * reported and ignored for cadence selection; invalid caller overrides remain
 * errors.
 */
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sidereon.h"
#include "w6_rinex_qc_interval_pins.h"

/* Every expected value below is sidereon-core's own result for the texts this
 * program builds, from tests/valgen (w6_rinex_qc_interval): lint findings,
 * observation QC summaries and JSON, and the INTERVAL record each repair
 * writes. */

static void require(bool condition, const char *message) {
    if (!condition) {
        char error[512] = {0};
        (void)sidereon_last_error_message(error, sizeof(error));
        fprintf(stderr, "FAIL: %s (last_error: %s)\n", message, error);
        exit(1);
    }
}

static uint8_t *read_file(const char *path, size_t *out_len) {
    FILE *file = fopen(path, "rb");
    require(file != NULL, "open observation fixture");
    require(fseek(file, 0, SEEK_END) == 0, "seek observation fixture");
    long length = ftell(file);
    require(length >= 0, "measure observation fixture");
    rewind(file);

    uint8_t *bytes = (uint8_t *)malloc((size_t)length + 1);
    require(bytes != NULL, "allocate observation fixture");
    require(fread(bytes, 1, (size_t)length, file) == (size_t)length,
            "read observation fixture");
    require(fclose(file) == 0, "close observation fixture");
    bytes[length] = 0;
    *out_len = (size_t)length;
    return bytes;
}

static uint8_t *copy_with_interval_record(const uint8_t *source, size_t len,
                                          const char *replacement) {
    static const char valid[] = "    30.000                                                  INTERVAL";
    require(strlen(replacement) == sizeof(valid) - 1,
            "replacement must preserve INTERVAL record length");

    uint8_t *copy = (uint8_t *)malloc(len + 1);
    require(copy != NULL, "allocate changed-interval fixture");
    memcpy(copy, source, len);
    copy[len] = 0;

    char *record = strstr((char *)copy, valid);
    require(record != NULL, "find INTERVAL record");
    memcpy(record, replacement, sizeof(valid) - 1);
    return copy;
}

static size_t one_epoch_length(const uint8_t *bytes, size_t len) {
    size_t first = SIZE_MAX;
    for (size_t i = 0; i + 2 < len; i++) {
        if (bytes[i] == '\n' && bytes[i + 1] == '>' && bytes[i + 2] == ' ') {
            if (first == SIZE_MAX) {
                first = i;
            } else {
                /* Retain the newline ending the first epoch's final data row. */
                return i + 1;
            }
        }
    }
    require(false, "find two observation epochs");
    return 0;
}

/* The lint finding with `code` in `bytes`: whether it is present and, when it
 * is, its severity and repairability. */
static void check_lint(const uint8_t *bytes, size_t len, const char *code, bool expect_present,
                       uint32_t expect_severity, bool expect_repairable, const char *message) {
    SidereonRinexLintReport *report = NULL;
    require(sidereon_rinex_lint_obs(bytes, len, &report) == SIDEREON_STATUS_OK &&
                report != NULL,
            "lint observation bytes");

    size_t written = 0;
    size_t required = 0;
    require(sidereon_rinex_lint_findings(report, NULL, 0, &written, &required) ==
                SIDEREON_STATUS_OK &&
                written == 0,
            "measure lint findings");
    SidereonRinexLintFinding *findings =
        (SidereonRinexLintFinding *)calloc(required, sizeof(*findings));
    require(required == 0 || findings != NULL, "allocate lint findings");
    require(sidereon_rinex_lint_findings(report, findings, required, &written, &required) ==
                SIDEREON_STATUS_OK &&
                written == required,
            "copy lint findings");

    bool found = false;
    for (size_t i = 0; i < written; i++) {
        if (strncmp((const char *)findings[i].code, code, sizeof(findings[i].code)) == 0) {
            found = true;
            require(findings[i].severity == expect_severity &&
                        findings[i].repairable == expect_repairable,
                    message);
        }
    }
    require(found == expect_present, message);
    free(findings);
    sidereon_rinex_lint_report_free(report);
}

static void check_summary(const SidereonObservationQcSummary *summary, bool has_interval,
                          uint64_t interval_bits, uint32_t source, size_t notes,
                          const char *message) {
    uint64_t bits = 0;
    memcpy(&bits, &summary->interval_s, sizeof(bits));
    require(summary->has_interval_s == has_interval &&
                (!has_interval || bits == interval_bits) &&
                summary->interval_source == source && summary->note_count == notes,
            message);
}

/* The repaired text holds the pinned INTERVAL record, or none when the pin is
 * empty. */
static void check_interval_record(const uint8_t *text, const char *record, const char *message) {
    if (record[0] == '\0') {
        require(strstr((const char *)text, "INTERVAL") == NULL, message);
    } else {
        require(strstr((const char *)text, record) != NULL, message);
    }
}

static char *observation_qc_json(const SidereonObservationQcReport *report) {
    size_t written = 0;
    size_t required = 0;
    require(sidereon_observation_qc_to_json(report, NULL, 0, &written, &required) ==
                    SIDEREON_STATUS_OK &&
                written == 0 && required > 0,
            "measure observation QC JSON");
    char *json = (char *)malloc(required + 1);
    require(json != NULL, "allocate observation QC JSON");
    require(sidereon_observation_qc_to_json(report, (uint8_t *)json, required, &written,
                                            &required) == SIDEREON_STATUS_OK &&
                written == required,
            "copy observation QC JSON");
    json[written] = '\0';
    return json;
}

static uint8_t *repair_observation(const uint8_t *bytes, size_t len, bool set_interval,
                                   size_t *out_len) {
    SidereonRinexRepairOptions options;
    require(sidereon_rinex_repair_options_init(&options) == SIDEREON_STATUS_OK,
            "initialize repair options");
    options.set_interval = set_interval;

    SidereonRinexRepair *repair = NULL;
    require(sidereon_rinex_repair_obs(bytes, len, &options, &repair) == SIDEREON_STATUS_OK &&
                repair != NULL,
            "repair observation text");

    size_t written = 0;
    size_t required = 0;
    require(sidereon_rinex_repair_text(repair, NULL, 0, &written, &required) ==
                SIDEREON_STATUS_OK &&
                written == 0,
            "measure repaired observation text");
    uint8_t *text = (uint8_t *)malloc(required + 1);
    require(text != NULL, "allocate repaired observation text");
    require(sidereon_rinex_repair_text(repair, text, required, &written, &required) ==
                SIDEREON_STATUS_OK &&
                written == required,
            "copy repaired observation text");
    text[written] = 0;
    *out_len = written;
    sidereon_rinex_repair_free(repair);
    return text;
}

int main(int argc, char **argv) {
    require(argc == 2, "usage: rinex_qc_interval_smoke OBS_FIXTURE");

    size_t source_len = 0;
    uint8_t *source = read_file(argv[1], &source_len);
    uint8_t *unavailable = copy_with_interval_record(
        source, source_len,
        "     0.000                                                  INTERVAL");

    check_lint(unavailable, source_len, "OBS-H19", W6_QCI_UNAVAILABLE_HAS_OBS_H19,
               W6_QCI_UNAVAILABLE_OBS_H19_SEVERITY, W6_QCI_UNAVAILABLE_OBS_H19_REPAIRABLE,
               "lint of the unavailable source INTERVAL");

    SidereonObservationQcOptions options;
    require(sidereon_observation_qc_options_init(&options) == SIDEREON_STATUS_OK,
            "initialize observation QC options");
    SidereonObservationQcReport *report = NULL;
    require(sidereon_observation_qc_parse(unavailable, source_len, &options, &report) ==
                SIDEREON_STATUS_OK &&
                report != NULL,
            "default QC accepts unavailable source INTERVAL");
    SidereonObservationQcSummary summary;
    require(sidereon_observation_qc_summary(report, &summary) == SIDEREON_STATUS_OK,
            "summarize inferred-interval QC");
    check_summary(&summary, W6_QCI_UNAVAILABLE_HAS_INTERVAL, W6_QCI_UNAVAILABLE_INTERVAL_S_BITS,
                  W6_QCI_UNAVAILABLE_INTERVAL_SOURCE, W6_QCI_UNAVAILABLE_NOTE_COUNT,
                  "QC cadence of the unavailable source INTERVAL");
    char *json = observation_qc_json(report);
    require(strcmp(json, W6_QCI_UNAVAILABLE_QC_JSON) == 0, "QC JSON of the unavailable INTERVAL");
    free(json);
    sidereon_observation_qc_report_free(report);

    options.has_interval_override_s = true;
    options.interval_override_s = 0.0;
    report = NULL;
    /* The binding maps a QC refusal to INVALID_ARGUMENT
     * (sidereon_observation_qc_parse in bindings/c/src/observation.rs). */
    require(W6_QCI_ZERO_OVERRIDE_REFUSED, "sidereon-core accepts a 0 s interval override");
    require(sidereon_observation_qc_parse(unavailable, source_len, &options, &report) ==
                SIDEREON_STATUS_INVALID_ARGUMENT &&
                report == NULL,
            "invalid caller interval override remains an error");

    size_t preserved_len = 0;
    uint8_t *preserved =
        repair_observation(unavailable, source_len, false, &preserved_len);
    check_interval_record(preserved, W6_QCI_UNAVAILABLE_PRESERVED_INTERVAL_RECORD,
                          "default repair of the unavailable INTERVAL");
    check_lint(preserved, preserved_len, "OBS-H19", W6_QCI_UNAVAILABLE_PRESERVED_HAS_OBS_H19,
               W6_QCI_UNAVAILABLE_PRESERVED_OBS_H19_SEVERITY,
               W6_QCI_UNAVAILABLE_PRESERVED_OBS_H19_REPAIRABLE,
               "lint of the preserved unavailable INTERVAL");

    size_t repaired_len = 0;
    uint8_t *repaired =
        repair_observation(unavailable, source_len, true, &repaired_len);
    check_interval_record(repaired, W6_QCI_UNAVAILABLE_REPAIRED_INTERVAL_RECORD,
                          "opt-in repair of the unavailable INTERVAL");
    check_lint(repaired, repaired_len, "OBS-H19", W6_QCI_UNAVAILABLE_REPAIRED_HAS_OBS_H19,
               W6_QCI_UNAVAILABLE_REPAIRED_OBS_H19_SEVERITY,
               W6_QCI_UNAVAILABLE_REPAIRED_OBS_H19_REPAIRABLE,
               "lint of the repaired unavailable INTERVAL");

    uint8_t *invalid = copy_with_interval_record(
        source, source_len,
        "    -1.000                                                  INTERVAL");
    check_lint(invalid, source_len, "OBS-H20", W6_QCI_INVALID_HAS_OBS_H20,
               W6_QCI_INVALID_OBS_H20_SEVERITY, W6_QCI_INVALID_OBS_H20_REPAIRABLE,
               "lint of the negative source INTERVAL");
    require(sidereon_observation_qc_options_init(&options) == SIDEREON_STATUS_OK,
            "reset observation QC options for invalid source metadata");
    report = NULL;
    require(sidereon_observation_qc_parse(invalid, source_len, &options, &report) ==
                SIDEREON_STATUS_OK &&
                report != NULL,
            "default QC accepts product with invalid source INTERVAL");
    require(sidereon_observation_qc_summary(report, &summary) == SIDEREON_STATUS_OK,
            "summarize negative-interval QC");
    check_summary(&summary, W6_QCI_INVALID_HAS_INTERVAL, W6_QCI_INVALID_INTERVAL_S_BITS,
                  W6_QCI_INVALID_INTERVAL_SOURCE, W6_QCI_INVALID_NOTE_COUNT,
                  "QC cadence of the negative source INTERVAL");
    json = observation_qc_json(report);
    require(strcmp(json, W6_QCI_INVALID_QC_JSON) == 0, "QC JSON of the negative INTERVAL");
    free(json);
    sidereon_observation_qc_report_free(report);

    size_t invalid_preserved_len = 0;
    uint8_t *invalid_preserved =
        repair_observation(invalid, source_len, false, &invalid_preserved_len);
    check_interval_record(invalid_preserved, W6_QCI_INVALID_PRESERVED_INTERVAL_RECORD,
                          "default repair of the negative INTERVAL");
    check_lint(invalid_preserved, invalid_preserved_len, "OBS-H20",
               W6_QCI_INVALID_PRESERVED_HAS_OBS_H20, W6_QCI_INVALID_PRESERVED_OBS_H20_SEVERITY,
               W6_QCI_INVALID_PRESERVED_OBS_H20_REPAIRABLE,
               "lint of the preserved negative INTERVAL");

    size_t invalid_repaired_len = 0;
    uint8_t *invalid_repaired =
        repair_observation(invalid, source_len, true, &invalid_repaired_len);
    check_interval_record(invalid_repaired, W6_QCI_INVALID_REPAIRED_INTERVAL_RECORD,
                          "opt-in repair of the negative INTERVAL");
    check_lint(invalid_repaired, invalid_repaired_len, "OBS-H20",
               W6_QCI_INVALID_REPAIRED_HAS_OBS_H20, W6_QCI_INVALID_REPAIRED_OBS_H20_SEVERITY,
               W6_QCI_INVALID_REPAIRED_OBS_H20_REPAIRABLE,
               "lint of the repaired negative INTERVAL");

    size_t single_epoch_len = one_epoch_length(unavailable, source_len);
    require(sidereon_observation_qc_options_init(&options) == SIDEREON_STATUS_OK,
            "reset observation QC options");
    report = NULL;
    require(sidereon_observation_qc_parse(unavailable, single_epoch_len, &options, &report) ==
                SIDEREON_STATUS_OK &&
                report != NULL,
            "default QC accepts unresolved unavailable source INTERVAL");
    require(sidereon_observation_qc_summary(report, &summary) == SIDEREON_STATUS_OK,
            "summarize unresolved-interval QC");
    check_summary(&summary, W6_QCI_SINGLE_HAS_INTERVAL, W6_QCI_SINGLE_INTERVAL_S_BITS,
                  W6_QCI_SINGLE_INTERVAL_SOURCE, W6_QCI_SINGLE_NOTE_COUNT,
                  "single-epoch QC cadence");
    sidereon_observation_qc_report_free(report);

    size_t unresolved_repaired_len = 0;
    uint8_t *unresolved_repaired = repair_observation(unavailable, single_epoch_len, true,
                                                      &unresolved_repaired_len);
    check_interval_record(unresolved_repaired, W6_QCI_SINGLE_REPAIRED_INTERVAL_RECORD,
                          "opt-in repair of the single epoch");
    check_lint(unresolved_repaired, unresolved_repaired_len, "OBS-H19",
               W6_QCI_SINGLE_REPAIRED_HAS_OBS_H19, W6_QCI_SINGLE_REPAIRED_OBS_H19_SEVERITY,
               W6_QCI_SINGLE_REPAIRED_OBS_H19_REPAIRABLE,
               "lint of the repaired single epoch");

    free(unresolved_repaired);
    free(invalid_repaired);
    free(invalid_preserved);
    free(invalid);
    free(repaired);
    free(preserved);
    free(unavailable);
    free(source);
    return 0;
}
