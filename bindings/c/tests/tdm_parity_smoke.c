#include "sidereon.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
static int check(int condition, const char *message) {
    if (!condition) {
        fprintf(stderr, "tdm_parity_smoke: %s\n", message);
        return 1;
    }
    return 0;
}

#define CHECK(condition, code)                                                                     \
    do {                                                                                           \
        if (check((condition), #condition))                                                        \
            return (code);                                                                         \
    } while (0)

static SidereonTdmText text(const char *s) {
    SidereonTdmText v = {(const uint8_t *)s, strlen(s)};
    return v;
}
static SidereonTdmOptionalText some(const char *s) {
    SidereonTdmOptionalText v = {true, text(s)};
    return v;
}
static SidereonTdmOptionalText none(void) {
    SidereonTdmOptionalText v = {false, {NULL, 0}};
    return v;
}
typedef SidereonStatus (*TextFn)(const SidereonTdm *, size_t, uint32_t, uint8_t *, size_t, size_t *,
                                 size_t *);
static char *copy_text(TextFn fn, const SidereonTdm *tdm, size_t row, uint32_t part) {
    size_t w = 0, r = 0;
    if (fn(tdm, row, part, NULL, 0, &w, &r) != SIDEREON_STATUS_OK)
        return NULL;
    char *out = calloc(r + 1, 1);
    if (!out)
        return NULL;
    if (fn(tdm, row, part, (uint8_t *)out, r, &w, &r) != SIDEREON_STATUS_OK) {
        free(out);
        return NULL;
    }
    return out;
}
static char *copy_comment(const SidereonTdm *tdm, size_t row) {
    size_t w = 0, r = 0;
    if (sidereon_tdm_comment_text(tdm, row, NULL, 0, &w, &r) != SIDEREON_STATUS_OK)
        return NULL;
    char *out = calloc(r + 1, 1);
    if (!out)
        return NULL;
    if (sidereon_tdm_comment_text(tdm, row, (uint8_t *)out, r, &w, &r) != SIDEREON_STATUS_OK) {
        free(out);
        return NULL;
    }
    return out;
}
static char *copy_departure(const SidereonTdmDepartures *d, size_t row) {
    size_t w = 0, r = 0;
    if (sidereon_tdm_departure_detail(d, row, NULL, 0, &w, &r) != SIDEREON_STATUS_OK)
        return NULL;
    char *out = calloc(r + 1, 1);
    if (!out)
        return NULL;
    if (sidereon_tdm_departure_detail(d, row, (uint8_t *)out, r, &w, &r) != SIDEREON_STATUS_OK) {
        free(out);
        return NULL;
    }
    return out;
}
static char *copy_warning(const SidereonTdmWarnings *d, size_t row) {
    size_t w = 0, r = 0;
    if (sidereon_tdm_warning_detail(d, row, NULL, 0, &w, &r) != SIDEREON_STATUS_OK)
        return NULL;
    char *out = calloc(r + 1, 1);
    if (!out)
        return NULL;
    if (sidereon_tdm_warning_detail(d, row, (uint8_t *)out, r, &w, &r) != SIDEREON_STATUS_OK) {
        free(out);
        return NULL;
    }
    return out;
}
static char *encode(const SidereonTdm *tdm) {
    size_t w = 0, r = 0;
    if (sidereon_tdm_to_kvn(tdm, NULL, 0, &w, &r) != SIDEREON_STATUS_OK)
        return NULL;
    char *out = calloc(r + 1, 1);
    if (!out)
        return NULL;
    if (sidereon_tdm_to_kvn(tdm, (uint8_t *)out, r, &w, &r) != SIDEREON_STATUS_OK) {
        free(out);
        return NULL;
    }
    return out;
}
static int error_is(const char *operation_class, const char *kind) {
    SidereonEngineErrorInfo info = {0};
    size_t w = 0, r = 0;
    if (sidereon_last_engine_error_info(&info) != SIDEREON_STATUS_OK ||
        info.family != SIDEREON_ENGINE_ERROR_FAMILY_TDM)
        return 0;
    char *json = calloc(info.payload_len + 1, 1);
    int ok = json &&
             sidereon_last_engine_error_payload((uint8_t *)json, info.payload_len, &w, &r) ==
                 SIDEREON_STATUS_OK &&
             strstr(json, operation_class) && strstr(json, kind);
    free(json);
    return ok;
}
static int policies(void) {
    SidereonTdmPolicy rs = sidereon_tdm_policy_strict(), rd = sidereon_tdm_policy_default(),
                      rl = sidereon_tdm_policy_lenient();
    CHECK(memcmp(&rs, &rd, sizeof(rs)) == 0, 20);
    CHECK(rs.non_printable == SIDEREON_TDM_LENIENCY_STRICT &&
              rs.final_terminator == SIDEREON_TDM_LENIENCY_STRICT,
          21);
    CHECK(rl.non_printable == SIDEREON_TDM_LENIENCY_FORGIVE &&
              rl.final_terminator == SIDEREON_TDM_LENIENCY_FORGIVE,
          22);
    SidereonTdmWritePolicy ws = sidereon_tdm_write_policy_strict(),
                           wd = sidereon_tdm_write_policy_default(),
                           wl = sidereon_tdm_write_policy_lenient();
    CHECK(memcmp(&ws, &wd, sizeof(ws)) == 0, 23);
    CHECK(ws.repeated_keywords == SIDEREON_TDM_LENIENCY_STRICT &&
              wl.repeated_keywords == SIDEREON_TDM_LENIENCY_FORGIVE,
          24);
    SidereonTdmPolicy converted = {0};
    CHECK(sidereon_tdm_write_policy_as_read(wl, &converted) == SIDEREON_STATUS_OK &&
              memcmp(&converted, &rl, sizeof(rl)) == 0,
          25);
    ws.keyword_order = 77;
    CHECK(sidereon_tdm_write_policy_as_read(ws, &converted) == SIDEREON_STATUS_INVALID_ARGUMENT,
          26);
    CHECK(error_is("validation", "invalid_policy"), 27);
    return 0;
}
int main(void) {
    int p = policies();
    if (p)
        return p;
    static const char expected[] =
        "CCSDS_TDM_VERS = 2.0\nCOMMENT header comment\nCREATION_DATE = "
        "2026-001T00:00:00\nORIGINATOR = TEST\n"
        "META_START\nCOMMENT metadata comment\nTIME_SYSTEM = UTC\nPARTICIPANT_1 = DSS-55\nMODE = "
        "SEQUENTIAL\nPATH = 1\nMETA_STOP\n"
        "DATA_START\nCOMMENT data comment\nRANGE = 2026-001T00:00:00 123.5\nDATA_STOP\n";
    SidereonTdm *tdm = NULL;
    CHECK(sidereon_tdm_create((const uint8_t *)"2.0", 3, &tdm) == SIDEREON_STATUS_OK, 30);
    SidereonTdmHeaderInput header = {text("2.0"), some("2026-001T00:00:00"), some("TEST"), none()};
    CHECK(sidereon_tdm_set_header(tdm, header) == SIDEREON_STATUS_OK, 31);
    SidereonTdmCommentInput hc[] = {{text("header comment"), 1}};
    CHECK(sidereon_tdm_replace_header_raw(tdm, hc, 1, NULL, 0) == SIDEREON_STATUS_OK, 32);
    SidereonTdmFieldInput fields[] = {{text("TIME_SYSTEM"), text("UTC")},
                                      {text("PARTICIPANT_1"), text("DSS-55")},
                                      {text("MODE"), text("SEQUENTIAL")},
                                      {text("PATH"), text("1")}};
    SidereonTdmCommentInput mc[] = {{text("metadata comment"), 0}},
                            dc[] = {{text("data comment"), 0}};
    SidereonTdmDataRecordInput record = {SIDEREON_TDM_OBSERVABLE_RANGE,
                                         false,
                                         0,
                                         {NULL, 0},
                                         SIDEREON_TDM_UNIT_KILOMETERS,
                                         text("km"),
                                         text("RANGE"),
                                         text("2026-001T00:00:00"),
                                         text("123.5"),
                                         123.5};
    SidereonTdmDepartures *departures = NULL;
    CHECK(sidereon_tdm_append_segment(tdm, fields, 4, mc, 1, dc, 1, &record, 1,
                                      sidereon_tdm_write_policy_strict(),
                                      &departures) == SIDEREON_STATUS_OK,
          33);
    size_t count = 99, written = 0, required = 0;
    CHECK(sidereon_tdm_departures_count(departures, &count) == SIDEREON_STATUS_OK && count == 0,
          34);
    CHECK(sidereon_tdm_departures_count(departures, (size_t *)(void *)departures) ==
              SIDEREON_STATUS_INVALID_ARGUMENT,
          134);
    CHECK(sidereon_tdm_departure_detail(departures, 0, (uint8_t *)(void *)departures, 1, &written,
                                        &required) == SIDEREON_STATUS_INVALID_ARGUMENT,
          234);
    sidereon_tdm_departures_free(departures);
    char *kvn = encode(tdm);
    CHECK(kvn && strcmp(kvn, expected) == 0, 35);
    size_t hw = 0, hr = 0;
    CHECK(sidereon_tdm_header_text(tdm, 2, NULL, 0, &hw, &hr) == SIDEREON_STATUS_OK && hr == 4,
          135);
    char htext[5] = {0};
    CHECK(sidereon_tdm_header_text(tdm, 2, (uint8_t *)htext, 4, &hw, &hr) == SIDEREON_STATUS_OK &&
              strcmp(htext, "TEST") == 0,
          235);
    SidereonTdmWritePolicy wp = sidereon_tdm_write_policy_lenient();
    departures = NULL;
    CHECK(sidereon_tdm_to_kvn_with_policy(tdm, wp, NULL, 0, &written, &required,
                                          (SidereonTdmDepartures **)(void *)tdm) ==
              SIDEREON_STATUS_INVALID_ARGUMENT,
          334);
    CHECK(sidereon_tdm_to_kvn_with_policy(tdm, wp, NULL, 0, &written, &required, &departures) ==
              SIDEREON_STATUS_OK,
          335);
    CHECK(sidereon_tdm_departures_count(departures, &count) == SIDEREON_STATUS_OK && count == 1,
          435);
    sidereon_tdm_departures_free(departures);
    SidereonTdm *parsed = NULL;
    CHECK(sidereon_tdm_parse_kvn((const uint8_t *)kvn, strlen(kvn), &parsed) == SIDEREON_STATUS_OK,
          36);
    bool equal = false;
    CHECK(sidereon_tdm_equal(tdm, parsed, &equal) == SIDEREON_STATUS_OK && equal,
          136);
    equal = true;
    CHECK(sidereon_tdm_equal(NULL, parsed, &equal) == SIDEREON_STATUS_NULL_POINTER && !equal,
          236);
    SidereonTdmHeader hs = {0};
    CHECK(sidereon_tdm_header(parsed, &hs) == SIDEREON_STATUS_OK, 37);
    CHECK(hs.version_len == 3 && hs.has_creation_date && hs.creation_date_len == 17 &&
              hs.segment_count == 1,
          137);
    SidereonTdmComment comments[3];
    CHECK(sidereon_tdm_comments(parsed, comments, 3, &written, &required) == SIDEREON_STATUS_OK &&
              written == 3 && comments[0].section == 0 && comments[0].before_record == 1 &&
              comments[1].section == 1 && comments[2].section == 2,
          38);
    char *ct = copy_comment(parsed, 1);
    CHECK(ct && strcmp(ct, "metadata comment") == 0, 39);
    free(ct);
    SidereonTdmField outf[4];
    CHECK(sidereon_tdm_fields(parsed, outf, 4, &written, &required) == SIDEREON_STATUS_OK &&
              written == 4 && outf[0].key_len == 11 && outf[1].value_len == 6,
          40);
    char *ft = copy_text(sidereon_tdm_field_text, parsed, 1, 1);
    CHECK(ft && strcmp(ft, "DSS-55") == 0, 41);
    free(ft);
    ft = copy_text(sidereon_tdm_field_text, parsed, 0, 0);
    CHECK(ft && strcmp(ft, "TIME_SYSTEM") == 0, 141);
    free(ft);
    char *rt = copy_text(sidereon_tdm_record_text, parsed, 0, 4);
    CHECK(rt && strcmp(rt, "km") == 0, 42);
    free(rt);
    CHECK(sidereon_tdm_remove_record(parsed, 0, 0) == SIDEREON_STATUS_OK, 142);
    CHECK(sidereon_tdm_equal(tdm, parsed, &equal) == SIDEREON_STATUS_OK && !equal,
          143);
    CHECK(sidereon_tdm_equal(tdm, parsed, (bool *)(void *)tdm) ==
              SIDEREON_STATUS_INVALID_ARGUMENT,
          144);
    CHECK(sidereon_tdm_insert_record(parsed, 0, 0, record) == SIDEREON_STATUS_OK, 242);
    char lcbuf[201], lvbuf[201];
    memset(lcbuf, 'C', 200);
    lcbuf[200] = 0;
    memset(lvbuf, 'P', 200);
    lvbuf[200] = 0;
    SidereonTdmCommentInput lmc[] = {{text(lcbuf), 0}};
    SidereonTdmFieldInput lf[] = {{text("TIME_SYSTEM"), text("UTC")},
                                  {text("PARTICIPANT_1"), text(lvbuf)},
                                  {text("MODE"), text("SEQUENTIAL")},
                                  {text("PATH"), text("1")}};
    departures = NULL;
    CHECK(sidereon_tdm_replace_metadata(parsed, 0, lf, 4, lmc, 1,
                                        sidereon_tdm_write_policy_strict(),
                                        &departures) == SIDEREON_STATUS_OK,
          43);
    sidereon_tdm_departures_free(departures);
    char *long_kvn = encode(parsed);
    CHECK(long_kvn && strstr(long_kvn, lcbuf) && strstr(long_kvn, lvbuf), 44);
    SidereonTdm *lp = NULL;
    CHECK(sidereon_tdm_parse_kvn((uint8_t *)long_kvn, strlen(long_kvn), &lp) == SIDEREON_STATUS_OK,
          45);
    char *lc = copy_comment(lp, 1), *lv = copy_text(sidereon_tdm_field_text, lp, 1, 1);
    CHECK(lc && lv && strcmp(lc, lcbuf) == 0 && strcmp(lv, lvbuf) == 0, 46);
    free(lc);
    free(lv);
    free(long_kvn);
    sidereon_tdm_free(lp);
    SidereonTdmFieldInput missing[] = {{text("PARTICIPANT_1"), text("DSS-55")}};
    departures = NULL;
    CHECK(sidereon_tdm_replace_metadata(tdm, 0, missing, 1, NULL, 0,
                                        sidereon_tdm_write_policy_lenient(),
                                        &departures) == SIDEREON_STATUS_OK,
          47);
    char *dj = copy_departure(departures, 0);
    CHECK(dj && strstr(dj, "missing_keyword") && strstr(dj, "TIME_SYSTEM") &&
              strstr(dj, "\"segment\":1"),
          48);
    free(dj);
    sidereon_tdm_departures_free(departures);
    departures = NULL;
    CHECK(sidereon_tdm_replace_metadata(tdm, 0, missing, 1, NULL, 0,
                                        sidereon_tdm_write_policy_strict(),
                                        &departures) == SIDEREON_STATUS_INVALID_ARGUMENT,
          49);
    CHECK(!departures && error_is("validation", "missing_keyword"), 50);
    CHECK(sidereon_tdm_replace_data(parsed, 0, dc, 1, &record, 1) == SIDEREON_STATUS_OK, 51);
    record.observable = 9999;
    CHECK(sidereon_tdm_insert_record(parsed, 0, 0, record) == SIDEREON_STATUS_INVALID_ARGUMENT,
          151);
    CHECK(error_is("validation", "invalid_argument"), 251);
    record.observable = SIDEREON_TDM_OBSERVABLE_RANGE;
    CHECK(sidereon_tdm_remove_segment(parsed, 0) == SIDEREON_STATUS_OK, 52);
    CHECK(sidereon_tdm_remove_segment(parsed, 0) == SIDEREON_STATUS_INVALID_ARGUMENT, 53);
    CHECK(error_is("validation", "invalid_argument"), 153);
    SidereonTdmPolicy bad = sidereon_tdm_policy_default();
    bad.record_order = 99;
    SidereonTdmWarnings *warnings = NULL;
    SidereonTdm *bad_tdm = NULL;
    CHECK(sidereon_tdm_parse_kvn_with_policy((uint8_t *)expected, strlen(expected), bad, &bad_tdm,
                                             &warnings) == SIDEREON_STATUS_INVALID_ARGUMENT &&
              !bad_tdm && !warnings,
          54);
    CHECK(error_is("parse", "invalid_policy"), 55);
    SidereonTdmPolicy lenient = sidereon_tdm_policy_lenient();
    CHECK(sidereon_tdm_parse_kvn_with_policy((uint8_t *)expected, strlen(expected) - 1, lenient,
                                             &bad_tdm, &warnings) == SIDEREON_STATUS_OK,
          56);
    CHECK(sidereon_tdm_warnings_count(warnings, &count) == SIDEREON_STATUS_OK && count == 1, 57);
    CHECK(sidereon_tdm_header(tdm, (SidereonTdmHeader *)(void *)tdm) ==
              SIDEREON_STATUS_INVALID_ARGUMENT,
          157);
    CHECK(sidereon_tdm_comments(tdm, (SidereonTdmComment *)(void *)tdm, 1, &written, &required) ==
              SIDEREON_STATUS_INVALID_ARGUMENT,
          158);
    CHECK(sidereon_tdm_header_text(tdm, 0, (uint8_t *)(void *)tdm, 1, &written, &required) ==
              SIDEREON_STATUS_INVALID_ARGUMENT,
          159);
    CHECK(sidereon_tdm_record_text(tdm, 0, 0, NULL, 0, (size_t *)(void *)tdm, &required) ==
              SIDEREON_STATUS_INVALID_ARGUMENT,
          160);
    CHECK(sidereon_tdm_warnings_count(warnings, (size_t *)(void *)warnings) ==
              SIDEREON_STATUS_INVALID_ARGUMENT,
          161);
    CHECK(sidereon_tdm_warning_detail(warnings, 0, (uint8_t *)(void *)warnings, 1, &written,
                                      &required) == SIDEREON_STATUS_INVALID_ARGUMENT,
          162);
    CHECK(sidereon_tdm_replace_metadata(
              tdm, 0, missing, 1, NULL, 0, sidereon_tdm_write_policy_lenient(),
              (SidereonTdmDepartures **)(void *)tdm) == SIDEREON_STATUS_INVALID_ARGUMENT,
          163);
    char *wj = copy_warning(warnings, 0);
    CHECK(wj && strstr(wj, "unterminated_final_line"), 58);
    free(wj);
    sidereon_tdm_warnings_free(warnings);
    sidereon_tdm_free(bad_tdm);
    free(kvn);
    sidereon_tdm_free(parsed);
    sidereon_tdm_free(tdm);
    return 0;
}
