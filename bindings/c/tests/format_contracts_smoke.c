/*
 * C smoke for the ANTEX retention and typed refusals, BLQ block reading and
 * writing with retained comments, the SBAS PRN window and unassigned-mask
 * accessor, and the RTCM MSM encode refusal.
 *
 * argv: <antex_fixture>
 */
#include "sidereon.h"
/* sidereon-core's results for the inputs below, from tests/valgen
 * (w6_format_contracts). */
#include "w6_format_contracts_pins.h"

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int failures = 0;

static void check(bool ok, const char *what) {
    if (!ok) {
        char message[512];
        size_t message_len = sidereon_last_error_message(message, sizeof(message));
        if (message_len == 0) {
            message[0] = '\0';
        }
        fprintf(stderr, "FAIL: %s (last ABI error, may predate this check: %s)\n", what,
                message);
        failures++;
    }
}

static uint8_t *read_file(const char *path, size_t *length) {
    FILE *file = fopen(path, "rb");
    if (!file) {
        return NULL;
    }
    if (fseek(file, 0, SEEK_END) != 0) {
        fclose(file);
        return NULL;
    }
    long end = ftell(file);
    if (end < 0 || fseek(file, 0, SEEK_SET) != 0) {
        fclose(file);
        return NULL;
    }
    uint8_t *bytes = (uint8_t *)malloc((size_t)end + 1);
    if (!bytes) {
        fclose(file);
        return NULL;
    }
    size_t read = fread(bytes, 1, (size_t)end, file);
    fclose(file);
    if (read != (size_t)end) {
        free(bytes);
        return NULL;
    }
    *length = read;
    return bytes;
}

static bool same_f64(double value, uint64_t expected) {
    uint64_t bits = 0;
    memcpy(&bits, &value, sizeof(bits));
    return bits == expected;
}

static bool text_is(SidereonStatus status, const char *buffer, size_t written,
                    const char *expected) {
    return status == SIDEREON_STATUS_OK && written == strlen(expected) &&
           memcmp(buffer, expected, written) == 0;
}

/* ------------------------------------------------------------------------ */
/* ANTEX                                                                     */

/* One antenna block with two sections labelled G01 whose offsets differ. */
static const char ambiguous_antex[] =
    "                                                            START OF ANTENNA\n"
    "TESTANT             TESTSER                                 TYPE / SERIAL NO\n"
    "     0.0                                                    DAZI\n"
    "     0.0  10.0   5.0                                        ZEN1 / ZEN2 / DZEN\n"
    "G01                                                         START OF FREQUENCY\n"
    "      0.00      0.00      0.00                              NORTH / EAST / UP\n"
    "   NOAZI    1.00    2.00    3.00\n"
    "                                                            END OF FREQUENCY\n"
    "G01                                                         START OF FREQUENCY\n"
    "      0.00      0.00      1.00                              NORTH / EAST / UP\n"
    "   NOAZI    1.00    2.00    3.00\n"
    "                                                            END OF FREQUENCY\n"
    "                                                            END OF ANTENNA\n";

/* The same block with a DAZI value that is not a number. */
static const char bad_dazi_antex[] =
    "                                                            START OF ANTENNA\n"
    "TESTANT             TESTSER                                 TYPE / SERIAL NO\n"
    "   x.y                                                      DAZI\n"
    "     0.0  10.0   5.0                                        ZEN1 / ZEN2 / DZEN\n"
    "G01                                                         START OF FREQUENCY\n"
    "      0.00      0.00      0.00                              NORTH / EAST / UP\n"
    "   NOAZI    1.00    2.00    3.00\n"
    "                                                            END OF FREQUENCY\n"
    "                                                            END OF ANTENNA\n";

