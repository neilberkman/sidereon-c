/*
 * Focused G02/G04/G05/G06 C smoke. The NAV and CLK paths use the committed
 * public fixtures passed as argv[1] and argv[2]. The GLONASS and SBAS inputs
 * are public core-format test literals.
 *
 * The complete smoke is registered in run_ci_smoke.sh with committed fixtures.
 */
#include "sidereon.h"

#include <inttypes.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static void require_true(bool condition, const char *message) {
    if (!condition) {
        fprintf(stderr, "rinex_nav_clock_smoke: %s\n", message);
        exit(1);
    }
}

static const char *last_error(void) {
    static char message[1024];
    (void)sidereon_last_error_message(message, sizeof(message));
    return message;
}

static uint64_t f64_bits(double value) {
    uint64_t bits = 0;
    memcpy(&bits, &value, sizeof(bits));
    return bits;
}

static bool token_is(SidereonSatelliteToken token, const char *expected) {
    size_t length = strlen(expected);
    return length < sizeof(token.bytes) && memcmp(token.bytes, expected, length) == 0 &&
           token.bytes[length] == 0;
}

static uint8_t *read_file(const char *path, size_t *length) {
    FILE *file = fopen(path, "rb");
    require_true(file != NULL, "cannot open fixture");
    require_true(fseek(file, 0, SEEK_END) == 0, "cannot seek fixture");
    long end = ftell(file);
    require_true(end >= 0, "cannot size fixture");
    require_true(fseek(file, 0, SEEK_SET) == 0, "cannot rewind fixture");
    uint8_t *data = (uint8_t *)malloc((size_t)end);
    require_true(data != NULL || end == 0, "cannot allocate fixture");
    require_true(fread(data, 1, (size_t)end, file) == (size_t)end, "cannot read fixture");
    require_true(fclose(file) == 0, "cannot close fixture");
    *length = (size_t)end;
    return data;
}

static void assert_first_raw_record(const SidereonBroadcastRecord *record) {
    require_true(token_is(record->sat_id, "C05"), "raw first satellite mismatch");
    require_true(record->message == 8 && record->issue == 1 && record->issue_message == 8 &&
                     record->week == 755,
                 "raw message identity mismatch");
    require_true(record->toe.system == SIDEREON_TIME_SCALE_BDT && record->toe.week == 755 &&
                     f64_bits(record->toe.tow_s) == UINT64_C(0x4114a78000000000) &&
                     record->toc.system == SIDEREON_TIME_SCALE_BDT && record->toc.week == 755 &&
                     f64_bits(record->toc.tow_s) == UINT64_C(0x4114a78000000000),
                 "raw scale-tagged epochs mismatch");
    require_true(f64_bits(record->elements.sqrt_a) == UINT64_C(0x40b95d6102dfffec) &&
                     f64_bits(record->elements.e) == UINT64_C(0x3f3919de80000029) &&
                     f64_bits(record->elements.m0) == UINT64_C(0xbff1a0c3ba7cd0c8) &&
                     f64_bits(record->elements.delta_n) == UINT64_C(0xbe2afc5cbc1ab63e) &&
                     f64_bits(record->elements.omega0) == UINT64_C(0x400594a533dfae5a) &&
                     f64_bits(record->elements.i0) == UINT64_C(0x3fbd16a5fbf0d7d4) &&
                     f64_bits(record->elements.omega) == UINT64_C(0xbff06f1b51c380c4) &&
                     f64_bits(record->elements.omega_dot) == UINT64_C(0x3e319c9402189508) &&
                     f64_bits(record->elements.idot) == UINT64_C(0x3df6d35cc207ee20),
                 "raw primary orbit fields mismatch");
    require_true(f64_bits(record->elements.cuc) == UINT64_C(0xbeeca6bffffffbf0) &&
                     f64_bits(record->elements.cus) == UINT64_C(0xbee8b24000000a91) &&
                     f64_bits(record->elements.crc) == UINT64_C(0x40762fc000000000) &&
                     f64_bits(record->elements.crs) == UINT64_C(0xc079e4c000000000) &&
                     f64_bits(record->elements.cic) == UINT64_C(0xbe707fffffffff88) &&
                     f64_bits(record->elements.cis) == UINT64_C(0x3e707fffffffff88) &&
                     f64_bits(record->elements.toe_sow) == UINT64_C(0x4114a78000000000),
                 "raw complete orbit fields mismatch");
    require_true(f64_bits(record->clock.af0) == UINT64_C(0xbf40e400000000ca) &&
                     f64_bits(record->clock.af1) == UINT64_C(0xbdd2706fffffff97) &&
                     f64_bits(record->clock.af2) == UINT64_C(0) &&
                     f64_bits(record->clock.toc_sow) == UINT64_C(0x4114a78000000000),
                 "raw clock polynomial mismatch");
    require_true(f64_bits(record->sv_health) == UINT64_C(0) &&
                     f64_bits(record->sv_accuracy_m) == UINT64_C(0x4000000000000000) &&
                     !record->has_fit_interval_s && f64_bits(record->fit_interval_s) == UINT64_C(0),
                 "raw health accuracy or fit mismatch");
    require_true(!record->group_delays.has_gps_tgd_s &&
                     !record->group_delays.has_galileo_bgd_e5a_e1_s &&
                     !record->group_delays.has_galileo_bgd_e5b_e1_s &&
                     record->group_delays.has_beidou_tgd1_s &&
                     f64_bits(record->group_delays.beidou_tgd1_s) == UINT64_C(0x3ddb7cdfd9d7bdbb) &&
                     record->group_delays.has_beidou_tgd2_s &&
                     f64_bits(record->group_delays.beidou_tgd2_s) == UINT64_C(0xbe43f8baa446bfda) &&
                     !record->cnav.present,
                 "raw optional field presence mismatch");
}

