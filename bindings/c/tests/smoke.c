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
#include "engine_pins_fixture.h"
#include "smoke_a_sp3_pins.h"
#include "smoke_a_rtk_pins.h"
#include "smoke_a_ppp_pins.h"
#include "smoke_a_spk_pins.h"
#include "smoke_a_prop_pins.h"
#include "smoke_a_glonass_pins.h"
#include "smoke_a_ionex_pins.h"
#include "smoke_a_tec_grid_pins.h"
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

/* Print the binding's last-error message, then return the given code. Use it
 * only when the call under test returned an error status: the last error is
 * sticky, so after a successful call it still holds the message of whatever
 * call failed before, often a refusal the smoke provoked on purpose. */
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

/* Report a value that differs from the fixture after every call succeeded.
 * No call failed, so the sticky last error is not printed. */
static int fail_value(const char *context, int code) {
    fprintf(stderr, "FAIL: %s: returned value differs from the fixture\n", context);
    return code;
}

static int last_error_contains(const char *needle) {
    char msg[512];
    size_t written = sidereon_last_error_message(msg, sizeof(msg));
    return written > 0 && strstr(msg, needle) != NULL;
}

static int ppp_auto_init_detail_matches(const char *operation, const char *kind, bool code_seed);

/* Whether the binding's last error is exactly `expected`. */
static int sa_last_error_equals(const char *expected) {
    size_t needed = sidereon_last_error_message(NULL, 0);
    char *msg = (char *)malloc(needed + 1);
    if (msg == NULL) {
        return 0;
    }
    sidereon_last_error_message(msg, needed + 1);
    int equal = strcmp(msg, expected) == 0;
    if (!equal) {
        fprintf(stderr, "last error \"%s\", expected \"%s\"\n", msg, expected);
    }
    free(msg);
    return equal;
}

/* The engine variant each C code stands for, as the binding maps them
 * (src/lib.rs rtk_solve_status_to_c, rtk_integer_status_to_c,
 * geometry_quality_to_c), so a C value compares with a generated variant
 * name. */
static const char *sa_rtk_solve_status_name(SidereonRtkSolveStatus status) {
    switch (status) {
    case SIDEREON_RTK_SOLVE_STATUS_STATE_TOLERANCE:
        return "StateTolerance";
    case SIDEREON_RTK_SOLVE_STATUS_MAX_ITERATIONS:
        return "MaxIterations";
    }
    return "";
}

static const char *sa_rtk_integer_status_name(SidereonRtkIntegerStatus status) {
    switch (status) {
    case SIDEREON_RTK_INTEGER_STATUS_FIXED:
        return "Fixed";
    case SIDEREON_RTK_INTEGER_STATUS_NOT_FIXED:
        return "NotFixed";
    }
    return "";
}

static const char *sa_ppp_solve_status_name(SidereonPppSolveStatus status) {
    switch (status) {
    case SIDEREON_PPP_SOLVE_STATUS_STATE_TOLERANCE:
        return "StateTolerance";
    case SIDEREON_PPP_SOLVE_STATUS_MAX_ITERATIONS:
        return "MaxIterations";
    }
    return "";
}