static void check_antex(const char *fixture_path) {
    size_t length = 0;
    uint8_t *bytes = read_file(fixture_path, &length);
    check(bytes != NULL, "ANTEX fixture read");
    if (!bytes) {
        return;
    }
    SidereonAntex *antex = NULL;
    SidereonAntexResult *result = NULL;
    check(sidereon_antex_parse_result(bytes, length, &antex, &result) == SIDEREON_STATUS_OK &&
              antex != NULL && result != NULL,
          "ANTEX parse result");
    SidereonAntexOutcome outcome;
    check(sidereon_antex_result_get_outcome(result, &outcome) == SIDEREON_STATUS_OK &&
              outcome.is_ok && outcome.error.kind == SIDEREON_ANTEX_ERROR_KIND_NONE,
          "ANTEX parse outcome");
    sidereon_antex_result_free(result);
    if (!antex) {
        free(bytes);
        return;
    }

    SidereonAntexHeader header;
    check(sidereon_antex_header(antex, &header) == SIDEREON_STATUS_OK &&
              header.has_version == W6_FC_HAS_VERSION &&
              same_f64(header.version, W6_FC_VERSION_BITS) &&
              header.has_system == W6_FC_HAS_SYSTEM && header.system == W6_FC_SYSTEM &&
              header.has_pcv_type == W6_FC_HAS_PCV_TYPE && header.pcv_type == W6_FC_PCV_TYPE &&
              header.has_reference_antenna == W6_FC_HAS_REFERENCE_ANTENNA &&
              header.comment_count == W6_FC_HEADER_COMMENTS &&
              header.end_of_header == W6_FC_END_OF_HEADER,
          "ANTEX header retained");
    size_t blocks = 0;
    size_t skipped = 99;
    check(sidereon_antex_block_count(antex, &blocks) == SIDEREON_STATUS_OK &&
              blocks == W6_FC_BLOCK_COUNT &&
              sidereon_antex_skipped_records(antex, &skipped) == SIDEREON_STATUS_OK,
          "ANTEX blocks");

    /* Block 9 is the receiver antenna block (the tenth START OF ANTENNA
     * record of igs20_wettzell_trim.atx). */
    SidereonAntenna *receiver = NULL;
    check(sidereon_antex_block(antex, 9, &receiver) == SIDEREON_STATUS_OK && receiver != NULL,
          "ANTEX receiver block");
    if (receiver) {
        SidereonAntennaInfo info;
        check(sidereon_antenna_info(receiver, &info) == SIDEREON_STATUS_OK &&
                  info.kind == W6_FC_RECEIVER_KIND && info.has_dazi_deg == W6_FC_RECEIVER_HAS_DAZI &&
                  same_f64(info.dazi_deg, W6_FC_RECEIVER_DAZI_BITS) &&
                  info.has_zenith_grid == W6_FC_RECEIVER_HAS_GRID &&
                  same_f64(info.zenith_end_deg, W6_FC_RECEIVER_ZENITH_END_BITS) &&
                  info.frequency_count == W6_FC_RECEIVER_FREQUENCIES &&
                  info.calibration_count == W6_FC_RECEIVER_CALIBRATIONS &&
                  info.comment_count == W6_FC_RECEIVER_COMMENTS &&
                  info.has_valid_from == W6_FC_RECEIVER_HAS_VALID_FROM,
              "ANTEX receiver block fields");
        char label[16];
        size_t written = 0;
        size_t required = 0;
        check(text_is(sidereon_antenna_frequency_label(receiver, 5, (uint8_t *)label,
                                                       sizeof(label), &written, &required),
                      label, written, W6_FC_RECEIVER_FREQUENCY5),
              "ANTEX frequency order kept");
        char method[32];
        check(text_is(sidereon_antenna_calibration_text(
                          receiver, 0, SIDEREON_ANTEX_CALIBRATION_TEXT_METHOD, (uint8_t *)method,
                          sizeof(method), &written, &required),
                      method, written, W6_FC_RECEIVER_METHOD),
              "ANTEX method record kept");
        sidereon_antenna_free(receiver);
    }

    /* A satellite antenna found by PRN and epoch, with its exact bound. */
    SidereonAntexDateTime epoch = {2020, 1, 1, 0, 0, 0, 0, 0};
    SidereonAntenna *satellite = NULL;
    check(sidereon_antex_satellite_antenna(antex, "G05", &epoch, &satellite) ==
                  SIDEREON_STATUS_OK &&
              (satellite != NULL) == W6_FC_G05_FOUND,
          "ANTEX satellite antenna at epoch");
    if (satellite) {
        SidereonAntennaInfo info;
        check(sidereon_antenna_info(satellite, &info) == SIDEREON_STATUS_OK &&
                  info.kind == W6_FC_G05_KIND && info.has_valid_from == W6_FC_G05_HAS_VALID_FROM &&
                  info.valid_from.year == W6_FC_G05_VALID_FROM_YEAR &&
                  info.valid_from.month == W6_FC_G05_VALID_FROM_MONTH &&
                  info.valid_from.day == W6_FC_G05_VALID_FROM_DAY &&
                  info.valid_from.fraction_digits == W6_FC_G05_VALID_FROM_FRACTION_DIGITS &&
                  info.valid_from.fraction_scale == W6_FC_G05_VALID_FROM_FRACTION_SCALE,
              "ANTEX validity bound exact");
        sidereon_antenna_free(satellite);
    }

    /* GPS time has no second 60: sidereon-core refuses the date-time, which the
     * binding reports as INVALID_ARGUMENT with the typed kind
     * (antex_date_time_from_c in bindings/c/src/antex.rs). */
    check(W6_FC_LEAP_REFUSED, "sidereon-core accepts second 60");
    SidereonAntexDateTime leap = {2016, 12, 31, 23, 59, 60, 0, 0};
    satellite = NULL;
    check(sidereon_antex_satellite_antenna(antex, "G05", &leap, &satellite) ==
                  SIDEREON_STATUS_INVALID_ARGUMENT &&
              satellite == NULL,
          "ANTEX leap-second epoch refused");
    SidereonAntexError error;
    check(sidereon_last_antex_error(&error) == SIDEREON_STATUS_OK &&
              error.kind == W6_FC_LEAP_KIND,
          "ANTEX leap-second epoch typed");

    /* The product writes back and reads back with the same blocks. */
    check(sidereon_antex_encode_result(antex, &result) == SIDEREON_STATUS_OK && result != NULL,
          "ANTEX encode result");
    if (result) {
        size_t written = 0;
        size_t required = 0;
        check(sidereon_antex_result_get_text(result, NULL, 0, &written, &required) ==
                      SIDEREON_STATUS_OK &&
                  required > 0,
              "ANTEX encode size");
        uint8_t *text = (uint8_t *)malloc(required);
        if (text) {
            check(sidereon_antex_result_get_text(result, text, required, &written, &required) ==
                      SIDEREON_STATUS_OK,
                  "ANTEX encode text");
            SidereonAntex *reread = NULL;
            size_t reread_blocks = 0;
            check(sidereon_antex_parse(text, written, &reread) == SIDEREON_STATUS_OK &&
                      sidereon_antex_block_count(reread, &reread_blocks) == SIDEREON_STATUS_OK &&
                      reread_blocks == blocks,
                  "ANTEX encoded text reads back");
            sidereon_antex_free(reread);
            free(text);
        }
        sidereon_antex_result_free(result);
    }
    sidereon_antex_free(antex);
    free(bytes);

    /* Two differing sections under one label: no single calibration answers. */
    antex = NULL;
    check(sidereon_antex_parse((const uint8_t *)ambiguous_antex, sizeof(ambiguous_antex) - 1,
                               &antex) == SIDEREON_STATUS_OK &&
              antex != NULL,
          "ANTEX repeated label parse");
    if (antex) {
        SidereonAntenna *antenna = NULL;
        check(sidereon_antex_block(antex, 0, &antenna) == SIDEREON_STATUS_OK && antenna != NULL,
              "ANTEX repeated label block");
        if (antenna) {
            SidereonAntennaInfo info;
            check(sidereon_antenna_info(antenna, &info) == SIDEREON_STATUS_OK &&
                      info.frequency_count == W6_FC_AMBIGUOUS_FREQUENCIES &&
                      info.has_frequency_count_record == W6_FC_AMBIGUOUS_HAS_COUNT_RECORD,
                  "ANTEX both sections kept");
            double pco[3] = {1.0, 1.0, 1.0};
            check(sidereon_antenna_pco(antenna, "G01", pco) == SIDEREON_STATUS_INVALID_ARGUMENT,
                  "ANTEX ambiguous frequency refused");
            check(sidereon_last_antex_error(&error) == SIDEREON_STATUS_OK &&
                      error.kind == W6_FC_AMBIGUOUS_KIND && error.has_antenna_id &&
                      error.has_frequency && error.has_sections &&
                      error.sections == W6_FC_AMBIGUOUS_SECTIONS,
                  "ANTEX ambiguous frequency typed");
            char frequency[16];
            size_t written = 0;
            size_t required = 0;
            check(text_is(sidereon_last_antex_error_text(SIDEREON_ANTEX_ERROR_TEXT_FREQUENCY,
                                                         (uint8_t *)frequency,
                                                         sizeof(frequency), &written, &required),
                          frequency, written, W6_FC_AMBIGUOUS_FREQUENCY),
                  "ANTEX ambiguous frequency label");
            sidereon_antenna_free(antenna);
        }
        sidereon_antex_free(antex);
    }

    /* A field that does not read is refused by record, field and value. */
    antex = NULL;
    result = NULL;
    check(sidereon_antex_parse_result((const uint8_t *)bad_dazi_antex,
                                      sizeof(bad_dazi_antex) - 1, &antex,
                                      &result) == SIDEREON_STATUS_OK &&
              antex == NULL && result != NULL,
          "ANTEX bad DAZI parse result");
    if (result) {
        check(sidereon_antex_result_get_outcome(result, &outcome) == SIDEREON_STATUS_OK &&
                  !outcome.is_ok &&
                  outcome.error.kind == W6_FC_BAD_DAZI_KIND &&
                  outcome.error.has_antenna_id == W6_FC_BAD_DAZI_HAS_ANTENNA_ID &&
                  outcome.error.has_record &&
                  outcome.error.has_field && outcome.error.has_value,
              "ANTEX bad DAZI typed");
        char part[64];
        size_t written = 0;
        size_t required = 0;
        check(text_is(sidereon_antex_result_error_text(result, SIDEREON_ANTEX_ERROR_TEXT_RECORD,
                                                       (uint8_t *)part, sizeof(part), &written,
                                                       &required),
                      part, written, W6_FC_BAD_DAZI_RECORD),
              "ANTEX bad DAZI record");
        check(text_is(sidereon_antex_result_error_text(result, SIDEREON_ANTEX_ERROR_TEXT_VALUE,
                                                       (uint8_t *)part, sizeof(part), &written,
                                                       &required),
                      part, written, W6_FC_BAD_DAZI_VALUE),
              "ANTEX bad DAZI value");
        check(text_is(sidereon_antex_result_error_text(
                          result, SIDEREON_ANTEX_ERROR_TEXT_ANTENNA_ID, (uint8_t *)part,
                          sizeof(part), &written, &required),
                      part, written, W6_FC_BAD_DAZI_ANTENNA_ID),
              "ANTEX bad DAZI antenna");
        sidereon_antex_result_free(result);
    }
}

