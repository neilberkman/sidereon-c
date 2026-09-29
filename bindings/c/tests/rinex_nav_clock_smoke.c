/*
 * Focused G02/G04/G05/G06 C smoke. The NAV and CLK paths use the committed
 * public fixtures passed as argv[1] and argv[2]. The GLONASS and SBAS inputs
 * are public core-format test literals.
 *
 * The complete smoke is registered in run_ci_smoke.sh with committed fixtures.
 */
#include "sidereon.h"
#include "engine_pins_fixture.h"
#include "w3_rinex_nav_clock_pins.h"

#include <inttypes.h>
#include <math.h>
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

/* Every field of a parsed NAV record against sidereon-core's parse of the same
 * text (tests/valgen, bin w3_rinex_nav_clock). */
static void assert_nav_record(const SidereonBroadcastRecord *record,
                              const W3NcNavRecord *expected, const char *what) {
    const double elements[16] = {
        record->elements.sqrt_a,  record->elements.e,         record->elements.m0,
        record->elements.delta_n, record->elements.omega0,    record->elements.i0,
        record->elements.omega,   record->elements.omega_dot, record->elements.idot,
        record->elements.cuc,     record->elements.cus,       record->elements.crc,
        record->elements.crs,     record->elements.cic,       record->elements.cis,
        record->elements.toe_sow,
    };
    const double clock[4] = {record->clock.af0, record->clock.af1, record->clock.af2,
                             record->clock.toc_sow};
    const SidereonBroadcastGroupDelays *g = &record->group_delays;
    const bool has_delay[11] = {
        g->has_gps_tgd_s,       g->has_galileo_bgd_e5a_e1_s, g->has_galileo_bgd_e5b_e1_s,
        g->has_beidou_tgd1_s,   g->has_beidou_tgd2_s,        g->has_cnav_isc_l1ca_s,
        g->has_cnav_isc_l2c_s,  g->has_cnav_isc_l5i5_s,      g->has_cnav_isc_l5q5_s,
        g->has_cnav_isc_l1cd_s, g->has_cnav_isc_l1cp_s,
    };
    const double delay[11] = {
        g->gps_tgd_s,       g->galileo_bgd_e5a_e1_s, g->galileo_bgd_e5b_e1_s,
        g->beidou_tgd1_s,   g->beidou_tgd2_s,        g->cnav_isc_l1ca_s,
        g->cnav_isc_l2c_s,  g->cnav_isc_l5i5_s,      g->cnav_isc_l5q5_s,
        g->cnav_isc_l1cd_s, g->cnav_isc_l1cp_s,
    };
    require_true(token_is(record->sat_id, expected->sat), what);
    require_true(record->message == expected->message &&
                     record->has_issue == expected->has_issue &&
                     record->issue == expected->issue &&
                     record->issue_message == expected->issue_message &&
                     record->week == expected->week,
                 what);
    require_true(record->toe.system == expected->toe_system &&
                     record->toe.week == expected->toe_week &&
                     f64_bits(record->toe.tow_s) == expected->toe_tow_s &&
                     record->toc.system == expected->toc_system &&
                     record->toc.week == expected->toc_week &&
                     f64_bits(record->toc.tow_s) == expected->toc_tow_s,
                 what);
    for (size_t i = 0; i < 16; ++i) {
        require_true(f64_bits(elements[i]) == expected->elements[i], what);
    }
    for (size_t i = 0; i < 4; ++i) {
        require_true(f64_bits(clock[i]) == expected->clock[i], what);
    }
    require_true(f64_bits(record->sv_health) == expected->sv_health &&
                     record->has_sv_accuracy_m == expected->has_sv_accuracy_m &&
                     f64_bits(record->sv_accuracy_m) == expected->sv_accuracy_m &&
                     record->has_fit_interval_s == expected->has_fit_interval_s &&
                     f64_bits(record->fit_interval_s) == expected->fit_interval_s,
                 what);
    for (size_t i = 0; i < 11; ++i) {
        require_true(has_delay[i] == expected->has_group_delay[i] &&
                         f64_bits(delay[i]) == expected->group_delay[i],
                     what);
    }
    require_true(record->cnav.present == expected->cnav_present, what);
}

/* A RINEX 3.04 GLONASS record has four lines (3.05 added a fifth), and header
   labels sit in columns 61-80. */
static const char *glonass_fixture(void) {
    return "     3.04           NAVIGATION DATA     M                   RINEX VERSION / TYPE\n"
           "                                                            END OF HEADER\n"
           "R01 2020 06 24 23 15 00 6.355904042721e-05 0.000000000000e+00 3.420000000000e+05\n"
           "     1.090894238281e+04 1.407806396484e+00-1.862645149231e-09 0.000000000000e+00\n"
           "    -2.885726074219e+03 2.795855522156e+00-0.000000000000e+00 1.000000000000e+00\n"
           "     2.288353955078e+04-3.169984817505e-01-2.793967723846e-09 0.000000000000e+00\n";
}

static const char *glonass_extended_fixture(void) {
    return "     3.04           NAVIGATION DATA     M                   RINEX VERSION / TYPE\n"
           "                                                            END OF HEADER\n"
           "R28 2020 06 24 23 15 00 6.355904042721e-05 0.000000000000e+00 3.420000000000e+05\n"
           "     1.090894238281e+04 1.407806396484e+00-1.862645149231e-09 0.000000000000e+00\n"
           "    -2.885726074219e+03 2.795855522156e+00-0.000000000000e+00 1.000000000000e+00\n"
           "     2.288353955078e+04-3.169984817505e-01-2.793967723846e-09 0.000000000000e+00\n";
}

/* Every field of a parsed GLONASS record against sidereon-core's parse of the
 * same text (tests/valgen, bin w3_rinex_nav_clock). */
static void assert_glonass_record(const SidereonGlonassRecord *record,
                                  const W3NcGlonassRecord *expected, const char *what) {
    const bool has_optional[6] = {
        record->has_message_frame_time_s,      record->has_age_days,
        record->has_status_flags,              record->has_l1_l2_group_delay_field_s,
        record->has_urai,                      record->has_health_flags,
    };
    const double optional[6] = {
        record->message_frame_time_s,      record->age_days, record->status_flags,
        record->l1_l2_group_delay_field_s, record->urai,     record->health_flags,
    };
    require_true(token_is(record->sat_id, expected->sat) &&
                     record->freq_channel == expected->freq_channel &&
                     record->stated_freq_channel == expected->stated_freq_channel &&
                     f64_bits(record->toe_utc_j2000_s) == expected->toe_utc_j2000_s &&
                     f64_bits(record->epoch_utc_j2000_s) == expected->epoch_utc_j2000_s,
                 what);
    for (size_t i = 0; i < 3; ++i) {
        require_true(f64_bits(record->pos_m[i]) == expected->pos_m[i] &&
                         f64_bits(record->vel_m_s[i]) == expected->vel_m_s[i] &&
                         f64_bits(record->acc_m_s2[i]) == expected->acc_m_s2[i],
                     what);
    }
    require_true(f64_bits(record->clk_bias) == expected->clk_bias &&
                     f64_bits(record->gamma_n) == expected->gamma_n &&
                     f64_bits(record->sv_health) == expected->sv_health,
                 what);
    for (size_t i = 0; i < 6; ++i) {
        require_true(has_optional[i] == expected->has_optional[i] &&
                         f64_bits(optional[i]) == expected->optional[i],
                     what);
    }
}

/* Whether a variable-length text route succeeded and copied exactly `expected`. */
static bool copied_text_is(SidereonStatus status, const char *buffer, size_t written,
                           const char *expected) {
    return status == SIDEREON_STATUS_OK && written == strlen(expected) &&
           memcmp(buffer, expected, written) == 0;
}

/* RINEX clock 3.04 Table A18 example: calibration and discontinuity records
   and no TIME SYSTEM ID, which 3.04 requires. */
