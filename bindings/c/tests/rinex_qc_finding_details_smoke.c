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

static void replace_once(uint8_t *bytes, size_t len, const char *old_text, const char *new_text) {
    size_t old_len = strlen(old_text);
    require(old_len == strlen(new_text), "mutation preserves field width");
    size_t matches = 0;
    for (size_t i = 0; i + old_len <= len; ++i) {
        if (memcmp(bytes + i, old_text, old_len) == 0) {
            memcpy(bytes + i, new_text, old_len);
            ++matches;
            break;
        }
    }
    require(matches == 1, "find mutation field");
}
static void check_mutated_finding(const char *fixture, const char *old_a, const char *new_a,
                                  const char *old_b, const char *new_b,
                                  const char *old_c, const char *new_c,
                                  const char *expected_code, uint32_t expected_severity, bool expected_repairable, const char *expected_field,
                                  const char *expected_json) {
    size_t len = 0;
    uint8_t *bytes = read_file(fixture, &len);
    if (old_a != NULL) replace_once(bytes, len, old_a, new_a);
    if (old_b != NULL) replace_once(bytes, len, old_b, new_b);
    if (old_c != NULL) replace_once(bytes, len, old_c, new_c);
    SidereonRinexLintReport *report = NULL;
    require(sidereon_rinex_lint_obs(bytes, len, &report) == SIDEREON_STATUS_OK && report != NULL, "lint mutated OBS fixture");
    free(bytes);
    size_t written = 0, required = 0;
    require(sidereon_rinex_lint_findings(report, NULL, 0, &written, &required) == SIDEREON_STATUS_OK && required > 0, "query mutated findings");
    SidereonRinexLintFinding *findings = (SidereonRinexLintFinding *)calloc(required, sizeof(*findings));
    require(findings != NULL, "allocate mutated findings");
    require(sidereon_rinex_lint_findings(report, findings, required, &written, &required) == SIDEREON_STATUS_OK, "copy mutated findings");
    size_t index = required;
    for (size_t i = 0; i < required; ++i) if (strcmp(findings[i].code, expected_code) == 0) { index = i; break; }
    require(index < required && findings[index].severity == expected_severity && findings[index].repairable == expected_repairable && findings[index].has_field && strcmp(findings[index].field, expected_field) == 0, "exact mutated finding code, severity, and header location");
    char *json = details_json(report, index);
    require(strcmp(json, expected_json) == 0, "exact mutated finding variant payload");
    free(json);
    free(findings);
    sidereon_rinex_lint_report_free(report);
}
static void check_header_mutations(const char *fixture) {
    check_mutated_finding(fixture, "30.0000", "01.0000", NULL, NULL, NULL, NULL,
        "OBS-H09", 2, true, "INTERVAL",
        "{\"kind\":\"ObsIntervalMismatch\",\"spec_ref\":\"RINEX 3.05 Table A2\",\"details\":{\"declared_s\":1.0,\"observed_s\":30.0}}");
    check_mutated_finding(fixture, "918129.4000", "     1.0000", "-4346071.2000", "       1.0000", "4561977.8000", "      1.0000",
        "OBS-H17", 2, false, "APPROX POSITION XYZ",
        "{\"kind\":\"ObsImplausibleApproxPosition\",\"spec_ref\":\"RINEX 3.05 Table A2\",\"details\":{\"radius_m\":1.7320508075688772}}");
    check_mutated_finding(fixture, "  2015     1     1     0     0    0.0000000", "  2015     1     2     0     0    0.0000000", NULL, NULL, NULL, NULL,
        "OBS-H07", 1, true, "TIME OF FIRST OBS",
        "{\"kind\":\"ObsTimeOfFirstMismatch\",\"spec_ref\":\"RINEX 3.05 Table A2\",\"details\":{\"declared\":{\"year\":2015,\"month\":1,\"day\":2,\"hour\":0,\"minute\":0,\"second\":0.0},\"declared_scale\":\"GPST\",\"observed\":{\"year\":2015,\"month\":1,\"day\":1,\"hour\":0,\"minute\":0,\"second\":0.0},\"observed_scale\":\"GPST\"}}");
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
    size_t written = 9, required = 9;
    require(sidereon_rinex_lint_finding_details_json(report, summary.finding_count, NULL, 0, &written, &required) == SIDEREON_STATUS_INVALID_ARGUMENT && written == 0 && required == 0, "reject out-of-range index and clear counts");
    written = 0; required = 0;
    require(sidereon_rinex_lint_finding_details_json(report, 0, NULL, 0, &written, &required) == SIDEREON_STATUS_OK && required > 1, "query short-buffer size");
    uint8_t short_buffer[1] = {0xA5};
    size_t short_capacity = sizeof(short_buffer);
    require(sidereon_rinex_lint_finding_details_json(report, 0, short_buffer, short_capacity, &written, &required) == SIDEREON_STATUS_INVALID_ARGUMENT && written == 0 && required > short_capacity && short_buffer[0] == 0xA5, "short buffer reports required size without writing");
    sidereon_rinex_lint_report_free(report);
    check_header_mutations(argv[1]);
    check_epoch_order(argv[2]);
    puts("rinex_qc_finding_details_smoke: OK"); return 0;
}