/* ------------------------------------------------------------------------ */
/* BLQ                                                                       */

static const char onsa_blq[] =
    "$$ Ocean loading displacement\n"
    "$$ COLUMN ORDER:  M2  S2  N2  K2  K1  O1  P1  Q1  MF  MM SSA\n"
    "  ONSA\n"
    "$$ ONSA,                 RADI TANG  lon/lat:   11.9255   57.3953    45.000\n"
    "  .00344 .00121 .00078 .00031 .00189 .00116 .00064 .00004 .00090 .00048 .00041\n"
    "  .00143 .00035 .00035 .00008 .00053 .00051 .00018 .00009 .00013 .00006 .00003\n"
    "  .00086 .00023 .00023 .00006 .00029 .00025 .00010 .00008 .00005 .00003 .00001\n"
    "   -64.7  -52.0  -96.2  -55.2  -58.8 -151.4  -65.6 -138.1    8.4    5.2    2.1\n"
    "    85.5  114.5   56.5  113.6   99.4   19.1   94.1  -10.4 -167.4 -170.0 -177.7\n"
    "   109.5  147.0   92.7  148.8   50.5  -55.1   50.5 -113.9   44.8    1.9    0.4\n";

static void check_blq(void) {
    SidereonBlqBlocks *blocks = NULL;
    check(sidereon_blq_parse((const uint8_t *)onsa_blq, sizeof(onsa_blq) - 1, &blocks, NULL) ==
                  SIDEREON_STATUS_OK &&
              blocks != NULL,
          "BLQ parse");
    if (!blocks) {
        return;
    }
    size_t count = 0;
    check(sidereon_blq_blocks_count(blocks, &count) == SIDEREON_STATUS_OK &&
              count == W6_FC_BLQ_BLOCKS &&
              sidereon_blq_blocks_comment_count(blocks, 0, &count) == SIDEREON_STATUS_OK &&
              count == W6_FC_BLQ_COMMENTS,
          "BLQ block and comments");
    SidereonBlqComment comment;
    char line[128];
    size_t written = 0;
    size_t required = 0;
    check(sidereon_blq_blocks_comment(blocks, 0, 1, &comment, (uint8_t *)line, sizeof(line),
                                      &written, &required) == SIDEREON_STATUS_OK &&
              comment.placement == W6_FC_BLQ_COMMENT1_PLACEMENT &&
              comment.row == W6_FC_BLQ_COMMENT1_ROW &&
              written == strlen(W6_FC_BLQ_COMMENT1_LINE) &&
              memcmp(line, W6_FC_BLQ_COMMENT1_LINE, written) == 0,
          "BLQ header line kept");
    check(sidereon_blq_blocks_comment(blocks, 0, 2, &comment, (uint8_t *)line, sizeof(line),
                                      &written, &required) == SIDEREON_STATUS_OK &&
              comment.placement == W6_FC_BLQ_COMMENT2_PLACEMENT &&
              comment.row == W6_FC_BLQ_COMMENT2_ROW,
          "BLQ station comment placed");
    char station[16];
    check(text_is(sidereon_blq_blocks_station(blocks, 0, (uint8_t *)station, sizeof(station),
                                              &written, &required),
                  station, written, W6_FC_BLQ_STATION),
          "BLQ station");
    SidereonOceanLoadingBlq coefficients;
    bool coefficients_same =
        sidereon_blq_blocks_coefficients(blocks, 0, &coefficients) == SIDEREON_STATUS_OK;
    for (size_t row = 0; row < 3; row++) {
        for (size_t column = 0; column < SIDEREON_PPP_OCEAN_CONSTITUENTS; column++) {
            size_t at = row * SIDEREON_PPP_OCEAN_CONSTITUENTS + column;
            coefficients_same = coefficients_same &&
                                same_f64(coefficients.amplitude_m[row][column],
                                         W6_FC_BLQ_AMPLITUDE_BITS[at]) &&
                                same_f64(coefficients.phase_deg[row][column],
                                         W6_FC_BLQ_PHASE_BITS[at]);
        }
    }
    check(coefficients_same, "BLQ coefficients");

    /* The written file reads back to the same block, comments included. */
    SidereonBlqResult *result = NULL;
    check(sidereon_blq_blocks_to_text_result(blocks, &result) == SIDEREON_STATUS_OK &&
              result != NULL,
          "BLQ write result");
    if (result) {
        SidereonBlqOutcome outcome;
        check(sidereon_blq_result_get_outcome(result, &outcome) == SIDEREON_STATUS_OK &&
                  outcome.is_ok,
              "BLQ write outcome");
        static char text[4096];
        check(sidereon_blq_result_get_text(result, (uint8_t *)text, sizeof(text), &written,
                                       &required) == SIDEREON_STATUS_OK,
              "BLQ write text");
        SidereonBlqBlocks *reread = NULL;
        size_t reread_comments = 0;
        SidereonOceanLoadingBlq reread_coefficients;
        check(sidereon_blq_parse((const uint8_t *)text, written, &reread, NULL) ==
                      SIDEREON_STATUS_OK &&
                  sidereon_blq_blocks_comment_count(reread, 0, &reread_comments) ==
                      SIDEREON_STATUS_OK &&
                  reread_comments == W6_FC_BLQ_COMMENTS &&
                  sidereon_blq_blocks_coefficients(reread, 0, &reread_coefficients) ==
                      SIDEREON_STATUS_OK &&
                  memcmp(&reread_coefficients, &coefficients, sizeof(coefficients)) == 0,
              "BLQ written text reads back");
        sidereon_blq_blocks_free(reread);
        sidereon_blq_result_free(result);
    }

    /* A station line the parser would read as a comment is refused. */
    SidereonBlqBlocks *built = NULL;
    check(sidereon_blq_blocks_new(&built) == SIDEREON_STATUS_OK &&
              sidereon_blq_blocks_push(built, "$ONSA", &coefficients) == SIDEREON_STATUS_OK,
          "BLQ build");
    result = NULL;
    check(sidereon_blq_blocks_to_text_result(built, &result) == SIDEREON_STATUS_OK &&
              result != NULL,
          "BLQ refused write result");
    if (result) {
        SidereonBlqOutcome outcome;
        check(sidereon_blq_result_get_outcome(result, &outcome) == SIDEREON_STATUS_OK &&
                  !outcome.is_ok && outcome.error.kind == SIDEREON_BLQ_ERROR_KIND_WRITE &&
                  outcome.error.write_kind == W6_FC_BLQ_REFUSED_KIND &&
                  outcome.error.has_block && outcome.error.block == W6_FC_BLQ_REFUSED_BLOCK,
              "BLQ comment-like station refused typed");
        check(sidereon_blq_result_get_text(result, NULL, 0, &written, &required) ==
                      SIDEREON_STATUS_INVALID_ARGUMENT &&
                  required == 0,
              "BLQ refused write has no text");
        sidereon_blq_result_free(result);
    }
    sidereon_blq_blocks_free(built);
    sidereon_blq_blocks_free(blocks);
}