static const char *glonass_fixture(void) {
    return "     3.05           NAVIGATION DATA     M                   RINEX VERSION / TYPE\n"
           "     XXX                                                         END OF HEADER\n"
           "R01 2020 06 24 23 15 00 6.355904042721e-05 0.000000000000e+00 3.420000000000e+05\n"
           "     1.090894238281e+04 1.407806396484e+00-1.862645149231e-09 0.000000000000e+00\n"
           "    -2.885726074219e+03 2.795855522156e+00-0.000000000000e+00 1.000000000000e+00\n"
           "     2.288353955078e+04-3.169984817505e-01-2.793967723846e-09 0.000000000000e+00\n";
}

static const char *glonass_extended_fixture(void) {
    return "     3.05           NAVIGATION DATA     M                   RINEX VERSION / TYPE\n"
           "     XXX                                                         END OF HEADER\n"
           "R28 2020 06 24 23 15 00 6.355904042721e-05 0.000000000000e+00 3.420000000000e+05\n"
           "     1.090894238281e+04 1.407806396484e+00-1.862645149231e-09 0.000000000000e+00\n"
           "    -2.885726074219e+03 2.795855522156e+00-0.000000000000e+00 1.000000000000e+00\n"
           "     2.288353955078e+04-3.169984817505e-01-2.793967723846e-09 0.000000000000e+00\n";
}

static void assert_glonass_record(const SidereonGlonassRecord *record) {
    require_true(token_is(record->sat_id, "R01") && record->freq_channel == 1,
                 "GLONASS identity mismatch");
    require_true(f64_bits(record->toe_utc_j2000_s) == UINT64_C(0x41c342f91a000000) &&
                     f64_bits(record->pos_m[0]) == UINT64_C(0x4164cea1cc3ffac2) &&
                     f64_bits(record->pos_m[1]) == UINT64_C(0xc146042f09800219) &&
                     f64_bits(record->pos_m[2]) == UINT64_C(0x4175d2cd38cffeb0),
                 "GLONASS position mismatch");
    require_true(f64_bits(record->vel_m_s[0]) == UINT64_C(0x4095ff39bffff98f) &&
                     f64_bits(record->vel_m_s[1]) == UINT64_C(0x40a5d7b60700020c) &&
                     f64_bits(record->vel_m_s[2]) == UINT64_C(0xc073cff9c80000ce),
                 "GLONASS velocity mismatch");
    require_true(f64_bits(record->acc_m_s2[0]) == UINT64_C(0xbebf4000000000cb) &&
                     f64_bits(record->acc_m_s2[1]) == UINT64_C(0x8000000000000000) &&
                     f64_bits(record->acc_m_s2[2]) == UINT64_C(0xbec76ffffffffbfc) &&
                     f64_bits(record->clk_bias) == UINT64_C(0x3f10a96000000098) &&
                     f64_bits(record->gamma_n) == UINT64_C(0) &&
                     f64_bits(record->sv_health) == UINT64_C(0),
                 "GLONASS acceleration or clock fields mismatch");
}