static const char *sa_observability_tier_name(SidereonObservabilityTier tier) {
    switch (tier) {
    case SIDEREON_OBSERVABILITY_TIER_RANK_DEFICIENT:
        return "RankDeficient";
    case SIDEREON_OBSERVABILITY_TIER_ZERO_REDUNDANCY:
        return "ZeroRedundancy";
    case SIDEREON_OBSERVABILITY_TIER_WEAK:
        return "Weak";
    case SIDEREON_OBSERVABILITY_TIER_NOMINAL:
        return "Nominal";
    }
    return "";
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

/* An optional merge-agreement value: when the engine reports none the
 * binding writes NaN (binding contract), otherwise the value is compared bit
 * for bit with sidereon-core's. */
static int sa_optional_bits_match(bool present, double value, bool expected_present,
                               uint64_t expected_bits) {
    if (present != expected_present) {
        return 0;
    }
    return expected_present ? f64_to_bits(value) == expected_bits : isnan(value);
}

/* The first per-epoch agreement row against smoke_a_sp3: `which` 0 is the
 * single-source merge, 1 the two-source merge. */
static int sa_check_sp3_epoch_agreement(const SidereonSp3EpochAgreement *row, int which) {
    int ok = which == 0
                 ? f64_to_bits(row->epoch_j2000_seconds) == SMOKE_A_SP3_SINGLE_AGREEMENT0_EPOCH_BITS &&
                       row->satellites == SMOKE_A_SP3_SINGLE_AGREEMENT0_SATELLITES &&
                       sa_optional_bits_match(row->position_rms_present, row->position_rms_m,
                                           SMOKE_A_SP3_SINGLE_AGREEMENT0_POSITION_RMS_PRESENT,
                                           SMOKE_A_SP3_SINGLE_AGREEMENT0_POSITION_RMS_BITS) &&
                       sa_optional_bits_match(row->position_max_present, row->position_max_m,
                                           SMOKE_A_SP3_SINGLE_AGREEMENT0_POSITION_MAX_PRESENT,
                                           SMOKE_A_SP3_SINGLE_AGREEMENT0_POSITION_MAX_BITS) &&
                       sa_optional_bits_match(row->clock_rms_present, row->clock_rms_s,
                                           SMOKE_A_SP3_SINGLE_AGREEMENT0_CLOCK_RMS_PRESENT,
                                           SMOKE_A_SP3_SINGLE_AGREEMENT0_CLOCK_RMS_BITS) &&
                       sa_optional_bits_match(row->clock_max_present, row->clock_max_s,
                                           SMOKE_A_SP3_SINGLE_AGREEMENT0_CLOCK_MAX_PRESENT,
                                           SMOKE_A_SP3_SINGLE_AGREEMENT0_CLOCK_MAX_BITS)
                 : f64_to_bits(row->epoch_j2000_seconds) == SMOKE_A_SP3_DOUBLE_AGREEMENT0_EPOCH_BITS &&
                       row->satellites == SMOKE_A_SP3_DOUBLE_AGREEMENT0_SATELLITES &&
                       sa_optional_bits_match(row->position_rms_present, row->position_rms_m,
                                           SMOKE_A_SP3_DOUBLE_AGREEMENT0_POSITION_RMS_PRESENT,
                                           SMOKE_A_SP3_DOUBLE_AGREEMENT0_POSITION_RMS_BITS) &&
                       sa_optional_bits_match(row->position_max_present, row->position_max_m,
                                           SMOKE_A_SP3_DOUBLE_AGREEMENT0_POSITION_MAX_PRESENT,
                                           SMOKE_A_SP3_DOUBLE_AGREEMENT0_POSITION_MAX_BITS) &&
                       sa_optional_bits_match(row->clock_rms_present, row->clock_rms_s,
                                           SMOKE_A_SP3_DOUBLE_AGREEMENT0_CLOCK_RMS_PRESENT,
                                           SMOKE_A_SP3_DOUBLE_AGREEMENT0_CLOCK_RMS_BITS) &&
                       sa_optional_bits_match(row->clock_max_present, row->clock_max_s,
                                           SMOKE_A_SP3_DOUBLE_AGREEMENT0_CLOCK_MAX_PRESENT,
                                           SMOKE_A_SP3_DOUBLE_AGREEMENT0_CLOCK_MAX_BITS);
    return ok ? 0 : 1;
}

/* The product agreement summary against smoke_a_sp3 (`which` as above). */
static int sa_check_sp3_agreement_summary(const SidereonSp3AgreementSummary *summary, int which) {
    int ok = which == 0
                 ? sa_optional_bits_match(summary->position_rms_present, summary->position_rms_m,
                                       SMOKE_A_SP3_SINGLE_SUMMARY_POSITION_RMS_PRESENT,
                                       SMOKE_A_SP3_SINGLE_SUMMARY_POSITION_RMS_BITS) &&
                       sa_optional_bits_match(summary->position_max_present, summary->position_max_m,
                                           SMOKE_A_SP3_SINGLE_SUMMARY_POSITION_MAX_PRESENT,
                                           SMOKE_A_SP3_SINGLE_SUMMARY_POSITION_MAX_BITS) &&
                       sa_optional_bits_match(summary->clock_rms_present, summary->clock_rms_s,
                                           SMOKE_A_SP3_SINGLE_SUMMARY_CLOCK_RMS_PRESENT,
                                           SMOKE_A_SP3_SINGLE_SUMMARY_CLOCK_RMS_BITS) &&
                       sa_optional_bits_match(summary->clock_max_present, summary->clock_max_s,
                                           SMOKE_A_SP3_SINGLE_SUMMARY_CLOCK_MAX_PRESENT,
                                           SMOKE_A_SP3_SINGLE_SUMMARY_CLOCK_MAX_BITS)
                 : sa_optional_bits_match(summary->position_rms_present, summary->position_rms_m,
                                       SMOKE_A_SP3_DOUBLE_SUMMARY_POSITION_RMS_PRESENT,
                                       SMOKE_A_SP3_DOUBLE_SUMMARY_POSITION_RMS_BITS) &&
                       sa_optional_bits_match(summary->position_max_present, summary->position_max_m,
                                           SMOKE_A_SP3_DOUBLE_SUMMARY_POSITION_MAX_PRESENT,
                                           SMOKE_A_SP3_DOUBLE_SUMMARY_POSITION_MAX_BITS) &&
                       sa_optional_bits_match(summary->clock_rms_present, summary->clock_rms_s,
                                           SMOKE_A_SP3_DOUBLE_SUMMARY_CLOCK_RMS_PRESENT,
                                           SMOKE_A_SP3_DOUBLE_SUMMARY_CLOCK_RMS_BITS) &&
                       sa_optional_bits_match(summary->clock_max_present, summary->clock_max_s,
                                           SMOKE_A_SP3_DOUBLE_SUMMARY_CLOCK_MAX_PRESENT,
                                           SMOKE_A_SP3_DOUBLE_SUMMARY_CLOCK_MAX_BITS);
    return ok ? 0 : 1;
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
    SidereonSp3 *frame_a = NULL;
    SidereonSp3 *frame_b = NULL;
    SidereonSp3 *frame_merged = NULL;
    SidereonSp3MergeReport *frame_report = NULL;

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
            rc = fail_value("sidereon_sp3_epochs_j2000_seconds exact bits", 1);
            goto cleanup;
        }
    }

    SidereonSp3PredictionSummary prediction_summary;
    /* Expected values: sidereon-core's prediction summary of the same file
     * (tests/valgen smoke_a_sp3). */
    if (sidereon_sp3_prediction_summary(sp3, &prediction_summary) != SIDEREON_STATUS_OK ||
        prediction_summary.epoch_count != SMOKE_A_SP3_PREDICTION_EPOCH_COUNT ||
        prediction_summary.observed_through_present != SMOKE_A_SP3_OBSERVED_THROUGH_PRESENT ||
        f64_to_bits(prediction_summary.observed_through_j2000_seconds) !=
            SMOKE_A_SP3_OBSERVED_THROUGH_BITS) {
        rc = fail("sidereon_sp3_prediction_summary", 1);
        goto cleanup;
    }
    SidereonSp3EpochPrediction epoch_prediction;
    if (sidereon_sp3_epoch_prediction(sp3, 0, &epoch_prediction) != SIDEREON_STATUS_OK ||
        epoch_prediction.observed != SMOKE_A_SP3_EPOCH0_OBSERVED ||
        epoch_prediction.orbit_predicted_satellite_count !=
            SMOKE_A_SP3_EPOCH0_ORBIT_PREDICTED_COUNT ||
        epoch_prediction.clock_predicted_satellite_count !=
            SMOKE_A_SP3_EPOCH0_CLOCK_PREDICTED_COUNT) {
        rc = fail("sidereon_sp3_epoch_prediction", 1);
        goto cleanup;
    }

    SidereonSp3State state;
    if (sidereon_sp3_state(sp3, "G01", 0, &state) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_sp3_state", 1);
        goto cleanup;
    }
    for (size_t i = 0; i < 3; i++) {
        if (f64_to_bits(state.position_m[i]) != SP3_STATE_G01_EPOCH0_POSITION_BITS[i]) {
            rc = fail_value("sidereon_sp3_state position bits", 1);
            goto cleanup;
        }
    }
    if (!state.has_clock_s ||
        f64_to_bits(state.clock_s) != SP3_STATE_G01_EPOCH0_CLOCK_BITS ||
        state.has_velocity_m_s != SMOKE_A_SP3_G01_E0_HAS_VELOCITY ||
        state.has_clock_rate_s_s != SMOKE_A_SP3_G01_E0_HAS_CLOCK_RATE ||
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
                    rc = fail_value("sidereon_sp3_interpolate position bits", 1);
                    goto cleanup;
                }
            }
            if (f64_to_bits(clocks[q]) != SP3_INTERP_CLOCK_BITS[sat][q]) {
                rc = fail_value("sidereon_sp3_interpolate clock bits", 1);
                goto cleanup;
            }
        }
    }

    written = 123;
    required = 123;
    SidereonStatus text_status = sidereon_sp3_to_sp3_text(sp3, NULL, 0, &written, &required);
    if (text_status != SIDEREON_STATUS_OK || written != 0 || required != SP3_TO_SP3_TEXT_LEN ||
        strlen(SP3_TO_SP3_TEXT) != SP3_TO_SP3_TEXT_LEN) {
        /* A size query that succeeds with another length leaves no message of
         * its own, so the counts are printed beside the sticky last error. */
        fprintf(stderr,
                "sidereon_sp3_to_sp3_text size query: status %d, written %zu, required %zu, "
                "expected %zu\n",
                (int)text_status, written, required, (size_t)SP3_TO_SP3_TEXT_LEN);
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
    /* The defaults are sidereon-core's MergeOptions::default(). */
    if ((merge_options.precedence_scope == SIDEREON_SP3_MERGE_PRECEDENCE_SCOPE_CELL) !=
            SMOKE_A_SP3_MERGE_DEFAULT_SCOPE_IS_CELL ||
        (merge_options.outlier_reject_enabled != 0) != SMOKE_A_SP3_MERGE_DEFAULT_OUTLIER_REJECT) {
        rc = fail("sidereon_sp3_merge_options_init new defaults", 1);
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
        merged_epochs != SMOKE_A_SP3_SINGLE_MERGED_EPOCH_COUNT) {
        rc = fail("sidereon_sp3_merge merged epoch count", 1);
        goto cleanup;
    }
    size_t merge_quarantined = 123;
    size_t merge_single_source = 123;
    size_t merge_outliers = 123;
    size_t merge_clock_outliers = 123;
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
        sidereon_sp3_merge_report_flag_count(
            report, SIDEREON_SP3_MERGE_FLAG_KIND_CLOCK_OUTLIER, &merge_clock_outliers) !=
            SIDEREON_STATUS_OK ||
        merge_quarantined != SMOKE_A_SP3_SINGLE_QUARANTINED ||
        merge_single_source != SMOKE_A_SP3_SINGLE_SINGLE_SOURCE ||
        merge_outliers != SMOKE_A_SP3_SINGLE_POSITION_OUTLIERS ||
        merge_clock_outliers != SMOKE_A_SP3_SINGLE_CLOCK_OUTLIERS) {
        rc = fail("sidereon_sp3_merge_report_flag_count values", 1);
        goto cleanup;
    }
    SidereonSp3MergeFlag flag;
    if (sidereon_sp3_merge_report_flag(
            report, SIDEREON_SP3_MERGE_FLAG_KIND_SINGLE_SOURCE, 0, &flag) != SIDEREON_STATUS_OK ||
        f64_to_bits(flag.epoch_j2000_seconds) != SMOKE_A_SP3_SINGLE_FLAG0_EPOCH_BITS ||
        !token_equals(&flag.sat_id, SMOKE_A_SP3_SINGLE_FLAG0_SAT_ID) ||
        flag.source_count != SMOKE_A_SP3_SINGLE_FLAG0_SOURCE_COUNT) {
        rc = fail("sidereon_sp3_merge_report_flag first single-source flag", 1);
        goto cleanup;
    }
    size_t source_written = 123;
    size_t source_required = 123;
    if (sidereon_sp3_merge_report_flag_sources(
            report, SIDEREON_SP3_MERGE_FLAG_KIND_SINGLE_SOURCE, 0, NULL, 0, &source_written,
            &source_required) != SIDEREON_STATUS_OK ||
        source_written != 0 || source_required != SMOKE_A_SP3_SINGLE_FLAG0_SOURCE_COUNT) {
        rc = fail("sidereon_sp3_merge_report_flag_sources size query", 1);
        goto cleanup;
    }
    size_t source_indices[1] = {99};
    if (sidereon_sp3_merge_report_flag_sources(
            report, SIDEREON_SP3_MERGE_FLAG_KIND_SINGLE_SOURCE, 0, source_indices, 1,
            &source_written, &source_required) != SIDEREON_STATUS_OK ||
        source_written != 1 || source_required != SMOKE_A_SP3_SINGLE_FLAG0_SOURCE_COUNT ||
        source_indices[0] != SMOKE_A_SP3_SINGLE_FLAG0_SOURCE0) {
        rc = fail("sidereon_sp3_merge_report_flag_sources full copy", 1);
        goto cleanup;
    }

    // Agreement metric on the single-source report, compared with
    // sidereon-core's own report of the same merge (smoke_a_sp3): the per-epoch
    // count, the first entry and the product summary. An absent value reads NaN
    // (binding contract); a present one is compared bit for bit.
    size_t epoch_agreement_count = 123;
    if (sidereon_sp3_merge_report_epoch_agreement_count(NULL, &epoch_agreement_count) !=
            SIDEREON_STATUS_NULL_POINTER ||
        epoch_agreement_count != 0) {
        rc = fail("sidereon_sp3_merge_report_epoch_agreement_count null clears count", 1);
        goto cleanup;
    }
    if (sidereon_sp3_merge_report_epoch_agreement_count(report, &epoch_agreement_count) !=
            SIDEREON_STATUS_OK ||
        epoch_agreement_count != SMOKE_A_SP3_SINGLE_EPOCH_AGREEMENT_COUNT) {
        rc = fail("sidereon_sp3_merge_report_epoch_agreement_count value", 1);
        goto cleanup;
    }
    SidereonSp3EpochAgreement single_epoch_agreement;
    if (sidereon_sp3_merge_report_epoch_agreement(report, 0, &single_epoch_agreement) !=
            SIDEREON_STATUS_OK ||
        sa_check_sp3_epoch_agreement(&single_epoch_agreement, 0) != 0) {
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
        sa_check_sp3_agreement_summary(&single_summary, 0) != 0) {
        rc = fail("sidereon_sp3_merge_report_agreement_summary single-source", 1);
        goto cleanup;
    }

    // Two identical sources form a 2-member consensus per cell, the
    // multi-source path; the expected values are sidereon-core's report of the
    // same merge (smoke_a_sp3).
    const SidereonSp3 *sources2[2] = {sp3, sp3};
    if (sidereon_sp3_merge(sources2, 2, &merge_options, &merged2, &report2) !=
        SIDEREON_STATUS_OK) {
        rc = fail("sidereon_sp3_merge two identical sources", 1);
        goto cleanup;
    }
    SidereonSp3EpochAgreement multi_epoch_agreement;
    size_t multi_agreement_count = 123;
    if (sidereon_sp3_merge_report_epoch_agreement_count(report2, &multi_agreement_count) !=
            SIDEREON_STATUS_OK ||
        multi_agreement_count != SMOKE_A_SP3_DOUBLE_EPOCH_AGREEMENT_COUNT ||
        sidereon_sp3_merge_report_epoch_agreement(report2, 0, &multi_epoch_agreement) !=
            SIDEREON_STATUS_OK ||
        sa_check_sp3_epoch_agreement(&multi_epoch_agreement, 1) != 0) {
        rc = fail("sidereon_sp3_merge_report_epoch_agreement multi-source entry", 1);
        goto cleanup;
    }
    SidereonSp3AgreementSummary multi_summary;
    if (sidereon_sp3_merge_report_agreement_summary(report2, &multi_summary) !=
            SIDEREON_STATUS_OK ||
        sa_check_sp3_agreement_summary(&multi_summary, 1) != 0) {
        rc = fail("sidereon_sp3_merge_report_agreement_summary multi-source", 1);
        goto cleanup;
    }

    const char *frame_a_text =
        "#cP2020  6 25  0  0  0.00000000       1 ORBIT IGS14 FIT  TST\n"
        "## 2111 432000.00000000   900.00000000 59025 0.0000000000000\n"
        "+    1   G01  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0\n"
        "++         0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0\n"
        "%c G  cc GPS ccc cccc cccc cccc cccc ccccc ccccc ccccc ccccc\n"
        "%c cc cc ccc ccc cccc cccc cccc cccc ccccc ccccc ccccc ccccc\n"
        "%f  1.2500000  1.025000000  0.00000000000  0.000000000000000\n"
        "%f  0.0000000  0.000000000  0.00000000000  0.000000000000000\n"
        "%i    0    0    0    0      0      0      0      0         0\n"
        "%i    0    0    0    0      0      0      0      0         0\n"
        "/* TEST SP3-c FIXTURE\n"
        "*  2020  6 25  0  0  0.00000000\n"
        "PG01  15000.000000 -20000.000000   5000.000000    100.000000\n"
        "EOF\n";
    const char *frame_b_text =
        "#cP2020  6 25  0  0  0.00000000       1 ORBIT ITRF2 FIT  TST\n"
        "## 2111 432000.00000000   900.00000000 59025 0.0000000000000\n"
        "+    1   G02  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0\n"
        "++         0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0\n"
        "%c G  cc GPS ccc cccc cccc cccc cccc ccccc ccccc ccccc ccccc\n"
        "%c cc cc ccc ccc cccc cccc cccc cccc ccccc ccccc ccccc ccccc\n"
        "%f  1.2500000  1.025000000  0.00000000000  0.000000000000000\n"
        "%f  0.0000000  0.000000000  0.00000000000  0.000000000000000\n"
        "%i    0    0    0    0      0      0      0      0         0\n"
        "%i    0    0    0    0      0      0      0      0         0\n"
        "/* TEST SP3-c FIXTURE\n"
        "*  2020  6 25  0  0  0.00000000\n"
        "PG02  16000.000000 -21000.000000   6000.000000    200.000000\n"
        "EOF\n";
    if (sidereon_sp3_load((const uint8_t *)frame_a_text, strlen(frame_a_text), &frame_a) !=
            SIDEREON_STATUS_OK ||
        sidereon_sp3_load((const uint8_t *)frame_b_text, strlen(frame_b_text), &frame_b) !=
            SIDEREON_STATUS_OK) {
        rc = fail("sidereon_sp3_load frame reconciliation fixtures", 1);
        goto cleanup;
    }
    const char *asserted_labels[2] = {"IGS14", "ITRF2"};
    SidereonSp3FrameLabelSet asserted_set = {
        .labels = asserted_labels,
        .label_count = 2,
    };
    SidereonSp3MergeOptions frame_options;
    if (sidereon_sp3_merge_options_init(&frame_options) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_sp3_merge_options_init frame options", 1);
        goto cleanup;
    }
    frame_options.min_agree = 1;
    frame_options.clock_min_common = 1;
    frame_options.asserted_frame_label_sets = &asserted_set;
    frame_options.asserted_frame_label_set_count = 1;
    const SidereonSp3 *frame_sources[2] = {frame_a, frame_b};
    if (sidereon_sp3_merge(frame_sources, 2, &frame_options, &frame_merged, &frame_report) !=
        SIDEREON_STATUS_OK) {
        rc = fail("sidereon_sp3_merge asserted frame reconciliation", 1);
        goto cleanup;
    }
    size_t reconciliation_count = 123;
    if (sidereon_sp3_merge_report_frame_reconciliation_count(frame_report, &reconciliation_count) !=
            SIDEREON_STATUS_OK ||
        reconciliation_count != SMOKE_A_SP3_FRAME_RECONCILIATION_COUNT) {
        rc = fail("sidereon_sp3_merge_report_frame_reconciliation_count", 1);
        goto cleanup;
    }
    SidereonSp3FrameReconciliation reconciliation;
    if (sidereon_sp3_merge_report_frame_reconciliation(frame_report, 0, &reconciliation) !=
            SIDEREON_STATUS_OK ||
        (reconciliation.method ==
         SIDEREON_SP3_FRAME_RECONCILIATION_METHOD_ASSERTED_EQUIVALENCE) !=
            SMOKE_A_SP3_FRAME_METHOD_IS_ASSERTED ||
        reconciliation.source_index != SMOKE_A_SP3_FRAME_SOURCE_INDEX ||
        reconciliation.asserted_label_count != SMOKE_A_SP3_FRAME_ASSERTED_LABEL_COUNT ||
        reconciliation.records_affected != SMOKE_A_SP3_FRAME_RECORDS_AFFECTED ||
        reconciliation.parameters_present != SMOKE_A_SP3_FRAME_PARAMETERS_PRESENT ||
        reconciliation.source_label_len != strlen(SMOKE_A_SP3_FRAME_SOURCE_LABEL) ||
        reconciliation.target_label_len != strlen(SMOKE_A_SP3_FRAME_TARGET_LABEL)) {
        rc = fail("sidereon_sp3_merge_report_frame_reconciliation row", 1);
        goto cleanup;
    }
    uint8_t label_buf[8] = {0};
    size_t label_written = 123;
    size_t label_required = 123;
    if (sidereon_sp3_merge_report_frame_reconciliation_source_label(
            frame_report, 0, label_buf, sizeof(label_buf), &label_written, &label_required) !=
            SIDEREON_STATUS_OK ||
        label_written != strlen(SMOKE_A_SP3_FRAME_SOURCE_LABEL) ||
        label_required != strlen(SMOKE_A_SP3_FRAME_SOURCE_LABEL) ||
        memcmp(label_buf, SMOKE_A_SP3_FRAME_SOURCE_LABEL, label_written) != 0) {
        rc = fail("sidereon_sp3_merge_report_frame_reconciliation_source_label", 1);
        goto cleanup;
    }
    memset(label_buf, 0, sizeof(label_buf));
    if (sidereon_sp3_merge_report_frame_reconciliation_asserted_label(
            frame_report, 0, 0, label_buf, sizeof(label_buf), &label_written,
            &label_required) != SIDEREON_STATUS_OK ||
        label_written != strlen(SMOKE_A_SP3_FRAME_ASSERTED_LABEL0) ||
        label_required != strlen(SMOKE_A_SP3_FRAME_ASSERTED_LABEL0) ||
        memcmp(label_buf, SMOKE_A_SP3_FRAME_ASSERTED_LABEL0, label_written) != 0) {
        rc = fail("sidereon_sp3_merge_report_frame_reconciliation_asserted_label", 1);
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
    sidereon_sp3_merge_report_free(frame_report);
    sidereon_sp3_free(frame_merged);
    sidereon_sp3_free(frame_b);
    sidereon_sp3_free(frame_a);
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
            rows[i].base_tx_pos[axis] = bits_to_f64(CFIX_RTK_SAT_POS_M_BITS[i][axis]);
            rows[i].rover_tx_pos[axis] = bits_to_f64(CFIX_RTK_SAT_POS_M_BITS[i][axis]);
            rows[i].pos[axis] = bits_to_f64(CFIX_RTK_SAT_POS_M_BITS[i][axis]);
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
            return fail_value("rtk float baseline bits", 1);
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
        metadata.iterations != CFIX_RTK_FLOAT_ITERATIONS ||
        metadata.converged != SMOKE_A_RTK_FLOAT_CONVERGED ||
        strcmp(sa_rtk_solve_status_name(metadata.status), SMOKE_A_RTK_FLOAT_STATUS) != 0 ||
        metadata.n_observations != CFIX_RTK_FLOAT_N_OBSERVATIONS ||
        metadata.ambiguity_count != CFIX_RTK_NONREF_COUNT ||
        metadata.residual_count != SMOKE_A_RTK_FLOAT_RESIDUAL_COUNT ||
        metadata.used_sat_count != SMOKE_A_RTK_FLOAT_USED_SAT_COUNT ||
        strcmp(sa_observability_tier_name(metadata.geometry_quality.tier),
               SMOKE_A_RTK_FLOAT_GEOMETRY_TIER) != 0 ||
        metadata.geometry_quality.redundancy != CFIX_RTK_FLOAT_GEOMETRY_REDUNDANCY ||
        metadata.geometry_quality.rank != CFIX_RTK_FLOAT_GEOMETRY_RANK ||
        f64_to_bits(metadata.geometry_quality.condition_number) !=
            SMOKE_A_RTK_FLOAT_GEOMETRY_CONDITION_NUMBER_BITS ||
        f64_to_bits(metadata.geometry_quality.gdop) != SMOKE_A_RTK_FLOAT_GEOMETRY_GDOP_BITS ||
        metadata.geometry_quality.raim_checkable != SMOKE_A_RTK_FLOAT_GEOMETRY_RAIM_CHECKABLE ||
        metadata.geometry_quality.covariance_validated !=
            SMOKE_A_RTK_FLOAT_GEOMETRY_COVARIANCE_VALIDATED ||
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
            return fail_value("rtk float ambiguity bits", 1);
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
            return fail_value("rtk fixed baseline bits", 1);
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
        metadata.iterations != CFIX_RTK_FIXED_ITERATIONS ||
        metadata.converged != SMOKE_A_RTK_FIXED_CONVERGED ||
        strcmp(sa_rtk_solve_status_name(metadata.status), SMOKE_A_RTK_FIXED_STATUS) != 0 ||
        metadata.n_observations != CFIX_RTK_FIXED_N_OBSERVATIONS ||
        metadata.free_ambiguity_count != CFIX_RTK_FIXED_FREE_AMBIGUITY_COUNT ||
        metadata.fixed_ambiguity_count != CFIX_RTK_NONREF_COUNT ||
        metadata.residual_count != SMOKE_A_RTK_FIXED_RESIDUAL_COUNT ||
        metadata.used_sat_count != SMOKE_A_RTK_FIXED_USED_SAT_COUNT ||
        strcmp(sa_rtk_integer_status_name(metadata.integer_status),
               SMOKE_A_RTK_FIXED_INTEGER_STATUS) != 0 ||
        metadata.has_integer_ratio != SMOKE_A_RTK_FIXED_HAS_INTEGER_RATIO ||
        metadata.has_integer_best_score != SMOKE_A_RTK_FIXED_HAS_INTEGER_BEST_SCORE ||
        metadata.has_integer_second_best_score !=
            SMOKE_A_RTK_FIXED_HAS_INTEGER_SECOND_BEST_SCORE ||
        strcmp(sa_observability_tier_name(metadata.geometry_quality.tier),
               SMOKE_A_RTK_FIXED_GEOMETRY_TIER) != 0 ||
        metadata.geometry_quality.redundancy != CFIX_RTK_FIXED_GEOMETRY_REDUNDANCY ||
        metadata.geometry_quality.rank != CFIX_RTK_FIXED_GEOMETRY_RANK ||
        f64_to_bits(metadata.geometry_quality.condition_number) !=
            SMOKE_A_RTK_FIXED_GEOMETRY_CONDITION_NUMBER_BITS ||
        f64_to_bits(metadata.geometry_quality.gdop) != SMOKE_A_RTK_FIXED_GEOMETRY_GDOP_BITS ||
        metadata.geometry_quality.raim_checkable != SMOKE_A_RTK_FIXED_GEOMETRY_RAIM_CHECKABLE ||
        metadata.geometry_quality.covariance_validated !=
            SMOKE_A_RTK_FIXED_GEOMETRY_COVARIANCE_VALIDATED ||
        f64_to_bits(metadata.code_rms_m) != CFIX_RTK_FIXED_CODE_RMS_BITS ||
        f64_to_bits(metadata.phase_rms_m) != CFIX_RTK_FIXED_PHASE_RMS_BITS ||
        f64_to_bits(metadata.weighted_rms_m) != CFIX_RTK_FIXED_WEIGHTED_RMS_BITS ||
        f64_to_bits(metadata.integer_ratio) != CFIX_RTK_FIXED_RATIO_BITS ||
        f64_to_bits(metadata.integer_best_score) != CFIX_RTK_FIXED_BEST_SCORE_BITS ||
        f64_to_bits(metadata.integer_second_best_score) != CFIX_RTK_FIXED_SECOND_BEST_SCORE_BITS ||
        metadata.integer_candidates != CFIX_RTK_FIXED_INTEGER_CANDIDATES) {
        return fail("sidereon_rtk_fixed_solution_metadata", 1);
    }

    size_t written = 123;
    size_t required = 123;
    if (sidereon_rtk_fixed_solution_free_ambiguities(solution, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != 0 || required != CFIX_RTK_FIXED_FREE_AMBIGUITY_COUNT) {
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
            return fail_value("rtk fixed ambiguity bits", 1);
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
        float_config.base_ecef_m[axis] = bits_to_f64(CFIX_RTK_BASE_ECEF_M_BITS[axis]);
        float_config.initial_baseline_m[axis] = bits_to_f64(CFIX_RTK_INITIAL_BASELINE_M_BITS[axis]);
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
    /* sidereon-core refuses this geometry (smoke_a_rtk pins the refusal and
     * its text); the binding reports any float-solve refusal as
     * SIDEREON_STATUS_SOLVE with the engine's text (src/solve.rs guard). */
    SidereonRtkFloatSolution *singular_float_solution = (SidereonRtkFloatSolution *)(uintptr_t)1;
    if (sidereon_solve_rtk_float(&singular_float_config, &singular_float_solution) !=
            SIDEREON_STATUS_SOLVE ||
        singular_float_solution != NULL || !SMOKE_A_RTK_SINGULAR_REFUSED ||
        !sa_last_error_equals(SMOKE_A_RTK_SINGULAR_ERROR_TEXT)) {
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
        fixed_config.base_ecef_m[axis] = bits_to_f64(CFIX_RTK_BASE_ECEF_M_BITS[axis]);
        fixed_config.initial_baseline_m[axis] = bits_to_f64(CFIX_RTK_INITIAL_BASELINE_M_BITS[axis]);
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
        observations[i].code1_signal = NULL;
        observations[i].code2_signal = NULL;
        observations[i].phase1_signal = NULL;
        observations[i].phase2_signal = NULL;
        observations[i].has_glonass_channel = false;
        observations[i].glonass_channel = 0;
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
    config->initial_state.tropo_gradient_north_m = 0.0;
    config->initial_state.tropo_gradient_east_m = 0.0;
    configure_ppp_weights(&config->weights);
    configure_ppp_tropo(&config->tropo);
    configure_ppp_corrections(&config->corrections, antenna);
    configure_ppp_options(&config->options);
    config->has_elevation_cutoff_deg = false;
    config->elevation_cutoff_deg = 0.0;
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
    config->has_elevation_cutoff_deg = false;
    config->elevation_cutoff_deg = 0.0;
    sidereon_ppp_fixed_ambiguity_options_init(&config->ambiguity);
    config->ambiguity.wavelengths_m = wavelengths;
    config->ambiguity.wavelength_count = PPP_FIXED_AMBIGUITY_COUNT;
    config->ambiguity.offsets_m = offsets;
    config->ambiguity.offset_count = PPP_FIXED_AMBIGUITY_COUNT;
    config->ambiguity.ratio_threshold = bits_to_f64(PPP_FIXED_RATIO_THRESHOLD_BITS);
}

/* Print the returned and expected bit patterns of a position that differs from
 * the fixture. */
static int ppp_position_bits_mismatch(const char *context, const double got[3],
                                      const uint64_t expected[3]) {
    for (size_t axis = 0; axis < 3; axis++) {
        fprintf(stderr, "%s axis %zu: got 0x%016llx (%.17g), expected 0x%016llx\n", context,
                axis, (unsigned long long)f64_to_bits(got[axis]), got[axis],
                (unsigned long long)expected[axis]);
    }
    return fail_value(context, 1);
}

static int check_ppp_float_solution(const SidereonPppFloatSolution *solution) {
    double position[3];
    if (sidereon_ppp_float_solution_position(solution, position, 3) != SIDEREON_STATUS_OK) {
        return fail("sidereon_ppp_float_solution_position", 1);
    }
    for (size_t axis = 0; axis < 3; axis++) {
        if (f64_to_bits(position[axis]) != PPP_EXPECTED_FLOAT_POSITION_BITS[axis]) {
            return ppp_position_bits_mismatch("ppp float position bits", position, PPP_EXPECTED_FLOAT_POSITION_BITS);
        }
    }

    SidereonPppFloatMetadata metadata;
    if (sidereon_ppp_float_solution_metadata(solution, &metadata) != SIDEREON_STATUS_OK) {
        return fail("sidereon_ppp_float_solution_metadata", 1);
    }
    /* Expected values: sidereon-core's float solve of the same fixture
     * (tests/valgen smoke_a_ppp). */
    if (metadata.iterations != SMOKE_A_PPP_FLOAT_ITERATIONS ||
        metadata.converged != SMOKE_A_PPP_FLOAT_CONVERGED ||
        strcmp(sa_ppp_solve_status_name(metadata.status), SMOKE_A_PPP_FLOAT_STATUS) != 0 ||
        metadata.has_ztd_residual_m != SMOKE_A_PPP_FLOAT_HAS_ZTD_RESIDUAL ||
        metadata.ambiguity_count != SMOKE_A_PPP_FLOAT_AMBIGUITY_COUNT ||
        metadata.residual_count != SMOKE_A_PPP_FLOAT_RESIDUAL_COUNT ||
        metadata.used_sat_count != SMOKE_A_PPP_FLOAT_USED_SAT_COUNT ||
        f64_to_bits(metadata.code_rms_m) != SMOKE_A_PPP_FLOAT_CODE_RMS_BITS ||
        f64_to_bits(metadata.phase_rms_m) != SMOKE_A_PPP_FLOAT_PHASE_RMS_BITS ||
        f64_to_bits(metadata.weighted_rms_m) != SMOKE_A_PPP_FLOAT_WEIGHTED_RMS_BITS) {
        return fail_value("ppp float metadata", 1);
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
            f64_to_bits(ambiguities[i].value_m) != SMOKE_A_PPP_FLOAT_AMBIGUITY_BITS[i]) {
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
            return ppp_position_bits_mismatch("ppp fixed position bits", position, PPP_EXPECTED_FIXED_POSITION_BITS);
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
            return ppp_position_bits_mismatch("ppp fixed embedded float position bits",
                                              float_position,
                                              PPP_EXPECTED_FIXED_FLOAT_POSITION_BITS);
        }
    }

    SidereonPppFixedMetadata metadata;
    if (sidereon_ppp_fixed_solution_metadata(solution, &metadata) != SIDEREON_STATUS_OK) {
        return fail("sidereon_ppp_fixed_solution_metadata", 1);
    }
    if (metadata.iterations != SMOKE_A_PPP_FIXED_ITERATIONS ||
        metadata.converged != SMOKE_A_PPP_FIXED_CONVERGED ||
        strcmp(sa_ppp_solve_status_name(metadata.status), SMOKE_A_PPP_FIXED_STATUS) != 0 ||
        metadata.has_ztd_residual_m != SMOKE_A_PPP_FIXED_HAS_ZTD_RESIDUAL ||
        metadata.fixed_ambiguity_count != PPP_FIXED_AMBIGUITY_COUNT ||
        metadata.residual_count != SMOKE_A_PPP_FIXED_RESIDUAL_COUNT ||
        metadata.used_sat_count != SMOKE_A_PPP_FIXED_USED_SAT_COUNT ||
        !ppp_integer_status_equals(
            metadata.integer_status, PPP_EXPECTED_FIXED_INTEGER_STATUS) ||
        f64_to_bits(metadata.integer_ratio) != PPP_EXPECTED_FIXED_INTEGER_RATIO_BITS ||
        metadata.integer_candidates != PPP_EXPECTED_FIXED_INTEGER_CANDIDATES ||
        f64_to_bits(metadata.code_rms_m) != SMOKE_A_PPP_FIXED_CODE_RMS_BITS ||
        f64_to_bits(metadata.phase_rms_m) != SMOKE_A_PPP_FIXED_PHASE_RMS_BITS ||
        f64_to_bits(metadata.weighted_rms_m) != SMOKE_A_PPP_FIXED_WEIGHTED_RMS_BITS ||
        f64_to_bits(metadata.integer_best_score) != SMOKE_A_PPP_FIXED_INTEGER_BEST_SCORE_BITS ||
        metadata.has_integer_second_best_score !=
            SMOKE_A_PPP_FIXED_HAS_INTEGER_SECOND_BEST_SCORE ||
        f64_to_bits(metadata.integer_second_best_score) !=
            SMOKE_A_PPP_FIXED_INTEGER_SECOND_BEST_SCORE_BITS) {
        fprintf(stderr,
                "ppp fixed metadata: integer ratio 0x%016llx (%.17g), expected 0x%016llx; "
                "candidates %zu, expected %zu\n",
                (unsigned long long)f64_to_bits(metadata.integer_ratio), metadata.integer_ratio,
                (unsigned long long)PPP_EXPECTED_FIXED_INTEGER_RATIO_BITS,
                (size_t)metadata.integer_candidates, (size_t)PPP_EXPECTED_FIXED_INTEGER_CANDIDATES);
        return fail_value("ppp fixed metadata", 1);
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
            return fail_value("ppp fixed ambiguity bits", 1);
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

    /* Bad-buffer error path: sidereon-core refuses these garbage bytes
     * (smoke_a_spk pins the refusal and its text). The load reports a non-OK
     * status with that text and leaves out_spk NULL. */
    uint8_t garbage[64];
    memset(garbage, 0xAB, sizeof(garbage));
    SidereonSpk *bad_spk = (SidereonSpk *)(uintptr_t)1;
    if (sidereon_spk_load(garbage, sizeof(garbage), &bad_spk) == SIDEREON_STATUS_OK ||
        bad_spk != NULL || !SMOKE_A_SPK_GARBAGE_REFUSED ||
        !last_error_contains(SMOKE_A_SPK_GARBAGE_ERROR_TEXT)) {
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

    /* sidereon-core refuses a non-finite epoch (smoke_a_spk). */
    if (sidereon_spk_state(spk, SPK_TARGET, SPK_CENTER, NAN, &probe) == SIDEREON_STATUS_OK ||
        !SMOKE_A_SPK_NAN_REFUSED || !last_error_contains(SMOKE_A_SPK_NAN_ERROR_TEXT)) {
        rc = fail("sidereon_spk_state rejects non-finite et", 1);
        goto cleanup;
    }

    double max_position_error = 0.0;
    double max_velocity_error = 0.0;
    if (SMOKE_A_SPK_STATE_COUNT != SPK_REFERENCE_COUNT) {
        rc = fail_value("SPK engine state count", 1);
        goto cleanup;
    }
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
        /* sidereon-core's own state at this epoch, bit for bit (smoke_a_spk);
         * the CSPICE rows below remain an independent reference check. */
        for (int axis = 0; axis < 3; axis++) {
            if (f64_to_bits(state.position_km[axis]) !=
                    SMOKE_A_SPK_POSITION_KM_BITS[i * 3 + (size_t)axis] ||
                f64_to_bits(state.velocity_km_s[axis]) !=
                    SMOKE_A_SPK_VELOCITY_KM_S_BITS[i * 3 + (size_t)axis]) {
                rc = fail_value("sidereon_spk_state engine bits", 1);
                goto cleanup;
            }
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

    /* Asking for the state in the frame it is already expressed in rotates
     * nothing, so the state is the same to the bit. */
    SidereonSpkState native;
    SidereonSpkState in_frame;
    if (sidereon_spk_state(spk, SPK_TARGET, SPK_CENTER, SPK_REFERENCE[0][0], &native) !=
            SIDEREON_STATUS_OK ||
        sidereon_spk_state_in_frame(spk, SPK_TARGET, SPK_CENTER, SPK_REFERENCE[0][0],
                                    native.frame, &in_frame) != SIDEREON_STATUS_OK ||
        in_frame.frame != native.frame ||
        memcmp(in_frame.position_km, native.position_km, sizeof(native.position_km)) != 0 ||
        memcmp(in_frame.velocity_km_s, native.velocity_km_s, sizeof(native.velocity_km_s)) != 0) {
        rc = fail("sidereon_spk_state_in_frame native frame", 1);
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

    SidereonPppAutoInitOptions auto_options;
    if (sidereon_ppp_auto_init_options_init(&auto_options) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_ppp_auto_init_options_init", 1);
        goto cleanup;
    }
    SidereonPppFloatConfig empty_auto_float_config = float_config;
    empty_auto_float_config.epochs = NULL;
    empty_auto_float_config.epoch_count = 0;
    SidereonPppFloatSolution *empty_auto_float = (SidereonPppFloatSolution *)(uintptr_t)1;
    if (sidereon_solve_ppp_auto_init_float(sp3, &empty_auto_float_config, &auto_options,
                                           &empty_auto_float) != SIDEREON_STATUS_SOLVE ||
        empty_auto_float != NULL ||
        ppp_auto_init_detail_matches("sidereon_solve_ppp_auto_init_float", "empty_epochs",
                                     false) != 0) {
        rc = fail("sidereon_solve_ppp_auto_init_float reports EmptyEpochs", 1);
        goto cleanup;
    }

    SidereonPppEpoch code_seed_epoch = epochs[0];
    code_seed_epoch.observations = NULL;
    code_seed_epoch.observation_count = 0;
    SidereonPppFloatConfig code_seed_float_config = float_config;
    code_seed_float_config.epochs = &code_seed_epoch;
    code_seed_float_config.epoch_count = 1;
    SidereonPppFloatSolution *code_seed_float = (SidereonPppFloatSolution *)(uintptr_t)1;
    if (sidereon_solve_ppp_auto_init_float(sp3, &code_seed_float_config, &auto_options,
                                           &code_seed_float) != SIDEREON_STATUS_SOLVE ||
        code_seed_float != NULL ||
        ppp_auto_init_detail_matches("sidereon_solve_ppp_auto_init_float", "code_seed_failed",
                                     true) != 0) {
        rc = fail("sidereon_solve_ppp_auto_init_float retains nested code-seed SppError", 1);
        goto cleanup;
    }

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
     * golden above); a VMF1 config with a site-wise a-coefficient series
     * bracketing the 2020-06-25 (MJD ~59025) arc solves to sidereon-core's
     * position for the same series (smoke_a_ppp). */
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
        f64_to_bits(vmf_position[0]) != SMOKE_A_PPP_VMF1_POSITION_BITS[0] ||
        f64_to_bits(vmf_position[1]) != SMOKE_A_PPP_VMF1_POSITION_BITS[1] ||
        f64_to_bits(vmf_position[2]) != SMOKE_A_PPP_VMF1_POSITION_BITS[2]) {
        sidereon_ppp_float_solution_free(vmf_solution);
        rc = fail("VMF1 float solution position bits", 1);
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

    /* sidereon-core's VmfSiteSeries::new refuses non-ascending MJD samples
     * (smoke_a_ppp pins the refusal and its text); the binding surfaces it as
     * INVALID_ARGUMENT with that text (src/lib.rs ppp_tropo_mapping_from_c). */
    SidereonPppFloatConfig vmf_unsorted = vmf_config;
    vmf_unsorted.tropo.vmf_samples[1].mjd = 59024.0; /* not strictly increasing */
    vmf_bad = (SidereonPppFloatSolution *)(uintptr_t)1;
    if (sidereon_solve_ppp_float(sp3, &vmf_unsorted, &vmf_bad) != SIDEREON_STATUS_INVALID_ARGUMENT ||
        vmf_bad != NULL || !SMOKE_A_PPP_VMF_UNSORTED_REFUSED ||
        !last_error_contains(SMOKE_A_PPP_VMF_UNSORTED_ERROR_TEXT)) {
        rc = fail("VMF1 non-ascending samples must be rejected", 1);
        goto cleanup;
    }
    printf("PPP VMF1: solved to the engine's position, zero-samples/bad-mapping/unsorted "
           "rejected\n");

    SidereonPppFloatMapEntry wavelengths[PPP_FIXED_AMBIGUITY_COUNT];
    SidereonPppFloatMapEntry offsets[PPP_FIXED_AMBIGUITY_COUNT];
    fill_ppp_fixed_maps(wavelengths, offsets);

    SidereonPppFixedConfig fixed_config;
    configure_ppp_fixed_config(
        &fixed_config, epochs, &float_config.corrections, wavelengths, offsets);

    SidereonPppFixedSolution *empty_auto_fixed = (SidereonPppFixedSolution *)(uintptr_t)1;
    if (sidereon_solve_ppp_auto_init_fixed(sp3, &empty_auto_float_config, &fixed_config,
                                           &auto_options, &empty_auto_fixed) !=
            SIDEREON_STATUS_SOLVE ||
        empty_auto_fixed != NULL ||
        ppp_auto_init_detail_matches("sidereon_solve_ppp_auto_init_fixed", "empty_epochs",
                                     false) != 0) {
        rc = fail("sidereon_solve_ppp_auto_init_fixed reports EmptyEpochs", 1);
        goto cleanup;
    }

    SidereonPppFixedSolution *code_seed_fixed = (SidereonPppFixedSolution *)(uintptr_t)1;
    if (sidereon_solve_ppp_auto_init_fixed(sp3, &code_seed_float_config, &fixed_config,
                                           &auto_options, &code_seed_fixed) !=
            SIDEREON_STATUS_SOLVE ||
        code_seed_fixed != NULL ||
        ppp_auto_init_detail_matches("sidereon_solve_ppp_auto_init_fixed", "code_seed_failed",
                                     true) != 0) {
        rc = fail("sidereon_solve_ppp_auto_init_fixed retains nested code-seed SppError", 1);
        goto cleanup;
    }

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
        fprintf(stderr, "%s: got %.17g, expected %.17g within %.17g\n", context, got,
                expected, tolerance);
        return fail_value(context, 1);
    }
    return 0;
}

static int check_vec3_bits(const double values[3], const uint64_t expected[3],
                           const char *context) {
    for (size_t axis = 0; axis < 3; axis++) {
        if (f64_to_bits(values[axis]) != expected[axis]) {
            fprintf(stderr, "%s axis %zu: got 0x%016llx (%.17g), expected 0x%016llx\n",
                    context, axis, (unsigned long long)f64_to_bits(values[axis]),
                    values[axis], (unsigned long long)expected[axis]);
            return fail_value(context, 1);
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
            return fail_value("look-angle bits", 1);
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
            return fail_value("numerical ephemeris epoch bits", 1);
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
    /* sidereon-core refuses these two lines (smoke_a_prop pins the refusal and
     * its text); the binding reports it as INVALID_ARGUMENT (src/tle.rs
     * parse_tle_handle). */
    if (sidereon_tle_load("not a tle", "also not a tle", PROP_TLE_OPSMODE, &bad_tle) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        bad_tle != NULL || !SMOKE_A_PROP_NOT_A_TLE_REFUSED ||
        !last_error_contains(SMOKE_A_PROP_NOT_A_TLE_ERROR_TEXT)) {
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
        metadata.has_ephemeris_type != PROP_TLE_HAS_EPHEMERIS_TYPE ||
        metadata.has_elset_number != PROP_TLE_HAS_ELSET_NUMBER ||
        metadata.has_rev_number != PROP_TLE_HAS_REV_NUMBER ||
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
        written != 0 || required != SMOKE_A_PROP_CLEAN_CHECKSUM_WARNING_COUNT) {
        rc = fail("sidereon_tle_checksum_warnings clean size query", 1);
        goto cleanup;
    }
    /* The strict default refuses a column-69 digit that disagrees with the
     * checksum; the lenient policy reads it and reports the warning. */
    if (sidereon_tle_load(
            PROP_TLE_BAD_CHECKSUM_LINE1, PROP_TLE_BAD_CHECKSUM_LINE2, PROP_TLE_OPSMODE,
            &bad_checksum_tle) == SIDEREON_STATUS_OK ||
        bad_checksum_tle != NULL || !SMOKE_A_PROP_BAD_CHECKSUM_STRICT_REFUSED ||
        !last_error_contains(SMOKE_A_PROP_BAD_CHECKSUM_STRICT_ERROR_TEXT)) {
        rc = fail("sidereon_tle_load strict checksum refusal", 1);
        goto cleanup;
    }
    if (sidereon_tle_load_with_policy(
            PROP_TLE_BAD_CHECKSUM_LINE1, PROP_TLE_BAD_CHECKSUM_LINE2, PROP_TLE_OPSMODE,
            SIDEREON_TLE_POLICY_LENIENT, &bad_checksum_tle) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_tle_load_with_policy checksum warning case", 1);
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
        warning.kind != PROP_TLE_CHECKSUM_WARNING_KIND ||
        warning.found != PROP_TLE_CHECKSUM_WARNING_FOUND ||
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
        pass_count != PROP_PASS_COUNT || pass_count != SMOKE_A_PROP_PASS_COUNT) {
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
            f64_to_bits(pass_values[i].max_elevation_deg) !=
                SMOKE_A_PROP_PASS_MAX_ELEVATION_BITS[i] ||
            pass_values[i].duration_s !=
                (double)(pass_values[i].los_unix_us - pass_values[i].aos_unix_us) / 1.0e6 ||
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
            rc = fail_value("sidereon_ephemeris_times_s bits", 1);
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
 * (f_L1/f_k)^2. These cases prove (end-to-end) GLONASS observations solve,
 * (a) ionosphere-on with no channel leaves every GLONASS satellite out, so the
 * solve fails with too few usable satellites, (b) supplying the channels lets
 * GLONASS solve with the ionosphere on, (b') withholding one satellite's
 * channel leaves only that satellite out, reported with
 * SIDEREON_SPP_REJECTION_REASON_IONOSPHERE_CARRIER_UNRESOLVED, and the rest of
 * the epoch solves, (c) an out-of-range channel is treated like a missing one,
 * and (d) a populated channel map is a bit-for-bit no-op on a GPS-only solve.
 *
 * The scenario (receiver, epoch, satellites, pseudoranges and FDMA slots) is
 * formed by tests/valgen smoke_a_glonass from the committed multi-GNSS SP3
 * product: the geometric range from the receiver to each GLONASS satellite 10
 * degrees or more above the horizon, minus that satellite's SP3 clock term.
 * Every solution, refusal and rejection below is compared with sidereon-core's
 * solve of the same inputs from that generator. */

#define GLO_MAX_SATS 32

/* Fill the observations and channel map from smoke_a_glonass. */
static size_t build_glonass_scenario(SidereonObservation observations[GLO_MAX_SATS],
                                     SidereonGlonassChannel channels[GLO_MAX_SATS]) {
    for (size_t i = 0; i < SMOKE_A_GLONASS_SAT_COUNT; i++) {
        observations[i].sat_id = SMOKE_A_GLONASS_SAT_IDS[i];
        observations[i].pseudorange_m = bits_to_f64(SMOKE_A_GLONASS_PSEUDORANGE_BITS[i]);
        channels[i].slot = SMOKE_A_GLONASS_SLOTS[i];
        channels[i].channel = 0; /* a valid FDMA channel for each slot */
    }
    return SMOKE_A_GLONASS_SAT_COUNT;
}

/* One solve's expected values from smoke_a_glonass. */
typedef struct {
    const uint64_t *position_bits;
    size_t used_count;
    const char *const *used_ids;
    size_t tdop_count;
    const char *const *tdop_systems;
    const uint64_t *tdop_bits;
    bool has_dop;
    uint64_t dop_tdop_bits;
    size_t rejected_count;
    const char *const *rejected_ids;
    const char *const *rejected_reasons;
} SaGlonassExpected;

#define SA_GLONASS_EXPECTED(P)                                                                     \
    {                                                                                           \
        P##_POSITION_BITS, P##_USED_COUNT, P##_USED_IDS, P##_TDOP_COUNT, P##_TDOP_SYSTEMS,      \
            P##_TDOP_BITS, P##_HAS_DOP, P##_DOP_TDOP_BITS, P##_REJECTED_COUNT, P##_REJECTED_IDS, \
            P##_REJECTED_REASONS                                                                \
    }

/* The engine variant each C code stands for, as the binding maps them
 * (src/lib.rs gnss_system_to_c, src/spp.rs rejection_reason_to_c). */
static const char *sa_gnss_system_name(SidereonGnssSystem system) {
    switch (system) {
    case SIDEREON_GNSS_SYSTEM_GPS:
        return "Gps";
    case SIDEREON_GNSS_SYSTEM_GLONASS:
        return "Glonass";
    case SIDEREON_GNSS_SYSTEM_GALILEO:
        return "Galileo";
    case SIDEREON_GNSS_SYSTEM_BEI_DOU:
        return "BeiDou";
    case SIDEREON_GNSS_SYSTEM_QZSS:
        return "Qzss";
    case SIDEREON_GNSS_SYSTEM_NAVIC:
        return "Navic";
    case SIDEREON_GNSS_SYSTEM_SBAS:
        return "Sbas";
    }
    return "";
}

static const char *sa_spp_rejection_reason_name(SidereonSppRejectionReason reason) {
    switch (reason) {
    case SIDEREON_SPP_REJECTION_REASON_NO_EPHEMERIS:
        return "NoEphemeris";
    case SIDEREON_SPP_REJECTION_REASON_LOW_ELEVATION:
        return "LowElevation";
    case SIDEREON_SPP_REJECTION_REASON_SBAS_WITHDRAWN:
        return "SbasWithdrawn";
    case SIDEREON_SPP_REJECTION_REASON_SBAS_IONO_UNCOVERED:
        return "SbasIonoUncovered";
    case SIDEREON_SPP_REJECTION_REASON_IONOSPHERE_CARRIER_UNRESOLVED:
        return "IonosphereCarrierUnresolved";
    case SIDEREON_SPP_REJECTION_REASON_SSR_CORRECTION_EXCEEDS_LIMIT:
        return "SsrCorrectionExceedsLimit";
    }
    return "";
}

/* A GLONASS solve against sidereon-core's solve of the same inputs: position
 * bits, used satellites in order, per-system TDOP, scalar DOP TDOP and the
 * rejected satellites with their reasons. */
static int check_glonass_solution(const SidereonSppSolution *sol, const SaGlonassExpected *expected,
                                  const char *label) {
    double pos[3];
    if (sidereon_spp_solution_position(sol, pos, 3) != SIDEREON_STATUS_OK) {
        return fail("GLONASS solve position", 1);
    }
    for (size_t axis = 0; axis < 3; axis++) {
        if (f64_to_bits(pos[axis]) != expected->position_bits[axis]) {
            fprintf(stderr, "GLONASS %s position axis %zu differs\n", label, axis);
            return fail_value("GLONASS solve position bits", 1);
        }
    }
    size_t used = 0;
    SidereonSatelliteToken used_tokens[GLO_MAX_SATS];
    size_t written = 0;
    size_t required = 0;
    if (sidereon_spp_solution_used_sat_count(sol, &used) != SIDEREON_STATUS_OK ||
        used != expected->used_count || used > GLO_MAX_SATS ||
        sidereon_spp_solution_used_sat_ids(sol, used_tokens, GLO_MAX_SATS, &written,
                                           &required) != SIDEREON_STATUS_OK ||
        written != expected->used_count) {
        return fail("GLONASS used satellites", 1);
    }
    for (size_t i = 0; i < written; i++) {
        if (!token_equals(&used_tokens[i], expected->used_ids[i])) {
            return fail_value("GLONASS used satellite order", 1);
        }
    }
    printf("GLONASS %s: used=%zu sats\n", label, used);

    SidereonSppSystemTdop tdops[GLO_MAX_SATS];
    size_t tdop_written = 123, tdop_required = 123;
    if (sidereon_spp_solution_system_tdops(sol, tdops, GLO_MAX_SATS, &tdop_written,
                                           &tdop_required) != SIDEREON_STATUS_OK ||
        tdop_written != expected->tdop_count || tdop_required != expected->tdop_count) {
        return fail("sidereon_spp_solution_system_tdops (GLONASS)", 1);
    }
    for (size_t i = 0; i < tdop_written; i++) {
        if (strcmp(sa_gnss_system_name(tdops[i].system), expected->tdop_systems[i]) != 0 ||
            f64_to_bits(tdops[i].tdop) != expected->tdop_bits[i]) {
            return fail_value("sidereon_spp_solution_system_tdops (GLONASS) values", 1);
        }
    }
    SidereonDop dop;
    if (!expected->has_dop || sidereon_spp_solution_dop(sol, &dop) != SIDEREON_STATUS_OK ||
        f64_to_bits(dop.tdop) != expected->dop_tdop_bits) {
        return fail("GLONASS scalar DOP tdop", 1);
    }

    SidereonSppRejectedSat rejected[GLO_MAX_SATS];
    size_t rejected_written = 0;
    size_t rejected_required = 0;
    if (sidereon_spp_solution_rejected_sats(sol, rejected, GLO_MAX_SATS, &rejected_written,
                                            &rejected_required) != SIDEREON_STATUS_OK ||
        rejected_written != expected->rejected_count) {
        return fail("GLONASS rejected satellites", 1);
    }
    for (size_t i = 0; i < rejected_written; i++) {
        if (!token_equals(&rejected[i].sat_id, expected->rejected_ids[i]) ||
            strcmp(sa_spp_rejection_reason_name(rejected[i].reason), expected->rejected_reasons[i]) !=
                0) {
            return fail_value("GLONASS rejected satellite and reason", 1);
        }
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

    /* tests/sppgen solves the same inputs through the same engine path, so
     * the bits agree exactly. */
    for (int i = 0; i < 3; i++) {
        if (f64_to_bits(pos_without[i]) != SPP_EXPECTED_X_BITS[i]) {
            return fail("GPS golden not reproduced through the V2 path", 1);
        }
    }
    return 0;
}

static int exercise_spp_glonass_channels(const SidereonSp3 *sp3) {
    SidereonObservation observations[GLO_MAX_SATS];
    SidereonGlonassChannel channels[GLO_MAX_SATS];
    size_t n = build_glonass_scenario(observations, channels);

    SidereonSppInputsV2 inputs;
    if (sidereon_spp_inputs_v2_init(&inputs) != SIDEREON_STATUS_OK) {
        return fail("sidereon_spp_inputs_v2_init", 1);
    }
    inputs.base.observations = observations;
    inputs.base.observation_count = n;
    inputs.base.t_rx_j2000_s = bits_to_f64(SMOKE_A_GLONASS_T_RX_BITS);
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
    const SaGlonassExpected off_expected = SA_GLONASS_EXPECTED(SMOKE_A_GLONASS_OFF);
    if (check_glonass_solution(off_sol, &off_expected, "ionosphere off") != 0) {
        sidereon_spp_solution_free(off_sol);
        return 1;
    }
    sidereon_spp_solution_free(off_sol);

    /* (a) ionosphere on, no channel map: every GLONASS satellite is left out
     * and sidereon-core refuses the epoch; the binding reports the refusal as
     * SIDEREON_STATUS_SOLVE with the engine's text (src/solve.rs guard). */
    inputs.base.ionosphere = true;
    inputs.glonass_channels = NULL;
    inputs.glonass_channel_count = 0;
    SidereonSppSolution *gated = (SidereonSppSolution *)(uintptr_t)1;
    if (sidereon_solve_spp_v2(sp3, &inputs, &gated) != SIDEREON_STATUS_SOLVE || gated != NULL ||
        !SMOKE_A_GLONASS_GATED_REFUSED || !sa_last_error_equals(SMOKE_A_GLONASS_GATED_ERROR_TEXT)) {
        return fail("GLONASS ionosphere-on solve without channels is refused", 1);
    }

    /* (b) channels resolve every carrier: GLONASS solves with the ionosphere on. */
    inputs.glonass_channels = channels;
    inputs.glonass_channel_count = n;
    SidereonSppSolution *on_sol = NULL;
    if (sidereon_solve_spp_v2(sp3, &inputs, &on_sol) != SIDEREON_STATUS_OK) {
        return fail("GLONASS SPP solve (ionosphere on, channels supplied)", 1);
    }
    const SaGlonassExpected on_expected = SA_GLONASS_EXPECTED(SMOKE_A_GLONASS_ON);
    if (check_glonass_solution(on_sol, &on_expected, "ionosphere on") != 0) {
        sidereon_spp_solution_free(on_sol);
        return 1;
    }
    sidereon_spp_solution_free(on_sol);

    /* (b') withhold the channel of the first used satellite: sidereon-core
     * leaves only that satellite out, with its own reason. */
    uint8_t withheld_slot = (uint8_t)atoi(SMOKE_A_GLONASS_WITHHELD_ID + 1);
    SidereonGlonassChannel partial_channels[GLO_MAX_SATS];
    size_t partial_count = 0;
    for (size_t i = 0; i < n; i++) {
        if (channels[i].slot != withheld_slot) {
            partial_channels[partial_count++] = channels[i];
        }
    }
    inputs.glonass_channels = partial_channels;
    inputs.glonass_channel_count = partial_count;
    SidereonSppSolution *partial = (SidereonSppSolution *)(uintptr_t)1;
    SidereonStatus partial_status = sidereon_solve_spp_v2(sp3, &inputs, &partial);
    if (SMOKE_A_GLONASS_PARTIAL_REFUSED) {
        if (partial_status != SIDEREON_STATUS_SOLVE || partial != NULL ||
            !sa_last_error_equals(SMOKE_A_GLONASS_PARTIAL_ERROR_TEXT)) {
            return fail("GLONASS solve with one channel withheld is refused", 1);
        }
    } else {
        if (partial_status != SIDEREON_STATUS_OK) {
            return fail("GLONASS solve with one channel withheld", 1);
        }
        const SaGlonassExpected partial_expected = SA_GLONASS_EXPECTED(SMOKE_A_GLONASS_PARTIAL);
        int partial_rc = check_glonass_solution(partial, &partial_expected, "one channel withheld");
        sidereon_spp_solution_free(partial);
        if (partial_rc != 0) {
            return 1;
        }
    }

    /* (c) channel 9 is outside the FDMA allocation [-7, +6], so no carrier
     * resolves and sidereon-core refuses the epoch, as for a missing channel. */
    SidereonGlonassChannel bad_channels[GLO_MAX_SATS];
    for (size_t i = 0; i < n; i++) {
        bad_channels[i].slot = channels[i].slot;
        bad_channels[i].channel = 9;
    }
    inputs.glonass_channels = bad_channels;
    inputs.glonass_channel_count = n;
    SidereonSppSolution *bad = (SidereonSppSolution *)(uintptr_t)1;
    if (sidereon_solve_spp_v2(sp3, &inputs, &bad) != SIDEREON_STATUS_SOLVE || bad != NULL ||
        !SMOKE_A_GLONASS_BAD_CHANNEL_REFUSED ||
        !sa_last_error_equals(SMOKE_A_GLONASS_BAD_CHANNEL_ERROR_TEXT)) {
        return fail("out-of-range GLONASS channels are refused", 1);
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

    printf("OK: GLONASS SPP end-to-end (iono off/on), unresolved carriers, and GPS no-op\n");
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
    /* G02 is observed at epoch 0 (line 75 of
     * fixtures/obs/ESBC00DNK_R_20201770000_01D_30S_MO_trim.rnx) and the GPS code
     * list (lines 14-15, SYS / # / OBS TYPES) holds no ZZ9, so the binding
     * reports the undeclared code (src/rinex.rs). */
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
/* The 3.0 IONEX surface: parse-with-warnings and its owned warning list, the
 * owned header handle and its typed records, the sample intermediate
 * representation with its explicit presence masks, the owned slant-delay result
 * list with per-row typed failures, and the standalone regular TEC grid. Every
 * exported type and lifecycle accessor is called here so the generated header
 * and the build catch an ABI mistake. The numeric policy tags match
 * SidereonIonexMissingNodePolicy and SidereonIonexMappingPolicy. */
/* The shape of every owned standalone TEC-grid text accessor. */
typedef SidereonStatus (*TecGridTextFn)(const SidereonTecGridResult *result, uint8_t *out,
                                        size_t len, size_t *out_written, size_t *out_required);

/* Read one owned standalone TEC-grid text through the variable-length output
 * contract: first the required length, then the bytes. Writes a NUL-terminated
 * copy into buf and returns 0, or returns -1 when the contract is broken or the
 * text does not fit. */
static int read_tec_grid_text(TecGridTextFn read, const SidereonTecGridResult *result, char *buf,
                              size_t cap) {
    size_t written = SIZE_MAX;
    size_t required = SIZE_MAX;
    if (read(result, NULL, 0, &written, &required) != SIDEREON_STATUS_OK || written != 0) {
        return -1;
    }
    if (cap == 0 || required + 1 > cap) {
        return -1;
    }
    if (required > 0) {
        if (read(result, (uint8_t *)buf, cap - 1, &written, &required) != SIDEREON_STATUS_OK ||
            written != required) {
            return -1;
        }
    }
    buf[required] = '\0';
    return 0;
}

/* A 2x2x2 sample grid the two checks below build products from. `rms_present`
 * may be NULL, which leaves the RMS maps out of the product entirely. */
static void fill_ionex_sample_grid(SidereonTecGridSamples *samples, const double *epochs,
                                   const double *lats, const double *lons, const double *tec,
                                   const bool *tec_present, const double *rms,
                                   const bool *rms_present,
                                   const SidereonIonexHeader *header) {
    memset(samples, 0, sizeof *samples);
    samples->time_scale = SIDEREON_TIME_SCALE_UTC;
    samples->map_epochs_j2000_s = epochs;
    samples->map_epoch_count = 2;
    samples->lat_nodes_deg = lats;
    samples->lat_node_count = 2;
    samples->lon_nodes_deg = lons;
    samples->lon_node_count = 2;
    samples->dlat_deg = -80.0;
    samples->dlon_deg = 40.0;
    samples->shell_height_km = 450.0;
    samples->base_radius_km = 6371.0;
    samples->exponent = -1;
    samples->tec_maps_tecu = tec;
    samples->tec_maps_present = tec_present;
    samples->tec_map_value_count = 8;
    samples->has_rms_maps = rms_present != NULL;
    samples->rms_maps_tecu = rms;
    samples->rms_maps_present = rms_present;
    samples->rms_map_value_count = rms_present != NULL ? 8 : 0;
    samples->has_height_maps = false;
    samples->height_map_value_count = 0;
    samples->header = header;
}

/* The engine variant each C code stands for, as the binding maps them
 * (src/ionex.rs ionex_assumed_mapping_to_c, ionex_slant_error_to_c,
 * ionex_slant_refusal_to_c, ionex_mapping_function_to_c), so a row compares
 * with smoke_a_ionex's variant names. */
static const char *sa_ionex_assumed_mapping_name(SidereonIonexAssumedMappingKind kind) {
    switch (kind) {
    case SIDEREON_IONEX_ASSUMED_MAPPING_KIND_NONE:
        return "";
    case SIDEREON_IONEX_ASSUMED_MAPPING_KIND_NO_MAPPING:
        return "NoMapping";
    case SIDEREON_IONEX_ASSUMED_MAPPING_KIND_Q_FACTOR:
        return "QFactor";
    case SIDEREON_IONEX_ASSUMED_MAPPING_KIND_OTHER:
        return "Other";
    case SIDEREON_IONEX_ASSUMED_MAPPING_KIND_ABSENT:
        return "Absent";
    }
    return "?";
}

static const char *sa_ionex_slant_error_name(SidereonIonexSlantErrorKind kind) {
    switch (kind) {
    case SIDEREON_IONEX_SLANT_ERROR_KIND_NONE:
        return "";
    case SIDEREON_IONEX_SLANT_ERROR_KIND_INVALID_INPUT:
        return "InvalidInput";
    case SIDEREON_IONEX_SLANT_ERROR_KIND_OUT_OF_COVERAGE:
        return "IonexOutOfCoverage";
    case SIDEREON_IONEX_SLANT_ERROR_KIND_NODES_NOT_AVAILABLE:
        return "IonexNodesNotAvailable";
    case SIDEREON_IONEX_SLANT_ERROR_KIND_SLANT_UNAVAILABLE:
        return "IonexSlantUnavailable";
    case SIDEREON_IONEX_SLANT_ERROR_KIND_UNKNOWN:
        return "?";
    }
    return "?";
}

static const char *sa_ionex_slant_refusal_name(SidereonIonexSlantRefusalKind kind) {
    switch (kind) {
    case SIDEREON_IONEX_SLANT_REFUSAL_KIND_NONE:
        return "";
    case SIDEREON_IONEX_SLANT_REFUSAL_KIND_VARYING_HEIGHTS:
        return "VaryingHeights";
    case SIDEREON_IONEX_SLANT_REFUSAL_KIND_HEIGHT_NOT_AVAILABLE:
        return "HeightNotAvailable";
    case SIDEREON_IONEX_SLANT_REFUSAL_KIND_MAPPING_FUNCTION:
        return "MappingFunction";
    case SIDEREON_IONEX_SLANT_REFUSAL_KIND_UNKNOWN:
        return "?";
    }
    return "?";
}

static const char *sa_ionex_mapping_function_name(SidereonIonexMappingFunctionKind kind) {
    switch (kind) {
    case SIDEREON_IONEX_MAPPING_FUNCTION_KIND_NO_MAPPING:
        return "NoMapping";
    case SIDEREON_IONEX_MAPPING_FUNCTION_KIND_COS_Z:
        return "CosZ";
    case SIDEREON_IONEX_MAPPING_FUNCTION_KIND_Q_FACTOR:
        return "QFactor";
    case SIDEREON_IONEX_MAPPING_FUNCTION_KIND_OTHER:
        return "Other";
    }
    return "?";
}

/* One slant row's expected values from smoke_a_ionex. */
typedef struct {
    bool ok;
    uint64_t delay_bits;
    bool is_valid;
    bool has_held;
    bool has_degraded;
    bool has_assumed_mapping;
    const char *assumed_mapping;
    const char *error;
    const char *refusal;
    const char *declaration;
    const char *declared_function;
} SaIonexRowExpected;

#define SA_IONEX_ROW_EXPECTED(P)                                                     \
    {                                                                             \
        P##_OK, P##_DELAY_BITS, P##_IS_VALID, P##_HAS_HELD, P##_HAS_DEGRADED,     \
            P##_HAS_ASSUMED_MAPPING, P##_ASSUMED_MAPPING, P##_ERROR, P##_REFUSAL, \
            P##_DECLARATION, P##_DECLARED_FUNCTION                                \
    }

/* A slant row against sidereon-core's evaluation of the same request: a
 * success carries the delay bits and status, a failure the error kind and, for
 * a mapping-function refusal, the declaration it names. */
static int sa_check_ionex_row(const SidereonIonexSlantRowResult *row,
                           const SaIonexRowExpected *expected) {
    int ok = row->is_ok == expected->ok &&
             strcmp(sa_ionex_slant_error_name(row->error.kind), expected->error) == 0;
    if (ok && expected->ok) {
        const SidereonIonexSlantDelayStatus *status = &row->evaluation.status;
        ok = f64_to_bits(row->evaluation.delay_m) == expected->delay_bits &&
             status->is_valid == expected->is_valid && status->has_held == expected->has_held &&
             status->has_degraded == expected->has_degraded &&
             status->has_assumed_mapping == expected->has_assumed_mapping &&
             strcmp(sa_ionex_assumed_mapping_name(status->assumed_mapping),
                    expected->assumed_mapping) == 0;
    } else if (ok) {
        const char *declaration = "";
        if (row->error.has_mapping_declaration) {
            declaration = row->error.mapping_declaration ==
                                  SIDEREON_IONEX_MAPPING_DECLARATION_KIND_DECLARED
                              ? "Declared"
                              : "Absent";
        }
        const char *function = row->error.has_mapping_function
                                   ? sa_ionex_mapping_function_name(row->error.mapping_function)
                                   : "";
        ok = !row->evaluation.status.is_valid &&
             strcmp(sa_ionex_slant_refusal_name(row->error.refusal), expected->refusal) == 0 &&
             strcmp(declaration, expected->declaration) == 0 &&
             strcmp(function, expected->declared_function) == 0;
    }
    if (!ok) {
        fprintf(stderr, "IONEX row differs from the engine's evaluation (error %s)\n",
                sa_ionex_slant_error_name(row->error.kind));
    }
    return ok ? 0 : 1;
}

/* An RMS map that is absent and one that is present with every node missing
 * are different products, and the samples surface reports them differently.
 *
 * Absent: has_rms_maps false and a required length of zero. Present with every
 * node missing: has_rms_maps true, the full flattened length, every presence
 * flag false and every value NaN. The flag, not the length, separates them.
 * The RMS authority stays in the engine; this binding reports what the engine
 * holds and does not duplicate it. The height half of the same distinction is
 * checked separately above. */
static int exercise_ionex_rms_presence_distinction(void) {
    const double epochs[2] = {0.0, 3600.0};
    const double lats[2] = {40.0, -40.0};
    const double lons[2] = {-20.0, 20.0};
    double tec[8];
    bool tec_present[8];
    double rms[8];
    bool rms_all_missing[8];
    for (size_t i = 0; i < 8; i++) {
        tec[i] = 10.0;
        tec_present[i] = true;
        rms[i] = 1.0;
        rms_all_missing[i] = false;
    }

    /* No RMS map at all: no values, and the extraction routes report a required
     * length of zero rather than a buffer of zeros. */
    SidereonTecGridSamples samples;
    fill_ionex_sample_grid(&samples, epochs, lats, lons, tec, tec_present, rms, NULL, NULL);
    SidereonIonex *without = NULL;
    if (sidereon_ionex_from_tec_grid_samples(&samples, &without) != SIDEREON_STATUS_OK ||
        without == NULL) {
        sidereon_ionex_free(without);
        return fail("IONEX RMS distinction: build product without RMS", 1);
    }
    SidereonTecGridSamplesInfo info;
    size_t written = 0, required = 1;
    memset(&info, 0, sizeof info);
    if (sidereon_ionex_tec_grid_samples_info(without, &info) != SIDEREON_STATUS_OK ||
        info.has_rms_maps != SMOKE_A_IONEX_NO_RMS_HAS_RMS_MAPS ||
        info.rms_map_value_count != SMOKE_A_IONEX_NO_RMS_RMS_VALUE_COUNT ||
        sidereon_ionex_tec_grid_samples_rms_maps_tecu(without, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        required != SMOKE_A_IONEX_NO_RMS_RMS_VALUE_COUNT) {
        sidereon_ionex_free(without);
        return fail("IONEX absent RMS map reports no values", 1);
    }
    required = 1;
    if (sidereon_ionex_tec_grid_samples_rms_presence(without, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        required != SMOKE_A_IONEX_NO_RMS_RMS_VALUE_COUNT) {
        sidereon_ionex_free(without);
        return fail("IONEX absent RMS map reports no presence flags", 1);
    }
    sidereon_ionex_free(without);

    /* A present RMS map whose every node is missing: sidereon-core keeps the
     * map (smoke_a_ionex pins its presence, count and flags) and the binding
     * reads an absent node as NaN. */
    fill_ionex_sample_grid(&samples, epochs, lats, lons, tec, tec_present, rms, rms_all_missing,
                           NULL);
    SidereonIonex *with = NULL;
    if (sidereon_ionex_from_tec_grid_samples(&samples, &with) != SIDEREON_STATUS_OK ||
        with == NULL) {
        sidereon_ionex_free(with);
        return fail("IONEX RMS distinction: build product with all-missing RMS", 1);
    }
    memset(&info, 0, sizeof info);
    if (sidereon_ionex_tec_grid_samples_info(with, &info) != SIDEREON_STATUS_OK ||
        info.has_rms_maps != SMOKE_A_IONEX_MISSING_RMS_HAS_RMS_MAPS ||
        info.rms_map_value_count != SMOKE_A_IONEX_MISSING_RMS_RMS_VALUE_COUNT ||
        SMOKE_A_IONEX_MISSING_RMS_RMS_VALUE_COUNT != 8) {
        sidereon_ionex_free(with);
        return fail("IONEX all-missing RMS map is not an absent one", 1);
    }
    double read_values[8];
    bool read_presence[8];
    if (sidereon_ionex_tec_grid_samples_rms_maps_tecu(with, read_values, 8, &written,
                                                      &required) != SIDEREON_STATUS_OK ||
        required != SMOKE_A_IONEX_MISSING_RMS_RMS_VALUE_COUNT ||
        sidereon_ionex_tec_grid_samples_rms_presence(with, read_presence, 8, &written,
                                                     &required) != SIDEREON_STATUS_OK ||
        required != SMOKE_A_IONEX_MISSING_RMS_RMS_VALUE_COUNT) {
        sidereon_ionex_free(with);
        return fail("IONEX all-missing RMS map extraction", 1);
    }
    for (size_t i = 0; i < 8; i++) {
        if (read_presence[i] != SMOKE_A_IONEX_MISSING_RMS_PRESENCE[i] ||
            (!read_presence[i] && !isnan(read_values[i]))) {
            sidereon_ionex_free(with);
            return fail("IONEX all-missing RMS node holds no value", 1);
        }
    }
    sidereon_ionex_free(with);
    return 0;
}

/* A successful row under the single-layer factor keeps the custom code the
 * product declared, in the result list's own authority.
 *
 * SIDEREON_IONEX_ASSUMED_MAPPING_KIND_OTHER names the case without carrying the
 * text, which the engine keeps only on the header, so the code is unreachable
 * the moment the product and header are freed. The owned-list promise is that a
 * row stays complete after that, and after a later failing call has overwritten
 * the thread-local message. */
static int exercise_ionex_owned_mapping_code_lifetime(void) {
    static const char custom[] = "SLAB_3D_CUSTOM";
    const double epochs[2] = {0.0, 3600.0};
    const double lats[2] = {40.0, -40.0};
    const double lons[2] = {-20.0, 20.0};
    double tec[8];
    bool tec_present[8];
    double rms[8];
    for (size_t i = 0; i < 8; i++) {
        tec[i] = 10.0;
        tec_present[i] = true;
        rms[i] = 1.0;
    }

    SidereonIonexHeader *header = NULL;
    if (sidereon_ionex_header_new(&header) != SIDEREON_STATUS_OK || header == NULL) {
        return fail("IONEX mapping-code lifetime: header", 1);
    }
    if (sidereon_ionex_header_set_mapping_function(header,
                                                   SIDEREON_IONEX_MAPPING_FUNCTION_KIND_OTHER,
                                                   (const uint8_t *)custom,
                                                   sizeof custom - 1) != SIDEREON_STATUS_OK) {
        sidereon_ionex_header_free(header);
        return fail("IONEX mapping-code lifetime: set custom code", 1);
    }

    SidereonTecGridSamples samples;
    fill_ionex_sample_grid(&samples, epochs, lats, lons, tec, tec_present, rms, NULL, header);
    SidereonIonex *product = NULL;
    if (sidereon_ionex_from_tec_grid_samples(&samples, &product) != SIDEREON_STATUS_OK ||
        product == NULL) {
        sidereon_ionex_free(product);
        sidereon_ionex_header_free(header);
        return fail("IONEX mapping-code lifetime: build product", 1);
    }

    SidereonIonexSlantRequest request;
    request.lat_deg = 0.0;
    request.lon_deg = 0.0;
    request.azimuth_deg = 0.0;
    request.elevation_deg = 85.0;
    /* Between the two maps, clear of either coverage bound. */
    request.epoch_j2000_s = 1800;
    request.frequency_hz = 1575420000.0;

    /* SingleLayer applies 1/cos(z') and reports the declaration it did not use,
     * so the row succeeds and still names a custom code. */
    SidereonIonexSlantResultList *rows = NULL;
    if (sidereon_ionex_slant_delay_results(
            product, &request, 1,
            sidereon_ionex_slant_policy_init(SIDEREON_IONEX_COVERAGE_POLICY_STRICT,
                                             SIDEREON_IONEX_MISSING_NODE_POLICY_STRICT,
                                             SIDEREON_IONEX_MAPPING_POLICY_SINGLE_LAYER),
            &rows) != SIDEREON_STATUS_OK ||
        rows == NULL) {
        sidereon_ionex_slant_result_list_free(rows);
        sidereon_ionex_free(product);
        sidereon_ionex_header_free(header);
        return fail("IONEX mapping-code lifetime: evaluate", 1);
    }
    SidereonIonexSlantRowResult row;
    memset(&row, 0, sizeof row);
    const SaIonexRowExpected custom_expected = SA_IONEX_ROW_EXPECTED(SMOKE_A_IONEX_CUSTOM_ROW);
    if (sidereon_ionex_slant_result_get_row(rows, 0, &row) != SIDEREON_STATUS_OK ||
        row.status != SIDEREON_STATUS_OK || sa_check_ionex_row(&row, &custom_expected) != 0) {
        sidereon_ionex_slant_result_list_free(rows);
        sidereon_ionex_free(product);
        sidereon_ionex_header_free(header);
        return fail("IONEX successful row names its assumed mapping", 1);
    }

    /* Both handles the code could otherwise be read from are released, and a
     * later failing call overwrites the thread-local message. */
    sidereon_ionex_free(product);
    sidereon_ionex_header_free(header);
    size_t ignored = 0;
    if (sidereon_ionex_slant_result_list_count(NULL, &ignored) != SIDEREON_STATUS_NULL_POINTER) {
        sidereon_ionex_slant_result_list_free(rows);
        return fail("IONEX mapping-code lifetime: intervening failure", 1);
    }

    char code[64];
    size_t written = 0, required = 0;
    if (sidereon_ionex_slant_result_get_mapping_code(rows, 0, (uint8_t *)code, sizeof code,
                                                     &written, &required) !=
            SIDEREON_STATUS_OK ||
        required != sizeof custom - 1 || written != required ||
        memcmp(code, custom, required) != 0) {
        sidereon_ionex_slant_result_list_free(rows);
        return fail("IONEX successful row keeps its custom mapping code", 1);
    }
    /* A successful row still carries no engine text: the mapping code is
     * mapping context, not a failure message. */
    if (sidereon_ionex_slant_result_get_message(rows, 0, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        required != 0) {
        sidereon_ionex_slant_result_list_free(rows);
        return fail("IONEX successful row reports no engine text", 1);
    }
    sidereon_ionex_slant_result_list_free(rows);
    return 0;
}

/* The data of the MAPPING FUNCTION record, which the writer emits whether or
 * not a code is declared, so the label alone proves nothing. Returns the
 * trimmed 60-byte data field, or NULL if no such record is present. */
static const char *ionex_mapping_field(const char *text, char *out, size_t out_len) {
    const char *line = text;
    while (line != NULL && *line != '\0') {
        const char *end = strchr(line, '\n');
        size_t line_len = end != NULL ? (size_t)(end - line) : strlen(line);
        if (line_len > 60 && strncmp(line + 60, "MAPPING FUNCTION", 16) == 0) {
            size_t start = 0, stop = 60;
            while (start < stop && line[start] == ' ') {
                start++;
            }
            while (stop > start && line[stop - 1] == ' ') {
                stop--;
            }
            if (stop - start >= out_len) {
                return NULL;
            }
            memcpy(out, line + start, stop - start);
            out[stop - start] = '\0';
            return out;
        }
        line = end != NULL ? end + 1 : NULL;
    }
    return NULL;
}

/* A caller who supplies no header gets one declaring no mapping function.
 *
 * sidereon_ionex_header_new and a NULL header on either builder route build the
 * same undeclared header, so the declaration reads ABSENT rather than COSZ.
 * Four consequences follow and each is asserted here: the handle reports
 * ABSENT, a successful row under the default single-layer policy reports
 * assumed mapping ABSENT, the DECLARED policy refuses the same query naming an
 * ABSENT declaration and no function, and the IONEX text carries a blank
 * MAPPING FUNCTION field that reads back ABSENT. Declaring COSZ on the same
 * grid reverses all four, so the ABSENT assertions are not merely what every
 * header reports. */
static int exercise_ionex_undeclared_default_header(void) {
    const double epochs[2] = {0.0, 3600.0};
    const double lats[2] = {40.0, -40.0};
    const double lons[2] = {-20.0, 20.0};
    double tec[8];
    bool tec_present[8];
    double rms[8];
    for (size_t i = 0; i < 8; i++) {
        tec[i] = 10.0;
        tec_present[i] = true;
        rms[i] = 1.0;
    }

    SidereonIonexSlantRequest request;
    request.lat_deg = 0.0;
    request.lon_deg = 0.0;
    request.azimuth_deg = 0.0;
    request.elevation_deg = 85.0;
    request.epoch_j2000_s = 1800;
    request.frequency_hz = 1575420000.0;
    const SidereonIonexSlantPolicy single_layer = sidereon_ionex_slant_policy_init(
        SIDEREON_IONEX_COVERAGE_POLICY_STRICT, SIDEREON_IONEX_MISSING_NODE_POLICY_STRICT,
        SIDEREON_IONEX_MAPPING_POLICY_SINGLE_LAYER);
    const SidereonIonexSlantPolicy declared_policy = sidereon_ionex_slant_policy_init(
        SIDEREON_IONEX_COVERAGE_POLICY_STRICT, SIDEREON_IONEX_MISSING_NODE_POLICY_STRICT,
        SIDEREON_IONEX_MAPPING_POLICY_DECLARED);

    SidereonIonexMappingDeclarationKind declaration = SIDEREON_IONEX_MAPPING_DECLARATION_KIND_DECLARED;
    SidereonIonexMappingFunctionKind function = SIDEREON_IONEX_MAPPING_FUNCTION_KIND_COS_Z;
    size_t written = 0, required = 0;

    /* 1. The handle sidereon_ionex_header_new builds declares nothing (binding
     * contract, src/ionex.rs undeclared_ionex_header). */
    SidereonIonexHeader *fresh = NULL;
    if (sidereon_ionex_header_new(&fresh) != SIDEREON_STATUS_OK || fresh == NULL) {
        return fail("IONEX undeclared default: header_new", 1);
    }
    if (sidereon_ionex_header_get_mapping_declaration(fresh, &declaration, &function) !=
            SIDEREON_STATUS_OK ||
        declaration != SIDEREON_IONEX_MAPPING_DECLARATION_KIND_ABSENT ||
        sidereon_ionex_header_get_mapping_function_code(fresh, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        required != 0) {
        sidereon_ionex_header_free(fresh);
        return fail("IONEX header_new declares no mapping function", 1);
    }
    sidereon_ionex_header_free(fresh);

    /* The same holds read off a product built with no header at all. */
    SidereonTecGridSamples bare_samples;
    fill_ionex_sample_grid(&bare_samples, epochs, lats, lons, tec, tec_present, rms, NULL, NULL);
    SidereonIonex *bare = NULL;
    if (sidereon_ionex_from_tec_grid_samples(&bare_samples, &bare) != SIDEREON_STATUS_OK ||
        bare == NULL) {
        return fail("IONEX undeclared default: build bare product", 1);
    }
    SidereonIonexHeader *stored = NULL;
    if (sidereon_ionex_get_header(bare, &stored) != SIDEREON_STATUS_OK || stored == NULL) {
        sidereon_ionex_free(bare);
        return fail("IONEX undeclared default: product header", 1);
    }
    declaration = SIDEREON_IONEX_MAPPING_DECLARATION_KIND_DECLARED;
    if (sidereon_ionex_header_get_mapping_declaration(stored, &declaration, &function) !=
            SIDEREON_STATUS_OK ||
        (declaration == SIDEREON_IONEX_MAPPING_DECLARATION_KIND_DECLARED) !=
            SMOKE_A_IONEX_BARE_HEADER_DECLARED) {
        sidereon_ionex_header_free(stored);
        sidereon_ionex_free(bare);
        return fail("IONEX NULL header declares no mapping function", 1);
    }
    sidereon_ionex_header_free(stored);

    /* 2. A successful row under the default policy assumes ABSENT, not the
     * COSZ factor a declared header would report as no assumption at all. */
    SidereonIonexSlantResultList *bare_rows = NULL;
    if (sidereon_ionex_slant_delay_results(bare, &request, 1, single_layer, &bare_rows) !=
            SIDEREON_STATUS_OK ||
        bare_rows == NULL) {
        sidereon_ionex_slant_result_list_free(bare_rows);
        sidereon_ionex_free(bare);
        return fail("IONEX undeclared default: single-layer evaluate", 1);
    }
    SidereonIonexSlantRowResult bare_row;
    memset(&bare_row, 0, sizeof bare_row);
    const SaIonexRowExpected bare_expected = SA_IONEX_ROW_EXPECTED(SMOKE_A_IONEX_BARE_SINGLE_LAYER);
    if (sidereon_ionex_slant_result_get_row(bare_rows, 0, &bare_row) != SIDEREON_STATUS_OK ||
        sa_check_ionex_row(&bare_row, &bare_expected) != 0) {
        sidereon_ionex_slant_result_list_free(bare_rows);
        sidereon_ionex_free(bare);
        return fail("IONEX undeclared default assumes ABSENT", 1);
    }
    const double bare_delay_m = bare_row.evaluation.delay_m;
    sidereon_ionex_slant_result_list_free(bare_rows);

    /* 3. The DECLARED policy refuses that same query: there is no declaration
     * to reach, and ABSENT names no code. */
    SidereonIonexSlantResultList *refused_rows = NULL;
    if (sidereon_ionex_slant_delay_results(bare, &request, 1, declared_policy, &refused_rows) !=
            SIDEREON_STATUS_OK ||
        refused_rows == NULL) {
        sidereon_ionex_slant_result_list_free(refused_rows);
        sidereon_ionex_free(bare);
        return fail("IONEX undeclared default: declared evaluate", 1);
    }
    SidereonIonexSlantRowResult refused;
    memset(&refused, 0, sizeof refused);
    const SaIonexRowExpected refused_expected = SA_IONEX_ROW_EXPECTED(SMOKE_A_IONEX_BARE_DECLARED);
    if (sidereon_ionex_slant_result_get_row(refused_rows, 0, &refused) != SIDEREON_STATUS_OK ||
        sa_check_ionex_row(&refused, &refused_expected) != 0) {
        sidereon_ionex_slant_result_list_free(refused_rows);
        sidereon_ionex_free(bare);
        return fail("IONEX DECLARED policy refuses an absent declaration", 1);
    }
    sidereon_ionex_slant_result_list_free(refused_rows);

    /* 4. The text keeps the absence: a blank field that reads back ABSENT,
     * not an invented COSZ record. */
    char field[64] = {0};
    char *text = NULL;
    if (sidereon_ionex_to_ionex_text(bare, NULL, 0, &written, &required) != SIDEREON_STATUS_OK ||
        required == 0) {
        sidereon_ionex_free(bare);
        return fail("IONEX undeclared default: text length", 1);
    }
    text = (char *)malloc(required + 1);
    if (text == NULL) {
        sidereon_ionex_free(bare);
        return fail("IONEX undeclared default: allocate text", 1);
    }
    if (sidereon_ionex_to_ionex_text(bare, (uint8_t *)text, required, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != required) {
        free(text);
        sidereon_ionex_free(bare);
        return fail("IONEX undeclared default: write text", 1);
    }
    text[written] = '\0';
    if ((ionex_mapping_field(text, field, sizeof field) != NULL) !=
            SMOKE_A_IONEX_BARE_TEXT_MAPPING_RECORD ||
        strcmp(field, SMOKE_A_IONEX_BARE_TEXT_MAPPING_FIELD) != 0) {
        free(text);
        sidereon_ionex_free(bare);
        return fail("IONEX undeclared default writes a blank MAPPING FUNCTION", 1);
    }
    SidereonIonex *reparsed = NULL;
    if (sidereon_ionex_parse((const uint8_t *)text, written, &reparsed) != SIDEREON_STATUS_OK ||
        reparsed == NULL) {
        free(text);
        sidereon_ionex_free(bare);
        return fail("IONEX undeclared default: reparse", 1);
    }
    free(text);
    sidereon_ionex_free(bare);
    SidereonIonexHeader *reparsed_header = NULL;
    declaration = SIDEREON_IONEX_MAPPING_DECLARATION_KIND_DECLARED;
    if (sidereon_ionex_get_header(reparsed, &reparsed_header) != SIDEREON_STATUS_OK ||
        sidereon_ionex_header_get_mapping_declaration(reparsed_header, &declaration, &function) !=
            SIDEREON_STATUS_OK ||
        (declaration == SIDEREON_IONEX_MAPPING_DECLARATION_KIND_DECLARED) !=
            SMOKE_A_IONEX_BARE_TEXT_REPARSED_DECLARED) {
        sidereon_ionex_header_free(reparsed_header);
        sidereon_ionex_free(reparsed);
        return fail("IONEX undeclared default round-trips as ABSENT", 1);
    }
    sidereon_ionex_header_free(reparsed_header);
    sidereon_ionex_free(reparsed);

    /* An explicitly declared COSZ reverses all four. */
    SidereonIonexHeader *declared_header = NULL;
    if (sidereon_ionex_header_new(&declared_header) != SIDEREON_STATUS_OK ||
        sidereon_ionex_header_set_mapping_function(
            declared_header, SIDEREON_IONEX_MAPPING_FUNCTION_KIND_COS_Z, NULL, 0) !=
            SIDEREON_STATUS_OK) {
        sidereon_ionex_header_free(declared_header);
        return fail("IONEX declared COSZ: header", 1);
    }
    declaration = SIDEREON_IONEX_MAPPING_DECLARATION_KIND_ABSENT;
    function = SIDEREON_IONEX_MAPPING_FUNCTION_KIND_NO_MAPPING;
    if (sidereon_ionex_header_get_mapping_declaration(declared_header, &declaration, &function) !=
            SIDEREON_STATUS_OK ||
        declaration != SIDEREON_IONEX_MAPPING_DECLARATION_KIND_DECLARED ||
        function != SIDEREON_IONEX_MAPPING_FUNCTION_KIND_COS_Z) {
        sidereon_ionex_header_free(declared_header);
        return fail("IONEX declared COSZ reads back as declared", 1);
    }
    SidereonTecGridSamples declared_samples;
    fill_ionex_sample_grid(&declared_samples, epochs, lats, lons, tec, tec_present, rms, NULL,
                           declared_header);
    SidereonIonex *declared = NULL;
    if (sidereon_ionex_from_tec_grid_samples(&declared_samples, &declared) !=
            SIDEREON_STATUS_OK ||
        declared == NULL) {
        sidereon_ionex_header_free(declared_header);
        return fail("IONEX declared COSZ: build product", 1);
    }
    sidereon_ionex_header_free(declared_header);

    /* COSZ is the factor applied, so nothing is assumed, and the same value
     * comes back: the declaration selects the policy outcome, not the
     * arithmetic. */
    SidereonIonexSlantResultList *declared_rows = NULL;
    if (sidereon_ionex_slant_delay_results(declared, &request, 1, single_layer, &declared_rows) !=
            SIDEREON_STATUS_OK ||
        declared_rows == NULL) {
        sidereon_ionex_slant_result_list_free(declared_rows);
        sidereon_ionex_free(declared);
        return fail("IONEX declared COSZ: single-layer evaluate", 1);
    }
    SidereonIonexSlantRowResult declared_row;
    memset(&declared_row, 0, sizeof declared_row);
    const SaIonexRowExpected declared_expected =
        SA_IONEX_ROW_EXPECTED(SMOKE_A_IONEX_COSZ_SINGLE_LAYER);
    if (sidereon_ionex_slant_result_get_row(declared_rows, 0, &declared_row) !=
            SIDEREON_STATUS_OK ||
        sa_check_ionex_row(&declared_row, &declared_expected) != 0 ||
        declared_row.evaluation.delay_m != bare_delay_m) {
        sidereon_ionex_slant_result_list_free(declared_rows);
        sidereon_ionex_free(declared);
        return fail("IONEX declared COSZ assumes nothing", 1);
    }
    sidereon_ionex_slant_result_list_free(declared_rows);

    /* The DECLARED policy is satisfied where it refused the bare product. */
    SidereonIonexSlantResultList *strict_rows = NULL;
    if (sidereon_ionex_slant_delay_results(declared, &request, 1, declared_policy, &strict_rows) !=
            SIDEREON_STATUS_OK ||
        strict_rows == NULL) {
        sidereon_ionex_slant_result_list_free(strict_rows);
        sidereon_ionex_free(declared);
        return fail("IONEX declared COSZ: declared evaluate", 1);
    }
    SidereonIonexSlantRowResult strict_row;
    memset(&strict_row, 0, sizeof strict_row);
    const SaIonexRowExpected strict_expected = SA_IONEX_ROW_EXPECTED(SMOKE_A_IONEX_COSZ_DECLARED);
    if (sidereon_ionex_slant_result_get_row(strict_rows, 0, &strict_row) != SIDEREON_STATUS_OK ||
        sa_check_ionex_row(&strict_row, &strict_expected) != 0) {
        sidereon_ionex_slant_result_list_free(strict_rows);
        sidereon_ionex_free(declared);
        return fail("IONEX declared COSZ satisfies the DECLARED policy", 1);
    }
    sidereon_ionex_slant_result_list_free(strict_rows);

    /* And the record is written and read back. */
    if (sidereon_ionex_to_ionex_text(declared, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        required == 0) {
        sidereon_ionex_free(declared);
        return fail("IONEX declared COSZ: text length", 1);
    }
    text = (char *)malloc(required + 1);
    if (text == NULL) {
        sidereon_ionex_free(declared);
        return fail("IONEX declared COSZ: allocate text", 1);
    }
    if (sidereon_ionex_to_ionex_text(declared, (uint8_t *)text, required, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != required) {
        free(text);
        sidereon_ionex_free(declared);
        return fail("IONEX declared COSZ: write text", 1);
    }
    text[written] = '\0';
    sidereon_ionex_free(declared);
    if ((ionex_mapping_field(text, field, sizeof field) != NULL) !=
            SMOKE_A_IONEX_COSZ_TEXT_MAPPING_RECORD ||
        strcmp(field, SMOKE_A_IONEX_COSZ_TEXT_MAPPING_FIELD) != 0) {
        free(text);
        return fail("IONEX declared COSZ writes its record", 1);
    }
    SidereonIonex *declared_reparsed = NULL;
    if (sidereon_ionex_parse((const uint8_t *)text, written, &declared_reparsed) !=
            SIDEREON_STATUS_OK ||
        declared_reparsed == NULL) {
        free(text);
        return fail("IONEX declared COSZ: reparse", 1);
    }
    free(text);
    SidereonIonexHeader *declared_reparsed_header = NULL;
    declaration = SIDEREON_IONEX_MAPPING_DECLARATION_KIND_ABSENT;
    function = SIDEREON_IONEX_MAPPING_FUNCTION_KIND_NO_MAPPING;
    char code[16];
    if (sidereon_ionex_get_header(declared_reparsed, &declared_reparsed_header) !=
            SIDEREON_STATUS_OK ||
        sidereon_ionex_header_get_mapping_declaration(declared_reparsed_header, &declaration,
                                                      &function) != SIDEREON_STATUS_OK ||
        (declaration == SIDEREON_IONEX_MAPPING_DECLARATION_KIND_DECLARED) !=
            SMOKE_A_IONEX_COSZ_TEXT_REPARSED_DECLARED ||
        strcmp(sa_ionex_mapping_function_name(function), SMOKE_A_IONEX_COSZ_TEXT_REPARSED_FUNCTION) !=
            0 ||
        sidereon_ionex_header_get_mapping_function_code(declared_reparsed_header, (uint8_t *)code,
                                                        sizeof code, &written, &required) !=
            SIDEREON_STATUS_OK ||
        required != strlen(SMOKE_A_IONEX_COSZ_TEXT_REPARSED_CODE) || written != required ||
        memcmp(code, SMOKE_A_IONEX_COSZ_TEXT_REPARSED_CODE, required) != 0) {
        sidereon_ionex_header_free(declared_reparsed_header);
        sidereon_ionex_free(declared_reparsed);
        return fail("IONEX declared COSZ round-trips as COSZ", 1);
    }
    sidereon_ionex_header_free(declared_reparsed_header);
    sidereon_ionex_free(declared_reparsed);
    return 0;
}

/* The engine variant or axis label each C code stands for, as the binding maps
 * them (src/ionex.rs tec_grid_error_to_c, tec_grid_axis_to_c). */
static const char *sa_tec_grid_error_name(SidereonTecGridErrorKind kind) {
    switch (kind) {
    case SIDEREON_TEC_GRID_ERROR_KIND_NONE:
        return "";
    case SIDEREON_TEC_GRID_ERROR_KIND_AXES_TOO_SHORT:
        return "AxesTooShort";
    case SIDEREON_TEC_GRID_ERROR_KIND_AXES_NOT_INCREASING:
        return "AxesNotIncreasing";
    case SIDEREON_TEC_GRID_ERROR_KIND_DIMENSIONS_OVERFLOW:
        return "DimensionsOverflow";
    case SIDEREON_TEC_GRID_ERROR_KIND_VALUE_COUNT_MISMATCH:
        return "ValueCountMismatch";
    case SIDEREON_TEC_GRID_ERROR_KIND_INVALID_FIELD:
        return "InvalidField";
    case SIDEREON_TEC_GRID_ERROR_KIND_NODES_NOT_AVAILABLE:
        return "NodesNotAvailable";
    case SIDEREON_TEC_GRID_ERROR_KIND_OUT_OF_BOUNDS:
        return "OutOfBounds";
    case SIDEREON_TEC_GRID_ERROR_KIND_VALUE_NOT_FINITE:
    case SIDEREON_TEC_GRID_ERROR_KIND_UNKNOWN:
        return "?";
    }
    return "?";
}

static const char *sa_tec_grid_axis_label(SidereonTecGridAxis axis) {
    switch (axis) {
    case SIDEREON_TEC_GRID_AXIS_NONE:
        return "";
    case SIDEREON_TEC_GRID_AXIS_EPOCH:
        return "timestamp";
    case SIDEREON_TEC_GRID_AXIS_LATITUDE:
        return "latitude";
    case SIDEREON_TEC_GRID_AXIS_LONGITUDE:
        return "longitude";
    case SIDEREON_TEC_GRID_AXIS_UNKNOWN:
        return "?";
    }
    return "?";
}

/* One pierce-point query's expected outcome from smoke_a_tec_grid. */
typedef struct {
    bool ok;
    uint64_t vtec_bits;
    bool has_gap;
    const char *error;
    const char *axis;
    uint64_t axis_value_bits;
    const char *field;
    const char *reason;
} SaTecGridExpected;

#define SA_TEC_GRID_EXPECTED(P)                                                           \
    {                                                                                     \
        P##_OK, P##_VTEC_BITS, P##_HAS_GAP, P##_ERROR, P##_AXIS, P##_AXIS_VALUE_BITS,     \
            P##_FIELD, P##_REASON                                                         \
    }

/* A typed error against the engine's: its kind and, for an out-of-bounds
 * failure, the axis and the coordinate it names. */
static int sa_tec_grid_error_matches(const SidereonTecGridError *error,
                                     const SaTecGridExpected *expected) {
    if (strcmp(sa_tec_grid_error_name(error->kind), expected->error) != 0 ||
        strcmp(sa_tec_grid_axis_label(error->axis), expected->axis) != 0) {
        return 0;
    }
    if (expected->axis[0] != '\0') {
        return error->has_axis_value && f64_to_bits(error->axis_value) == expected->axis_value_bits;
    }
    return !error->has_axis_value;
}

/* The typed pierce-point route against the engine's outcome for the same
 * query. The status for a failure is the binding's (src/ionex.rs
 * tec_grid_error_status); NaN in place of a value is the binding's. */
static int sa_check_tec_grid_result(const SidereonTecGrid *grid, double lat_deg,
                                    SidereonStatus failure_status,
                                    const SaTecGridExpected *expected, const char *label) {
    SidereonTecGridResult *result = NULL;
    SidereonTecGridOutcome outcome;
    memset(&outcome, 0, sizeof outcome);
    int ok = sidereon_tec_grid_vtec_at_pierce_point_result(
                 grid, 0, lat_deg, 20.0, SIDEREON_IONEX_MISSING_NODE_POLICY_STRICT, &result) ==
                 SIDEREON_STATUS_OK &&
             result != NULL &&
             sidereon_tec_grid_result_get_outcome(result, &outcome) == SIDEREON_STATUS_OK &&
             outcome.is_ok == expected->ok && sa_tec_grid_error_matches(&outcome.error, expected);
    if (ok && expected->ok) {
        ok = outcome.status == SIDEREON_STATUS_OK && outcome.has_vtec &&
             f64_to_bits(outcome.vtec_tecu) == expected->vtec_bits &&
             outcome.degraded.has_gap == expected->has_gap;
    } else if (ok) {
        ok = outcome.status == failure_status && !outcome.has_vtec && isnan(outcome.vtec_tecu);
        if (ok && expected->field[0] != '\0') {
            char field[64];
            char reason[64];
            ok = read_tec_grid_text(sidereon_tec_grid_result_get_field, result, field,
                                    sizeof field) == 0 &&
                 strcmp(field, expected->field) == 0 &&
                 read_tec_grid_text(sidereon_tec_grid_result_get_reason, result, reason,
                                    sizeof reason) == 0 &&
                 strcmp(reason, expected->reason) == 0;
        }
    }
    sidereon_tec_grid_result_free(result);
    if (!ok) {
        fprintf(stderr, "TEC grid %s: typed route differs from the engine\n", label);
    }
    return ok ? 0 : 1;
}

/* The convenience pierce-point route against the engine's outcome. */
static int sa_check_tec_grid_value(const SidereonTecGrid *grid, double lat_deg,
                                   const SaTecGridExpected *expected, const char *label) {
    double vtec = 0.0;
    SidereonIonexNodeGap gap;
    SidereonTecGridError error;
    memset(&gap, 0, sizeof gap);
    memset(&error, 0, sizeof error);
    SidereonStatus status = sidereon_tec_grid_vtec_at_pierce_point(
        grid, 0, lat_deg, 20.0, SIDEREON_IONEX_MISSING_NODE_POLICY_STRICT, &vtec, &gap, &error);
    int ok = sa_tec_grid_error_matches(&error, expected) &&
             (expected->ok ? status == SIDEREON_STATUS_OK &&
                                 f64_to_bits(vtec) == expected->vtec_bits &&
                                 gap.has_gap == expected->has_gap
                           : status != SIDEREON_STATUS_OK);
    if (!ok) {
        fprintf(stderr, "TEC grid %s: convenience route differs from the engine\n", label);
    }
    return ok ? 0 : 1;
}

/* The engine clamps a pierce-point latitude to [-87.5, 87.5] degrees, and the
 * value that comes back is the one at the effective coordinate.
 *
 * The latitude axis here is [80.0, 89.0], so 89 degrees is a node of it: an
 * unclamped evaluation would return that node's value outright (7.0, the
 * third entry of `values`). The expected outcomes are sidereon-core's own
 * evaluations of the same queries (tests/valgen smoke_a_tec_grid): the
 * 89-degree query lands on the value a supplied 87.5 gives, with no gap and
 * no typed error. Both exported pierce-point routes reach the engine through
 * one helper, so each is checked, and a narrower axis shows the effective
 * coordinate is also what an out-of-bounds failure names. */
static int exercise_tec_grid_latitude_clamp(void) {
    const double epochs_ns[2] = {0.0, 1000.0};
    const double latitudes_deg[2] = {80.0, 89.0};
    const double narrow_latitudes_deg[2] = {80.0, 85.0};
    const double longitudes_deg[2] = {20.0, 60.0};
    /* Flat epoch-latitude-longitude order, longitude fastest. */
    const double values[8] = {1.0, 3.0, 7.0, 15.0, 2.0, 6.0, 14.0, 30.0};
    bool presence[8];
    for (size_t i = 0; i < 8; i++) {
        presence[i] = true;
    }
    const SaTecGridExpected at_89 = SA_TEC_GRID_EXPECTED(SMOKE_A_TEC_GRID_LAT_89);
    const SaTecGridExpected at_87_5 = SA_TEC_GRID_EXPECTED(SMOKE_A_TEC_GRID_LAT_87_5);
    const SaTecGridExpected narrow_89 = SA_TEC_GRID_EXPECTED(SMOKE_A_TEC_GRID_NARROW_LAT_89);

    SidereonTecGrid *grid = NULL;
    SidereonTecGridError error;
    memset(&error, 0, sizeof error);
    if (sidereon_tec_grid_new(epochs_ns, 2, latitudes_deg, 2, longitudes_deg, 2, values,
                              presence, 8, &grid, &error) != SIDEREON_STATUS_OK ||
        grid == NULL || error.kind != SIDEREON_TEC_GRID_ERROR_KIND_NONE) {
        sidereon_tec_grid_free(grid);
        return fail("TEC grid clamp: build grid", 1);
    }

    /* The clamp substitutes 87.5, so the caller's 89 does not reach the
     * interpolation, and the result is not the 89-degree node's 7.0. */
    if (sa_check_tec_grid_value(grid, 89.0, &at_89, "89 degrees") != 0 ||
        at_89.vtec_bits != at_87_5.vtec_bits || bits_to_f64(at_89.vtec_bits) == 7.0 ||
        sa_check_tec_grid_value(grid, 87.5, &at_87_5, "87.5 degrees") != 0 ||
        sa_check_tec_grid_result(grid, 89.0, SIDEREON_STATUS_SOLVE, &at_89, "89 degrees") != 0) {
        sidereon_tec_grid_free(grid);
        return fail_value("TEC grid clamps 89 degrees to 87.5 silently", 1);
    }
    sidereon_tec_grid_free(grid);

    /* On an axis narrower than the clamp band the query still fails, and the
     * coordinate the failure names is the effective one the engine reports. */
    SidereonTecGrid *narrow = NULL;
    if (sidereon_tec_grid_new(epochs_ns, 2, narrow_latitudes_deg, 2, longitudes_deg, 2, values,
                              presence, 8, &narrow, &error) != SIDEREON_STATUS_OK ||
        narrow == NULL) {
        sidereon_tec_grid_free(narrow);
        return fail("TEC grid clamp: build narrow grid", 1);
    }
    if (sa_check_tec_grid_result(narrow, 89.0, SIDEREON_STATUS_SOLVE, &narrow_89,
                                 "narrow 89 degrees") != 0 ||
        sa_check_tec_grid_value(narrow, 89.0, &narrow_89, "narrow 89 degrees") != 0) {
        sidereon_tec_grid_free(narrow);
        return fail_value("TEC grid out-of-bounds names the effective latitude", 1);
    }
    sidereon_tec_grid_free(narrow);
    return 0;
}

/* The clamp is a comparison, so it takes an infinite latitude and leaves a NaN
 * one alone.
 *
 * On the same [80.0, 89.0] axis, +INFINITY is above the upper bound and gives
 * the value the 89-degree query gives; -INFINITY is below the lower bound,
 * which that axis cannot reach, so it is an ordinary out-of-bounds failure
 * naming a finite coordinate rather than an infinity. NAN compares false
 * against both bounds, so no clamp applies and the engine's shared validation
 * refuses it, with the field and reason kept on the owned result. The expected
 * outcomes are sidereon-core's (smoke_a_tec_grid). The binding adds no
 * finite-angle check of its own. */
static int exercise_tec_grid_nonfinite_latitude(void) {
    const double epochs_ns[2] = {0.0, 1000.0};
    const double latitudes_deg[2] = {80.0, 89.0};
    const double longitudes_deg[2] = {20.0, 60.0};
    const double values[8] = {1.0, 3.0, 7.0, 15.0, 2.0, 6.0, 14.0, 30.0};
    bool presence[8];
    for (size_t i = 0; i < 8; i++) {
        presence[i] = true;
    }
    const SaTecGridExpected at_89 = SA_TEC_GRID_EXPECTED(SMOKE_A_TEC_GRID_LAT_89);
    const SaTecGridExpected plus_inf = SA_TEC_GRID_EXPECTED(SMOKE_A_TEC_GRID_LAT_PLUS_INF);
    const SaTecGridExpected minus_inf = SA_TEC_GRID_EXPECTED(SMOKE_A_TEC_GRID_LAT_MINUS_INF);
    const SaTecGridExpected nan_lat = SA_TEC_GRID_EXPECTED(SMOKE_A_TEC_GRID_LAT_NAN);

    SidereonTecGrid *grid = NULL;
    SidereonTecGridError error;
    memset(&error, 0, sizeof error);
    if (sidereon_tec_grid_new(epochs_ns, 2, latitudes_deg, 2, longitudes_deg, 2, values,
                              presence, 8, &grid, &error) != SIDEREON_STATUS_OK ||
        grid == NULL) {
        sidereon_tec_grid_free(grid);
        return fail("TEC grid non-finite latitude: build grid", 1);
    }

    if (sa_check_tec_grid_value(grid, INFINITY, &plus_inf, "+inf") != 0 ||
        plus_inf.vtec_bits != at_89.vtec_bits) {
        sidereon_tec_grid_free(grid);
        return fail_value("TEC grid clamps an infinite latitude", 1);
    }
    if (sa_check_tec_grid_result(grid, -INFINITY, SIDEREON_STATUS_SOLVE, &minus_inf, "-inf") != 0 ||
        !isfinite(bits_to_f64(minus_inf.axis_value_bits))) {
        sidereon_tec_grid_free(grid);
        return fail_value("TEC grid clamps a negative infinite latitude", 1);
    }
    if (sa_check_tec_grid_result(grid, NAN, SIDEREON_STATUS_INVALID_ARGUMENT, &nan_lat, "NaN") !=
        0) {
        sidereon_tec_grid_free(grid);
        return fail_value("TEC grid refuses a NaN latitude as a not-finite field", 1);
    }
    sidereon_tec_grid_free(grid);
    return 0;
}

/* The plain batch route clears its output before it reads anything else.
 *
 * sidereon_ionex_slant_delays promises a caller who hands it storage it can
 * write that the storage holds `count` zeroes on every failure the call
 * reports. A null product and a null request array are the two failures that
 * reach no engine at all, so each starts from its own sentinel the zeroes
 * cannot be mistaken for. The output pointer itself is outside that promise:
 * a null one with a nonzero count, and a count no slice can span, name no
 * storage to initialize, so nothing is written. */
/* Engine values for the checks from here to the end of the file, written by
 * tests/valgen (bins smoke_b_*) from sidereon-core. */
#include "smoke_b_constellation_pins.h"
#include "smoke_b_timescale_pins.h"
#include "smoke_b_geometry_pins.h"
#include "smoke_b_ionex_pins.h"
#include "smoke_b_staleness_pins.h"
#include "smoke_b_positioning_pins.h"
#include "smoke_b_tle_pins.h"

/* A selection outcome against sidereon-core's (tests/valgen bin
 * smoke_b_staleness). A refused selection leaves the binding's zeroed
 * metadata, which is not compared. */
static int staleness_matches(SidereonSelectionStatus status, const SidereonStalenessMetadata *meta,
                             const SmokeBStalenessPin *pin) {
    if (status != pin->status) {
        return 0;
    }
    if (status != SIDEREON_SELECTION_STATUS_OK) {
        return 1;
    }
    return meta->kind == pin->kind &&
           f64_to_bits(meta->requested_epoch_j2000_s) == pin->requested_bits &&
           f64_to_bits(meta->source_epoch_j2000_s) == pin->source_bits &&
           f64_to_bits(meta->staleness_s) == pin->staleness_s_bits &&
           f64_to_bits(meta->staleness_days) == pin->staleness_days_bits;
}

static int missing_matches(const SidereonIonexMissingNodes *got, const SmokeBMissingPin *pin) {
    return got->has_missing == pin->has_missing && got->map_number == pin->map_number &&
           got->lat_index == pin->lat_index && got->lon_index == pin->lon_index &&
           got->lon_index_next == pin->lon_index_next && got->missing[0] == pin->missing[0] &&
           got->missing[1] == pin->missing[1] && got->missing[2] == pin->missing[2] &&
           got->missing[3] == pin->missing[3];
}

static int gap_matches(const SidereonIonexNodeGap *got, const SmokeBGapPin *pin) {
    return got->has_gap == pin->has_gap && missing_matches(&got->earlier, &pin->earlier) &&
           missing_matches(&got->later, &pin->later);
}

/* A slant outcome against sidereon-core's (tests/valgen bin smoke_b_ionex).
 * A refused evaluation is the binding's own form (NaN, not valid); every
 * other field is the engine's. `error` may be NULL where the route writes
 * none. */
static int slant_matches(SidereonStatus status, const SidereonIonexSlantDelayEvaluation *eval,
                         const SidereonIonexSlantError *error, const SmokeBSlantPin *pin) {
    if (status != pin->status) {
        return 0;
    }
    if (pin->ok) {
        if (f64_to_bits(eval->delay_m) != pin->delay_bits ||
            eval->status.is_valid != pin->is_valid || eval->status.has_held != pin->has_held ||
            eval->status.coverage_error != pin->coverage_error ||
            eval->status.has_degraded != pin->has_degraded ||
            !gap_matches(&eval->status.gap, &pin->gap) ||
            eval->status.has_assumed_mapping != pin->has_assumed_mapping ||
            eval->status.assumed_mapping != pin->assumed_mapping) {
            return 0;
        }
    } else if (eval->status.is_valid || !isnan(eval->delay_m)) {
        return 0;
    }
    if (error == NULL) {
        return 1;
    }
    return error->kind == pin->error_kind && error->coverage_error == pin->error_coverage &&
           error->has_gap == pin->error_has_gap && gap_matches(&error->gap, &pin->error_gap) &&
           error->refusal == pin->refusal && error->refusal_map_number == pin->refusal_map_number &&
           error->refusal_lat_index == pin->refusal_lat_index &&
           error->refusal_lon_index == pin->refusal_lon_index &&
           error->has_mapping_declaration == pin->has_mapping_declaration &&
           (!pin->has_mapping_declaration ||
            error->mapping_declaration == pin->mapping_declaration) &&
           error->has_mapping_function == pin->has_mapping_function &&
           (!pin->has_mapping_function || error->mapping_function == pin->mapping_function);
}

static int row_matches(const SidereonIonexSlantRowResult *row, const SmokeBSlantPin *pin) {
    return row->is_ok == pin->ok &&
           slant_matches(row->status, &row->evaluation, &row->error, pin);
}

static int exercise_ionex_batch_output_zeroing(void) {
    const double epochs[2] = {0.0, 3600.0};
    const double lats[2] = {40.0, -40.0};
    const double lons[2] = {-20.0, 20.0};
    double tec[8];
    bool tec_present[8];
    double rms[8];
    for (size_t i = 0; i < 8; i++) {
        tec[i] = 10.0;
        tec_present[i] = true;
        rms[i] = 1.0;
    }

    SidereonTecGridSamples samples;
    fill_ionex_sample_grid(&samples, epochs, lats, lons, tec, tec_present, rms, NULL, NULL);
    SidereonIonex *ionex = NULL;
    if (!SMOKE_B_IONEX_UNIFORM_BUILD_OK ||
        sidereon_ionex_from_tec_grid_samples(&samples, &ionex) != SIDEREON_STATUS_OK ||
        ionex == NULL) {
        sidereon_ionex_free(ionex);
        return fail("IONEX batch zeroing: build product", 1);
    }

    SidereonIonexSlantRequest requests[2];
    for (size_t i = 0; i < 2; i++) {
        requests[i].lat_deg = 0.0;
        requests[i].lon_deg = 0.0;
        requests[i].azimuth_deg = 0.0;
        requests[i].elevation_deg = 85.0;
        requests[i].epoch_j2000_s = 0;
        requests[i].frequency_hz = 1575420000.0;
    }

    /* A null product, from a sentinel that is neither zero nor NaN. */
    double delays[2] = {-12345.5, 6789.25};
    if (sidereon_ionex_slant_delays(NULL, requests, 2, delays) != SIDEREON_STATUS_NULL_POINTER ||
        delays[0] != 0.0 || delays[1] != 0.0) {
        sidereon_ionex_free(ionex);
        return fail("IONEX batch null product leaves the output zero", 1);
    }

    /* A null request array with a nonzero count, from a different sentinel. */
    delays[0] = -4242.25;
    delays[1] = 98765.5;
    if (sidereon_ionex_slant_delays(ionex, NULL, 2, delays) != SIDEREON_STATUS_NULL_POINTER ||
        delays[0] != 0.0 || delays[1] != 0.0) {
        sidereon_ionex_free(ionex);
        return fail("IONEX batch null requests leaves the output zero", 1);
    }

    /* A null output with a nonzero count names no storage, so the refusal
     * claims none was initialized. Neither does a count no slice can span:
     * that array comes back holding exactly what it held. */
    double untouched[2] = {-1.0, -2.0};
    if (sidereon_ionex_slant_delays(ionex, requests, 2, NULL) != SIDEREON_STATUS_NULL_POINTER ||
        sidereon_ionex_slant_delays(ionex, requests, (size_t)-1, untouched) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        untouched[0] != -1.0 || untouched[1] != -2.0) {
        sidereon_ionex_free(ionex);
        return fail("IONEX batch unusable output initializes nothing", 1);
    }

    sidereon_ionex_free(ionex);
    return 0;
}

/* A custom code may be empty, and that is not the absent case.
 *
 * sidereon_ionex_header_set_mapping_function accepts
 * SIDEREON_IONEX_MAPPING_FUNCTION_KIND_OTHER with a zero-length code, the
 * product keeps it, and the single-layer factor reports it without reading the
 * text, so the row succeeds with SIDEREON_IONEX_ASSUMED_MAPPING_KIND_OTHER and
 * a required code length of zero. A product that declares nothing reaches the
 * same zero length under SIDEREON_IONEX_ASSUMED_MAPPING_KIND_ABSENT, so the
 * typed kind is the only thing that tells the two apart. Serialization is the
 * one route that refuses a blank code; it is checked elsewhere and does not
 * reach construction or evaluation. */
static int exercise_ionex_blank_custom_mapping_code(void) {
    const double epochs[2] = {0.0, 3600.0};
    const double lats[2] = {40.0, -40.0};
    const double lons[2] = {-20.0, 20.0};
    double tec[8];
    bool tec_present[8];
    double rms[8];
    for (size_t i = 0; i < 8; i++) {
        tec[i] = 10.0;
        tec_present[i] = true;
        rms[i] = 1.0;
    }

    SidereonIonexSlantRequest request;
    request.lat_deg = 0.0;
    request.lon_deg = 0.0;
    request.azimuth_deg = 0.0;
    request.elevation_deg = 85.0;
    request.epoch_j2000_s = 1800;
    request.frequency_hz = 1575420000.0;
    const SidereonIonexSlantPolicy single_layer = sidereon_ionex_slant_policy_init(
        SIDEREON_IONEX_COVERAGE_POLICY_STRICT, SIDEREON_IONEX_MISSING_NODE_POLICY_STRICT,
        SIDEREON_IONEX_MAPPING_POLICY_SINGLE_LAYER);

    SidereonIonexHeader *header = NULL;
    if (sidereon_ionex_header_new(&header) != SIDEREON_STATUS_OK || header == NULL) {
        return fail("IONEX blank custom code: header", 1);
    }
    /* A zero length reads no bytes, so the code pointer is never touched. */
    if (sidereon_ionex_header_set_mapping_function(
            header, SIDEREON_IONEX_MAPPING_FUNCTION_KIND_OTHER, NULL, 0) !=
        SIDEREON_STATUS_OK) {
        sidereon_ionex_header_free(header);
        return fail("IONEX blank custom code is constructible", 1);
    }

    SidereonTecGridSamples samples;
    fill_ionex_sample_grid(&samples, epochs, lats, lons, tec, tec_present, rms, NULL, header);
    SidereonIonex *product = NULL;
    if (sidereon_ionex_from_tec_grid_samples(&samples, &product) != SIDEREON_STATUS_OK ||
        product == NULL) {
        sidereon_ionex_header_free(header);
        return fail("IONEX blank custom code: build product", 1);
    }

    SidereonIonexSlantResultList *rows = NULL;
    if (sidereon_ionex_slant_delay_results(product, &request, 1, single_layer, &rows) !=
            SIDEREON_STATUS_OK ||
        rows == NULL) {
        sidereon_ionex_slant_result_list_free(rows);
        sidereon_ionex_free(product);
        sidereon_ionex_header_free(header);
        return fail("IONEX blank custom code: evaluate", 1);
    }
    SidereonIonexSlantRowResult row;
    memset(&row, 0, sizeof row);
    /* The row is sidereon-core's evaluation of the product declaring an empty
     * custom code (tests/valgen bin smoke_b_ionex). */
    if (sidereon_ionex_slant_result_get_row(rows, 0, &row) != SIDEREON_STATUS_OK ||
        !row_matches(&row, &SMOKE_B_IONEX_BLANK_CODE)) {
        sidereon_ionex_slant_result_list_free(rows);
        sidereon_ionex_free(product);
        sidereon_ionex_header_free(header);
        return fail("IONEX empty custom code evaluates as Other", 1);
    }

    /* Both handles the code could otherwise be read from are released, and the
     * row still answers for itself. */
    sidereon_ionex_free(product);
    sidereon_ionex_header_free(header);

    size_t written = 0, required = 0;
    if (sidereon_ionex_slant_result_get_mapping_code(rows, 0, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        required != 0 || written != 0) {
        sidereon_ionex_slant_result_list_free(rows);
        return fail("IONEX empty custom code reports a zero length", 1);
    }
    sidereon_ionex_slant_result_list_free(rows);

    /* A product that declares nothing: the same zero length, a different kind. */
    SidereonTecGridSamples bare_samples;
    fill_ionex_sample_grid(&bare_samples, epochs, lats, lons, tec, tec_present, rms, NULL, NULL);
    SidereonIonex *bare_product = NULL;
    if (sidereon_ionex_from_tec_grid_samples(&bare_samples, &bare_product) !=
            SIDEREON_STATUS_OK ||
        bare_product == NULL) {
        return fail("IONEX blank custom code: build undeclared product", 1);
    }
    SidereonIonexSlantResultList *bare_rows = NULL;
    if (sidereon_ionex_slant_delay_results(bare_product, &request, 1, single_layer, &bare_rows) !=
            SIDEREON_STATUS_OK ||
        bare_rows == NULL) {
        sidereon_ionex_slant_result_list_free(bare_rows);
        sidereon_ionex_free(bare_product);
        return fail("IONEX blank custom code: evaluate undeclared product", 1);
    }
    SidereonIonexSlantRowResult bare_row;
    memset(&bare_row, 0, sizeof bare_row);
    size_t bare_required = 0;
    if (sidereon_ionex_slant_result_get_row(bare_rows, 0, &bare_row) != SIDEREON_STATUS_OK ||
        !row_matches(&bare_row, &SMOKE_B_IONEX_BARE) ||
        sidereon_ionex_slant_result_get_mapping_code(bare_rows, 0, NULL, 0, &written,
                                                     &bare_required) != SIDEREON_STATUS_OK ||
        bare_required != required ||
        bare_row.evaluation.status.assumed_mapping == row.evaluation.status.assumed_mapping) {
        sidereon_ionex_slant_result_list_free(bare_rows);
        sidereon_ionex_free(bare_product);
        return fail("IONEX absent declaration is a kind, not a length", 1);
    }

    sidereon_ionex_slant_result_list_free(bare_rows);
    sidereon_ionex_free(bare_product);
    return 0;
}

/* The fallible writer refuses a MAPPING FUNCTION code that does not read back
 * as itself, and a refusal writes nothing. Which codes the writer refuses, and
 * its text, are sidereon-core's (tests/valgen bin smoke_b_ionex).
 *
 * A custom code is constructible and evaluable whatever its text, but the
 * `2X,A4` field holds only a code of at most four characters, without blanks,
 * that is not one of the codes the spec names. A blank code, the standard COSZ
 * given as a custom code, and a five-character code each fail
 * sidereon_ionex_to_ionex_text with a non-OK status and a required length of
 * zero. A four-character custom code writes, so the refusals are not merely
 * what every custom code gets. */
static int exercise_ionex_writer_refusal(void) {
    const double epochs[2] = {0.0, 3600.0};
    const double lats[2] = {40.0, -40.0};
    const double lons[2] = {-20.0, 20.0};
    double tec[8];
    bool tec_present[8];
    double rms[8];
    for (size_t i = 0; i < 8; i++) {
        tec[i] = 10.0;
        tec_present[i] = true;
        rms[i] = 1.0;
    }
    static const char *const codes[4] = {"", "COSZ", "ABCDE", "ABCD"};
    for (size_t c = 0; c < 4; c++) {
        const bool writes = SMOKE_B_IONEX_WRITER_CODE_OK[c];
        SidereonIonexHeader *header = NULL;
        if (sidereon_ionex_header_new(&header) != SIDEREON_STATUS_OK || header == NULL ||
            sidereon_ionex_header_set_mapping_function(
                header, SIDEREON_IONEX_MAPPING_FUNCTION_KIND_OTHER, (const uint8_t *)codes[c],
                strlen(codes[c])) != SIDEREON_STATUS_OK) {
            sidereon_ionex_header_free(header);
            return fail("IONEX writer refusal: header", 1);
        }
        SidereonTecGridSamples samples;
        fill_ionex_sample_grid(&samples, epochs, lats, lons, tec, tec_present, rms, NULL, header);
        SidereonIonex *product = NULL;
        if (sidereon_ionex_from_tec_grid_samples(&samples, &product) != SIDEREON_STATUS_OK ||
            product == NULL) {
            sidereon_ionex_header_free(header);
            return fail("IONEX writer refusal: build product", 1);
        }
        sidereon_ionex_header_free(header);

        size_t written = SIZE_MAX, required = SIZE_MAX;
        SidereonStatus st = sidereon_ionex_to_ionex_text(product, NULL, 0, &written, &required);
        if (writes) {
            if (st != SIDEREON_STATUS_OK || required == 0 || written != 0) {
                sidereon_ionex_free(product);
                return fail("IONEX writer writes a four-character custom code", 1);
            }
        } else {
            char buf[16];
            memset(buf, 7, sizeof buf);
            size_t buf_written = SIZE_MAX, buf_required = SIZE_MAX;
            if (st != SMOKE_B_IONEX_WRITER_CODE_STATUS[c] || required != 0 || written != 0 ||
                sidereon_ionex_to_ionex_text(product, (uint8_t *)buf, sizeof buf, &buf_written,
                                             &buf_required) != SMOKE_B_IONEX_WRITER_CODE_STATUS[c] ||
                buf_required != 0 || buf_written != 0 || buf[0] != 7 ||
                !last_error_contains(SMOKE_B_IONEX_WRITER_CODE_ERROR[c])) {
                sidereon_ionex_free(product);
                return fail("IONEX writer refuses a code that does not read back", 1);
            }
        }
        sidereon_ionex_free(product);
    }
    return 0;
}

/* The height half of the absent-versus-all-missing distinction: a product
 * whose height stack is declared with no value at any node keeps that stack,
 * with the full flattened count and every presence flag false, while a product
 * without a height stack reports a count of zero. */
static int exercise_ionex_height_presence_distinction(void) {
    const double epochs[2] = {0.0, 3600.0};
    const double lats[2] = {40.0, -40.0};
    const double lons[2] = {-20.0, 20.0};
    double tec[8];
    bool tec_present[8];
    double rms[8];
    double heights[8];
    bool heights_missing[8];
    for (size_t i = 0; i < 8; i++) {
        tec[i] = 10.0;
        tec_present[i] = true;
        rms[i] = 1.0;
        heights[i] = 0.0;
        heights_missing[i] = false;
    }

    SidereonTecGridSamples samples;
    fill_ionex_sample_grid(&samples, epochs, lats, lons, tec, tec_present, rms, NULL, NULL);
    SidereonIonex *without = NULL;
    SidereonTecGridSamplesInfo info;
    size_t written = 0, required = 1;
    memset(&info, 0, sizeof info);
    if (sidereon_ionex_from_tec_grid_samples(&samples, &without) != SIDEREON_STATUS_OK ||
        without == NULL ||
        sidereon_ionex_tec_grid_samples_info(without, &info) != SIDEREON_STATUS_OK ||
        info.has_height_maps != SMOKE_B_IONEX_NO_HEIGHT_HAS || info.height_map_value_count != 0 ||
        sidereon_ionex_tec_grid_samples_height_presence(without, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        required != 0) {
        sidereon_ionex_free(without);
        return fail("IONEX absent height map reports no values", 1);
    }
    sidereon_ionex_free(without);

    samples.has_height_maps = true;
    samples.height_maps_km = heights;
    samples.height_maps_present = heights_missing;
    samples.height_map_value_count = 8;
    SidereonIonex *with = NULL;
    if (sidereon_ionex_from_tec_grid_samples(&samples, &with) != SIDEREON_STATUS_OK ||
        with == NULL) {
        sidereon_ionex_free(with);
        return fail("IONEX all-missing height map: build product", 1);
    }
    memset(&info, 0, sizeof info);
    double read_values[8];
    bool read_presence[8];
    if (sidereon_ionex_tec_grid_samples_info(with, &info) != SIDEREON_STATUS_OK ||
        info.has_height_maps != SMOKE_B_IONEX_ALL_MISSING_HEIGHT_HAS ||
        info.height_map_value_count != SMOKE_B_IONEX_ALL_MISSING_HEIGHT_COUNT ||
        sidereon_ionex_tec_grid_samples_height_maps_km(with, read_values, 8, &written,
                                                       &required) != SIDEREON_STATUS_OK ||
        required != 8 ||
        sidereon_ionex_tec_grid_samples_height_presence(with, read_presence, 8, &written,
                                                        &required) != SIDEREON_STATUS_OK ||
        required != 8) {
        sidereon_ionex_free(with);
        return fail("IONEX all-missing height map is not an absent one", 1);
    }
    for (size_t i = 0; i < 8; i++) {
        if (read_presence[i] != SMOKE_B_IONEX_ALL_MISSING_HEIGHT_PRESENT[i] ||
            (!read_presence[i] && !isnan(read_values[i]))) {
            sidereon_ionex_free(with);
            return fail("IONEX all-missing height node holds no value", 1);
        }
    }
    sidereon_ionex_free(with);
    return 0;
}

/* A product with a non-available TEC node, and products with height maps,
 * evaluated through the slant route.
 *
 * The query sits between the two maps, so both are weighted, and its pierce
 * point lies in the one cell the 2x2 grid has, so every node is weighted. The
 * first node of the first map holds no value, and later the second height
 * node none. Every outcome is sidereon-core's (tests/valgen bin
 * smoke_b_ionex); the C side adds that the renormalized value differs from
 * the complete one and that uniform zero heights give the complete one. */
static int exercise_ionex_missing_nodes_and_heights(void) {
    const double epochs[2] = {0.0, 3600.0};
    const double lats[2] = {40.0, -40.0};
    const double lons[2] = {-20.0, 20.0};
    double tec[8] = {10.0, 12.0, 14.0, 16.0, 20.0, 22.0, 24.0, 26.0};
    bool tec_present[8];
    double rms[8];
    double heights[8];
    bool height_present[8];
    for (size_t i = 0; i < 8; i++) {
        tec_present[i] = true;
        rms[i] = 1.0;
        heights[i] = 0.0;
        height_present[i] = true;
    }
    const double f_l1 = 1575420000.0;
    const SidereonIonexSlantPolicy strict = sidereon_ionex_slant_policy_default();
    const SidereonIonexSlantPolicy renormalize = sidereon_ionex_slant_policy_init(
        SIDEREON_IONEX_COVERAGE_POLICY_STRICT, SIDEREON_IONEX_MISSING_NODE_POLICY_RENORMALIZE,
        SIDEREON_IONEX_MAPPING_POLICY_SINGLE_LAYER);

    /* The complete product first, as the reference delay. */
    SidereonTecGridSamples samples;
    fill_ionex_sample_grid(&samples, epochs, lats, lons, tec, tec_present, rms, NULL, NULL);
    SidereonIonex *complete = NULL;
    SidereonIonexSlantDelayEvaluation eval;
    SidereonIonexSlantError error;
    if (sidereon_ionex_from_tec_grid_samples(&samples, &complete) != SIDEREON_STATUS_OK ||
        complete == NULL) {
        sidereon_ionex_free(complete);
        return fail("IONEX complete product", 1);
    }
    SidereonStatus eval_status = sidereon_ionex_slant_delay_with_policy(
        complete, 0.0, 0.0, 0.0, 85.0, 1800, f_l1, strict, &eval, &error);
    if (!slant_matches(eval_status, &eval, &error, &SMOKE_B_IONEX_COMPLETE)) {
        sidereon_ionex_free(complete);
        return fail("IONEX complete product delay", 1);
    }
    const double complete_delay_m = eval.delay_m;
    sidereon_ionex_free(complete);

    /* One non-available node. */
    tec_present[0] = false;
    fill_ionex_sample_grid(&samples, epochs, lats, lons, tec, tec_present, rms, NULL, NULL);
    SidereonIonex *sparse = NULL;
    if (sidereon_ionex_from_tec_grid_samples(&samples, &sparse) != SIDEREON_STATUS_OK ||
        sparse == NULL) {
        sidereon_ionex_free(sparse);
        return fail("IONEX missing-node product", 1);
    }
    eval_status = sidereon_ionex_slant_delay_with_policy(sparse, 0.0, 0.0, 0.0, 85.0, 1800, f_l1,
                                                         strict, &eval, &error);
    if (!slant_matches(eval_status, &eval, &error, &SMOKE_B_IONEX_SPARSE_STRICT)) {
        sidereon_ionex_free(sparse);
        return fail("IONEX strict policy refuses a weighted non-available node", 1);
    }
    eval_status = sidereon_ionex_slant_delay_with_policy(sparse, 0.0, 0.0, 0.0, 85.0, 1800, f_l1,
                                                         renormalize, &eval, &error);
    if (!slant_matches(eval_status, &eval, &error, &SMOKE_B_IONEX_SPARSE_RENORMALIZE) ||
        eval.delay_m == complete_delay_m) {
        sidereon_ionex_free(sparse);
        return fail("IONEX renormalizing policy marks the value degraded", 1);
    }
    sidereon_ionex_free(sparse);
    tec_present[0] = true;

    /* Uniform zero heights: the single-layer height is HGT1, as with none. */
    fill_ionex_sample_grid(&samples, epochs, lats, lons, tec, tec_present, rms, NULL, NULL);
    samples.has_height_maps = true;
    samples.height_maps_km = heights;
    samples.height_maps_present = height_present;
    samples.height_map_value_count = 8;
    SidereonIonex *heighted = NULL;
    if (sidereon_ionex_from_tec_grid_samples(&samples, &heighted) != SIDEREON_STATUS_OK ||
        heighted == NULL) {
        sidereon_ionex_free(heighted);
        return fail("IONEX uniform zero height product", 1);
    }
    eval_status = sidereon_ionex_slant_delay_with_policy(heighted, 0.0, 0.0, 0.0, 85.0, 1800,
                                                         f_l1, strict, &eval, &error);
    if (!slant_matches(eval_status, &eval, &error, &SMOKE_B_IONEX_HEIGHTED) ||
        eval.delay_m != complete_delay_m) {
        sidereon_ionex_free(heighted);
        return fail("IONEX uniform zero height keeps the shell height", 1);
    }
    sidereon_ionex_free(heighted);

    /* A height given as non-available is refused by map and node. */
    height_present[1] = false;
    SidereonIonex *unknown_height = NULL;
    if (sidereon_ionex_from_tec_grid_samples(&samples, &unknown_height) != SIDEREON_STATUS_OK ||
        unknown_height == NULL) {
        sidereon_ionex_free(unknown_height);
        return fail("IONEX non-available height product", 1);
    }
    eval_status = sidereon_ionex_slant_delay_with_policy(unknown_height, 0.0, 0.0, 0.0, 85.0, 0,
                                                         f_l1, strict, &eval, &error);
    if (!slant_matches(eval_status, &eval, &error, &SMOKE_B_IONEX_UNKNOWN_HEIGHT)) {
        sidereon_ionex_free(unknown_height);
        return fail("IONEX non-available height is refused by name", 1);
    }
    sidereon_ionex_free(unknown_height);
    return 0;
}

/* An IONEX map epoch is a whole second. A double holding a fraction of one is
 * refused by index with a typed kind, never rounded; the integer epoch buffer
 * states seconds past 2^53 that no double holds, and every output keeps them. */
static int exercise_ionex_whole_second_epochs(void) {
    const double lats[2] = {40.0, -40.0};
    const double lons[2] = {-20.0, 20.0};
    double tec[8];
    bool tec_present[8];
    double rms[8];
    for (size_t i = 0; i < 8; i++) {
        tec[i] = 10.0;
        tec_present[i] = true;
        rms[i] = 1.0;
    }

    const double fractional[2] = {0.0, 3600.5};
    SidereonTecGridSamples samples;
    fill_ionex_sample_grid(&samples, fractional, lats, lons, tec, tec_present, rms, NULL, NULL);
    SidereonIonex *refused = (SidereonIonex *)(uintptr_t)1;
    if (sidereon_ionex_from_tec_grid_samples(&samples, &refused) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        refused != NULL || !last_error_contains("samples.map_epochs_j2000_s[1]")) {
        return fail("IONEX fractional epoch is refused by index", 1);
    }
    SidereonTecSamplesResult *result = NULL;
    SidereonTecSamplesOutcome outcome;
    memset(&outcome, 0, sizeof outcome);
    char text[256];
    size_t written = 0, required = 0;
    refused = (SidereonIonex *)(uintptr_t)1;
    if (sidereon_ionex_from_tec_grid_samples_result(&samples, &refused, &result) !=
            SIDEREON_STATUS_OK ||
        refused != NULL || result == NULL ||
        sidereon_tec_samples_result_get_outcome(result, &outcome) != SIDEREON_STATUS_OK ||
        outcome.is_ok || outcome.status != SIDEREON_STATUS_INVALID_ARGUMENT ||
        outcome.error.kind != SIDEREON_TEC_SAMPLES_ERROR_KIND_EPOCH_NOT_REPRESENTABLE ||
        outcome.error.input != SIDEREON_TEC_SAMPLES_INPUT_MAP_EPOCH || !outcome.error.has_index ||
        outcome.error.index != 1 ||
        sidereon_tec_samples_result_get_message(result, (uint8_t *)text, sizeof text - 1,
                                                &written, &required) != SIDEREON_STATUS_OK ||
        written != required) {
        sidereon_tec_samples_result_free(result);
        return fail("IONEX fractional epoch typed refusal", 1);
    }
    text[written] = '\0';
    sidereon_tec_samples_result_free(result);
    if (strstr(text, "samples.map_epochs_j2000_s[1]") == NULL ||
        strstr(text, "is not a whole second") == NULL) {
        return fail("IONEX fractional epoch owned text", 1);
    }

    /* 2^53 + 1 and an hour later, through the integer buffer. */
    const int64_t far[2] = {INT64_C(9007199254740993), INT64_C(9007199254744593)};
    fill_ionex_sample_grid(&samples, NULL, lats, lons, tec, tec_present, rms, NULL, NULL);
    samples.map_epochs_j2000_whole_s = far;
    SidereonIonex *product = NULL;
    if (!SMOKE_B_IONEX_FAR_BUILD_OK ||
        sidereon_ionex_from_tec_grid_samples(&samples, &product) != SIDEREON_STATUS_OK ||
        product == NULL) {
        sidereon_ionex_free(product);
        return fail("IONEX whole-second epochs past 2^53", 1);
    }
    int64_t axis[2] = {0, 0};
    double doubles[2] = {0.0, 0.0};
    if (sidereon_ionex_map_epochs_j2000_s(product, axis, 2, &written, &required) !=
            SIDEREON_STATUS_OK ||
        axis[0] != SMOKE_B_IONEX_FAR_EPOCH0 || axis[1] != SMOKE_B_IONEX_FAR_EPOCH1 ||
        sidereon_ionex_tec_grid_samples_epochs_j2000_s(product, doubles, 2, &written,
                                                       &required) != SIDEREON_STATUS_OK ||
        doubles[0] != (double)far[0] || doubles[1] != (double)far[1]) {
        sidereon_ionex_free(product);
        return fail("IONEX epochs past 2^53 keep every second", 1);
    }
    SidereonTecSample nodes[8];
    if (sidereon_ionex_tec_samples(product, nodes, 8, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != 8 || !nodes[0].has_epoch_j2000_whole_s ||
        nodes[0].epoch_j2000_whole_s != SMOKE_B_IONEX_FAR_EPOCH0 ||
        nodes[7].epoch_j2000_whole_s != SMOKE_B_IONEX_FAR_EPOCH1) {
        sidereon_ionex_free(product);
        return fail("IONEX node samples carry whole-second epochs", 1);
    }
    sidereon_ionex_free(product);
    return 0;
}

/* Every writable output of a call is cleared before any output is validated,
 * so a null partner never leaves another output holding the caller's value. */
static int exercise_ionex_output_clearing(const SidereonIonex *ionex) {
    SidereonIonexSlantError error;
    memset(&error, 0, sizeof error);
    error.kind = SIDEREON_IONEX_SLANT_ERROR_KIND_UNKNOWN;
    if (sidereon_ionex_slant_delay_with_policy(ionex, 0.0, 0.0, 0.0, 45.0, 0, 1575420000.0,
                                               sidereon_ionex_slant_policy_default(), NULL,
                                               &error) != SIDEREON_STATUS_NULL_POINTER ||
        error.kind != SIDEREON_IONEX_SLANT_ERROR_KIND_NONE) {
        return fail("IONEX slant null output clears the error detail", 1);
    }

    SidereonIonexHeader *header = NULL;
    if (sidereon_ionex_get_header(ionex, &header) != SIDEREON_STATUS_OK || header == NULL) {
        return fail("IONEX output clearing: header", 1);
    }
    bool has_count = true;
    uint32_t count = 77;
    if (sidereon_ionex_header_get_satellite_count(header, NULL, &has_count) !=
            SIDEREON_STATUS_NULL_POINTER ||
        has_count ||
        sidereon_ionex_header_get_maps_in_file(header, &count, NULL) !=
            SIDEREON_STATUS_NULL_POINTER ||
        count != 0) {
        sidereon_ionex_header_free(header);
        return fail("IONEX has-count pair clears its writable half", 1);
    }
    sidereon_ionex_header_free(header);

    size_t required = 99;
    if (sidereon_ionex_lat_nodes_deg(ionex, NULL, 0, NULL, &required) !=
            SIDEREON_STATUS_NULL_POINTER ||
        required != 0) {
        return fail("IONEX copy route clears the required count", 1);
    }

    size_t skipped = 5;
    if (sidereon_ionex_skipped_records(ionex, &skipped) != SIDEREON_STATUS_OK ||
        sidereon_ionex_skipped_records(NULL, &skipped) != SIDEREON_STATUS_NULL_POINTER ||
        skipped != 0) {
        return fail("sidereon_ionex_skipped_records", 1);
    }
    return 0;
}

static int exercise_ionex_3_0_surface(const SidereonIonex *ionex, const char *ionex_path) {
    const double f_l1 = 1575420000.0;
    /* The generated header names every policy tag. These are short local
     * aliases for the call sites below, not values of their own. */
    const uint32_t MISSING_NODE_STRICT = SIDEREON_IONEX_MISSING_NODE_POLICY_STRICT;
    const uint32_t MISSING_NODE_RENORMALIZE = SIDEREON_IONEX_MISSING_NODE_POLICY_RENORMALIZE;
    const uint32_t MAPPING_DECLARED = SIDEREON_IONEX_MAPPING_POLICY_DECLARED;
    const uint32_t MAPPING_SINGLE_LAYER = SIDEREON_IONEX_MAPPING_POLICY_SINGLE_LAYER;
    size_t written = 0, required = 0;

    /* ---- parse_with_warnings, its owned list and its accessors ---- */
    size_t len = 0;
    uint8_t *bytes = read_file(ionex_path, &len);
    if (bytes == NULL) {
        return fail("ionex 3.0: read IONEX file", 1);
    }
    SidereonIonex *warned = NULL;
    SidereonIonexWarningList *warnings = NULL;
    SidereonStatus st = sidereon_ionex_parse_with_warnings(bytes, len, &warned, &warnings);

    /* A null output slot still clears the partner slot the call can write.
     * Both orders are checked: only one of them is the slot the call looks at
     * first, and the other is the one the earlier order left stale. These
     * bytes parse, so the refusal is the null slot and nothing else, and no
     * owned handle is transferred either way. */
    SidereonIonexWarningList *orphan_warnings = (SidereonIonexWarningList *)(uintptr_t)1;
    SidereonIonex *orphan_product = (SidereonIonex *)(uintptr_t)1;
    if (sidereon_ionex_parse_with_warnings(bytes, len, NULL, &orphan_warnings) !=
            SIDEREON_STATUS_NULL_POINTER ||
        orphan_warnings != NULL ||
        sidereon_ionex_parse_with_warnings(bytes, len, &orphan_product, NULL) !=
            SIDEREON_STATUS_NULL_POINTER ||
        orphan_product != NULL ||
        sidereon_ionex_parse_with_warnings(bytes, len, NULL, NULL) !=
            SIDEREON_STATUS_NULL_POINTER) {
        free(bytes);
        sidereon_ionex_free(warned);
        sidereon_ionex_warning_list_free(warnings);
        return fail("sidereon_ionex_parse_with_warnings clears the writable partner", 1);
    }

    free(bytes);
    if (st != SIDEREON_STATUS_OK || warned == NULL || warnings == NULL) {
        sidereon_ionex_free(warned);
        sidereon_ionex_warning_list_free(warnings);
        return fail("sidereon_ionex_parse_with_warnings", 1);
    }
    size_t warning_count = 0;
    if (sidereon_ionex_warning_list_count(warnings, &warning_count) != SIDEREON_STATUS_OK) {
        sidereon_ionex_free(warned);
        sidereon_ionex_warning_list_free(warnings);
        return fail("sidereon_ionex_warning_list_count", 1);
    }
    /* Every warning the reader reports for the smoke fixture, its kind, label
     * and text, as sidereon-core reports them (tests/valgen bin
     * smoke_b_ionex). */
    if (warning_count != SMOKE_B_IONEX_WARNING_COUNT) {
        sidereon_ionex_free(warned);
        sidereon_ionex_warning_list_free(warnings);
        return fail("IONEX warning count", 1);
    }
    for (size_t i = 0; i < warning_count; i++) {
        SidereonIonexWarningInfo info;
        char label[64];
        char message[512];
        size_t label_len = 0;
        if (sidereon_ionex_warning_get_info(warnings, i, &info) != SIDEREON_STATUS_OK ||
            sidereon_ionex_warning_get_label(warnings, i, (uint8_t *)label, sizeof label - 1,
                                             &label_len, &required) != SIDEREON_STATUS_OK ||
            sidereon_ionex_warning_get_message(warnings, i, (uint8_t *)message, sizeof message,
                                               &written, &required) != SIDEREON_STATUS_OK ||
            required == 0) {
            sidereon_ionex_free(warned);
            sidereon_ionex_warning_list_free(warnings);
            return fail("IONEX warning accessors", 1);
        }
        label[label_len] = '\0';
        if (info.kind != SMOKE_B_IONEX_WARNING_KIND[i] ||
            strcmp(label, SMOKE_B_IONEX_WARNING_LABEL[i]) != 0 ||
            written != strlen(SMOKE_B_IONEX_WARNING_MESSAGE[i]) ||
            memcmp(message, SMOKE_B_IONEX_WARNING_MESSAGE[i], written) != 0) {
            sidereon_ionex_free(warned);
            sidereon_ionex_warning_list_free(warnings);
            return fail("IONEX warning kind, label and text", 1);
        }
    }
    /* An index past the end is refused rather than read. */
    SidereonIonexWarningInfo past_end;
    if (sidereon_ionex_warning_get_info(warnings, warning_count, &past_end) ==
        SIDEREON_STATUS_OK) {
        sidereon_ionex_free(warned);
        sidereon_ionex_warning_list_free(warnings);
        return fail("sidereon_ionex_warning_get_info index gate", 1);
    }
    sidereon_ionex_free(warned);
    sidereon_ionex_warning_list_free(warnings);
    sidereon_ionex_warning_list_free(NULL); /* free(NULL) is a no-op. */

    /* A refused parse transfers neither handle. */
    SidereonIonex *refused_product = (SidereonIonex *)(uintptr_t)1;
    SidereonIonexWarningList *refused_warnings = (SidereonIonexWarningList *)(uintptr_t)1;
    const uint8_t garbage[] = "not an IONEX file";
    if (SMOKE_B_IONEX_GARBAGE_OK ||
        sidereon_ionex_parse_with_warnings(garbage, sizeof garbage - 1, &refused_product,
                                           &refused_warnings) == SIDEREON_STATUS_OK ||
        refused_product != NULL || refused_warnings != NULL) {
        return fail("sidereon_ionex_parse_with_warnings refusal ownership", 1);
    }

    /* ---- owned header handle and its typed records ---- */
    SidereonIonexHeader *header = NULL;
    if (sidereon_ionex_get_header(ionex, &header) != SIDEREON_STATUS_OK || header == NULL) {
        return fail("sidereon_ionex_get_header", 1);
    }
    SidereonIonexMappingDeclarationKind declaration = SIDEREON_IONEX_MAPPING_DECLARATION_KIND_ABSENT;
    SidereonIonexMappingFunctionKind function = SIDEREON_IONEX_MAPPING_FUNCTION_KIND_OTHER;
    double version = 0.0;
    if (sidereon_ionex_header_get_version(header, &version) != SIDEREON_STATUS_OK ||
        sidereon_ionex_header_get_mapping_declaration(header, &declaration, &function) !=
            SIDEREON_STATUS_OK) {
        sidereon_ionex_header_free(header);
        return fail("IONEX header accessors", 1);
    }
    /* The version and mapping declaration sidereon-core reads from the smoke
     * fixture. An absent declaration leaves the function at NO_MAPPING
     * (src/ionex.rs sidereon_ionex_header_get_mapping_declaration). */
    if (f64_to_bits(version) != SMOKE_B_IONEX_HEADER_VERSION_BITS ||
        declaration != SMOKE_B_IONEX_HEADER_DECLARATION ||
        function != SMOKE_B_IONEX_HEADER_FUNCTION) {
        sidereon_ionex_header_free(header);
        return fail("IONEX header mapping declaration", 1);
    }
    char code[32];
    if (sidereon_ionex_header_get_mapping_function_code(header, (uint8_t *)code, sizeof code,
                                                        &written, &required) !=
            SIDEREON_STATUS_OK ||
        required != sizeof(SMOKE_B_IONEX_HEADER_MAPPING_CODE) - 1 ||
        written != required || memcmp(code, SMOKE_B_IONEX_HEADER_MAPPING_CODE, written) != 0) {
        sidereon_ionex_header_free(header);
        return fail("IONEX mapping code text", 1);
    }
    sidereon_ionex_header_free(header);

    /* A caller-built header carries a custom code the IONEX text cannot state;
     * the accessor still reads it back whole. */
    SidereonIonexHeader *built = NULL;
    if (sidereon_ionex_header_new(&built) != SIDEREON_STATUS_OK || built == NULL) {
        return fail("sidereon_ionex_header_new", 1);
    }
    static const char custom[] = "SLAB_3D_CUSTOM";
    if (sidereon_ionex_header_set_mapping_function(built,
                                                   SIDEREON_IONEX_MAPPING_FUNCTION_KIND_OTHER,
                                                   (const uint8_t *)custom, sizeof custom - 1) !=
            SIDEREON_STATUS_OK ||
        sidereon_ionex_header_get_mapping_function_code(built, (uint8_t *)code, sizeof code,
                                                        &written, &required) !=
            SIDEREON_STATUS_OK ||
        required != sizeof custom - 1 || memcmp(code, custom, required) != 0) {
        sidereon_ionex_header_free(built);
        return fail("IONEX custom mapping code round trip", 1);
    }
    static const char note[] = "C SMOKE DESCRIPTION";
    size_t description_count = 0;
    if (sidereon_ionex_header_add_description(built, (const uint8_t *)note, sizeof note - 1) !=
            SIDEREON_STATUS_OK ||
        sidereon_ionex_header_description_count(built, &description_count) !=
            SIDEREON_STATUS_OK ||
        description_count != 1) {
        sidereon_ionex_header_free(built);
        return fail("IONEX header description list", 1);
    }
    SidereonIonexHeader *cloned = NULL;
    if (sidereon_ionex_header_clone(built, &cloned) != SIDEREON_STATUS_OK || cloned == NULL) {
        sidereon_ionex_header_free(built);
        return fail("sidereon_ionex_header_clone", 1);
    }
    sidereon_ionex_header_free(cloned);
    sidereon_ionex_header_free(NULL); /* free(NULL) is a no-op. */

    /* ---- sample intermediate representation and presence masks ---- */
    SidereonTecGridSamplesInfo info;
    memset(&info, 0, sizeof info);
    if (sidereon_ionex_tec_grid_samples_info(ionex, &info) != SIDEREON_STATUS_OK ||
        info.map_epoch_count != SMOKE_B_IONEX_INFO_EPOCH_COUNT ||
        info.lat_node_count != SMOKE_B_IONEX_INFO_LAT_COUNT ||
        info.lon_node_count != SMOKE_B_IONEX_INFO_LON_COUNT ||
        info.tec_map_value_count != SMOKE_B_IONEX_INFO_TEC_COUNT ||
        info.tec_map_value_count !=
            info.map_epoch_count * info.lat_node_count * info.lon_node_count) {
        sidereon_ionex_header_free(built);
        return fail("sidereon_ionex_tec_grid_samples_info", 1);
    }
    /* The optional maps as sidereon-core holds them for the smoke fixture. */
    if (info.has_rms_maps != SMOKE_B_IONEX_INFO_HAS_RMS ||
        info.rms_map_value_count != SMOKE_B_IONEX_INFO_RMS_COUNT ||
        info.has_height_maps != SMOKE_B_IONEX_INFO_HAS_HEIGHT ||
        info.height_map_value_count != SMOKE_B_IONEX_INFO_HEIGHT_COUNT) {
        sidereon_ionex_header_free(built);
        return fail("IONEX absent optional maps", 1);
    }

    double *values = (double *)malloc(info.tec_map_value_count * sizeof(double));
    bool *presence = (bool *)malloc(info.tec_map_value_count * sizeof(bool));
    double *epochs_s = (double *)malloc(info.map_epoch_count * sizeof(double));
    if (values == NULL || presence == NULL || epochs_s == NULL) {
        free(values);
        free(presence);
        free(epochs_s);
        sidereon_ionex_header_free(built);
        return fail("ionex 3.0: allocate sample buffers", 2);
    }
    int sample_ok =
        sidereon_ionex_tec_grid_samples_tec_maps_tecu(ionex, values, info.tec_map_value_count,
                                                      &written, &required) ==
            SIDEREON_STATUS_OK &&
        written == info.tec_map_value_count &&
        sidereon_ionex_tec_grid_samples_tec_presence(ionex, presence, info.tec_map_value_count,
                                                     &written, &required) ==
            SIDEREON_STATUS_OK &&
        sidereon_ionex_tec_grid_samples_epochs_j2000_s(ionex, epochs_s, info.map_epoch_count,
                                                       &written, &required) ==
            SIDEREON_STATUS_OK;
    for (size_t i = 0; sample_ok && i < info.tec_map_value_count; i++) {
        /* Whether every node holds a value is sidereon-core's reading; a node
         * without one reads as NaN. */
        if ((SMOKE_B_IONEX_TEC_ALL_PRESENT && !presence[i]) || presence[i] == isnan(values[i])) {
            sample_ok = 0;
        }
    }
    /* A size query reports the count without writing, and a short buffer is
     * refused with the required count. */
    if (sample_ok) {
        written = 1;
        required = 0;
        sample_ok = sidereon_ionex_tec_grid_samples_tec_maps_tecu(ionex, NULL, 0, &written,
                                                                  &required) ==
                        SIDEREON_STATUS_OK &&
                    written == 0 && required == info.tec_map_value_count &&
                    sidereon_ionex_tec_grid_samples_tec_maps_tecu(ionex, values, 1, &written,
                                                                  &required) ==
                        SIDEREON_STATUS_INVALID_ARGUMENT &&
                    written == 0 && required == info.tec_map_value_count;
    }
    /* An absent map reports no values rather than a buffer of zeros. */
    if (sample_ok) {
        sample_ok = sidereon_ionex_tec_grid_samples_rms_maps_tecu(ionex, NULL, 0, &written,
                                                                  &required) ==
                        SIDEREON_STATUS_OK &&
                    required == SMOKE_B_IONEX_INFO_RMS_COUNT &&
                    sidereon_ionex_tec_grid_samples_rms_presence(ionex, NULL, 0, &written,
                                                                 &required) ==
                        SIDEREON_STATUS_OK &&
                    required == SMOKE_B_IONEX_INFO_RMS_COUNT &&
                    sidereon_ionex_tec_grid_samples_height_maps_km(ionex, NULL, 0, &written,
                                                                   &required) ==
                        SIDEREON_STATUS_OK &&
                    required == SMOKE_B_IONEX_INFO_HEIGHT_COUNT &&
                    sidereon_ionex_tec_grid_samples_height_presence(ionex, NULL, 0, &written,
                                                                    &required) ==
                        SIDEREON_STATUS_OK &&
                    required == SMOKE_B_IONEX_INFO_HEIGHT_COUNT;
    }
    free(values);
    free(presence);
    free(epochs_s);
    if (!sample_ok) {
        sidereon_ionex_header_free(built);
        return fail("IONEX sample extraction and presence masks", 1);
    }

    /* The per-node sample view reports the same presence authority. */
    size_t node_count = 0;
    if (sidereon_ionex_tec_samples(ionex, NULL, 0, &written, &required) != SIDEREON_STATUS_OK ||
        required != SMOKE_B_IONEX_NODE_COUNT) {
        sidereon_ionex_header_free(built);
        return fail("sidereon_ionex_tec_samples size query", 1);
    }
    node_count = required;
    SidereonTecSample *nodes = (SidereonTecSample *)malloc(node_count * sizeof(SidereonTecSample));
    if (nodes == NULL) {
        sidereon_ionex_header_free(built);
        return fail("ionex 3.0: allocate node samples", 2);
    }
    int nodes_ok = sidereon_ionex_tec_samples(ionex, nodes, node_count, &written, &required) ==
                       SIDEREON_STATUS_OK &&
                   written == node_count;
    for (size_t i = 0; nodes_ok && i < node_count; i++) {
        if ((SMOKE_B_IONEX_NODES_ALL_VTEC && !nodes[i].has_vtec_tecu) ||
            (!SMOKE_B_IONEX_NODES_ANY_RMS && nodes[i].has_rms_tecu) ||
            (!SMOKE_B_IONEX_NODES_ANY_HEIGHT && nodes[i].has_height_offset_km)) {
            nodes_ok = 0;
        }
    }
    free(nodes);
    if (!nodes_ok) {
        sidereon_ionex_header_free(built);
        return fail("sidereon_ionex_tec_samples presence flags", 1);
    }

    /* ---- the fallible writer propagates its refusal ---- */
    if (!SMOKE_B_IONEX_WRITER_OK ||
        sidereon_ionex_to_ionex_text(ionex, NULL, 0, &written, &required) != SIDEREON_STATUS_OK ||
        required == 0) {
        sidereon_ionex_header_free(built);
        return fail("sidereon_ionex_to_ionex_text size query", 1);
    }
    sidereon_ionex_header_free(built);

    /* ---- the owned slant-delay result list ---- */
    const IonexCase *inside = &IONEX_CASES[0];
    SidereonIonexSlantRequest requests[3];
    requests[0].lat_deg = bits_to_f64(inside->lat_deg_bits);
    requests[0].lon_deg = bits_to_f64(inside->lon_deg_bits);
    requests[0].azimuth_deg = bits_to_f64(inside->az_deg_bits);
    requests[0].elevation_deg = bits_to_f64(inside->el_deg_bits);
    requests[0].epoch_j2000_s = inside->epoch_j2000_s;
    requests[0].frequency_hz = bits_to_f64(inside->frequency_hz_bits);
    /* A receiver latitude no geodetic position can hold. */
    requests[1] = requests[0];
    requests[1].lat_deg = 95.0;
    /* Inside coverage again on another line of sight, so a failure cannot
     * truncate the list. */
    requests[2] = requests[0];
    requests[2].azimuth_deg = 120.0;

    SidereonIonexSlantResultList *rows = NULL;
    if (sidereon_ionex_slant_delay_results(ionex, requests, 3,
                                           sidereon_ionex_slant_policy_default(),
                                           &rows) != SIDEREON_STATUS_OK ||
        rows == NULL) {
        sidereon_ionex_slant_result_list_free(rows);
        return fail("sidereon_ionex_slant_delay_results", 1);
    }
    size_t row_count = 0;
    if (sidereon_ionex_slant_result_list_count(rows, &row_count) != SIDEREON_STATUS_OK ||
        row_count != SMOKE_B_IONEX_ROW_COUNT) {
        sidereon_ionex_slant_result_list_free(rows);
        return fail("IONEX result list keeps every row", 1);
    }
    SidereonIonexSlantRowResult first;
    SidereonIonexSlantRowResult bad;
    SidereonIonexSlantRowResult last;
    if (sidereon_ionex_slant_result_get_row(rows, 0, &first) != SIDEREON_STATUS_OK ||
        sidereon_ionex_slant_result_get_row(rows, 1, &bad) != SIDEREON_STATUS_OK ||
        sidereon_ionex_slant_result_get_row(rows, 2, &last) != SIDEREON_STATUS_OK) {
        sidereon_ionex_slant_result_list_free(rows);
        return fail("sidereon_ionex_slant_result_get_row", 1);
    }
    /* Each row is sidereon-core's outcome (tests/valgen bin smoke_b_ionex); the
     * first also equals the golden scalar delay. */
    if (!row_matches(&first, &SMOKE_B_IONEX_ROW[0]) ||
        f64_to_bits(first.evaluation.delay_m) != inside->delay_m_bits) {
        sidereon_ionex_slant_result_list_free(rows);
        return fail("IONEX result row matches the scalar delay", 1);
    }
    /* A failed row carries its typed detail, and its evaluation is a NaN that
     * cannot be read as a nominal zero. */
    if (!row_matches(&bad, &SMOKE_B_IONEX_ROW[1])) {
        sidereon_ionex_slant_result_list_free(rows);
        return fail("IONEX failed row keeps its typed detail", 1);
    }
    char row_message[256];
    if (sidereon_ionex_slant_result_get_message(rows, 1, (uint8_t *)row_message,
                                                sizeof row_message, &written, &required) !=
            SIDEREON_STATUS_OK ||
        required == 0) {
        sidereon_ionex_slant_result_list_free(rows);
        return fail("sidereon_ionex_slant_result_get_message", 1);
    }
    /* A successful row reports no text, and no mapping code. */
    if (sidereon_ionex_slant_result_get_message(rows, 0, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        required != 0 ||
        sidereon_ionex_slant_result_get_mapping_code(rows, 0, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        required != 0) {
        sidereon_ionex_slant_result_list_free(rows);
        return fail("IONEX successful row reports no failure text", 1);
    }
    if (!row_matches(&last, &SMOKE_B_IONEX_ROW[2])) {
        sidereon_ionex_slant_result_list_free(rows);
        return fail("IONEX result list retains the trailing row", 1);
    }
    if (sidereon_ionex_slant_result_get_row(rows, row_count, &first) == SIDEREON_STATUS_OK) {
        sidereon_ionex_slant_result_list_free(rows);
        return fail("sidereon_ionex_slant_result_get_row index gate", 1);
    }
    sidereon_ionex_slant_result_list_free(rows);
    sidereon_ionex_slant_result_list_free(NULL); /* free(NULL) is a no-op. */

    /* A malformed argument fails the whole call and transfers no list. */
    SidereonIonexSlantResultList *no_rows = (SidereonIonexSlantResultList *)(uintptr_t)1;
    if (sidereon_ionex_slant_delay_results(NULL, requests, 3,
                                           sidereon_ionex_slant_policy_default(),
                                           &no_rows) != SIDEREON_STATUS_NULL_POINTER ||
        no_rows != NULL) {
        return fail("IONEX result list null product", 1);
    }
    no_rows = (SidereonIonexSlantResultList *)(uintptr_t)1;
    if (sidereon_ionex_slant_delay_results(
            ionex, requests, 3, sidereon_ionex_slant_policy_init(9, MISSING_NODE_STRICT,
                                                                 MAPPING_SINGLE_LAYER),
            &no_rows) != SIDEREON_STATUS_INVALID_ARGUMENT ||
        no_rows != NULL) {
        return fail("IONEX result list invalid policy tag", 1);
    }

    /* The Declared mapping policy reaches the product's own MAPPING FUNCTION
     * declaration; the outcome is sidereon-core's. */
    SidereonIonexSlantDelayEvaluation declared_eval;
    SidereonIonexSlantError declared_error;
    SidereonStatus declared_status = sidereon_ionex_slant_delay_with_policy(
        ionex, requests[0].lat_deg, requests[0].lon_deg, requests[0].azimuth_deg,
        requests[0].elevation_deg, requests[0].epoch_j2000_s, f_l1,
        sidereon_ionex_slant_policy_init(SIDEREON_IONEX_COVERAGE_POLICY_STRICT,
                                         MISSING_NODE_STRICT, MAPPING_DECLARED),
        &declared_eval, &declared_error);
    if (!slant_matches(declared_status, &declared_eval, &declared_error,
                       &SMOKE_B_IONEX_DECLARED)) {
        return fail("IONEX declared mapping refusal detail", 1);
    }

    /* ---- the standalone regular TEC grid ---- */
    const double grid_epochs_ns[2] = {0.0, 1000.0};
    const double grid_latitudes_deg[2] = {-10.0, 10.0};
    const double grid_longitudes_deg[2] = {20.0, 60.0};
    /* Flat epoch-latitude-longitude order, longitude fastest; the second epoch
     * holds twice the first everywhere. */
    const double grid_values[8] = {1.0, 3.0, 7.0, 15.0, 2.0, 6.0, 14.0, 30.0};
    bool grid_presence[8];
    for (size_t i = 0; i < 8; i++) {
        grid_presence[i] = true;
    }

    SidereonTecGrid *grid = NULL;
    SidereonTecGridError grid_error;
    if (sidereon_tec_grid_new(grid_epochs_ns, 2, grid_latitudes_deg, 2, grid_longitudes_deg, 2,
                              grid_values, grid_presence, 8, &grid, &grid_error) !=
            SIDEREON_STATUS_OK ||
        grid == NULL || grid_error.kind != SIDEREON_TEC_GRID_ERROR_KIND_NONE) {
        sidereon_tec_grid_free(grid);
        return fail("sidereon_tec_grid_new", 1);
    }

    size_t epoch_count = 0, latitude_count = 0, longitude_count = 0, value_count = 0;
    if (sidereon_tec_grid_dimensions(grid, &epoch_count, &latitude_count, &longitude_count,
                                     &value_count) != SIDEREON_STATUS_OK ||
        epoch_count != 2 || latitude_count != 2 || longitude_count != 2 || value_count != 8) {
        sidereon_tec_grid_free(grid);
        return fail("sidereon_tec_grid_dimensions", 1);
    }

    double read_epochs[2], read_latitudes[2], read_longitudes[2], read_values[8];
    bool read_presence[8];
    if (sidereon_tec_grid_epochs_ns(grid, read_epochs, 2, &written, &required) !=
            SIDEREON_STATUS_OK ||
        sidereon_tec_grid_latitudes_deg(grid, read_latitudes, 2, &written, &required) !=
            SIDEREON_STATUS_OK ||
        sidereon_tec_grid_longitudes_deg(grid, read_longitudes, 2, &written, &required) !=
            SIDEREON_STATUS_OK ||
        sidereon_tec_grid_values_tecu(grid, read_values, 8, &written, &required) !=
            SIDEREON_STATUS_OK ||
        sidereon_tec_grid_value_presence(grid, read_presence, 8, &written, &required) !=
            SIDEREON_STATUS_OK) {
        sidereon_tec_grid_free(grid);
        return fail("standalone TEC grid extraction", 1);
    }
    /* Longitude varies fastest, then latitude, then epoch. */
    if (read_epochs[1] != 1000.0 || read_latitudes[0] != -10.0 || read_longitudes[1] != 60.0 ||
        read_values[1] != 3.0 || read_values[2] != 7.0 || read_values[4] != 2.0 ||
        !read_presence[0]) {
        sidereon_tec_grid_free(grid);
        return fail("standalone TEC grid flat order", 1);
    }

    /* The boundary takes degrees, latitude first, and hands them to the
     * engine unchanged. Every expected value below is sidereon-core's
     * evaluation of the same grid (tests/valgen bin smoke_b_ionex). */
    double vtec = 0.0;
    SidereonIonexNodeGap gap;
    memset(&gap, 0, sizeof gap);
    if (!SMOKE_B_IONEX_GRID_PIERCE_OK ||
        sidereon_tec_grid_vtec_at_pierce_point(grid, 250, 0.0, 30.0,
                                               MISSING_NODE_STRICT, &vtec, &gap, &grid_error) !=
            SIDEREON_STATUS_OK ||
        f64_to_bits(vtec) != SMOKE_B_IONEX_GRID_PIERCE_VTEC_BITS ||
        !gap_matches(&gap, &SMOKE_B_IONEX_GRID_PIERCE_GAP)) {
        sidereon_tec_grid_free(grid);
        return fail("standalone TEC grid pierce-point value", 1);
    }
    /* A query on the node at the first epoch, latitude 10 and longitude 60. */
    if (!SMOKE_B_IONEX_GRID_NODE_OK ||
        sidereon_tec_grid_vtec_at_pierce_point(grid, 0, 10.0, 60.0, MISSING_NODE_STRICT, &vtec,
                                               &gap, &grid_error) != SIDEREON_STATUS_OK ||
        f64_to_bits(vtec) != SMOKE_B_IONEX_GRID_NODE_VTEC_BITS) {
        sidereon_tec_grid_free(grid);
        return fail("standalone TEC grid node value in degrees", 1);
    }
    /* Passing the pair the other way round sends 30 degrees at a latitude axis
     * that covers only 10 either side, which pins the order. */
    if (SMOKE_B_IONEX_GRID_SWAPPED_OK ||
        sidereon_tec_grid_vtec_at_pierce_point(grid, 250, 30.0, 0.0,
                                               MISSING_NODE_STRICT, &vtec, &gap, &grid_error) !=
            SMOKE_B_IONEX_GRID_SWAPPED_STATUS ||
        grid_error.kind != SMOKE_B_IONEX_GRID_SWAPPED_KIND ||
        grid_error.axis != SMOKE_B_IONEX_GRID_SWAPPED_AXIS || !grid_error.has_axis_value ||
        f64_to_bits(grid_error.axis_value) != SMOKE_B_IONEX_GRID_SWAPPED_AXIS_VALUE_BITS ||
        !isnan(vtec)) {
        sidereon_tec_grid_free(grid);
        return fail("standalone TEC grid angular contract", 1);
    }
    sidereon_tec_grid_free(grid);

    /* A missing node keeps an explicit zero apart from no value, refuses a
     * strict query with its indexed corners, and is interpolated around under
     * the renormalizing policy. */
    double sparse_values[8];
    memcpy(sparse_values, grid_values, sizeof sparse_values);
    sparse_values[0] = 0.0;
    sparse_values[3] = 4242.0;
    grid_presence[3] = false;
    SidereonTecGrid *sparse = NULL;
    if (sidereon_tec_grid_new(grid_epochs_ns, 2, grid_latitudes_deg, 2, grid_longitudes_deg, 2,
                              sparse_values, grid_presence, 8, &sparse, &grid_error) !=
            SIDEREON_STATUS_OK ||
        sparse == NULL) {
        sidereon_tec_grid_free(sparse);
        return fail("standalone TEC grid with a missing node", 1);
    }
    if (sidereon_tec_grid_values_tecu(sparse, read_values, 8, &written, &required) !=
            SIDEREON_STATUS_OK ||
        sidereon_tec_grid_value_presence(sparse, read_presence, 8, &written, &required) !=
            SIDEREON_STATUS_OK ||
        read_values[0] != 0.0 || !read_presence[0] || !isnan(read_values[3]) ||
        read_presence[3]) {
        sidereon_tec_grid_free(sparse);
        return fail("standalone TEC grid zero versus missing", 1);
    }
    memset(&gap, 0, sizeof gap);
    if (SMOKE_B_IONEX_GRID_SPARSE_STRICT_OK ||
        sidereon_tec_grid_vtec_at_pierce_point(sparse, 250, 0.0, 30.0,
                                               MISSING_NODE_STRICT, &vtec, &gap, &grid_error) !=
            SMOKE_B_IONEX_GRID_SPARSE_STRICT_STATUS ||
        grid_error.kind != SMOKE_B_IONEX_GRID_SPARSE_STRICT_KIND || !grid_error.has_gap ||
        !gap_matches(&grid_error.gap, &SMOKE_B_IONEX_GRID_SPARSE_STRICT_GAP) ||
        !gap_matches(&gap, &SMOKE_B_IONEX_GRID_SPARSE_STRICT_GAP)) {
        sidereon_tec_grid_free(sparse);
        return fail("standalone TEC grid strict missing-node detail", 1);
    }
    memset(&gap, 0, sizeof gap);
    if (!SMOKE_B_IONEX_GRID_SPARSE_RENORMALIZE_OK ||
        sidereon_tec_grid_vtec_at_pierce_point(sparse, 250, 0.0, 30.0,
                                               MISSING_NODE_RENORMALIZE, &vtec, &gap,
                                               &grid_error) != SIDEREON_STATUS_OK ||
        f64_to_bits(vtec) != SMOKE_B_IONEX_GRID_SPARSE_RENORMALIZE_VTEC_BITS ||
        !gap_matches(&gap, &SMOKE_B_IONEX_GRID_SPARSE_RENORMALIZE_GAP)) {
        sidereon_tec_grid_free(sparse);
        return fail("standalone TEC grid renormalized value", 1);
    }
    /* An unrecognized policy tag is refused rather than read as a policy the
     * engine names. */
    if (sidereon_tec_grid_vtec_at_pierce_point(sparse, 250, 0.0, 30.0, 7, &vtec,
                                               &gap, NULL) != SIDEREON_STATUS_INVALID_ARGUMENT) {
        sidereon_tec_grid_free(sparse);
        return fail("standalone TEC grid policy tag gate", 1);
    }
    sidereon_tec_grid_free(sparse);
    sidereon_tec_grid_free(NULL); /* free(NULL) is a no-op. */

    /* A value count that disagrees with the axes transfers no handle. */
    SidereonTecGrid *unbuilt = (SidereonTecGrid *)(uintptr_t)1;
    if (sidereon_tec_grid_new(grid_epochs_ns, 2, grid_latitudes_deg, 2, grid_longitudes_deg, 2,
                              grid_values, NULL, 7, &unbuilt, &grid_error) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        unbuilt != NULL || grid_error.kind != SIDEREON_TEC_GRID_ERROR_KIND_VALUE_COUNT_MISMATCH ||
        grid_error.value_count != 7 || grid_error.expected_value_count != 8) {
        /* unbuilt may still hold the sentinel this check exists to catch, so it
         * is not a handle to free. */
        return fail("sidereon_tec_grid_new value count mismatch", 1);
    }
    /* Axis lengths whose product overflows are refused before any buffer is
     * read, so the counts may exceed the arrays supplied. */
    unbuilt = (SidereonTecGrid *)(uintptr_t)1;
    const size_t huge = (size_t)1 << 40;
    if (sidereon_tec_grid_new(grid_epochs_ns, huge, grid_latitudes_deg, huge,
                              grid_longitudes_deg, huge, grid_values, NULL, 8, &unbuilt,
                              &grid_error) != SIDEREON_STATUS_INVALID_ARGUMENT ||
        unbuilt != NULL ||
        grid_error.kind != SIDEREON_TEC_GRID_ERROR_KIND_DIMENSIONS_OVERFLOW) {
        return fail("sidereon_tec_grid_new dimension overflow", 1);
    }
    /* A null output pointer is refused before anything is allocated. */
    if (sidereon_tec_grid_new(grid_epochs_ns, 2, grid_latitudes_deg, 2, grid_longitudes_deg, 2,
                              grid_values, NULL, 8, NULL, &grid_error) !=
        SIDEREON_STATUS_NULL_POINTER) {
        return fail("sidereon_tec_grid_new null output", 1);
    }

    /* ---- the owned standalone TEC-grid result ---- */
    char text[256];
    SidereonTecGrid *owned_grid = NULL;
    SidereonTecGridResult *owned_result = NULL;
    SidereonTecGridOutcome outcome;
    memset(&outcome, 0, sizeof outcome);
    if (sidereon_tec_grid_new_result(grid_epochs_ns, 2, grid_latitudes_deg, 2,
                                     grid_longitudes_deg, 2, grid_values, NULL, 8, &owned_grid,
                                     &owned_result) != SIDEREON_STATUS_OK ||
        owned_grid == NULL || owned_result == NULL) {
        sidereon_tec_grid_result_free(owned_result);
        sidereon_tec_grid_free(owned_grid);
        return fail("sidereon_tec_grid_new_result", 1);
    }
    /* Building a grid evaluates nothing, so the result holds no value. */
    if (sidereon_tec_grid_result_get_outcome(owned_result, &outcome) != SIDEREON_STATUS_OK ||
        !outcome.is_ok || outcome.status != SIDEREON_STATUS_OK || outcome.has_vtec ||
        !isnan(outcome.vtec_tecu) || outcome.error.kind != SIDEREON_TEC_GRID_ERROR_KIND_NONE ||
        read_tec_grid_text(sidereon_tec_grid_result_get_message, owned_result, text,
                           sizeof text) != 0 ||
        text[0] != '\0') {
        sidereon_tec_grid_result_free(owned_result);
        sidereon_tec_grid_free(owned_grid);
        return fail("owned TEC grid construction outcome", 1);
    }
    sidereon_tec_grid_result_free(owned_result);

    /* A latitude that is not finite reaches the engine's shared validation,
     * which names the field and the reason. */
    SidereonTecGridResult *invalid_field = NULL;
    if (sidereon_tec_grid_vtec_at_pierce_point_result(owned_grid, 250, NAN, 30.0,
                                                      MISSING_NODE_STRICT,
                                                      &invalid_field) != SIDEREON_STATUS_OK ||
        invalid_field == NULL ||
        sidereon_tec_grid_result_get_outcome(invalid_field, &outcome) != SIDEREON_STATUS_OK ||
        SMOKE_B_IONEX_GRID_NAN_LAT_OK || outcome.is_ok ||
        outcome.status != SMOKE_B_IONEX_GRID_NAN_LAT_STATUS ||
        outcome.error.kind != SMOKE_B_IONEX_GRID_NAN_LAT_KIND || outcome.has_vtec ||
        !isnan(outcome.vtec_tecu)) {
        sidereon_tec_grid_result_free(invalid_field);
        sidereon_tec_grid_free(owned_grid);
        return fail("owned TEC grid invalid-field outcome", 1);
    }

    /* Sending the pair the other way round puts 30 degrees on a latitude axis
     * that covers only 10 either side. */
    SidereonTecGridResult *out_of_bounds = NULL;
    if (sidereon_tec_grid_vtec_at_pierce_point_result(owned_grid, 250, 30.0, 0.0,
                                                      MISSING_NODE_STRICT,
                                                      &out_of_bounds) != SIDEREON_STATUS_OK ||
        out_of_bounds == NULL ||
        sidereon_tec_grid_result_get_outcome(out_of_bounds, &outcome) != SIDEREON_STATUS_OK ||
        outcome.is_ok || outcome.status != SMOKE_B_IONEX_GRID_SWAPPED_STATUS ||
        outcome.error.kind != SMOKE_B_IONEX_GRID_SWAPPED_KIND ||
        outcome.error.axis != SMOKE_B_IONEX_GRID_SWAPPED_AXIS || !outcome.error.has_axis_value ||
        f64_to_bits(outcome.error.axis_value) != SMOKE_B_IONEX_GRID_SWAPPED_AXIS_VALUE_BITS ||
        outcome.degraded.has_gap ||
        outcome.error.has_gap != SMOKE_B_IONEX_GRID_SWAPPED_GAP.has_gap) {
        sidereon_tec_grid_result_free(out_of_bounds);
        sidereon_tec_grid_result_free(invalid_field);
        sidereon_tec_grid_free(owned_grid);
        return fail("owned TEC grid out-of-bounds outcome", 1);
    }

    /* A policy tag this binding does not name, and a null grid, are structural
     * failures of the call: the status is not OK and no result is allocated. */
    SidereonTecGridResult *refused = (SidereonTecGridResult *)(uintptr_t)1;
    if (sidereon_tec_grid_vtec_at_pierce_point_result(owned_grid, 250, 0.0, 30.0, 7,
                                                      &refused) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        refused != NULL) {
        /* refused may still hold the sentinel; only the handles this scope
         * definitely owns are released. */
        sidereon_tec_grid_result_free(out_of_bounds);
        sidereon_tec_grid_result_free(invalid_field);
        sidereon_tec_grid_free(owned_grid);
        return fail("owned TEC grid policy tag gate", 1);
    }
    refused = (SidereonTecGridResult *)(uintptr_t)1;
    if (sidereon_tec_grid_vtec_at_pierce_point_result(NULL, 250, 0.0, 30.0,
                                                      MISSING_NODE_STRICT, &refused) !=
            SIDEREON_STATUS_NULL_POINTER ||
        refused != NULL) {
        sidereon_tec_grid_result_free(out_of_bounds);
        sidereon_tec_grid_result_free(invalid_field);
        sidereon_tec_grid_free(owned_grid);
        return fail("owned TEC grid null grid gate", 1);
    }

    /* A later failing call overwrites the thread-local message, and the grid is
     * then released. The owned results keep every byte through both. */
    if (sidereon_tec_grid_vtec_at_pierce_point(owned_grid, 250, 30.0, 0.0,
                                               MISSING_NODE_STRICT, &vtec, &gap, &grid_error) ==
        SIDEREON_STATUS_OK) {
        sidereon_tec_grid_result_free(out_of_bounds);
        sidereon_tec_grid_result_free(invalid_field);
        sidereon_tec_grid_free(owned_grid);
        return fail("owned TEC grid thread-local overwrite", 1);
    }
    char last_error[256];
    last_error[0] = '\0';
    sidereon_last_error_message(last_error, sizeof last_error);
    sidereon_tec_grid_free(owned_grid);
    char expected_bound[256];
    snprintf(expected_bound, sizeof expected_bound, "sidereon_tec_grid_vtec_at_pierce_point: %s",
             SMOKE_B_IONEX_GRID_SWAPPED_MESSAGE);
    if (strcmp(last_error, expected_bound) != 0) {
        sidereon_tec_grid_result_free(out_of_bounds);
        sidereon_tec_grid_result_free(invalid_field);
        return fail("owned TEC grid thread-local contents", 1);
    }

    /* The exact bytes the engine gave, after the overwrite and the release. */
    char expected_invalid[256];
    snprintf(expected_invalid, sizeof expected_invalid,
             "sidereon_tec_grid_vtec_at_pierce_point_result: %s",
             SMOKE_B_IONEX_GRID_NAN_LAT_MESSAGE);
    if (read_tec_grid_text(sidereon_tec_grid_result_get_field, invalid_field, text,
                           sizeof text) != 0 ||
        strcmp(text, SMOKE_B_IONEX_GRID_NAN_LAT_FIELD) != 0 ||
        read_tec_grid_text(sidereon_tec_grid_result_get_reason, invalid_field, text,
                           sizeof text) != 0 ||
        strcmp(text, SMOKE_B_IONEX_GRID_NAN_LAT_REASON) != 0 ||
        read_tec_grid_text(sidereon_tec_grid_result_get_message, invalid_field, text,
                           sizeof text) != 0 ||
        strcmp(text, expected_invalid) != 0) {
        sidereon_tec_grid_result_free(out_of_bounds);
        sidereon_tec_grid_result_free(invalid_field);
        return fail("owned TEC grid retained invalid-field bytes", 1);
    }
    /* The engine named no field for a bound failure, so this binding invents
     * none, and the result still reports the whole message byte for byte. */
    char expected_bound_result[256];
    snprintf(expected_bound_result, sizeof expected_bound_result,
             "sidereon_tec_grid_vtec_at_pierce_point_result: %s",
             SMOKE_B_IONEX_GRID_SWAPPED_MESSAGE);
    if (read_tec_grid_text(sidereon_tec_grid_result_get_field, out_of_bounds, text,
                           sizeof text) != 0 ||
        text[0] != '\0' ||
        read_tec_grid_text(sidereon_tec_grid_result_get_reason, out_of_bounds, text,
                           sizeof text) != 0 ||
        text[0] != '\0' ||
        read_tec_grid_text(sidereon_tec_grid_result_get_message, out_of_bounds, text,
                           sizeof text) != 0 ||
        strcmp(text, expected_bound_result) != 0) {
        sidereon_tec_grid_result_free(out_of_bounds);
        sidereon_tec_grid_result_free(invalid_field);
        return fail("owned TEC grid retained bound bytes", 1);
    }
    sidereon_tec_grid_result_free(out_of_bounds);

    /* A buffer shorter than the message writes nothing and still reports what
     * the message needs; NULL with a nonzero length is refused. */
    written = SIZE_MAX;
    required = 0;
    if (sidereon_tec_grid_result_get_message(invalid_field, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != 0 || required == 0) {
        sidereon_tec_grid_result_free(invalid_field);
        return fail("owned TEC grid message length query", 1);
    }
    char short_buf[4];
    memset(short_buf, 7, sizeof short_buf);
    if (sidereon_tec_grid_result_get_message(invalid_field, (uint8_t *)short_buf,
                                             sizeof short_buf, &written, &required) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        written != 0 || short_buf[0] != 7 ||
        sidereon_tec_grid_result_get_message(invalid_field, NULL, 4, &written, &required) !=
            SIDEREON_STATUS_NULL_POINTER ||
        sidereon_tec_grid_result_get_field(invalid_field, NULL, 0, NULL, &required) !=
            SIDEREON_STATUS_NULL_POINTER ||
        sidereon_tec_grid_result_get_reason(invalid_field, NULL, 0, &written, NULL) !=
            SIDEREON_STATUS_NULL_POINTER) {
        sidereon_tec_grid_result_free(invalid_field);
        return fail("owned TEC grid short and null buffers", 1);
    }
    sidereon_tec_grid_result_free(invalid_field);
    sidereon_tec_grid_result_free(NULL); /* free(NULL) is a no-op. */

    /* An accessor writes its output before it validates the handle. */
    outcome.is_ok = true;
    outcome.has_vtec = true;
    outcome.vtec_tecu = 1.0;
    if (sidereon_tec_grid_result_get_outcome(NULL, &outcome) != SIDEREON_STATUS_NULL_POINTER ||
        outcome.is_ok || outcome.has_vtec || !isnan(outcome.vtec_tecu) ||
        sidereon_tec_grid_result_get_outcome(NULL, NULL) != SIDEREON_STATUS_NULL_POINTER) {
        return fail("owned TEC grid outcome initialization", 1);
    }

    /* A construction failure travels inside the owned result: the outer status
     * is OK, no grid is transferred, and the result carries the whole text. */
    SidereonTecGrid *unbuilt_grid = (SidereonTecGrid *)(uintptr_t)1;
    SidereonTecGridResult *unbuilt_result = (SidereonTecGridResult *)(uintptr_t)1;
    if (sidereon_tec_grid_new_result(grid_epochs_ns, 2, grid_latitudes_deg, 2,
                                     grid_longitudes_deg, 2, grid_values, NULL, 7, &unbuilt_grid,
                                     &unbuilt_result) != SIDEREON_STATUS_OK ||
        unbuilt_grid != NULL || unbuilt_result == NULL ||
        sidereon_tec_grid_result_get_outcome(unbuilt_result, &outcome) != SIDEREON_STATUS_OK ||
        outcome.is_ok || outcome.status != SIDEREON_STATUS_INVALID_ARGUMENT ||
        outcome.error.kind != SIDEREON_TEC_GRID_ERROR_KIND_VALUE_COUNT_MISMATCH ||
        outcome.error.value_count != 7 || outcome.error.expected_value_count != 8 ||
        read_tec_grid_text(sidereon_tec_grid_result_get_message, unbuilt_result, text,
                           sizeof text) != 0 ||
        strcmp(text, "sidereon_tec_grid_new_result: expected 8 values, got 7") != 0) {
        /* Either out-parameter may still hold its sentinel on this branch, so
         * neither is freed. */
        return fail("owned TEC grid value count mismatch", 1);
    }
    sidereon_tec_grid_result_free(unbuilt_result);

    /* A present value that is not finite is this binding's own check, by index.
     * The engine never saw it, so the result claims no field and no reason. */
    double bad_values[8];
    memcpy(bad_values, grid_values, sizeof bad_values);
    bad_values[5] = NAN;
    unbuilt_grid = (SidereonTecGrid *)(uintptr_t)1;
    unbuilt_result = (SidereonTecGridResult *)(uintptr_t)1;
    if (sidereon_tec_grid_new_result(grid_epochs_ns, 2, grid_latitudes_deg, 2,
                                     grid_longitudes_deg, 2, bad_values, NULL, 8, &unbuilt_grid,
                                     &unbuilt_result) != SIDEREON_STATUS_OK ||
        unbuilt_grid != NULL || unbuilt_result == NULL ||
        sidereon_tec_grid_result_get_outcome(unbuilt_result, &outcome) != SIDEREON_STATUS_OK ||
        outcome.error.kind != SIDEREON_TEC_GRID_ERROR_KIND_VALUE_NOT_FINITE ||
        !outcome.error.has_value_index || outcome.error.value_index != 5 ||
        read_tec_grid_text(sidereon_tec_grid_result_get_message, unbuilt_result, text,
                           sizeof text) != 0 ||
        strcmp(text, "sidereon_tec_grid_new_result: the value at index 5 is marked present and "
                     "is not finite") != 0 ||
        read_tec_grid_text(sidereon_tec_grid_result_get_field, unbuilt_result, text,
                           sizeof text) != 0 ||
        text[0] != '\0' ||
        read_tec_grid_text(sidereon_tec_grid_result_get_reason, unbuilt_result, text,
                           sizeof text) != 0 ||
        text[0] != '\0') {
        return fail("owned TEC grid non-finite value", 1);
    }
    sidereon_tec_grid_result_free(unbuilt_result);

    /* A null result out-parameter clears the grid out-parameter, transfers
     * nothing, and allocates nothing. */
    unbuilt_grid = (SidereonTecGrid *)(uintptr_t)1;
    if (sidereon_tec_grid_new_result(grid_epochs_ns, 2, grid_latitudes_deg, 2,
                                     grid_longitudes_deg, 2, grid_values, NULL, 8, &unbuilt_grid,
                                     NULL) != SIDEREON_STATUS_NULL_POINTER ||
        unbuilt_grid != NULL) {
        return fail("owned TEC grid null result output", 1);
    }
    /* The other ordering: a null grid out-parameter must still clear the result
     * out-parameter, or a caller checking it for NULL reads whatever it left
     * there as a transferred handle. */
    unbuilt_result = (SidereonTecGridResult *)(uintptr_t)1;
    if (sidereon_tec_grid_new_result(grid_epochs_ns, 2, grid_latitudes_deg, 2,
                                     grid_longitudes_deg, 2, grid_values, NULL, 8, NULL,
                                     &unbuilt_result) != SIDEREON_STATUS_NULL_POINTER ||
        unbuilt_result != NULL) {
        return fail("owned TEC grid null grid output", 1);
    }
    /* Both out-parameters null writes nothing and allocates nothing. */
    if (sidereon_tec_grid_new_result(grid_epochs_ns, 2, grid_latitudes_deg, 2,
                                     grid_longitudes_deg, 2, grid_values, NULL, 8, NULL,
                                     NULL) != SIDEREON_STATUS_NULL_POINTER) {
        return fail("owned TEC grid both outputs null", 1);
    }
    /* A null value buffer with a matching nonzero count is structural too, so
     * the call itself fails and neither output is transferred. */
    unbuilt_grid = (SidereonTecGrid *)(uintptr_t)1;
    unbuilt_result = (SidereonTecGridResult *)(uintptr_t)1;
    if (sidereon_tec_grid_new_result(grid_epochs_ns, 2, grid_latitudes_deg, 2,
                                     grid_longitudes_deg, 2, NULL, NULL, 8, &unbuilt_grid,
                                     &unbuilt_result) != SIDEREON_STATUS_NULL_POINTER ||
        unbuilt_grid != NULL || unbuilt_result != NULL) {
        return fail("owned TEC grid null value buffer", 1);
    }

    if (exercise_ionex_rms_presence_distinction() != 0 ||
        exercise_ionex_owned_mapping_code_lifetime() != 0 ||
        exercise_ionex_blank_custom_mapping_code() != 0 ||
        exercise_ionex_undeclared_default_header() != 0 ||
        exercise_ionex_batch_output_zeroing() != 0 ||
        exercise_tec_grid_latitude_clamp() != 0 ||
        exercise_tec_grid_nonfinite_latitude() != 0 ||
        exercise_ionex_writer_refusal() != 0 ||
        exercise_ionex_height_presence_distinction() != 0 ||
        exercise_ionex_missing_nodes_and_heights() != 0 ||
        exercise_ionex_whole_second_epochs() != 0 ||
        exercise_ionex_output_clearing(ionex) != 0) {
        return 1;
    }

    printf("IONEX 3.0 surface: warnings, header, samples, owned results and standalone grid\n");
    return 0;
}

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
    /* A second-of-day of 1e9: sidereon-core refuses it, and the refusal
     * reaches C with the output zeroed (src/ionex.rs map_iono_error). */
    if (SMOKE_B_IONEX_KLOB_T1E9_OK ||
        sidereon_klobuchar_native(a4, a4, 0.0, 0.0, 0.0, 45.0, 1.0e9, f_l1, &out) !=
            SMOKE_B_IONEX_KLOB_T1E9_STATUS ||
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
    if (sidereon_ionex_epoch_count(ionex, &epoch_count) != SIDEREON_STATUS_OK ||
        epoch_count != SMOKE_B_IONEX_EPOCH_COUNT) {
        sidereon_ionex_free(ionex);
        return fail("sidereon_ionex_epoch_count", 1);
    }
    int32_t exponent = 0;
    if (sidereon_ionex_exponent(ionex, &exponent) != SIDEREON_STATUS_OK ||
        exponent != SMOKE_B_IONEX_EXPONENT || exponent != IONEX_EXPONENT) {
        sidereon_ionex_free(ionex);
        return fail("sidereon_ionex_exponent", 1);
    }
    size_t written = 0, required = 0;
    double lat_nodes[16];
    if (sidereon_ionex_lat_nodes_deg(ionex, lat_nodes, 16, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != SMOKE_B_IONEX_LAT_NODE_COUNT || required != SMOKE_B_IONEX_LAT_NODE_COUNT) {
        sidereon_ionex_free(ionex);
        return fail("sidereon_ionex_lat_nodes_deg", 1);
    }
    double lon_nodes[16];
    if (sidereon_ionex_lon_nodes_deg(ionex, lon_nodes, 16, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != SMOKE_B_IONEX_LON_NODE_COUNT || required != SMOKE_B_IONEX_LON_NODE_COUNT) {
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

    SidereonIonexSlantPolicy default_policy = sidereon_ionex_slant_policy_default();
    SidereonIonexSlantPolicy hold_policy =
        sidereon_ionex_slant_policy_from_coverage(SIDEREON_IONEX_COVERAGE_POLICY_HOLD);

    /* Every golden case under the strict default policy and under the hold
     * coverage policy, against sidereon-core's outcomes (tests/valgen bin
     * smoke_b_ionex). Where the default succeeds, the scalar route gives the
     * golden delay; where it refuses, the hold policy gives it. */
    for (size_t c = 0; c < IONEX_CASE_COUNT; c++) {
        const IonexCase *ic = &IONEX_CASES[c];
        double delay = -1.0;
        const double lat = bits_to_f64(ic->lat_deg_bits);
        const double lon = bits_to_f64(ic->lon_deg_bits);
        const double az = bits_to_f64(ic->az_deg_bits);
        const double el = bits_to_f64(ic->el_deg_bits);
        const double freq = bits_to_f64(ic->frequency_hz_bits);
        SidereonStatus default_st =
            sidereon_ionex_slant_delay(ionex, lat, lon, az, el, ic->epoch_j2000_s, freq, &delay);
        const SmokeBSlantPin *default_pin = &SMOKE_B_IONEX_CASE_DEFAULT[c];
        const SmokeBSlantPin *hold_pin = &SMOKE_B_IONEX_CASE_HOLD[c];
        if (default_st != default_pin->status ||
            (default_pin->ok && (f64_to_bits(delay) != default_pin->delay_bits ||
                                 f64_to_bits(delay) != ic->delay_m_bits))) {
            fprintf(stderr, "FAIL: IONEX slant %s default outcome (%.17g)\n", ic->name, delay);
            sidereon_ionex_free(ionex);
            return 1;
        }
        SidereonIonexSlantDelayEvaluation eval;
        SidereonIonexSlantError eval_error;
        SidereonStatus st = sidereon_ionex_slant_delay_with_policy(
            ionex, lat, lon, az, el, ic->epoch_j2000_s, freq, default_policy, &eval, &eval_error);
        if (!slant_matches(st, &eval, &eval_error, default_pin)) {
            fprintf(stderr, "FAIL: IONEX slant %s default policy detail\n", ic->name);
            sidereon_ionex_free(ionex);
            return 1;
        }
        st = sidereon_ionex_slant_delay_with_policy(ionex, lat, lon, az, el, ic->epoch_j2000_s,
                                                    freq, hold_policy, &eval, &eval_error);
        if (!slant_matches(st, &eval, &eval_error, hold_pin) ||
            (!default_pin->ok && f64_to_bits(eval.delay_m) != ic->delay_m_bits)) {
            fprintf(stderr, "FAIL: IONEX slant %s hold policy detail\n", ic->name);
            sidereon_ionex_free(ionex);
            return 1;
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

    if (exercise_ionex_3_0_surface(ionex, ionex_path) != 0) {
        sidereon_ionex_free(ionex);
        return 1;
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

    /* The defaults are sidereon-core's VelocitySolveOptions::default(). */
    SidereonVelocityOptions opts;
    if (sidereon_velocity_options_init(&opts) != SIDEREON_STATUS_OK ||
        (opts.observable == SIDEREON_VELOCITY_OBSERVABLE_RANGE_RATE) !=
            SMOKE_B_GEOMETRY_VELOCITY_DEFAULT_RANGE_RATE ||
        opts.light_time != SMOKE_B_GEOMETRY_VELOCITY_DEFAULT_LIGHT_TIME ||
        opts.sagnac != SMOKE_B_GEOMETRY_VELOCITY_DEFAULT_SAGNAC) {
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

    /* Three observations: sidereon-core's outcome, with a refusal reported as
     * SIDEREON_STATUS_SOLVE (src/velocity.rs guard). */
    SidereonVelocitySolution *thin = (SidereonVelocitySolution *)(uintptr_t)1;
    SidereonStatus thin_status = sidereon_solve_velocity(sp3, rr, 3, receiver, t_rx, &opts, &thin);
    if (SMOKE_B_GEOMETRY_VELOCITY_THREE_SATS_OK || thin_status != SIDEREON_STATUS_SOLVE ||
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
        if (sidereon_antenna_pco(antenna, pc->frequency, neu) != SIDEREON_STATUS_OK) {
            int rc = fail(pc->frequency, 1);
            sidereon_antenna_free(antenna);
            sidereon_antex_free(antex);
            return rc;
        }
        if (f64_to_bits(neu[0]) != pc->north_m_bits ||
            f64_to_bits(neu[1]) != pc->east_m_bits ||
            f64_to_bits(neu[2]) != pc->up_m_bits) {
            fprintf(stderr,
                    "FAIL: ANTEX PCO %s/%s not bit-exact: got 0x%016llx 0x%016llx 0x%016llx "
                    "(%.17g %.17g %.17g), expected 0x%016llx 0x%016llx 0x%016llx\n",
                    pc->antenna_id, pc->frequency, (unsigned long long)f64_to_bits(neu[0]),
                    (unsigned long long)f64_to_bits(neu[1]),
                    (unsigned long long)f64_to_bits(neu[2]), neu[0], neu[1], neu[2],
                    (unsigned long long)pc->north_m_bits, (unsigned long long)pc->east_m_bits,
                    (unsigned long long)pc->up_m_bits);
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

    /* An id the product does not hold is a successful query; whether the
     * engine finds it is sidereon-core's lookup. */
    SidereonAntenna *missing = (SidereonAntenna *)(uintptr_t)1;
    if (sidereon_antex_antenna(antex, "NO SUCH ANTENNA", &missing) != SIDEREON_STATUS_OK ||
        SMOKE_B_GEOMETRY_ANTEX_NO_SUCH_ANTENNA_FOUND || missing != NULL) {
        sidereon_antex_free(antex);
        return fail("sidereon_antex_antenna missing id clears out_antenna", 1);
    }

    /* A frequency the antenna does not carry: sidereon-core refuses it, and
     * the refusal reaches C as SIDEREON_STATUS_INVALID_ARGUMENT with the
     * output cleared (src/lib.rs map_antex_error). */
    SidereonAntenna *real = NULL;
    if (sidereon_antex_antenna(antex, ANTEX_PCO_CASES[0].antenna_id, &real) !=
            SIDEREON_STATUS_OK ||
        real == NULL) {
        sidereon_antex_free(antex);
        return fail("sidereon_antex_antenna lookup for negative test", 1);
    }
    double neu[3] = {1.0, 2.0, 3.0};
    if (SMOKE_B_GEOMETRY_ANTEX_PCO_ZZ9_OK ||
        sidereon_antenna_pco(real, "ZZ9", neu) != SIDEREON_STATUS_INVALID_ARGUMENT ||
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

    /* az/el line-of-sight constructor: sidereon-core's vectors for four
     * directions at one site, and the DOP of that geometry (tests/valgen bin
     * smoke_b_geometry). */
    SidereonGeodetic site = {bits_to_f64(SMOKE_B_GEOMETRY_SITE_BITS[0]),
                             bits_to_f64(SMOKE_B_GEOMETRY_SITE_BITS[1]),
                             bits_to_f64(SMOKE_B_GEOMETRY_SITE_BITS[2])};
    SidereonLineOfSight built[4];
    double built_w[4];
    for (int i = 0; i < 4; i++) {
        SidereonLineOfSight l = {7.0, 7.0, 7.0};
        if (sidereon_line_of_sight_from_az_el_deg(bits_to_f64(SMOKE_B_GEOMETRY_LOS_AZ_DEG_BITS[i]),
                                                  bits_to_f64(SMOKE_B_GEOMETRY_LOS_EL_DEG_BITS[i]),
                                                  site, &l) != SIDEREON_STATUS_OK ||
            f64_to_bits(l.e_x) != SMOKE_B_GEOMETRY_LOS_XYZ_BITS[3 * i] ||
            f64_to_bits(l.e_y) != SMOKE_B_GEOMETRY_LOS_XYZ_BITS[3 * i + 1] ||
            f64_to_bits(l.e_z) != SMOKE_B_GEOMETRY_LOS_XYZ_BITS[3 * i + 2]) {
            return fail("sidereon_line_of_sight_from_az_el_deg", 1);
        }
        built[i] = l;
        built_w[i] = 1.0;
    }
    SidereonDop built_dop = {0.0, 0.0, 0.0, 0.0, 0.0};
    if (sidereon_dop(built, built_w, 4, site, &built_dop) != SIDEREON_STATUS_OK ||
        f64_to_bits(built_dop.gdop) != SMOKE_B_GEOMETRY_BUILT_DOP_BITS[0] ||
        f64_to_bits(built_dop.pdop) != SMOKE_B_GEOMETRY_BUILT_DOP_BITS[1] ||
        f64_to_bits(built_dop.hdop) != SMOKE_B_GEOMETRY_BUILT_DOP_BITS[2] ||
        f64_to_bits(built_dop.vdop) != SMOKE_B_GEOMETRY_BUILT_DOP_BITS[3] ||
        f64_to_bits(built_dop.tdop) != SMOKE_B_GEOMETRY_BUILT_DOP_BITS[4]) {
        return fail("sidereon_dop from az/el geometry", 1);
    }
    /* An elevation of 91 degrees: sidereon-core refuses it, and the refusal
     * reaches C as SIDEREON_STATUS_INVALID_ARGUMENT with out_los cleared
     * (src/lib.rs map_dop_error). */
    SidereonLineOfSight bad = {1.0, 1.0, 1.0};
    if (SMOKE_B_GEOMETRY_LOS_EL91_OK ||
        sidereon_line_of_sight_from_az_el_deg(0.0, 91.0, site, &bad) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        bad.e_x != 0.0 || bad.e_y != 0.0 || bad.e_z != 0.0) {
        return fail("sidereon_line_of_sight_from_az_el_deg out-of-range elevation", 1);
    }

    printf("DOP surface: %d golden cases bit-exact, %d singular rejected, az/el LOS OK\n",
           (int)DOP_CASE_COUNT, (int)DOP_SINGULAR_COUNT);
    return 0;
}

/* Inter-system time-scale offsets. Each expected offset, and whether the
 * engine refuses the query, is sidereon-core's (tests/valgen bin
 * smoke_b_timescale). A refusal reaches C as SIDEREON_STATUS_INVALID_ARGUMENT
 * with the output zeroed (src/time.rs time_offset_error_to_status). */
static int check_timescale_offset(SidereonStatus status, double off, bool expected_ok,
                                  uint64_t expected_bits, const char *what) {
    if (status != (expected_ok ? SIDEREON_STATUS_OK : SIDEREON_STATUS_INVALID_ARGUMENT) ||
        f64_to_bits(off) != expected_bits) {
        return fail(what, 1);
    }
    return 0;
}

static int exercise_timescale_surface(void) {
    const double jd_2017 = bits_to_f64(SMOKE_B_TIMESCALE_JD_2017_BITS);
    double off = 0.0;
    SidereonStatus st = SIDEREON_STATUS_OK;
    off = 123.0;
    st = sidereon_timescale_offset_s(SIDEREON_TIME_SCALE_GPST, SIDEREON_TIME_SCALE_BDT, &off);
    if (check_timescale_offset(st, off, SMOKE_B_TIMESCALE_GPST_BDT_OK,
                               SMOKE_B_TIMESCALE_GPST_BDT_BITS, "timescale offset GPST_BDT") != 0) {
        return 1;
    }
    off = 123.0;
    st = sidereon_timescale_offset_s(SIDEREON_TIME_SCALE_BDT, SIDEREON_TIME_SCALE_GPST, &off);
    if (check_timescale_offset(st, off, SMOKE_B_TIMESCALE_BDT_GPST_OK,
                               SMOKE_B_TIMESCALE_BDT_GPST_BITS, "timescale offset BDT_GPST") != 0) {
        return 1;
    }
    off = 123.0;
    st = sidereon_timescale_offset_s(SIDEREON_TIME_SCALE_TAI, SIDEREON_TIME_SCALE_TT, &off);
    if (check_timescale_offset(st, off, SMOKE_B_TIMESCALE_TAI_TT_OK,
                               SMOKE_B_TIMESCALE_TAI_TT_BITS, "timescale offset TAI_TT") != 0) {
        return 1;
    }
    off = 123.0;
    st = sidereon_timescale_offset_s(SIDEREON_TIME_SCALE_GPST, SIDEREON_TIME_SCALE_TT, &off);
    if (check_timescale_offset(st, off, SMOKE_B_TIMESCALE_GPST_TT_OK,
                               SMOKE_B_TIMESCALE_GPST_TT_BITS, "timescale offset GPST_TT") != 0) {
        return 1;
    }
    off = 123.0;
    st = sidereon_timescale_offset_s(SIDEREON_TIME_SCALE_GPST, SIDEREON_TIME_SCALE_TAI, &off);
    if (check_timescale_offset(st, off, SMOKE_B_TIMESCALE_GPST_TAI_OK,
                               SMOKE_B_TIMESCALE_GPST_TAI_BITS, "timescale offset GPST_TAI") != 0) {
        return 1;
    }
    off = 123.0;
    st = sidereon_timescale_offset_s(SIDEREON_TIME_SCALE_GPST, SIDEREON_TIME_SCALE_GST, &off);
    if (check_timescale_offset(st, off, SMOKE_B_TIMESCALE_GPST_GST_OK,
                               SMOKE_B_TIMESCALE_GPST_GST_BITS, "timescale offset GPST_GST") != 0) {
        return 1;
    }
    off = 123.0;
    st = sidereon_timescale_offset_s(SIDEREON_TIME_SCALE_GPST, SIDEREON_TIME_SCALE_QZSST, &off);
    if (check_timescale_offset(st, off, SMOKE_B_TIMESCALE_GPST_QZSST_OK,
                               SMOKE_B_TIMESCALE_GPST_QZSST_BITS, "timescale offset GPST_QZSST") != 0) {
        return 1;
    }
    off = 123.0;
    st = sidereon_timescale_offset_at_s(SIDEREON_TIME_SCALE_UTC, SIDEREON_TIME_SCALE_GPST, jd_2017, &off);
    if (check_timescale_offset(st, off, SMOKE_B_TIMESCALE_AT_UTC_GPST_OK,
                               SMOKE_B_TIMESCALE_AT_UTC_GPST_BITS, "timescale offset AT_UTC_GPST") != 0) {
        return 1;
    }
    off = 123.0;
    st = sidereon_timescale_offset_at_s(SIDEREON_TIME_SCALE_GPST, SIDEREON_TIME_SCALE_UTC, jd_2017, &off);
    if (check_timescale_offset(st, off, SMOKE_B_TIMESCALE_AT_GPST_UTC_OK,
                               SMOKE_B_TIMESCALE_AT_GPST_UTC_BITS, "timescale offset AT_GPST_UTC") != 0) {
        return 1;
    }
    off = 123.0;
    st = sidereon_timescale_offset_at_s(SIDEREON_TIME_SCALE_GPST, SIDEREON_TIME_SCALE_GLONASST, jd_2017, &off);
    if (check_timescale_offset(st, off, SMOKE_B_TIMESCALE_AT_GPST_GLONASST_OK,
                               SMOKE_B_TIMESCALE_AT_GPST_GLONASST_BITS, "timescale offset AT_GPST_GLONASST") != 0) {
        return 1;
    }
    off = 123.0;
    st = sidereon_timescale_offset_s(SIDEREON_TIME_SCALE_GPST, SIDEREON_TIME_SCALE_UTC, &off);
    if (check_timescale_offset(st, off, SMOKE_B_TIMESCALE_GPST_UTC_OK,
                               SMOKE_B_TIMESCALE_GPST_UTC_BITS, "timescale offset GPST_UTC") != 0) {
        return 1;
    }
    off = 123.0;
    st = sidereon_timescale_offset_s(SIDEREON_TIME_SCALE_GLONASST, SIDEREON_TIME_SCALE_GPST, &off);
    if (check_timescale_offset(st, off, SMOKE_B_TIMESCALE_GLONASST_GPST_OK,
                               SMOKE_B_TIMESCALE_GLONASST_GPST_BITS, "timescale offset GLONASST_GPST") != 0) {
        return 1;
    }
    off = 123.0;
    st = sidereon_timescale_offset_s(SIDEREON_TIME_SCALE_GPST, SIDEREON_TIME_SCALE_TDB, &off);
    if (check_timescale_offset(st, off, SMOKE_B_TIMESCALE_GPST_TDB_OK,
                               SMOKE_B_TIMESCALE_GPST_TDB_BITS, "timescale offset GPST_TDB") != 0) {
        return 1;
    }
    off = 123.0;
    st = sidereon_timescale_offset_at_s(SIDEREON_TIME_SCALE_GPST, SIDEREON_TIME_SCALE_UTC, (double)NAN, &off);
    if (check_timescale_offset(st, off, SMOKE_B_TIMESCALE_AT_GPST_UTC_NAN_OK,
                               SMOKE_B_TIMESCALE_AT_GPST_UTC_NAN_BITS, "timescale offset AT_GPST_UTC_NAN") != 0) {
        return 1;
    }
    /* An unknown scale code is refused by the binding. */
    if (sidereon_timescale_offset_s(999, SIDEREON_TIME_SCALE_GPST, &off) !=
        SIDEREON_STATUS_INVALID_ARGUMENT) {
        return fail("timescale_offset_s invalid scale code", 1);
    }

    printf("timescale surface: fixed and leap-aware offsets and refusals match sidereon-core\n");
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

    /* Explicit-time NAVCEN assessment at the instants tests/valgen chose: a
     * minute before the PRN 7 forecast outage the fixture states, its start and
     * its end. Every expected value below is sidereon-core's assessment. */
    const int64_t forecast_before_us = SMOKE_B_CONSTELLATION_EVAL_BEFORE_US;
    const int64_t forecast_start_us = SMOKE_B_CONSTELLATION_EVAL_START_US;
    const int64_t forecast_end_us = SMOKE_B_CONSTELLATION_EVAL_END_US;
    SidereonNavcenAssessments *assessments = NULL;
    if (sidereon_navcen_parse_at(CONSTELLATION_NAVCEN_FORECAST_HTML,
                                 CONSTELLATION_NAVCEN_FORECAST_HTML_LEN, forecast_start_us,
                                 &assessments) != SIDEREON_STATUS_OK ||
        assessments == NULL) {
        sidereon_constellation_free(catalog);
        return fail("sidereon_navcen_parse_at", 1);
    }
    size_t assessment_count = 0;
    if (sidereon_navcen_assessment_count(assessments, &assessment_count) !=
            SIDEREON_STATUS_OK ||
        assessment_count != SMOKE_B_CONSTELLATION_ASSESSMENT_COUNT) {
        sidereon_navcen_assessments_free(assessments);
        sidereon_constellation_free(catalog);
        return fail("sidereon_navcen_assessment_count", 1);
    }
    SidereonNavcenAssessment forecast;
    for (size_t i = 0; i < SMOKE_B_CONSTELLATION_ASSESSMENT_COUNT; i++) {
        SidereonNavcenAssessment row;
        if (sidereon_navcen_assessment(assessments, i, &row) != SIDEREON_STATUS_OK ||
            row.prn != SMOKE_B_CONSTELLATION_ASSESSMENT_PRN[i] ||
            row.usable != SMOKE_B_CONSTELLATION_ASSESSMENT_USABLE[i] ||
            row.active_nanu != SMOKE_B_CONSTELLATION_ASSESSMENT_ACTIVE_NANU[i] ||
            (uint32_t)row.timing != SMOKE_B_CONSTELLATION_ASSESSMENT_TIMING[i] ||
            row.effective_start_present != SMOKE_B_CONSTELLATION_ASSESSMENT_START_PRESENT[i] ||
            row.effective_start_unix_us != SMOKE_B_CONSTELLATION_ASSESSMENT_START_US[i] ||
            row.effective_end_present != SMOKE_B_CONSTELLATION_ASSESSMENT_END_PRESENT[i] ||
            row.effective_end_unix_us != SMOKE_B_CONSTELLATION_ASSESSMENT_END_US[i] ||
            row.evaluated_at_unix_us != forecast_start_us) {
            sidereon_navcen_assessments_free(assessments);
            sidereon_constellation_free(catalog);
            return fail("time-aware NAVCEN assessment values", 1);
        }
    }
    size_t text_written = 123, text_required = 123;
    if (sidereon_navcen_assessment_nanu_type(assessments, 1, NULL, 0, &text_written,
                                             &text_required) != SIDEREON_STATUS_OK ||
        text_written != 0 ||
        text_required != sizeof(SMOKE_B_CONSTELLATION_ROW1_NANU_TYPE) - 1) {
        sidereon_navcen_assessments_free(assessments);
        sidereon_constellation_free(catalog);
        return fail("NAVCEN NANU type size", 1);
    }
    uint8_t nanu_type[sizeof(SMOKE_B_CONSTELLATION_ROW1_NANU_TYPE) - 1];
    if (sidereon_navcen_assessment_nanu_type(assessments, 1, nanu_type, sizeof(nanu_type),
                                             &text_written, &text_required) != SIDEREON_STATUS_OK ||
        text_written != sizeof(nanu_type) ||
        memcmp(nanu_type, SMOKE_B_CONSTELLATION_ROW1_NANU_TYPE, sizeof(nanu_type)) != 0) {
        sidereon_navcen_assessments_free(assessments);
        sidereon_constellation_free(catalog);
        return fail("NAVCEN NANU type bytes", 1);
    }
    sidereon_navcen_assessments_free(assessments);

    for (size_t i = 0; i < 2; i++) {
        int64_t evaluation_us = i == 0 ? forecast_before_us : forecast_end_us;
        bool expected_usable = i == 0 ? SMOKE_B_CONSTELLATION_ROW1_USABLE_BEFORE
                                      : SMOKE_B_CONSTELLATION_ROW1_USABLE_END;
        assessments = NULL;
        if (sidereon_navcen_parse_at(CONSTELLATION_NAVCEN_FORECAST_HTML,
                                     CONSTELLATION_NAVCEN_FORECAST_HTML_LEN, evaluation_us,
                                     &assessments) != SIDEREON_STATUS_OK ||
            sidereon_navcen_assessment(assessments, 1, &forecast) != SIDEREON_STATUS_OK ||
            forecast.usable != expected_usable) {
            sidereon_navcen_assessments_free(assessments);
            sidereon_constellation_free(catalog);
            return fail("forecast usability before start and at end", 1);
        }
        sidereon_navcen_assessments_free(assessments);
    }

    /* The one-row PRN 19 forecast table merged into the CelesTrak catalog, as
     * tests/valgen states it. */
    const uint8_t *merge_forecast_html =
        (const uint8_t *)SMOKE_B_CONSTELLATION_MERGE_FORECAST_HTML;
    const size_t merge_forecast_len = sizeof(SMOKE_B_CONSTELLATION_MERGE_FORECAST_HTML) - 1;
    SidereonConstellation *time_aware_catalog = NULL;
    if (sidereon_constellation_build_at(
            SIDEREON_GNSS_SYSTEM_GPS, CONSTELLATION_GPS_OPS_JSON,
            CONSTELLATION_GPS_OPS_JSON_LEN, merge_forecast_html,
            merge_forecast_len, forecast_start_us, &time_aware_catalog) !=
            SIDEREON_STATUS_OK ||
        time_aware_catalog == NULL) {
        sidereon_constellation_free(catalog);
        return fail("sidereon_constellation_build_at", 1);
    }
    SidereonConstellationRecord timed_prn19;
    if (sidereon_constellation_record(time_aware_catalog, 3, &timed_prn19) !=
            SIDEREON_STATUS_OK ||
        timed_prn19.prn != SMOKE_B_CONSTELLATION_TIMED_RECORD3_PRN_START ||
        timed_prn19.usable != SMOKE_B_CONSTELLATION_TIMED_RECORD3_USABLE_START) {
        sidereon_constellation_free(time_aware_catalog);
        sidereon_constellation_free(catalog);
        return fail("sidereon_constellation_build_at merged usability", 1);
    }
    sidereon_constellation_free(time_aware_catalog);

    time_aware_catalog = NULL;
    if (sidereon_constellation_build_at(
            SIDEREON_GNSS_SYSTEM_GPS, CONSTELLATION_GPS_OPS_JSON,
            CONSTELLATION_GPS_OPS_JSON_LEN, merge_forecast_html,
            merge_forecast_len, forecast_end_us, &time_aware_catalog) !=
            SIDEREON_STATUS_OK ||
        sidereon_constellation_record(time_aware_catalog, 3, &timed_prn19) !=
            SIDEREON_STATUS_OK ||
        timed_prn19.prn != SMOKE_B_CONSTELLATION_TIMED_RECORD3_PRN_END ||
        timed_prn19.usable != SMOKE_B_CONSTELLATION_TIMED_RECORD3_USABLE_END) {
        sidereon_constellation_free(time_aware_catalog);
        sidereon_constellation_free(catalog);
        return fail("sidereon_constellation_build_at forecast end", 1);
    }
    sidereon_constellation_free(time_aware_catalog);

    size_t record_count = 0;
    if (sidereon_constellation_record_count(catalog, &record_count) != SIDEREON_STATUS_OK ||
        record_count != PIN_CONSTELLATION_RECORD_COUNT) {
        sidereon_constellation_free(catalog);
        return fail("sidereon_constellation_record_count", 1);
    }

    /* Per-record accessor: the first and the fourth record of the merged
     * catalog, as sidereon-core sorts and merges it. */
    SidereonConstellationRecord rec0;
    if (sidereon_constellation_record(catalog, 0, &rec0) != SIDEREON_STATUS_OK ||
        rec0.system != SMOKE_B_CONSTELLATION_RECORD0_SYSTEM ||
        rec0.prn != SMOKE_B_CONSTELLATION_RECORD0_PRN ||
        rec0.active != SMOKE_B_CONSTELLATION_RECORD0_ACTIVE ||
        rec0.usable != SMOKE_B_CONSTELLATION_RECORD0_USABLE ||
        rec0.fdma_channel_present != SMOKE_B_CONSTELLATION_RECORD0_FDMA_CHANNEL_PRESENT) {
        sidereon_constellation_free(catalog);
        return fail("sidereon_constellation_record[0]", 1);
    }
    /* PRN 19 is present in the base source but the NAVCEN overlay marks it
     * unusable. */
    SidereonConstellationRecord rec3;
    if (sidereon_constellation_record(catalog, 3, &rec3) != SIDEREON_STATUS_OK ||
        rec3.system != SMOKE_B_CONSTELLATION_RECORD3_SYSTEM ||
        rec3.prn != SMOKE_B_CONSTELLATION_RECORD3_PRN ||
        rec3.active != SMOKE_B_CONSTELLATION_RECORD3_ACTIVE ||
        rec3.usable != SMOKE_B_CONSTELLATION_RECORD3_USABLE ||
        rec3.fdma_channel_present != SMOKE_B_CONSTELLATION_RECORD3_FDMA_CHANNEL_PRESENT) {
        sidereon_constellation_free(catalog);
        return fail("sidereon_constellation_record[3]", 1);
    }
    SidereonConstellationRecord oob;
    if (sidereon_constellation_record(catalog, record_count, &oob) !=
        SIDEREON_STATUS_INVALID_ARGUMENT) {
        sidereon_constellation_free(catalog);
        return fail("sidereon_constellation_record out-of-range index", 1);
    }

    /* Standalone system-aware SP3-id builder, for GPS PRN 5 and GLONASS slot
     * 12. Not null-terminated. */
    char sp3id[8];
    size_t sp3id_written = 123, sp3id_required = 123;
    if (sidereon_constellation_gnss_sp3_id(SIDEREON_GNSS_SYSTEM_GPS, 5, (uint8_t *)sp3id,
                                           sizeof(sp3id), &sp3id_written, &sp3id_required) !=
            SIDEREON_STATUS_OK ||
        sp3id_written != sizeof(SMOKE_B_CONSTELLATION_SP3_ID_GPS_5) - 1 ||
        memcmp(sp3id, SMOKE_B_CONSTELLATION_SP3_ID_GPS_5, sp3id_written) != 0) {
        sidereon_constellation_free(catalog);
        return fail("sidereon_constellation_gnss_sp3_id GPS", 1);
    }
    if (sidereon_constellation_gnss_sp3_id(SIDEREON_GNSS_SYSTEM_GLONASS, 12, (uint8_t *)sp3id,
                                           sizeof(sp3id), &sp3id_written, &sp3id_required) !=
            SIDEREON_STATUS_OK ||
        sp3id_written != sizeof(SMOKE_B_CONSTELLATION_SP3_ID_GLONASS_12) - 1 ||
        memcmp(sp3id, SMOKE_B_CONSTELLATION_SP3_ID_GLONASS_12, sp3id_written) != 0) {
        sidereon_constellation_free(catalog);
        return fail("sidereon_constellation_gnss_sp3_id GLONASS", 1);
    }

    /* CSV: size query, then full copy, then byte-exact compare. */
    size_t csv_written = 123, csv_required = 123;
    if (sidereon_constellation_to_csv(catalog, SIDEREON_CONSTELLATION_BOOL_STYLE_LOWER, NULL, 0,
                                      &csv_written, &csv_required) != SIDEREON_STATUS_OK ||
        csv_written != 0 || csv_required != strlen(PIN_CONSTELLATION_CSV)) {
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
        memcmp(csv, PIN_CONSTELLATION_CSV, csv_required) != 0) {
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

    /* Validate against the operational SP3 ids (G03, G05, G13). Every expected
     * value is sidereon-core's own validation, written by tests/pingen. */
    SidereonConstellationValidation *validation = NULL;
    if (sidereon_constellation_validate_against_sp3_ids(catalog, PIN_CONSTELLATION_VALIDATE_SP3_IDS,
                                                        PIN_CONSTELLATION_VALIDATE_SP3_ID_COUNT,
                                                        &validation) != SIDEREON_STATUS_OK ||
        validation == NULL) {
        sidereon_constellation_free(catalog);
        return fail("sidereon_constellation_validate_against_sp3_ids", 1);
    }

    bool valid = true;
    if (sidereon_constellation_validation_is_valid(validation, &valid) != SIDEREON_STATUS_OK ||
        valid != PIN_CONSTELLATION_VALID) {
        sidereon_constellation_validation_free(validation);
        sidereon_constellation_free(catalog);
        return fail("constellation validation is not clean (inactive PRN expected)", 1);
    }

    size_t inactive_written = 123, inactive_required = 123;
    SidereonConstellationPrn inactive[PIN_CONSTELLATION_INACTIVE_UNUSABLE_PRN_COUNT + 1];
    if (sidereon_constellation_validation_inactive_unusable_prns(
            validation, inactive, PIN_CONSTELLATION_INACTIVE_UNUSABLE_PRN_COUNT + 1,
            &inactive_written, &inactive_required) !=
            SIDEREON_STATUS_OK ||
        inactive_written != PIN_CONSTELLATION_INACTIVE_UNUSABLE_PRN_COUNT ||
        inactive_required != PIN_CONSTELLATION_INACTIVE_UNUSABLE_PRN_COUNT) {
        sidereon_constellation_validation_free(validation);
        sidereon_constellation_free(catalog);
        return fail("sidereon_constellation_validation_inactive_unusable_prns count", 1);
    }
    for (size_t i = 0; i < inactive_written; i++) {
        /* The PRN list is now system-qualified: GPS PRN 19. */
        if (inactive[i].prn != PIN_CONSTELLATION_INACTIVE_UNUSABLE_PRNS[i] ||
            inactive[i].system != SIDEREON_GNSS_SYSTEM_GPS) {
            sidereon_constellation_validation_free(validation);
            sidereon_constellation_free(catalog);
            return fail("sidereon_constellation_validation_inactive_unusable_prns value", 1);
        }
    }

    /* The missing and extra SP3 id lists, as sidereon-core reports them. */
    size_t missing_written = 123, missing_required = 123;
    if (sidereon_constellation_validation_missing_sp3_ids(validation, NULL, 0, &missing_written,
                                                          &missing_required) !=
            SIDEREON_STATUS_OK ||
        missing_written != 0 || missing_required != PIN_CONSTELLATION_MISSING_SP3_ID_COUNT) {
        sidereon_constellation_validation_free(validation);
        sidereon_constellation_free(catalog);
        return fail("sidereon_constellation_validation_missing_sp3_ids count", 1);
    }
    size_t extra_written = 123, extra_required = 123;
    if (sidereon_constellation_validation_extra_sp3_ids(validation, NULL, 0, &extra_written,
                                                        &extra_required) != SIDEREON_STATUS_OK ||
        extra_written != 0 || extra_required != PIN_CONSTELLATION_EXTRA_SP3_ID_COUNT) {
        sidereon_constellation_validation_free(validation);
        sidereon_constellation_free(catalog);
        return fail("sidereon_constellation_validation_extra_sp3_ids count", 1);
    }

    sidereon_constellation_validation_free(validation);

    /* Build without the NAVCEN overlay and validate against the same ids plus
     * G19. */
    SidereonConstellation *celestrak_only = NULL;
    if (sidereon_constellation_build(SIDEREON_GNSS_SYSTEM_GPS, CONSTELLATION_GPS_OPS_JSON,
                                     CONSTELLATION_GPS_OPS_JSON_LEN, NULL, 0,
                                     &celestrak_only) != SIDEREON_STATUS_OK ||
        celestrak_only == NULL) {
        sidereon_constellation_free(catalog);
        return fail("sidereon_constellation_build without NAVCEN", 1);
    }
    SidereonConstellationValidation *clean = NULL;
    if (sidereon_constellation_validate_against_sp3_ids(celestrak_only,
                                                        SMOKE_B_CONSTELLATION_ALL_IDS,
                                                        SMOKE_B_CONSTELLATION_ALL_ID_COUNT,
                                                        &clean) !=
            SIDEREON_STATUS_OK ||
        clean == NULL) {
        sidereon_constellation_free(celestrak_only);
        sidereon_constellation_free(catalog);
        return fail("validate CelesTrak-only catalog", 1);
    }
    bool clean_valid = false;
    if (sidereon_constellation_validation_is_valid(clean, &clean_valid) != SIDEREON_STATUS_OK ||
        clean_valid != SMOKE_B_CONSTELLATION_CELESTRAK_ONLY_VALID) {
        sidereon_constellation_validation_free(clean);
        sidereon_constellation_free(celestrak_only);
        sidereon_constellation_free(catalog);
        return fail("CelesTrak-only catalog validation", 1);
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
    printf("constellation surface: NAVCEN assessments, merged catalog, CSV and validation "
           "match sidereon-core\n");
    return 0;
}

static int selection_engine_detail_matches(const char *operation, const char *kind,
                                           const char *field) {
    SidereonEngineErrorInfo info;
    if (sidereon_last_engine_error_info(&info) != SIDEREON_STATUS_OK ||
        info.family != SIDEREON_ENGINE_ERROR_FAMILY_SELECTION || info.payload_len == 0) {
        return fail("staleness: typed SelectionError family", 1);
    }
    size_t written = 0;
    size_t required = 0;
    if (sidereon_last_engine_error_payload(NULL, 0, &written, &required) != SIDEREON_STATUS_OK ||
        written != 0 || required != info.payload_len) {
        return fail("staleness: SelectionError payload size query", 1);
    }
    uint8_t *payload = (uint8_t *)malloc(required + 1);
    if (payload == NULL) {
        return fail("staleness: SelectionError payload allocation", 1);
    }
    if (sidereon_last_engine_error_payload(payload, required, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != required) {
        free(payload);
        return fail("staleness: SelectionError payload copy", 1);
    }
    payload[written] = 0;
    char operation_field[160];
    char kind_field[96];
    (void)snprintf(operation_field, sizeof(operation_field), "\"operation\":\"%s\"", operation);
    (void)snprintf(kind_field, sizeof(kind_field), "\"kind\":\"%s\"", kind);
    int ok = strstr((const char *)payload, "\"family\":\"selection\"") != NULL &&
             strstr((const char *)payload, operation_field) != NULL &&
             strstr((const char *)payload, "\"family\":\"SelectionError\"") != NULL &&
             strstr((const char *)payload, kind_field) != NULL &&
             (field == NULL || strstr((const char *)payload, field) != NULL);
    free(payload);
    return ok ? 0 : fail("staleness: complete SelectionError detail", 1);
}

static int fallback_engine_detail_matches(const char *path, size_t used, size_t required_sats) {
    SidereonEngineErrorInfo info;
    if (sidereon_last_engine_error_info(&info) != SIDEREON_STATUS_OK ||
        info.family != SIDEREON_ENGINE_ERROR_FAMILY_SPP || info.payload_len == 0) {
        return fail("fallback: typed SppError family", 1);
    }
    size_t written = 0;
    size_t required = 0;
    if (sidereon_last_engine_error_payload(NULL, 0, &written, &required) != SIDEREON_STATUS_OK ||
        written != 0 || required != info.payload_len) {
        return fail("fallback: SppError payload size query", 1);
    }
    uint8_t *payload = (uint8_t *)malloc(required + 1);
    if (payload == NULL) {
        return fail("fallback: SppError payload allocation", 1);
    }
    if (sidereon_last_engine_error_payload(payload, required, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != required) {
        free(payload);
        return fail("fallback: SppError payload copy", 1);
    }
    payload[written] = 0;
    char outer_kind[80];
    char used_field[64];
    char required_field[64];
    (void)snprintf(outer_kind, sizeof(outer_kind), "\"kind\":\"fallback_%s\"", path);
    (void)snprintf(used_field, sizeof(used_field), "\"used\":%zu", used);
    (void)snprintf(required_field, sizeof(required_field), "\"required\":%zu", required_sats);
    int ok = strstr((const char *)payload, "\"family\":\"spp\"") != NULL &&
             strstr((const char *)payload, "\"operation\":\"sidereon_solve_with_fallback\"") !=
                 NULL &&
             strstr((const char *)payload, outer_kind) != NULL &&
             strstr((const char *)payload, "\"kind\":\"too_few_satellites\"") != NULL &&
             strstr((const char *)payload, used_field) != NULL &&
             strstr((const char *)payload, required_field) != NULL &&
             strstr((const char *)payload, "\"message\":") != NULL;
    free(payload);
    if (!ok) {
        return fail("fallback: complete nested SppError detail", 1);
    }
    char message[512] = {0};
    (void)sidereon_last_error_message(message, sizeof(message));
    const char *prefix = strcmp(path, "precise") == 0
                             ? "sidereon_solve_with_fallback: precise SPP solve failed:"
                             : "sidereon_solve_with_fallback: broadcast-fallback SPP solve failed:";
    return strncmp(message, prefix, strlen(prefix)) == 0
               ? 0
               : fail("fallback: legacy last-error message prefix", 1);
}

static int direct_broadcast_engine_detail_matches(const char *operation) {
    SidereonEngineErrorInfo info;
    if (sidereon_last_engine_error_info(&info) != SIDEREON_STATUS_OK ||
        info.family != SIDEREON_ENGINE_ERROR_FAMILY_SPP || info.payload_len == 0) {
        return fail("broadcast: typed direct SppError family", 1);
    }
    size_t written = 0;
    size_t required = 0;
    if (sidereon_last_engine_error_payload(NULL, 0, &written, &required) != SIDEREON_STATUS_OK ||
        written != 0 || required != info.payload_len) {
        return fail("broadcast: direct SppError payload size query", 1);
    }
    uint8_t *payload = (uint8_t *)malloc(required + 1);
    if (payload == NULL) {
        return fail("broadcast: direct SppError payload allocation", 1);
    }
    if (sidereon_last_engine_error_payload(payload, required, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != required) {
        free(payload);
        return fail("broadcast: direct SppError payload copy", 1);
    }
    payload[written] = 0;
    char operation_field[160];
    (void)snprintf(operation_field, sizeof(operation_field), "\"operation\":\"%s\"",
                   operation);
    const int ok = strstr((const char *)payload, "\"family\":\"spp\"") != NULL &&
                   strstr((const char *)payload, operation_field) != NULL &&
                   strstr((const char *)payload, "\"kind\":\"too_few_satellites\"") != NULL &&
                   strstr((const char *)payload, "\"used\":0") != NULL &&
                   strstr((const char *)payload, "\"required\":4") != NULL;
    free(payload);
    if (!ok) {
        return fail("broadcast: complete direct SppError payload", 1);
    }
    char message[512] = {0};
    (void)sidereon_last_error_message(message, sizeof(message));
    char message_prefix[160];
    (void)snprintf(message_prefix, sizeof(message_prefix), "%s:", operation);
    return strncmp(message, message_prefix, strlen(message_prefix)) == 0
               ? 0
               : fail("broadcast: direct legacy last-error prefix", 1);
}

static int ppp_auto_init_detail_matches(const char *operation, const char *kind, bool code_seed) {
    SidereonEngineErrorInfo info;
    if (sidereon_last_engine_error_info(&info) != SIDEREON_STATUS_OK ||
        info.family != SIDEREON_ENGINE_ERROR_FAMILY_PPP_AUTO_INIT || info.payload_len == 0) {
        return fail("PPP auto-init: typed error family", 1);
    }
    size_t written = 0;
    size_t required = 0;
    if (sidereon_last_engine_error_payload(NULL, 0, &written, &required) != SIDEREON_STATUS_OK ||
        written != 0 || required != info.payload_len) {
        return fail("PPP auto-init: payload size query", 1);
    }
    uint8_t *payload = (uint8_t *)malloc(required + 1);
    if (payload == NULL) {
        return fail("PPP auto-init: payload allocation", 1);
    }
    if (sidereon_last_engine_error_payload(payload, required, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != required) {
        free(payload);
        return fail("PPP auto-init: payload copy", 1);
    }
    payload[written] = 0;
    char operation_field[160];
    (void)snprintf(operation_field, sizeof(operation_field), "\"operation\":\"%s\"",
                   operation);
    char kind_field[128];
    (void)snprintf(kind_field, sizeof(kind_field), "\"kind\":\"%s\"", kind);
    const int ok = strstr((const char *)payload, "\"family\":\"ppp_auto_init\"") != NULL &&
                   strstr((const char *)payload, operation_field) != NULL &&
                   strstr((const char *)payload, kind_field) != NULL &&
                   (!code_seed ||
                    (strstr((const char *)payload, "\"epoch_index\":0") != NULL &&
                     strstr((const char *)payload,
                            "\"kind\":\"too_few_satellites\"") != NULL &&
                     strstr((const char *)payload, "\"used\":0") != NULL &&
                     strstr((const char *)payload, "\"required\":4") != NULL));
    free(payload);
    if (!ok) {
        return fail("PPP auto-init: full EmptyEpochs detail", 1);
    }
    const char *legacy_text = code_seed ? "PPP code seed failed at epoch 0:"
                                        : "PPP auto-init requires at least one epoch";
    return last_error_contains(legacy_text) ? 0 : fail("PPP auto-init: legacy error text", 1);
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
    if (f64_to_bits(covered_epoch) != SMOKE_B_STALENESS_COVERED_EPOCH_BITS ||
        f64_to_bits(last_epoch) != SMOKE_B_STALENESS_LAST_EPOCH_BITS) {
        return fail("staleness: sp3 epochs", 1);
    }

    /* Every selection outcome below is sidereon-core's (tests/valgen bin
     * smoke_b_staleness). */
    SidereonStalenessPolicy policy = sidereon_staleness_policy_default();
    if (f64_to_bits(policy.max_staleness_s) != SMOKE_B_STALENESS_DEFAULT_CAP_S_BITS) {
        return fail("staleness: default policy cap", 1);
    }

    const SidereonSp3 *one_sp3[1] = {sp3};

    /* Exact: a covered epoch returns a byte-identical clone with zero staleness,
     * and interpolating it matches the original product bit-for-bit. */
    SidereonSp3 *exact_sel = NULL;
    SidereonStalenessMetadata exact_meta;
    SidereonSelectionStatus sel_status =
        sidereon_select_sp3(one_sp3, 1, covered_epoch, policy, &exact_sel, &exact_meta);
    if (!staleness_matches(sel_status, &exact_meta, &SMOKE_B_STALENESS_SP3_EXACT) ||
        exact_sel == NULL) {
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
    sel_status = sidereon_select_sp3(one_sp3, 1, stale_epoch, policy, &prior_sel, &prior_meta);
    if (!staleness_matches(sel_status, &prior_meta, &SMOKE_B_STALENESS_SP3_PRIOR) ||
        prior_sel == NULL) {
        sidereon_sp3_free(prior_sel);
        return fail("staleness: select_sp3 nearest-prior metadata", 1);
    }
    sidereon_sp3_free(prior_sel);

    /* Beyond cap: a 1-second cap rejects the 100-second-stale prior with the
     * typed BeyondStalenessCap status and writes no handle. */
    SidereonSp3 *cap_sel = (SidereonSp3 *)(uintptr_t)1;
    SidereonStalenessMetadata cap_meta;
    sel_status = sidereon_select_sp3(one_sp3, 1, stale_epoch,
                                     sidereon_staleness_policy_seconds(1.0), &cap_sel, &cap_meta);
    if (!staleness_matches(sel_status, &cap_meta, &SMOKE_B_STALENESS_SP3_CAP) ||
        (sel_status != SIDEREON_SELECTION_STATUS_OK && cap_sel != NULL)) {
        sidereon_sp3_free(cap_sel);
        return fail("staleness: select_sp3 beyond-cap typed error", 1);
    }
    if (selection_engine_detail_matches("sidereon_select_sp3_over_range", "beyond_staleness_cap",
                                        "\"max_staleness_s\"")) {
        return 1;
    }

    /* Empty product set: the typed EmptyProductSet status, no handle. */
    SidereonSp3 *empty_sel = (SidereonSp3 *)(uintptr_t)1;
    SidereonStalenessMetadata empty_meta;
    sel_status = sidereon_select_sp3(NULL, 0, covered_epoch, policy, &empty_sel, &empty_meta);
    if (!staleness_matches(sel_status, &empty_meta, &SMOKE_B_STALENESS_SP3_EMPTY) ||
        (sel_status != SIDEREON_SELECTION_STATUS_OK && empty_sel != NULL)) {
        sidereon_sp3_free(empty_sel);
        return fail("staleness: select_sp3 empty-set typed error", 1);
    }
    if (selection_engine_detail_matches("sidereon_select_sp3_over_range", "empty_product_set",
                                        "\"message\":\"product set is empty\"")) {
        return 1;
    }

    /* No prior product: the loaded SP3 cannot serve an epoch a week before its
     * earliest coverage (covered_epoch - 7 days), so selection has no prior. */
    SidereonSp3 *no_prior_sel = (SidereonSp3 *)(uintptr_t)1;
    SidereonStalenessMetadata no_prior_meta;
    sel_status = sidereon_select_sp3(one_sp3, 1, covered_epoch - 7.0 * 86400.0, policy,
                                     &no_prior_sel, &no_prior_meta);
    if (!staleness_matches(sel_status, &no_prior_meta, &SMOKE_B_STALENESS_SP3_NO_PRIOR) ||
        (sel_status != SIDEREON_SELECTION_STATUS_OK && no_prior_sel != NULL)) {
        sidereon_sp3_free(no_prior_sel);
        return fail("staleness: select_sp3 no-prior typed error", 1);
    }
    if (selection_engine_detail_matches("sidereon_select_sp3_over_range", "no_prior_product",
                                        "\"requested_epoch_j2000_s\"")) {
        return 1;
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
    sel_status = sidereon_select_ionex(one_ionex, 1, covered_ionex_epoch, policy, &ionex_exact,
                                       &ionex_exact_meta);
    if (!staleness_matches(sel_status, &ionex_exact_meta, &SMOKE_B_STALENESS_IONEX_EXACT) ||
        ionex_exact == NULL) {
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
    sel_status = sidereon_select_ionex(one_ionex, 1, shifted_epoch, policy, &ionex_shift,
                                       &ionex_shift_meta);
    if (!staleness_matches(sel_status, &ionex_shift_meta, &SMOKE_B_STALENESS_IONEX_SHIFT) ||
        ionex_shift == NULL) {
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
    sel_status = sidereon_select_ionex(one_ionex, 1, covered_ionex_epoch, nan_policy,
                                       &bad_policy_sel, &bad_policy_meta);
    if (!staleness_matches(sel_status, &bad_policy_meta, &SMOKE_B_STALENESS_IONEX_NAN_POLICY) ||
        (sel_status != SIDEREON_SELECTION_STATUS_OK && bad_policy_sel != NULL)) {
        sidereon_ionex_free(bad_policy_sel);
        sidereon_ionex_free(ionex);
        return fail("staleness: select_ionex invalid-policy typed error", 1);
    }
    if (selection_engine_detail_matches("sidereon_select_ionex_over_range", "invalid_policy",
                                        "\"max_staleness_s\"")) {
        sidereon_ionex_free(ionex);
        return 1;
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

/* File-path RINEX NAV/OBS loaders plus the RINEX OBS -> SPP assembly and solve
 * conveniences. Uses the committed ESBC mixed NAV and trimmed OBS fixtures. */
static int exercise_rinex_spp_surface(const char *nav_path, const char *obs_path) {
    SidereonBroadcastEphemeris *broadcast = NULL;
    SidereonRinexObs *obs = NULL;
    SidereonRinexSppInputs *assembled = NULL;
    SidereonRinexSppSolutions *solutions = NULL;
    int rc = 1;

    if (sidereon_broadcast_ephemeris_load_nav(nav_path, &broadcast) != SIDEREON_STATUS_OK ||
        broadcast == NULL) {
        return fail("rinex_spp: sidereon_broadcast_ephemeris_load_nav", 1);
    }
    if (sidereon_rinex_obs_load(obs_path, &obs) != SIDEREON_STATUS_OK || obs == NULL) {
        (void)fail("rinex_spp: sidereon_rinex_obs_load", 1);
        goto cleanup;
    }

    SidereonRinexSppOptions options;
    if (sidereon_rinex_spp_options_init(&options) != SIDEREON_STATUS_OK) {
        (void)fail("rinex_spp: options_init", 1);
        goto cleanup;
    }

    if (sidereon_spp_inputs_from_rinex_obs(obs, broadcast, &options, &assembled) !=
            SIDEREON_STATUS_OK ||
        assembled == NULL) {
        (void)fail("rinex_spp: sidereon_spp_inputs_from_rinex_obs", 1);
        goto cleanup;
    }

    size_t raw_epoch_count = 0;
    size_t assembled_count = 0;
    if (sidereon_rinex_obs_epoch_count(obs, &raw_epoch_count) != SIDEREON_STATUS_OK ||
        sidereon_rinex_spp_inputs_count(assembled, &assembled_count) != SIDEREON_STATUS_OK ||
        raw_epoch_count != SMOKE_B_POSITIONING_RINEX_RAW_EPOCHS ||
        assembled_count != SMOKE_B_POSITIONING_RINEX_ASSEMBLED) {
        (void)fail("rinex_spp: assembled epoch counts", 1);
        goto cleanup;
    }

    SidereonRinexSppEpoch first_epoch;
    SidereonSppInputsV2 first_inputs;
    if (sidereon_rinex_spp_inputs_epoch(assembled, 0, &first_epoch) != SIDEREON_STATUS_OK ||
        sidereon_rinex_spp_inputs_epoch_inputs(assembled, 0, &first_inputs) !=
            SIDEREON_STATUS_OK ||
        first_epoch.observation_count != SMOKE_B_POSITIONING_RINEX_FIRST_OBS_COUNT ||
        first_inputs.base.observation_count != first_epoch.observation_count ||
        first_inputs.base.observations == NULL ||
        f64_to_bits(first_inputs.base.t_rx_j2000_s) != SMOKE_B_POSITIONING_RINEX_FIRST_T_RX_BITS) {
        (void)fail("rinex_spp: assembled epoch accessors", 1);
        goto cleanup;
    }

    if (sidereon_solve_spp_from_rinex_obs(broadcast, obs, &options, true, NULL, &solutions) !=
            SIDEREON_STATUS_OK ||
        solutions == NULL) {
        (void)fail("rinex_spp: sidereon_solve_spp_from_rinex_obs", 1);
        goto cleanup;
    }

    size_t solution_count = 0;
    if (sidereon_rinex_spp_solutions_count(solutions, &solution_count) != SIDEREON_STATUS_OK ||
        solution_count != SMOKE_B_POSITIONING_RINEX_SOLUTIONS) {
        (void)fail("rinex_spp: solution count", 1);
        goto cleanup;
    }

    /* Which epochs solve, and the first solved epoch's fix, are
     * sidereon-core's (tests/valgen bin smoke_b_positioning). */
    for (size_t i = 0; i < solution_count; i++) {
        bool ok = false;
        if (sidereon_rinex_spp_solution_ok(solutions, i, &ok) != SIDEREON_STATUS_OK ||
            ok != SMOKE_B_POSITIONING_RINEX_SOLUTION_OK[i]) {
            (void)fail("rinex_spp: solution_ok", 1);
            goto cleanup;
        }
    }
    {
        const size_t i = SMOKE_B_POSITIONING_RINEX_FIRST_SOLVED;
        SidereonRinexSppEpoch solved_epoch;
        SidereonSppSolution *sol = NULL;
        double pos[3];
        double clock = 0.0;
        size_t used = 0;
        if (sidereon_rinex_spp_solutions_epoch(solutions, i, &solved_epoch) !=
                SIDEREON_STATUS_OK ||
            sidereon_rinex_spp_solution(solutions, i, &sol) != SIDEREON_STATUS_OK ||
            sol == NULL ||
            spp_solution_pos_clock(sol, pos, &clock, &used) != 0) {
            sidereon_spp_solution_free(sol);
            (void)fail("rinex_spp: solved epoch readout", 1);
            goto cleanup;
        }
        sidereon_spp_solution_free(sol);
        if (solved_epoch.epoch_index != SMOKE_B_POSITIONING_RINEX_FIRST_SOLVED_EPOCH_INDEX ||
            used != SMOKE_B_POSITIONING_RINEX_FIRST_SOLVED_USED ||
            f64_to_bits(clock) != SMOKE_B_POSITIONING_RINEX_FIRST_SOLVED_CLOCK_BITS ||
            f64_to_bits(pos[0]) != SMOKE_B_POSITIONING_RINEX_FIRST_SOLVED_POS_BITS[0] ||
            f64_to_bits(pos[1]) != SMOKE_B_POSITIONING_RINEX_FIRST_SOLVED_POS_BITS[1] ||
            f64_to_bits(pos[2]) != SMOKE_B_POSITIONING_RINEX_FIRST_SOLVED_POS_BITS[2]) {
            (void)fail("rinex_spp: first solved epoch", 1);
            goto cleanup;
        }
    }

    printf("RINEX SPP surface: path NAV/OBS loaders + assembled inputs + serial solve\n");
    rc = 0;

cleanup:
    sidereon_rinex_spp_solutions_free(solutions);
    sidereon_rinex_spp_inputs_free(assembled);
    sidereon_rinex_obs_free(obs);
    sidereon_broadcast_ephemeris_free(broadcast);
    return rc;
}

static int fix_matches(const double pos[3], double clock, size_t used, const uint64_t pos_bits[3],
                       uint64_t clock_bits, size_t expected_used) {
    return f64_to_bits(pos[0]) == pos_bits[0] && f64_to_bits(pos[1]) == pos_bits[1] &&
           f64_to_bits(pos[2]) == pos_bits[2] && f64_to_bits(clock) == clock_bits &&
           used == expected_used;
}

static int staleness_meta_matches(const SidereonStalenessMetadata *meta, SidereonDegradationKind kind,
                                  uint64_t staleness_s_bits, uint64_t source_bits) {
    return meta->kind == kind && f64_to_bits(meta->staleness_s) == staleness_s_bits &&
           f64_to_bits(meta->source_epoch_j2000_s) == source_bits;
}

/* The provenance of a sourced solution against sidereon-core's (tests/valgen
 * bin smoke_b_positioning). */
static int sourced_matches(const SidereonSourcedSolution *sol, const SmokeBSourcedPin *pin) {
    SidereonFixSourceKind kind = SIDEREON_FIX_SOURCE_KIND_BROADCAST;
    bool is_exact = !pin->precise_exact;
    SidereonStalenessMetadata meta;
    bool present = !pin->has_staleness;
    if (sidereon_sourced_solution_source_kind(sol, &kind) != SIDEREON_STATUS_OK ||
        (kind == SIDEREON_FIX_SOURCE_KIND_PRECISE) != pin->precise ||
        sidereon_sourced_solution_is_precise_exact(sol, &is_exact) != SIDEREON_STATUS_OK ||
        is_exact != pin->precise_exact ||
        sidereon_sourced_solution_staleness(sol, &meta, &present) != SIDEREON_STATUS_OK ||
        present != pin->has_staleness ||
        (present && !staleness_meta_matches(&meta, pin->staleness_kind, pin->staleness_s_bits,
                                            pin->source_epoch_bits))) {
        return 0;
    }
    if (pin->precise) {
        return 1;
    }
    SidereonBroadcastReasonKind reason = SIDEREON_BROADCAST_REASON_KIND_PRECISE_DEGRADED_UNUSABLE;
    SidereonSelectionStatus unavailable = SIDEREON_SELECTION_STATUS_OK;
    SidereonStalenessMetadata attempted;
    bool has_attempted = !pin->has_attempted;
    return sidereon_sourced_solution_broadcast_reason(sol, &reason, &unavailable, &attempted,
                                                      &has_attempted) == SIDEREON_STATUS_OK &&
           reason == pin->reason && unavailable == pin->unavailable &&
           has_attempted == pin->has_attempted &&
           (!has_attempted ||
            staleness_meta_matches(&attempted, pin->attempted_kind,
                                   pin->attempted_staleness_s_bits,
                                   pin->attempted_source_epoch_bits));
}

/* Broadcast-ephemeris SPP and the precise-with-broadcast fallback through the C
 * ABI. Inputs are the ESBC DOY177 GPS C1C first-epoch observations extracted from
 * the engine into broadcast_fixture.h; the products are the committed broadcast
 * NAV and SP3 fixtures. Asserts the source + staleness provenance on each branch,
 * the labeled broadcast-vs-precise agreement, and bit-exact broadcast fallback. */
static int sourced_reason_detail_contains(const SidereonSourcedSolution *sol,
                                          const char *outer_kind, const char *inner_kind) {
    size_t written = 0;
    size_t required = 0;
    if (sidereon_sourced_solution_broadcast_reason_detail(sol, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != 0 || required == 0) {
        return fail("broadcast: reason detail size query", 1);
    }
    uint8_t *bytes = (uint8_t *)malloc(required + 1);
    if (bytes == NULL) {
        return fail("broadcast: reason detail allocation", 1);
    }
    if (sidereon_sourced_solution_broadcast_reason_detail(sol, bytes, required, &written,
                                                         &required) != SIDEREON_STATUS_OK ||
        written != required) {
        free(bytes);
        return fail("broadcast: reason detail copy", 1);
    }
    bytes[written] = 0;
    char outer[96];
    char inner[96];
    (void)snprintf(outer, sizeof(outer), "\"kind\":\"%s\"", outer_kind);
    (void)snprintf(inner, sizeof(inner), "\"kind\":\"%s\"", inner_kind);
    int ok = strstr((const char *)bytes, "\"family\":\"BroadcastReason\"") != NULL &&
             strstr((const char *)bytes, outer) != NULL &&
             strstr((const char *)bytes, inner) != NULL;
    if (ok && strcmp(inner_kind, "too_few_satellites") == 0) {
        ok = strstr((const char *)bytes, "\"used\":0") != NULL &&
             strstr((const char *)bytes, "\"required\":4") != NULL &&
             strstr((const char *)bytes, "\"requested_epoch_j2000_s\"") != NULL &&
             strstr((const char *)bytes, "\"source_epoch_j2000_s\"") != NULL &&
             strstr((const char *)bytes, "\"staleness_s\"") != NULL &&
             strstr((const char *)bytes, "\"staleness_days\"") != NULL;
    } else if (ok && strcmp(inner_kind, "no_prior_product") == 0) {
        ok = strstr((const char *)bytes, "\"requested_epoch_j2000_s\"") != NULL &&
             strstr((const char *)bytes, "\"message\"") != NULL;
    }
    free(bytes);
    return ok ? 0 : fail("broadcast: typed reason detail payload", 1);
}

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
    inputs.pseudorange_code = SIDEREON_PSEUDORANGE_CODE_SINGLE_FREQUENCY;

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
    if (f64_to_bits(policy.max_staleness_s) != SMOKE_B_POSITIONING_FALLBACK_CAP_S_BITS) {
        (void)fail("broadcast: staleness policy cap", 1);
        goto cleanup;
    }

    /* Broadcast-only SPP: the supported real-time / offline mode. */
    SidereonSppSolution *broadcast_sol = NULL;
    if (sidereon_solve_broadcast(broadcast, &inputs, &broadcast_sol) != SIDEREON_STATUS_OK) {
        (void)fail("broadcast: sidereon_solve_broadcast", 1);
        goto cleanup;
    }
    double pos_b[3];
    double clk_b;
    size_t used_b;
    if (spp_solution_pos_clock(broadcast_sol, pos_b, &clk_b, &used_b) ||
        !fix_matches(pos_b, clk_b, used_b, SMOKE_B_POSITIONING_BROADCAST_POS_BITS,
                     SMOKE_B_POSITIONING_BROADCAST_CLOCK_BITS, SMOKE_B_POSITIONING_BROADCAST_USED)) {
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: broadcast-only solution readout", 1);
        goto cleanup;
    }
    const SidereonSp3 *precise_set[1] = {precise};

    /* A fallback output slot may not overwrite the live SP3 handle that the
     * precise product list points to. The refusal happens before any write. */
    size_t precise_epoch_count_before = 0;
    size_t precise_epoch_count_after = 0;
    if (sidereon_sp3_epoch_count(precise, &precise_epoch_count_before) != SIDEREON_STATUS_OK ||
        sidereon_solve_with_fallback(
            precise_set, 1, broadcast, &inputs, policy,
            (SidereonSourcedSolution **)(uintptr_t)(const void *)precise) !=
            SIDEREON_FALLBACK_STATUS_INVALID_ARGUMENT ||
        sidereon_solve_with_fallback(
            precise_set, 1, broadcast, NULL, policy,
            (SidereonSourcedSolution **)(uintptr_t)(const void *)precise) !=
            SIDEREON_FALLBACK_STATUS_INVALID_ARGUMENT ||
        sidereon_sp3_epoch_count(precise, &precise_epoch_count_after) != SIDEREON_STATUS_OK ||
        precise_epoch_count_after != precise_epoch_count_before) {
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: fallback refuses output overlapping live precise handle", 1);
        goto cleanup;
    }

    /* Exact precise selection with no configured observations refuses with the
     * complete nested TooFewSatellites error; it does not silently rerun the
     * same request on broadcast. */
    SidereonSppInputs no_precise_observations = inputs;
    no_precise_observations.observation_count = 0;
    SidereonSourcedSolution *failed_precise = (SidereonSourcedSolution *)(uintptr_t)1;
    if (sidereon_solve_with_fallback(precise_set, 1, broadcast, &no_precise_observations, policy,
                                    &failed_precise) != SIDEREON_FALLBACK_STATUS_PRECISE_SOLVE ||
        failed_precise != NULL || fallback_engine_detail_matches("precise", 0, 4) != 0) {
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: precise fallback refusal keeps full SppError", 1);
        goto cleanup;
    }

    /* With no precise products and no observations, the broadcast path fails
     * independently and retains its distinct FallbackError path plus cause. */
    SidereonSppInputs no_observations = inputs;
    no_observations.observation_count = 0;
    SidereonSourcedSolution *failed_broadcast = (SidereonSourcedSolution *)(uintptr_t)1;
    if (sidereon_solve_with_fallback(NULL, 0, broadcast, &no_observations, policy,
                                    &failed_broadcast) !=
            SIDEREON_FALLBACK_STATUS_BROADCAST_SOLVE ||
        failed_broadcast != NULL || fallback_engine_detail_matches("broadcast", 0, 4) != 0) {
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: broadcast fallback refusal keeps full SppError", 1);
        goto cleanup;
    }
    SidereonSppSolution *direct_failed = (SidereonSppSolution *)(uintptr_t)1;
    if (sidereon_solve_broadcast(broadcast, &no_observations, &direct_failed) !=
            SIDEREON_STATUS_SOLVE ||
        direct_failed != NULL ||
        direct_broadcast_engine_detail_matches("sidereon_solve_broadcast") != 0) {
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: direct SPP refusal keeps full SppError", 1);
        goto cleanup;
    }
    SidereonSppInputsV2 no_observations_v2;
    if (sidereon_spp_inputs_v2_init(&no_observations_v2) != SIDEREON_STATUS_OK) {
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: initialize V2 direct failure control", 1);
        goto cleanup;
    }
    no_observations_v2.base = no_observations;
    SidereonSppSolution *direct_v2_failed = (SidereonSppSolution *)(uintptr_t)1;
    if (sidereon_solve_broadcast_with_models(broadcast, &no_observations_v2, NULL,
                                             &direct_v2_failed) != SIDEREON_STATUS_SOLVE ||
        direct_v2_failed != NULL ||
        direct_broadcast_engine_detail_matches("sidereon_solve_broadcast_with_models") != 0) {
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: V2 direct SPP refusal keeps full SppError", 1);
        goto cleanup;
    }

    /* Fallback with a precise product covering the epoch. Both fixes are
     * sidereon-core's; the C side adds that they differ and agree within the
     * labeled signal-in-space bound. */
    SidereonSourcedSolution *fb_exact = NULL;
    if (sidereon_solve_with_fallback(precise_set, 1, broadcast, &inputs, policy, &fb_exact) !=
        SIDEREON_FALLBACK_STATUS_OK) {
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: fallback exact solve", 1);
        goto cleanup;
    }
    SidereonEngineErrorInfo cleared_fallback_info;
    if (sidereon_last_engine_error_info(&cleared_fallback_info) != SIDEREON_STATUS_OK ||
        cleared_fallback_info.family != SIDEREON_ENGINE_ERROR_FAMILY_NONE ||
        cleared_fallback_info.payload_len != 0) {
        sidereon_sourced_solution_free(fb_exact);
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: successful fallback clears prior typed failure", 1);
        goto cleanup;
    }
    if (!sourced_matches(fb_exact, &SMOKE_B_POSITIONING_FB_EXACT)) {
        sidereon_sourced_solution_free(fb_exact);
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: fallback exact provenance", 1);
        goto cleanup;
    }
    double pos_e[3];
    double clk_e;
    size_t used_e;
    if (sourced_pos_clock(fb_exact, pos_e, &clk_e, &used_e) ||
        !fix_matches(pos_e, clk_e, used_e, SMOKE_B_POSITIONING_FB_EXACT_POS_BITS,
                     SMOKE_B_POSITIONING_FB_EXACT_CLOCK_BITS, SMOKE_B_POSITIONING_FB_EXACT_USED)) {
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

    /* Fallback with no precise product: the provenance is sidereon-core's, and
     * the fix also equals the broadcast-only solve. */
    SidereonSourcedSolution *fb_empty = NULL;
    if (sidereon_solve_with_fallback(NULL, 0, broadcast, &inputs, policy, &fb_empty) !=
        SIDEREON_FALLBACK_STATUS_OK) {
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: fallback empty solve", 1);
        goto cleanup;
    }
    if (!sourced_matches(fb_empty, &SMOKE_B_POSITIONING_FB_EMPTY)) {
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
        f64_to_bits(clk_x) != f64_to_bits(clk_b) || used_x != used_b ||
        !fix_matches(pos_x, clk_x, used_x, SMOKE_B_POSITIONING_FB_EMPTY_POS_BITS,
                     SMOKE_B_POSITIONING_FB_EMPTY_CLOCK_BITS, SMOKE_B_POSITIONING_FB_EMPTY_USED)) {
        sidereon_sourced_solution_free(fb_empty);
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: fallback empty not bit-exact vs broadcast-only", 1);
        goto cleanup;
    }
    sidereon_sourced_solution_free(fb_empty);

    /* Fallback with the 2026 precise product; the provenance is
     * sidereon-core's. */
    const SidereonSp3 *wrong_set[1] = {wrong};
    SidereonSourcedSolution *fb_wrong = NULL;
    if (sidereon_solve_with_fallback(wrong_set, 1, broadcast, &inputs, policy, &fb_wrong) !=
        SIDEREON_FALLBACK_STATUS_OK) {
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: fallback wrong-epoch solve", 1);
        goto cleanup;
    }
    if (!sourced_matches(fb_wrong, &SMOKE_B_POSITIONING_FB_WRONG)) {
        sidereon_sourced_solution_free(fb_wrong);
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: fallback wrong-epoch provenance", 1);
        goto cleanup;
    }
    if (sourced_reason_detail_contains(fb_wrong, "precise_unavailable", "no_prior_product")) {
        sidereon_sourced_solution_free(fb_wrong);
        sidereon_spp_solution_free(broadcast_sol);
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
        SidereonBroadcastReasonKind reason_kind;
        SidereonSelectionStatus unavail;
        SidereonStalenessMetadata attempted;
        bool has_attempted;
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
        f64_to_bits(clk_x) != f64_to_bits(clk_b) ||
        !fix_matches(pos_x, clk_x, used_x, SMOKE_B_POSITIONING_FB_WRONG_POS_BITS,
                     SMOKE_B_POSITIONING_FB_WRONG_CLOCK_BITS, SMOKE_B_POSITIONING_FB_WRONG_USED)) {
        sidereon_sourced_solution_free(fb_wrong);
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: fallback wrong-epoch not bit-exact vs broadcast-only", 1);
        goto cleanup;
    }
    sidereon_sourced_solution_free(fb_wrong);

    /* Fallback with a prior-day precise product; source, staleness and fix are
     * sidereon-core's. */
    const SidereonSp3 *prior_set[1] = {prior};
    SidereonSourcedSolution *fb_prior = NULL;
    if (sidereon_solve_with_fallback(prior_set, 1, broadcast, &inputs, policy, &fb_prior) !=
        SIDEREON_FALLBACK_STATUS_OK) {
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: fallback degraded-precise solve", 1);
        goto cleanup;
    }
    if (!sourced_matches(fb_prior, &SMOKE_B_POSITIONING_FB_PRIOR)) {
        sidereon_sourced_solution_free(fb_prior);
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: fallback degraded-precise provenance", 1);
        goto cleanup;
    }
    double pos_p[3];
    double clk_p;
    size_t used_p;
    if (sourced_pos_clock(fb_prior, pos_p, &clk_p, &used_p) ||
        !fix_matches(pos_p, clk_p, used_p, SMOKE_B_POSITIONING_FB_PRIOR_POS_BITS,
                     SMOKE_B_POSITIONING_FB_PRIOR_CLOCK_BITS, SMOKE_B_POSITIONING_FB_PRIOR_USED)) {
        sidereon_sourced_solution_free(fb_prior);
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: fallback degraded-precise readout", 1);
        goto cleanup;
    }
    sidereon_sourced_solution_free(fb_prior);

    /* Rename the real prior-day SP3's GPS satellite records into an otherwise
     * unused constellation. The stale selection still succeeds, while precise
     * SPP sees zero of the configured GPS observations and returns the core's
     * TooFewSatellites error before broadcast fallback. */
    size_t prior_text_written = 0;
    size_t prior_text_required = 0;
    if (sidereon_sp3_to_sp3_text(prior, NULL, 0, &prior_text_written, &prior_text_required) !=
            SIDEREON_STATUS_OK ||
        prior_text_written != 0 || prior_text_required == 0) {
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: prior SP3 text size query", 1);
        goto cleanup;
    }
    uint8_t *prior_text = (uint8_t *)malloc(prior_text_required + 1);
    if (prior_text == NULL ||
        sidereon_sp3_to_sp3_text(prior, prior_text, prior_text_required, &prior_text_written,
                                 &prior_text_required) != SIDEREON_STATUS_OK ||
        prior_text_written != prior_text_required) {
        free(prior_text);
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: prior SP3 text copy", 1);
        goto cleanup;
    }
    prior_text[prior_text_written] = 0;
    bool present_system[26] = {false};
    for (size_t i = 0; i + 2 < prior_text_written; i++) {
        if (prior_text[i] >= 'A' && prior_text[i] <= 'Z' &&
            prior_text[i + 1] >= '0' && prior_text[i + 1] <= '9' &&
            prior_text[i + 2] >= '0' && prior_text[i + 2] <= '9') {
            present_system[prior_text[i] - 'A'] = true;
        }
    }
    const char *replacement_systems = "RECIJ";
    char replacement_system = '\0';
    for (const char *candidate = replacement_systems; *candidate != '\0'; candidate++) {
        if (!present_system[*candidate - 'A']) {
            replacement_system = *candidate;
            break;
        }
    }
    if (replacement_system == '\0') {
        free(prior_text);
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: choose unused SP3 constellation", 1);
        goto cleanup;
    }
    size_t renamed_gps_records = 0;
    for (size_t i = 0; i + 2 < prior_text_written; i++) {
        if (prior_text[i] == 'G' && prior_text[i + 1] >= '0' && prior_text[i + 1] <= '9' &&
            prior_text[i + 2] >= '0' && prior_text[i + 2] <= '9') {
            prior_text[i] = (uint8_t)replacement_system;
            renamed_gps_records++;
        }
    }
    SidereonSp3 *unusable_prior = NULL;
    if (renamed_gps_records == 0 ||
        sidereon_sp3_load(prior_text, prior_text_written, &unusable_prior) != SIDEREON_STATUS_OK) {
        free(prior_text);
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: parse renamed prior SP3", 1);
        goto cleanup;
    }
    free(prior_text);
    size_t renamed_satellites_written = 0;
    size_t renamed_satellites_required = 0;
    if (sidereon_sp3_satellites(unusable_prior, NULL, 0, &renamed_satellites_written,
                                &renamed_satellites_required) != SIDEREON_STATUS_OK ||
        renamed_satellites_written != 0) {
        sidereon_sp3_free(unusable_prior);
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: renamed prior satellite size query", 1);
        goto cleanup;
    }
    SidereonSatelliteToken *renamed_satellites = (SidereonSatelliteToken *)malloc(
        renamed_satellites_required * sizeof(*renamed_satellites));
    if (renamed_satellites == NULL ||
        sidereon_sp3_satellites(unusable_prior, renamed_satellites,
                                renamed_satellites_required, &renamed_satellites_written,
                                &renamed_satellites_required) != SIDEREON_STATUS_OK ||
        renamed_satellites_written != renamed_satellites_required) {
        free(renamed_satellites);
        sidereon_sp3_free(unusable_prior);
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: renamed prior satellite readout", 1);
        goto cleanup;
    }
    bool retained_gps = false;
    for (size_t i = 0; i < renamed_satellites_written; i++) {
        retained_gps |= renamed_satellites[i].bytes[0] == 'G';
    }
    free(renamed_satellites);
    if (retained_gps) {
        sidereon_sp3_free(unusable_prior);
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: renamed prior has no GPS records", 1);
        goto cleanup;
    }
    const SidereonSp3 *unusable_prior_set[1] = {unusable_prior};
    SidereonSourcedSolution *fb_degraded_unusable = NULL;
    if (sidereon_solve_with_fallback(unusable_prior_set, 1, broadcast, &inputs, policy,
                                    &fb_degraded_unusable) != SIDEREON_FALLBACK_STATUS_OK) {
        sidereon_sp3_free(unusable_prior);
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: degraded-but-unusable fallback solve", 1);
        goto cleanup;
    }
    SidereonFixSourceKind unusable_source;
    if (sidereon_sourced_solution_source_kind(fb_degraded_unusable, &unusable_source) !=
            SIDEREON_STATUS_OK ||
        unusable_source != SIDEREON_FIX_SOURCE_KIND_BROADCAST ||
        sourced_reason_detail_contains(fb_degraded_unusable, "precise_degraded_unusable",
                                       "too_few_satellites")) {
        sidereon_sourced_solution_free(fb_degraded_unusable);
        sidereon_sp3_free(unusable_prior);
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: degraded-unusable typed provenance", 1);
        goto cleanup;
    }
    if (sourced_pos_clock(fb_degraded_unusable, pos_x, &clk_x, &used_x) ||
        f64_to_bits(pos_x[0]) != f64_to_bits(pos_b[0]) ||
        f64_to_bits(pos_x[1]) != f64_to_bits(pos_b[1]) ||
        f64_to_bits(pos_x[2]) != f64_to_bits(pos_b[2]) ||
        f64_to_bits(clk_x) != f64_to_bits(clk_b) || used_x != used_b) {
        sidereon_sourced_solution_free(fb_degraded_unusable);
        sidereon_sp3_free(unusable_prior);
        sidereon_spp_solution_free(broadcast_sol);
        (void)fail("broadcast: degraded-unusable result equals broadcast-only", 1);
        goto cleanup;
    }
    sidereon_sourced_solution_free(fb_degraded_unusable);
    sidereon_sp3_free(unusable_prior);

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
    /* The ISS element set tests/valgen bin smoke_b_tle reads the same text
     * from; every expected value below is sidereon-core's reading of it. */
    const char *ISS_L1 = SMOKE_B_TLE_ISS_L1;
    const char *ISS_L2 = SMOKE_B_TLE_ISS_L2;

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
    if (sidereon_tle_file_count(file, &count) != SIDEREON_STATUS_OK || count != SMOKE_B_TLE_COUNT) {
        sidereon_tle_file_free(file);
        return fail("sidereon_tle_file_count", 1);
    }

    size_t skipped = 99;
    if (sidereon_tle_file_skipped(file, &skipped) != SIDEREON_STATUS_OK ||
        skipped != SMOKE_B_TLE_SKIPPED) {
        sidereon_tle_file_free(file);
        return fail("sidereon_tle_file_skipped", 1);
    }
    SidereonTleRejectedRecord rejected;
    if (sidereon_tle_file_rejected(file, 0, &rejected) != SIDEREON_STATUS_OK ||
        rejected.line_number != SMOKE_B_TLE_REJECTED0_LINE ||
        rejected.issue != SMOKE_B_TLE_REJECTED0_ISSUE) {
        sidereon_tle_file_free(file);
        return fail("sidereon_tle_file_rejected", 1);
    }
    size_t rejected_written = 0;
    size_t rejected_required = 0;
    if (sidereon_tle_file_rejected_error(file, 0, NULL, 0, &rejected_written,
                                         &rejected_required) != SIDEREON_STATUS_OK ||
        rejected_required != sizeof(SMOKE_B_TLE_REJECTED0_ERROR) - 1) {
        sidereon_tle_file_free(file);
        return fail("sidereon_tle_file_rejected_error", 1);
    }
    size_t first_line = 0;
    if (sidereon_tle_file_line_number(file, 0, &first_line) != SIDEREON_STATUS_OK ||
        first_line != SMOKE_B_TLE_RECORD0_LINE) {
        sidereon_tle_file_free(file);
        return fail("sidereon_tle_file_line_number", 1);
    }

    /* Query-then-fill the name of the first record. */
    size_t name_required = 0;
    if (sidereon_tle_file_name(file, 0, NULL, 0, &name_required) != SIDEREON_STATUS_OK ||
        name_required != sizeof(SMOKE_B_TLE_RECORD0_NAME)) {
        sidereon_tle_file_free(file);
        return fail("sidereon_tle_file_name size query", 1);
    }
    char name[64];
    size_t name_required2 = 0;
    if (sidereon_tle_file_name(file, 0, name, sizeof(name), &name_required2) !=
            SIDEREON_STATUS_OK ||
        name_required2 != name_required || strcmp(name, SMOKE_B_TLE_RECORD0_NAME) != 0) {
        sidereon_tle_file_free(file);
        return fail("sidereon_tle_file_name full copy", 1);
    }

    /* The name of the second record. */
    char bare_name[8];
    size_t bare_required = 0;
    if (sidereon_tle_file_name(file, 1, bare_name, sizeof(bare_name), &bare_required) !=
            SIDEREON_STATUS_OK ||
        bare_required != sizeof(SMOKE_B_TLE_RECORD1_NAME) ||
        strcmp(bare_name, SMOKE_B_TLE_RECORD1_NAME) != 0) {
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
    int64_t epoch_unix_us = SMOKE_B_TLE_LOOK_UNIX_US;
    SidereonLookAngles *look = NULL;
    if (sidereon_tle_look_angles(tle, &station, &epoch_unix_us, 1, &look) != SIDEREON_STATUS_OK ||
        look == NULL) {
        sidereon_tle_free(tle);
        return fail("sidereon_tle_look_angles on parsed satellite", 1);
    }

    size_t look_count = 0;
    if (sidereon_look_angles_epoch_count(look, &look_count) != SIDEREON_STATUS_OK ||
        look_count != SMOKE_B_TLE_LOOK_COUNT) {
        sidereon_look_angles_free(look);
        sidereon_tle_free(tle);
        return fail("sidereon_look_angles_epoch_count on parsed satellite", 1);
    }

    SidereonLookAngle angle = {0.0, 0.0, 0.0};
    size_t written = 0, required = 0;
    if (sidereon_look_angles_values(look, &angle, 1, &written, &required) != SIDEREON_STATUS_OK ||
        written != 1 || required != 1 ||
        f64_to_bits(angle.azimuth_deg) != SMOKE_B_TLE_LOOK_BITS[0] ||
        f64_to_bits(angle.elevation_deg) != SMOKE_B_TLE_LOOK_BITS[1] ||
        f64_to_bits(angle.range_km) != SMOKE_B_TLE_LOOK_BITS[2]) {
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
    sidereon_rinex_spp_inputs_free(NULL);
    sidereon_rinex_spp_solutions_free(NULL);
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

    int rinex_spp_status = exercise_rinex_spp_surface(argv[11], argv[8]);
    if (rinex_spp_status != 0) {
        sidereon_sp3_free(sp3);
        return rinex_spp_status;
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
    inputs.pseudorange_code = SIDEREON_PSEUDORANGE_CODE_SINGLE_FREQUENCY;

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
    if (SMOKE_B_POSITIONING_SPP_EMPTY_OK || empty_status != SIDEREON_STATUS_SOLVE ||
        empty_sol != NULL) {
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
        !geodetic_present ||
        f64_to_bits(geodetic.lat_rad) != SMOKE_B_POSITIONING_SPP_GEODETIC_BITS[0] ||
        f64_to_bits(geodetic.lon_rad) != SMOKE_B_POSITIONING_SPP_GEODETIC_BITS[1] ||
        f64_to_bits(geodetic.height_m) != SMOKE_B_POSITIONING_SPP_GEODETIC_BITS[2]) {
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
        clock_written != 0 || clock_required != SMOKE_B_POSITIONING_SPP_SYSTEM_CLOCK_COUNT) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_system_clocks size query", 1);
    }
    SidereonSppSystemClock system_clocks[4];
    clock_written = 123;
    clock_required = 123;
    if (sidereon_spp_solution_system_clocks(
            sol, system_clocks, 4, &clock_written, &clock_required) != SIDEREON_STATUS_OK ||
        clock_written != SMOKE_B_POSITIONING_SPP_SYSTEM_CLOCK_COUNT ||
        clock_required != SMOKE_B_POSITIONING_SPP_SYSTEM_CLOCK_COUNT ||
        system_clocks[0].system != SMOKE_B_POSITIONING_SPP_SYSTEM_CLOCK0_SYSTEM ||
        f64_to_bits(system_clocks[0].rx_clock_s) != SMOKE_B_POSITIONING_SPP_SYSTEM_CLOCK0_BITS) {
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
        metadata.used_count != SMOKE_B_POSITIONING_SPP_USED_COUNT || metadata.used_count != used ||
        metadata.system_count != SMOKE_B_POSITIONING_SPP_SYSTEM_COUNT ||
        metadata.converged != SMOKE_B_POSITIONING_SPP_CONVERGED ||
        metadata.iterations != SMOKE_B_POSITIONING_SPP_ITERATIONS ||
        metadata.ionosphere_applied != SMOKE_B_POSITIONING_SPP_IONO_APPLIED ||
        metadata.troposphere_applied != SMOKE_B_POSITIONING_SPP_TROPO_APPLIED ||
        metadata.outer_iterations != SMOKE_B_POSITIONING_SPP_OUTER_ITERATIONS ||
        metadata.has_final_robust_scale_m != SMOKE_B_POSITIONING_SPP_HAS_ROBUST_SCALE ||
        metadata.redundancy != SMOKE_B_POSITIONING_SPP_REDUNDANCY ||
        metadata.raim_checkable != SMOKE_B_POSITIONING_SPP_RAIM_CHECKABLE ||
        metadata.geometry_quality.tier != SMOKE_B_POSITIONING_SPP_GQ_TIER ||
        metadata.geometry_quality.redundancy != SMOKE_B_POSITIONING_SPP_GQ_REDUNDANCY ||
        metadata.geometry_quality.rank != SMOKE_B_POSITIONING_SPP_GQ_RANK ||
        f64_to_bits(metadata.geometry_quality.condition_number) !=
            SMOKE_B_POSITIONING_SPP_GQ_CONDITION_BITS ||
        f64_to_bits(metadata.geometry_quality.gdop) != SMOKE_B_POSITIONING_SPP_GQ_GDOP_BITS ||
        metadata.geometry_quality.raim_checkable != SMOKE_B_POSITIONING_SPP_GQ_RAIM_CHECKABLE ||
        metadata.geometry_quality.covariance_validated !=
            SMOKE_B_POSITIONING_SPP_GQ_COVARIANCE_VALIDATED ||
        metadata.status != SMOKE_B_POSITIONING_SPP_STATUS) {
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
        if (f64_to_bits(residuals[i]) != SMOKE_B_POSITIONING_SPP_RESIDUAL_BITS[i]) {
            sidereon_spp_solution_free(sol);
            sidereon_sp3_free(sp3);
            return fail("sidereon_spp_solution_residuals values", 1);
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
    if (have_dop != SMOKE_B_POSITIONING_SPP_HAS_DOP) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_dop presence", 1);
    }
#if SMOKE_B_POSITIONING_SPP_HAS_DOP
    if (f64_to_bits(dop.gdop) != SMOKE_B_POSITIONING_SPP_DOP_BITS[0] ||
        f64_to_bits(dop.pdop) != SMOKE_B_POSITIONING_SPP_DOP_BITS[1] ||
        f64_to_bits(dop.hdop) != SMOKE_B_POSITIONING_SPP_DOP_BITS[2] ||
        f64_to_bits(dop.vdop) != SMOKE_B_POSITIONING_SPP_DOP_BITS[3] ||
        f64_to_bits(dop.tdop) != SMOKE_B_POSITIONING_SPP_DOP_BITS[4]) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_spp_solution_dop values", 1);
    }
#endif

    SidereonObservation observations_with_reject[SPP_OBS_COUNT + 1];
    for (size_t i = 0; i < SPP_OBS_COUNT; i++) {
        observations_with_reject[i] = observations[i];
    }
    /* A QZSS observation J01 appended; the rejected list, its last entry and
     * the reason are sidereon-core's (tests/valgen bin smoke_b_positioning). */
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
    SidereonSppRejectedSat rejected[SMOKE_B_POSITIONING_SPP_J01_REJECTED_COUNT];
    rejected_written = 123;
    rejected_required = 123;
    if (sidereon_spp_solution_rejected_sats(
            reject_sol, rejected, SMOKE_B_POSITIONING_SPP_J01_REJECTED_COUNT, &rejected_written,
            &rejected_required) !=
            SIDEREON_STATUS_OK ||
        rejected_written != SMOKE_B_POSITIONING_SPP_J01_REJECTED_COUNT ||
        rejected_required != SMOKE_B_POSITIONING_SPP_J01_REJECTED_COUNT ||
        !token_equals(&rejected[SMOKE_B_POSITIONING_SPP_J01_REJECTED_COUNT - 1].sat_id,
                      SMOKE_B_POSITIONING_SPP_J01_LAST_REJECTED_ID) ||
        rejected[SMOKE_B_POSITIONING_SPP_J01_REJECTED_COUNT - 1].reason !=
            SMOKE_B_POSITIONING_SPP_J01_LAST_REJECTED_REASON) {
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
    /* Robust reweighting at max_outer 2; the reported iterations and scale are
     * sidereon-core's. */
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
        v2_metadata.outer_iterations != SMOKE_B_POSITIONING_SPP_V2_OUTER_ITERATIONS ||
        v2_metadata.has_final_robust_scale_m != SMOKE_B_POSITIONING_SPP_V2_HAS_ROBUST_SCALE ||
        f64_to_bits(v2_metadata.final_robust_scale_m) !=
            SMOKE_B_POSITIONING_SPP_V2_ROBUST_SCALE_BITS) {
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
    if (SMOKE_B_POSITIONING_SPP_V2_STRICT_OK ||
        sidereon_solve_spp_v2(sp3, &strict_v2_inputs, &strict_v2_sol) !=
            SIDEREON_STATUS_SOLVE ||
        strict_v2_sol != NULL) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("sidereon_solve_spp_v2 policy failure clears out_solution", 1);
    }

    /* A real public static-positioning refusal with a nested, payload-bearing
     * core SPP cause. Starting from this valid SPP request and setting one
     * pseudorange to zero reaches core input validation for epoch zero. */
    SidereonStaticPositionEpoch static_epoch;
    memset(&static_epoch, 0, sizeof static_epoch);
    if (sidereon_spp_inputs_v2_init(&static_epoch.inputs) != SIDEREON_STATUS_OK) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("static-positioning nested SPP input initialization", 1);
    }
    static_epoch.inputs.base = inputs;
    SidereonObservation static_bad_observations[SPP_OBS_COUNT];
    memcpy(static_bad_observations, observations, sizeof static_bad_observations);
    static_bad_observations[0].pseudorange_m = 0.0;
    static_epoch.inputs.base.observations = static_bad_observations;
    static_epoch.inputs.base.observation_count = SPP_OBS_COUNT;
    SidereonStaticPositionErrorKind static_error = SIDEREON_STATIC_POSITION_ERROR_KIND_NONE;
    SidereonStaticPositionSolution *static_solution = (SidereonStaticPositionSolution *)(uintptr_t)1;
    const SidereonStatus static_status = sidereon_solve_static_position_sp3(
        sp3, &static_epoch, 1, NULL, &static_error, &static_solution);
    SidereonEngineErrorInfo static_info;
    memset(&static_info, 0, sizeof static_info);
    size_t static_payload_written = 0;
    size_t static_payload_required = 0;
    const SidereonStatus static_info_status = sidereon_last_engine_error_info(&static_info);
    const SidereonStatus static_payload_query_status = sidereon_last_engine_error_payload(
        NULL, 0, &static_payload_written, &static_payload_required);
    if (static_status != SIDEREON_STATUS_INVALID_ARGUMENT ||
        static_error != SIDEREON_STATIC_POSITION_ERROR_KIND_EPOCH_INPUT || static_solution != NULL ||
        static_info_status != SIDEREON_STATUS_OK ||
        static_info.family != SIDEREON_ENGINE_ERROR_FAMILY_STATIC_POSITIONING ||
        static_payload_query_status != SIDEREON_STATUS_OK || static_payload_written != 0 ||
        static_payload_required != static_info.payload_len) {
        fprintf(stderr,
                "static-positioning diagnostic: status=%d error_kind=%d solution=%p "
                "info_status=%d family=%d payload_len=%zu query_status=%d "
                "written=%zu required=%zu\n",
                (int)static_status, (int)static_error, (void *)static_solution,
                (int)static_info_status, (int)static_info.family, static_info.payload_len,
                (int)static_payload_query_status, static_payload_written,
                static_payload_required);
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("static-positioning nested SPP refusal diagnostics", 1);
    }
    uint8_t *static_payload = (uint8_t *)malloc(static_payload_required + 1);
    if (static_payload == NULL ||
        sidereon_last_engine_error_payload(static_payload, static_payload_required,
                                           &static_payload_written,
                                           &static_payload_required) != SIDEREON_STATUS_OK ||
        static_payload_written != static_payload_required) {
        free(static_payload);
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("static-positioning nested SPP payload copy", 1);
    }
    static_payload[static_payload_written] = 0;
    if (strstr((const char *)static_payload, "\"schema_version\":1") == NULL ||
        strstr((const char *)static_payload, "\"family\":\"static_positioning\"") == NULL ||
        strstr((const char *)static_payload,
               "\"operation\":\"sidereon_solve_static_position_sp3\"") == NULL ||
        strstr((const char *)static_payload, "\"kind\":\"epoch_input\"") == NULL ||
        strstr((const char *)static_payload, "\"epoch_index\":0") == NULL ||
        strstr((const char *)static_payload, "\"kind\":\"invalid_input\"") == NULL ||
        strstr((const char *)static_payload, "\"field\":\"observation.pseudorange_m\"") == NULL ||
        strstr((const char *)static_payload, "\"kind\":\"not_positive\"") == NULL) {
        free(static_payload);
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("static-positioning complete nested SPP payload", 1);
    }
    free(static_payload);
    char static_message[512] = {0};
    (void)sidereon_last_error_message(static_message, sizeof static_message);
    if (strcmp(static_message,
               "sidereon_solve_static_position_sp3: invalid static epoch 0: "
               "invalid SPP input observation.pseudorange_m: not positive") != 0) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return fail("static-positioning exact legacy error message", 1);
    }

    printf("position = [%.6f, %.6f, %.6f] m\n", position[0], position[1], position[2]);
    printf("rx_clock_s = %.9e\n", rx_clock_s);
    printf("used_sats = %zu\n", used);
    if (have_dop) {
        printf("gdop = %.4f, pdop = %.4f, hdop = %.4f, vdop = %.4f, tdop = %.4f\n",
               dop.gdop, dop.pdop, dop.hdop, dop.vdop, dop.tdop);
    }

    /* tests/sppgen solves the same inputs through the same engine path, so
     * the position and clock bits agree exactly. */
    int position_exact = 1;
    for (int i = 0; i < 3; i++) {
        if (f64_to_bits(position[i]) != SPP_EXPECTED_X_BITS[i]) {
            position_exact = 0;
            fprintf(stderr, "position[%d] = %016llx, expected %016llx\n", i,
                    (unsigned long long)f64_to_bits(position[i]),
                    (unsigned long long)SPP_EXPECTED_X_BITS[i]);
        }
    }
    int clock_exact = f64_to_bits(rx_clock_s) == SPP_EXPECTED_RX_CLOCK_S_BITS;

    int glonass_status = exercise_spp_glonass_channels(sp3);
    if (glonass_status != 0) {
        sidereon_spp_solution_free(sol);
        sidereon_sp3_free(sp3);
        return glonass_status;
    }

    sidereon_spp_solution_free(sol);
    sidereon_sp3_free(sp3);

    if (!position_exact) {
        fprintf(stderr, "FAIL: position differs from the engine's solve\n");
        return 1;
    }
    if (!clock_exact) {
        fprintf(stderr, "FAIL: clock %016llx differs from the engine's solve %016llx\n",
                (unsigned long long)f64_to_bits(rx_clock_s),
                (unsigned long long)SPP_EXPECTED_RX_CLOCK_S_BITS);
        return 1;
    }

    printf("OK: binding reproduces the crate SPP reference\n");
    return 0;
}
