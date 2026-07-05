/* sidereon C bindings smoke test.
 *
 * Loads the committed crate-side SP3 product, runs one SPP solve through the C
 * ABI, prints the recovered position, and asserts it matches the crate's frozen
 * reference solution (transcribed into spp_fixture.h) to within the engine's own
 * documented agreement bound. Exits 0 only if the binding reproduces the
 * reference numbers; any failure prints a reason and exits non-zero.
 *
 * Build/run is driven by tests/run_smoke.sh, which passes the SP3 path as argv[1]
 * and links against the cdylib + generated header.
 */

#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sidereon.h"
#include "antex_fixture.h"
#include "constellation_fixture.h"
#include "dop_fixture.h"
#include "iono_fixture.h"
#include "rinex_fixture.h"
#include "velocity_fixture.h"
#include "ppp_fixture.h"
#include "prop_fixture.h"
#include "rtk_fixture.h"
#include "spk_fixture.h"
#include "spp_fixture.h"
#include "broadcast_fixture.h"

/* Reinterpret a stored IEEE-754 bit pattern as a double, exactly. */
static double bits_to_f64(uint64_t bits) {
    double value;
    memcpy(&value, &bits, sizeof(value));
    return value;
}

static uint64_t f64_to_bits(double value) {
    uint64_t bits;
    memcpy(&bits, &value, sizeof(bits));
    return bits;
}

static int token_equals(const SidereonSatelliteToken *token, const char *expected) {
    return strncmp((const char *)token->bytes, expected, sizeof(token->bytes)) == 0;
}

static int rtk_id_equals(const SidereonRtkId *token, const char *expected) {
    return strncmp((const char *)token->bytes, expected, sizeof(token->bytes)) == 0;
}

static int ppp_id_equals(const SidereonPppId *token, const char *expected) {
    return strncmp((const char *)token->bytes, expected, sizeof(token->bytes)) == 0;
}

static int ppp_id_list_contains(const SidereonPppId *tokens, size_t count,
                                const char *expected) {
    for (size_t i = 0; i < count; i++) {
        if (ppp_id_equals(&tokens[i], expected)) {
            return 1;
        }
    }
    return 0;
}

static int ppp_integer_status_equals(SidereonPppIntegerStatus got, const char *expected) {
    if (strcmp(expected, "Fixed") == 0) {
        return got == SIDEREON_PPP_INTEGER_STATUS_FIXED;
    }
    if (strcmp(expected, "NotFixed") == 0) {
        return got == SIDEREON_PPP_INTEGER_STATUS_NOT_FIXED;
    }
    return 0;
}

static int rejection_reason_equals(SidereonSppRejectionReason got, const char *expected) {
    if (strcmp(expected, "low_elevation") == 0) {
        return got == SIDEREON_SPP_REJECTION_REASON_LOW_ELEVATION;
    }
    if (strcmp(expected, "no_ephemeris") == 0) {
        return got == SIDEREON_SPP_REJECTION_REASON_NO_EPHEMERIS;
    }
    return 0;
}

/* Print the binding's last-error message, then return the given code. */
static int fail(const char *context, int code) {
    size_t needed = sidereon_last_error_message(NULL, 0);
    char *msg = (char *)malloc(needed + 1);
    if (msg != NULL) {
        sidereon_last_error_message(msg, needed + 1);
        fprintf(stderr, "FAIL: %s: %s\n", context, msg);
        free(msg);
    } else {
        fprintf(stderr, "FAIL: %s\n", context);
    }
    return code;
}

static int last_error_contains(const char *needle) {
    char msg[512];
    size_t written = sidereon_last_error_message(msg, sizeof(msg));
    return written > 0 && strstr(msg, needle) != NULL;
}

/* Read an entire file into a heap buffer; caller frees. */
static uint8_t *read_file(const char *path, size_t *out_len) {
    FILE *f = fopen(path, "rb");
    if (f == NULL) {
        return NULL;
    }
    if (fseek(f, 0, SEEK_END) != 0) {
        fclose(f);
        return NULL;
    }
    long size = ftell(f);
    if (size < 0) {
        fclose(f);
        return NULL;
    }
    rewind(f);
    uint8_t *buf = (uint8_t *)malloc((size_t)size);
    if (buf == NULL) {
        fclose(f);
        return NULL;
    }
    size_t read = fread(buf, 1, (size_t)size, f);
    fclose(f);
    if (read != (size_t)size) {
        free(buf);
        return NULL;
    }
    *out_len = read;
    return buf;
}