/* ------------------------------------------------------------------------ */
/* SBAS and RTCM                                                             */

static void check_sbas_window(void) {
    uint16_t prn = 0;
    bool present = false;
    check(sidereon_satellite_id_to_sbas_prn("S20", &prn, &present) == SIDEREON_STATUS_OK &&
              present == W6_FC_S20_PRESENT && prn == W6_FC_S20_PRN,
          "SBAS S20 PRN");
    check(sidereon_satellite_id_to_sbas_prn("S99", &prn, &present) == SIDEREON_STATUS_OK &&
              present == W6_FC_S99_PRESENT && (present || prn == 0),
          "SBAS S99 PRN");
    SidereonSbasCorrectionStore *store = NULL;
    check(sidereon_sbas_store_new(&store) == SIDEREON_STATUS_OK && store != NULL,
          "SBAS store");
    SidereonSbasUnassignedMaskCorrections rows[1];
    size_t written = 1;
    size_t required = 1;
    present = true;
    check(sidereon_sbas_store_unassigned_mask_corrections(store, "S20", &present, rows, 1,
                                                          &written, &required) ==
                  SIDEREON_STATUS_OK &&
              present == W6_FC_S20_UNASSIGNED_PRESENT && written == 0 && required == 0,
          "SBAS GEO without a partition");
    sidereon_sbas_store_free(store);
}

