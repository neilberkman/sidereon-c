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

enum {
    RINEX_QC_SEVERITY_ERROR = 1,
    RINEX_QC_SEVERITY_INFO = 3,
    OBSERVATION_QC_INTERVAL_INFERRED = 2,
    OBSERVATION_QC_INTERVAL_UNRESOLVED = 3,
};

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

static bool lint_has_code(const uint8_t *bytes, size_t len, const char *code) {
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
            if (strcmp(code, "OBS-H19") == 0) {
                require(findings[i].severity == RINEX_QC_SEVERITY_INFO &&
                            findings[i].repairable,
                        "OBS-H19 is repairable information");
            } else if (strcmp(code, "OBS-H20") == 0) {
                require(findings[i].severity == RINEX_QC_SEVERITY_ERROR &&
                            findings[i].repairable,
                        "OBS-H20 is a repairable error");
            }
        }
    }
    free(findings);
    sidereon_rinex_lint_report_free(report);
    return found;
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

    require(lint_has_code(unavailable, source_len, "OBS-H19"),
            "lint exposes unavailable source INTERVAL as OBS-H19");

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
    require(summary.has_interval_s && summary.interval_s == 30.0 &&
                summary.interval_source == OBSERVATION_QC_INTERVAL_INFERRED &&
                summary.note_count == 0,
            "QC infers 30-second cadence from epochs");
    char *json = observation_qc_json(report);
    require(strstr(json, "\"code\":\"OBS-H19\"") != NULL,
            "QC JSON exposes OBS-H19");
    free(json);
    sidereon_observation_qc_report_free(report);

    options.has_interval_override_s = true;
    options.interval_override_s = 0.0;
    report = NULL;
    require(sidereon_observation_qc_parse(unavailable, source_len, &options, &report) ==
                SIDEREON_STATUS_INVALID_ARGUMENT &&
                report == NULL,
            "invalid caller interval override remains an error");

    size_t preserved_len = 0;
    uint8_t *preserved =
        repair_observation(unavailable, source_len, false, &preserved_len);
    require(strstr((const char *)preserved,
                   "     0.000                                                  INTERVAL") != NULL,
            "default repair preserves standards-compatible unavailable INTERVAL");
    require(lint_has_code(preserved, preserved_len, "OBS-H19"),
            "preserved unavailable interval retains OBS-H19");

    size_t repaired_len = 0;
    uint8_t *repaired =
        repair_observation(unavailable, source_len, true, &repaired_len);
    require(strstr((const char *)repaired,
                   "    30.000                                                  INTERVAL") != NULL,
            "repair replaces unavailable INTERVAL with inferred cadence");
    require(!lint_has_code(repaired, repaired_len, "OBS-H19"),
            "repaired inferred cadence clears OBS-H19");

    uint8_t *invalid = copy_with_interval_record(
        source, source_len,
        "    -1.000                                                  INTERVAL");
    require(lint_has_code(invalid, source_len, "OBS-H20"),
            "lint exposes invalid negative source INTERVAL as OBS-H20");
    require(sidereon_observation_qc_options_init(&options) == SIDEREON_STATUS_OK,
            "reset observation QC options for invalid source metadata");
    report = NULL;
    require(sidereon_observation_qc_parse(invalid, source_len, &options, &report) ==
                SIDEREON_STATUS_OK &&
                report != NULL,
            "default QC accepts product with invalid source INTERVAL");
    require(sidereon_observation_qc_summary(report, &summary) == SIDEREON_STATUS_OK &&
                summary.has_interval_s && summary.interval_s == 30.0 &&
                summary.interval_source == OBSERVATION_QC_INTERVAL_INFERRED,
            "QC ignores invalid source INTERVAL and infers cadence");
    json = observation_qc_json(report);
    require(strstr(json, "\"code\":\"OBS-H20\"") != NULL,
            "QC JSON exposes OBS-H20");
    free(json);
    sidereon_observation_qc_report_free(report);

    size_t invalid_preserved_len = 0;
    uint8_t *invalid_preserved =
        repair_observation(invalid, source_len, false, &invalid_preserved_len);
    require(strstr((const char *)invalid_preserved,
                   "    -1.000                                                  INTERVAL") != NULL,
            "default repair preserves invalid source INTERVAL");
    require(lint_has_code(invalid_preserved, invalid_preserved_len, "OBS-H20"),
            "preserved invalid interval retains OBS-H20");

    size_t invalid_repaired_len = 0;
    uint8_t *invalid_repaired =
        repair_observation(invalid, source_len, true, &invalid_repaired_len);
    require(strstr((const char *)invalid_repaired,
                   "    30.000                                                  INTERVAL") != NULL,
            "opt-in repair replaces invalid source INTERVAL");
    require(!lint_has_code(invalid_repaired, invalid_repaired_len, "OBS-H20"),
            "repaired invalid interval clears OBS-H20");

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
    require(!summary.has_interval_s &&
                summary.interval_source == OBSERVATION_QC_INTERVAL_UNRESOLVED &&
                summary.note_count == 1,
            "single-epoch QC reports unresolved cadence");
    sidereon_observation_qc_report_free(report);

    size_t unresolved_repaired_len = 0;
    uint8_t *unresolved_repaired = repair_observation(unavailable, single_epoch_len, true,
                                                      &unresolved_repaired_len);
    require(strstr((const char *)unresolved_repaired, "INTERVAL") == NULL,
            "opt-in repair removes unavailable INTERVAL when cadence is unresolved");
    require(!lint_has_code(unresolved_repaired, unresolved_repaired_len, "OBS-H19"),
            "removed unresolved interval clears OBS-H19");

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