static int exercise_sp3_surface(const char *path) {
    int rc = 1;
    size_t sp3_len = 0;
    uint8_t *sp3_bytes = read_file(path, &sp3_len);
    SidereonSp3 *sp3 = NULL;
    SidereonSp3 *roundtrip = NULL;
    SidereonSp3 *merged = NULL;
    SidereonSp3MergeReport *report = NULL;
    SidereonSp3 *merged2 = NULL;
    SidereonSp3MergeReport *report2 = NULL;

    if (sp3_bytes == NULL) {
        fprintf(stderr, "FAIL: could not read SP3 surface file: %s\n", path);
        return 2;
    }
    if (sidereon_sp3_load(sp3_bytes, sp3_len, &sp3) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_sp3_load surface", 1);
        goto cleanup;
    }

    size_t written = 123;
    size_t required = 123;
    if (sidereon_sp3_satellites(NULL, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_NULL_POINTER ||
        written != 0 || required != 0) {
        rc = fail("sidereon_sp3_satellites null sp3 clears counts", 1);
        goto cleanup;
    }
    if (sidereon_sp3_satellites(sp3, NULL, 0, &written, &required) != SIDEREON_STATUS_OK ||
        written != 0 || required != SP3_SURFACE_SAT_COUNT) {
        rc = fail("sidereon_sp3_satellites size query", 1);
        goto cleanup;
    }
    SidereonSatelliteToken short_satellites[1];
    written = 123;
    required = 123;
    if (sidereon_sp3_satellites(sp3, short_satellites, 1, &written, &required) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        written != 0 || required != SP3_SURFACE_SAT_COUNT) {
        rc = fail("sidereon_sp3_satellites short buffer", 1);
        goto cleanup;
    }
    SidereonSatelliteToken satellites[SP3_SURFACE_SAT_COUNT];
    written = 123;
    required = 123;
    if (sidereon_sp3_satellites(sp3, satellites, SP3_SURFACE_SAT_COUNT, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != SP3_SURFACE_SAT_COUNT || required != SP3_SURFACE_SAT_COUNT) {
        rc = fail("sidereon_sp3_satellites full copy", 1);
        goto cleanup;
    }
    for (size_t i = 0; i < SP3_SURFACE_SAT_COUNT; i++) {
        if (!token_equals(&satellites[i], SP3_SURFACE_SAT_IDS[i])) {
            rc = fail("sidereon_sp3_satellites token order", 1);
            goto cleanup;
        }
    }

    double epochs[SP3_SURFACE_EPOCH_COUNT];
    written = 123;
    required = 123;
    if (sidereon_sp3_epochs_j2000_seconds(sp3, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != 0 || required != SP3_SURFACE_EPOCH_COUNT) {
        rc = fail("sidereon_sp3_epochs_j2000_seconds size query", 1);
        goto cleanup;
    }
    if (sidereon_sp3_epochs_j2000_seconds(
            sp3, epochs, SP3_SURFACE_EPOCH_COUNT, &written, &required) != SIDEREON_STATUS_OK ||
        written != SP3_SURFACE_EPOCH_COUNT || required != SP3_SURFACE_EPOCH_COUNT) {
        rc = fail("sidereon_sp3_epochs_j2000_seconds full copy", 1);
        goto cleanup;
    }
    for (size_t i = 0; i < SP3_SURFACE_EPOCH_COUNT; i++) {
        if (f64_to_bits(epochs[i]) != SP3_SURFACE_EPOCH_BITS[i]) {
            rc = fail("sidereon_sp3_epochs_j2000_seconds exact bits", 1);
            goto cleanup;
        }
    }

    SidereonSp3State state;
    if (sidereon_sp3_state(sp3, "G01", 0, &state) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_sp3_state", 1);
        goto cleanup;
    }
    for (size_t i = 0; i < 3; i++) {
        if (f64_to_bits(state.position_m[i]) != SP3_STATE_G01_EPOCH0_POSITION_BITS[i]) {
            rc = fail("sidereon_sp3_state position bits", 1);
            goto cleanup;
        }
    }
    if (!state.has_clock_s ||
        f64_to_bits(state.clock_s) != SP3_STATE_G01_EPOCH0_CLOCK_BITS ||
        state.has_velocity_m_s || state.has_clock_rate_s_s ||
        state.clock_event != (bool)SP3_STATE_G01_EPOCH0_CLOCK_EVENT ||
        state.clock_predicted != (bool)SP3_STATE_G01_EPOCH0_CLOCK_PREDICTED ||
        state.maneuver != (bool)SP3_STATE_G01_EPOCH0_MANEUVER ||
        state.orbit_predicted != (bool)SP3_STATE_G01_EPOCH0_ORBIT_PREDICTED) {
        rc = fail("sidereon_sp3_state optional fields and flags", 1);
        goto cleanup;
    }
    SidereonSp3State bad_state;
    if (sidereon_sp3_state(sp3, "G01", SP3_SURFACE_EPOCH_COUNT + 100, &bad_state) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        bad_state.has_clock_s || bad_state.position_m[0] != 0.0) {
        rc = fail("sidereon_sp3_state out-of-range clears output", 1);
        goto cleanup;
    }

    double queries[SP3_INTERP_QUERY_COUNT];
    for (size_t i = 0; i < SP3_INTERP_QUERY_COUNT; i++) {
        queries[i] = bits_to_f64(SP3_INTERP_QUERY_BITS[i]);
    }
    double positions[SP3_INTERP_QUERY_COUNT * 3];
    double clocks[SP3_INTERP_QUERY_COUNT];
    size_t interp_written = 123;
    if (sidereon_sp3_interpolate(
            sp3, "G01", NULL, 0, positions, SP3_INTERP_QUERY_COUNT * 3, clocks,
            SP3_INTERP_QUERY_COUNT, &interp_written) != SIDEREON_STATUS_INVALID_ARGUMENT ||
        interp_written != 0) {
        rc = fail("sidereon_sp3_interpolate empty query", 1);
        goto cleanup;
    }
    for (size_t sat = 0; sat < SP3_INTERP_SAT_COUNT; sat++) {
        interp_written = 123;
        if (sidereon_sp3_interpolate(
                sp3, SP3_INTERP_SAT_IDS[sat], queries, SP3_INTERP_QUERY_COUNT, positions,
                SP3_INTERP_QUERY_COUNT * 3, clocks, SP3_INTERP_QUERY_COUNT,
                &interp_written) != SIDEREON_STATUS_OK ||
            interp_written != SP3_INTERP_QUERY_COUNT) {
            rc = fail("sidereon_sp3_interpolate", 1);
            goto cleanup;
        }
        for (size_t q = 0; q < SP3_INTERP_QUERY_COUNT; q++) {
            for (size_t axis = 0; axis < 3; axis++) {
                if (f64_to_bits(positions[q * 3 + axis]) !=
                    SP3_INTERP_POSITION_BITS[sat][q][axis]) {
                    rc = fail("sidereon_sp3_interpolate position bits", 1);
                    goto cleanup;
                }
            }
            if (f64_to_bits(clocks[q]) != SP3_INTERP_CLOCK_BITS[sat][q]) {
                rc = fail("sidereon_sp3_interpolate clock bits", 1);
                goto cleanup;
            }
        }
    }

    written = 123;
    required = 123;
    if (sidereon_sp3_to_sp3_text(sp3, NULL, 0, &written, &required) != SIDEREON_STATUS_OK ||
        written != 0 || required != SP3_TO_SP3_TEXT_LEN ||
        strlen(SP3_TO_SP3_TEXT) != SP3_TO_SP3_TEXT_LEN) {
        rc = fail("sidereon_sp3_to_sp3_text size query", 1);
        goto cleanup;
    }
    uint8_t short_text[1] = {42};
    written = 123;
    required = 123;
    if (sidereon_sp3_to_sp3_text(sp3, short_text, 1, &written, &required) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        written != 0 || required != SP3_TO_SP3_TEXT_LEN || short_text[0] != 42) {
        rc = fail("sidereon_sp3_to_sp3_text short buffer", 1);
        goto cleanup;
    }
    uint8_t *text = (uint8_t *)malloc(SP3_TO_SP3_TEXT_LEN);
    if (text == NULL) {
        fprintf(stderr, "FAIL: could not allocate SP3 text buffer\n");
        goto cleanup;
    }
    written = 123;
    required = 123;
    if (sidereon_sp3_to_sp3_text(sp3, text, SP3_TO_SP3_TEXT_LEN, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != SP3_TO_SP3_TEXT_LEN || required != SP3_TO_SP3_TEXT_LEN ||
        memcmp(text, SP3_TO_SP3_TEXT, SP3_TO_SP3_TEXT_LEN) != 0) {
        free(text);
        rc = fail("sidereon_sp3_to_sp3_text exact bytes", 1);
        goto cleanup;
    }
    if (sidereon_sp3_load(text, written, &roundtrip) != SIDEREON_STATUS_OK) {
        free(text);
        rc = fail("sidereon_sp3_to_sp3_text roundtrip load", 1);
        goto cleanup;
    }
    free(text);
    size_t roundtrip_epochs = 0;
    if (sidereon_sp3_epoch_count(roundtrip, &roundtrip_epochs) != SIDEREON_STATUS_OK ||
        roundtrip_epochs != SP3_SURFACE_EPOCH_COUNT) {
        rc = fail("sidereon_sp3_to_sp3_text roundtrip epoch count", 1);
        goto cleanup;
    }

    SidereonSp3MergeOptions merge_options;
    if (sidereon_sp3_merge_options_init(&merge_options) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_sp3_merge_options_init", 1);
        goto cleanup;
    }
    merge_options.min_agree = 1;
    merge_options.clock_min_common = 1;
    SidereonSp3 *empty_merged = (SidereonSp3 *)(uintptr_t)1;
    SidereonSp3MergeReport *empty_report = (SidereonSp3MergeReport *)(uintptr_t)1;
    if (sidereon_sp3_merge(NULL, 0, NULL, &empty_merged, &empty_report) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        empty_merged != NULL || empty_report != NULL) {
        rc = fail("sidereon_sp3_merge empty sources clears outputs", 1);
        goto cleanup;
    }
    SidereonSp3 *oversized_merged = (SidereonSp3 *)(uintptr_t)1;
    SidereonSp3MergeReport *oversized_report = (SidereonSp3MergeReport *)(uintptr_t)1;
    if (sidereon_sp3_merge(NULL, (size_t)-1, NULL, &oversized_merged, &oversized_report) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        oversized_merged != NULL || oversized_report != NULL) {
        rc = fail("sidereon_sp3_merge oversized source_count clears outputs", 1);
        goto cleanup;
    }
    const SidereonSp3 *sources[1] = {sp3};
    if (sidereon_sp3_merge(sources, 1, &merge_options, &merged, &report) !=
        SIDEREON_STATUS_OK) {
        rc = fail("sidereon_sp3_merge single source", 1);
        goto cleanup;
    }
    size_t merged_epochs = 0;
    if (sidereon_sp3_epoch_count(merged, &merged_epochs) != SIDEREON_STATUS_OK ||
        merged_epochs != SP3_SURFACE_EPOCH_COUNT) {
        rc = fail("sidereon_sp3_merge merged epoch count", 1);
        goto cleanup;
    }
    size_t merge_quarantined = 123;
    size_t merge_single_source = 123;
    size_t merge_outliers = 123;
    if (sidereon_sp3_merge_report_flag_count(
            NULL, SIDEREON_SP3_MERGE_FLAG_KIND_SINGLE_SOURCE, &merge_single_source) !=
            SIDEREON_STATUS_NULL_POINTER ||
        merge_single_source != 0) {
        rc = fail("sidereon_sp3_merge_report_flag_count null report clears count", 1);
        goto cleanup;
    }
    if (sidereon_sp3_merge_report_flag_count(
            report, SIDEREON_SP3_MERGE_FLAG_KIND_QUARANTINED, &merge_quarantined) !=
            SIDEREON_STATUS_OK ||
        sidereon_sp3_merge_report_flag_count(
            report, SIDEREON_SP3_MERGE_FLAG_KIND_SINGLE_SOURCE, &merge_single_source) !=
            SIDEREON_STATUS_OK ||
        sidereon_sp3_merge_report_flag_count(
            report, SIDEREON_SP3_MERGE_FLAG_KIND_POSITION_OUTLIER, &merge_outliers) !=
            SIDEREON_STATUS_OK ||
        merge_quarantined != 0 ||
        merge_single_source != SP3_SURFACE_EPOCH_COUNT * SP3_SURFACE_SAT_COUNT ||
        merge_outliers != 0) {
        rc = fail("sidereon_sp3_merge_report_flag_count values", 1);
        goto cleanup;
    }
    SidereonSp3MergeFlag flag;
    if (sidereon_sp3_merge_report_flag(
            report, SIDEREON_SP3_MERGE_FLAG_KIND_SINGLE_SOURCE, 0, &flag) != SIDEREON_STATUS_OK ||
        !isfinite(flag.epoch_j2000_seconds) || !token_equals(&flag.sat_id, SP3_SURFACE_SAT_IDS[0]) ||
        flag.source_count != 1) {
        rc = fail("sidereon_sp3_merge_report_flag first single-source flag", 1);
        goto cleanup;
    }
    size_t source_written = 123;
    size_t source_required = 123;
    if (sidereon_sp3_merge_report_flag_sources(
            report, SIDEREON_SP3_MERGE_FLAG_KIND_SINGLE_SOURCE, 0, NULL, 0, &source_written,
            &source_required) != SIDEREON_STATUS_OK ||
        source_written != 0 || source_required != 1) {
        rc = fail("sidereon_sp3_merge_report_flag_sources size query", 1);
        goto cleanup;
    }
    size_t source_indices[1] = {99};
    if (sidereon_sp3_merge_report_flag_sources(
            report, SIDEREON_SP3_MERGE_FLAG_KIND_SINGLE_SOURCE, 0, source_indices, 1,
            &source_written, &source_required) != SIDEREON_STATUS_OK ||
        source_written != 1 || source_required != 1 || source_indices[0] != 0) {
        rc = fail("sidereon_sp3_merge_report_flag_sources full copy", 1);
        goto cleanup;
    }

    // Agreement metric on the single-source report: every accepted cell is
    // single-source, so per-epoch agreement lists one entry per output epoch with
    // no multi-source satellites, and the product summary has no position RMS (the
    // pooled RMS is absent when no cell had >= 2 consensus members).
    size_t epoch_agreement_count = 123;
    if (sidereon_sp3_merge_report_epoch_agreement_count(NULL, &epoch_agreement_count) !=
            SIDEREON_STATUS_NULL_POINTER ||
        epoch_agreement_count != 0) {
        rc = fail("sidereon_sp3_merge_report_epoch_agreement_count null clears count", 1);
        goto cleanup;
    }
    if (sidereon_sp3_merge_report_epoch_agreement_count(report, &epoch_agreement_count) !=
            SIDEREON_STATUS_OK ||
        epoch_agreement_count != SP3_SURFACE_EPOCH_COUNT) {
        rc = fail("sidereon_sp3_merge_report_epoch_agreement_count value", 1);
        goto cleanup;
    }
    SidereonSp3EpochAgreement single_epoch_agreement;
    if (sidereon_sp3_merge_report_epoch_agreement(report, 0, &single_epoch_agreement) !=
            SIDEREON_STATUS_OK ||
        !isfinite(single_epoch_agreement.epoch_j2000_seconds) ||
        single_epoch_agreement.satellites != 0 ||
        single_epoch_agreement.position_rms_m != 0.0 ||
        single_epoch_agreement.clock_rms_present) {
        rc = fail("sidereon_sp3_merge_report_epoch_agreement single-source entry", 1);
        goto cleanup;
    }
    if (sidereon_sp3_merge_report_epoch_agreement(report, SP3_SURFACE_EPOCH_COUNT,
                                                  &single_epoch_agreement) !=
        SIDEREON_STATUS_INVALID_ARGUMENT) {
        rc = fail("sidereon_sp3_merge_report_epoch_agreement out-of-range index", 1);
        goto cleanup;
    }
    SidereonSp3AgreementSummary single_summary;
    if (sidereon_sp3_merge_report_agreement_summary(report, &single_summary) !=
            SIDEREON_STATUS_OK ||
        single_summary.position_rms_present || single_summary.clock_rms_present ||
        !single_summary.position_max_present) {
        rc = fail("sidereon_sp3_merge_report_agreement_summary single-source", 1);
        goto cleanup;
    }

    // Two identical sources form a 2-member consensus per cell: the members agree
    // exactly, so the dispersion is zero but the multi-source path is exercised
    // (present flags true, RMS == 0).
    const SidereonSp3 *sources2[2] = {sp3, sp3};
    if (sidereon_sp3_merge(sources2, 2, &merge_options, &merged2, &report2) !=
        SIDEREON_STATUS_OK) {
        rc = fail("sidereon_sp3_merge two identical sources", 1);
        goto cleanup;
    }
    SidereonSp3EpochAgreement multi_epoch_agreement;
    if (sidereon_sp3_merge_report_epoch_agreement(report2, 0, &multi_epoch_agreement) !=
            SIDEREON_STATUS_OK ||
        multi_epoch_agreement.satellites != SP3_SURFACE_SAT_COUNT ||
        multi_epoch_agreement.position_rms_m != 0.0 ||
        multi_epoch_agreement.position_max_m != 0.0) {
        rc = fail("sidereon_sp3_merge_report_epoch_agreement multi-source entry", 1);
        goto cleanup;
    }
    SidereonSp3AgreementSummary multi_summary;
    if (sidereon_sp3_merge_report_agreement_summary(report2, &multi_summary) !=
            SIDEREON_STATUS_OK ||
        !multi_summary.position_rms_present || multi_summary.position_rms_m != 0.0 ||
        !multi_summary.position_max_present || multi_summary.position_max_m != 0.0) {
        rc = fail("sidereon_sp3_merge_report_agreement_summary multi-source", 1);
        goto cleanup;
    }

    printf("SP3 surface: %zu epochs, %zu satellites, %zu single-source merge flags, "
           "%zu agreement epochs\n",
           (size_t)SP3_SURFACE_EPOCH_COUNT, (size_t)SP3_SURFACE_SAT_COUNT,
           merge_single_source, epoch_agreement_count);
    rc = 0;

cleanup:
    sidereon_sp3_merge_report_free(report2);
    sidereon_sp3_free(merged2);
    sidereon_sp3_merge_report_free(report);
    sidereon_sp3_free(merged);
    sidereon_sp3_free(roundtrip);
    sidereon_sp3_free(sp3);
    free(sp3_bytes);
    return rc;
}

static void fill_rtk_rows(const uint64_t rover_phase_bits[CFIX_RTK_SAT_COUNT],
                          SidereonRtkSatMeasurement *reference,
                          SidereonRtkSatMeasurement nonref[CFIX_RTK_NONREF_COUNT]) {
    SidereonRtkSatMeasurement rows[CFIX_RTK_SAT_COUNT];
    for (size_t i = 0; i < CFIX_RTK_SAT_COUNT; i++) {
        rows[i].sat_id = CFIX_RTK_SAT_IDS[i];
        rows[i].sd_ambiguity_id = CFIX_RTK_SAT_IDS[i];
        rows[i].base_code_m = bits_to_f64(CFIX_RTK_BASE_CODE_BITS[i]);
        rows[i].base_phase_m = bits_to_f64(CFIX_RTK_BASE_CODE_BITS[i]);
        rows[i].rover_code_m = bits_to_f64(CFIX_RTK_ROVER_CODE_BITS[i]);
        rows[i].rover_phase_m = bits_to_f64(rover_phase_bits[i]);
        for (size_t axis = 0; axis < 3; axis++) {
            rows[i].base_tx_pos[axis] = CFIX_RTK_SAT_POS_M[i][axis];
            rows[i].rover_tx_pos[axis] = CFIX_RTK_SAT_POS_M[i][axis];
            rows[i].pos[axis] = CFIX_RTK_SAT_POS_M[i][axis];
        }
    }
    *reference = rows[0];
    for (size_t i = 0; i < CFIX_RTK_NONREF_COUNT; i++) {
        nonref[i] = rows[i + 1];
    }
}

static void fill_rtk_epoch(const uint64_t rover_phase_bits[CFIX_RTK_SAT_COUNT],
                           SidereonRtkSatMeasurement *reference,
                           SidereonRtkSatMeasurement nonref[CFIX_RTK_NONREF_COUNT],
                           SidereonRtkEpoch *epoch) {
    fill_rtk_rows(rover_phase_bits, reference, nonref);
    epoch->references = reference;
    epoch->reference_count = 1;
    epoch->nonref = nonref;
    epoch->nonref_count = CFIX_RTK_NONREF_COUNT;
    epoch->has_velocity_mps = false;
    epoch->velocity_mps[0] = 0.0;
    epoch->velocity_mps[1] = 0.0;
    epoch->velocity_mps[2] = 0.0;
    epoch->dt_s = 0.0;
}

static const SidereonReceiverAntennaNoaziPcvSample ZERO_RECEIVER_NOAZI_PCV[2] = {
    {0.0, 0.0},
    {90.0, 0.0},
};

static void configure_zero_receiver_antenna_calibration(
    SidereonReceiverAntennaCalibration *calibration) {
    for (size_t axis = 0; axis < 3; axis++) {
        calibration->pco_neu_m[axis] = 0.0;
    }
    calibration->noazi_pcv_m = ZERO_RECEIVER_NOAZI_PCV;
    calibration->noazi_pcv_count = 2;
    calibration->azimuth_pcv_m = NULL;
    calibration->azimuth_pcv_count = 0;
}

static void configure_rtk_zero_receiver_antenna(
    SidereonRtkReceiverAntennaCorrections *antenna) {
    configure_zero_receiver_antenna_calibration(&antenna->base);
    configure_zero_receiver_antenna_calibration(&antenna->rover);
}

static void configure_rtk_model(SidereonRtkMeasurementModel *model) {
    sidereon_rtk_measurement_model_init(model);
    model->code_sigma_m = 0.3;
    model->phase_sigma_m = 0.003;
    model->sagnac = false;
    model->stochastic = SIDEREON_RTK_STOCHASTIC_MODEL_SIMPLE;
    model->elevation_weighting = false;
}

static void configure_rtk_float_options(SidereonRtkFloatOptions *options) {
    sidereon_rtk_float_options_init(options);
    options->position_tol_m = 1.0e-3;
    options->ambiguity_tol_m = 1.0e-6;
    options->max_iterations = 10;
}

static int check_rtk_float_solution(const SidereonRtkFloatSolution *solution) {
    double baseline[3];
    if (sidereon_rtk_float_solution_baseline_ecef(solution, baseline, 3) != SIDEREON_STATUS_OK) {
        return fail("sidereon_rtk_float_solution_baseline_ecef", 1);
    }
    for (size_t axis = 0; axis < 3; axis++) {
        if (f64_to_bits(baseline[axis]) != CFIX_RTK_FLOAT_BASELINE_BITS[axis]) {
            return fail("rtk float baseline bits", 1);
        }
    }

    double enu[3];
    if (sidereon_rtk_float_solution_baseline_enu(solution, enu, 3) != SIDEREON_STATUS_OK ||
        !isfinite(enu[0]) || !isfinite(enu[1]) || !isfinite(enu[2])) {
        return fail("sidereon_rtk_float_solution_baseline_enu", 1);
    }
    /* The ENU projection is the baseline expressed in core's geocentric NEU basis
     * (sidereon_core::frame::geocentric_neu_basis), an orthonormal rotation, so it
     * must preserve the ECEF baseline length. */
    {
        double ecef_norm =
            sqrt(baseline[0] * baseline[0] + baseline[1] * baseline[1] + baseline[2] * baseline[2]);
        double enu_norm = sqrt(enu[0] * enu[0] + enu[1] * enu[1] + enu[2] * enu[2]);
        if (fabs(enu_norm - ecef_norm) > 1e-6) {
            return fail("rtk float baseline enu length preserved", 1);
        }
    }

    SidereonRtkFloatMetadata metadata;
    if (sidereon_rtk_float_solution_metadata(solution, &metadata) != SIDEREON_STATUS_OK ||
        metadata.iterations != 3 || !metadata.converged ||
        metadata.status != SIDEREON_RTK_SOLVE_STATUS_STATE_TOLERANCE ||
        metadata.n_observations != 8 || metadata.ambiguity_count != CFIX_RTK_NONREF_COUNT ||
        metadata.residual_count != CFIX_RTK_NONREF_COUNT ||
        metadata.used_sat_count != CFIX_RTK_NONREF_COUNT ||
        metadata.geometry_quality.tier != SIDEREON_OBSERVABILITY_TIER_NOMINAL ||
        metadata.geometry_quality.redundancy != 1 || metadata.geometry_quality.rank != 7 ||
        !isfinite(metadata.geometry_quality.condition_number) ||
        !isfinite(metadata.geometry_quality.gdop) || metadata.geometry_quality.gdop <= 0.0 ||
        !metadata.geometry_quality.raim_checkable ||
        !metadata.geometry_quality.covariance_validated ||
        f64_to_bits(metadata.code_rms_m) != CFIX_RTK_FLOAT_CODE_RMS_BITS ||
        f64_to_bits(metadata.phase_rms_m) != CFIX_RTK_FLOAT_PHASE_RMS_BITS ||
        f64_to_bits(metadata.weighted_rms_m) != CFIX_RTK_FLOAT_WEIGHTED_RMS_BITS) {
        return fail("sidereon_rtk_float_solution_metadata", 1);
    }

    size_t written = 123;
    size_t required = 123;
    if (sidereon_rtk_float_solution_ambiguities(solution, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != 0 || required != CFIX_RTK_NONREF_COUNT) {
        return fail("sidereon_rtk_float_solution_ambiguities size query", 1);
    }
    SidereonRtkAmbiguity ambiguities[CFIX_RTK_NONREF_COUNT];
    if (sidereon_rtk_float_solution_ambiguities(
            solution, ambiguities, CFIX_RTK_NONREF_COUNT, &written, &required) != SIDEREON_STATUS_OK ||
        written != CFIX_RTK_NONREF_COUNT || required != CFIX_RTK_NONREF_COUNT) {
        return fail("sidereon_rtk_float_solution_ambiguities full copy", 1);
    }
    for (size_t i = 0; i < CFIX_RTK_NONREF_COUNT; i++) {
        if (!rtk_id_equals(&ambiguities[i].id, CFIX_RTK_AMBIGUITY_IDS[i]) ||
            f64_to_bits(ambiguities[i].value_m) != CFIX_RTK_FLOAT_AMBIGUITY_BITS[i]) {
            return fail("rtk float ambiguity bits", 1);
        }
    }

    SidereonSatelliteToken used[CFIX_RTK_NONREF_COUNT];
    if (sidereon_rtk_float_solution_used_sat_ids(
            solution, used, CFIX_RTK_NONREF_COUNT, &written, &required) != SIDEREON_STATUS_OK ||
        written != CFIX_RTK_NONREF_COUNT || required != CFIX_RTK_NONREF_COUNT) {
        return fail("sidereon_rtk_float_solution_used_sat_ids full copy", 1);
    }
    for (size_t i = 0; i < CFIX_RTK_NONREF_COUNT; i++) {
        if (!token_equals(&used[i], CFIX_RTK_AMBIGUITY_IDS[i])) {
            return fail("rtk float used satellite order", 1);
        }
    }
    return 0;
}

static int check_rtk_fixed_solution(const SidereonRtkFixedSolution *solution) {
    double baseline[3];
    if (sidereon_rtk_fixed_solution_fixed_baseline_ecef(solution, baseline, 3) !=
        SIDEREON_STATUS_OK) {
        return fail("sidereon_rtk_fixed_solution_fixed_baseline_ecef", 1);
    }
    for (size_t axis = 0; axis < 3; axis++) {
        if (f64_to_bits(baseline[axis]) != CFIX_RTK_FIXED_BASELINE_BITS[axis]) {
            return fail("rtk fixed baseline bits", 1);
        }
    }

    double enu[3];
    if (sidereon_rtk_fixed_solution_fixed_baseline_enu(solution, enu, 3) != SIDEREON_STATUS_OK ||
        !isfinite(enu[0]) || !isfinite(enu[1]) || !isfinite(enu[2])) {
        return fail("sidereon_rtk_fixed_solution_fixed_baseline_enu", 1);
    }
    /* Orthonormal NEU rotation preserves the ECEF baseline length (see the float
     * path). */
    {
        double ecef_norm =
            sqrt(baseline[0] * baseline[0] + baseline[1] * baseline[1] + baseline[2] * baseline[2]);
        double enu_norm = sqrt(enu[0] * enu[0] + enu[1] * enu[1] + enu[2] * enu[2]);
        if (fabs(enu_norm - ecef_norm) > 1e-6) {
            return fail("rtk fixed baseline enu length preserved", 1);
        }
    }

    double float_baseline[3];
    if (sidereon_rtk_fixed_solution_float_baseline_ecef(solution, float_baseline, 3) !=
        SIDEREON_STATUS_OK) {
        return fail("sidereon_rtk_fixed_solution_float_baseline_ecef", 1);
    }

    SidereonRtkFixedMetadata metadata;
    if (sidereon_rtk_fixed_solution_metadata(solution, &metadata) != SIDEREON_STATUS_OK ||
        metadata.iterations != 1 || !metadata.converged ||
        metadata.status != SIDEREON_RTK_SOLVE_STATUS_STATE_TOLERANCE ||
        metadata.n_observations != 8 || metadata.free_ambiguity_count != 0 ||
        metadata.fixed_ambiguity_count != CFIX_RTK_NONREF_COUNT ||
        metadata.residual_count != CFIX_RTK_NONREF_COUNT ||
        metadata.used_sat_count != CFIX_RTK_NONREF_COUNT ||
        metadata.integer_status != SIDEREON_RTK_INTEGER_STATUS_FIXED ||
        !metadata.has_integer_ratio || !metadata.has_integer_best_score ||
        !metadata.has_integer_second_best_score ||
        metadata.geometry_quality.tier != SIDEREON_OBSERVABILITY_TIER_NOMINAL ||
        metadata.geometry_quality.redundancy != 1 || metadata.geometry_quality.rank != 7 ||
        !isfinite(metadata.geometry_quality.condition_number) ||
        !isfinite(metadata.geometry_quality.gdop) || metadata.geometry_quality.gdop <= 0.0 ||
        !metadata.geometry_quality.raim_checkable ||
        !metadata.geometry_quality.covariance_validated ||
        f64_to_bits(metadata.code_rms_m) != CFIX_RTK_FIXED_CODE_RMS_BITS ||
        f64_to_bits(metadata.phase_rms_m) != CFIX_RTK_FIXED_PHASE_RMS_BITS ||
        f64_to_bits(metadata.weighted_rms_m) != CFIX_RTK_FIXED_WEIGHTED_RMS_BITS ||
        f64_to_bits(metadata.integer_ratio) != CFIX_RTK_FIXED_RATIO_BITS ||
        f64_to_bits(metadata.integer_best_score) != CFIX_RTK_FIXED_BEST_SCORE_BITS ||
        f64_to_bits(metadata.integer_second_best_score) != CFIX_RTK_FIXED_SECOND_BEST_SCORE_BITS ||
        metadata.integer_candidates == 0) {
        return fail("sidereon_rtk_fixed_solution_metadata", 1);
    }

    size_t written = 123;
    size_t required = 123;
    if (sidereon_rtk_fixed_solution_free_ambiguities(solution, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != 0 || required != 0) {
        return fail("sidereon_rtk_fixed_solution_free_ambiguities size query", 1);
    }

    SidereonRtkFixedAmbiguity fixed[CFIX_RTK_NONREF_COUNT];
    if (sidereon_rtk_fixed_solution_fixed_ambiguities(
            solution, fixed, CFIX_RTK_NONREF_COUNT, &written, &required) != SIDEREON_STATUS_OK ||
        written != CFIX_RTK_NONREF_COUNT || required != CFIX_RTK_NONREF_COUNT) {
        return fail("sidereon_rtk_fixed_solution_fixed_ambiguities full copy", 1);
    }
    for (size_t i = 0; i < CFIX_RTK_NONREF_COUNT; i++) {
        if (!rtk_id_equals(&fixed[i].id, CFIX_RTK_AMBIGUITY_IDS[i]) ||
            fixed[i].cycles != CFIX_RTK_FIXED_AMBIGUITY_CYCLES[i] ||
            f64_to_bits(fixed[i].value_m) != CFIX_RTK_FIXED_AMBIGUITY_M_BITS[i]) {
            return fail("rtk fixed ambiguity bits", 1);
        }
    }

    SidereonSatelliteToken used[CFIX_RTK_NONREF_COUNT];
    if (sidereon_rtk_fixed_solution_used_sat_ids(
            solution, used, CFIX_RTK_NONREF_COUNT, &written, &required) != SIDEREON_STATUS_OK ||
        written != CFIX_RTK_NONREF_COUNT || required != CFIX_RTK_NONREF_COUNT) {
        return fail("sidereon_rtk_fixed_solution_used_sat_ids full copy", 1);
    }
    for (size_t i = 0; i < CFIX_RTK_NONREF_COUNT; i++) {
        if (!token_equals(&used[i], CFIX_RTK_AMBIGUITY_IDS[i])) {
            return fail("rtk fixed used satellite order", 1);
        }
    }
    return 0;
}

static int exercise_rtk_surface(void) {
    SidereonRtkSatMeasurement float_ref;
    SidereonRtkSatMeasurement float_nonref[CFIX_RTK_NONREF_COUNT];
    SidereonRtkEpoch float_epoch;
    fill_rtk_epoch(CFIX_RTK_FLOAT_ROVER_PHASE_BITS, &float_ref, float_nonref, &float_epoch);

    SidereonRtkMeasurementModel model;
    configure_rtk_model(&model);
    SidereonRtkFloatOptions float_options;
    configure_rtk_float_options(&float_options);
    SidereonRtkReceiverAntennaCorrections rtk_receiver_antenna;
    configure_rtk_zero_receiver_antenna(&rtk_receiver_antenna);

    SidereonRtkFloatConfig float_config;
    float_config.epochs = &float_epoch;
    float_config.epoch_count = 1;
    for (size_t axis = 0; axis < 3; axis++) {
        float_config.base_ecef_m[axis] = CFIX_RTK_BASE_ECEF_M[axis];
        float_config.initial_baseline_m[axis] = CFIX_RTK_INITIAL_BASELINE_M[axis];
    }
    float_config.ambiguity_ids = CFIX_RTK_AMBIGUITY_IDS;
    float_config.ambiguity_id_count = CFIX_RTK_NONREF_COUNT;
    float_config.model = model;
    float_config.receiver_antenna = &rtk_receiver_antenna;
    float_config.options = float_options;

    SidereonRtkFloatSolution *float_solution = NULL;
    if (sidereon_solve_rtk_float(&float_config, &float_solution) != SIDEREON_STATUS_OK) {
        return fail("sidereon_solve_rtk_float", 1);
    }
    int rc = check_rtk_float_solution(float_solution);
    sidereon_rtk_float_solution_free(float_solution);
    if (rc != 0) {
        return rc;
    }

    SidereonRtkEpoch singular_epoch;
    SidereonRtkSatMeasurement singular_ref;
    SidereonRtkSatMeasurement singular_nonref[CFIX_RTK_NONREF_COUNT];
    fill_rtk_epoch(CFIX_RTK_FLOAT_ROVER_PHASE_BITS, &singular_ref, singular_nonref,
                   &singular_epoch);
    for (size_t i = 1; i < CFIX_RTK_NONREF_COUNT; i++) {
        memcpy(singular_nonref[i].base_tx_pos, singular_nonref[0].base_tx_pos,
               sizeof(singular_nonref[i].base_tx_pos));
        memcpy(singular_nonref[i].rover_tx_pos, singular_nonref[0].rover_tx_pos,
               sizeof(singular_nonref[i].rover_tx_pos));
        memcpy(singular_nonref[i].pos, singular_nonref[0].pos, sizeof(singular_nonref[i].pos));
    }
    SidereonRtkFloatConfig singular_float_config = float_config;
    singular_float_config.epochs = &singular_epoch;
    SidereonRtkFloatSolution *singular_float_solution = (SidereonRtkFloatSolution *)(uintptr_t)1;
    if (sidereon_solve_rtk_float(&singular_float_config, &singular_float_solution) !=
            SIDEREON_STATUS_SOLVE ||
        singular_float_solution != NULL || !last_error_contains("singular")) {
        return fail("sidereon_solve_rtk_float singular geometry", 1);
    }

    SidereonRtkSatMeasurement fixed_ref;
    SidereonRtkSatMeasurement fixed_nonref[CFIX_RTK_NONREF_COUNT];
    SidereonRtkEpoch fixed_epoch;
    fill_rtk_epoch(CFIX_RTK_FIXED_ROVER_PHASE_BITS, &fixed_ref, fixed_nonref, &fixed_epoch);

    SidereonRtkFixedOptions fixed_options;
    sidereon_rtk_fixed_options_init(&fixed_options);
    fixed_options.position_tol_m = 1.0e-3;
    fixed_options.ambiguity_tol_m = 1.0e-6;
    fixed_options.max_iterations = 10;
    fixed_options.ratio_threshold = 3.0;
    fixed_options.partial_ambiguity_resolution = false;
    fixed_options.partial_min_ambiguities = 4;
    SidereonRtkResidualValidationOptions residual_options;
    sidereon_rtk_residual_validation_options_init(&residual_options);

    SidereonRtkAmbiguitySatellite ambiguity_satellites[CFIX_RTK_NONREF_COUNT];
    SidereonRtkFloatMapEntry wavelengths[CFIX_RTK_NONREF_COUNT];
    SidereonRtkFloatMapEntry offsets[CFIX_RTK_NONREF_COUNT];
    for (size_t i = 0; i < CFIX_RTK_NONREF_COUNT; i++) {
        ambiguity_satellites[i].id = CFIX_RTK_AMBIGUITY_IDS[i];
        ambiguity_satellites[i].sat_id = CFIX_RTK_AMBIGUITY_IDS[i];
        wavelengths[i].id = CFIX_RTK_AMBIGUITY_IDS[i];
        wavelengths[i].value = bits_to_f64(CFIX_RTK_L1_WAVELENGTH_M_BITS);
        offsets[i].id = CFIX_RTK_AMBIGUITY_IDS[i];
        offsets[i].value = 0.0;
    }

    SidereonRtkFixedConfig fixed_config;
    fixed_config.epochs = &fixed_epoch;
    fixed_config.epoch_count = 1;
    for (size_t axis = 0; axis < 3; axis++) {
        fixed_config.base_ecef_m[axis] = CFIX_RTK_BASE_ECEF_M[axis];
        fixed_config.initial_baseline_m[axis] = CFIX_RTK_INITIAL_BASELINE_M[axis];
    }
    fixed_config.ambiguity_ids = CFIX_RTK_AMBIGUITY_IDS;
    fixed_config.ambiguity_id_count = CFIX_RTK_NONREF_COUNT;
    fixed_config.ambiguity_satellites = ambiguity_satellites;
    fixed_config.ambiguity_satellite_count = CFIX_RTK_NONREF_COUNT;
    fixed_config.wavelengths_m = wavelengths;
    fixed_config.wavelength_count = CFIX_RTK_NONREF_COUNT;
    fixed_config.offsets_m = offsets;
    fixed_config.offset_count = CFIX_RTK_NONREF_COUNT;
    fixed_config.model = model;
    fixed_config.receiver_antenna = &rtk_receiver_antenna;
    fixed_config.float_options = float_options;
    fixed_config.fixed_options = fixed_options;
    fixed_config.residual_options = residual_options;
    fixed_config.float_only_systems = NULL;
    fixed_config.float_only_system_count = 0;

    SidereonRtkFixedSolution *fixed_solution = NULL;
    if (sidereon_solve_rtk_fixed(&fixed_config, &fixed_solution) != SIDEREON_STATUS_OK) {
        return fail("sidereon_solve_rtk_fixed", 1);
    }
    rc = check_rtk_fixed_solution(fixed_solution);
    sidereon_rtk_fixed_solution_free(fixed_solution);
    if (rc != 0) {
        return rc;
    }

    SidereonRtkFloatMapEntry duplicate_wavelengths[CFIX_RTK_NONREF_COUNT];
    memcpy(duplicate_wavelengths, wavelengths, sizeof(wavelengths));
    duplicate_wavelengths[1].id = duplicate_wavelengths[0].id;
    SidereonRtkFixedConfig duplicate_fixed_config = fixed_config;
    duplicate_fixed_config.wavelengths_m = duplicate_wavelengths;
    SidereonRtkFixedSolution *bad_fixed = (SidereonRtkFixedSolution *)(uintptr_t)1;
    if (sidereon_solve_rtk_fixed(&duplicate_fixed_config, &bad_fixed) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        bad_fixed != NULL) {
        return fail("sidereon_solve_rtk_fixed duplicate wavelength id", 1);
    }

    SidereonRtkAmbiguitySatellite duplicate_ambiguity_satellites[CFIX_RTK_NONREF_COUNT];
    memcpy(duplicate_ambiguity_satellites, ambiguity_satellites,
           sizeof(ambiguity_satellites));
    duplicate_ambiguity_satellites[1].id = duplicate_ambiguity_satellites[0].id;
    duplicate_fixed_config = fixed_config;
    duplicate_fixed_config.ambiguity_satellites = duplicate_ambiguity_satellites;
    bad_fixed = (SidereonRtkFixedSolution *)(uintptr_t)1;
    if (sidereon_solve_rtk_fixed(&duplicate_fixed_config, &bad_fixed) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        bad_fixed != NULL) {
        return fail("sidereon_solve_rtk_fixed duplicate ambiguity satellite id", 1);
    }

    SidereonRtkFixedConfig oversized_float_only_config = fixed_config;
    oversized_float_only_config.float_only_systems = NULL;
    oversized_float_only_config.float_only_system_count = (size_t)-1;
    bad_fixed = (SidereonRtkFixedSolution *)(uintptr_t)1;
    if (sidereon_solve_rtk_fixed(&oversized_float_only_config, &bad_fixed) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        bad_fixed != NULL) {
        return fail("sidereon_solve_rtk_fixed oversized float_only_system_count", 1);
    }

    SidereonRtkFloatSolution *bad_float = (SidereonRtkFloatSolution *)(uintptr_t)1;
    if (sidereon_solve_rtk_float(NULL, &bad_float) != SIDEREON_STATUS_NULL_POINTER ||
        bad_float != NULL) {
        return fail("sidereon_solve_rtk_float null config clears out_solution", 1);
    }

    printf("RTK surface: float and fixed one-epoch frozen-bit cases OK\n");
    return 0;
}

static void fill_ppp_epochs(SidereonPppObservation observations[PPP_OBS_COUNT],
                            SidereonPppEpoch epochs[PPP_EPOCH_COUNT]) {
    for (size_t i = 0; i < PPP_OBS_COUNT; i++) {
        observations[i].sat_id = PPP_OBS_SAT_IDS[i];
        observations[i].ambiguity_id = PPP_OBS_AMBIGUITY_IDS[i];
        observations[i].code_m = bits_to_f64(PPP_OBS_CODE_BITS[i]);
        observations[i].phase_m = bits_to_f64(PPP_OBS_PHASE_BITS[i]);
        observations[i].freq1_hz = bits_to_f64(PPP_OBS_FREQ1_HZ_BITS[i]);
        observations[i].freq2_hz = bits_to_f64(PPP_OBS_FREQ2_HZ_BITS[i]);
    }
    for (size_t i = 0; i < PPP_EPOCH_COUNT; i++) {
        epochs[i].civil.year = PPP_EPOCH_YEARS[i];
        epochs[i].civil.month = PPP_EPOCH_MONTHS[i];
        epochs[i].civil.day = PPP_EPOCH_DAYS[i];
        epochs[i].civil.hour = PPP_EPOCH_HOURS[i];
        epochs[i].civil.minute = PPP_EPOCH_MINUTES[i];
        epochs[i].civil.second = bits_to_f64(PPP_EPOCH_SECOND_BITS[i]);
        epochs[i].jd_whole = bits_to_f64(PPP_EPOCH_JD_WHOLE_BITS[i]);
        epochs[i].jd_fraction = bits_to_f64(PPP_EPOCH_JD_FRACTION_BITS[i]);
        epochs[i].t_rx_j2000_s = bits_to_f64(PPP_EPOCH_T_RX_J2000_S_BITS[i]);
        epochs[i].observations = &observations[PPP_EPOCH_OBS_OFFSETS[i]];
        epochs[i].observation_count = PPP_EPOCH_OBS_COUNTS[i];
    }
}

static void configure_ppp_weights(SidereonPppMeasurementWeights *weights) {
    sidereon_ppp_measurement_weights_init(weights);
    weights->code = bits_to_f64(PPP_WEIGHT_CODE_BITS);
    weights->phase = bits_to_f64(PPP_WEIGHT_PHASE_BITS);
    weights->elevation_weighting = (bool)PPP_WEIGHT_ELEVATION_WEIGHTING;
}

static void configure_ppp_tropo(SidereonPppTroposphereOptions *tropo) {
    sidereon_ppp_troposphere_options_init(tropo);
    tropo->enabled = (bool)PPP_TROPO_ENABLED;
    tropo->estimate_ztd = (bool)PPP_TROPO_ESTIMATE_ZTD;
    tropo->pressure_hpa = bits_to_f64(PPP_TROPO_PRESSURE_HPA_BITS);
    tropo->temperature_k = bits_to_f64(PPP_TROPO_TEMPERATURE_K_BITS);
    tropo->relative_humidity = bits_to_f64(PPP_TROPO_RELATIVE_HUMIDITY_BITS);
}

static void configure_ppp_options(SidereonPppFloatOptions *options) {
    sidereon_ppp_float_options_init(options);
    options->max_iterations = PPP_OPTS_MAX_ITERATIONS;
    options->position_tolerance_m = bits_to_f64(PPP_OPTS_POSITION_TOLERANCE_BITS);
    options->clock_tolerance_m = bits_to_f64(PPP_OPTS_CLOCK_TOLERANCE_BITS);
    options->ambiguity_tolerance_m = bits_to_f64(PPP_OPTS_AMBIGUITY_TOLERANCE_BITS);
    options->ztd_tolerance_m = bits_to_f64(PPP_OPTS_ZTD_TOLERANCE_BITS);
}

static void configure_ppp_zero_receiver_antenna(
    SidereonPppReceiverAntennaOptions *antenna) {
    antenna->freq1_label = "L1";
    antenna->freq1_hz = 1575420000.0;
    configure_zero_receiver_antenna_calibration(&antenna->freq1);
    antenna->freq2_label = "L2";
    antenna->freq2_hz = 1227600000.0;
    configure_zero_receiver_antenna_calibration(&antenna->freq2);
}

static void configure_ppp_corrections(
    SidereonPppRangeCorrections *corrections,
    SidereonPppReceiverAntennaOptions *antenna) {
    sidereon_ppp_range_corrections_init(corrections);
    configure_ppp_zero_receiver_antenna(antenna);
    corrections->receiver_antenna = antenna;
}

static void fill_ppp_initial_clocks(double clocks[PPP_EPOCH_COUNT]) {
    for (size_t i = 0; i < PPP_EPOCH_COUNT; i++) {
        clocks[i] = bits_to_f64(PPP_INITIAL_CLOCK_BITS[i]);
    }
}

static void fill_ppp_initial_ambiguities(
    SidereonPppFloatMapEntry initial_ambiguities[PPP_INITIAL_AMBIGUITY_COUNT]) {
    for (size_t i = 0; i < PPP_INITIAL_AMBIGUITY_COUNT; i++) {
        initial_ambiguities[i].id = PPP_INITIAL_AMBIGUITY_IDS[i];
        initial_ambiguities[i].value = bits_to_f64(PPP_INITIAL_AMBIGUITY_BITS[i]);
    }
}

static void fill_ppp_fixed_maps(
    SidereonPppFloatMapEntry wavelengths[PPP_FIXED_AMBIGUITY_COUNT],
    SidereonPppFloatMapEntry offsets[PPP_FIXED_AMBIGUITY_COUNT]) {
    for (size_t i = 0; i < PPP_FIXED_AMBIGUITY_COUNT; i++) {
        wavelengths[i].id = PPP_WAVELENGTH_IDS[i];
        wavelengths[i].value = bits_to_f64(PPP_WAVELENGTH_BITS[i]);
        offsets[i].id = PPP_OFFSET_IDS[i];
        offsets[i].value = bits_to_f64(PPP_OFFSET_BITS[i]);
    }
}

static void configure_ppp_float_config(
    SidereonPppFloatConfig *config,
    SidereonPppEpoch epochs[PPP_EPOCH_COUNT],
    const double clocks[PPP_EPOCH_COUNT],
    SidereonPppFloatMapEntry initial_ambiguities[PPP_INITIAL_AMBIGUITY_COUNT],
    SidereonPppReceiverAntennaOptions *antenna) {
    config->epochs = epochs;
    config->epoch_count = PPP_EPOCH_COUNT;
    for (size_t axis = 0; axis < 3; axis++) {
        config->initial_state.position_m[axis] =
            bits_to_f64(PPP_INITIAL_POSITION_BITS[axis]);
    }
    config->initial_state.clocks_m = clocks;
    config->initial_state.clock_count = PPP_EPOCH_COUNT;
    config->initial_state.ambiguities_m = initial_ambiguities;
    config->initial_state.ambiguity_count = PPP_INITIAL_AMBIGUITY_COUNT;
    config->initial_state.ztd_m = bits_to_f64(PPP_INITIAL_ZTD_BITS);
    configure_ppp_weights(&config->weights);
    configure_ppp_tropo(&config->tropo);
    configure_ppp_corrections(&config->corrections, antenna);
    configure_ppp_options(&config->options);
    config->residual_screen = (bool)PPP_RESIDUAL_SCREEN;
}

static void configure_ppp_fixed_config(
    SidereonPppFixedConfig *config,
    SidereonPppEpoch epochs[PPP_EPOCH_COUNT],
    const SidereonPppRangeCorrections *corrections,
    SidereonPppFloatMapEntry wavelengths[PPP_FIXED_AMBIGUITY_COUNT],
    SidereonPppFloatMapEntry offsets[PPP_FIXED_AMBIGUITY_COUNT]) {
    config->epochs = epochs;
    config->epoch_count = PPP_EPOCH_COUNT;
    configure_ppp_weights(&config->weights);
    configure_ppp_tropo(&config->tropo);
    config->corrections = *corrections;
    configure_ppp_options(&config->options);
    sidereon_ppp_fixed_ambiguity_options_init(&config->ambiguity);
    config->ambiguity.wavelengths_m = wavelengths;
    config->ambiguity.wavelength_count = PPP_FIXED_AMBIGUITY_COUNT;
    config->ambiguity.offsets_m = offsets;
    config->ambiguity.offset_count = PPP_FIXED_AMBIGUITY_COUNT;
    config->ambiguity.ratio_threshold = bits_to_f64(PPP_FIXED_RATIO_THRESHOLD_BITS);
}

static int check_ppp_float_solution(const SidereonPppFloatSolution *solution) {
    double position[3];
    if (sidereon_ppp_float_solution_position(solution, position, 3) != SIDEREON_STATUS_OK) {
        return fail("sidereon_ppp_float_solution_position", 1);
    }
    for (size_t axis = 0; axis < 3; axis++) {
        if (f64_to_bits(position[axis]) != PPP_EXPECTED_FLOAT_POSITION_BITS[axis]) {
            return fail("ppp float position bits", 1);
        }
    }

    SidereonPppFloatMetadata metadata;
    if (sidereon_ppp_float_solution_metadata(solution, &metadata) != SIDEREON_STATUS_OK ||
        metadata.iterations == 0 || !metadata.converged ||
        metadata.status != SIDEREON_PPP_SOLVE_STATUS_STATE_TOLERANCE ||
        metadata.has_ztd_residual_m || metadata.ambiguity_count != PPP_USED_SAT_COUNT ||
        metadata.residual_count != PPP_OBS_COUNT || metadata.used_sat_count != PPP_USED_SAT_COUNT ||
        !isfinite(metadata.code_rms_m) || !isfinite(metadata.phase_rms_m) ||
        !isfinite(metadata.weighted_rms_m)) {
        return fail("sidereon_ppp_float_solution_metadata", 1);
    }

    size_t written = 123;
    size_t required = 123;
    if (sidereon_ppp_float_solution_used_sat_ids(solution, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != 0 || required != PPP_USED_SAT_COUNT) {
        return fail("sidereon_ppp_float_solution_used_sat_ids size query", 1);
    }
    SidereonSatelliteToken short_used[1];
    if (sidereon_ppp_float_solution_used_sat_ids(solution, short_used, 1, &written, &required) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        written != 0 || required != PPP_USED_SAT_COUNT) {
        return fail("sidereon_ppp_float_solution_used_sat_ids short buffer", 1);
    }
    SidereonSatelliteToken used[PPP_USED_SAT_COUNT];
    if (sidereon_ppp_float_solution_used_sat_ids(
            solution, used, PPP_USED_SAT_COUNT, &written, &required) != SIDEREON_STATUS_OK ||
        written != PPP_USED_SAT_COUNT || required != PPP_USED_SAT_COUNT) {
        return fail("sidereon_ppp_float_solution_used_sat_ids full copy", 1);
    }
    for (size_t i = 0; i < PPP_USED_SAT_COUNT; i++) {
        if (!token_equals(&used[i], PPP_USED_SAT_IDS[i])) {
            return fail("ppp float used satellite order", 1);
        }
    }

    SidereonPppId used_ids[PPP_USED_SAT_COUNT];
    if (sidereon_ppp_float_solution_used_ids(
            solution, used_ids, PPP_USED_SAT_COUNT, &written, &required) != SIDEREON_STATUS_OK ||
        written != PPP_USED_SAT_COUNT || required != PPP_USED_SAT_COUNT) {
        return fail("sidereon_ppp_float_solution_used_ids full copy", 1);
    }
    for (size_t i = 0; i < PPP_USED_SAT_COUNT; i++) {
        if (!ppp_id_equals(&used_ids[i], PPP_USED_SAT_IDS[i])) {
            return fail("ppp float used id order", 1);
        }
    }

    SidereonPppAmbiguity ambiguities[PPP_USED_SAT_COUNT];
    if (sidereon_ppp_float_solution_ambiguities(
            solution, ambiguities, PPP_USED_SAT_COUNT, &written, &required) != SIDEREON_STATUS_OK ||
        written != PPP_USED_SAT_COUNT || required != PPP_USED_SAT_COUNT) {
        return fail("sidereon_ppp_float_solution_ambiguities full copy", 1);
    }
    for (size_t i = 0; i < PPP_USED_SAT_COUNT; i++) {
        if (!ppp_id_equals(&ambiguities[i].id, PPP_USED_SAT_IDS[i]) ||
            !isfinite(ambiguities[i].value_m)) {
            return fail("ppp float ambiguity ids", 1);
        }
    }
    return 0;
}

static int check_ppp_fixed_solution(const SidereonPppFixedSolution *solution) {
    double position[3];
    if (sidereon_ppp_fixed_solution_position(solution, position, 3) != SIDEREON_STATUS_OK) {
        return fail("sidereon_ppp_fixed_solution_position", 1);
    }
    for (size_t axis = 0; axis < 3; axis++) {
        if (f64_to_bits(position[axis]) != PPP_EXPECTED_FIXED_POSITION_BITS[axis]) {
            return fail("ppp fixed position bits", 1);
        }
    }

    double float_position[3];
    if (sidereon_ppp_fixed_solution_float_position(solution, float_position, 3) !=
        SIDEREON_STATUS_OK) {
        return fail("sidereon_ppp_fixed_solution_float_position", 1);
    }
    for (size_t axis = 0; axis < 3; axis++) {
        if (f64_to_bits(float_position[axis]) !=
            PPP_EXPECTED_FIXED_FLOAT_POSITION_BITS[axis]) {
            return fail("ppp fixed embedded float position bits", 1);
        }
    }

    SidereonPppFixedMetadata metadata;
    if (sidereon_ppp_fixed_solution_metadata(solution, &metadata) != SIDEREON_STATUS_OK ||
        metadata.iterations == 0 || !metadata.converged ||
        metadata.status != SIDEREON_PPP_SOLVE_STATUS_STATE_TOLERANCE ||
        metadata.has_ztd_residual_m || metadata.fixed_ambiguity_count != PPP_FIXED_AMBIGUITY_COUNT ||
        metadata.residual_count != PPP_OBS_COUNT || metadata.used_sat_count != PPP_USED_SAT_COUNT ||
        !ppp_integer_status_equals(
            metadata.integer_status, PPP_EXPECTED_FIXED_INTEGER_STATUS) ||
        f64_to_bits(metadata.integer_ratio) != PPP_EXPECTED_FIXED_INTEGER_RATIO_BITS ||
        metadata.integer_candidates != PPP_EXPECTED_FIXED_INTEGER_CANDIDATES ||
        !isfinite(metadata.code_rms_m) || !isfinite(metadata.phase_rms_m) ||
        !isfinite(metadata.weighted_rms_m) || !isfinite(metadata.integer_best_score) ||
        !metadata.has_integer_second_best_score ||
        !isfinite(metadata.integer_second_best_score)) {
        return fail("sidereon_ppp_fixed_solution_metadata", 1);
    }

    size_t written = 123;
    size_t required = 123;
    if (sidereon_ppp_fixed_solution_fixed_ambiguities(solution, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != 0 || required != PPP_FIXED_AMBIGUITY_COUNT) {
        return fail("sidereon_ppp_fixed_solution_fixed_ambiguities size query", 1);
    }
    SidereonPppFixedAmbiguity fixed[PPP_FIXED_AMBIGUITY_COUNT];
    if (sidereon_ppp_fixed_solution_fixed_ambiguities(
            solution, fixed, PPP_FIXED_AMBIGUITY_COUNT, &written, &required) != SIDEREON_STATUS_OK ||
        written != PPP_FIXED_AMBIGUITY_COUNT || required != PPP_FIXED_AMBIGUITY_COUNT) {
        return fail("sidereon_ppp_fixed_solution_fixed_ambiguities full copy", 1);
    }
    for (size_t i = 0; i < PPP_FIXED_AMBIGUITY_COUNT; i++) {
        if (!ppp_id_equals(&fixed[i].id, PPP_FIXED_AMBIGUITY_IDS[i]) ||
            fixed[i].cycles != PPP_FIXED_AMBIGUITY_CYCLES[i] ||
            f64_to_bits(fixed[i].value_m) != PPP_FIXED_AMBIGUITY_M_BITS[i]) {
            return fail("ppp fixed ambiguity bits", 1);
        }
    }

    SidereonSatelliteToken used[PPP_USED_SAT_COUNT];
    if (sidereon_ppp_fixed_solution_used_sat_ids(
            solution, used, PPP_USED_SAT_COUNT, &written, &required) != SIDEREON_STATUS_OK ||
        written != PPP_USED_SAT_COUNT || required != PPP_USED_SAT_COUNT) {
        return fail("sidereon_ppp_fixed_solution_used_sat_ids full copy", 1);
    }
    for (size_t i = 0; i < PPP_USED_SAT_COUNT; i++) {
        if (!token_equals(&used[i], PPP_USED_SAT_IDS[i])) {
            return fail("ppp fixed used satellite order", 1);
        }
    }
    SidereonPppId used_ids[PPP_USED_SAT_COUNT];
    if (sidereon_ppp_fixed_solution_used_ids(
            solution, used_ids, PPP_USED_SAT_COUNT, &written, &required) != SIDEREON_STATUS_OK ||
        written != PPP_USED_SAT_COUNT || required != PPP_USED_SAT_COUNT) {
        return fail("sidereon_ppp_fixed_solution_used_ids full copy", 1);
    }
    for (size_t i = 0; i < PPP_USED_SAT_COUNT; i++) {
        if (!ppp_id_equals(&used_ids[i], PPP_USED_SAT_IDS[i])) {
            return fail("ppp fixed used id order", 1);
        }
    }
    return 0;
}

static int check_ppp_long_used_ids(const SidereonSp3 *sp3) {
    static const char long_id[] = "G01#ppp_arc_012345678901234567890123456789";
    const char *replaced_id = PPP_USED_SAT_IDS[0];
    SidereonPppFloatSolution *float_solution = NULL;
    SidereonPppFixedSolution *fixed_solution = NULL;
    int rc = 1;

    SidereonPppObservation observations[PPP_OBS_COUNT];
    SidereonPppEpoch epochs[PPP_EPOCH_COUNT];
    fill_ppp_epochs(observations, epochs);
    for (size_t i = 0; i < PPP_OBS_COUNT; i++) {
        if (strcmp(observations[i].ambiguity_id, replaced_id) == 0) {
            observations[i].ambiguity_id = long_id;
        }
    }

    double clocks[PPP_EPOCH_COUNT];
    fill_ppp_initial_clocks(clocks);
    SidereonPppFloatMapEntry initial_ambiguities[PPP_INITIAL_AMBIGUITY_COUNT];
    fill_ppp_initial_ambiguities(initial_ambiguities);
    for (size_t i = 0; i < PPP_INITIAL_AMBIGUITY_COUNT; i++) {
        if (strcmp(initial_ambiguities[i].id, replaced_id) == 0) {
            initial_ambiguities[i].id = long_id;
        }
    }

    SidereonPppReceiverAntennaOptions receiver_antenna;
    SidereonPppFloatConfig float_config;
    configure_ppp_float_config(
        &float_config, epochs, clocks, initial_ambiguities, &receiver_antenna);

    if (sidereon_solve_ppp_float(sp3, &float_config, &float_solution) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_solve_ppp_float long ambiguity id", 1);
        goto cleanup;
    }

    size_t written = 123;
    size_t required = 123;
    SidereonPppId used_ids[PPP_USED_SAT_COUNT];
    if (sidereon_ppp_float_solution_used_ids(
            float_solution, used_ids, PPP_USED_SAT_COUNT, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != PPP_USED_SAT_COUNT || required != PPP_USED_SAT_COUNT ||
        !ppp_id_list_contains(used_ids, PPP_USED_SAT_COUNT, long_id)) {
        rc = fail("sidereon_ppp_float_solution_used_ids long id", 1);
        goto cleanup;
    }

    SidereonSatelliteToken narrow_used[PPP_USED_SAT_COUNT];
    written = 123;
    required = 123;
    if (sidereon_ppp_float_solution_used_sat_ids(
            float_solution, narrow_used, PPP_USED_SAT_COUNT, &written, &required) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        written != 0) {
        rc = fail("sidereon_ppp_float_solution_used_sat_ids rejects long id", 1);
        goto cleanup;
    }

    SidereonPppFloatMapEntry wavelengths[PPP_FIXED_AMBIGUITY_COUNT];
    SidereonPppFloatMapEntry offsets[PPP_FIXED_AMBIGUITY_COUNT];
    fill_ppp_fixed_maps(wavelengths, offsets);
    for (size_t i = 0; i < PPP_FIXED_AMBIGUITY_COUNT; i++) {
        if (strcmp(wavelengths[i].id, replaced_id) == 0) {
            wavelengths[i].id = long_id;
        }
        if (strcmp(offsets[i].id, replaced_id) == 0) {
            offsets[i].id = long_id;
        }
    }

    SidereonPppFixedConfig fixed_config;
    configure_ppp_fixed_config(
        &fixed_config, epochs, &float_config.corrections, wavelengths, offsets);
    if (sidereon_solve_ppp_fixed(sp3, float_solution, &fixed_config, &fixed_solution) !=
        SIDEREON_STATUS_OK) {
        rc = fail("sidereon_solve_ppp_fixed long ambiguity id", 1);
        goto cleanup;
    }

    written = 123;
    required = 123;
    if (sidereon_ppp_fixed_solution_used_ids(
            fixed_solution, used_ids, PPP_USED_SAT_COUNT, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != PPP_USED_SAT_COUNT || required != PPP_USED_SAT_COUNT ||
        !ppp_id_list_contains(used_ids, PPP_USED_SAT_COUNT, long_id)) {
        rc = fail("sidereon_ppp_fixed_solution_used_ids long id", 1);
        goto cleanup;
    }

    written = 123;
    required = 123;
    if (sidereon_ppp_fixed_solution_used_sat_ids(
            fixed_solution, narrow_used, PPP_USED_SAT_COUNT, &written, &required) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        written != 0) {
        rc = fail("sidereon_ppp_fixed_solution_used_sat_ids rejects long id", 1);
        goto cleanup;
    }

    rc = 0;

cleanup:
    sidereon_ppp_fixed_solution_free(fixed_solution);
    sidereon_ppp_float_solution_free(float_solution);
    return rc;
}

/* Load the committed Type-21 Eros kernel through the C SPK surface, query every
 * CSPICE reference epoch, and assert the recovered state matches CSPICE within
 * the engine's own near-ULP gates. Also exercises a malformed-buffer error path:
 * a bad load must return a status code (not OK) and must not crash or leak a
 * handle. Proves Type 21 end-to-end through the C ABI. */
static int exercise_spk_surface(const char *path) {
    int rc = 1;
    size_t spk_len = 0;
    uint8_t *spk_bytes = read_file(path, &spk_len);
    SidereonSpk *spk = NULL;

    if (spk_bytes == NULL) {
        fprintf(stderr, "FAIL: could not read SPK kernel file: %s\n", path);
        return 2;
    }

    /* Bad-buffer error path: garbage bytes are not a DAF/SPK kernel. The load
     * must report a non-OK status, leave out_spk NULL, and not crash. */
    uint8_t garbage[64];
    memset(garbage, 0xAB, sizeof(garbage));
    SidereonSpk *bad_spk = (SidereonSpk *)(uintptr_t)1;
    if (sidereon_spk_load(garbage, sizeof(garbage), &bad_spk) == SIDEREON_STATUS_OK ||
        bad_spk != NULL) {
        rc = fail("sidereon_spk_load rejects garbage and clears out_spk", 1);
        goto cleanup;
    }

    /* Null-pointer contracts: null data and null out-param. */
    SidereonSpk *null_data_spk = (SidereonSpk *)(uintptr_t)1;
    if (sidereon_spk_load(NULL, spk_len, &null_data_spk) != SIDEREON_STATUS_NULL_POINTER ||
        null_data_spk != NULL) {
        rc = fail("sidereon_spk_load null data clears out_spk", 1);
        goto cleanup;
    }
    if (sidereon_spk_load(spk_bytes, spk_len, NULL) != SIDEREON_STATUS_NULL_POINTER) {
        rc = fail("sidereon_spk_load null out_spk", 1);
        goto cleanup;
    }

    if (sidereon_spk_load(spk_bytes, spk_len, &spk) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_spk_load", 1);
        goto cleanup;
    }

    /* state out-param and handle null-pointer contracts. */
    SidereonSpkState probe;
    if (sidereon_spk_state(NULL, SPK_TARGET, SPK_CENTER, SPK_REFERENCE[0][0], &probe) !=
        SIDEREON_STATUS_NULL_POINTER) {
        rc = fail("sidereon_spk_state null spk", 1);
        goto cleanup;
    }
    if (sidereon_spk_state(spk, SPK_TARGET, SPK_CENTER, SPK_REFERENCE[0][0], NULL) !=
        SIDEREON_STATUS_NULL_POINTER) {
        rc = fail("sidereon_spk_state null out_state", 1);
        goto cleanup;
    }

    /* A non-finite epoch is rejected without crashing. */
    if (sidereon_spk_state(spk, SPK_TARGET, SPK_CENTER, NAN, &probe) == SIDEREON_STATUS_OK) {
        rc = fail("sidereon_spk_state rejects non-finite et", 1);
        goto cleanup;
    }

    double max_position_error = 0.0;
    double max_velocity_error = 0.0;
    for (size_t i = 0; i < SPK_REFERENCE_COUNT; i++) {
        const double *row = SPK_REFERENCE[i];
        double et = row[0];
        SidereonSpkState state;
        if (sidereon_spk_state(spk, SPK_TARGET, SPK_CENTER, et, &state) != SIDEREON_STATUS_OK) {
            rc = fail("sidereon_spk_state query", 1);
            goto cleanup;
        }
        if (state.target != SPK_TARGET || state.center != SPK_CENTER) {
            rc = fail("sidereon_spk_state echoes target/center", 1);
            goto cleanup;
        }
        if (!state.has_velocity_km_s) {
            rc = fail("sidereon_spk_state type-21 yields velocity", 1);
            goto cleanup;
        }
        for (int axis = 0; axis < 3; axis++) {
            double pos_err = fabs(state.position_km[axis] - row[1 + axis]);
            double vel_err = fabs(state.velocity_km_s[axis] - row[4 + axis]);
            if (pos_err > max_position_error) {
                max_position_error = pos_err;
            }
            if (vel_err > max_velocity_error) {
                max_velocity_error = vel_err;
            }
        }
    }

    if (max_position_error > SPK_POSITION_GATE_KM) {
        fprintf(stderr,
                "FAIL: SPK type-21 position drift %e km exceeds CSPICE parity gate %e\n",
                max_position_error, SPK_POSITION_GATE_KM);
        rc = 1;
        goto cleanup;
    }
    if (max_velocity_error > SPK_VELOCITY_GATE_KM_S) {
        fprintf(stderr,
                "FAIL: SPK type-21 velocity drift %e km/s exceeds CSPICE parity gate %e\n",
                max_velocity_error, SPK_VELOCITY_GATE_KM_S);
        rc = 1;
        goto cleanup;
    }

    printf("SPK type-21 (Eros->Sun) through C ABI: max |dpos| %e km, max |dvel| %e km/s "
           "(gates %e km, %e km/s) over %zu CSPICE epochs\n",
           max_position_error, max_velocity_error, SPK_POSITION_GATE_KM,
           SPK_VELOCITY_GATE_KM_S, (size_t)SPK_REFERENCE_COUNT);

    rc = 0;

cleanup:
    sidereon_spk_free(spk);
    free(spk_bytes);
    return rc;
}

static int exercise_ppp_surface(const char *path) {
    int rc = 1;
    size_t sp3_len = 0;
    uint8_t *sp3_bytes = read_file(path, &sp3_len);
    SidereonSp3 *sp3 = NULL;
    SidereonPppFloatSolution *float_solution = NULL;
    SidereonPppFixedSolution *fixed_solution = NULL;

    if (sp3_bytes == NULL) {
        fprintf(stderr, "FAIL: could not read PPP SP3 file: %s\n", path);
        return 2;
    }
    if (sidereon_sp3_load(sp3_bytes, sp3_len, &sp3) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_sp3_load PPP", 1);
        goto cleanup;
    }

    SidereonPppObservation observations[PPP_OBS_COUNT];
    SidereonPppEpoch epochs[PPP_EPOCH_COUNT];
    fill_ppp_epochs(observations, epochs);

    double clocks[PPP_EPOCH_COUNT];
    fill_ppp_initial_clocks(clocks);
    SidereonPppFloatMapEntry initial_ambiguities[PPP_INITIAL_AMBIGUITY_COUNT];
    fill_ppp_initial_ambiguities(initial_ambiguities);
    SidereonPppReceiverAntennaOptions ppp_receiver_antenna;

    SidereonPppFloatConfig float_config;
    configure_ppp_float_config(
        &float_config, epochs, clocks, initial_ambiguities, &ppp_receiver_antenna);

    SidereonPppFloatSolution *bad_float = (SidereonPppFloatSolution *)(uintptr_t)1;
    if (sidereon_solve_ppp_float(NULL, &float_config, &bad_float) !=
            SIDEREON_STATUS_NULL_POINTER ||
        bad_float != NULL) {
        rc = fail("sidereon_solve_ppp_float null sp3 clears out_solution", 1);
        goto cleanup;
    }
    if (sidereon_solve_ppp_float(sp3, NULL, &bad_float) != SIDEREON_STATUS_NULL_POINTER ||
        bad_float != NULL) {
        rc = fail("sidereon_solve_ppp_float null config clears out_solution", 1);
        goto cleanup;
    }

    SidereonPppFloatConfig unsupported_float_config = float_config;
    unsupported_float_config.corrections.phase_windup = true;
    bad_float = (SidereonPppFloatSolution *)(uintptr_t)1;
    if (sidereon_solve_ppp_float(sp3, &unsupported_float_config, &bad_float) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        bad_float != NULL) {
        rc = fail("sidereon_solve_ppp_float unsupported phase windup", 1);
        goto cleanup;
    }

    SidereonPppFloatMapEntry duplicate_initial_ambiguities[PPP_INITIAL_AMBIGUITY_COUNT];
    memcpy(duplicate_initial_ambiguities, initial_ambiguities,
           sizeof(initial_ambiguities));
    duplicate_initial_ambiguities[1].id = duplicate_initial_ambiguities[0].id;
    SidereonPppFloatConfig duplicate_float_config = float_config;
    duplicate_float_config.initial_state.ambiguities_m = duplicate_initial_ambiguities;
    bad_float = (SidereonPppFloatSolution *)(uintptr_t)1;
    if (sidereon_solve_ppp_float(sp3, &duplicate_float_config, &bad_float) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        bad_float != NULL) {
        rc = fail("sidereon_solve_ppp_float duplicate initial ambiguity id", 1);
        goto cleanup;
    }

    if (sidereon_solve_ppp_float(sp3, &float_config, &float_solution) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_solve_ppp_float", 1);
        goto cleanup;
    }
    rc = check_ppp_float_solution(float_solution);
    if (rc != 0) {
        goto cleanup;
    }

    /* B1: VMF1 troposphere mapping. The default config uses Niell (bit-exact
     * golden above); a VMF1 config with a valid site-wise a-coefficient series
     * bracketing the 2020-06-25 (MJD ~59025) arc must solve to a finite
     * position. The mapping differs from Niell, so this is not the same golden -
     * the point is that the new VMF surface crosses the FFI and is accepted. */
    SidereonPppFloatConfig vmf_config = float_config;
    vmf_config.tropo.mapping = SIDEREON_PPP_TROPO_MAPPING_VMF1;
    vmf_config.tropo.vmf_sample_count = 2;
    vmf_config.tropo.vmf_samples[0].mjd = 59025.0;
    vmf_config.tropo.vmf_samples[0].ah = 0.00123;
    vmf_config.tropo.vmf_samples[0].aw = 0.00055;
    vmf_config.tropo.vmf_samples[1].mjd = 59025.5;
    vmf_config.tropo.vmf_samples[1].ah = 0.00124;
    vmf_config.tropo.vmf_samples[1].aw = 0.00056;
    SidereonPppFloatSolution *vmf_solution = NULL;
    if (sidereon_solve_ppp_float(sp3, &vmf_config, &vmf_solution) != SIDEREON_STATUS_OK ||
        vmf_solution == NULL) {
        rc = fail("sidereon_solve_ppp_float VMF1 mapping", 1);
        goto cleanup;
    }
    double vmf_position[3];
    if (sidereon_ppp_float_solution_position(vmf_solution, vmf_position, 3) != SIDEREON_STATUS_OK ||
        !isfinite(vmf_position[0]) || !isfinite(vmf_position[1]) || !isfinite(vmf_position[2])) {
        sidereon_ppp_float_solution_free(vmf_solution);
        rc = fail("VMF1 float solution position finite", 1);
        goto cleanup;
    }
    sidereon_ppp_float_solution_free(vmf_solution);

    /* VMF1 with zero samples is rejected at the FFI boundary. */
    SidereonPppFloatConfig vmf_empty = vmf_config;
    vmf_empty.tropo.vmf_sample_count = 0;
    SidereonPppFloatSolution *vmf_bad = (SidereonPppFloatSolution *)(uintptr_t)1;
    if (sidereon_solve_ppp_float(sp3, &vmf_empty, &vmf_bad) != SIDEREON_STATUS_INVALID_ARGUMENT ||
        vmf_bad != NULL) {
        rc = fail("VMF1 zero samples must be rejected", 1);
        goto cleanup;
    }

    /* An invalid mapping selector is rejected. */
    SidereonPppFloatConfig vmf_badmap = vmf_config;
    vmf_badmap.tropo.mapping = 99;
    vmf_bad = (SidereonPppFloatSolution *)(uintptr_t)1;
    if (sidereon_solve_ppp_float(sp3, &vmf_badmap, &vmf_bad) != SIDEREON_STATUS_INVALID_ARGUMENT ||
        vmf_bad != NULL) {
        rc = fail("invalid tropo mapping must be rejected", 1);
        goto cleanup;
    }

    /* Non-ascending MJD samples are rejected by the engine validation, surfaced
     * through the FFI as INVALID_ARGUMENT. */
    SidereonPppFloatConfig vmf_unsorted = vmf_config;
    vmf_unsorted.tropo.vmf_samples[1].mjd = 59024.0; /* not strictly increasing */
    vmf_bad = (SidereonPppFloatSolution *)(uintptr_t)1;
    if (sidereon_solve_ppp_float(sp3, &vmf_unsorted, &vmf_bad) != SIDEREON_STATUS_INVALID_ARGUMENT ||
        vmf_bad != NULL) {
        rc = fail("VMF1 non-ascending samples must be rejected", 1);
        goto cleanup;
    }
    printf("PPP VMF1: solved finite, zero-samples/bad-mapping/unsorted rejected\n");

    SidereonPppFloatMapEntry wavelengths[PPP_FIXED_AMBIGUITY_COUNT];
    SidereonPppFloatMapEntry offsets[PPP_FIXED_AMBIGUITY_COUNT];
    fill_ppp_fixed_maps(wavelengths, offsets);

    SidereonPppFixedConfig fixed_config;
    configure_ppp_fixed_config(
        &fixed_config, epochs, &float_config.corrections, wavelengths, offsets);

    if (sidereon_solve_ppp_fixed(sp3, float_solution, &fixed_config, &fixed_solution) !=
        SIDEREON_STATUS_OK) {
        rc = fail("sidereon_solve_ppp_fixed", 1);
        goto cleanup;
    }
    rc = check_ppp_fixed_solution(fixed_solution);
    if (rc != 0) {
        goto cleanup;
    }
    rc = check_ppp_long_used_ids(sp3);
    if (rc != 0) {
        goto cleanup;
    }

    printf("PPP surface: float and fixed ESBC fixture cases OK\n");
    rc = 0;

cleanup:
    sidereon_ppp_fixed_solution_free(fixed_solution);
    sidereon_ppp_float_solution_free(float_solution);
    sidereon_sp3_free(sp3);
    free(sp3_bytes);
    return rc;
}

static int check_close(double got, double expected, double tolerance, const char *context) {
    if (fabs(got - expected) > tolerance) {
        return fail(context, 1);
    }
    return 0;
}

static int check_vec3_bits(const double values[3], const uint64_t expected[3],
                           const char *context) {
    for (size_t axis = 0; axis < 3; axis++) {
        if (f64_to_bits(values[axis]) != expected[axis]) {
            return fail(context, 1);
        }
    }
    return 0;
}

static int check_teme_states(const SidereonTemeState *states, size_t count) {
    if (count != PROP_EPOCH_COUNT) {
        return fail("propagation TEME state count", 1);
    }
    for (size_t i = 0; i < PROP_EPOCH_COUNT; i++) {
        int rc = check_vec3_bits(
            states[i].position_km, PROP_TEME_POSITION_BITS[i], "TEME position bits");
        if (rc != 0) {
            return rc;
        }
        rc = check_vec3_bits(
            states[i].velocity_km_s, PROP_TEME_VELOCITY_BITS[i], "TEME velocity bits");
        if (rc != 0) {
            return rc;
        }
    }
    return 0;
}

static int check_look_angles(const SidereonLookAngle *looks, size_t count) {
    if (count != PROP_EPOCH_COUNT) {
        return fail("look-angle count", 1);
    }
    for (size_t i = 0; i < PROP_EPOCH_COUNT; i++) {
        if (f64_to_bits(looks[i].azimuth_deg) != PROP_LOOK_AZIMUTH_BITS[i] ||
            f64_to_bits(looks[i].elevation_deg) != PROP_LOOK_ELEVATION_BITS[i] ||
            f64_to_bits(looks[i].range_km) != PROP_LOOK_RANGE_BITS[i]) {
            return fail("look-angle bits", 1);
        }
    }
    return 0;
}

static int check_numerical_ephemeris_states(const SidereonCartesianState *states, size_t count) {
    if (count != PROP_NUM_SAMPLE_COUNT) {
        return fail("numerical ephemeris state count", 1);
    }
    for (size_t i = 0; i < PROP_NUM_SAMPLE_COUNT; i++) {
        if (f64_to_bits(states[i].epoch_s) != PROP_NUM_TIME_BITS[i]) {
            return fail("numerical ephemeris epoch bits", 1);
        }
        int rc = check_vec3_bits(
            states[i].position_km, PROP_NUM_POSITION_BITS[i], "numerical position bits");
        if (rc != 0) {
            return rc;
        }
        rc = check_vec3_bits(
            states[i].velocity_km_s, PROP_NUM_VELOCITY_BITS[i], "numerical velocity bits");
        if (rc != 0) {
            return rc;
        }
    }
    return 0;
}

static int exercise_propagation_surface(void) {
    int rc = 1;
    SidereonTle *tle = NULL;
    SidereonTle *bad_checksum_tle = NULL;
    SidereonTlePropagation *propagation = NULL;
    SidereonTlePropagation *empty_propagation = NULL;
    SidereonLookAngles *look_angles = NULL;
    SidereonLookAngles *empty_look_angles = NULL;
    SidereonPassList *passes = NULL;
    SidereonTleBatchPropagation *batch_propagation = NULL;
    SidereonTleBatchPropagation *empty_batch_propagation = NULL;
    SidereonTleBatchPropagation *empty_epoch_batch_propagation = NULL;
    SidereonTleBatchLookAngles *batch_look_angles = NULL;
    SidereonTleBatchLookAngles *empty_batch_look_angles = NULL;
    SidereonTleBatchLookAngles *empty_epoch_batch_look_angles = NULL;
    SidereonEphemeris *ephemeris = NULL;
    SidereonEphemeris *empty_ephemeris = NULL;

    sidereon_tle_free(NULL);
    sidereon_tle_propagation_free(NULL);
    sidereon_look_angles_free(NULL);
    sidereon_pass_list_free(NULL);
    sidereon_tle_batch_propagation_free(NULL);
    sidereon_tle_batch_look_angles_free(NULL);
    sidereon_ephemeris_free(NULL);

    SidereonTle *bad_tle = (SidereonTle *)(uintptr_t)1;
    if (sidereon_tle_load(NULL, PROP_TLE_LINE2, PROP_TLE_OPSMODE, &bad_tle) !=
            SIDEREON_STATUS_NULL_POINTER ||
        bad_tle != NULL) {
        return fail("sidereon_tle_load null line clears out_tle", 1);
    }
    bad_tle = (SidereonTle *)(uintptr_t)1;
    if (sidereon_tle_load(PROP_TLE_LINE1, PROP_TLE_LINE2, 99, &bad_tle) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        bad_tle != NULL) {
        return fail("sidereon_tle_load invalid opsmode clears out_tle", 1);
    }
    bad_tle = (SidereonTle *)(uintptr_t)1;
    if (sidereon_tle_load("not a tle", "also not a tle", PROP_TLE_OPSMODE, &bad_tle) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        bad_tle != NULL) {
        return fail("sidereon_tle_load bad TLE clears out_tle", 1);
    }

    if (sidereon_tle_load(PROP_TLE_LINE1, PROP_TLE_LINE2, PROP_TLE_OPSMODE, &tle) !=
        SIDEREON_STATUS_OK) {
        return fail("sidereon_tle_load", 1);
    }

    SidereonTleLines null_lines = {
        .line1 = {.bytes = {42}},
        .line2 = {.bytes = {42}},
    };
    if (sidereon_tle_to_lines(NULL, &null_lines) != SIDEREON_STATUS_NULL_POINTER ||
        null_lines.line1.bytes[0] != 0 || null_lines.line2.bytes[0] != 0) {
        rc = fail("sidereon_tle_to_lines null TLE clears lines", 1);
        goto cleanup;
    }
    SidereonTleLines lines;
    if (sidereon_tle_to_lines(tle, &lines) != SIDEREON_STATUS_OK ||
        strcmp(lines.line1.bytes, PROP_TLE_ENCODED_LINE1) != 0 ||
        strcmp(lines.line2.bytes, PROP_TLE_ENCODED_LINE2) != 0) {
        rc = fail("sidereon_tle_to_lines exact lines", 1);
        goto cleanup;
    }

    SidereonTleMetadata metadata;
    if (sidereon_tle_metadata(tle, &metadata) != SIDEREON_STATUS_OK ||
        strcmp(metadata.catalog_number, PROP_TLE_CATALOG_NUMBER) != 0 ||
        strcmp(metadata.classification, PROP_TLE_CLASSIFICATION) != 0 ||
        strcmp(metadata.international_designator, PROP_TLE_INTERNATIONAL_DESIGNATOR) != 0 ||
        metadata.epoch_year != PROP_TLE_EPOCH_YEAR ||
        metadata.ephemeris_type != PROP_TLE_EPHEMERIS_TYPE ||
        metadata.elset_number != PROP_TLE_ELSET_NUMBER ||
        metadata.rev_number != PROP_TLE_REV_NUMBER) {
        rc = fail("sidereon_tle_metadata strings and integers", 1);
        goto cleanup;
    }
    if (check_close(
            metadata.epoch_day_of_year, PROP_TLE_EPOCH_DAY_OF_YEAR, 0.0,
            "sidereon_tle_metadata epoch day") != 0 ||
        check_close(
            metadata.inclination_deg, PROP_TLE_INCLINATION_DEG, 0.0,
            "sidereon_tle_metadata inclination") != 0 ||
        check_close(metadata.raan_deg, PROP_TLE_RAAN_DEG, 0.0, "sidereon_tle_metadata raan") !=
            0 ||
        check_close(
            metadata.eccentricity, PROP_TLE_ECCENTRICITY, 0.0,
            "sidereon_tle_metadata eccentricity") != 0 ||
        check_close(
            metadata.arg_perigee_deg, PROP_TLE_ARG_PERIGEE_DEG, 0.0,
            "sidereon_tle_metadata argument of perigee") != 0 ||
        check_close(
            metadata.mean_anomaly_deg, PROP_TLE_MEAN_ANOMALY_DEG, 0.0,
            "sidereon_tle_metadata mean anomaly") != 0 ||
        check_close(
            metadata.mean_motion_rev_per_day, PROP_TLE_MEAN_MOTION_REV_PER_DAY, 0.0,
            "sidereon_tle_metadata mean motion") != 0 ||
        check_close(
            metadata.mean_motion_dot, PROP_TLE_MEAN_MOTION_DOT, 0.0,
            "sidereon_tle_metadata mean motion dot") != 0 ||
        check_close(
            metadata.mean_motion_double_dot, PROP_TLE_MEAN_MOTION_DOUBLE_DOT, 0.0,
            "sidereon_tle_metadata mean motion double dot") != 0 ||
        check_close(metadata.bstar, PROP_TLE_BSTAR, 0.0, "sidereon_tle_metadata bstar") != 0) {
        rc = 1;
        goto cleanup;
    }

    size_t written = 123;
    size_t required = 123;
    if (sidereon_tle_checksum_warnings(tle, NULL, 0, &written, &required) != SIDEREON_STATUS_OK ||
        written != 0 || required != 0) {
        rc = fail("sidereon_tle_checksum_warnings clean size query", 1);
        goto cleanup;
    }
    if (sidereon_tle_load(
            PROP_TLE_BAD_CHECKSUM_LINE1, PROP_TLE_BAD_CHECKSUM_LINE2, PROP_TLE_OPSMODE,
            &bad_checksum_tle) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_tle_load checksum warning case", 1);
        goto cleanup;
    }
    if (sidereon_tle_checksum_warnings(
            bad_checksum_tle, NULL, 0, &written, &required) != SIDEREON_STATUS_OK ||
        written != 0 || required != PROP_TLE_CHECKSUM_WARNING_COUNT) {
        rc = fail("sidereon_tle_checksum_warnings warning size query", 1);
        goto cleanup;
    }
    SidereonTleChecksumWarning warning;
    if (sidereon_tle_checksum_warnings(
            bad_checksum_tle, &warning, 1, &written, &required) != SIDEREON_STATUS_OK ||
        written != PROP_TLE_CHECKSUM_WARNING_COUNT ||
        required != PROP_TLE_CHECKSUM_WARNING_COUNT ||
        warning.line_number != PROP_TLE_CHECKSUM_WARNING_LINE_NUMBER ||
        warning.expected != PROP_TLE_CHECKSUM_WARNING_EXPECTED ||
        warning.computed != PROP_TLE_CHECKSUM_WARNING_COMPUTED) {
        rc = fail("sidereon_tle_checksum_warnings warning values", 1);
        goto cleanup;
    }

    SidereonTlePropagation *bad_propagation = (SidereonTlePropagation *)(uintptr_t)1;
    if (sidereon_tle_propagate(tle, NULL, PROP_EPOCH_COUNT, &bad_propagation) !=
            SIDEREON_STATUS_NULL_POINTER ||
        bad_propagation != NULL) {
        rc = fail("sidereon_tle_propagate null epochs clears out_propagation", 1);
        goto cleanup;
    }
    if (sidereon_tle_propagate(tle, NULL, 0, &empty_propagation) != SIDEREON_STATUS_OK ||
        empty_propagation == NULL) {
        rc = fail("sidereon_tle_propagate empty epochs returns empty propagation", 1);
        goto cleanup;
    }
    size_t empty_epoch_count = 123;
    if (sidereon_tle_propagation_epoch_count(empty_propagation, &empty_epoch_count) !=
            SIDEREON_STATUS_OK ||
        empty_epoch_count != 0) {
        rc = fail("sidereon_tle_propagation_epoch_count empty propagation", 1);
        goto cleanup;
    }
    if (sidereon_tle_propagation_states(empty_propagation, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != 0 || required != 0) {
        rc = fail("sidereon_tle_propagation_states empty size query", 1);
        goto cleanup;
    }
    if (sidereon_tle_propagate(tle, PROP_EPOCHS_UNIX_US, PROP_EPOCH_COUNT, &propagation) !=
        SIDEREON_STATUS_OK) {
        rc = fail("sidereon_tle_propagate", 1);
        goto cleanup;
    }
    size_t epoch_count = 123;
    if (sidereon_tle_propagation_epoch_count(NULL, &epoch_count) != SIDEREON_STATUS_NULL_POINTER ||
        epoch_count != 0) {
        rc = fail("sidereon_tle_propagation_epoch_count null propagation clears count", 1);
        goto cleanup;
    }
    if (sidereon_tle_propagation_epoch_count(propagation, &epoch_count) != SIDEREON_STATUS_OK ||
        epoch_count != PROP_EPOCH_COUNT) {
        rc = fail("sidereon_tle_propagation_epoch_count", 1);
        goto cleanup;
    }
    if (sidereon_tle_propagation_states(propagation, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != 0 || required != PROP_EPOCH_COUNT) {
        rc = fail("sidereon_tle_propagation_states size query", 1);
        goto cleanup;
    }
    SidereonTemeState short_states[1];
    short_states[0].position_km[0] = 42.0;
    if (sidereon_tle_propagation_states(propagation, short_states, 1, &written, &required) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        written != 0 || required != PROP_EPOCH_COUNT || short_states[0].position_km[0] != 42.0) {
        rc = fail("sidereon_tle_propagation_states short buffer", 1);
        goto cleanup;
    }
    SidereonTemeState states[PROP_EPOCH_COUNT];
    if (sidereon_tle_propagation_states(
            propagation, states, PROP_EPOCH_COUNT, &written, &required) != SIDEREON_STATUS_OK ||
        written != PROP_EPOCH_COUNT || required != PROP_EPOCH_COUNT) {
        rc = fail("sidereon_tle_propagation_states full copy", 1);
        goto cleanup;
    }
    rc = check_teme_states(states, written);
    if (rc != 0) {
        goto cleanup;
    }

    SidereonGroundStation station = {
        .latitude_deg = PROP_STATION_LATITUDE_DEG,
        .longitude_deg = PROP_STATION_LONGITUDE_DEG,
        .altitude_m = PROP_STATION_ALTITUDE_M,
    };
    if (sidereon_tle_look_angles(tle, &station, NULL, 0, &empty_look_angles) !=
            SIDEREON_STATUS_OK ||
        empty_look_angles == NULL) {
        rc = fail("sidereon_tle_look_angles empty epochs returns empty arc", 1);
        goto cleanup;
    }
    empty_epoch_count = 123;
    if (sidereon_look_angles_epoch_count(empty_look_angles, &empty_epoch_count) !=
            SIDEREON_STATUS_OK ||
        empty_epoch_count != 0) {
        rc = fail("sidereon_look_angles_epoch_count empty arc", 1);
        goto cleanup;
    }
    if (sidereon_look_angles_values(empty_look_angles, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != 0 || required != 0) {
        rc = fail("sidereon_look_angles_values empty size query", 1);
        goto cleanup;
    }
    if (sidereon_tle_look_angles(tle, &station, PROP_EPOCHS_UNIX_US, PROP_EPOCH_COUNT,
                                 &look_angles) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_tle_look_angles", 1);
        goto cleanup;
    }
    if (sidereon_look_angles_epoch_count(look_angles, &epoch_count) != SIDEREON_STATUS_OK ||
        epoch_count != PROP_EPOCH_COUNT) {
        rc = fail("sidereon_look_angles_epoch_count", 1);
        goto cleanup;
    }
    SidereonLookAngle looks[PROP_EPOCH_COUNT];
    if (sidereon_look_angles_values(look_angles, looks, PROP_EPOCH_COUNT, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != PROP_EPOCH_COUNT || required != PROP_EPOCH_COUNT) {
        rc = fail("sidereon_look_angles_values full copy", 1);
        goto cleanup;
    }
    rc = check_look_angles(looks, written);
    if (rc != 0) {
        goto cleanup;
    }

    SidereonPassFinderOptions pass_options;
    if (sidereon_pass_finder_options_init(NULL) != SIDEREON_STATUS_NULL_POINTER ||
        sidereon_pass_finder_options_init(&pass_options) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_pass_finder_options_init", 1);
        goto cleanup;
    }
    pass_options.elevation_mask_deg = PROP_PASS_ELEVATION_MASK_DEG;
    pass_options.step_seconds = PROP_PASS_STEP_SECONDS;
    pass_options.time_tolerance_s = PROP_PASS_TIME_TOLERANCE_S;
    SidereonPassList *bad_passes = (SidereonPassList *)(uintptr_t)1;
    if (sidereon_tle_find_passes(
            tle, &station, PROP_PASS_START_UNIX_US, PROP_PASS_START_UNIX_US, &pass_options,
            &bad_passes) != SIDEREON_STATUS_INVALID_ARGUMENT ||
        bad_passes != NULL) {
        rc = fail("sidereon_tle_find_passes invalid window clears out_passes", 1);
        goto cleanup;
    }
    if (sidereon_tle_find_passes(
            tle, &station, PROP_PASS_START_UNIX_US, PROP_PASS_END_UNIX_US, &pass_options,
            &passes) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_tle_find_passes", 1);
        goto cleanup;
    }
    size_t pass_count = 123;
    if (sidereon_pass_list_count(passes, &pass_count) != SIDEREON_STATUS_OK ||
        pass_count != PROP_PASS_COUNT) {
        rc = fail("sidereon_pass_list_count", 1);
        goto cleanup;
    }
    SidereonSatellitePass pass_values[PROP_PASS_COUNT];
    if (sidereon_pass_list_values(passes, pass_values, PROP_PASS_COUNT, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != PROP_PASS_COUNT || required != PROP_PASS_COUNT) {
        rc = fail("sidereon_pass_list_values full copy", 1);
        goto cleanup;
    }
    for (size_t i = 0; i < PROP_PASS_COUNT; i++) {
        if (pass_values[i].aos_unix_us != PROP_PASS_AOS_UNIX_US[i] ||
            pass_values[i].los_unix_us != PROP_PASS_LOS_UNIX_US[i] ||
            pass_values[i].culmination_unix_us != PROP_PASS_CULMINATION_UNIX_US[i] ||
            fabs(pass_values[i].max_elevation_deg -
                 bits_to_f64(PROP_PASS_MAX_ELEVATION_BITS[i])) >= 1.0e-9 ||
            pass_values[i].duration_s <= 0.0 ||
            pass_values[i].aos_unix_us > pass_values[i].culmination_unix_us ||
            pass_values[i].culmination_unix_us > pass_values[i].los_unix_us) {
            rc = fail("sidereon_pass_list_values reference pass", 1);
            goto cleanup;
        }
    }

    SidereonTlePair tle_pair = {
        .line1 = PROP_TLE_LINE1,
        .line2 = PROP_TLE_LINE2,
    };
    if (sidereon_propagate_tle_batch(
            NULL, 0, PROP_EPOCHS_UNIX_US, PROP_EPOCH_COUNT, PROP_TLE_OPSMODE, true,
            &empty_batch_propagation) != SIDEREON_STATUS_OK ||
        empty_batch_propagation == NULL) {
        rc = fail("sidereon_propagate_tle_batch empty fleet returns empty batch", 1);
        goto cleanup;
    }
    size_t sat_count = 123;
    size_t batch_epoch_count = 123;
    if (sidereon_tle_batch_propagation_shape(
            empty_batch_propagation, &sat_count, &batch_epoch_count) != SIDEREON_STATUS_OK ||
        sat_count != 0 || batch_epoch_count != PROP_EPOCH_COUNT) {
        rc = fail("sidereon_tle_batch_propagation_shape empty fleet", 1);
        goto cleanup;
    }
    if (sidereon_tle_batch_propagation_states(
            empty_batch_propagation, NULL, 0, &written, &required) != SIDEREON_STATUS_OK ||
        written != 0 || required != 0) {
        rc = fail("sidereon_tle_batch_propagation_states empty fleet size query", 1);
        goto cleanup;
    }
    if (sidereon_propagate_tle_batch(
            &tle_pair, 1, NULL, 0, PROP_TLE_OPSMODE, false, &empty_epoch_batch_propagation) !=
            SIDEREON_STATUS_OK ||
        empty_epoch_batch_propagation == NULL) {
        rc = fail("sidereon_propagate_tle_batch empty epochs returns empty batch", 1);
        goto cleanup;
    }
    sat_count = 123;
    batch_epoch_count = 123;
    if (sidereon_tle_batch_propagation_shape(
            empty_epoch_batch_propagation, &sat_count, &batch_epoch_count) != SIDEREON_STATUS_OK ||
        sat_count != 1 || batch_epoch_count != 0) {
        rc = fail("sidereon_tle_batch_propagation_shape empty epochs", 1);
        goto cleanup;
    }
    if (sidereon_tle_batch_propagation_states(
            empty_epoch_batch_propagation, NULL, 0, &written, &required) != SIDEREON_STATUS_OK ||
        written != 0 || required != 0) {
        rc = fail("sidereon_tle_batch_propagation_states empty epochs size query", 1);
        goto cleanup;
    }
    if (sidereon_propagate_tle_batch(&tle_pair, 1, PROP_EPOCHS_UNIX_US, PROP_EPOCH_COUNT,
                                     PROP_TLE_OPSMODE, true, &batch_propagation) !=
        SIDEREON_STATUS_OK) {
        rc = fail("sidereon_propagate_tle_batch", 1);
        goto cleanup;
    }
    sat_count = 123;
    batch_epoch_count = 123;
    if (sidereon_tle_batch_propagation_shape(
            NULL, &sat_count, &batch_epoch_count) != SIDEREON_STATUS_NULL_POINTER ||
        sat_count != 0 || batch_epoch_count != 0) {
        rc = fail("sidereon_tle_batch_propagation_shape null batch clears shape", 1);
        goto cleanup;
    }
    if (sidereon_tle_batch_propagation_shape(
            batch_propagation, &sat_count, &batch_epoch_count) != SIDEREON_STATUS_OK ||
        sat_count != 1 || batch_epoch_count != PROP_EPOCH_COUNT) {
        rc = fail("sidereon_tle_batch_propagation_shape", 1);
        goto cleanup;
    }
    if (sidereon_tle_batch_propagation_states(batch_propagation, NULL, 0, &written,
                                              &required) != SIDEREON_STATUS_OK ||
        written != 0 || required != PROP_EPOCH_COUNT) {
        rc = fail("sidereon_tle_batch_propagation_states size query", 1);
        goto cleanup;
    }
    SidereonTemeState short_batch_states[1];
    short_batch_states[0].position_km[0] = 42.0;
    if (sidereon_tle_batch_propagation_states(
            batch_propagation, short_batch_states, 1, &written, &required) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        written != 0 || required != PROP_EPOCH_COUNT ||
        short_batch_states[0].position_km[0] != 42.0) {
        rc = fail("sidereon_tle_batch_propagation_states short buffer", 1);
        goto cleanup;
    }
    SidereonTemeState batch_states[PROP_EPOCH_COUNT];
    if (sidereon_tle_batch_propagation_states(
            batch_propagation, batch_states, PROP_EPOCH_COUNT, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != PROP_EPOCH_COUNT || required != PROP_EPOCH_COUNT) {
        rc = fail("sidereon_tle_batch_propagation_states full copy", 1);
        goto cleanup;
    }
    rc = check_teme_states(batch_states, written);
    if (rc != 0) {
        goto cleanup;
    }

    if (sidereon_tle_batch_look_angles(
            NULL, 0, &station, PROP_EPOCHS_UNIX_US, PROP_EPOCH_COUNT, PROP_TLE_OPSMODE, true,
            &empty_batch_look_angles) != SIDEREON_STATUS_OK ||
        empty_batch_look_angles == NULL) {
        rc = fail("sidereon_tle_batch_look_angles empty fleet returns empty batch", 1);
        goto cleanup;
    }
    sat_count = 123;
    batch_epoch_count = 123;
    if (sidereon_tle_batch_look_angles_shape(
            empty_batch_look_angles, &sat_count, &batch_epoch_count) != SIDEREON_STATUS_OK ||
        sat_count != 0 || batch_epoch_count != PROP_EPOCH_COUNT) {
        rc = fail("sidereon_tle_batch_look_angles_shape empty fleet", 1);
        goto cleanup;
    }
    if (sidereon_tle_batch_look_angles_values(
            empty_batch_look_angles, NULL, 0, &written, &required) != SIDEREON_STATUS_OK ||
        written != 0 || required != 0) {
        rc = fail("sidereon_tle_batch_look_angles_values empty fleet size query", 1);
        goto cleanup;
    }
    if (sidereon_tle_batch_look_angles(
            &tle_pair, 1, &station, NULL, 0, PROP_TLE_OPSMODE, false,
            &empty_epoch_batch_look_angles) != SIDEREON_STATUS_OK ||
        empty_epoch_batch_look_angles == NULL) {
        rc = fail("sidereon_tle_batch_look_angles empty epochs returns empty batch", 1);
        goto cleanup;
    }
    sat_count = 123;
    batch_epoch_count = 123;
    if (sidereon_tle_batch_look_angles_shape(
            empty_epoch_batch_look_angles, &sat_count, &batch_epoch_count) != SIDEREON_STATUS_OK ||
        sat_count != 1 || batch_epoch_count != 0) {
        rc = fail("sidereon_tle_batch_look_angles_shape empty epochs", 1);
        goto cleanup;
    }
    if (sidereon_tle_batch_look_angles_values(
            empty_epoch_batch_look_angles, NULL, 0, &written, &required) != SIDEREON_STATUS_OK ||
        written != 0 || required != 0) {
        rc = fail("sidereon_tle_batch_look_angles_values empty epochs size query", 1);
        goto cleanup;
    }
    if (sidereon_tle_batch_look_angles(
            &tle_pair, 1, &station, PROP_EPOCHS_UNIX_US, PROP_EPOCH_COUNT, PROP_TLE_OPSMODE,
            false, &batch_look_angles) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_tle_batch_look_angles", 1);
        goto cleanup;
    }
    if (sidereon_tle_batch_look_angles_shape(
            batch_look_angles, &sat_count, &batch_epoch_count) != SIDEREON_STATUS_OK ||
        sat_count != 1 || batch_epoch_count != PROP_EPOCH_COUNT) {
        rc = fail("sidereon_tle_batch_look_angles_shape", 1);
        goto cleanup;
    }
    if (sidereon_tle_batch_look_angles_values(batch_look_angles, NULL, 0, &written,
                                              &required) != SIDEREON_STATUS_OK ||
        written != 0 || required != PROP_EPOCH_COUNT) {
        rc = fail("sidereon_tle_batch_look_angles_values size query", 1);
        goto cleanup;
    }
    SidereonLookAngle short_batch_looks[1];
    short_batch_looks[0].azimuth_deg = 42.0;
    if (sidereon_tle_batch_look_angles_values(
            batch_look_angles, short_batch_looks, 1, &written, &required) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        written != 0 || required != PROP_EPOCH_COUNT || short_batch_looks[0].azimuth_deg != 42.0) {
        rc = fail("sidereon_tle_batch_look_angles_values short buffer", 1);
        goto cleanup;
    }
    SidereonLookAngle batch_looks[PROP_EPOCH_COUNT];
    if (sidereon_tle_batch_look_angles_values(
            batch_look_angles, batch_looks, PROP_EPOCH_COUNT, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != PROP_EPOCH_COUNT || required != PROP_EPOCH_COUNT) {
        rc = fail("sidereon_tle_batch_look_angles_values full copy", 1);
        goto cleanup;
    }
    rc = check_look_angles(batch_looks, written);
    if (rc != 0) {
        goto cleanup;
    }

    SidereonStatePropagationConfig config;
    if (sidereon_state_propagation_config_init(NULL) != SIDEREON_STATUS_NULL_POINTER ||
        sidereon_state_propagation_config_init(&config) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_state_propagation_config_init", 1);
        goto cleanup;
    }
    config.epoch_s = bits_to_f64(PROP_NUM_EPOCH_S_BITS);
    for (size_t axis = 0; axis < 3; axis++) {
        config.position_km[axis] = bits_to_f64(PROP_NUM_INITIAL_POSITION_BITS[axis]);
        config.velocity_km_s[axis] = bits_to_f64(PROP_NUM_INITIAL_VELOCITY_BITS[axis]);
    }
    config.force_model = PROP_NUM_FORCE_MODEL;
    config.integrator = PROP_NUM_INTEGRATOR;
    config.abs_tol = PROP_NUM_ABS_TOL;
    config.rel_tol = PROP_NUM_REL_TOL;
    config.initial_step_s = PROP_NUM_INITIAL_STEP_S;
    config.min_step_s = PROP_NUM_MIN_STEP_S;
    config.max_step_s = PROP_NUM_MAX_STEP_S;
    config.max_steps = PROP_NUM_MAX_STEPS;

    double times[PROP_NUM_SAMPLE_COUNT];
    for (size_t i = 0; i < PROP_NUM_SAMPLE_COUNT; i++) {
        times[i] = bits_to_f64(PROP_NUM_TIME_BITS[i]);
    }
    SidereonEphemeris *bad_ephemeris = (SidereonEphemeris *)(uintptr_t)1;
    if (sidereon_propagate_state(NULL, times, PROP_NUM_SAMPLE_COUNT, &bad_ephemeris) !=
            SIDEREON_STATUS_NULL_POINTER ||
        bad_ephemeris != NULL) {
        rc = fail("sidereon_propagate_state null config clears out_ephemeris", 1);
        goto cleanup;
    }
    bad_ephemeris = (SidereonEphemeris *)(uintptr_t)1;
    if (sidereon_propagate_state(&config, NULL, PROP_NUM_SAMPLE_COUNT, &bad_ephemeris) !=
            SIDEREON_STATUS_NULL_POINTER ||
        bad_ephemeris != NULL) {
        rc = fail("sidereon_propagate_state null times clears out_ephemeris", 1);
        goto cleanup;
    }
    if (sidereon_propagate_state(&config, NULL, 0, &empty_ephemeris) != SIDEREON_STATUS_OK ||
        empty_ephemeris == NULL) {
        rc = fail("sidereon_propagate_state empty times returns empty ephemeris", 1);
        goto cleanup;
    }
    empty_epoch_count = 123;
    if (sidereon_ephemeris_epoch_count(empty_ephemeris, &empty_epoch_count) !=
            SIDEREON_STATUS_OK ||
        empty_epoch_count != 0) {
        rc = fail("sidereon_ephemeris_epoch_count empty ephemeris", 1);
        goto cleanup;
    }
    if (sidereon_ephemeris_times_s(empty_ephemeris, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != 0 || required != 0) {
        rc = fail("sidereon_ephemeris_times_s empty size query", 1);
        goto cleanup;
    }
    if (sidereon_ephemeris_states(empty_ephemeris, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != 0 || required != 0) {
        rc = fail("sidereon_ephemeris_states empty size query", 1);
        goto cleanup;
    }
    if (sidereon_propagate_state(&config, times, PROP_NUM_SAMPLE_COUNT, &ephemeris) !=
        SIDEREON_STATUS_OK) {
        rc = fail("sidereon_propagate_state", 1);
        goto cleanup;
    }
    if (sidereon_ephemeris_epoch_count(ephemeris, &epoch_count) != SIDEREON_STATUS_OK ||
        epoch_count != PROP_NUM_SAMPLE_COUNT) {
        rc = fail("sidereon_ephemeris_epoch_count", 1);
        goto cleanup;
    }
    double copied_times[PROP_NUM_SAMPLE_COUNT];
    if (sidereon_ephemeris_times_s(
            ephemeris, NULL, 0, &written, &required) != SIDEREON_STATUS_OK ||
        written != 0 || required != PROP_NUM_SAMPLE_COUNT) {
        rc = fail("sidereon_ephemeris_times_s size query", 1);
        goto cleanup;
    }
    if (sidereon_ephemeris_times_s(
            ephemeris, copied_times, PROP_NUM_SAMPLE_COUNT, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != PROP_NUM_SAMPLE_COUNT || required != PROP_NUM_SAMPLE_COUNT) {
        rc = fail("sidereon_ephemeris_times_s full copy", 1);
        goto cleanup;
    }
    for (size_t i = 0; i < PROP_NUM_SAMPLE_COUNT; i++) {
        if (f64_to_bits(copied_times[i]) != PROP_NUM_TIME_BITS[i]) {
            rc = fail("sidereon_ephemeris_times_s bits", 1);
            goto cleanup;
        }
    }
    SidereonCartesianState numerical_states[PROP_NUM_SAMPLE_COUNT];
    if (sidereon_ephemeris_states(
            ephemeris, numerical_states, PROP_NUM_SAMPLE_COUNT, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != PROP_NUM_SAMPLE_COUNT || required != PROP_NUM_SAMPLE_COUNT) {
        rc = fail("sidereon_ephemeris_states full copy", 1);
        goto cleanup;
    }
    rc = check_numerical_ephemeris_states(numerical_states, written);
    if (rc != 0) {
        goto cleanup;
    }

    printf("Propagation surface: TLE, SGP4, passes, batch, and state propagation OK\n");
    rc = 0;

cleanup:
    sidereon_ephemeris_free(empty_ephemeris);
    sidereon_ephemeris_free(ephemeris);
    sidereon_tle_batch_look_angles_free(empty_epoch_batch_look_angles);
    sidereon_tle_batch_look_angles_free(empty_batch_look_angles);
    sidereon_tle_batch_look_angles_free(batch_look_angles);
    sidereon_tle_batch_propagation_free(empty_epoch_batch_propagation);
    sidereon_tle_batch_propagation_free(empty_batch_propagation);
    sidereon_tle_batch_propagation_free(batch_propagation);
    sidereon_pass_list_free(passes);
    sidereon_look_angles_free(empty_look_angles);
    sidereon_look_angles_free(look_angles);
    sidereon_tle_propagation_free(empty_propagation);
    sidereon_tle_propagation_free(propagation);
    sidereon_tle_free(bad_checksum_tle);
    sidereon_tle_free(tle);
    return rc;
}

/* GLONASS first-class SPP through the C ABI.
 *
 * GLONASS is FDMA: each satellite's L1 carrier is resolved from the V2
 * SidereonGlonassChannel array so the L1 Klobuchar ionosphere delay scales by
 * (f_L1/f_k)^2. These cases prove (end-to-end) GLONASS observations solve and
 * recover a known position, (a) ionosphere-on with no channel is rejected with
 * the engine's IonosphereUnsupported error, (b) supplying the channels lifts that
 * gate and GLONASS still solves with the ionosphere on, (c) an out-of-range
 * channel is rejected like a missing one, and (d) a populated channel map is a
 * bit-for-bit no-op on a GPS-only solve.
 *
 * Pseudoranges are synthesized from the committed multi-GNSS SP3 product itself:
 * the geometric range from the receiver truth to each satellite (sampled through
 * the engine's own SP3 interpolation, so positions match what the solver will
 * reproduce) minus that satellite's broadcast clock term. No golden is
 * fabricated; the recovered position is checked against the synthesis truth.
 * Neglected light-time / Sagnac terms leave a few-hundred-metre residual, well
 * inside the bound asserted below. */

#define GLO_MAX_SATS 32
static const double LIGHT_M_S = 299792458.0;
static const double SCENARIO_PI = 3.14159265358979323846;

/* WGS84 geodetic -> ECEF, metres. */
static void geodetic_to_ecef(double lat_deg, double lon_deg, double h_m, double out[3]) {
    const double a = 6378137.0;
    const double f = 1.0 / 298.257223563;
    const double e2 = f * (2.0 - f);
    double lat = lat_deg * SCENARIO_PI / 180.0;
    double lon = lon_deg * SCENARIO_PI / 180.0;
    double slat = sin(lat);
    double clat = cos(lat);
    double n = a / sqrt(1.0 - e2 * slat * slat);
    out[0] = (n + h_m) * clat * cos(lon);
    out[1] = (n + h_m) * clat * sin(lon);
    out[2] = (n * (1.0 - e2) + h_m) * slat;
}

/* Synthesize a self-consistent GLONASS observation set at one mid-day SP3 epoch
 * for a receiver at Moscow (a GLONASS-favourable latitude), keeping satellites
 * above a 10-degree elevation mask. tokens[] backs the observation sat_id
 * pointers and must outlive the solve. Returns the observation count, or -1 on
 * an unexpected ABI error. */
static int build_glonass_scenario(const SidereonSp3 *sp3, double rx_out[3],
                                  SidereonSatelliteToken tokens[GLO_MAX_SATS],
                                  SidereonObservation observations[GLO_MAX_SATS],
                                  SidereonGlonassChannel channels[GLO_MAX_SATS],
                                  double *t_rx_out) {
    size_t epoch_written = 0;
    size_t epoch_required = 0;
    if (sidereon_sp3_epochs_j2000_seconds(sp3, NULL, 0, &epoch_written, &epoch_required) !=
            SIDEREON_STATUS_OK ||
        epoch_required < 49) {
        return -1;
    }
    double *epochs = (double *)malloc(epoch_required * sizeof(double));
    if (epochs == NULL) {
        return -1;
    }
    size_t w = 0;
    size_t r = 0;
    if (sidereon_sp3_epochs_j2000_seconds(sp3, epochs, epoch_required, &w, &r) !=
        SIDEREON_STATUS_OK) {
        free(epochs);
        return -1;
    }
    double t_rx = epochs[48];
    free(epochs);

    double rx[3];
    geodetic_to_ecef(55.75, 37.62, 200.0, rx);
    double rx_radius = sqrt(rx[0] * rx[0] + rx[1] * rx[1] + rx[2] * rx[2]);

    size_t sat_written = 0;
    size_t sat_required = 0;
    if (sidereon_sp3_satellites(sp3, NULL, 0, &sat_written, &sat_required) != SIDEREON_STATUS_OK) {
        return -1;
    }
    SidereonSatelliteToken *all =
        (SidereonSatelliteToken *)malloc(sat_required * sizeof(SidereonSatelliteToken));
    if (all == NULL) {
        return -1;
    }
    if (sidereon_sp3_satellites(sp3, all, sat_required, &sat_written, &sat_required) !=
        SIDEREON_STATUS_OK) {
        free(all);
        return -1;
    }

    int count = 0;
    for (size_t i = 0; i < sat_written && count < GLO_MAX_SATS; i++) {
        if (all[i].bytes[0] != 'R') {
            continue;
        }
        double pos[3];
        double clk = 0.0;
        size_t interp_written = 0;
        if (sidereon_sp3_interpolate(sp3, all[i].bytes, &t_rx, 1, pos, 3, &clk, 1,
                                     &interp_written) != SIDEREON_STATUS_OK) {
            continue;
        }
        if (!isfinite(pos[0]) || !isfinite(pos[1]) || !isfinite(pos[2]) || !isfinite(clk)) {
            continue;
        }
        const double los[3] = {pos[0] - rx[0], pos[1] - rx[1], pos[2] - rx[2]};
        double range = sqrt(los[0] * los[0] + los[1] * los[1] + los[2] * los[2]);
        double up_dot = (los[0] * rx[0] + los[1] * rx[1] + los[2] * rx[2]) / (range * rx_radius);
        if (up_dot > 1.0) {
            up_dot = 1.0;
        } else if (up_dot < -1.0) {
            up_dot = -1.0;
        }
        double el_deg = asin(up_dot) * 180.0 / SCENARIO_PI;
        if (el_deg < 10.0) {
            continue;
        }
        tokens[count] = all[i];
        observations[count].sat_id = tokens[count].bytes;
        observations[count].pseudorange_m = range - LIGHT_M_S * clk;
        channels[count].slot = (uint8_t)atoi(all[i].bytes + 1);
        channels[count].channel = 0; /* a valid FDMA channel for each slot */
        count++;
    }
    free(all);

    rx_out[0] = rx[0];
    rx_out[1] = rx[1];
    rx_out[2] = rx[2];
    *t_rx_out = t_rx;
    return count;
}

/* True iff the binding's last error is the engine's IonosphereUnsupported message
 * naming a GLONASS satellite's carrier. */
static int last_error_is_glonass_carrier_gate(void) {
    size_t needed = sidereon_last_error_message(NULL, 0);
    char *msg = (char *)malloc(needed + 1);
    if (msg == NULL) {
        return 0;
    }
    sidereon_last_error_message(msg, needed + 1);
    int ok = strstr(msg, "modeled carrier frequency") != NULL && strstr(msg, "for R") != NULL;
    free(msg);
    return ok;
}

/* Assert a converged GLONASS-only solve: at least four GLONASS satellites used
 * and the recovered position within 2 km of the synthesis truth. */
static int check_glonass_solution(const SidereonSppSolution *sol, const double rx[3],
                                  const char *label) {
    size_t used = 0;
    if (sidereon_spp_solution_used_sat_count(sol, &used) != SIDEREON_STATUS_OK || used < 4 ||
        used > GLO_MAX_SATS) {
        return fail("GLONASS solve used too few/many satellites", 1);
    }
    SidereonSatelliteToken used_tokens[GLO_MAX_SATS];
    size_t written = 0;
    size_t required = 0;
    if (sidereon_spp_solution_used_sat_ids(sol, used_tokens, used, &written, &required) !=
        SIDEREON_STATUS_OK) {
        return fail("GLONASS used-sat ids", 1);
    }
    for (size_t i = 0; i < written; i++) {
        if (used_tokens[i].bytes[0] != 'R') {
            return fail("non-GLONASS satellite used in GLONASS-only solve", 1);
        }
    }
    double pos[3];
    if (sidereon_spp_solution_position(sol, pos, 3) != SIDEREON_STATUS_OK) {
        return fail("GLONASS solve position", 1);
    }
    double dx = pos[0] - rx[0];
    double dy = pos[1] - rx[1];
    double dz = pos[2] - rx[2];
    double err = sqrt(dx * dx + dy * dy + dz * dz);
    printf("GLONASS %s: used=%zu sats, recovered %.1f m from synthesis truth\n", label, used, err);
    if (!(isfinite(err) && err < 2000.0)) {
        return fail("GLONASS recovered position outside 2 km of truth", 1);
    }

    /* A3: per-system TDOP. A GLONASS-only solve has exactly one entry, for
     * GLONASS, and its value equals the scalar DOP's tdop. */
    SidereonSppSystemTdop tdops[GLO_MAX_SATS];
    size_t tdop_written = 123, tdop_required = 123;
    if (sidereon_spp_solution_system_tdops(sol, tdops, GLO_MAX_SATS, &tdop_written,
                                           &tdop_required) != SIDEREON_STATUS_OK ||
        tdop_written != 1 || tdop_required != 1 ||
        tdops[0].system != SIDEREON_GNSS_SYSTEM_GLONASS ||
        !(isfinite(tdops[0].tdop) && tdops[0].tdop > 0.0)) {
        return fail("sidereon_spp_solution_system_tdops (GLONASS)", 1);
    }
    SidereonDop dop;
    if (sidereon_spp_solution_dop(sol, &dop) != SIDEREON_STATUS_OK || dop.tdop != tdops[0].tdop) {
        return fail("system_tdops[0] must equal scalar DOP tdop", 1);
    }
    return 0;
}

/* (d) A populated GLONASS channel map must not perturb a GPS-only solve: same
 * position and clock, bit-for-bit, and the GPS golden is still reproduced. */
static int exercise_glonass_channel_noop_on_gps(const SidereonSp3 *sp3) {
    SidereonObservation observations[SPP_OBS_COUNT];
    for (size_t i = 0; i < SPP_OBS_COUNT; i++) {
        observations[i].sat_id = SPP_SAT_IDS[i];
        observations[i].pseudorange_m = bits_to_f64(SPP_PSEUDORANGE_BITS[i]);
    }
    SidereonSppInputsV2 inputs;
    if (sidereon_spp_inputs_v2_init(&inputs) != SIDEREON_STATUS_OK) {
        return fail("sidereon_spp_inputs_v2_init (gps no-op)", 1);
    }
    inputs.base.observations = observations;
    inputs.base.observation_count = SPP_OBS_COUNT;
    inputs.base.t_rx_j2000_s = bits_to_f64(SPP_T_RX_J2000_S_BITS);
    inputs.base.t_rx_second_of_day_s = bits_to_f64(SPP_T_RX_SOD_S_BITS);
    inputs.base.day_of_year = bits_to_f64(SPP_DOY_BITS);
    for (int i = 0; i < 4; i++) {
        inputs.base.initial_guess[i] = bits_to_f64(SPP_INITIAL_GUESS_BITS[i]);
        inputs.base.klobuchar_alpha[i] = bits_to_f64(SPP_KLOB_ALPHA_BITS[i]);
        inputs.base.klobuchar_beta[i] = bits_to_f64(SPP_KLOB_BETA_BITS[i]);
    }
    inputs.base.ionosphere = false;
    inputs.base.troposphere = false;
    inputs.base.pressure_hpa = bits_to_f64(SPP_PRESSURE_HPA_BITS);
    inputs.base.temperature_k = bits_to_f64(SPP_TEMPERATURE_K_BITS);
    inputs.base.relative_humidity = bits_to_f64(SPP_RELATIVE_HUMIDITY_BITS);
    inputs.base.with_geodetic = true;

    SidereonSppSolution *without = NULL;
    if (sidereon_solve_spp_v2(sp3, &inputs, &without) != SIDEREON_STATUS_OK) {
        return fail("GPS-only V2 solve without channels", 1);
    }
    double pos_without[3];
    double clk_without = 0.0;
    if (sidereon_spp_solution_position(without, pos_without, 3) != SIDEREON_STATUS_OK ||
        sidereon_spp_solution_rx_clock_s(without, &clk_without) != SIDEREON_STATUS_OK) {
        sidereon_spp_solution_free(without);
        return fail("GPS-only solve outputs (without channels)", 1);
    }
    sidereon_spp_solution_free(without);

    SidereonGlonassChannel channels[3] = {{1, 0}, {2, 3}, {7, -7}};
    inputs.glonass_channels = channels;
    inputs.glonass_channel_count = 3;
    SidereonSppSolution *with = NULL;
    if (sidereon_solve_spp_v2(sp3, &inputs, &with) != SIDEREON_STATUS_OK) {
        return fail("GPS-only V2 solve with channels", 1);
    }
    double pos_with[3];
    double clk_with = 0.0;
    if (sidereon_spp_solution_position(with, pos_with, 3) != SIDEREON_STATUS_OK ||
        sidereon_spp_solution_rx_clock_s(with, &clk_with) != SIDEREON_STATUS_OK) {
        sidereon_spp_solution_free(with);
        return fail("GPS-only solve outputs (with channels)", 1);
    }
    sidereon_spp_solution_free(with);

    for (int i = 0; i < 3; i++) {
        if (pos_with[i] != pos_without[i]) {
            return fail("GLONASS channels perturbed a GPS-only solve position", 1);
        }
    }
    if (clk_with != clk_without) {
        return fail("GLONASS channels perturbed a GPS-only solve clock", 1);
    }

    double expected[3];
    for (int i = 0; i < 3; i++) {
        expected[i] = bits_to_f64(SPP_EXPECTED_X_BITS[i]);
    }
    double dx = pos_without[0] - expected[0];
    double dy = pos_without[1] - expected[1];
    double dz = pos_without[2] - expected[2];
    if (!(sqrt(dx * dx + dy * dy + dz * dz) < SPP_AGREEMENT_BOUND_M)) {
        return fail("GPS golden not reproduced through the V2 path", 1);
    }
    return 0;
}

static int exercise_spp_glonass_channels(const SidereonSp3 *sp3) {
    double rx[3];
    double t_rx = 0.0;
    SidereonSatelliteToken tokens[GLO_MAX_SATS];
    SidereonObservation observations[GLO_MAX_SATS];
    SidereonGlonassChannel channels[GLO_MAX_SATS];
    int n = build_glonass_scenario(sp3, rx, tokens, observations, channels, &t_rx);
    if (n < 4) {
        return fail("GLONASS scenario needs at least four visible satellites", 1);
    }

    SidereonSppInputsV2 inputs;
    if (sidereon_spp_inputs_v2_init(&inputs) != SIDEREON_STATUS_OK) {
        return fail("sidereon_spp_inputs_v2_init", 1);
    }
    inputs.base.observations = observations;
    inputs.base.observation_count = (size_t)n;
    inputs.base.t_rx_j2000_s = t_rx;
    inputs.base.t_rx_second_of_day_s = 0.0;
    inputs.base.day_of_year = 176.0;
    inputs.base.initial_guess[0] = 6378137.0; /* equator/prime-meridian seed */
    inputs.base.initial_guess[1] = 0.0;
    inputs.base.initial_guess[2] = 0.0;
    inputs.base.initial_guess[3] = 0.0;
    inputs.base.klobuchar_alpha[0] = 1e-8;
    inputs.base.klobuchar_beta[0] = 1e5;
    for (int i = 1; i < 4; i++) {
        inputs.base.klobuchar_alpha[i] = 0.0;
        inputs.base.klobuchar_beta[i] = 0.0;
    }
    inputs.base.troposphere = false;
    inputs.base.pressure_hpa = 1013.25;
    inputs.base.temperature_k = 288.15;
    inputs.base.relative_humidity = 0.5;
    inputs.base.with_geodetic = true;

    /* End-to-end: ionosphere off, GLONASS solves with no channels needed. */
    inputs.base.ionosphere = false;
    SidereonSppSolution *off_sol = NULL;
    if (sidereon_solve_spp_v2(sp3, &inputs, &off_sol) != SIDEREON_STATUS_OK) {
        return fail("GLONASS-only SPP solve (ionosphere off)", 1);
    }
    if (check_glonass_solution(off_sol, rx, "ionosphere off") != 0) {
        sidereon_spp_solution_free(off_sol);
        return 1;
    }
    sidereon_spp_solution_free(off_sol);

    /* (a) ionosphere on, no channel map: rejected with IonosphereUnsupported. */
    inputs.base.ionosphere = true;
    inputs.glonass_channels = NULL;
    inputs.glonass_channel_count = 0;
    SidereonSppSolution *gated = (SidereonSppSolution *)(uintptr_t)1;
    if (sidereon_solve_spp_v2(sp3, &inputs, &gated) != SIDEREON_STATUS_SOLVE || gated != NULL) {
        return fail("GLONASS ionosphere-on solve without channels must be rejected", 1);
    }
    if (!last_error_is_glonass_carrier_gate()) {
        return fail("rejection must name a GLONASS satellite's carrier", 1);
    }

    /* (b) channels lift the gate: GLONASS solves with the ionosphere on. */
    inputs.glonass_channels = channels;
    inputs.glonass_channel_count = (size_t)n;
    SidereonSppSolution *on_sol = NULL;
    if (sidereon_solve_spp_v2(sp3, &inputs, &on_sol) != SIDEREON_STATUS_OK) {
        return fail("GLONASS SPP solve (ionosphere on, channels supplied)", 1);
    }
    if (check_glonass_solution(on_sol, rx, "ionosphere on") != 0) {
        sidereon_spp_solution_free(on_sol);
        return 1;
    }
    sidereon_spp_solution_free(on_sol);

    /* (c) channel 9 is outside the valid FDMA range [-7, +6], so the carrier is
     * unresolvable and the gate fires exactly as for a missing channel. */
    SidereonGlonassChannel bad_channels[GLO_MAX_SATS];
    for (int i = 0; i < n; i++) {
        bad_channels[i].slot = channels[i].slot;
        bad_channels[i].channel = 9;
    }
    inputs.glonass_channels = bad_channels;
    inputs.glonass_channel_count = (size_t)n;
    SidereonSppSolution *bad = (SidereonSppSolution *)(uintptr_t)1;
    if (sidereon_solve_spp_v2(sp3, &inputs, &bad) != SIDEREON_STATUS_SOLVE || bad != NULL) {
        return fail("out-of-range GLONASS channel must be rejected", 1);
    }
    if (!last_error_is_glonass_carrier_gate()) {
        return fail("out-of-range channel rejection must name a GLONASS carrier", 1);
    }

    /* A duplicate slot in the channel array is rejected by the binding before the
     * engine is reached. */
    SidereonGlonassChannel dup_channels[2] = {{1, 1}, {1, 2}};
    inputs.glonass_channels = dup_channels;
    inputs.glonass_channel_count = 2;
    SidereonSppSolution *dup = (SidereonSppSolution *)(uintptr_t)1;
    if (sidereon_solve_spp_v2(sp3, &inputs, &dup) != SIDEREON_STATUS_INVALID_ARGUMENT ||
        dup != NULL) {
        return fail("duplicate GLONASS channel slot must be rejected", 1);
    }

    /* (d) backward compatible: channels are a no-op on a GPS-only solve. */
    if (exercise_glonass_channel_noop_on_gps(sp3) != 0) {
        return 1;
    }

    printf("OK: GLONASS SPP end-to-end (iono off/on within 2 km), carrier gate, and GPS no-op\n");
    return 0;
}

/* Decode a CRINEX file through the C ABI (size query then buffer) and assert the
 * decoded text matches the committed crx2rnx reference .rnx line-for-line, the
 * real Hatanaka acceptance bar. Returns 0 on success. */
static int crinex_decode_matches_reference(const char *crx_path, const char *rnx_path,
                                           const char *label) {
    size_t crx_len = 0;
    uint8_t *crx = read_file(crx_path, &crx_len);
    if (crx == NULL) {
        fprintf(stderr, "FAIL: could not read CRINEX file: %s\n", crx_path);
        return 2;
    }
    size_t rnx_len = 0;
    uint8_t *rnx = read_file(rnx_path, &rnx_len);
    if (rnx == NULL) {
        free(crx);
        fprintf(stderr, "FAIL: could not read reference RINEX file: %s\n", rnx_path);
        return 2;
    }

    size_t written = 123, required = 123;
    if (sidereon_crinex_decode(crx, crx_len, NULL, 0, &written, &required) != SIDEREON_STATUS_OK ||
        written != 0 || required == 0) {
        free(crx);
        free(rnx);
        return fail(label, 1);
    }
    uint8_t *decoded = malloc(required);
    if (decoded == NULL) {
        free(crx);
        free(rnx);
        return fail("malloc decoded CRINEX", 1);
    }
    size_t decoded_len = required;
    written = 123;
    required = 123;
    if (sidereon_crinex_decode(crx, crx_len, decoded, decoded_len, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != decoded_len) {
        free(crx);
        free(rnx);
        free(decoded);
        return fail(label, 1);
    }
    free(crx);

    /* Line-by-line comparison, ignoring a single trailing newline on either side
     * (mirrors the engine's own crx2rnx byte-for-byte test, which compares
     * .lines()). */
    size_t di = 0, ri = 0;
    size_t line_no = 0;
    int rc = 0;
    while (di < decoded_len || ri < rnx_len) {
        size_t ds = di, rs = ri;
        while (di < decoded_len && decoded[di] != '\n') di++;
        while (ri < rnx_len && rnx[ri] != '\n') ri++;
        size_t dl = di - ds, rl = ri - rs;
        /* Trim a trailing CR for CRLF tolerance. */
        if (dl > 0 && decoded[ds + dl - 1] == '\r') dl--;
        if (rl > 0 && rnx[rs + rl - 1] == '\r') rl--;
        line_no++;
        if (dl != rl || memcmp(decoded + ds, rnx + rs, dl) != 0) {
            fprintf(stderr, "FAIL: %s line %zu differs\n", label, line_no);
            rc = 1;
            break;
        }
        if (di < decoded_len) di++; /* skip the newline */
        if (ri < rnx_len) ri++;
    }
    free(rnx);
    free(decoded);
    return rc;
}

/* CRINEX (Hatanaka) decode and RINEX-3 observation read. The decoded text must
 * match the committed crx2rnx reference byte-for-byte (v1 and v3); the observation
 * reader must reproduce the engine-parsed version, epoch count and sampled
 * observation values bit-for-bit, and parse the decoded CRINEX identically to the
 * reference. */
static int exercise_rinex_surface(const char *esbc_crx, const char *esbc_rnx,
                                  const char *algo_crx, const char *algo_rnx) {
    if (crinex_decode_matches_reference(esbc_crx, esbc_rnx, "CRINEX v3 (ESBC)") != 0) {
        return 1;
    }
    if (crinex_decode_matches_reference(algo_crx, algo_rnx, "CRINEX v1 (algo)") != 0) {
        return 1;
    }

    /* Parse the reference .rnx and assert version + epoch count + sampled values
     * against the engine-generated golden. */
    size_t rnx_len = 0;
    uint8_t *rnx = read_file(esbc_rnx, &rnx_len);
    if (rnx == NULL) {
        fprintf(stderr, "FAIL: could not read reference RINEX: %s\n", esbc_rnx);
        return 2;
    }
    SidereonRinexObs *obs = NULL;
    if (sidereon_rinex_obs_parse(rnx, rnx_len, &obs) != SIDEREON_STATUS_OK) {
        free(rnx);
        return fail("sidereon_rinex_obs_parse", 1);
    }
    free(rnx);

    double version = 0.0;
    if (sidereon_rinex_obs_version(obs, &version) != SIDEREON_STATUS_OK ||
        f64_to_bits(version) != RINEX_VERSION_BITS) {
        sidereon_rinex_obs_free(obs);
        return fail("sidereon_rinex_obs_version", 1);
    }
    size_t epoch_count = 0;
    if (sidereon_rinex_obs_epoch_count(obs, &epoch_count) != SIDEREON_STATUS_OK ||
        epoch_count != RINEX_EPOCH_COUNT) {
        sidereon_rinex_obs_free(obs);
        return fail("sidereon_rinex_obs_epoch_count", 1);
    }
    for (size_t i = 0; i < RINEX_SAMPLE_COUNT; i++) {
        const RinexObsSample *s = &RINEX_SAMPLES[i];
        double value = -1.0;
        bool present = false;
        int32_t lli = -2, ssi = -2;
        if (sidereon_rinex_obs_observation(obs, s->epoch_index, s->sat, s->code, &value, &present,
                                           &lli, &ssi) != SIDEREON_STATUS_OK ||
            !present || f64_to_bits(value) != s->value_bits) {
            fprintf(stderr, "FAIL: RINEX obs %s/%s epoch %zu not bit-exact (%.17g)\n", s->sat,
                    s->code, s->epoch_index, value);
            sidereon_rinex_obs_free(obs);
            return 1;
        }
    }

    /* Decode the matching CRINEX and assert it parses identically to the .rnx
     * (epoch count plus the sampled observation values), mirroring the engine's
     * parses_crinex_decoded_text_identically. */
    size_t crx_len = 0;
    uint8_t *crx = read_file(esbc_crx, &crx_len);
    if (crx == NULL) {
        sidereon_rinex_obs_free(obs);
        fprintf(stderr, "FAIL: could not read CRINEX: %s\n", esbc_crx);
        return 2;
    }
    size_t required = 0, written = 0;
    sidereon_crinex_decode(crx, crx_len, NULL, 0, &written, &required);
    uint8_t *decoded = malloc(required);
    if (decoded == NULL || sidereon_crinex_decode(crx, crx_len, decoded, required, &written,
                                                  &required) != SIDEREON_STATUS_OK) {
        free(crx);
        free(decoded);
        sidereon_rinex_obs_free(obs);
        return fail("decode CRINEX for parse-equality", 1);
    }
    free(crx);
    SidereonRinexObs *from_crx = NULL;
    if (sidereon_rinex_obs_parse(decoded, written, &from_crx) != SIDEREON_STATUS_OK) {
        free(decoded);
        sidereon_rinex_obs_free(obs);
        return fail("parse decoded CRINEX", 1);
    }
    free(decoded);
    size_t crx_epochs = 0;
    if (sidereon_rinex_obs_epoch_count(from_crx, &crx_epochs) != SIDEREON_STATUS_OK ||
        crx_epochs != epoch_count) {
        sidereon_rinex_obs_free(from_crx);
        sidereon_rinex_obs_free(obs);
        return fail("decoded CRINEX epoch count matches reference", 1);
    }
    for (size_t i = 0; i < RINEX_SAMPLE_COUNT; i++) {
        const RinexObsSample *s = &RINEX_SAMPLES[i];
        double value = -1.0;
        bool present = false;
        int32_t lli = -2, ssi = -2;
        if (sidereon_rinex_obs_observation(from_crx, s->epoch_index, s->sat, s->code, &value,
                                           &present, &lli, &ssi) != SIDEREON_STATUS_OK ||
            !present || f64_to_bits(value) != s->value_bits) {
            sidereon_rinex_obs_free(from_crx);
            sidereon_rinex_obs_free(obs);
            return fail("decoded CRINEX observation matches reference", 1);
        }
    }
    sidereon_rinex_obs_free(from_crx);

    /* Argument gates. */
    double value = 0.0;
    bool present = true;
    int32_t lli = 0, ssi = 0;
    if (sidereon_rinex_obs_observation(obs, (size_t)-1, "G02", "C1C", &value, &present, &lli,
                                       &ssi) != SIDEREON_STATUS_INVALID_ARGUMENT ||
        present) {
        sidereon_rinex_obs_free(obs);
        return fail("sidereon_rinex_obs_observation out-of-range epoch", 1);
    }
    if (sidereon_rinex_obs_observation(obs, 0, "G02", "ZZ9", &value, &present, &lli, &ssi) !=
        SIDEREON_STATUS_INVALID_ARGUMENT) {
        sidereon_rinex_obs_free(obs);
        return fail("sidereon_rinex_obs_observation unknown code", 1);
    }

    sidereon_rinex_obs_free(obs);
    sidereon_rinex_obs_free(NULL); /* free(NULL) is a no-op. */
    printf("RINEX surface: CRINEX v1+v3 decode byte-exact, %d obs samples bit-exact, decoded "
           "parses identically\n",
           (int)RINEX_SAMPLE_COUNT);
    return 0;
}

/* Ionosphere: standalone Klobuchar (native units) and IONEX slant delay. Both C
 * entries call the engine kernels the engine certifies bit-for-bit against
 * klobuchar_golden.json / ionex_golden.json. The binding reproduces the L1 and
 * BeiDou-B1I Klobuchar delays and the IONEX slant delays bit-for-bit (IONEX
 * takes degrees and applies the same pi/180 boundary multiply as the goldens). */
static int exercise_iono_surface(const char *ionex_path) {
    double f_l1 = bits_to_f64(KLOB_F_L1_HZ_BITS);
    double f_b1i = bits_to_f64(KLOB_F_B1I_HZ_BITS);

    for (size_t c = 0; c < KLOB_CASE_COUNT; c++) {
        const KlobucharCase *kc = &KLOB_CASES[c];
        double alpha[4], beta[4];
        for (int i = 0; i < 4; i++) {
            alpha[i] = bits_to_f64(kc->alpha_bits[i]);
            beta[i] = bits_to_f64(kc->beta_bits[i]);
        }
        double lat = bits_to_f64(kc->lat_deg_bits);
        double lon = bits_to_f64(kc->lon_deg_bits);
        double az = bits_to_f64(kc->az_deg_bits);
        double el = bits_to_f64(kc->el_deg_bits);
        double t = bits_to_f64(kc->t_gps_s_bits);

        double delay_l1 = -1.0;
        if (sidereon_klobuchar_native(alpha, beta, lat, lon, az, el, t, f_l1, &delay_l1) !=
                SIDEREON_STATUS_OK ||
            f64_to_bits(delay_l1) != kc->delay_l1_m_bits) {
            fprintf(stderr, "FAIL: Klobuchar L1 %s not bit-exact (%.17g)\n", kc->name, delay_l1);
            return 1;
        }
        double delay_b1i = -1.0;
        if (sidereon_klobuchar_native(alpha, beta, lat, lon, az, el, t, f_b1i, &delay_b1i) !=
                SIDEREON_STATUS_OK ||
            f64_to_bits(delay_b1i) != kc->delay_b1i_m_bits) {
            fprintf(stderr, "FAIL: Klobuchar B1I %s not bit-exact (%.17g)\n", kc->name, delay_b1i);
            return 1;
        }
    }

    /* Klobuchar argument gates. */
    const double a4[4] = {0.0, 0.0, 0.0, 0.0};
    double out = -1.0;
    if (sidereon_klobuchar_native(NULL, a4, 0.0, 0.0, 0.0, 45.0, 0.0, f_l1, &out) !=
            SIDEREON_STATUS_NULL_POINTER ||
        out != 0.0) {
        return fail("sidereon_klobuchar_native null alpha clears out", 1);
    }
    if (sidereon_klobuchar_native(a4, a4, 0.0, 0.0, 0.0, 45.0, 0.0, f_l1, NULL) !=
        SIDEREON_STATUS_NULL_POINTER) {
        return fail("sidereon_klobuchar_native null out", 1);
    }
    /* A second-of-day past one day is out of range. */
    if (sidereon_klobuchar_native(a4, a4, 0.0, 0.0, 0.0, 45.0, 1.0e9, f_l1, &out) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        out != 0.0) {
        return fail("sidereon_klobuchar_native out-of-range t_gps_s", 1);
    }

    /* IONEX. */
    size_t len = 0;
    uint8_t *bytes = read_file(ionex_path, &len);
    if (bytes == NULL) {
        fprintf(stderr, "FAIL: could not read IONEX file: %s\n", ionex_path);
        return 2;
    }
    SidereonIonex *ionex = NULL;
    if (sidereon_ionex_parse(bytes, len, &ionex) != SIDEREON_STATUS_OK) {
        free(bytes);
        return fail("sidereon_ionex_parse", 1);
    }
    free(bytes);

    size_t epoch_count = 0;
    if (sidereon_ionex_epoch_count(ionex, &epoch_count) != SIDEREON_STATUS_OK || epoch_count != 2) {
        sidereon_ionex_free(ionex);
        return fail("sidereon_ionex_epoch_count", 1);
    }
    int32_t exponent = 0;
    if (sidereon_ionex_exponent(ionex, &exponent) != SIDEREON_STATUS_OK ||
        exponent != IONEX_EXPONENT) {
        sidereon_ionex_free(ionex);
        return fail("sidereon_ionex_exponent", 1);
    }
    size_t written = 0, required = 0;
    double lat_nodes[16];
    if (sidereon_ionex_lat_nodes_deg(ionex, lat_nodes, 16, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != IONEX_LAT_NODE_COUNT || required != IONEX_LAT_NODE_COUNT) {
        sidereon_ionex_free(ionex);
        return fail("sidereon_ionex_lat_nodes_deg", 1);
    }
    double lon_nodes[16];
    if (sidereon_ionex_lon_nodes_deg(ionex, lon_nodes, 16, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != IONEX_LON_NODE_COUNT || required != IONEX_LON_NODE_COUNT) {
        sidereon_ionex_free(ionex);
        return fail("sidereon_ionex_lon_nodes_deg", 1);
    }
    int64_t epochs[8];
    if (sidereon_ionex_map_epochs_j2000_s(ionex, epochs, 8, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != epoch_count) {
        sidereon_ionex_free(ionex);
        return fail("sidereon_ionex_map_epochs_j2000_s", 1);
    }

    for (size_t c = 0; c < IONEX_CASE_COUNT; c++) {
        const IonexCase *ic = &IONEX_CASES[c];
        double delay = -1.0;
        if (sidereon_ionex_slant_delay(ionex, bits_to_f64(ic->lat_deg_bits),
                                       bits_to_f64(ic->lon_deg_bits), bits_to_f64(ic->az_deg_bits),
                                       bits_to_f64(ic->el_deg_bits), ic->epoch_j2000_s,
                                       bits_to_f64(ic->frequency_hz_bits), &delay) !=
                SIDEREON_STATUS_OK ||
            f64_to_bits(delay) != ic->delay_m_bits) {
            fprintf(stderr, "FAIL: IONEX slant %s not bit-exact (%.17g)\n", ic->name, delay);
            sidereon_ionex_free(ionex);
            return 1;
        }
        if (c == 0) {
            SidereonIonexSlantDelayEvaluation eval;
            if (sidereon_ionex_slant_delay_with_policy(
                    ionex, bits_to_f64(ic->lat_deg_bits), bits_to_f64(ic->lon_deg_bits),
                    bits_to_f64(ic->az_deg_bits), bits_to_f64(ic->el_deg_bits),
                    ic->epoch_j2000_s, bits_to_f64(ic->frequency_hz_bits),
                    SIDEREON_IONEX_COVERAGE_POLICY_STRICT, &eval) != SIDEREON_STATUS_OK ||
                f64_to_bits(eval.delay_m) != ic->delay_m_bits ||
                eval.status != SIDEREON_IONEX_SLANT_DELAY_STATUS_VALID ||
                eval.coverage_error != SIDEREON_IONEX_COVERAGE_ERROR_KIND_NONE) {
                sidereon_ionex_free(ionex);
                return fail("sidereon_ionex_slant_delay_with_policy valid status", 1);
            }
        }
        if (strcmp(ic->name, "epoch_before_hold") == 0) {
            SidereonIonexSlantDelayEvaluation held;
            if (sidereon_ionex_slant_delay_with_policy(
                    ionex, bits_to_f64(ic->lat_deg_bits), bits_to_f64(ic->lon_deg_bits),
                    bits_to_f64(ic->az_deg_bits), bits_to_f64(ic->el_deg_bits),
                    ic->epoch_j2000_s, bits_to_f64(ic->frequency_hz_bits),
                    SIDEREON_IONEX_COVERAGE_POLICY_HOLD, &held) != SIDEREON_STATUS_OK ||
                f64_to_bits(held.delay_m) != ic->delay_m_bits ||
                held.status != SIDEREON_IONEX_SLANT_DELAY_STATUS_HELD ||
                held.coverage_error !=
                    SIDEREON_IONEX_COVERAGE_ERROR_KIND_EPOCH_BEFORE_FIRST_MAP) {
                sidereon_ionex_free(ionex);
                return fail("sidereon_ionex_slant_delay_with_policy held status", 1);
            }
            if (sidereon_ionex_slant_delay_with_policy(
                    ionex, bits_to_f64(ic->lat_deg_bits), bits_to_f64(ic->lon_deg_bits),
                    bits_to_f64(ic->az_deg_bits), bits_to_f64(ic->el_deg_bits),
                    ic->epoch_j2000_s, bits_to_f64(ic->frequency_hz_bits),
                    SIDEREON_IONEX_COVERAGE_POLICY_STRICT, &held) == SIDEREON_STATUS_OK) {
                sidereon_ionex_free(ionex);
                return fail("sidereon_ionex_slant_delay_with_policy strict coverage", 1);
            }
        }
    }

    /* IONEX argument gates. */
    double dirty = -1.0;
    if (sidereon_ionex_slant_delay(NULL, 0.0, 0.0, 0.0, 45.0, 0, f_l1, &dirty) !=
            SIDEREON_STATUS_NULL_POINTER ||
        dirty != 0.0) {
        sidereon_ionex_free(ionex);
        return fail("sidereon_ionex_slant_delay null ionex clears out", 1);
    }

    sidereon_ionex_free(ionex);
    sidereon_ionex_free(NULL); /* free(NULL) is a no-op. */
    printf("Iono surface: %d Klobuchar cases (L1 + B1I) and %d IONEX slant cases bit-exact\n",
           (int)KLOB_CASE_COUNT, (int)IONEX_CASE_COUNT);
    return 0;
}

/* Receiver velocity. The engine-synthesized observations and frozen solution
 * come from sidereon-core's velocity golden scenario (velocity_fixture.h). The
 * binding feeds those observations against the same SP3 source and must
 * reproduce the engine's velocity/clock-drift/residuals bit-for-bit, for both
 * the range-rate and Doppler paths, plus the too-few-satellites gate. */
static int check_velocity_solution(const SidereonVelocitySolution *sol, const uint64_t vel_bits[3],
                                   uint64_t speed_bits, uint64_t drift_bits, size_t used_count,
                                   const char *const *used_ids, const uint64_t *residual_bits,
                                   const char *label) {
    double vel[3] = {0.0, 0.0, 0.0};
    if (sidereon_velocity_solution_velocity(sol, vel, 3) != SIDEREON_STATUS_OK ||
        f64_to_bits(vel[0]) != vel_bits[0] || f64_to_bits(vel[1]) != vel_bits[1] ||
        f64_to_bits(vel[2]) != vel_bits[2]) {
        return fail(label, 1);
    }
    double speed = 0.0;
    if (sidereon_velocity_solution_speed(sol, &speed) != SIDEREON_STATUS_OK ||
        f64_to_bits(speed) != speed_bits) {
        return fail(label, 1);
    }
    double drift = 0.0;
    if (sidereon_velocity_solution_clock_drift(sol, &drift) != SIDEREON_STATUS_OK ||
        f64_to_bits(drift) != drift_bits) {
        return fail(label, 1);
    }
    size_t count = 0;
    if (sidereon_velocity_solution_used_sat_count(sol, &count) != SIDEREON_STATUS_OK ||
        count != used_count) {
        return fail(label, 1);
    }
    SidereonSatelliteToken tokens[VEL_OBS_COUNT];
    size_t written = 0, required = 0;
    if (sidereon_velocity_solution_used_sat_ids(sol, tokens, VEL_OBS_COUNT, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != used_count || required != used_count) {
        return fail(label, 1);
    }
    for (size_t i = 0; i < written; i++) {
        if (!token_equals(&tokens[i], used_ids[i])) {
            return fail(label, 1);
        }
    }
    double residuals[VEL_OBS_COUNT];
    written = 0;
    required = 0;
    if (sidereon_velocity_solution_residuals(sol, residuals, VEL_OBS_COUNT, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != used_count || required != used_count) {
        return fail(label, 1);
    }
    for (size_t i = 0; i < written; i++) {
        if (f64_to_bits(residuals[i]) != residual_bits[i]) {
            fprintf(stderr, "FAIL: %s residual[%zu] not bit-exact\n", label, i);
            return 1;
        }
    }
    return 0;
}

static int exercise_velocity_surface(const SidereonSp3 *sp3) {
    const double receiver[3] = {bits_to_f64(VEL_RECEIVER_BITS[0]), bits_to_f64(VEL_RECEIVER_BITS[1]),
                                bits_to_f64(VEL_RECEIVER_BITS[2])};
    double t_rx = bits_to_f64(VEL_T_RX_J2000_S_BITS);

    SidereonVelocityOptions opts;
    if (sidereon_velocity_options_init(&opts) != SIDEREON_STATUS_OK ||
        opts.observable != SIDEREON_VELOCITY_OBSERVABLE_RANGE_RATE || !opts.light_time ||
        !opts.sagnac) {
        return fail("sidereon_velocity_options_init defaults", 1);
    }

    /* Range-rate path. */
    SidereonVelocityObservation rr[VEL_OBS_COUNT];
    for (size_t i = 0; i < VEL_OBS_COUNT; i++) {
        rr[i].sat_id = VEL_SAT_IDS[i];
        rr[i].value = bits_to_f64(VEL_RANGE_RATE_BITS[i]);
        rr[i].carrier_hz = bits_to_f64(VEL_F_L1_HZ_BITS);
        rr[i].sat_clock_drift_s_s = 0.0;
    }
    SidereonVelocitySolution *rr_sol = NULL;
    if (sidereon_solve_velocity(sp3, rr, VEL_OBS_COUNT, receiver, t_rx, &opts, &rr_sol) !=
            SIDEREON_STATUS_OK ||
        rr_sol == NULL) {
        return fail("sidereon_solve_velocity range-rate", 1);
    }
    if (check_velocity_solution(rr_sol, VEL_RR_VELOCITY_BITS, VEL_RR_SPEED_BITS,
                                VEL_RR_CLOCK_DRIFT_BITS, VEL_RR_USED_COUNT, VEL_RR_USED_IDS,
                                VEL_RR_RESIDUAL_BITS, "velocity range-rate") != 0) {
        sidereon_velocity_solution_free(rr_sol);
        return 1;
    }
    sidereon_velocity_solution_free(rr_sol);

    /* Doppler path with per-satellite carriers. NULL options would default to
     * range-rate, so pass explicit Doppler options. */
    SidereonVelocityObservation dop[VEL_OBS_COUNT];
    for (size_t i = 0; i < VEL_OBS_COUNT; i++) {
        dop[i].sat_id = VEL_SAT_IDS[i];
        dop[i].value = bits_to_f64(VEL_DOPPLER_BITS[i]);
        dop[i].carrier_hz = bits_to_f64(VEL_DOPPLER_CARRIER_BITS[i]);
        dop[i].sat_clock_drift_s_s = 0.0;
    }
    SidereonVelocityOptions dop_opts = opts;
    dop_opts.observable = SIDEREON_VELOCITY_OBSERVABLE_DOPPLER;
    SidereonVelocitySolution *dop_sol = NULL;
    if (sidereon_solve_velocity(sp3, dop, VEL_OBS_COUNT, receiver, t_rx, &dop_opts, &dop_sol) !=
            SIDEREON_STATUS_OK ||
        dop_sol == NULL) {
        return fail("sidereon_solve_velocity doppler", 1);
    }
    if (check_velocity_solution(dop_sol, VEL_DOP_VELOCITY_BITS, VEL_DOP_SPEED_BITS,
                                VEL_DOP_CLOCK_DRIFT_BITS, VEL_DOP_USED_COUNT, VEL_DOP_USED_IDS,
                                VEL_DOP_RESIDUAL_BITS, "velocity doppler") != 0) {
        sidereon_velocity_solution_free(dop_sol);
        return 1;
    }
    sidereon_velocity_solution_free(dop_sol);

    /* Fewer than four observations cannot resolve the 4-state geometry. */
    SidereonVelocitySolution *thin = (SidereonVelocitySolution *)(uintptr_t)1;
    if (sidereon_solve_velocity(sp3, rr, 3, receiver, t_rx, &opts, &thin) !=
            SIDEREON_STATUS_SOLVE ||
        thin != NULL) {
        return fail("sidereon_solve_velocity too few sats clears out_solution", 1);
    }

    /* Argument gates. */
    SidereonVelocitySolution *bad = (SidereonVelocitySolution *)(uintptr_t)1;
    if (sidereon_solve_velocity(NULL, rr, VEL_OBS_COUNT, receiver, t_rx, &opts, &bad) !=
            SIDEREON_STATUS_NULL_POINTER ||
        bad != NULL) {
        return fail("sidereon_solve_velocity null sp3 clears out_solution", 1);
    }
    if (sidereon_solve_velocity(sp3, rr, VEL_OBS_COUNT, receiver, t_rx, &opts, NULL) !=
        SIDEREON_STATUS_NULL_POINTER) {
        return fail("sidereon_solve_velocity null out_solution", 1);
    }
    sidereon_velocity_solution_free(NULL); /* free(NULL) is a no-op. */

    printf("Velocity surface: range-rate and Doppler solves bit-exact (%d sats), too-few gate OK\n",
           (int)VEL_OBS_COUNT);
    return 0;
}

/* ANTEX antenna PCO/PCV. The C accessors call the engine's ANTEX parser/lookup,
 * which the engine certifies bit-for-bit against the trimmed real igs20 .atx via
 * antex_golden.json. So the binding must parse the same .atx and reproduce every
 * golden PCO triple and PCV grid node bit-for-bit. */
static int exercise_antex_surface(const char *path) {
    size_t len = 0;
    uint8_t *bytes = read_file(path, &len);
    if (bytes == NULL) {
        fprintf(stderr, "FAIL: could not read ANTEX file: %s\n", path);
        return 2;
    }

    SidereonAntex *antex = NULL;
    if (sidereon_antex_parse(bytes, len, &antex) != SIDEREON_STATUS_OK) {
        free(bytes);
        return fail("sidereon_antex_parse", 1);
    }
    free(bytes);

    size_t antenna_count = 999;
    if (sidereon_antex_antenna_count(antex, &antenna_count) != SIDEREON_STATUS_OK ||
        antenna_count != ANTEX_ANTENNA_COUNT) {
        sidereon_antex_free(antex);
        return fail("sidereon_antex_antenna_count", 1);
    }

    for (size_t c = 0; c < ANTEX_PCO_CASE_COUNT; c++) {
        const AntexPcoCase *pc = &ANTEX_PCO_CASES[c];
        SidereonAntenna *antenna = NULL;
        if (sidereon_antex_antenna(antex, pc->antenna_id, &antenna) != SIDEREON_STATUS_OK ||
            antenna == NULL) {
            sidereon_antex_free(antex);
            return fail(pc->antenna_id, 1);
        }
        double neu[3] = {1.0, 2.0, 3.0};
        if (sidereon_antenna_pco(antenna, pc->frequency, neu) != SIDEREON_STATUS_OK ||
            f64_to_bits(neu[0]) != pc->north_m_bits ||
            f64_to_bits(neu[1]) != pc->east_m_bits ||
            f64_to_bits(neu[2]) != pc->up_m_bits) {
            fprintf(stderr, "FAIL: ANTEX PCO %s/%s not bit-exact\n", pc->antenna_id,
                    pc->frequency);
            sidereon_antenna_free(antenna);
            sidereon_antex_free(antex);
            return 1;
        }
        sidereon_antenna_free(antenna);
    }

    for (size_t c = 0; c < ANTEX_PCV_CASE_COUNT; c++) {
        const AntexPcvCase *pc = &ANTEX_PCV_CASES[c];
        SidereonAntenna *antenna = NULL;
        if (sidereon_antex_antenna(antex, pc->antenna_id, &antenna) != SIDEREON_STATUS_OK ||
            antenna == NULL) {
            sidereon_antex_free(antex);
            return fail(pc->antenna_id, 1);
        }
        double value = -7.0;
        if (sidereon_antenna_pcv(antenna, pc->frequency, pc->zenith_deg, pc->has_azimuth,
                                 pc->azimuth_deg, &value) != SIDEREON_STATUS_OK ||
            f64_to_bits(value) != pc->value_m_bits) {
            fprintf(stderr, "FAIL: ANTEX PCV %s/%s zen=%.1f not bit-exact\n", pc->antenna_id,
                    pc->frequency, pc->zenith_deg);
            sidereon_antenna_free(antenna);
            sidereon_antex_free(antex);
            return 1;
        }
        sidereon_antenna_free(antenna);
    }

    /* A missing antenna id is a successful query that found nothing. */
    SidereonAntenna *missing = (SidereonAntenna *)(uintptr_t)1;
    if (sidereon_antex_antenna(antex, "NO SUCH ANTENNA", &missing) != SIDEREON_STATUS_OK ||
        missing != NULL) {
        sidereon_antex_free(antex);
        return fail("sidereon_antex_antenna missing id clears out_antenna", 1);
    }

    /* An unknown frequency on a real antenna is an error and clears the output. */
    SidereonAntenna *real = NULL;
    if (sidereon_antex_antenna(antex, ANTEX_PCO_CASES[0].antenna_id, &real) !=
            SIDEREON_STATUS_OK ||
        real == NULL) {
        sidereon_antex_free(antex);
        return fail("sidereon_antex_antenna lookup for negative test", 1);
    }
    double neu[3] = {1.0, 2.0, 3.0};
    if (sidereon_antenna_pco(real, "ZZ9", neu) != SIDEREON_STATUS_INVALID_ARGUMENT ||
        neu[0] != 0.0 || neu[1] != 0.0 || neu[2] != 0.0) {
        sidereon_antenna_free(real);
        sidereon_antex_free(antex);
        return fail("sidereon_antenna_pco unknown frequency clears out_neu", 1);
    }
    sidereon_antenna_free(real);

    /* Argument gates. */
    SidereonAntenna *out = (SidereonAntenna *)(uintptr_t)1;
    if (sidereon_antex_antenna(NULL, ANTEX_PCO_CASES[0].antenna_id, &out) !=
            SIDEREON_STATUS_NULL_POINTER ||
        out != NULL) {
        sidereon_antex_free(antex);
        return fail("sidereon_antex_antenna null antex clears out_antenna", 1);
    }
    /* Non-UTF-8 bytes are rejected (ANTEX is line-based ASCII text) and the
     * output handle is cleared. */
    const uint8_t not_utf8[] = {0xff, 0xfe, 0x00, 0x01};
    SidereonAntex *bad = (SidereonAntex *)(uintptr_t)1;
    if (sidereon_antex_parse(not_utf8, sizeof(not_utf8), &bad) != SIDEREON_STATUS_INVALID_TOKEN ||
        bad != NULL) {
        sidereon_antex_free(antex);
        return fail("sidereon_antex_parse rejects non-UTF-8 and clears out_antex", 1);
    }

    sidereon_antex_free(antex);
    sidereon_antex_free(NULL); /* free(NULL) is a no-op. */
    printf("ANTEX surface: %d antennas, %d PCO triples and %d PCV nodes bit-exact\n",
           (int)ANTEX_ANTENNA_COUNT, (int)ANTEX_PCO_CASE_COUNT, (int)ANTEX_PCV_CASE_COUNT);
    return 0;
}

/* Standalone DOP. The C entry calls the engine's `dop` kernel, which the engine
 * certifies 0-ULP against the reference recipe via dop_golden.json. So the
 * binding must reproduce each golden case's scalars bit-for-bit, the singular
 * family must be rejected, and the az/el line-of-sight constructor must produce
 * a unit vector that feeds a finite DOP. */
static int exercise_dop_surface(void) {
    for (size_t c = 0; c < DOP_CASE_COUNT; c++) {
        const DopGoldenCase *gc = &DOP_CASES[c];
        SidereonLineOfSight los[8];
        double weights[8] = {0.0};
        if (gc->sat_count > 8) {
            return fail("dop golden case too large for smoke buffer", 1);
        }
        for (size_t i = 0; i < gc->sat_count; i++) {
            los[i].e_x = bits_to_f64(gc->los_bits[i][0]);
            los[i].e_y = bits_to_f64(gc->los_bits[i][1]);
            los[i].e_z = bits_to_f64(gc->los_bits[i][2]);
            weights[i] = bits_to_f64(gc->weight_bits[i]);
        }
        SidereonGeodetic rx = {bits_to_f64(gc->lat_rad_bits),
                               bits_to_f64(gc->lon_rad_bits), 0.0};
        SidereonDop out = {1.0, 2.0, 3.0, 4.0, 5.0};
        if (sidereon_dop(los, weights, gc->sat_count, rx, &out) != SIDEREON_STATUS_OK) {
            return fail(gc->name, 1);
        }
        /* 0 ULP: the binding delegates to the same kernel the golden certifies. */
        if (f64_to_bits(out.gdop) != gc->gdop_bits ||
            f64_to_bits(out.pdop) != gc->pdop_bits ||
            f64_to_bits(out.hdop) != gc->hdop_bits ||
            f64_to_bits(out.vdop) != gc->vdop_bits ||
            f64_to_bits(out.tdop) != gc->tdop_bits) {
            fprintf(stderr,
                    "FAIL: DOP %s not bit-exact: gdop=%.17g pdop=%.17g hdop=%.17g "
                    "vdop=%.17g tdop=%.17g\n",
                    gc->name, out.gdop, out.pdop, out.hdop, out.vdop, out.tdop);
            return 1;
        }
    }

    for (size_t c = 0; c < DOP_SINGULAR_COUNT; c++) {
        const DopSingularCase *sc = &DOP_SINGULAR[c];
        SidereonLineOfSight los[8];
        double weights[8] = {0.0};
        if (sc->sat_count > 8) {
            return fail("dop singular case too large for smoke buffer", 1);
        }
        for (size_t i = 0; i < sc->sat_count; i++) {
            los[i].e_x = bits_to_f64(sc->los_bits[i][0]);
            los[i].e_y = bits_to_f64(sc->los_bits[i][1]);
            los[i].e_z = bits_to_f64(sc->los_bits[i][2]);
            weights[i] = bits_to_f64(sc->weight_bits[i]);
        }
        SidereonGeodetic rx = {bits_to_f64(sc->lat_rad_bits),
                               bits_to_f64(sc->lon_rad_bits), 0.0};
        SidereonDop out = {1.0, 2.0, 3.0, 4.0, 5.0};
        /* Rank-deficient / singular geometry has no finite DOP: the engine
         * reports it (too-few-sats and singular both surface as SOLVE) and the
         * binding clears out_dop. */
        if (sidereon_dop(los, weights, sc->sat_count, rx, &out) != SIDEREON_STATUS_SOLVE ||
            out.gdop != 0.0 || out.pdop != 0.0 || out.hdop != 0.0 || out.vdop != 0.0 ||
            out.tdop != 0.0) {
            return fail(sc->name, 1);
        }
    }

    /* Argument gates. */
    SidereonLineOfSight one_los = {1.0, 0.0, 0.0};
    double one_w = 1.0;
    SidereonGeodetic rx0 = {0.0, 0.0, 0.0};
    SidereonDop dirty = {1.0, 2.0, 3.0, 4.0, 5.0};
    if (sidereon_dop(&one_los, &one_w, 1, rx0, NULL) != SIDEREON_STATUS_NULL_POINTER) {
        return fail("sidereon_dop null out_dop", 1);
    }
    if (sidereon_dop(NULL, &one_w, 1, rx0, &dirty) != SIDEREON_STATUS_NULL_POINTER ||
        dirty.gdop != 0.0) {
        return fail("sidereon_dop null los clears out_dop", 1);
    }
    dirty.gdop = 9.0;
    if (sidereon_dop(&one_los, &one_w, (size_t)-1, rx0, &dirty) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        dirty.gdop != 0.0) {
        return fail("sidereon_dop oversized count clears out_dop", 1);
    }

    /* az/el line-of-sight constructor: four well-spread directions form a unit
     * basis that yields a finite DOP, and the engine validates the constructed
     * vector is unit length. */
    const double az[4] = {0.0, 90.0, 180.0, 270.0};
    const double el[4] = {80.0, 30.0, 30.0, 30.0};
    SidereonGeodetic site = {0.6, 0.1, 0.0};
    SidereonLineOfSight built[4];
    double built_w[4];
    for (int i = 0; i < 4; i++) {
        SidereonLineOfSight l = {7.0, 7.0, 7.0};
        if (sidereon_line_of_sight_from_az_el_deg(az[i], el[i], site, &l) !=
            SIDEREON_STATUS_OK) {
            return fail("sidereon_line_of_sight_from_az_el_deg", 1);
        }
        double norm = sqrt(l.e_x * l.e_x + l.e_y * l.e_y + l.e_z * l.e_z);
        if (fabs(norm - 1.0) > 1.0e-12) {
            return fail("sidereon_line_of_sight_from_az_el_deg unit length", 1);
        }
        built[i] = l;
        built_w[i] = 1.0;
    }
    SidereonDop built_dop = {0.0, 0.0, 0.0, 0.0, 0.0};
    if (sidereon_dop(built, built_w, 4, site, &built_dop) != SIDEREON_STATUS_OK ||
        !isfinite(built_dop.gdop) || built_dop.gdop <= 0.0) {
        return fail("sidereon_dop from az/el geometry", 1);
    }
    /* Out-of-range elevation is rejected and out_los cleared. */
    SidereonLineOfSight bad = {1.0, 1.0, 1.0};
    if (sidereon_line_of_sight_from_az_el_deg(0.0, 91.0, site, &bad) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        bad.e_x != 0.0 || bad.e_y != 0.0 || bad.e_z != 0.0) {
        return fail("sidereon_line_of_sight_from_az_el_deg out-of-range elevation", 1);
    }

    printf("DOP surface: %d golden cases bit-exact, %d singular rejected, az/el LOS OK\n",
           (int)DOP_CASE_COUNT, (int)DOP_SINGULAR_COUNT);
    return 0;
}

/* A1: inter-system time-scale offsets. The fixed atomic offsets and the
 * leap-aware UTC/GLONASST offsets are golden bit-exact against the engine (and
 * RTKLIB); the new GLONASST/QZSST scales are exercised, and the UTC-based and
 * TDB cases surface the typed rejection as INVALID_ARGUMENT. */
static int exercise_timescale_surface(void) {
    double off = 0.0;

    /* Fixed atomic offsets (to_reading - from_reading). */
    if (sidereon_timescale_offset_s(SIDEREON_TIME_SCALE_GPST, SIDEREON_TIME_SCALE_BDT, &off) !=
            SIDEREON_STATUS_OK ||
        off != -14.0) {
        return fail("timescale_offset_s GPST->BDT", 1);
    }
    if (sidereon_timescale_offset_s(SIDEREON_TIME_SCALE_BDT, SIDEREON_TIME_SCALE_GPST, &off) !=
            SIDEREON_STATUS_OK ||
        off != 14.0) {
        return fail("timescale_offset_s BDT->GPST", 1);
    }
    if (sidereon_timescale_offset_s(SIDEREON_TIME_SCALE_TAI, SIDEREON_TIME_SCALE_TT, &off) !=
            SIDEREON_STATUS_OK ||
        off != 32.184) {
        return fail("timescale_offset_s TAI->TT", 1);
    }
    if (sidereon_timescale_offset_s(SIDEREON_TIME_SCALE_GPST, SIDEREON_TIME_SCALE_TT, &off) !=
            SIDEREON_STATUS_OK ||
        off != 51.184) {
        return fail("timescale_offset_s GPST->TT", 1);
    }
    if (sidereon_timescale_offset_s(SIDEREON_TIME_SCALE_GPST, SIDEREON_TIME_SCALE_TAI, &off) !=
            SIDEREON_STATUS_OK ||
        off != 19.0) {
        return fail("timescale_offset_s GPST->TAI", 1);
    }
    /* New scales: GST and QZSST are steered to GPST (nominal offset 0). */
    if (sidereon_timescale_offset_s(SIDEREON_TIME_SCALE_GPST, SIDEREON_TIME_SCALE_GST, &off) !=
            SIDEREON_STATUS_OK ||
        off != 0.0) {
        return fail("timescale_offset_s GPST->GST", 1);
    }
    if (sidereon_timescale_offset_s(SIDEREON_TIME_SCALE_GPST, SIDEREON_TIME_SCALE_QZSST, &off) !=
            SIDEREON_STATUS_OK ||
        off != 0.0) {
        return fail("timescale_offset_s GPST->QZSST", 1);
    }

    /* Leap-aware offsets at 2017-01-01 00:00:00 UTC (JD 2457754.5). */
    const double jd_2017 = 2457754.5;
    if (sidereon_timescale_offset_at_s(SIDEREON_TIME_SCALE_UTC, SIDEREON_TIME_SCALE_GPST, jd_2017,
                                       &off) != SIDEREON_STATUS_OK ||
        off != 18.0) {
        return fail("timescale_offset_at_s UTC->GPST 2017", 1);
    }
    if (sidereon_timescale_offset_at_s(SIDEREON_TIME_SCALE_GPST, SIDEREON_TIME_SCALE_UTC, jd_2017,
                                       &off) != SIDEREON_STATUS_OK ||
        off != -18.0) {
        return fail("timescale_offset_at_s GPST->UTC 2017", 1);
    }
    /* New GLONASST scale, leap-aware: GLONASST - GPST = +10782 s at 2017. */
    if (sidereon_timescale_offset_at_s(SIDEREON_TIME_SCALE_GPST, SIDEREON_TIME_SCALE_GLONASST,
                                       jd_2017, &off) != SIDEREON_STATUS_OK ||
        off != 10782.0) {
        return fail("timescale_offset_at_s GPST->GLONASST 2017", 1);
    }

    /* Fixed query on a UTC-based scale is rejected (needs an epoch). */
    off = 123.0;
    if (sidereon_timescale_offset_s(SIDEREON_TIME_SCALE_GPST, SIDEREON_TIME_SCALE_UTC, &off) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        off != 0.0) {
        return fail("timescale_offset_s UTC must require epoch", 1);
    }
    off = 123.0;
    if (sidereon_timescale_offset_s(SIDEREON_TIME_SCALE_GLONASST, SIDEREON_TIME_SCALE_GPST, &off) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        off != 0.0) {
        return fail("timescale_offset_s GLONASST must require epoch", 1);
    }
    /* TDB has no fixed/constant offset. */
    if (sidereon_timescale_offset_s(SIDEREON_TIME_SCALE_GPST, SIDEREON_TIME_SCALE_TDB, &off) !=
        SIDEREON_STATUS_INVALID_ARGUMENT) {
        return fail("timescale_offset_s TDB must be rejected", 1);
    }
    /* Leap-aware query with a non-finite epoch for a UTC-based scale is rejected. */
    if (sidereon_timescale_offset_at_s(SIDEREON_TIME_SCALE_GPST, SIDEREON_TIME_SCALE_UTC,
                                       (double)NAN, &off) != SIDEREON_STATUS_INVALID_ARGUMENT) {
        return fail("timescale_offset_at_s non-finite UTC epoch must be rejected", 1);
    }
    /* An unknown scale code is rejected. */
    if (sidereon_timescale_offset_s(999, SIDEREON_TIME_SCALE_GPST, &off) !=
        SIDEREON_STATUS_INVALID_ARGUMENT) {
        return fail("timescale_offset_s invalid scale code", 1);
    }

    printf("timescale surface: fixed + leap-aware offsets golden, GLONASST/QZSST OK, "
           "UTC/TDB rejected\n");
    return 0;
}

/* GNSS constellation identity catalog: build the merged GPS catalog from the
 * same CelesTrak gps-ops OMM/JSON and NAVCEN status HTML the engine certifies,
 * then assert the compact mapping CSV byte-for-byte and the SP3-id validation
 * result. PRN 19 is unusable per NAVCEN, so it renders active=false in the CSV
 * and surfaces in inactive_unusable_prns while missing/extra stay empty. */
static int exercise_constellation_surface(void) {
    SidereonConstellation *catalog = NULL;
    if (sidereon_constellation_build(SIDEREON_GNSS_SYSTEM_GPS, CONSTELLATION_GPS_OPS_JSON,
                                     CONSTELLATION_GPS_OPS_JSON_LEN, CONSTELLATION_NAVCEN_HTML,
                                     CONSTELLATION_NAVCEN_HTML_LEN, &catalog) != SIDEREON_STATUS_OK ||
        catalog == NULL) {
        return fail("sidereon_constellation_build", 1);
    }

    size_t record_count = 0;
    if (sidereon_constellation_record_count(catalog, &record_count) != SIDEREON_STATUS_OK ||
        record_count != 4) {
        sidereon_constellation_free(catalog);
        return fail("sidereon_constellation_record_count", 1);
    }

    /* Per-record accessor (A2): the catalog is sorted by (system, prn), so the
     * first record is GPS PRN 3 (active, no FDMA channel) and the last is PRN 19
     * (NAVCEN-unusable -> active=false). */
    SidereonConstellationRecord rec0;
    if (sidereon_constellation_record(catalog, 0, &rec0) != SIDEREON_STATUS_OK ||
        rec0.system != SIDEREON_GNSS_SYSTEM_GPS || rec0.prn != 3 || !rec0.active ||
        rec0.fdma_channel_present) {
        sidereon_constellation_free(catalog);
        return fail("sidereon_constellation_record[0]", 1);
    }
    /* PRN 19 is present in the base source (active) but marked unusable by the
     * NAVCEN overlay, which is why it renders active=false in the CSV. */
    SidereonConstellationRecord rec3;
    if (sidereon_constellation_record(catalog, 3, &rec3) != SIDEREON_STATUS_OK ||
        rec3.prn != 19 || !rec3.active || rec3.usable) {
        sidereon_constellation_free(catalog);
        return fail("sidereon_constellation_record[3]", 1);
    }
    SidereonConstellationRecord oob;
    if (sidereon_constellation_record(catalog, record_count, &oob) !=
        SIDEREON_STATUS_INVALID_ARGUMENT) {
        sidereon_constellation_free(catalog);
        return fail("sidereon_constellation_record out-of-range index", 1);
    }

    /* Standalone system-aware SP3-id builder (A2): GPS PRN 5 -> "G05", and the
     * GLONASS adapter letter -> "R12". Not null-terminated. */
    char sp3id[8];
    size_t sp3id_written = 123, sp3id_required = 123;
    if (sidereon_constellation_gnss_sp3_id(SIDEREON_GNSS_SYSTEM_GPS, 5, (uint8_t *)sp3id,
                                           sizeof(sp3id), &sp3id_written, &sp3id_required) !=
            SIDEREON_STATUS_OK ||
        sp3id_written != 3 || memcmp(sp3id, "G05", 3) != 0) {
        sidereon_constellation_free(catalog);
        return fail("sidereon_constellation_gnss_sp3_id GPS", 1);
    }
    if (sidereon_constellation_gnss_sp3_id(SIDEREON_GNSS_SYSTEM_GLONASS, 12, (uint8_t *)sp3id,
                                           sizeof(sp3id), &sp3id_written, &sp3id_required) !=
            SIDEREON_STATUS_OK ||
        sp3id_written != 3 || memcmp(sp3id, "R12", 3) != 0) {
        sidereon_constellation_free(catalog);
        return fail("sidereon_constellation_gnss_sp3_id GLONASS", 1);
    }

    /* CSV: size query, then full copy, then byte-exact compare. */
    size_t csv_written = 123, csv_required = 123;
    if (sidereon_constellation_to_csv(catalog, SIDEREON_CONSTELLATION_BOOL_STYLE_LOWER, NULL, 0,
                                      &csv_written, &csv_required) != SIDEREON_STATUS_OK ||
        csv_written != 0 || csv_required != strlen(CONSTELLATION_EXPECTED_CSV)) {
        sidereon_constellation_free(catalog);
        return fail("sidereon_constellation_to_csv size query", 1);
    }
    uint8_t *csv = malloc(csv_required);
    if (csv == NULL) {
        sidereon_constellation_free(catalog);
        return fail("constellation CSV allocation", 2);
    }
    if (sidereon_constellation_to_csv(catalog, SIDEREON_CONSTELLATION_BOOL_STYLE_LOWER, csv,
                                      csv_required, &csv_written, &csv_required) !=
            SIDEREON_STATUS_OK ||
        csv_written != csv_required ||
        memcmp(csv, CONSTELLATION_EXPECTED_CSV, csv_required) != 0) {
        free(csv);
        sidereon_constellation_free(catalog);
        return fail("sidereon_constellation_to_csv exact bytes", 1);
    }
    free(csv);

    /* An out-of-range CSV boolean style is rejected. */
    size_t bad_written = 123, bad_required = 123;
    if (sidereon_constellation_to_csv(catalog, 99, NULL, 0, &bad_written, &bad_required) !=
        SIDEREON_STATUS_INVALID_ARGUMENT) {
        sidereon_constellation_free(catalog);
        return fail("sidereon_constellation_to_csv invalid bool style", 1);
    }

    /* Validate against the operational SP3 ids (G03, G05, G13). */
    SidereonConstellationValidation *validation = NULL;
    if (sidereon_constellation_validate_against_sp3_ids(catalog, CONSTELLATION_VALIDATE_SP3_IDS,
                                                        CONSTELLATION_VALIDATE_SP3_ID_COUNT,
                                                        &validation) != SIDEREON_STATUS_OK ||
        validation == NULL) {
        sidereon_constellation_free(catalog);
        return fail("sidereon_constellation_validate_against_sp3_ids", 1);
    }

    bool valid = true;
    if (sidereon_constellation_validation_is_valid(validation, &valid) != SIDEREON_STATUS_OK ||
        valid) {
        sidereon_constellation_validation_free(validation);
        sidereon_constellation_free(catalog);
        return fail("constellation validation is not clean (inactive PRN expected)", 1);
    }

    size_t inactive_written = 123, inactive_required = 123;
    SidereonConstellationPrn inactive[8];
    if (sidereon_constellation_validation_inactive_unusable_prns(
            validation, inactive, 8, &inactive_written, &inactive_required) !=
            SIDEREON_STATUS_OK ||
        inactive_written != CONSTELLATION_EXPECTED_INACTIVE_UNUSABLE_PRN_COUNT ||
        inactive_required != CONSTELLATION_EXPECTED_INACTIVE_UNUSABLE_PRN_COUNT) {
        sidereon_constellation_validation_free(validation);
        sidereon_constellation_free(catalog);
        return fail("sidereon_constellation_validation_inactive_unusable_prns count", 1);
    }
    for (size_t i = 0; i < inactive_written; i++) {
        /* The PRN list is now system-qualified: GPS PRN 19. */
        if (inactive[i].prn != CONSTELLATION_EXPECTED_INACTIVE_UNUSABLE_PRNS[i] ||
            inactive[i].system != SIDEREON_GNSS_SYSTEM_GPS) {
            sidereon_constellation_validation_free(validation);
            sidereon_constellation_free(catalog);
            return fail("sidereon_constellation_validation_inactive_unusable_prns value", 1);
        }
    }

    /* Missing and extra SP3 id lists must both be empty. */
    size_t missing_written = 123, missing_required = 123;
    if (sidereon_constellation_validation_missing_sp3_ids(validation, NULL, 0, &missing_written,
                                                          &missing_required) !=
            SIDEREON_STATUS_OK ||
        missing_written != 0 || missing_required != 0) {
        sidereon_constellation_validation_free(validation);
        sidereon_constellation_free(catalog);
        return fail("sidereon_constellation_validation_missing_sp3_ids empty", 1);
    }
    size_t extra_written = 123, extra_required = 123;
    if (sidereon_constellation_validation_extra_sp3_ids(validation, NULL, 0, &extra_written,
                                                        &extra_required) != SIDEREON_STATUS_OK ||
        extra_written != 0 || extra_required != 0) {
        sidereon_constellation_validation_free(validation);
        sidereon_constellation_free(catalog);
        return fail("sidereon_constellation_validation_extra_sp3_ids empty", 1);
    }

    sidereon_constellation_validation_free(validation);

    /* Build without the NAVCEN overlay: PRN 19 stays usable, so the catalog is
     * clean against the same ids plus G19. */
    SidereonConstellation *celestrak_only = NULL;
    if (sidereon_constellation_build(SIDEREON_GNSS_SYSTEM_GPS, CONSTELLATION_GPS_OPS_JSON,
                                     CONSTELLATION_GPS_OPS_JSON_LEN, NULL, 0,
                                     &celestrak_only) != SIDEREON_STATUS_OK ||
        celestrak_only == NULL) {
        sidereon_constellation_free(catalog);
        return fail("sidereon_constellation_build without NAVCEN", 1);
    }
    const char *all_ids[] = {"G03", "G05", "G13", "G19"};
    SidereonConstellationValidation *clean = NULL;
    if (sidereon_constellation_validate_against_sp3_ids(celestrak_only, all_ids, 4, &clean) !=
            SIDEREON_STATUS_OK ||
        clean == NULL) {
        sidereon_constellation_free(celestrak_only);
        sidereon_constellation_free(catalog);
        return fail("validate CelesTrak-only catalog", 1);
    }
    bool clean_valid = false;
    if (sidereon_constellation_validation_is_valid(clean, &clean_valid) != SIDEREON_STATUS_OK ||
        !clean_valid) {
        sidereon_constellation_validation_free(clean);
        sidereon_constellation_free(celestrak_only);
        sidereon_constellation_free(catalog);
        return fail("CelesTrak-only catalog should validate clean", 1);
    }
    sidereon_constellation_validation_free(clean);
    sidereon_constellation_free(celestrak_only);

    /* Argument gate: a null catalog clears the record count. */
    record_count = 123;
    if (sidereon_constellation_record_count(NULL, &record_count) != SIDEREON_STATUS_NULL_POINTER ||
        record_count != 0) {
        sidereon_constellation_free(catalog);
        return fail("sidereon_constellation_record_count null catalog clears out_count", 1);
    }

    sidereon_constellation_free(catalog);
    sidereon_constellation_free(NULL);            /* free(NULL) is a no-op. */
    sidereon_constellation_validation_free(NULL); /* free(NULL) is a no-op. */
    printf("constellation surface: merged catalog CSV byte-exact, PRN 19 inactive, "
           "missing/extra empty\n");
    return 0;
}

/* Product-staleness selection (sidereon_core::staleness through the C ABI). The
 * exact-selection path returns a byte-identical product, so interpolating /
 * evaluating it reproduces the engine bit-for-bit; the degraded paths attach the
 * staleness provenance; and the over-cap / empty / no-prior cases surface the
 * typed SidereonSelectionStatus. The SP3 product is the loaded GRG fixture; the
 * IONEX product is the synthetic 2-map fixture also used by the iono surface. */
static int exercise_staleness_surface(const SidereonSp3 *sp3, const char *ionex_path) {
    /* ---- SP3 staleness selection ---- */
    size_t sp3_epoch_count = 0;
    size_t required = 0;
    if (sidereon_sp3_epochs_j2000_seconds(sp3, NULL, 0, &sp3_epoch_count, &required) !=
            SIDEREON_STATUS_OK ||
        required < 2) {
        return fail("staleness: sp3 epoch count", 1);
    }
    double *sp3_epochs = (double *)malloc(required * sizeof(double));
    if (sp3_epochs == NULL) {
        return fail("staleness: sp3 epochs alloc", 1);
    }
    if (sidereon_sp3_epochs_j2000_seconds(sp3, sp3_epochs, required, &sp3_epoch_count, &required) !=
        SIDEREON_STATUS_OK) {
        free(sp3_epochs);
        return fail("staleness: sp3 epochs copy", 1);
    }
    double covered_epoch = sp3_epochs[1];
    double last_epoch = sp3_epochs[required - 1];
    free(sp3_epochs);

    SidereonStalenessPolicy policy = sidereon_staleness_policy_default();
    if (f64_to_bits(policy.max_staleness_s) != f64_to_bits(3.0 * 86400.0)) {
        return fail("staleness: default policy cap", 1);
    }

    const SidereonSp3 *one_sp3[1] = {sp3};

    /* Exact: a covered epoch returns a byte-identical clone with zero staleness,
     * and interpolating it matches the original product bit-for-bit. */
    SidereonSp3 *exact_sel = NULL;
    SidereonStalenessMetadata exact_meta;
    if (sidereon_select_sp3(one_sp3, 1, covered_epoch, policy, &exact_sel, &exact_meta) !=
            SIDEREON_SELECTION_STATUS_OK ||
        exact_sel == NULL || exact_meta.kind != SIDEREON_DEGRADATION_KIND_EXACT ||
        exact_meta.staleness_s != 0.0 || exact_meta.staleness_days != 0.0) {
        sidereon_sp3_free(exact_sel);
        return fail("staleness: select_sp3 exact metadata", 1);
    }
    {
        const char *sat = VEL_SAT_IDS[0];
        double sel_pos[3] = {0};
        double sel_clk = 0.0;
        size_t written = 0;
        double orig_pos[3] = {0};
        double orig_clk = 0.0;
        if (sidereon_sp3_interpolate(exact_sel, sat, &covered_epoch, 1, sel_pos, 3, &sel_clk, 1,
                                     &written) != SIDEREON_STATUS_OK ||
            sidereon_sp3_interpolate(sp3, sat, &covered_epoch, 1, orig_pos, 3, &orig_clk, 1,
                                     &written) != SIDEREON_STATUS_OK ||
            f64_to_bits(sel_pos[0]) != f64_to_bits(orig_pos[0]) ||
            f64_to_bits(sel_pos[1]) != f64_to_bits(orig_pos[1]) ||
            f64_to_bits(sel_pos[2]) != f64_to_bits(orig_pos[2]) ||
            f64_to_bits(sel_clk) != f64_to_bits(orig_clk)) {
            sidereon_sp3_free(exact_sel);
            return fail("staleness: select_sp3 exact interpolation not bit-exact", 1);
        }
    }
    sidereon_sp3_free(exact_sel);

    /* Nearest-prior: an epoch past coverage selects the product as-is with the
     * staleness measured from its last epoch. */
    double stale_epoch = last_epoch + 100.0;
    SidereonSp3 *prior_sel = NULL;
    SidereonStalenessMetadata prior_meta;
    if (sidereon_select_sp3(one_sp3, 1, stale_epoch, policy, &prior_sel, &prior_meta) !=
            SIDEREON_SELECTION_STATUS_OK ||
        prior_sel == NULL || prior_meta.kind != SIDEREON_DEGRADATION_KIND_NEAREST_PRIOR ||
        fabs(prior_meta.staleness_s - 100.0) > 1.0e-6 ||
        fabs(prior_meta.source_epoch_j2000_s - last_epoch) > 1.0e-6) {
        sidereon_sp3_free(prior_sel);
        return fail("staleness: select_sp3 nearest-prior metadata", 1);
    }
    sidereon_sp3_free(prior_sel);

    /* Beyond cap: a 1-second cap rejects the 100-second-stale prior with the
     * typed BeyondStalenessCap status and writes no handle. */
    SidereonSp3 *cap_sel = (SidereonSp3 *)(uintptr_t)1;
    SidereonStalenessMetadata cap_meta;
    if (sidereon_select_sp3(one_sp3, 1, stale_epoch, sidereon_staleness_policy_seconds(1.0),
                            &cap_sel, &cap_meta) != SIDEREON_SELECTION_STATUS_BEYOND_STALENESS_CAP ||
        cap_sel != NULL) {
        sidereon_sp3_free(cap_sel);
        return fail("staleness: select_sp3 beyond-cap typed error", 1);
    }

    /* Empty product set: the typed EmptyProductSet status, no handle. */
    SidereonSp3 *empty_sel = (SidereonSp3 *)(uintptr_t)1;
    SidereonStalenessMetadata empty_meta;
    if (sidereon_select_sp3(NULL, 0, covered_epoch, policy, &empty_sel, &empty_meta) !=
            SIDEREON_SELECTION_STATUS_EMPTY_PRODUCT_SET ||
        empty_sel != NULL) {
        sidereon_sp3_free(empty_sel);
        return fail("staleness: select_sp3 empty-set typed error", 1);
    }

    /* No prior product: the loaded SP3 cannot serve an epoch a week before its
     * earliest coverage (covered_epoch - 7 days), so selection has no prior. */
    SidereonSp3 *no_prior_sel = (SidereonSp3 *)(uintptr_t)1;
    SidereonStalenessMetadata no_prior_meta;
    if (sidereon_select_sp3(one_sp3, 1, covered_epoch - 7.0 * 86400.0, policy, &no_prior_sel,
                            &no_prior_meta) != SIDEREON_SELECTION_STATUS_NO_PRIOR_PRODUCT ||
        no_prior_sel != NULL) {
        sidereon_sp3_free(no_prior_sel);
        return fail("staleness: select_sp3 no-prior typed error", 1);
    }

    /* ---- IONEX staleness selection ---- */
    size_t ionex_len = 0;
    uint8_t *ionex_bytes = read_file(ionex_path, &ionex_len);
    if (ionex_bytes == NULL) {
        return fail("staleness: read IONEX file", 1);
    }
    SidereonIonex *ionex = NULL;
    if (sidereon_ionex_parse(ionex_bytes, ionex_len, &ionex) != SIDEREON_STATUS_OK) {
        free(ionex_bytes);
        return fail("staleness: sidereon_ionex_parse", 1);
    }
    free(ionex_bytes);

    const SidereonIonex *one_ionex[1] = {ionex};
    const IonexCase *ic = &IONEX_CASES[0]; /* epoch within the product coverage */
    int64_t covered_ionex_epoch = ic->epoch_j2000_s;

    /* Exact: byte-identical product, slant delay reproduces the iono golden. */
    SidereonIonex *ionex_exact = NULL;
    SidereonStalenessMetadata ionex_exact_meta;
    if (sidereon_select_ionex(one_ionex, 1, covered_ionex_epoch, policy, &ionex_exact,
                              &ionex_exact_meta) != SIDEREON_SELECTION_STATUS_OK ||
        ionex_exact == NULL || ionex_exact_meta.kind != SIDEREON_DEGRADATION_KIND_EXACT ||
        ionex_exact_meta.staleness_s != 0.0) {
        sidereon_ionex_free(ionex_exact);
        sidereon_ionex_free(ionex);
        return fail("staleness: select_ionex exact metadata", 1);
    }
    {
        double delay = -1.0;
        if (sidereon_ionex_slant_delay(ionex_exact, bits_to_f64(ic->lat_deg_bits),
                                       bits_to_f64(ic->lon_deg_bits), bits_to_f64(ic->az_deg_bits),
                                       bits_to_f64(ic->el_deg_bits), covered_ionex_epoch,
                                       bits_to_f64(ic->frequency_hz_bits), &delay) !=
                SIDEREON_STATUS_OK ||
            f64_to_bits(delay) != ic->delay_m_bits) {
            sidereon_ionex_free(ionex_exact);
            sidereon_ionex_free(ionex);
            return fail("staleness: select_ionex exact slant not bit-exact", 1);
        }
    }
    sidereon_ionex_free(ionex_exact);

    /* Diurnal shift: an epoch one day past coverage advances the grid by a whole
     * day; the slant delay at the shifted epoch reproduces the un-shifted delay
     * bit-for-bit (the grid values are unchanged, only the epoch axis moves). */
    int64_t shifted_epoch = covered_ionex_epoch + 86400;
    SidereonIonex *ionex_shift = NULL;
    SidereonStalenessMetadata ionex_shift_meta;
    if (sidereon_select_ionex(one_ionex, 1, shifted_epoch, policy, &ionex_shift,
                              &ionex_shift_meta) != SIDEREON_SELECTION_STATUS_OK ||
        ionex_shift == NULL ||
        ionex_shift_meta.kind != SIDEREON_DEGRADATION_KIND_DIURNAL_SHIFT ||
        ionex_shift_meta.staleness_s != 86400.0 || ionex_shift_meta.staleness_days != 1.0) {
        sidereon_ionex_free(ionex_shift);
        sidereon_ionex_free(ionex);
        return fail("staleness: select_ionex diurnal-shift metadata", 1);
    }
    {
        double delay = -1.0;
        if (sidereon_ionex_slant_delay(ionex_shift, bits_to_f64(ic->lat_deg_bits),
                                       bits_to_f64(ic->lon_deg_bits), bits_to_f64(ic->az_deg_bits),
                                       bits_to_f64(ic->el_deg_bits), shifted_epoch,
                                       bits_to_f64(ic->frequency_hz_bits), &delay) !=
                SIDEREON_STATUS_OK ||
            f64_to_bits(delay) != ic->delay_m_bits) {
            sidereon_ionex_free(ionex_shift);
            sidereon_ionex_free(ionex);
            return fail("staleness: select_ionex diurnal-shift slant not bit-exact", 1);
        }
    }
    sidereon_ionex_free(ionex_shift);

    /* Invalid policy: a non-finite cap is the silent-masking hazard and is a
     * typed error, not a default. */
    SidereonIonex *bad_policy_sel = (SidereonIonex *)(uintptr_t)1;
    SidereonStalenessMetadata bad_policy_meta;
    SidereonStalenessPolicy nan_policy = {.max_staleness_s = NAN};
    if (sidereon_select_ionex(one_ionex, 1, covered_ionex_epoch, nan_policy, &bad_policy_sel,
                              &bad_policy_meta) != SIDEREON_SELECTION_STATUS_INVALID_POLICY ||
        bad_policy_sel != NULL) {
        sidereon_ionex_free(bad_policy_sel);
        sidereon_ionex_free(ionex);
        return fail("staleness: select_ionex invalid-policy typed error", 1);
    }

    sidereon_ionex_free(ionex);
    printf("staleness surface: SP3 exact/nearest-prior + cap/empty/no-prior, IONEX exact/diurnal "
           "+ invalid-policy, exact selections bit-exact\n");
    return 0;
}

/* Read the ECEF position and receiver clock out of an SPP solution handle. */
static int spp_solution_pos_clock(const SidereonSppSolution *sol, double pos[3], double *clock,
                                  size_t *used) {
    if (sidereon_spp_solution_position(sol, pos, 3) != SIDEREON_STATUS_OK ||
        sidereon_spp_solution_rx_clock_s(sol, clock) != SIDEREON_STATUS_OK ||
        sidereon_spp_solution_used_sat_count(sol, used) != SIDEREON_STATUS_OK) {
        return 1;
    }
    return 0;
}

/* Read the ECEF position and receiver clock out of a sourced solution handle. */
static int sourced_pos_clock(const SidereonSourcedSolution *sourced, double pos[3], double *clock,
                             size_t *used) {
    SidereonSppSolution *inner = NULL;
    if (sidereon_sourced_solution_solution(sourced, &inner) != SIDEREON_STATUS_OK) {
        return 1;
    }
    int rc = spp_solution_pos_clock(inner, pos, clock, used);
    sidereon_spp_solution_free(inner);
    return rc;
}

/* Broadcast-ephemeris SPP and the precise-with-broadcast fallback through the C
 * ABI. Inputs are the ESBC DOY177 GPS C1C first-epoch observations extracted from
 * the engine into broadcast_fixture.h; the products are the committed broadcast
 * NAV and SP3 fixtures. Asserts the source + staleness provenance on each branch,
 * the labeled broadcast-vs-precise agreement, and bit-exact broadcast fallback. */
static int exercise_broadcast_fallback_surface(const char *nav_path, const char *precise_sp3_path,
                                               const char *prior_sp3_path,
                                               const char *wrong_epoch_sp3_path) {
    SidereonObservation obs[BC_OBS_COUNT];
    for (size_t i = 0; i < BC_OBS_COUNT; i++) {
        obs[i].sat_id = BC_SAT_IDS[i];
        obs[i].pseudorange_m = bits_to_f64(BC_PSEUDORANGE_BITS[i]);
    }
    SidereonSppInputs inputs;
    inputs.observations = obs;
    inputs.observation_count = BC_OBS_COUNT;
    inputs.t_rx_j2000_s = bits_to_f64(BC_T_RX_J2000_S_BITS);
    inputs.t_rx_second_of_day_s = bits_to_f64(BC_T_RX_SOD_S_BITS);
    inputs.day_of_year = bits_to_f64(BC_DOY_BITS);
    for (int i = 0; i < 4; i++) {
        inputs.initial_guess[i] = bits_to_f64(BC_INITIAL_GUESS_BITS[i]);
        inputs.klobuchar_alpha[i] = 0.0;
        inputs.klobuchar_beta[i] = 0.0;
    }
    inputs.ionosphere = false;
    inputs.troposphere = true;
    inputs.pressure_hpa = bits_to_f64(BC_PRESSURE_HPA_BITS);
    inputs.temperature_k = bits_to_f64(BC_TEMPERATURE_K_BITS);
    inputs.relative_humidity = bits_to_f64(BC_RELATIVE_HUMIDITY_BITS);
    inputs.with_geodetic = true;

    /* Parse the broadcast navigation message. */
    size_t nav_len = 0;
    uint8_t *nav_bytes = read_file(nav_path, &nav_len);
    if (nav_bytes == NULL) {
        return fail("broadcast: read NAV file", 1);
    }
    SidereonBroadcastEphemeris *broadcast = NULL;
    if (sidereon_broadcast_ephemeris_parse_nav(nav_bytes, nav_len, &broadcast) !=
        SIDEREON_STATUS_OK) {
        free(nav_bytes);
        return fail("broadcast: sidereon_broadcast_ephemeris_parse_nav", 1);
    }
    free(nav_bytes);

    /* Load the precise, prior-day, and wrong-epoch SP3 products. */
    SidereonSp3 *precise = NULL;
    SidereonSp3 *prior = NULL;
    SidereonSp3 *wrong = NULL;
    int rc = 1;
    size_t blen = 0;
    uint8_t *bytes = read_file(precise_sp3_path, &blen);
    if (bytes == NULL || sidereon_sp3_load(bytes, blen, &precise) != SIDEREON_STATUS_OK) {
        free(bytes);
        sidereon_broadcast_ephemeris_free(broadcast);
        return fail("broadcast: load precise SP3", 1);
    }
    free(bytes);
    bytes = read_file(prior_sp3_path, &blen);
    if (bytes == NULL || sidereon_sp3_load(bytes, blen, &prior) != SIDEREON_STATUS_OK) {
        free(bytes);
        goto cleanup;
    }
    free(bytes);
    bytes = read_file(wrong_epoch_sp3_path, &blen);
    if (bytes == NULL || sidereon_sp3_load(bytes, blen, &wrong) != SIDEREON_STATUS_OK) {
        free(bytes);
        goto cleanup;
    }
    free(bytes);

    SidereonStalenessPolicy policy = sidereon_staleness_policy_days(3.0);

    /* Broadcast-only SPP: the supported real-time / offline mode. */
    SidereonSppSolution *broadcast_sol = NULL;
    if (sidereon_solve_broadcast(broadcast, &inputs, &broadcast_sol) != SIDEREON_STATUS_OK) {
        (void)fail("broadcast: sidereon_solve_broadcast", 1);
        goto cleanup;
    }
    double pos_b[3];
    double clk_b;
    size_t used_b;
    if (spp_solution_pos_clock(broadcast_sol, pos_b, &clk_b, &used_b) || used_b < 5) {
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: broadcast-only solution readout", 1);
        goto cleanup;
    }

    /* Fallback with a precise product covering the epoch: source is precise-exact
     * with zero staleness, and the precise fix agrees with the broadcast fix
     * within the labeled signal-in-space bound (and is not implausibly equal). */
    const SidereonSp3 *precise_set[1] = {precise};
    SidereonSourcedSolution *fb_exact = NULL;
    if (sidereon_solve_with_fallback(precise_set, 1, broadcast, &inputs, policy, &fb_exact) !=
        SIDEREON_FALLBACK_STATUS_OK) {
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: fallback exact solve", 1);
        goto cleanup;
    }
    SidereonFixSourceKind kind = SIDEREON_FIX_SOURCE_KIND_BROADCAST;
    bool is_exact = false;
    SidereonStalenessMetadata meta;
    bool present = false;
    if (sidereon_sourced_solution_source_kind(fb_exact, &kind) != SIDEREON_STATUS_OK ||
        kind != SIDEREON_FIX_SOURCE_KIND_PRECISE ||
        sidereon_sourced_solution_is_precise_exact(fb_exact, &is_exact) != SIDEREON_STATUS_OK ||
        !is_exact ||
        sidereon_sourced_solution_staleness(fb_exact, &meta, &present) != SIDEREON_STATUS_OK ||
        !present || meta.kind != SIDEREON_DEGRADATION_KIND_EXACT || meta.staleness_s != 0.0) {
        sidereon_sourced_solution_free(fb_exact);
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: fallback exact provenance", 1);
        goto cleanup;
    }
    double pos_e[3];
    double clk_e;
    size_t used_e;
    if (sourced_pos_clock(fb_exact, pos_e, &clk_e, &used_e)) {
        sidereon_sourced_solution_free(fb_exact);
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: fallback exact readout", 1);
        goto cleanup;
    }
    sidereon_sourced_solution_free(fb_exact);
    double dpe = sqrt((pos_e[0] - pos_b[0]) * (pos_e[0] - pos_b[0]) +
                      (pos_e[1] - pos_b[1]) * (pos_e[1] - pos_b[1]) +
                      (pos_e[2] - pos_b[2]) * (pos_e[2] - pos_b[2]));
    if (!(dpe > 0.01) || !(dpe < BC_VS_PRECISE_POSITION_BOUND_M)) {
        sidereon_spp_solution_free(broadcast_sol);
        fprintf(stderr, "FAIL: broadcast-vs-precise delta %.4f m outside (0.01, %.1f)\n", dpe,
                BC_VS_PRECISE_POSITION_BOUND_M);
        goto cleanup;
    }

    /* Fallback with no precise product: drops to broadcast, reason
     * PreciseUnavailable(EmptyProductSet), and the fix is bit-for-bit the
     * broadcast-only solve (both go through the engine's solve on broadcast). */
    SidereonSourcedSolution *fb_empty = NULL;
    if (sidereon_solve_with_fallback(NULL, 0, broadcast, &inputs, policy, &fb_empty) !=
        SIDEREON_FALLBACK_STATUS_OK) {
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: fallback empty solve", 1);
        goto cleanup;
    }
    SidereonBroadcastReasonKind reason_kind = SIDEREON_BROADCAST_REASON_KIND_PRECISE_DEGRADED_UNUSABLE;
    SidereonSelectionStatus unavail = SIDEREON_SELECTION_STATUS_OK;
    SidereonStalenessMetadata attempted;
    bool has_attempted = true;
    present = true;
    if (sidereon_sourced_solution_source_kind(fb_empty, &kind) != SIDEREON_STATUS_OK ||
        kind != SIDEREON_FIX_SOURCE_KIND_BROADCAST ||
        sidereon_sourced_solution_staleness(fb_empty, &meta, &present) != SIDEREON_STATUS_OK ||
        present ||
        sidereon_sourced_solution_broadcast_reason(fb_empty, &reason_kind, &unavail, &attempted,
                                                   &has_attempted) != SIDEREON_STATUS_OK ||
        reason_kind != SIDEREON_BROADCAST_REASON_KIND_PRECISE_UNAVAILABLE ||
        unavail != SIDEREON_SELECTION_STATUS_EMPTY_PRODUCT_SET || has_attempted) {
        sidereon_sourced_solution_free(fb_empty);
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: fallback empty provenance", 1);
        goto cleanup;
    }
    double pos_x[3];
    double clk_x;
    size_t used_x;
    if (sourced_pos_clock(fb_empty, pos_x, &clk_x, &used_x) ||
        f64_to_bits(pos_x[0]) != f64_to_bits(pos_b[0]) ||
        f64_to_bits(pos_x[1]) != f64_to_bits(pos_b[1]) ||
        f64_to_bits(pos_x[2]) != f64_to_bits(pos_b[2]) ||
        f64_to_bits(clk_x) != f64_to_bits(clk_b) || used_x != used_b) {
        sidereon_sourced_solution_free(fb_empty);
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: fallback empty not bit-exact vs broadcast-only", 1);
        goto cleanup;
    }
    sidereon_sourced_solution_free(fb_empty);

    /* Fallback with a 2026 precise product: it lies after the 2020 epoch, so the
     * staleness layer finds no prior product and drops to broadcast. */
    const SidereonSp3 *wrong_set[1] = {wrong};
    SidereonSourcedSolution *fb_wrong = NULL;
    if (sidereon_solve_with_fallback(wrong_set, 1, broadcast, &inputs, policy, &fb_wrong) !=
        SIDEREON_FALLBACK_STATUS_OK) {
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: fallback wrong-epoch solve", 1);
        goto cleanup;
    }
    if (sidereon_sourced_solution_source_kind(fb_wrong, &kind) != SIDEREON_STATUS_OK ||
        kind != SIDEREON_FIX_SOURCE_KIND_BROADCAST ||
        sidereon_sourced_solution_broadcast_reason(fb_wrong, &reason_kind, &unavail, &attempted,
                                                   &has_attempted) != SIDEREON_STATUS_OK ||
        reason_kind != SIDEREON_BROADCAST_REASON_KIND_PRECISE_UNAVAILABLE ||
        unavail != SIDEREON_SELECTION_STATUS_NO_PRIOR_PRODUCT) {
        sidereon_sourced_solution_free(fb_wrong);
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: fallback wrong-epoch provenance", 1);
        goto cleanup;
    }
    /* The PreciseUnavailable readout is a successful provenance surface, so it
     * must not overwrite the thread last-error message. Set a sentinel via a
     * deliberate null-arg failure, re-read the same provenance, and confirm the
     * sentinel survives. */
    {
        char sentinel[256] = {0};
        char after[256] = {0};
        SidereonBroadcastReasonKind junk_kind;
        if (sidereon_sourced_solution_broadcast_reason(NULL, &junk_kind, &unavail, &attempted,
                                                       &has_attempted) == SIDEREON_STATUS_OK) {
            sidereon_sourced_solution_free(fb_wrong);
            sidereon_spp_solution_free(broadcast_sol);
            (void)fail("broadcast: null-sol provenance should fail", 1);
            goto cleanup;
        }
        (void)sidereon_last_error_message(sentinel, sizeof(sentinel));
        if (sidereon_sourced_solution_broadcast_reason(fb_wrong, &reason_kind, &unavail, &attempted,
                                                       &has_attempted) != SIDEREON_STATUS_OK) {
            sidereon_sourced_solution_free(fb_wrong);
            sidereon_spp_solution_free(broadcast_sol);
            (void)fail("broadcast: wrong-epoch provenance re-read", 1);
            goto cleanup;
        }
        (void)sidereon_last_error_message(after, sizeof(after));
        if (strcmp(sentinel, after) != 0) {
            sidereon_sourced_solution_free(fb_wrong);
            sidereon_spp_solution_free(broadcast_sol);
            (void)fail("broadcast: successful provenance polluted last-error", 1);
            goto cleanup;
        }
    }
    if (sourced_pos_clock(fb_wrong, pos_x, &clk_x, &used_x) ||
        f64_to_bits(pos_x[0]) != f64_to_bits(pos_b[0]) ||
        f64_to_bits(clk_x) != f64_to_bits(clk_b)) {
        sidereon_sourced_solution_free(fb_wrong);
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: fallback wrong-epoch not bit-exact vs broadcast-only", 1);
        goto cleanup;
    }
    sidereon_sourced_solution_free(fb_wrong);

    /* Fallback with a within-cap prior-day precise product that still serves the
     * epoch: the degraded precise product is used (nonzero staleness, under cap),
     * not over-eagerly dropped to broadcast. */
    const SidereonSp3 *prior_set[1] = {prior};
    SidereonSourcedSolution *fb_prior = NULL;
    if (sidereon_solve_with_fallback(prior_set, 1, broadcast, &inputs, policy, &fb_prior) !=
        SIDEREON_FALLBACK_STATUS_OK) {
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: fallback degraded-precise solve", 1);
        goto cleanup;
    }
    if (sidereon_sourced_solution_source_kind(fb_prior, &kind) != SIDEREON_STATUS_OK ||
        kind != SIDEREON_FIX_SOURCE_KIND_PRECISE ||
        sidereon_sourced_solution_is_precise_exact(fb_prior, &is_exact) != SIDEREON_STATUS_OK ||
        is_exact ||
        sidereon_sourced_solution_staleness(fb_prior, &meta, &present) != SIDEREON_STATUS_OK ||
        !present || meta.kind != SIDEREON_DEGRADATION_KIND_NEAREST_PRIOR || !(meta.staleness_s > 0.0) ||
        !(meta.staleness_s < policy.max_staleness_s)) {
        sidereon_sourced_solution_free(fb_prior);
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: fallback degraded-precise provenance", 1);
        goto cleanup;
    }
    double pos_p[3];
    double clk_p;
    size_t used_p;
    if (sourced_pos_clock(fb_prior, pos_p, &clk_p, &used_p) || used_p < 5 ||
        !isfinite(pos_p[0]) || !isfinite(pos_p[1]) || !isfinite(pos_p[2])) {
        sidereon_sourced_solution_free(fb_prior);
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: fallback degraded-precise readout", 1);
        goto cleanup;
    }
    sidereon_sourced_solution_free(fb_prior);

    /* Typed argument gate: a null broadcast source is NullPointer, no handle. */
    SidereonSourcedSolution *null_fb = (SidereonSourcedSolution *)(uintptr_t)1;
    if (sidereon_solve_with_fallback(precise_set, 1, NULL, &inputs, policy, &null_fb) !=
            SIDEREON_FALLBACK_STATUS_NULL_POINTER ||
        null_fb != NULL) {
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: fallback null broadcast clears out_solution", 1);
        goto cleanup;
    }

    sidereon_spp_solution_free(broadcast_sol);
    printf("broadcast/fallback surface: broadcast-only SPP (%zu sats), precise-exact (delta %.3f m "
           "< %.1f), degraded-precise nearest-prior, and broadcast fallback bit-exact with typed "
           "reasons\n",
           used_b, dpe, BC_VS_PRECISE_POSITION_BOUND_M);
    rc = 0;

cleanup:
    sidereon_sp3_free(precise);
    sidereon_sp3_free(prior);
    sidereon_sp3_free(wrong);
    sidereon_broadcast_ephemeris_free(broadcast);
    return rc;
}

/* Exercise the multi-record TLE file parser: a 3-line named record, a bare
 * 2-line record, and a malformed (complete but non-initializing) record. Then
 * confirm a parsed satellite still drives the existing TLE/SGP4 surface by
 * producing finite look-angles. Returns 0 on success, non-zero on failure. */
static int exercise_tle_file_surface(void) {
    static const char ISS_L1[] =
        "1 25544U 98067A   18184.80969102  .00001614  00000-0  31745-4 0  9993";
    static const char ISS_L2[] =
        "2 25544  51.6414 295.8524 0003435 262.6267 204.2868 15.54005638121106";

    /* Record 1: a 3-line named ISS set. Record 2: the same element set as a
     * bare 2-line set (no name). Record 3: a complete (line 1, line 2) pair that
     * fails SGP4 initialization, so it must be skipped and counted. CRLF and a
     * blank line are mixed in to exercise the tolerant parser. */
    char text[1024];
    int n = snprintf(text, sizeof(text),
                     "ISS (ZARYA)\r\n%s\r\n%s\r\n"
                     "\r\n"
                     "%s\n%s\n"
                     "1 00000U 00000XYZ BADDATA\n2 00000 BADDATA\n",
                     ISS_L1, ISS_L2, ISS_L1, ISS_L2);
    if (n <= 0 || (size_t)n >= sizeof(text)) {
        return fail("tle file sample formatting", 1);
    }

    SidereonTleFile *file = NULL;
    if (sidereon_parse_tle_file((const uint8_t *)text, (size_t)n,
                                SIDEREON_TLE_OPS_MODE_IMPROVED, &file) != SIDEREON_STATUS_OK ||
        file == NULL) {
        return fail("sidereon_parse_tle_file", 1);
    }

    size_t count = 99;
    if (sidereon_tle_file_count(file, &count) != SIDEREON_STATUS_OK || count != 2) {
        sidereon_tle_file_free(file);
        return fail("sidereon_tle_file_count", 1);
    }

    size_t skipped = 99;
    if (sidereon_tle_file_skipped(file, &skipped) != SIDEREON_STATUS_OK || skipped != 1) {
        sidereon_tle_file_free(file);
        return fail("sidereon_tle_file_skipped", 1);
    }

    /* Query-then-fill the name of the first record. */
    size_t name_required = 0;
    if (sidereon_tle_file_name(file, 0, NULL, 0, &name_required) != SIDEREON_STATUS_OK ||
        name_required != strlen("ISS (ZARYA)") + 1) {
        sidereon_tle_file_free(file);
        return fail("sidereon_tle_file_name size query", 1);
    }
    char name[64];
    size_t name_required2 = 0;
    if (sidereon_tle_file_name(file, 0, name, sizeof(name), &name_required2) !=
            SIDEREON_STATUS_OK ||
        name_required2 != name_required || strcmp(name, "ISS (ZARYA)") != 0) {
        sidereon_tle_file_free(file);
        return fail("sidereon_tle_file_name full copy", 1);
    }

    /* The bare 2-line record carries an empty name. */
    char bare_name[8];
    size_t bare_required = 0;
    if (sidereon_tle_file_name(file, 1, bare_name, sizeof(bare_name), &bare_required) !=
            SIDEREON_STATUS_OK ||
        bare_required != 1 || bare_name[0] != '\0') {
        sidereon_tle_file_free(file);
        return fail("sidereon_tle_file_name bare record", 1);
    }

    /* Out-of-range index is rejected and leaves out_required cleared. */
    size_t oor_required = 99;
    if (sidereon_tle_file_name(file, 2, NULL, 0, &oor_required) != SIDEREON_STATUS_INVALID_ARGUMENT ||
        oor_required != 0) {
        sidereon_tle_file_free(file);
        return fail("sidereon_tle_file_name out-of-range index", 1);
    }

    /* The first record's satellite drives the existing TLE/SGP4 look-angle
     * surface. The handle is independent of the file. */
    SidereonTle *tle = NULL;
    if (sidereon_tle_file_satellite(file, 0, &tle) != SIDEREON_STATUS_OK || tle == NULL) {
        sidereon_tle_file_free(file);
        return fail("sidereon_tle_file_satellite", 1);
    }
    /* Freeing the file first proves the satellite handle is independent. */
    sidereon_tle_file_free(file);

    SidereonGroundStation station = {40.0, -75.0, 0.0};
    int64_t epoch_unix_us = 1530645960000000LL; /* ~2018-07-03T19:26:00Z, near epoch */
    SidereonLookAngles *look = NULL;
    if (sidereon_tle_look_angles(tle, &station, &epoch_unix_us, 1, &look) != SIDEREON_STATUS_OK ||
        look == NULL) {
        sidereon_tle_free(tle);
        return fail("sidereon_tle_look_angles on parsed satellite", 1);
    }

    size_t look_count = 0;
    if (sidereon_look_angles_epoch_count(look, &look_count) != SIDEREON_STATUS_OK ||
        look_count != 1) {
        sidereon_look_angles_free(look);
        sidereon_tle_free(tle);
        return fail("sidereon_look_angles_epoch_count on parsed satellite", 1);
    }

    SidereonLookAngle angle = {0.0, 0.0, 0.0};
    size_t written = 0, required = 0;
    if (sidereon_look_angles_values(look, &angle, 1, &written, &required) != SIDEREON_STATUS_OK ||
        written != 1 || required != 1 || !isfinite(angle.azimuth_deg) ||
        !isfinite(angle.elevation_deg) || !isfinite(angle.range_km) || !(angle.range_km > 0.0)) {
        sidereon_look_angles_free(look);
        sidereon_tle_free(tle);
        return fail("sidereon_look_angles_values on parsed satellite", 1);
    }

    sidereon_look_angles_free(look);
    sidereon_tle_free(tle);

    printf("tle file: count=%zu skipped=%zu name='%s' look=[az %.3f, el %.3f, range %.3f km]\n",
           count, skipped, name, angle.azimuth_deg, angle.elevation_deg, angle.range_km);
    return 0;
}

int main(int argc, char **argv) {
    if (argc < 14) {
        fprintf(stderr,
                "usage: %s <%s> <%s> <%s> <%s> <%s> <%s> <esbc.crx> <esbc.rnx> <algo.crx> "
                "<algo.rnx> <%s> <%s> <%s>\n",
                argv[0], SPP_SP3_FILE, SP3_SURFACE_FILE, PPP_SP3_FILE, SPK_KERNEL_FILE, ANTEX_FILE,
                IONEX_FILE, BC_NAV_FILE, BC_PRECISE_SP3_FILE, BC_PRIOR_SP3_FILE);
        return 2;
    }

    sidereon_sp3_free(NULL);
    sidereon_spk_free(NULL);
    sidereon_sp3_merge_report_free(NULL);
    sidereon_spp_solution_free(NULL);
    sidereon_rtk_float_solution_free(NULL);
    sidereon_rtk_fixed_solution_free(NULL);
    sidereon_ppp_float_solution_free(NULL);
    sidereon_ppp_fixed_solution_free(NULL);
    sidereon_tle_free(NULL);
    sidereon_tle_file_free(NULL);
    sidereon_tle_propagation_free(NULL);
    sidereon_look_angles_free(NULL);
    sidereon_pass_list_free(NULL);
    sidereon_tle_batch_propagation_free(NULL);
    sidereon_tle_batch_look_angles_free(NULL);
    sidereon_ephemeris_free(NULL);
    sidereon_broadcast_ephemeris_free(NULL);
    sidereon_sourced_solution_free(NULL);

    /* Version and status-string accessors agree with the compile-time macros. */
    uint32_t v_major = 99, v_minor = 99, v_patch = 99;
    sidereon_version(&v_major, &v_minor, &v_patch);
    if (v_major != SIDEREON_VERSION_MAJOR || v_minor != SIDEREON_VERSION_MINOR ||
        v_patch != SIDEREON_VERSION_PATCH ||
        strcmp(sidereon_version_string(), SIDEREON_VERSION_STRING) != 0) {
        return fail("sidereon_version accessors", 1);
    }
    sidereon_version(NULL, NULL, NULL); /* all-NULL is a no-op, not a crash. */
    if (strcmp(sidereon_status_message(SIDEREON_STATUS_OK), "ok") != 0 ||
        strcmp(sidereon_status_message(SIDEREON_STATUS_NULL_POINTER),
               "null pointer argument") != 0) {
        return fail("sidereon_status_message", 1);
    }

    int tle_file_status = exercise_tle_file_surface();
    if (tle_file_status != 0) {
        return tle_file_status;
    }

    size_t sp3_len = 0;
    uint8_t *sp3_bytes = read_file(argv[1], &sp3_len);
    if (sp3_bytes == NULL) {
        fprintf(stderr, "FAIL: could not read SP3 file: %s\n", argv[1]);
        return 2;
    }

    SidereonSp3 *sp3 = NULL;
    if (sidereon_sp3_load(sp3_bytes, sp3_len, &sp3) != SIDEREON_STATUS_OK) {
        free(sp3_bytes);
        return fail("sidereon_sp3_load", 1);
    }

    SidereonSp3 *null_data_sp3 = (SidereonSp3 *)(uintptr_t)1;
    if (sidereon_sp3_load(NULL, sp3_len, &null_data_sp3) != SIDEREON_STATUS_NULL_POINTER ||
        null_data_sp3 != NULL) {
        free(sp3_bytes);
        sidereon_sp3_free(sp3);
        return fail("sidereon_sp3_load null data clears out_sp3", 1);
    }

    SidereonSp3 *oversized_sp3 = (SidereonSp3 *)(uintptr_t)1;
    if (sidereon_sp3_load(sp3_bytes, (size_t)-1, &oversized_sp3) != SIDEREON_STATUS_INVALID_ARGUMENT ||
        oversized_sp3 != NULL) {
        free(sp3_bytes);
        sidereon_sp3_free(sp3);
        return fail("sidereon_sp3_load oversized len clears out_sp3", 1);
    }
    free(sp3_bytes);

    size_t epoch_count = 123;
    if (sidereon_sp3_epoch_count(NULL, &epoch_count) != SIDEREON_STATUS_NULL_POINTER ||
        epoch_count != 0) {
        sidereon_sp3_free(sp3);
        return fail("sidereon_sp3_epoch_count null sp3 clears out_count", 1);
    }
    if (sidereon_sp3_epoch_count(sp3, NULL) != SIDEREON_STATUS_NULL_POINTER) {
        sidereon_sp3_free(sp3);
        return fail("sidereon_sp3_epoch_count null out_count", 1);
    }
    if (sidereon_sp3_epoch_count(sp3, &epoch_count) != SIDEREON_STATUS_OK) {
        sidereon_sp3_free(sp3);
        return fail("sidereon_sp3_epoch_count", 1);
    }
    printf("loaded SP3: %zu epochs\n", epoch_count);

    int sp3_surface_status = exercise_sp3_surface(argv[2]);
    if (sp3_surface_status != 0) {
        sidereon_sp3_free(sp3);
        return sp3_surface_status;
    }

    int rtk_status = exercise_rtk_surface();
    if (rtk_status != 0) {
        sidereon_sp3_free(sp3);
        return rtk_status;
    }

    int ppp_status = exercise_ppp_surface(argv[3]);
    if (ppp_status != 0) {
        sidereon_sp3_free(sp3);
        return ppp_status;
    }

    int spk_status = exercise_spk_surface(argv[4]);
    if (spk_status != 0) {
        sidereon_sp3_free(sp3);
        return spk_status;
    }

    int propagation_status = exercise_propagation_surface();
    if (propagation_status != 0) {
        sidereon_sp3_free(sp3);
        return propagation_status;
    }

    int dop_status = exercise_dop_surface();
    if (dop_status != 0) {
        sidereon_sp3_free(sp3);
        return dop_status;
    }

    int antex_status = exercise_antex_surface(argv[5]);
    if (antex_status != 0) {
        sidereon_sp3_free(sp3);
        return antex_status;
    }

    /* The velocity fixture is synthesized against this same SP3 (argv[1]). */
    int velocity_status = exercise_velocity_surface(sp3);
    if (velocity_status != 0) {
        sidereon_sp3_free(sp3);
        return velocity_status;
    }

    int iono_status = exercise_iono_surface(argv[6]);
    if (iono_status != 0) {
        sidereon_sp3_free(sp3);
        return iono_status;
    }

    int rinex_status = exercise_rinex_surface(argv[7], argv[8], argv[9], argv[10]);
    if (rinex_status != 0) {
        sidereon_sp3_free(sp3);
        return rinex_status;
    }

    int timescale_status = exercise_timescale_surface();
    if (timescale_status != 0) {
        sidereon_sp3_free(sp3);
        return timescale_status;
    }

    int constellation_status = exercise_constellation_surface();
    if (constellation_status != 0) {
        sidereon_sp3_free(sp3);
        return constellation_status;
    }

    int staleness_status = exercise_staleness_surface(sp3, argv[6]);
    if (staleness_status != 0) {
        sidereon_sp3_free(sp3);
        return staleness_status;
    }

    /* Broadcast SPP + precise/broadcast fallback. The 2026 "wrong epoch" precise
     * product is the SP3 surface fixture (argv[2]). */
    int broadcast_status =
        exercise_broadcast_fallback_surface(argv[11], argv[12], argv[13], argv[2]);
    if (broadcast_status != 0) {
        sidereon_sp3_free(sp3);
        return broadcast_status;
    }

    SidereonObservation observations[SPP_OBS_COUNT];
    for (size_t i = 0; i < SPP_OBS_COUNT; i++) {
        observations[i].sat_id = SPP_SAT_IDS[i];
        observations[i].pseudorange_m = bits_to_f64(SPP_PSEUDORANGE_BITS[i]);
    }

    SidereonSppInputs inputs;
    inputs.observations = observations;
    inputs.observation_count = SPP_OBS_COUNT;
    inputs.t_rx_j2000_s = bits_to_f64(SPP_T_RX_J2000_S_BITS);
    inputs.t_rx_second_of_day_s = bits_to_f64(SPP_T_RX_SOD_S_BITS);
    inputs.day_of_year = bits_to_f64(SPP_DOY_BITS);
    for (int i = 0; i < 4; i++) {
        inputs.initial_guess[i] = bits_to_f64(SPP_INITIAL_GUESS_BITS[i]);
        inputs.klobuchar_alpha[i] = bits_to_f64(SPP_KLOB_ALPHA_BITS[i]);
        inputs.klobuchar_beta[i] = bits_to_f64(SPP_KLOB_BETA_BITS[i]);
    }
    /* L0_minimal: geometry + clock + Sagnac only, no iono, no tropo. */
    inputs.ionosphere = false;
    inputs.troposphere = false;
    inputs.pressure_hpa = bits_to_f64(SPP_PRESSURE_HPA_BITS);
    inputs.temperature_k = bits_to_f64(SPP_TEMPERATURE_K_BITS);
    inputs.relative_humidity = bits_to_f64(SPP_RELATIVE_HUMIDITY_BITS);
    inputs.with_geodetic = true;

    SidereonSppSolution *null_sp3_sol = (SidereonSppSolution *)(uintptr_t)1;
    if (sidereon_solve_spp(NULL, &inputs, &null_sp3_sol) != SIDEREON_STATUS_NULL_POINTER ||
        null_sp3_sol != NULL) {
        sidereon_sp3_free(sp3);
        return fail("sidereon_solve_spp null sp3 clears out_solution", 1);
    }

    SidereonSppInputs empty_inputs = inputs;
    empty_inputs.observations = NULL;
    empty_inputs.observation_count = 0;
    SidereonSppSolution *empty_sol = (SidereonSppSolution *)(uintptr_t)1;
    SidereonStatus empty_status = sidereon_solve_spp(sp3, &empty_inputs, &empty_sol);
    if (empty_status != SIDEREON_STATUS_SOLVE || empty_sol != NULL) {
        sidereon_sp3_free(sp3);
        return fail("sidereon_solve_spp empty observations clears out_solution", 1);
    }

    SidereonSppInputs oversized_inputs = inputs;
    oversized_inputs.observation_count = (size_t)-1;
    SidereonSppSolution *oversized_sol = (SidereonSppSolution *)(uintptr_t)1;
    SidereonStatus oversized_status = sidereon_solve_spp(sp3, &oversized_inputs, &oversized_sol);
    if (oversized_status != SIDEREON_STATUS_INVALID_ARGUMENT || oversized_sol != NULL) {
        sidereon_sp3_free(sp3);
        return fail("sidereon_solve_spp oversized observation_count clears out_solution", 1);
    }

    char unterminated_sat_id[17];
    memset(unterminated_sat_id, 'G', sizeof(unterminated_sat_id));
    SidereonObservation unterminated_observation = observations[0];
    unterminated_observation.sat_id = unterminated_sat_id;
    SidereonSppInputs unterminated_inputs = inputs;
    unterminated_inputs.observations = &unterminated_observation;
    unterminated_inputs.observation_count = 1;
    SidereonSppSolution *unterminated_sol = (SidereonSppSolution *)(uintptr_t)1;
    SidereonStatus unterminated_status =
        sidereon_solve_spp(sp3, &unterminated_inputs, &unterminated_sol);
    if (unterminated_status != SIDEREON_STATUS_INVALID_ARGUMENT || unterminated_sol != NULL) {
        sidereon_sp3_free(sp3);
        return fail("sidereon_solve_spp unterminated sat_id clears out_solution", 1);
    }

    SidereonSppSolution *sol = NULL;
    if (sidereon_solve_spp(sp3, &inputs, &sol) != SIDEREON_STATUS_OK) {
        sidereon_sp3_free(sp3);
        return fail("sidereon_solve_spp", 1);
    }

    double null_position[3] = {1.0, 2.0, 3.0};
    if (sidereon_spp_solution_position(NULL, null_position, 3) != SIDEREON_STATUS_NULL_POINTER ||
        null_position[0] != 0.0 || null_position[1] != 0.0 || null_position[2] != 0.0) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_position null solution clears out_xyz", 1);
    }

    double short_position[3] = {1.0, 2.0, 3.0};
    if (sidereon_spp_solution_position(sol, short_position, 2) != SIDEREON_STATUS_INVALID_ARGUMENT ||
        short_position[0] != 0.0 || short_position[1] != 0.0 || short_position[2] != 3.0) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_position short len clears writable prefix", 1);
    }

    double oversized_position[3] = {1.0, 2.0, 3.0};
    if (sidereon_spp_solution_position(sol, oversized_position, (size_t)-1) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        oversized_position[0] != 0.0 || oversized_position[1] != 0.0 ||
        oversized_position[2] != 0.0) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_position oversized len clears out_xyz", 1);
    }

    double position[3];
    if (sidereon_spp_solution_position(sol, position, 3) != SIDEREON_STATUS_OK) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_position", 1);
    }

    SidereonGeodetic null_geodetic = {1.0, 2.0, 3.0};
    bool null_geodetic_present = true;
    if (sidereon_spp_solution_geodetic(NULL, &null_geodetic, &null_geodetic_present) !=
            SIDEREON_STATUS_NULL_POINTER ||
        null_geodetic.lat_rad != 0.0 || null_geodetic.lon_rad != 0.0 ||
        null_geodetic.height_m != 0.0 || null_geodetic_present) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_geodetic null solution clears outputs", 1);
    }
    if (sidereon_spp_solution_geodetic(sol, NULL, &null_geodetic_present) !=
        SIDEREON_STATUS_NULL_POINTER) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_geodetic null out_geodetic", 1);
    }
    SidereonGeodetic geodetic = {0.0, 0.0, 0.0};
    bool geodetic_present = false;
    if (sidereon_spp_solution_geodetic(sol, &geodetic, &geodetic_present) !=
            SIDEREON_STATUS_OK ||
        !geodetic_present || !isfinite(geodetic.lat_rad) || !isfinite(geodetic.lon_rad) ||
        !isfinite(geodetic.height_m)) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_geodetic", 1);
    }

    double rx_clock_s = 123.0;
    if (sidereon_spp_solution_rx_clock_s(NULL, &rx_clock_s) != SIDEREON_STATUS_NULL_POINTER ||
        rx_clock_s != 0.0) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_rx_clock_s null solution clears out_rx_clock_s", 1);
    }
    if (sidereon_spp_solution_rx_clock_s(sol, NULL) != SIDEREON_STATUS_NULL_POINTER) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_rx_clock_s null out_rx_clock_s", 1);
    }
    if (sidereon_spp_solution_rx_clock_s(sol, &rx_clock_s) != SIDEREON_STATUS_OK) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_rx_clock_s", 1);
    }

    size_t used = 123;
    if (sidereon_spp_solution_used_sat_count(NULL, &used) != SIDEREON_STATUS_NULL_POINTER ||
        used != 0) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_used_sat_count null solution clears out_count", 1);
    }
    if (sidereon_spp_solution_used_sat_count(sol, NULL) != SIDEREON_STATUS_NULL_POINTER) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_used_sat_count null out_count", 1);
    }
    if (sidereon_spp_solution_used_sat_count(sol, &used) != SIDEREON_STATUS_OK) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_used_sat_count", 1);
    }
    if (used != SPP_USED_SAT_COUNT) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_used_sat_count fixture count", 1);
    }

    size_t null_used_token_written = 123;
    size_t null_used_token_required = 123;
    if (sidereon_spp_solution_used_sat_ids(
            NULL, NULL, 0, &null_used_token_written, &null_used_token_required) !=
            SIDEREON_STATUS_NULL_POINTER ||
        null_used_token_written != 0 || null_used_token_required != 0) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_used_sat_ids null solution clears counts", 1);
    }

    size_t query_used_token_written = 123;
    size_t query_used_token_required = 123;
    if (sidereon_spp_solution_used_sat_ids(
            sol, NULL, 0, &query_used_token_written, &query_used_token_required) !=
            SIDEREON_STATUS_OK ||
        query_used_token_written != 0 || query_used_token_required != used) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_used_sat_ids size query", 1);
    }

    SidereonSatelliteToken short_used_tokens[1];
    size_t short_used_token_written = 123;
    size_t short_used_token_required = 123;
    if (sidereon_spp_solution_used_sat_ids(
            sol, short_used_tokens, 1, &short_used_token_written, &short_used_token_required) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        short_used_token_written != 0 || short_used_token_required != used) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_used_sat_ids short buffer", 1);
    }

    SidereonSatelliteToken used_tokens[SPP_USED_SAT_COUNT];
    size_t full_used_token_written = 123;
    size_t full_used_token_required = 123;
    if (sidereon_spp_solution_used_sat_ids(
            sol, used_tokens, SPP_USED_SAT_COUNT, &full_used_token_written,
            &full_used_token_required) != SIDEREON_STATUS_OK ||
        full_used_token_written != used || full_used_token_required != used) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_used_sat_ids full copy", 1);
    }
    for (size_t i = 0; i < full_used_token_written; i++) {
        if (!token_equals(&used_tokens[i], SPP_USED_SAT_IDS[i])) {
            sidereon_spp_solution_free(sol);
            sidereon_sp3_free(sp3);
            return fail("sidereon_spp_solution_used_sat_ids token order", 1);
        }
    }

    size_t rejected_written = 123;
    size_t rejected_required = 123;
    if (sidereon_spp_solution_rejected_sats(sol, NULL, 0, &rejected_written, &rejected_required) !=
            SIDEREON_STATUS_OK ||
        rejected_written != 0 || rejected_required != SPP_REJECTED_SAT_COUNT) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_rejected_sats size query", 1);
    }
    SidereonSppRejectedSat rejected_sats[SPP_REJECTED_SAT_COUNT];
    rejected_written = 123;
    rejected_required = 123;
    if (sidereon_spp_solution_rejected_sats(
            sol, rejected_sats, SPP_REJECTED_SAT_COUNT, &rejected_written, &rejected_required) !=
            SIDEREON_STATUS_OK ||
        rejected_written != SPP_REJECTED_SAT_COUNT ||
        rejected_required != SPP_REJECTED_SAT_COUNT) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_rejected_sats full copy", 1);
    }
    for (size_t i = 0; i < rejected_written; i++) {
        if (!token_equals(&rejected_sats[i].sat_id, SPP_REJECTED_SAT_IDS[i]) ||
            !rejection_reason_equals(rejected_sats[i].reason, SPP_REJECTED_SAT_REASONS[i])) {
            sidereon_spp_solution_free(sol);
            sidereon_sp3_free(sp3);
            return fail("sidereon_spp_solution_rejected_sats order", 1);
        }
    }

    size_t clock_written = 123;
    size_t clock_required = 123;
    if (sidereon_spp_solution_system_clocks(sol, NULL, 0, &clock_written, &clock_required) !=
            SIDEREON_STATUS_OK ||
        clock_written != 0 || clock_required != 1) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_system_clocks size query", 1);
    }
    SidereonSppSystemClock system_clocks[4];
    clock_written = 123;
    clock_required = 123;
    if (sidereon_spp_solution_system_clocks(
            sol, system_clocks, 4, &clock_written, &clock_required) != SIDEREON_STATUS_OK ||
        clock_written != 1 || clock_required != 1 ||
        system_clocks[0].system != SIDEREON_GNSS_SYSTEM_GPS ||
        fabs(system_clocks[0].rx_clock_s - rx_clock_s) >= 1.0e-15) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_system_clocks full copy", 1);
    }

    SidereonSppMetadata null_metadata;
    if (sidereon_spp_solution_metadata(NULL, &null_metadata) != SIDEREON_STATUS_NULL_POINTER ||
        null_metadata.used_count != 0 || null_metadata.system_count != 0 ||
        null_metadata.raim_checkable) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_metadata null solution clears metadata", 1);
    }
    SidereonSppMetadata metadata;
    if (sidereon_spp_solution_metadata(sol, &metadata) != SIDEREON_STATUS_OK ||
        metadata.used_count != used || metadata.system_count != 1 || !metadata.converged ||
        metadata.iterations == 0 || metadata.ionosphere_applied ||
        metadata.troposphere_applied || metadata.outer_iterations != 0 ||
        metadata.has_final_robust_scale_m || metadata.redundancy != (int64_t)(used - 4) ||
        !metadata.raim_checkable ||
        metadata.geometry_quality.tier != SIDEREON_OBSERVABILITY_TIER_NOMINAL ||
        metadata.geometry_quality.redundancy != (int32_t)metadata.redundancy ||
        metadata.geometry_quality.rank < 4 ||
        !isfinite(metadata.geometry_quality.condition_number) ||
        !isfinite(metadata.geometry_quality.gdop) || metadata.geometry_quality.gdop <= 0.0 ||
        !metadata.geometry_quality.raim_checkable ||
        !metadata.geometry_quality.covariance_validated ||
        metadata.status > SIDEREON_SPP_SOLVE_STATUS_MAX_EVALUATIONS) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_metadata", 1);
    }

    double residual = 0.0;
    size_t residual_written = 123;
    size_t residual_required = 123;
    if (sidereon_spp_solution_residuals(
            sol, &residual, (size_t)-1, &residual_written, &residual_required) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        residual_written != 0 || residual_required != used) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_residuals oversized len reports required", 1);
    }

    size_t null_residual_written = 123;
    size_t null_residual_required = 123;
    if (sidereon_spp_solution_residuals(
            NULL, &residual, 1, &null_residual_written, &null_residual_required) !=
            SIDEREON_STATUS_NULL_POINTER ||
        null_residual_written != 0 || null_residual_required != 0) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_residuals null solution clears counts", 1);
    }

    size_t query_residual_written = 123;
    size_t query_residual_required = 123;
    if (sidereon_spp_solution_residuals(
            sol, NULL, 0, &query_residual_written, &query_residual_required) !=
            SIDEREON_STATUS_OK ||
        query_residual_written != 0 || query_residual_required != used) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_residuals size query", 1);
    }

    double short_residuals[1] = {42.0};
    size_t short_residual_written = 123;
    size_t short_residual_required = 123;
    if (sidereon_spp_solution_residuals(
            sol, short_residuals, 1, &short_residual_written, &short_residual_required) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        short_residual_written != 0 || short_residual_required != used ||
        short_residuals[0] != 42.0) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_residuals short buffer copies nothing", 1);
    }

    double residuals[SPP_OBS_COUNT];
    size_t full_residual_written = 123;
    size_t full_residual_required = 123;
    if (sidereon_spp_solution_residuals(
            sol, residuals, SPP_OBS_COUNT, &full_residual_written, &full_residual_required) !=
            SIDEREON_STATUS_OK ||
        full_residual_written != used || full_residual_required != used) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_residuals full copy", 1);
    }
    for (size_t i = 0; i < full_residual_written; i++) {
        if (!isfinite(residuals[i])) {
            sidereon_spp_solution_free(sol);
            sidereon_sp3_free(sp3);
            return fail("sidereon_spp_solution_residuals finite values", 1);
        }
    }

    SidereonDop null_dop = {1.0, 2.0, 3.0, 4.0, 5.0};
    if (sidereon_spp_solution_dop(NULL, &null_dop) != SIDEREON_STATUS_NULL_POINTER ||
        null_dop.gdop != 0.0 || null_dop.pdop != 0.0 || null_dop.hdop != 0.0 ||
        null_dop.vdop != 0.0 || null_dop.tdop != 0.0) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_dop null solution clears out_dop", 1);
    }

    SidereonDop dop;
    int have_dop = sidereon_spp_solution_dop(sol, &dop) == SIDEREON_STATUS_OK;

    SidereonObservation observations_with_reject[SPP_OBS_COUNT + 1];
    for (size_t i = 0; i < SPP_OBS_COUNT; i++) {
        observations_with_reject[i] = observations[i];
    }
    /* A valid satellite token that is absent from this (GPS/GLONASS/Galileo) SP3
     * product, so the solve rejects it for NoEphemeris rather than failing to
     * parse the token. PRNs outside a constellation's documented range (e.g.
     * "G99") are not valid tokens in the engine and would be rejected at input
     * validation. The solve sorts observations by (system, PRN), so a QZSS PRN
     * sorts after every GPS satellite and lands last in the rejected list. */
    observations_with_reject[SPP_OBS_COUNT].sat_id = "J01";
    observations_with_reject[SPP_OBS_COUNT].pseudorange_m = observations[0].pseudorange_m;
    SidereonSppInputs reject_inputs = inputs;
    reject_inputs.observations = observations_with_reject;
    reject_inputs.observation_count = SPP_OBS_COUNT + 1;
    SidereonSppSolution *reject_sol = NULL;
    if (sidereon_solve_spp(sp3, &reject_inputs, &reject_sol) != SIDEREON_STATUS_OK) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_solve_spp with rejected satellite", 1);
    }
    SidereonSppRejectedSat rejected[SPP_REJECTED_SAT_COUNT + 1];
    rejected_written = 123;
    rejected_required = 123;
    if (sidereon_spp_solution_rejected_sats(
            reject_sol, rejected, SPP_REJECTED_SAT_COUNT + 1, &rejected_written,
            &rejected_required) !=
            SIDEREON_STATUS_OK ||
        rejected_written != SPP_REJECTED_SAT_COUNT + 1 ||
        rejected_required != SPP_REJECTED_SAT_COUNT + 1 ||
        !token_equals(&rejected[SPP_REJECTED_SAT_COUNT].sat_id, "J01") ||
        rejected[SPP_REJECTED_SAT_COUNT].reason != SIDEREON_SPP_REJECTION_REASON_NO_EPHEMERIS) {
        sidereon_spp_solution_free(reject_sol);
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_rejected_sats full copy", 1);
    }
    sidereon_spp_solution_free(reject_sol);

    if (sidereon_spp_inputs_v2_init(NULL) != SIDEREON_STATUS_NULL_POINTER) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_inputs_v2_init null out_inputs", 1);
    }

    SidereonSppInputsV2 v2_inputs;
    if (sidereon_spp_inputs_v2_init(&v2_inputs) != SIDEREON_STATUS_OK) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_inputs_v2_init", 1);
    }
    v2_inputs.base = inputs;
    v2_inputs.beidou_klobuchar_enabled = true;
    for (int i = 0; i < 4; i++) {
        v2_inputs.beidou_klobuchar_alpha[i] = inputs.klobuchar_alpha[i];
        v2_inputs.beidou_klobuchar_beta[i] = inputs.klobuchar_beta[i];
    }
    v2_inputs.robust_enabled = true;
    /* max_outer counts the initial warm-start solve, so the reweighting loop runs
     * max_outer - 1 times. Use 2 to drive exactly one reweighted solve, which
     * reports outer_iterations == 1 and a finite final robust scale. */
    v2_inputs.robust.max_outer = 2;
    v2_inputs.policy.validation.max_pdop_enabled = true;
    v2_inputs.policy.validation.max_pdop = 9999.0;
    v2_inputs.policy.coarse_search_enabled = true;
    v2_inputs.policy.coarse_search_seeds = 1;

    SidereonSppSolution *v2_sol = NULL;
    if (sidereon_solve_spp_v2(sp3, &v2_inputs, &v2_sol) != SIDEREON_STATUS_OK) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_solve_spp_v2", 1);
    }
    SidereonSppMetadata v2_metadata;
    if (sidereon_spp_solution_metadata(v2_sol, &v2_metadata) != SIDEREON_STATUS_OK ||
        v2_metadata.outer_iterations != 1 || !v2_metadata.has_final_robust_scale_m ||
        !isfinite(v2_metadata.final_robust_scale_m)) {
        sidereon_spp_solution_free(v2_sol);
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_solve_spp_v2 robust metadata", 1);
    }
    sidereon_spp_solution_free(v2_sol);

    SidereonSppInputsV2 strict_v2_inputs = v2_inputs;
    strict_v2_inputs.robust_enabled = false;
    strict_v2_inputs.policy.coarse_search_enabled = false;
    strict_v2_inputs.policy.validation.max_pdop = 0.1;
    SidereonSppSolution *strict_v2_sol = (SidereonSppSolution *)(uintptr_t)1;
    if (sidereon_solve_spp_v2(sp3, &strict_v2_inputs, &strict_v2_sol) !=
            SIDEREON_STATUS_SOLVE ||
        strict_v2_sol != NULL) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_solve_spp_v2 policy failure clears out_solution", 1);
    }

    printf("position = [%.6f, %.6f, %.6f] m\n", position[0], position[1], position[2]);
    printf("rx_clock_s = %.9e\n", rx_clock_s);
    printf("used_sats = %zu\n", used);
    if (have_dop) {
        printf("gdop = %.4f, pdop = %.4f, hdop = %.4f, vdop = %.4f, tdop = %.4f\n",
               dop.gdop, dop.pdop, dop.hdop, dop.vdop, dop.tdop);
    }

    double expected[3];
    for (int i = 0; i < 3; i++) {
        expected[i] = bits_to_f64(SPP_EXPECTED_X_BITS[i]);
    }
    double dx = position[0] - expected[0];
    double dy = position[1] - expected[1];
    double dz = position[2] - expected[2];
    double dpos = sqrt(dx * dx + dy * dy + dz * dz);

    double expected_clock = bits_to_f64(SPP_EXPECTED_RX_CLOCK_S_BITS);
    double dclock = fabs(rx_clock_s - expected_clock);

    printf("dpos vs reference = %.3e m (bound %.0e)\n", dpos, SPP_AGREEMENT_BOUND_M);
    printf("dclock vs reference = %.3e s\n", dclock);

    int glonass_status = exercise_spp_glonass_channels(sp3);
    if (glonass_status != 0) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return glonass_status;
    }

    sidereon_spp_solution_free(sol);
    sidereon_sp3_free(sp3);

    if (!(dpos < SPP_AGREEMENT_BOUND_M)) {
        fprintf(stderr, "FAIL: position disagrees with crate reference by %.3e m\n", dpos);
        return 1;
    }
    if (!(dclock < 1.0e-9)) {
        fprintf(stderr, "FAIL: clock disagrees with crate reference by %.3e s\n", dclock);
        return 1;
    }

    printf("OK: binding reproduces the crate SPP reference\n");
    return 0;
}