static void check_rtcm_msm_refusal(void) {
    SidereonRtcmMsmInfo info;
    memset(&info, 0, sizeof(info));
    info.message_number = 1077;
    info.system = SIDEREON_GNSS_SYSTEM_GPS;
    info.kind = SIDEREON_RTCM_MSM_KIND_MSM7;
    info.satellite_count = 1;
    SidereonRtcmMsmSatellite satellite;
    memset(&satellite, 0, sizeof(satellite));
    satellite.id = 65; /* the satellite mask has bits 1..64 */
    satellite.has_rough_range_ms = true;
    satellite.rough_range_ms = 70;
    satellite.has_extended_info = true;
    satellite.has_rough_phase_range_rate = true;
    SidereonRtcmMessages *messages = NULL;
    check(sidereon_rtcm_build_msm(&info, &satellite, 1, NULL, 0, &messages) ==
                  SIDEREON_STATUS_OK &&
              messages != NULL,
          "RTCM MSM build");
    size_t written = 7;
    size_t required = 7;
    check(sidereon_rtcm_message_encode(messages, 0, NULL, 0, &written, &required) ==
                  W6_FC_MSM_ENCODE_STATUS &&
              written == 0 && required == 0,
          "RTCM MSM satellite outside the mask refused");
    check(sidereon_rtcm_message_to_frame(messages, 0, NULL, 0, &written, &required) ==
              W6_FC_MSM_FRAME_STATUS,
          "RTCM MSM frame refused");
    sidereon_rtcm_messages_free(messages);
}

int main(int argc, char **argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: %s <antex_fixture>\n", argv[0]);
        return 2;
    }
    check_antex(argv[1]);
    check_blq();
    check_sbas_window();
    check_rtcm_msm_refusal();
    if (failures != 0) {
        fprintf(stderr, "format_contracts_smoke: %d failure(s)\n", failures);
        return 1;
    }
    puts("format_contracts_smoke: OK");
    return 0;
}