int main(int argc, char **argv) {
    require_true(argc == 3, "usage: rinex_nav_clock_smoke NAV_FIXTURE CLK_FIXTURE");
    size_t nav_len = 0;
    size_t clk_len = 0;
    uint8_t *nav = read_file(argv[1], &nav_len);
    uint8_t *clk = read_file(argv[2], &clk_len);

    SidereonRinexNavRecords *raw = NULL;
    require_true(sidereon_parse_rinex_nav_records(nav, nav_len, &raw) == SIDEREON_STATUS_OK,
                 last_error());
    size_t raw_count = 0;
    require_true(sidereon_rinex_nav_records_count(raw, &raw_count) == SIDEREON_STATUS_OK,
                 last_error());
    require_true(raw_count == 2216, "raw NAV count changed");
    SidereonBroadcastRecord record;
    require_true(sidereon_rinex_nav_records_item(raw, 0, &record) == SIDEREON_STATUS_OK,
                 last_error());
    assert_first_raw_record(&record);
    SidereonBroadcastRecord out_of_range_record;
    require_true(sidereon_rinex_nav_records_item(raw, raw_count, &out_of_range_record) ==
                     SIDEREON_STATUS_INVALID_ARGUMENT,
                 "raw NAV out-of-range item status mismatch");

    size_t written = 0;
    size_t required = 0;
    require_true(sidereon_encode_rinex_nav(&record, 1, NULL, 0, &written, &required) ==
                     SIDEREON_STATUS_OK,
                 last_error());
    require_true(written == 0 && required == 810, "NAV encoding size changed");
    uint8_t *encoded = (uint8_t *)malloc(required);
    require_true(encoded != NULL, "cannot allocate encoded NAV");
    require_true(sidereon_encode_rinex_nav(
                     &record, 1, encoded, required - 1, &written, &required) ==
                     SIDEREON_STATUS_INVALID_ARGUMENT &&
                     written == 0 && required == 810,
                 "NAV short-buffer status mismatch");
    require_true(sidereon_encode_rinex_nav(&record, 1, encoded, required, &written, &required) ==
                     SIDEREON_STATUS_OK &&
                     written == 810 && encoded[written - 1] == '\n',
                 last_error());
    static const char encoded_header[] =
        "     3.04           N: GNSS NAV DATA    M (MIXED)           RINEX VERSION / TYPE\n";
    require_true(sizeof(encoded_header) - 1 == 81 &&
                     memcmp(encoded, encoded_header, sizeof(encoded_header) - 1) == 0,
                 "NAV encoded header changed");
    SidereonRinexNavRecords *reparsed = NULL;
    require_true(sidereon_parse_rinex_nav_records(encoded, written, &reparsed) ==
                     SIDEREON_STATUS_OK,
                 last_error());
    size_t reparsed_count = 0;
    require_true(sidereon_rinex_nav_records_count(reparsed, &reparsed_count) ==
                     SIDEREON_STATUS_OK && reparsed_count == 1,
                 "encoded NAV reparse count changed");
    SidereonBroadcastRecord reparsed_record;
    require_true(sidereon_rinex_nav_records_item(reparsed, 0, &reparsed_record) ==
                     SIDEREON_STATUS_OK &&
                     f64_bits(reparsed_record.elements.sqrt_a) ==
                         UINT64_C(0x40b95d6102dfffec) &&
                     f64_bits(reparsed_record.clock.af0) == UINT64_C(0xbf40e400000000ca),
                 "encoded NAV representative fields changed");
    sidereon_rinex_nav_records_free(reparsed);
    require_true(sidereon_encode_rinex_nav(NULL, 1, NULL, 0, &written, &required) ==
                     SIDEREON_STATUS_NULL_POINTER && written == 0 && required == 0,
                 "NAV encoder null-record status mismatch");

    SidereonRinexNavParse *lenient = NULL;
    require_true(sidereon_parse_rinex_nav_lenient(nav, nav_len, &lenient) == SIDEREON_STATUS_OK,
                 last_error());
    size_t lenient_records = 0;
    size_t skipped = 0;
    require_true(sidereon_nav_parse_record_count(lenient, &lenient_records) ==
                     SIDEREON_STATUS_OK &&
                     sidereon_nav_parse_skipped_count(lenient, &skipped) == SIDEREON_STATUS_OK &&
                     lenient_records == 2216 && skipped == 0,
                 "lenient NAV counts changed");
    SidereonBroadcastRecord lenient_record;
    require_true(sidereon_nav_parse_record(lenient, 0, &lenient_record) == SIDEREON_STATUS_OK,
                 last_error());
    assert_first_raw_record(&lenient_record);
    SidereonRinexNavParse *bad_lenient = NULL;
    uint8_t *bad_nav = (uint8_t *)malloc(nav_len);
    require_true(bad_nav != NULL, "cannot allocate malformed NAV");
    memcpy(bad_nav, nav, nav_len);
    bool replaced = false;
    for (size_t i = 0; i + 8 <= nav_len; ++i) {
        if (memcmp(bad_nav + i, "C05 2020", 8) == 0) {
            memcpy(bad_nav + i + 4, "XXXX", 4);
            replaced = true;
            break;
        }
    }
    require_true(replaced, "cannot construct malformed NAV");
    require_true(sidereon_parse_rinex_nav_lenient(bad_nav, nav_len, &bad_lenient) ==
                     SIDEREON_STATUS_OK,
                 last_error());
    size_t bad_records = 0;
    size_t bad_skipped = 0;
    require_true(sidereon_nav_parse_record_count(bad_lenient, &bad_records) ==
                     SIDEREON_STATUS_OK &&
                     sidereon_nav_parse_skipped_count(bad_lenient, &bad_skipped) ==
                         SIDEREON_STATUS_OK &&
                     bad_records == 2215 && bad_skipped == 1,
                 "lenient malformed NAV counts changed");
    SidereonSkippedNavBlock diagnostic;
    require_true(sidereon_nav_parse_skipped(bad_lenient, 0, &diagnostic) == SIDEREON_STATUS_OK &&
                     token_is(diagnostic.satellite, "C05") &&
                     strcmp(diagnostic.message, "bad/missing toc epoch field in record for C05") == 0,
                 "lenient skipped diagnostic changed");
    const char expected_diagnostic[] = "bad/missing toc epoch field in record for C05";
    require_true(sizeof(expected_diagnostic) - 1 == 45, "diagnostic fixture length changed");
    require_true(sidereon_nav_parse_skipped_message(
                     bad_lenient, 0, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
                     written == 0 && required == sizeof(expected_diagnostic) - 1,
                 "diagnostic size query changed");
    uint8_t diagnostic_text[sizeof(expected_diagnostic) - 1];
    require_true(sidereon_nav_parse_skipped_message(bad_lenient, 0, diagnostic_text,
                                                    sizeof(diagnostic_text), &written, &required) ==
                     SIDEREON_STATUS_OK && written == sizeof(diagnostic_text) &&
                     memcmp(diagnostic_text, expected_diagnostic, sizeof(diagnostic_text)) == 0,
                 "diagnostic copy changed");
    SidereonRinexNavRecords *bad_raw = NULL;
    require_true(sidereon_parse_rinex_nav_records(bad_nav, nav_len, &bad_raw) ==
                     SIDEREON_STATUS_INVALID_ARGUMENT && bad_raw == NULL,
                 "strict malformed NAV status mismatch");
    SidereonRinexNavRecords *bad_utf8 = NULL;
    const uint8_t bad_utf8_data[] = {0xff};
    require_true(sidereon_parse_rinex_nav_records(bad_utf8_data, sizeof(bad_utf8_data), &bad_utf8) ==
                     SIDEREON_STATUS_INVALID_TOKEN && bad_utf8 == NULL,
                 "NAV malformed UTF-8 status mismatch");

    SidereonIonoCorrections iono;
    require_true(sidereon_parse_rinex_iono_corrections(nav, nav_len, &iono) ==
                     SIDEREON_STATUS_OK,
                 last_error());
    require_true(iono.gps.present && iono.galileo.present && !iono.beidou.present &&
                     f64_bits(iono.gps.alpha[0]) == UINT64_C(0x3e33fffc6065a2ca) &&
                     f64_bits(iono.gps.alpha[1]) == UINT64_C(0x3e4fffe950612c4b) &&
                     f64_bits(iono.gps.alpha[2]) == UINT64_C(0xbe7000063fca1753) &&
                     f64_bits(iono.gps.alpha[3]) == UINT64_C(0xbe8000063fca1753) &&
                     f64_bits(iono.gps.beta[0]) == UINT64_C(0x40f4000000000000) &&
                     f64_bits(iono.gps.beta[1]) == UINT64_C(0x40f8000000000000) &&
                     f64_bits(iono.gps.beta[2]) == UINT64_C(0xc0f0000000000000) &&
                     f64_bits(iono.gps.beta[3]) == UINT64_C(0xc120000400000000) &&
                     f64_bits(iono.galileo.ai0) == UINT64_C(0x403c400000000000) &&
                     f64_bits(iono.galileo.ai1) == UINT64_C(0x3f80000000000000) &&
                     f64_bits(iono.galileo.ai2) == UINT64_C(0x3f84a01abd1aa822),
                 "NAV ionosphere fields changed");
    double leap = 0.0;
    bool leap_present = false;
    require_true(sidereon_parse_rinex_leap_seconds(nav, nav_len, &leap, &leap_present) ==
                     SIDEREON_STATUS_OK && leap_present &&
                     f64_bits(leap) == UINT64_C(0x4032000000000000),
                 "NAV leap-second field changed");
    const uint8_t empty_header[] =
        "     3.05           NAVIGATION DATA     MIXED               RINEX VERSION / TYPE\n"
        "                                                            END OF HEADER\n";
    leap = 123.0;
    leap_present = true;
    require_true(sidereon_parse_rinex_leap_seconds(empty_header, sizeof(empty_header) - 1, &leap,
                                                   &leap_present) == SIDEREON_STATUS_OK &&
                     !leap_present && f64_bits(leap) == UINT64_C(0),
                 "absent NAV leap-second presence changed");

    SidereonRinexGlonassRecords *glonass_records = NULL;
    const char *glonass = glonass_fixture();
    require_true(sidereon_parse_rinex_glonass_records(
                     (const uint8_t *)glonass, strlen(glonass), &glonass_records) ==
                     SIDEREON_STATUS_OK,
                 last_error());
    size_t standalone_glonass_count = 0;
    require_true(sidereon_rinex_glonass_records_count(glonass_records,
                                                     &standalone_glonass_count) ==
                     SIDEREON_STATUS_OK && standalone_glonass_count == 1,
                 "standalone GLONASS count changed");
    SidereonGlonassRecord standalone_glonass;
    require_true(sidereon_rinex_glonass_records_item(glonass_records, 0, &standalone_glonass) ==
                     SIDEREON_STATUS_OK,
                 last_error());
    assert_glonass_record(&standalone_glonass);

    SidereonRinexGlonassRecords *extended_glonass_records = NULL;
    const char *extended_glonass = glonass_extended_fixture();
    require_true(sidereon_parse_rinex_glonass_records(
                     (const uint8_t *)extended_glonass, strlen(extended_glonass),
                     &extended_glonass_records) == SIDEREON_STATUS_OK,
                 last_error());
    size_t extended_record_count = SIZE_MAX;
    require_true(sidereon_rinex_glonass_records_count(extended_glonass_records,
                                                     &extended_record_count) ==
                     SIDEREON_STATUS_OK && extended_record_count == 0,
                 "extended GLONASS record was unexpectedly representable");
    size_t skipped_glonass_count = SIZE_MAX;
    require_true(sidereon_rinex_glonass_records_skipped_count(
                     extended_glonass_records, &skipped_glonass_count) ==
                     SIDEREON_STATUS_OK && skipped_glonass_count == 1,
                 "extended GLONASS skip count changed");
    SidereonSkippedGlonassRecord skipped_glonass;
    require_true(sidereon_rinex_glonass_records_skipped_item(
                     extended_glonass_records, 0, &skipped_glonass) == SIDEREON_STATUS_OK &&
                     token_is(skipped_glonass.satellite, "R28"),
                 "extended GLONASS skipped token changed");

    size_t combined_len = nav_len + strlen(glonass);
    uint8_t *combined_nav = (uint8_t *)malloc(combined_len);
    require_true(combined_nav != NULL, "cannot allocate combined NAV");
    memcpy(combined_nav, nav, nav_len);
    memcpy(combined_nav + nav_len, glonass, strlen(glonass));
    SidereonBroadcastEphemeris *broadcast = NULL;
    require_true(sidereon_broadcast_ephemeris_parse_nav(combined_nav, combined_len, &broadcast) ==
                     SIDEREON_STATUS_OK,
                 last_error());
    size_t broadcast_count = 0;
    size_t broadcast_glonass_count = 0;
    size_t channel_count = 0;
    require_true(sidereon_broadcast_ephemeris_record_count(broadcast, &broadcast_count) ==
                     SIDEREON_STATUS_OK &&
                     sidereon_broadcast_ephemeris_glonass_record_count(
                         broadcast, &broadcast_glonass_count) == SIDEREON_STATUS_OK &&
                     sidereon_broadcast_ephemeris_glonass_frequency_channel_count(
                         broadcast, &channel_count) == SIDEREON_STATUS_OK &&
                     broadcast_count == 1395 && broadcast_glonass_count == 1 && channel_count == 1,
                 "rich broadcast counts changed");
    require_true(sidereon_broadcast_ephemeris_records_full(
                     broadcast, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
                     written == 0 && required == broadcast_count,
                 "rich broadcast size query changed");
    SidereonBroadcastRecord rich_probe;
    require_true(sidereon_broadcast_ephemeris_records_full(
                     broadcast, &rich_probe, 1, &written, &required) ==
                     SIDEREON_STATUS_INVALID_ARGUMENT && written == 0 &&
                     required == broadcast_count,
                 "rich broadcast short-buffer status changed");
    SidereonBroadcastRecord *rich_records =
        (SidereonBroadcastRecord *)calloc(broadcast_count, sizeof(*rich_records));
    require_true(rich_records != NULL, "cannot allocate rich broadcast records");
    require_true(sidereon_broadcast_ephemeris_records_full(
                     broadcast, rich_records, broadcast_count, &written, &required) ==
                     SIDEREON_STATUS_OK && written == broadcast_count && required == broadcast_count,
                 last_error());
    assert_first_raw_record(&rich_records[0]);
    SidereonGlonassRecord *rich_glonass =
        (SidereonGlonassRecord *)calloc(broadcast_glonass_count, sizeof(*rich_glonass));
    require_true(rich_glonass != NULL, "cannot allocate rich GLONASS records");
    require_true(sidereon_broadcast_ephemeris_glonass_records(
                     broadcast, rich_glonass, broadcast_glonass_count, &written, &required) ==
                     SIDEREON_STATUS_OK && written == 1 && required == 1,
                 last_error());
    assert_glonass_record(&rich_glonass[0]);
    SidereonFrequencyChannel channel;
    require_true(sidereon_broadcast_ephemeris_glonass_frequency_channels(
                     broadcast, &channel, 1, &written, &required) == SIDEREON_STATUS_OK &&
                     written == 1 && required == 1 && channel.slot == 1 && channel.channel == 1,
                 last_error());
    require_true(sidereon_broadcast_ephemeris_iono_corrections(broadcast, &iono) ==
                     SIDEREON_STATUS_OK && iono.gps.present && iono.galileo.present,
                 last_error());
    require_true(sidereon_broadcast_ephemeris_leap_seconds(
                     broadcast, &leap, &leap_present) == SIDEREON_STATUS_OK && leap_present &&
                     f64_bits(leap) == UINT64_C(0x4032000000000000),
                 last_error());

    const char *ems_hex =
        "0000000000000000000000000000000000000000000000000000000000000000";
    const char *rtklib_hex =
        "0000000000000000000000000000000000000000000000000000000000";
    char ems[128];
    char rtklib[128];
    require_true(snprintf(ems, sizeof(ems), "120,26,7,1,0,0,1,1,%s\n", ems_hex) > 0 &&
                     snprintf(rtklib, sizeof(rtklib), "2360 259200 120 1 : %s\n", rtklib_hex) > 0,
                 "cannot construct SBAS logs");
    SidereonSbasLogBlocks *ems_blocks = NULL;
    SidereonSbasLogBlocks *rtklib_blocks = NULL;
    require_true(sidereon_parse_sbas_ems_lines(
                     (const uint8_t *)ems, strlen(ems), &ems_blocks) == SIDEREON_STATUS_OK &&
                     sidereon_parse_sbas_rtklib_lines(
                         (const uint8_t *)rtklib, strlen(rtklib), &rtklib_blocks) ==
                         SIDEREON_STATUS_OK,
                 last_error());
    size_t ems_count = 0;
    size_t rtklib_count = 0;
    require_true(sidereon_sbas_log_blocks_count(ems_blocks, &ems_count) == SIDEREON_STATUS_OK &&
                     sidereon_sbas_log_blocks_count(rtklib_blocks, &rtklib_count) ==
                         SIDEREON_STATUS_OK && ems_count == 1 && rtklib_count == 1,
                 "SBAS log counts changed");
    SidereonSbasLogBlock ems_block;
    SidereonSbasLogBlock rtklib_block;
    require_true(sidereon_sbas_log_blocks_item(ems_blocks, 0, &ems_block) == SIDEREON_STATUS_OK &&
                     token_is(ems_block.sat_id, "S20") &&
                     ems_block.epoch.system == SIDEREON_TIME_SCALE_GPST &&
                     ems_block.epoch.week == 2425 &&
                     f64_bits(ems_block.epoch.tow_s) == UINT64_C(0x410fa40800000000) &&
                     ems_block.form == SIDEREON_SBAS_WIRE_FORM_FRAMED250 &&
                     ems_block.byte_count == 32,
                 "EMS metadata changed");
    require_true(sidereon_sbas_log_blocks_item(rtklib_blocks, 0, &rtklib_block) ==
                     SIDEREON_STATUS_OK && token_is(rtklib_block.sat_id, "S20") &&
                     rtklib_block.epoch.system == SIDEREON_TIME_SCALE_GPST &&
                     rtklib_block.epoch.week == 2360 &&
                     f64_bits(rtklib_block.epoch.tow_s) == UINT64_C(0x410fa40000000000) &&
                     rtklib_block.form == SIDEREON_SBAS_WIRE_FORM_BODY226 &&
                     rtklib_block.byte_count == 29,
                 "RTKLIB metadata changed");
    uint8_t sbas_payload[32];
    require_true(sidereon_sbas_log_blocks_bytes(
                     ems_blocks, 0, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
                     written == 0 && required == 32,
                 "SBAS payload size query changed");
    require_true(sidereon_sbas_log_blocks_bytes(
                     ems_blocks, 0, sbas_payload, sizeof(sbas_payload) - 1, &written, &required) ==
                     SIDEREON_STATUS_INVALID_ARGUMENT && written == 0 && required == 32,
                 "SBAS payload short-buffer status changed");
    memset(sbas_payload, 0xff, sizeof(sbas_payload));
    require_true(sidereon_sbas_log_blocks_bytes(
                     ems_blocks, 0, sbas_payload, sizeof(sbas_payload), &written, &required) ==
                     SIDEREON_STATUS_OK && written == 32 && required == 32,
                 last_error());
    for (size_t i = 0; i < sizeof(sbas_payload); ++i) {
        require_true(sbas_payload[i] == 0, "EMS payload byte changed");
    }
    const uint8_t bad_sbas[] = {0xff};
    SidereonSbasLogBlocks *bad_sbas_blocks = NULL;
    require_true(sidereon_parse_sbas_ems_lines(
                     bad_sbas, sizeof(bad_sbas), &bad_sbas_blocks) == SIDEREON_STATUS_INVALID_TOKEN &&
                     bad_sbas_blocks == NULL,
                 "SBAS malformed UTF-8 status mismatch");

    SidereonRinexClock *clock = NULL;
    require_true(sidereon_rinex_clock_parse_lossy(clk, clk_len, &clock) == SIDEREON_STATUS_OK,
                 last_error());
    size_t satellite_count = 0;
    size_t sample_count = 0;
    require_true(sidereon_rinex_clock_series_count(clock, &satellite_count) ==
                     SIDEREON_STATUS_OK &&
                     sidereon_rinex_clock_sample_count(clock, &sample_count) ==
                         SIDEREON_STATUS_OK && satellite_count == 2 && sample_count == 5,
                 "clock lossy counts changed");
    require_true(sidereon_rinex_clock_satellites(
                     clock, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
                     written == 0 && required == 2,
                 "clock satellite size query changed");
    SidereonSatelliteToken satellites[2];
    require_true(sidereon_rinex_clock_satellites(
                     clock, satellites, 1, &written, &required) == SIDEREON_STATUS_INVALID_ARGUMENT &&
                     written == 0 && required == 2,
                 "clock satellite short-buffer status changed");
    require_true(sidereon_rinex_clock_satellites(
                     clock, satellites, 2, &written, &required) == SIDEREON_STATUS_OK &&
                     written == 2 && required == 2 && token_is(satellites[0], "G05") &&
                     token_is(satellites[1], "G24"),
                 "clock satellite ordering changed");
    SidereonClockSeries *series_by_index = NULL;
    require_true(sidereon_rinex_clock_series(clock, 0, &series_by_index) == SIDEREON_STATUS_OK &&
                     series_by_index != NULL,
                 last_error());
    SidereonSatelliteToken series_satellite;
    require_true(sidereon_rinex_clock_series_satellite(series_by_index, &series_satellite) ==
                     SIDEREON_STATUS_OK && token_is(series_satellite, "G05"),
                 last_error());
    SidereonClockSeries *series = NULL;
    require_true(sidereon_rinex_clock_series_for(clock, "G05", &series) == SIDEREON_STATUS_OK &&
                     series != NULL,
                 last_error());
    size_t g05_count = 0;
    require_true(sidereon_rinex_clock_series_sample_count(series, &g05_count) ==
                     SIDEREON_STATUS_OK && g05_count == 3,
                 "clock G05 sample count changed");
    require_true(sidereon_rinex_clock_series_samples(
                     series, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
                     written == 0 && required == 3,
                 "clock sample size query changed");
    SidereonClockPoint samples[3];
    require_true(sidereon_rinex_clock_series_samples(
                     series, samples, 2, &written, &required) == SIDEREON_STATUS_INVALID_ARGUMENT &&
                     written == 0 && required == 3,
                 "clock sample short-buffer status changed");
    require_true(sidereon_rinex_clock_series_samples(
                     series, samples, 3, &written, &required) == SIDEREON_STATUS_OK &&
                     written == 3,
                 last_error());
    require_true(samples[0].epoch.scale == SIDEREON_TIME_SCALE_GPST &&
                     samples[0].epoch.representation == 0 &&
                     f64_bits(samples[0].epoch.jd_whole) == UINT64_C(0x4142c6fac0000000) &&
                     f64_bits(samples[0].epoch.jd_fraction) == UINT64_C(0) &&
                     f64_bits(samples[0].bias_s) == UINT64_C(0xbf2a36e2eb1c432d),
                 "clock first sample changed");
    require_true(samples[1].epoch.scale == SIDEREON_TIME_SCALE_GPST &&
                     samples[1].epoch.representation == 0 &&
                     f64_bits(samples[1].epoch.jd_whole) == UINT64_C(0x4142c6fac0000000) &&
                     f64_bits(samples[1].epoch.jd_fraction) == UINT64_C(0x3f36c16c16c16c17) &&
                     f64_bits(samples[1].bias_s) == UINT64_C(0xbf2a36e36f0d4275),
                 "clock second sample changed");
    require_true(samples[2].epoch.scale == SIDEREON_TIME_SCALE_GPST &&
                     samples[2].epoch.representation == 0 &&
                     f64_bits(samples[2].epoch.jd_whole) == UINT64_C(0x4142c6fac0000000) &&
                     f64_bits(samples[2].epoch.jd_fraction) == UINT64_C(0x3f46c16c16c16c17) &&
                     f64_bits(samples[2].bias_s) == UINT64_C(0xbf2a36e3f2fe41be),
                 "clock third sample changed");
    SidereonClockSeries *missing_series = (SidereonClockSeries *)(uintptr_t)1;
    require_true(sidereon_rinex_clock_series_for(clock, "G99", &missing_series) ==
                     SIDEREON_STATUS_OK && missing_series == NULL,
                 "clock missing-series behavior changed");
    SidereonClockSeries *bad_series = NULL;
    const char bad_id[] = "G05\xff";
    require_true(sidereon_rinex_clock_series_for(clock, bad_id, &bad_series) ==
                     SIDEREON_STATUS_INVALID_TOKEN && bad_series == NULL,
                 "clock malformed satellite status mismatch");

    static const char malformed_as_clock[] =
        "     3.00           C                                       RINEX VERSION / TYPE\n"
        "                    GPS                                                         TIME SYSTEM ID\n"
        "                                                                        END OF HEADER\n"
        "AS G05  2026 05 13 00 00  bad-second  1   2.0e-04\n";
    SidereonRinexClock *malformed_lossy = NULL;
    require_true(sidereon_rinex_clock_parse_lossy(
                     (const uint8_t *)malformed_as_clock, sizeof(malformed_as_clock) - 1,
                     &malformed_lossy) == SIDEREON_STATUS_OK &&
                     malformed_lossy != NULL,
                 last_error());
    size_t malformed_sample_count = SIZE_MAX;
    require_true(sidereon_rinex_clock_sample_count(malformed_lossy, &malformed_sample_count) ==
                     SIDEREON_STATUS_OK &&
                     malformed_sample_count == 0,
                 "clock malformed AS row was not skipped");
    SidereonRinexClock *malformed_strict = (SidereonRinexClock *)(uintptr_t)1;
    require_true(sidereon_rinex_clock_parse(
                     (const uint8_t *)malformed_as_clock, sizeof(malformed_as_clock) - 1,
                     &malformed_strict) == SIDEREON_STATUS_INVALID_ARGUMENT &&
                     malformed_strict == NULL,
                 "clock strict malformed AS status mismatch");
    sidereon_rinex_clock_free(malformed_lossy);
    sidereon_rinex_clock_free(malformed_strict);

    sidereon_rinex_clock_series_free(NULL);
    sidereon_rinex_clock_series_free(series_by_index);
    sidereon_rinex_clock_series_free(series);
    sidereon_rinex_clock_free(NULL);
    sidereon_rinex_clock_free(clock);
    sidereon_sbas_log_blocks_free(NULL);
    sidereon_sbas_log_blocks_free(ems_blocks);
    sidereon_sbas_log_blocks_free(rtklib_blocks);
    sidereon_rinex_glonass_records_free(NULL);
    sidereon_rinex_glonass_records_free(extended_glonass_records);
    sidereon_rinex_glonass_records_free(glonass_records);
    sidereon_broadcast_ephemeris_free(NULL);
    sidereon_broadcast_ephemeris_free(broadcast);
    sidereon_nav_parse_free(NULL);
    sidereon_nav_parse_free(lenient);
    sidereon_nav_parse_free(bad_lenient);
    sidereon_rinex_nav_records_free(NULL);
    sidereon_rinex_nav_records_free(raw);
    free(rich_glonass);
    free(rich_records);
    free(combined_nav);
    free(bad_nav);
    free(encoded);
    free(nav);
    free(clk);
    puts("rinex_nav_clock_smoke: OK");
    return 0;
}