static const char clock_304_a18[] =
    "3.04                 C                                           RINEX VERSION / TYPE\n"
    "TORINEXC V9.9        USNO                 19960403  001000 UTC   PGM / RUN BY / DATE\n"
    "EXAMPLE OF A CLOCK DATA FILE                                     COMMENT\n"
    "IN THIS CASE CALIBRATION/DISCONTINUITY DATA GIVEN                COMMENT\n"
    "    10                                                           LEAP SECONDS GNSS\n"
    "     2    CR    DR                                               # / TYPES OF DATA\n"
    "USNO 40451S003                                                   STATION NAME / NUM\n"
    "UTC(USNO) MASTER CLOCK VIA CONTINUOUS CABLE MONITOR              STATION CLK REF\n"
    "                                                                 END OF HEADER\n"
    "CR USNO      1995 07 14 20 59 50.000000  2    0.123456789012E+00  -0.123456789012E-01\n"
    "CR USNO      1995 07 14 22 19 30.000000  2   -0.123456789012E+00   0.123456789012E-02\n"
    "DR USNO      1995 07 14 22 23 14.500000  2   -0.123456789012E+01   0.123456789012E+00\n"
    "CR USNO      1995 07 14 23 44 50.000000  2   -0.123456789012E+02   0.123456789012E+00\n";

/* Lines of an EMR0OPSRAP product: every AS record declares one value and
   carries its bias sigma in the sigma columns. */
static const char clock_200_surplus[] =
    "     2.00           CLOCK DATA                              RINEX VERSION / TYPE\n"
    "   GPS                                                      TIME SYSTEM ID      \n"
    "                                                            END OF HEADER       \n"
    "AR NRC1 2026 09 17 00 00  0.000000  2   -0.121218628367E-04  0.387783342482E-11\n"
    "AS G01  2026 09 17 00 00  0.000000  1    0.170710878415E-03  5.556437046250E-12\n";

static SidereonRinexClock *clock_parse_ok(const char *text, size_t length) {
    SidereonRinexClock *clock = NULL;
    SidereonRinexClockResult *result = NULL;
    require_true(sidereon_rinex_clock_parse_result((const uint8_t *)text, length, &clock,
                                                   &result) == SIDEREON_STATUS_OK &&
                     clock != NULL && result != NULL,
                 last_error());
    SidereonRinexClockOutcome outcome;
    require_true(sidereon_rinex_clock_result_get_outcome(result, &outcome) ==
                         SIDEREON_STATUS_OK &&
                     outcome.is_ok && outcome.error.kind == SIDEREON_RINEX_CLOCK_ERROR_KIND_NONE,
                 "clock parse outcome not ok");
    sidereon_rinex_clock_result_free(result);
    return clock;
}

static bool clock_writes(const SidereonRinexClock *clock, const char *expected) {
    static char text[8192];
    size_t written = 0;
    size_t required = 0;
    SidereonStatus status =
        sidereon_rinex_clock_to_text(clock, (uint8_t *)text, sizeof(text), &written, &required);
    return copied_text_is(status, text, written, expected);
}

/* The values each check compares against are sidereon-core's reading of the
 * same text, written by tests/valgen (bin w3_rinex_nav_clock). */
