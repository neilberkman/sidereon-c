#include <sidereon.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
static void require(int ok, const char *message) { if (!ok) { fprintf(stderr, "rinex_qc_finding_details_smoke: %s\n", message); exit(1); } }
static uint8_t *read_file(const char *path, size_t *out_len) {
    FILE *file = fopen(path, "rb"); require(file != NULL, "open fixture");
    require(fseek(file, 0, SEEK_END) == 0, "seek end"); long size = ftell(file); require(size >= 0, "measure fixture");
    require(fseek(file, 0, SEEK_SET) == 0, "rewind fixture"); uint8_t *bytes = (uint8_t *)malloc((size_t)size); require(bytes != NULL, "allocate fixture");
    require(fread(bytes, 1, (size_t)size, file) == (size_t)size, "read fixture"); fclose(file); *out_len = (size_t)size; return bytes;
}
static char *details_json(const SidereonRinexLintReport *report, size_t index) {
    size_t written = 0, required = 0;
    require(sidereon_rinex_lint_finding_details_json(report, index, NULL, 0, &written, &required) == SIDEREON_STATUS_OK && written == 0 && required > 0, "query JSON size");
    char *json = (char *)malloc(required + 1); require(json != NULL, "allocate JSON");
    require(sidereon_rinex_lint_finding_details_json(report, index, (uint8_t *)json, required, &written, &required) == SIDEREON_STATUS_OK && written == required, "copy JSON");
    json[written] = '\0'; return json;
}
static void check_epoch_order(const char *path) {
    size_t len = 0;
    uint8_t *bytes = read_file(path, &len);
    char *text = (char *)malloc(len + 1);
    require(text != NULL, "allocate swapped fixture text");
    memcpy(text, bytes, len);
    text[len] = '\0';
    free(bytes);
    const char *first = "> 2020 06 25 00 00 00.0000000  0 43";
    const char *second = "> 2020 06 25 00 00 30.0000000  0 43";
    char *p_first = strstr(text, first);
    char *p_second = strstr(text, second);
    require(p_first != NULL && p_second != NULL && strlen(first) == strlen(second), "find fixture epochs");
    char marker[64];
    require(strlen(first) < sizeof(marker), "epoch marker capacity");
    memset(marker, 'X', strlen(first));
    marker[strlen(first)] = '\0';
    memcpy(p_first, marker, strlen(first));
    memcpy(p_second, first, strlen(first));
    p_first = strstr(text, marker);
    require(p_first != NULL, "find epoch marker");
    memcpy(p_first, second, strlen(second));

    SidereonRinexLintReport *report = NULL;
    require(sidereon_rinex_lint_obs((const uint8_t *)text, len, &report) == SIDEREON_STATUS_OK && report != NULL, "lint swapped epochs");
    free(text);
    SidereonRinexLintSummary summary;
    require(sidereon_rinex_lint_summary(report, &summary) == SIDEREON_STATUS_OK, "read swapped summary");
    size_t written = 0, required = 0;
    require(sidereon_rinex_lint_findings(report, NULL, 0, &written, &required) == SIDEREON_STATUS_OK && required == summary.finding_count, "query swapped findings");
    SidereonRinexLintFinding *findings = (SidereonRinexLintFinding *)calloc(required, sizeof(*findings));
    require(findings != NULL, "allocate swapped findings");
    require(sidereon_rinex_lint_findings(report, findings, required, &written, &required) == SIDEREON_STATUS_OK, "copy swapped findings");
    size_t index = required;
    for (size_t i = 0; i < required; ++i) if (strcmp(findings[i].code, "OBS-B01") == 0) { index = i; break; }
    require(index < required && findings[index].has_epoch_index && findings[index].epoch_index == 1, "epoch-order public location");
    char *json = details_json(report, index);
    require(strstr(json, "\"kind\":\"ObsEpochOrder\"") != NULL &&
                strstr(json, "\"spec_ref\":\"RINEX 3.05 Table A3\"") != NULL &&
                strstr(json, "\"second\":0.0") != NULL &&
                strstr(json, "\"second\":30.0") != NULL,
            "exact epoch-order nested time payload");
    free(json);
    free(findings);
    sidereon_rinex_lint_report_free(report);
}

int main(int argc, char **argv) {
    require(argc == 3, "expected two OBS fixture paths"); size_t len = 0; uint8_t *bytes = read_file(argv[1], &len);
    SidereonRinexLintReport *report = NULL; require(sidereon_rinex_lint_obs(bytes, len, &report) == SIDEREON_STATUS_OK && report != NULL, "lint fixture"); free(bytes);
    SidereonRinexLintSummary summary; require(sidereon_rinex_lint_summary(report, &summary) == SIDEREON_STATUS_OK && summary.finding_count == 9, "expect nine findings");
    const char *satellites[] = {"R05", "R06", "R07", "R09", "R15", "R16", "R17", "R24"};
    for (size_t i = 0; i < summary.finding_count; ++i) {
        char *json = details_json(report, i);
        if (i == 0) {
            require(strstr(json, "\"kind\":\"ObsUnretainedHeader\"") && strstr(json, "\"spec_ref\":\"RINEX 3.05 section 6.6\"") && strstr(json, "\"details\":{\"label\":\"WAVELENGTH FACT L1/2\"}"), "exact unretained-header payload");
        } else {
            char expected[256]; snprintf(expected, sizeof(expected), "\"details\":{\"satellite\":\"%s\",\"issue\":\"missing slot\"}", satellites[i - 1]);
            require(strstr(json, "\"kind\":\"ObsGlonassSlotIssue\"") && strstr(json, "\"spec_ref\":\"RINEX 3.05 Table A2\"") && strstr(json, "\"issue\":\"missing slot\"") && strstr(json, expected), "exact GLONASS payload");
        }
        free(json);
    }
    size_t written = 0, required = 0;
    require(sidereon_rinex_lint_finding_details_json(report, summary.finding_count, NULL, 0, &written, &required) == SIDEREON_STATUS_INVALID_ARGUMENT, "reject out-of-range index");
    sidereon_rinex_lint_report_free(report);
    check_epoch_order(argv[2]);
    puts("rinex_qc_finding_details_smoke: OK"); return 0;
}