static void check_clock_table_a18(void) {
    SidereonRinexClock *clock = clock_parse_ok(clock_304_a18, sizeof(clock_304_a18) - 1);
    require_true(clock_writes(clock, clock_304_a18), "A18 product not restated byte for byte");

    SidereonRinexClockInfo info;
    require_true(sidereon_rinex_clock_info(clock, &info) == SIDEREON_STATUS_OK, last_error());
    require_true(info.has_version == W3NC_A18_HAS_VERSION &&
                     f64_bits(info.version) == W3NC_A18_VERSION &&
                     info.has_layout == W3NC_A18_HAS_LAYOUT && info.layout == W3NC_A18_LAYOUT &&
                     info.has_time_system == W3NC_A18_HAS_TIME_SYSTEM &&
                     info.time_system == W3NC_A18_TIME_SYSTEM &&
                     info.time_system_status == W3NC_A18_TIME_SYSTEM_STATUS &&
                     info.has_time_scale == W3NC_A18_HAS_TIME_SCALE &&
                     info.time_scale == W3NC_A18_TIME_SCALE,
                 "A18 time system changed");
    require_true(info.header_record_count == W3NC_A18_HEADER_RECORD_COUNT &&
                     info.record_count == W3NC_A18_RECORD_COUNT &&
                     info.series_count == W3NC_A18_SERIES_COUNT &&
                     info.sample_count == W3NC_A18_SAMPLE_COUNT &&
                     info.skipped_record_count == W3NC_A18_SKIPPED_COUNT &&
                     info.diagnostic_count == W3NC_A18_DIAGNOSTIC_COUNT &&
                     info.notice_count == W3NC_A18_NOTICE_COUNT,
                 "A18 counts changed");

    SidereonClockNotice notices[W3NC_A18_NOTICE_COUNT];
    size_t written = 0;
    size_t required = 0;
    require_true(sidereon_rinex_clock_notices(clock, notices, W3NC_A18_NOTICE_COUNT, &written,
                                              &required) == SIDEREON_STATUS_OK &&
                     written == W3NC_A18_NOTICE_COUNT,
                 last_error());
    require_true(notices[0].kind == W3NC_A18_NOTICE0_KIND &&
                     notices[0].has_line == W3NC_A18_NOTICE0_HAS_LINE &&
                     notices[0].line == W3NC_A18_NOTICE0_LINE &&
                     notices[1].kind == W3NC_A18_NOTICE1_KIND &&
                     notices[1].has_time_system == W3NC_A18_NOTICE1_HAS_TIME_SYSTEM &&
                     notices[1].time_system == W3NC_A18_NOTICE1_TIME_SYSTEM &&
                     notices[2].kind == W3NC_A18_NOTICE2_KIND,
                 "A18 notices changed");

    SidereonClockSkip skips[W3NC_A18_SKIPPED_COUNT];
    require_true(sidereon_rinex_clock_skipped_records(clock, skips, W3NC_A18_SKIPPED_COUNT,
                                                      &written, &required) ==
                         SIDEREON_STATUS_OK &&
                     written == W3NC_A18_SKIPPED_COUNT && skips[0].line == W3NC_A18_SKIP0_LINE &&
                     skips[0].record_type == W3NC_A18_SKIP0_TYPE &&
                     skips[1].line == W3NC_A18_SKIP1_LINE &&
                     skips[1].record_type == W3NC_A18_SKIP1_TYPE &&
                     skips[2].line == W3NC_A18_SKIP2_LINE &&
                     skips[2].record_type == W3NC_A18_SKIP2_TYPE &&
                     skips[3].line == W3NC_A18_SKIP3_LINE &&
                     skips[3].record_type == W3NC_A18_SKIP3_TYPE,
                 "A18 skipped records changed");

    SidereonClockRecords *records = NULL;
    require_true(sidereon_rinex_clock_records(clock, &records) == SIDEREON_STATUS_OK &&
                     records != NULL,
                 last_error());
    SidereonClockRecord rows[W3NC_A18_RECORDS_LEN];
    require_true(sidereon_clock_records_copy(records, rows, W3NC_A18_RECORDS_LEN, &written,
                                             &required) == SIDEREON_STATUS_OK &&
                     written == W3NC_A18_RECORDS_LEN,
                 last_error());
    const SidereonClockRecord *dr = &rows[2];
    require_true(dr->record_type == W3NC_A18_RECORD2_TYPE &&
                     dr->has_satellite == W3NC_A18_RECORD2_HAS_SATELLITE &&
                     dr->civil_epoch.year == W3NC_A18_RECORD2_YEAR &&
                     dr->civil_epoch.month == W3NC_A18_RECORD2_MONTH &&
                     dr->civil_epoch.day == W3NC_A18_RECORD2_DAY &&
                     dr->civil_epoch.hour == W3NC_A18_RECORD2_HOUR &&
                     dr->civil_epoch.minute == W3NC_A18_RECORD2_MINUTE &&
                     f64_bits(dr->civil_epoch.second) == W3NC_A18_RECORD2_SECOND &&
                     dr->has_epoch == W3NC_A18_RECORD2_HAS_EPOCH &&
                     dr->epoch.scale == W3NC_A18_RECORD2_EPOCH_SCALE &&
                     dr->value_count == W3NC_A18_RECORD2_VALUE_COUNT &&
                     f64_bits(dr->values[0]) == W3NC_A18_RECORD2_VALUE0 &&
                     f64_bits(dr->values[1]) == W3NC_A18_RECORD2_VALUE1 &&
                     isnan(dr->values[2]) &&
                     dr->surplus_count == W3NC_A18_RECORD2_SURPLUS_COUNT &&
                     dr->has_line == W3NC_A18_RECORD2_HAS_LINE &&
                     dr->line == W3NC_A18_RECORD2_LINE &&
                     dr->line_count == W3NC_A18_RECORD2_LINE_COUNT &&
                     dr->reading == W3NC_A18_RECORD2_READING &&
                     dr->has_continuation_reading == W3NC_A18_RECORD2_HAS_CONTINUATION_READING,
                 "A18 DR record changed");
    char name[32];
    require_true(copied_text_is(sidereon_clock_records_name(records, 2, (uint8_t *)name,
                                                            sizeof(name), &written, &required),
                                name, written, W3NC_A18_RECORD2_NAME),
                 "A18 record name changed");
    sidereon_clock_records_free(records);

    SidereonClockHeaderRecords *header = NULL;
    require_true(sidereon_rinex_clock_header_records(clock, &header) == SIDEREON_STATUS_OK &&
                     header != NULL,
                 last_error());
    size_t header_count = 0;
    require_true(sidereon_clock_header_records_count(header, &header_count) ==
                         SIDEREON_STATUS_OK &&
                     header_count == W3NC_A18_HEADER_RECORD_COUNT,
                 "A18 header count changed");
    SidereonClockHeaderRecord leap;
    require_true(sidereon_clock_header_records_get(header, 4, &leap) == SIDEREON_STATUS_OK &&
                     leap.field_kind == W3NC_A18_HEADER4_FIELD_KIND &&
                     leap.has_integer == W3NC_A18_HEADER4_HAS_INTEGER &&
                     leap.integer == W3NC_A18_HEADER4_INTEGER &&
                     leap.has_line == W3NC_A18_HEADER4_HAS_LINE &&
                     leap.line == W3NC_A18_HEADER4_LINE &&
                     leap.label_column == W3NC_A18_HEADER4_LABEL_COLUMN,
                 "A18 LEAP SECONDS GNSS changed");
    char label[64];
    require_true(copied_text_is(sidereon_clock_header_records_text(
                                    header, 4, SIDEREON_CLOCK_HEADER_TEXT_LABEL,
                                    (uint8_t *)label, sizeof(label), &written, &required),
                                label, written, W3NC_A18_HEADER4_LABEL),
                 "A18 header label changed");
    SidereonClockHeaderRecord types;
    require_true(sidereon_clock_header_records_get(header, 5, &types) == SIDEREON_STATUS_OK &&
                     types.field_kind == W3NC_A18_HEADER5_FIELD_KIND &&
                     types.has_count == W3NC_A18_HEADER5_HAS_COUNT &&
                     types.count == W3NC_A18_HEADER5_COUNT &&
                     types.text_part_count == W3NC_A18_HEADER5_TEXT_PART_COUNT,
                 "A18 TYPES OF DATA changed");
    char part[16];
    require_true(copied_text_is(sidereon_clock_header_records_field_text(
                                    header, 5, 1, (uint8_t *)part, sizeof(part), &written,
                                    &required),
                                part, written, W3NC_A18_HEADER5_PART1),
                 "A18 TYPES OF DATA part changed");
    sidereon_clock_header_records_free(header);

    /* Declaring the time system inserts one TIME SYSTEM ID record at the 3.04
       columns before LEAP SECONDS GNSS; every other line is unchanged. */
    SidereonRinexClockResult *result = NULL;
    require_true(sidereon_rinex_clock_set_time_system(clock, SIDEREON_CLOCK_TIME_SYSTEM_GPS,
                                                      &result) == SIDEREON_STATUS_OK &&
                     result != NULL,
                 last_error());
    sidereon_rinex_clock_result_free(result);
    require_true(clock_writes(clock, W3NC_A18_DECLARED_TEXT), "declared A18 product changed");
    require_true(sidereon_rinex_clock_info(clock, &info) == SIDEREON_STATUS_OK &&
                     info.time_system_status == W3NC_A18_DECLARED_TIME_SYSTEM_STATUS &&
                     info.header_record_count == W3NC_A18_DECLARED_HEADER_RECORD_COUNT,
                 "declared A18 status changed");
    sidereon_rinex_clock_free(clock);
}

static void check_clock_surplus_and_edits(void) {
    SidereonRinexClock *clock =
        clock_parse_ok(clock_200_surplus, sizeof(clock_200_surplus) - 1);
    SidereonClockNotice notice;
    size_t written = 0;
    size_t required = 0;
    require_true(sidereon_rinex_clock_notices(clock, &notice, 1, &written, &required) ==
                         SIDEREON_STATUS_OK &&
                     written == W3NC_SURPLUS_NOTICE_COUNT &&
                     notice.kind == W3NC_SURPLUS_NOTICE0_KIND &&
                     notice.has_records == W3NC_SURPLUS_NOTICE0_HAS_RECORDS &&
                     notice.records == W3NC_SURPLUS_NOTICE0_RECORDS &&
                     notice.first_line == W3NC_SURPLUS_NOTICE0_FIRST_LINE,
                 "surplus notice changed");

    SidereonClockRecords *records = NULL;
    require_true(sidereon_rinex_clock_records(clock, &records) == SIDEREON_STATUS_OK, last_error());
    SidereonClockRecord as_record;
    require_true(sidereon_clock_records_get(records, 1, &as_record) == SIDEREON_STATUS_OK &&
                     as_record.record_type == W3NC_SURPLUS_RECORD1_TYPE &&
                     as_record.has_satellite == W3NC_SURPLUS_RECORD1_HAS_SATELLITE &&
                     token_is(as_record.satellite, W3NC_SURPLUS_RECORD1_SATELLITE) &&
                     as_record.value_count == W3NC_SURPLUS_RECORD1_VALUE_COUNT &&
                     f64_bits(as_record.values[0]) == W3NC_SURPLUS_RECORD1_VALUE0 &&
                     as_record.surplus_count == W3NC_SURPLUS_RECORD1_SURPLUS_COUNT &&
                     as_record.surplus[0].position == W3NC_SURPLUS_RECORD1_SURPLUS0_POSITION &&
                     f64_bits(as_record.surplus[0].value) == W3NC_SURPLUS_RECORD1_SURPLUS0_VALUE &&
                     as_record.reading == W3NC_SURPLUS_RECORD1_READING,
                 "surplus AS record changed");
    sidereon_clock_records_free(records);

    /* The series sample keeps the declared values only. */
    SidereonClockSeries *series = NULL;
    SidereonClockPoint point;
    require_true(sidereon_rinex_clock_series_for(clock, "G01", &series) == SIDEREON_STATUS_OK &&
                     series != NULL &&
                     sidereon_rinex_clock_series_samples(series, &point, 1, &written,
                                                         &required) == SIDEREON_STATUS_OK &&
                     written == 1 &&
                     point.additional_value_count == W3NC_SURPLUS_G01_ADDITIONAL_COUNT &&
                     isnan(point.additional_values[0]),
                 "surplus sample changed");
    sidereon_rinex_clock_series_free(series);

    /* An edit that would drop the surplus sigma changes nothing. The C status
     * of a refused edit is SIDEREON_STATUS_INVALID_ARGUMENT
     * (clock_result_refused, src/rinex_clock.rs). */
    SidereonRinexClockResult *result = NULL;
    require_true(sidereon_rinex_clock_set_record_values(clock, 1, W3NC_SURPLUS_BIAS_ONLY, 1,
                                                        &result) ==
                         SIDEREON_STATUS_INVALID_ARGUMENT &&
                     result != NULL,
                 "surplus-dropping edit not refused");
    SidereonRinexClockOutcome outcome;
    char field[32];
    require_true(sidereon_rinex_clock_result_get_outcome(result, &outcome) ==
                         SIDEREON_STATUS_OK &&
                     !outcome.is_ok && outcome.error.kind == W3NC_SURPLUS_DROP_ERROR_KIND &&
                     outcome.error.has_field && outcome.error.has_reason &&
                     !outcome.error.has_line &&
                     copied_text_is(sidereon_rinex_clock_result_error_text(
                                        result, SIDEREON_RINEX_CLOCK_ERROR_TEXT_FIELD,
                                        (uint8_t *)field, sizeof(field), &written, &required),
                                    field, written, W3NC_SURPLUS_DROP_ERROR_FIELD),
                 "surplus-dropping refusal not typed");
    sidereon_rinex_clock_result_free(result);
    require_true(clock_writes(clock, clock_200_surplus), "refused edit changed the product");

    /* Restating the sigma as a declared value is accepted. */
    require_true(sidereon_rinex_clock_set_record_values(clock, 1, W3NC_SURPLUS_BIAS_AND_SIGMA, 2,
                                                        NULL) == SIDEREON_STATUS_OK,
                 last_error());

    /* Insert a receiver record after the last one, then remove the first. */
    SidereonClockCivilEpoch epoch = W3NC_SURPLUS_INSERT_EPOCH_INIT;
    require_true(sidereon_rinex_clock_insert_record(clock, 2, SIDEREON_CLOCK_RECORD_TYPE_AR,
                                                    "WTZR", &epoch, W3NC_SURPLUS_INSERT_VALUES,
                                                    1, NULL) == SIDEREON_STATUS_OK,
                 last_error());
    require_true(sidereon_rinex_clock_remove_record(clock, 0, &result) == SIDEREON_STATUS_OK &&
                     result != NULL,
                 last_error());
    bool present = false;
    SidereonClockRecord removed;
    char removed_name[16];
    require_true(sidereon_rinex_clock_result_record(result, &present, &removed) ==
                         SIDEREON_STATUS_OK &&
                     present && removed.record_type == W3NC_SURPLUS_REMOVED_TYPE &&
                     removed.value_count == W3NC_SURPLUS_REMOVED_VALUE_COUNT &&
                     f64_bits(removed.values[0]) == W3NC_SURPLUS_REMOVED_VALUE0 &&
                     f64_bits(removed.values[1]) == W3NC_SURPLUS_REMOVED_VALUE1 &&
                     copied_text_is(sidereon_rinex_clock_result_record_name(
                                        result, (uint8_t *)removed_name, sizeof(removed_name),
                                        &written, &required),
                                    removed_name, written, W3NC_SURPLUS_REMOVED_NAME),
                 "removed record changed");
    sidereon_rinex_clock_result_free(result);

    /* The edited product writes, and reads back with every value. */
    static char text[4096];
    require_true(sidereon_rinex_clock_to_text(clock, (uint8_t *)text, sizeof(text), &written,
                                              &required) == SIDEREON_STATUS_OK,
                 last_error());
    SidereonRinexClock *reread = clock_parse_ok(text, written);
    size_t record_count = 0;
    require_true(sidereon_rinex_clock_record_count(reread, &record_count) == SIDEREON_STATUS_OK &&
                     record_count == W3NC_SURPLUS_REREAD_RECORD_COUNT,
                 "edited product record count changed");
    require_true(sidereon_rinex_clock_records(reread, &records) == SIDEREON_STATUS_OK,
                 last_error());
    SidereonClockRecord reread_rows[W3NC_SURPLUS_REREAD_RECORD_COUNT];
    require_true(sidereon_clock_records_copy(records, reread_rows,
                                             W3NC_SURPLUS_REREAD_RECORD_COUNT, &written,
                                             &required) == SIDEREON_STATUS_OK &&
                     reread_rows[0].record_type == W3NC_SURPLUS_REREAD0_TYPE &&
                     reread_rows[0].value_count == W3NC_SURPLUS_REREAD0_VALUE_COUNT &&
                     reread_rows[0].surplus_count == W3NC_SURPLUS_REREAD0_SURPLUS_COUNT &&
                     f64_bits(reread_rows[0].values[0]) == W3NC_SURPLUS_REREAD0_VALUE0 &&
                     f64_bits(reread_rows[0].values[1]) == W3NC_SURPLUS_REREAD0_VALUE1 &&
                     reread_rows[1].record_type == W3NC_SURPLUS_REREAD1_TYPE &&
                     f64_bits(reread_rows[1].civil_epoch.second) == W3NC_SURPLUS_REREAD1_SECOND &&
                     reread_rows[1].value_count == W3NC_SURPLUS_REREAD1_VALUE_COUNT &&
                     f64_bits(reread_rows[1].values[0]) == W3NC_SURPLUS_REREAD1_VALUE0,
                 "edited product did not read back");
    require_true(copied_text_is(sidereon_clock_records_name(records, 1, (uint8_t *)removed_name,
                                                            sizeof(removed_name), &written,
                                                            &required),
                                removed_name, written, W3NC_SURPLUS_REREAD1_NAME),
                 "inserted record name changed");
    sidereon_clock_records_free(records);
    sidereon_rinex_clock_free(reread);
    sidereon_rinex_clock_free(clock);
}

static SidereonClockSatellitePoint clock_input_point(const char *satellite,
                                                     SidereonClockEpoch epoch, double bias_s) {
    SidereonClockSatellitePoint entry;
    memset(&entry, 0, sizeof(entry));
    size_t length = strlen(satellite);
    memcpy(entry.satellite.bytes, satellite, length);
    entry.point.epoch = epoch;
    entry.point.bias_s = bias_s;
    entry.point.additional_value_count = 0;
    return entry;
}

static SidereonClockEpoch clock_epoch_at(uint32_t scale, int32_t year, uint8_t month,
                                         uint8_t day, uint8_t hour, uint8_t minute,
                                         double second) {
    SidereonClockEpoch epoch;
    bool available = false;
    require_true(sidereon_civil_to_clock_epoch(scale, year, month, day, hour, minute, second,
                                               &epoch, &available) == SIDEREON_STATUS_OK &&
                     available,
                 "civil clock epoch not available");
    return epoch;
}

/* tests/valgen (bin w3_rinex_nav_clock) builds the same products from the same
 * points and states sidereon-core's answers. */
static void check_clock_built_products(void) {
    /* A UTC product answers a query on the 2016-12-31 leap second, and
       interpolates across it by elapsed time. */
    SidereonClockSatellitePoint utc_points[2] = {
        clock_input_point("G01", clock_epoch_at(SIDEREON_TIME_SCALE_UTC, 2016, 12, 31, 23, 59,
                                                59.0),
                          1.0e-6),
        clock_input_point("G01", clock_epoch_at(SIDEREON_TIME_SCALE_UTC, 2017, 1, 1, 0, 0, 0.0),
                          3.0e-6),
    };
    SidereonRinexClock *utc = NULL;
    require_true(sidereon_rinex_clock_from_points(SIDEREON_TIME_SCALE_UTC, utc_points, 2, &utc,
                                                  NULL) == SIDEREON_STATUS_OK &&
                     utc != NULL,
                 last_error());
    SidereonClockCivilEpoch leap = {2016, 12, 31, 23, 59, 60.0};
    double bias = 0.0;
    bool available = false;
    require_true(sidereon_rinex_clock_bias_at_civil(utc, "G01", &leap, &bias, &available) ==
                         SIDEREON_STATUS_OK &&
                     available == W3NC_UTC_LEAP_AVAILABLE &&
                     f64_bits(bias) == W3NC_UTC_LEAP_BIAS,
                 "UTC leap-second query changed");
    SidereonClockEpoch gps_leap;
    require_true(sidereon_civil_to_clock_epoch(SIDEREON_TIME_SCALE_GPST, 2016, 12, 31, 23, 59,
                                               60.0, &gps_leap, &available) ==
                         SIDEREON_STATUS_OK &&
                     available == W3NC_GPS_LEAP_LABEL_AVAILABLE && isnan(gps_leap.jd_whole),
                 "GPS time accepted a leap-second label");
    SidereonRinexClockInfo info;
    require_true(sidereon_rinex_clock_info(utc, &info) == SIDEREON_STATUS_OK &&
                     info.time_system == W3NC_UTC_TIME_SYSTEM &&
                     info.time_system_status == W3NC_UTC_TIME_SYSTEM_STATUS &&
                     info.header_record_count == W3NC_UTC_HEADER_RECORD_COUNT &&
                     info.record_count == W3NC_UTC_RECORD_COUNT,
                 "built UTC product summary changed");
    sidereon_rinex_clock_free(utc);

    /* An epoch no microsecond text states is refused by the strict writer and
       written at the nearest microsecond, and reported, when allowed. */
    SidereonClockEpoch off_grid = clock_epoch_at(SIDEREON_TIME_SCALE_GPST, 2020, 1, 1, 0, 0, 30.0);
    off_grid.jd_fraction += 1.0e-12;
    SidereonClockSatellitePoint gps_points[2] = {
        clock_input_point("G01", clock_epoch_at(SIDEREON_TIME_SCALE_GPST, 2020, 1, 1, 0, 0, 0.0),
                          1.0e-6),
        clock_input_point("G01", off_grid, 2.0e-6),
    };
    SidereonRinexClock *gps = NULL;
    require_true(sidereon_rinex_clock_from_points(SIDEREON_TIME_SCALE_GPST, gps_points, 2, &gps,
                                                  NULL) == SIDEREON_STATUS_OK,
                 last_error());
    SidereonRinexClockResult *result = NULL;
    SidereonRinexClockOutcome outcome;
    size_t written = 0;
    size_t required = 0;
    char field[32];
    require_true(sidereon_rinex_clock_to_text_result(gps, NULL, &result) == SIDEREON_STATUS_OK &&
                     sidereon_rinex_clock_result_get_outcome(result, &outcome) ==
                         SIDEREON_STATUS_OK &&
                     !outcome.is_ok &&
                     outcome.error.kind == W3NC_OFF_GRID_STRICT_ERROR_KIND &&
                     copied_text_is(sidereon_rinex_clock_result_error_text(
                                        result, SIDEREON_RINEX_CLOCK_ERROR_TEXT_FIELD,
                                        (uint8_t *)field, sizeof(field), &written, &required),
                                    field, written, W3NC_OFF_GRID_STRICT_ERROR_FIELD),
                 "off-grid epoch not refused by the strict writer");
    require_true(sidereon_rinex_clock_result_get_text(result, NULL, 0, &written, &required) ==
                         SIDEREON_STATUS_INVALID_ARGUMENT &&
                     written == 0 && required == 0,
                 "refused write carried text");
    sidereon_rinex_clock_result_free(result);

    SidereonClockWritePolicy policy;
    require_true(sidereon_clock_write_policy_init(&policy) == SIDEREON_STATUS_OK &&
                     policy.nearest_microsecond_epochs == SIDEREON_CLOCK_WRITE_LENIENCY_STRICT,
                 "write policy default changed");
    policy.nearest_microsecond_epochs = SIDEREON_CLOCK_WRITE_LENIENCY_ALLOW;
    require_true(sidereon_rinex_clock_to_text_result(gps, &policy, &result) ==
                         SIDEREON_STATUS_OK &&
                     sidereon_rinex_clock_result_get_outcome(result, &outcome) ==
                         SIDEREON_STATUS_OK &&
                     outcome.is_ok,
                 "lenient write refused");
    SidereonClockWriteDeparture departure;
    require_true(sidereon_rinex_clock_result_departures(result, &departure, 1, &written,
                                                        &required) == SIDEREON_STATUS_OK &&
                     written == W3NC_OFF_GRID_DEPARTURE_COUNT &&
                     required == W3NC_OFF_GRID_DEPARTURE_COUNT &&
                     departure.kind == W3NC_OFF_GRID_DEPARTURE0_KIND &&
                     departure.record == W3NC_OFF_GRID_DEPARTURE0_RECORD &&
                     departure.has_epoch == W3NC_OFF_GRID_DEPARTURE0_HAS_EPOCH &&
                     f64_bits(departure.epoch.jd_fraction) == f64_bits(off_grid.jd_fraction),
                 "departure changed");
    char departure_text[64];
    require_true(copied_text_is(sidereon_rinex_clock_result_departure_text(
                                    result, 0, SIDEREON_CLOCK_DEPARTURE_TEXT_WRITTEN,
                                    (uint8_t *)departure_text, sizeof(departure_text), &written,
                                    &required),
                                departure_text, written, W3NC_OFF_GRID_DEPARTURE0_WRITTEN) &&
                     copied_text_is(sidereon_rinex_clock_result_departure_text(
                                        result, 0, SIDEREON_CLOCK_DEPARTURE_TEXT_NAME,
                                        (uint8_t *)departure_text, sizeof(departure_text),
                                        &written, &required),
                                    departure_text, written, W3NC_OFF_GRID_DEPARTURE0_NAME),
                 "departure text changed");
    require_true(sidereon_rinex_clock_result_get_text(result, NULL, 0, &written, &required) ==
                         SIDEREON_STATUS_OK &&
                     required > 0,
                 "lenient write carried no text");
    sidereon_rinex_clock_result_free(result);
    sidereon_rinex_clock_free(gps);

    /* GLONASS system time has no RINEX clock time system: GLO names UTC hours. */
    SidereonClockEpoch glonass_epoch =
        clock_epoch_at(SIDEREON_TIME_SCALE_GPST, 2020, 1, 1, 0, 0, 0.0);
    glonass_epoch.scale = SIDEREON_TIME_SCALE_GLONASST;
    SidereonClockSatellitePoint glonass_point = clock_input_point("R01", glonass_epoch, 1.0e-6);
    SidereonRinexClock *glonass = NULL;
    require_true(sidereon_rinex_clock_from_points(SIDEREON_TIME_SCALE_GLONASST, &glonass_point, 1,
                                                  &glonass, NULL) == SIDEREON_STATUS_OK,
                 last_error());
    require_true(sidereon_rinex_clock_to_text_result(glonass, NULL, &result) ==
                         SIDEREON_STATUS_OK &&
                     sidereon_rinex_clock_result_get_outcome(result, &outcome) ==
                         SIDEREON_STATUS_OK &&
                     !outcome.is_ok &&
                     outcome.error.kind == W3NC_GLONASS_WRITE_ERROR_KIND &&
                     outcome.error.has_time_scale &&
                     outcome.error.time_scale == W3NC_GLONASS_WRITE_ERROR_TIME_SCALE,
                 "GLONASS system time product not refused by name");
    sidereon_rinex_clock_result_free(result);
    sidereon_rinex_clock_free(glonass);
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
    require_true(raw_count == W3NC_RAW_COUNT, "raw NAV count changed");
    SidereonBroadcastRecord record;
    require_true(sidereon_rinex_nav_records_item(raw, 0, &record) == SIDEREON_STATUS_OK,
                 last_error());
    assert_nav_record(&record, &W3NC_RAW0, "raw first record changed");
    SidereonBroadcastRecord out_of_range_record;
    require_true(sidereon_rinex_nav_records_item(raw, raw_count, &out_of_range_record) ==
                     SIDEREON_STATUS_INVALID_ARGUMENT,
                 "raw NAV out-of-range item status mismatch");

    size_t written = 0;
    size_t required = 0;
    require_true(sidereon_encode_rinex_nav(&record, 1, NULL, 0, &written, &required) ==
                     SIDEREON_STATUS_OK,
                 last_error());
    /* The writer's size is checked by writing into that many bytes and one
     * fewer, and the text by reading every field of the record back. */
    require_true(written == 0 && required > 0, "NAV encoding size query");
    const size_t encoded_len = required;
    uint8_t *encoded = (uint8_t *)malloc(required);
    require_true(encoded != NULL, "cannot allocate encoded NAV");
    require_true(sidereon_encode_rinex_nav(
                     &record, 1, encoded, required - 1, &written, &required) ==
                     SIDEREON_STATUS_INVALID_ARGUMENT &&
                     written == 0 && required == encoded_len,
                 "NAV short-buffer status mismatch");
    require_true(sidereon_encode_rinex_nav(&record, 1, encoded, required, &written, &required) ==
                     SIDEREON_STATUS_OK &&
                     written == encoded_len && encoded[written - 1] == '\n',
                 last_error());
    /* The length sidereon_core::rinex::nav::encode_nav writes for this
     * record, from tests/pingen. */
    require_true(encoded_len == PIN_NAV_ENCODED_FIRST_RECORD_LEN, "NAV encoded length");
    static const char encoded_header[] = W3NC_ENCODED_HEADER_LINE;
    require_true(encoded_len >= sizeof(encoded_header) - 1 &&
                     memcmp(encoded, encoded_header, sizeof(encoded_header) - 1) == 0,
                 "NAV encoded header changed");
    SidereonRinexNavRecords *reparsed = NULL;
    require_true(sidereon_parse_rinex_nav_records(encoded, written, &reparsed) ==
                     SIDEREON_STATUS_OK,
                 last_error());
    size_t reparsed_count = 0;
    require_true(sidereon_rinex_nav_records_count(reparsed, &reparsed_count) ==
                     SIDEREON_STATUS_OK && reparsed_count == W3NC_REENCODED_COUNT,
                 "encoded NAV reparse count changed");
    SidereonBroadcastRecord reparsed_record;
    require_true(sidereon_rinex_nav_records_item(reparsed, 0, &reparsed_record) ==
                     SIDEREON_STATUS_OK,
                 last_error());
    assert_nav_record(&reparsed_record, &W3NC_RAW0, "re-encoded record changed");
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
                     lenient_records == W3NC_LENIENT_RECORD_COUNT &&
                     skipped == W3NC_LENIENT_SKIPPED_COUNT,
                 "lenient NAV counts changed");
    SidereonBroadcastRecord lenient_record;
    require_true(sidereon_nav_parse_record(lenient, 0, &lenient_record) == SIDEREON_STATUS_OK,
                 last_error());
    assert_nav_record(&lenient_record, &W3NC_RAW0, "lenient first record changed");
    SidereonRinexNavParse *bad_lenient = NULL;
    uint8_t *bad_nav = (uint8_t *)malloc(nav_len);
    require_true(bad_nav != NULL, "cannot allocate malformed NAV");
    memcpy(bad_nav, nav, nav_len);
    /* The first "C05 2020" is the epoch line of the first record,
     * fixtures/nav/ESBC00DNK_R_20201770000_01D_MN.rnx line 208. */
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
                     bad_records == W3NC_BAD_LENIENT_RECORD_COUNT &&
                     bad_skipped == W3NC_BAD_LENIENT_SKIPPED_COUNT,
                 "lenient malformed NAV counts changed");
    SidereonSkippedNavBlock diagnostic;
    require_true(sidereon_nav_parse_skipped(bad_lenient, 0, &diagnostic) == SIDEREON_STATUS_OK &&
                     token_is(diagnostic.satellite, W3NC_BAD_SKIPPED0_SATELLITE) &&
                     strcmp(diagnostic.message, W3NC_BAD_SKIPPED0_MESSAGE) == 0,
                 "lenient skipped diagnostic changed");
    const char expected_diagnostic[] = W3NC_BAD_SKIPPED0_MESSAGE;
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
                     W3NC_BAD_STRICT_STATUS &&
                     (bad_raw == NULL) == (W3NC_BAD_STRICT_STATUS != SIDEREON_STATUS_OK),
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
    require_true(iono.gps.present == W3NC_IONO_GPS_PRESENT &&
                     iono.galileo.present == W3NC_IONO_GALILEO_PRESENT &&
                     iono.beidou.present == W3NC_IONO_BEIDOU_PRESENT &&
                     f64_bits(iono.gps.alpha[0]) == W3NC_IONO_GPS_ALPHA0 &&
                     f64_bits(iono.gps.alpha[1]) == W3NC_IONO_GPS_ALPHA1 &&
                     f64_bits(iono.gps.alpha[2]) == W3NC_IONO_GPS_ALPHA2 &&
                     f64_bits(iono.gps.alpha[3]) == W3NC_IONO_GPS_ALPHA3 &&
                     f64_bits(iono.gps.beta[0]) == W3NC_IONO_GPS_BETA0 &&
                     f64_bits(iono.gps.beta[1]) == W3NC_IONO_GPS_BETA1 &&
                     f64_bits(iono.gps.beta[2]) == W3NC_IONO_GPS_BETA2 &&
                     f64_bits(iono.gps.beta[3]) == W3NC_IONO_GPS_BETA3 &&
                     f64_bits(iono.galileo.ai0) == W3NC_IONO_GALILEO_AI0 &&
                     f64_bits(iono.galileo.ai1) == W3NC_IONO_GALILEO_AI1 &&
                     f64_bits(iono.galileo.ai2) == W3NC_IONO_GALILEO_AI2,
                 "NAV ionosphere fields changed");
    double leap = 0.0;
    bool leap_present = false;
    require_true(sidereon_parse_rinex_leap_seconds(nav, nav_len, &leap, &leap_present) ==
                     SIDEREON_STATUS_OK && leap_present == W3NC_LEAP_PRESENT &&
                     f64_bits(leap) == W3NC_LEAP_SECONDS,
                 "NAV leap-second field changed");
    const uint8_t empty_header[] =
        "     3.05           NAVIGATION DATA     MIXED               RINEX VERSION / TYPE\n"
        "                                                            END OF HEADER\n";
    leap = 123.0;
    leap_present = true;
    require_true(sidereon_parse_rinex_leap_seconds(empty_header, sizeof(empty_header) - 1, &leap,
                                                   &leap_present) == SIDEREON_STATUS_OK &&
                     leap_present == W3NC_EMPTY_HEADER_LEAP_PRESENT &&
                     (leap_present || f64_bits(leap) == UINT64_C(0)),
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
                     SIDEREON_STATUS_OK && standalone_glonass_count == W3NC_GLONASS_COUNT,
                 "standalone GLONASS count changed");
    SidereonGlonassRecord standalone_glonass;
    require_true(sidereon_rinex_glonass_records_item(glonass_records, 0, &standalone_glonass) ==
                     SIDEREON_STATUS_OK,
                 last_error());
    assert_glonass_record(&standalone_glonass, &W3NC_GLONASS0, "GLONASS record changed");

    SidereonRinexGlonassRecords *extended_glonass_records = NULL;
    const char *extended_glonass_text = glonass_extended_fixture();
    require_true(sidereon_parse_rinex_glonass_records(
                     (const uint8_t *)extended_glonass_text, strlen(extended_glonass_text),
                     &extended_glonass_records) == SIDEREON_STATUS_OK,
                 last_error());
    /* R28 is inside the shared 01..99 satellite-token range, so the extended
       slot is read with its own values instead of being skipped. */
    size_t extended_record_count = SIZE_MAX;
    require_true(sidereon_rinex_glonass_records_count(extended_glonass_records,
                                                     &extended_record_count) ==
                     SIDEREON_STATUS_OK && extended_record_count == W3NC_EXTENDED_GLONASS_COUNT,
                 "extended GLONASS record was not read");
    size_t skipped_glonass_count = SIZE_MAX;
    require_true(sidereon_rinex_glonass_records_skipped_count(
                     extended_glonass_records, &skipped_glonass_count) ==
                     SIDEREON_STATUS_OK && skipped_glonass_count == W3NC_EXTENDED_GLONASS_SKIPPED_COUNT,
                 "extended GLONASS skip count changed");
    SidereonGlonassRecord extended_glonass;
    require_true(sidereon_rinex_glonass_records_item(extended_glonass_records, 0,
                                                     &extended_glonass) == SIDEREON_STATUS_OK,
                 last_error());
    assert_glonass_record(&extended_glonass, &W3NC_EXTENDED_GLONASS0,
                          "extended GLONASS record changed");

    size_t combined_len = nav_len + strlen(glonass);
    uint8_t *combined_nav = (uint8_t *)malloc(combined_len);
    require_true(combined_nav != NULL, "cannot allocate combined NAV");
    memcpy(combined_nav, nav, nav_len);
    memcpy(combined_nav + nav_len, glonass, strlen(glonass));
    SidereonBroadcastEphemeris *broadcast = NULL;
    require_true(sidereon_broadcast_ephemeris_parse_nav(combined_nav, combined_len, &broadcast) ==
                     SIDEREON_STATUS_OK,
                 last_error());
    /* The store keeps every record of the single-frequency messages, healthy
     * or not, and leaves out Galileo F/NAV, so its count is the raw records
     * less the F/NAV ones. */
    size_t single_frequency_count = 0;
    for (size_t i = 0; i < raw_count; ++i) {
        SidereonBroadcastRecord raw_record;
        require_true(sidereon_rinex_nav_records_item(raw, i, &raw_record) == SIDEREON_STATUS_OK,
                     last_error());
        if (raw_record.message != SIDEREON_NAV_MESSAGE_GALILEO_FNAV) {
            single_frequency_count += 1;
        }
    }
    size_t broadcast_count = 0;
    size_t broadcast_glonass_count = 0;
    size_t channel_count = 0;
    require_true(sidereon_broadcast_ephemeris_record_count(broadcast, &broadcast_count) ==
                     SIDEREON_STATUS_OK &&
                     sidereon_broadcast_ephemeris_glonass_record_count(
                         broadcast, &broadcast_glonass_count) == SIDEREON_STATUS_OK &&
                     sidereon_broadcast_ephemeris_glonass_frequency_channel_count(
                         broadcast, &channel_count) == SIDEREON_STATUS_OK &&
                     broadcast_count == single_frequency_count &&
                     broadcast_count == W3NC_STORE_RECORD_COUNT &&
                     broadcast_glonass_count == W3NC_STORE_GLONASS_COUNT &&
                     channel_count == W3NC_STORE_CHANNEL_COUNT,
                 "rich broadcast counts changed");
    /* The record count of sidereon-core's own store of the same text, from
     * tests/pingen. */
    require_true(broadcast_count == PIN_NAV_STORE_RECORD_COUNT, "broadcast store record count");
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
    assert_nav_record(&rich_records[0], &W3NC_STORE0, "store first record changed");
    SidereonGlonassRecord *rich_glonass =
        (SidereonGlonassRecord *)calloc(broadcast_glonass_count, sizeof(*rich_glonass));
    require_true(rich_glonass != NULL, "cannot allocate rich GLONASS records");
    require_true(sidereon_broadcast_ephemeris_glonass_records(
                     broadcast, rich_glonass, broadcast_glonass_count, &written, &required) ==
                     SIDEREON_STATUS_OK && written == W3NC_STORE_GLONASS_COUNT &&
                     required == W3NC_STORE_GLONASS_COUNT,
                 last_error());
    assert_glonass_record(&rich_glonass[0], &W3NC_STORE_GLONASS0,
                          "store GLONASS record changed");
    SidereonFrequencyChannel channel;
    require_true(sidereon_broadcast_ephemeris_glonass_frequency_channels(
                     broadcast, &channel, 1, &written, &required) == SIDEREON_STATUS_OK &&
                     written == W3NC_STORE_CHANNEL_COUNT && required == W3NC_STORE_CHANNEL_COUNT &&
                     channel.slot == W3NC_STORE_CHANNEL0_SLOT &&
                     channel.channel == W3NC_STORE_CHANNEL0_CHANNEL,
                 last_error());
    require_true(sidereon_broadcast_ephemeris_iono_corrections(broadcast, &iono) ==
                     SIDEREON_STATUS_OK && iono.gps.present == W3NC_STORE_IONO_GPS_PRESENT &&
                     iono.galileo.present == W3NC_STORE_IONO_GALILEO_PRESENT,
                 last_error());
    require_true(sidereon_broadcast_ephemeris_leap_seconds(
                     broadcast, &leap, &leap_present) == SIDEREON_STATUS_OK &&
                     leap_present == W3NC_STORE_LEAP_PRESENT &&
                     f64_bits(leap) == W3NC_STORE_LEAP_SECONDS,
                 last_error());

    /* An SBAS MT2 capture in both wire forms: the 226-bit body, and the framed
     * form with its CRC-24Q and six zero pad bits. A log record must declare
     * the message type its payload carries. The lines and every expected value
     * come from tests/valgen (bin w3_rinex_nav_clock). */
    const char *ems = W3NC_SBAS_EMS_LINE;
    const char *rtklib = W3NC_SBAS_RTKLIB_LINE;
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
                         SIDEREON_STATUS_OK &&
                     ems_count == W3NC_SBAS_EMS_COUNT && rtklib_count == W3NC_SBAS_RTKLIB_COUNT,
                 "SBAS log counts changed");
    SidereonSbasLogBlock ems_block;
    SidereonSbasLogBlock rtklib_block;
    require_true(sidereon_sbas_log_blocks_item(ems_blocks, 0, &ems_block) == SIDEREON_STATUS_OK &&
                     token_is(ems_block.sat_id, W3NC_SBAS_EMS0_SAT) &&
                     ems_block.epoch.system == W3NC_SBAS_EMS0_EPOCH_SYSTEM &&
                     ems_block.epoch.week == W3NC_SBAS_EMS0_EPOCH_WEEK &&
                     f64_bits(ems_block.epoch.tow_s) == W3NC_SBAS_EMS0_EPOCH_TOW_S &&
                     ems_block.form == W3NC_SBAS_EMS0_FORM &&
                     ems_block.byte_count == W3NC_SBAS_EMS0_BYTE_COUNT &&
                     ems_block.has_declared_message_type ==
                         W3NC_SBAS_EMS0_HAS_DECLARED_MESSAGE_TYPE &&
                     ems_block.declared_message_type == W3NC_SBAS_EMS0_DECLARED_MESSAGE_TYPE &&
                     ems_block.has_message_type == W3NC_SBAS_EMS0_HAS_MESSAGE_TYPE &&
                     ems_block.message_type == W3NC_SBAS_EMS0_MESSAGE_TYPE,
                 "EMS metadata changed");
    require_true(sidereon_sbas_log_blocks_item(rtklib_blocks, 0, &rtklib_block) ==
                         SIDEREON_STATUS_OK &&
                     token_is(rtklib_block.sat_id, W3NC_SBAS_RTKLIB0_SAT) &&
                     rtklib_block.epoch.system == W3NC_SBAS_RTKLIB0_EPOCH_SYSTEM &&
                     rtklib_block.epoch.week == W3NC_SBAS_RTKLIB0_EPOCH_WEEK &&
                     f64_bits(rtklib_block.epoch.tow_s) == W3NC_SBAS_RTKLIB0_EPOCH_TOW_S &&
                     rtklib_block.form == W3NC_SBAS_RTKLIB0_FORM &&
                     rtklib_block.byte_count == W3NC_SBAS_RTKLIB0_BYTE_COUNT &&
                     rtklib_block.has_declared_message_type ==
                         W3NC_SBAS_RTKLIB0_HAS_DECLARED_MESSAGE_TYPE &&
                     rtklib_block.declared_message_type ==
                         W3NC_SBAS_RTKLIB0_DECLARED_MESSAGE_TYPE &&
                     rtklib_block.has_message_type == W3NC_SBAS_RTKLIB0_HAS_MESSAGE_TYPE &&
                     rtklib_block.message_type == W3NC_SBAS_RTKLIB0_MESSAGE_TYPE,
                 "RTKLIB metadata changed");
    uint8_t sbas_payload[W3NC_SBAS_EMS0_BYTE_COUNT];
    require_true(sidereon_sbas_log_blocks_bytes(
                     ems_blocks, 0, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
                     written == 0 && required == W3NC_SBAS_EMS0_BYTE_COUNT,
                 "SBAS payload size query changed");
    require_true(sidereon_sbas_log_blocks_bytes(
                     ems_blocks, 0, sbas_payload, sizeof(sbas_payload) - 1, &written, &required) ==
                     SIDEREON_STATUS_INVALID_ARGUMENT && written == 0 &&
                     required == W3NC_SBAS_EMS0_BYTE_COUNT,
                 "SBAS payload short-buffer status changed");
    memset(sbas_payload, 0xff, sizeof(sbas_payload));
    require_true(sidereon_sbas_log_blocks_bytes(
                     ems_blocks, 0, sbas_payload, sizeof(sbas_payload), &written, &required) ==
                     SIDEREON_STATUS_OK && written == W3NC_SBAS_EMS0_BYTE_COUNT &&
                     required == W3NC_SBAS_EMS0_BYTE_COUNT,
                 last_error());
    require_true(memcmp(sbas_payload, W3NC_SBAS_EMS0_BYTES, sizeof(sbas_payload)) == 0,
                 "EMS payload bytes changed");
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
                         SIDEREON_STATUS_OK && satellite_count == W3NC_CLK_SERIES_COUNT &&
                     sample_count == W3NC_CLK_SAMPLE_COUNT,
                 "clock lossy counts changed");
    require_true(sidereon_rinex_clock_satellites(
                     clock, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
                     written == 0 && required == W3NC_CLK_SERIES_COUNT,
                 "clock satellite size query changed");
    SidereonSatelliteToken satellites[W3NC_CLK_SERIES_COUNT];
    require_true(sidereon_rinex_clock_satellites(
                     clock, satellites, 1, &written, &required) == SIDEREON_STATUS_INVALID_ARGUMENT &&
                     written == 0 && required == W3NC_CLK_SERIES_COUNT,
                 "clock satellite short-buffer status changed");
    require_true(sidereon_rinex_clock_satellites(
                     clock, satellites, W3NC_CLK_SERIES_COUNT, &written, &required) ==
                         SIDEREON_STATUS_OK &&
                     written == W3NC_CLK_SERIES_COUNT && required == W3NC_CLK_SERIES_COUNT &&
                     token_is(satellites[0], W3NC_CLK_SATELLITE0) &&
                     token_is(satellites[1], W3NC_CLK_SATELLITE1),
                 "clock satellite ordering changed");
    SidereonClockSeries *series_by_index = NULL;
    require_true(sidereon_rinex_clock_series(clock, 0, &series_by_index) == SIDEREON_STATUS_OK &&
                     series_by_index != NULL,
                 last_error());
    SidereonSatelliteToken series_satellite;
    require_true(sidereon_rinex_clock_series_satellite(series_by_index, &series_satellite) ==
                     SIDEREON_STATUS_OK && token_is(series_satellite, W3NC_CLK_SATELLITE0),
                 last_error());
    /* G05 is the satellite of the AS records at
     * fixtures/clk/synthetic_rinex_clock.clk lines 5-7. */
    SidereonClockSeries *series = NULL;
    require_true(sidereon_rinex_clock_series_for(clock, "G05", &series) == SIDEREON_STATUS_OK &&
                     series != NULL,
                 last_error());
    size_t g05_count = 0;
    require_true(sidereon_rinex_clock_series_sample_count(series, &g05_count) ==
                     SIDEREON_STATUS_OK && g05_count == W3NC_CLK_G05_COUNT,
                 "clock G05 sample count changed");
    require_true(sidereon_rinex_clock_series_samples(
                     series, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
                     written == 0 && required == W3NC_CLK_G05_COUNT,
                 "clock sample size query changed");
    SidereonClockPoint samples[W3NC_CLK_G05_COUNT];
    require_true(sidereon_rinex_clock_series_samples(
                     series, samples, W3NC_CLK_G05_COUNT - 1, &written, &required) ==
                         SIDEREON_STATUS_INVALID_ARGUMENT &&
                     written == 0 && required == W3NC_CLK_G05_COUNT,
                 "clock sample short-buffer status changed");
    require_true(sidereon_rinex_clock_series_samples(
                     series, samples, W3NC_CLK_G05_COUNT, &written, &required) ==
                         SIDEREON_STATUS_OK &&
                     written == W3NC_CLK_G05_COUNT,
                 last_error());
    /* Each G05 record declares its bias sigma, so each sample carries it. */
#define CHECK_G05_SAMPLE(I)                                                                  \
    require_true(samples[I].epoch.scale == W3NC_CLK_G05_##I##_EPOCH_SCALE &&                \
                     samples[I].epoch.representation == W3NC_CLK_G05_##I##_EPOCH_REPRESENTATION && \
                     f64_bits(samples[I].epoch.jd_whole) == W3NC_CLK_G05_##I##_EPOCH_JD_WHOLE && \
                     f64_bits(samples[I].epoch.jd_fraction) ==                               \
                         W3NC_CLK_G05_##I##_EPOCH_JD_FRACTION &&                             \
                     f64_bits(samples[I].bias_s) == W3NC_CLK_G05_##I##_BIAS &&               \
                     samples[I].additional_value_count == W3NC_CLK_G05_##I##_ADDITIONAL_COUNT && \
                     f64_bits(samples[I].additional_values[0]) ==                            \
                         W3NC_CLK_G05_##I##_ADDITIONAL0 &&                                   \
                     isnan(samples[I].additional_values[1]),                                 \
                 "clock G05 sample changed")
    CHECK_G05_SAMPLE(0);
    CHECK_G05_SAMPLE(1);
    CHECK_G05_SAMPLE(2);
#undef CHECK_G05_SAMPLE
    SidereonClockSeries *missing_series = (SidereonClockSeries *)(uintptr_t)1;
    require_true(sidereon_rinex_clock_series_for(clock, "G99", &missing_series) ==
                     SIDEREON_STATUS_OK &&
                     (missing_series != NULL) == W3NC_CLK_HAS_G99,
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
                     malformed_sample_count == W3NC_MALFORMED_AS_LOSSY_SAMPLE_COUNT,
                 "clock malformed AS row was not skipped");
    SidereonRinexClock *malformed_strict = (SidereonRinexClock *)(uintptr_t)1;
    require_true(sidereon_rinex_clock_parse(
                     (const uint8_t *)malformed_as_clock, sizeof(malformed_as_clock) - 1,
                     &malformed_strict) == W3NC_MALFORMED_AS_STRICT_STATUS &&
                     (malformed_strict == NULL) ==
                         (W3NC_MALFORMED_AS_STRICT_STATUS != SIDEREON_STATUS_OK),
                 "clock strict malformed AS status mismatch");
    sidereon_rinex_clock_free(malformed_lossy);
    sidereon_rinex_clock_free(malformed_strict);

    check_clock_table_a18();
    check_clock_surplus_and_edits();
    check_clock_built_products();

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
