/*
 * Focused C smoke for the sample-backed precise-ephemeris source and the batch
 * range predictor. Each exercise delegates to the engine; this program only
 * marshals inputs and asserts the surfaced structure:
 *
 *   1. Extract a loaded SP3 product's canonical precise-ephemeris samples
 *      (sidereon_sp3_precise_ephemeris_samples), using the variable-length
 *      query/copy contract.
 *   2. Rebuild an interpolatable source from those samples
 *      (sidereon_precise_ephemeris_samples_from_samples) and assert the batch
 *      range predictor (sidereon_sp3_predict_ranges vs
 *      sidereon_precise_ephemeris_samples_predict_ranges) and the interpolated
 *      satellite states agree with the SP3-parsed source within the documented
 *      round-trip tolerance (the km -> meters map is not injective, so a
 *      meters-carrying sample reconstructs within <= 1 ULP of the fit node, far
 *      below any physical threshold; see the core module docs).
 *   3. Assert the one-call batch is bit-identical to per-request length-1 calls.
 *   4. Assert the validation-error paths (no samples; a single-sample satellite)
 *      return SIDEREON_STATUS_INVALID_ARGUMENT and leave no handle.
 *
 * Build/run is driven by tests/run_smoke.sh, which passes the GRG SP3 as argv.
 */
#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sidereon.h"

static int fail(const char *what, int code) {
    char message[512];
    size_t written = sidereon_last_error_message(message, sizeof(message));
    if (written > 0) {
        fprintf(stderr, "FAIL: %s: %s\n", what, message);
    } else {
        fprintf(stderr, "FAIL: %s\n", what);
    }
    return code;
}

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
    uint8_t *buf = malloc((size_t)size);
    if (buf == NULL) {
        fclose(f);
        return NULL;
    }
    if (fread(buf, 1, (size_t)size, f) != (size_t)size) {
        free(buf);
        fclose(f);
        return NULL;
    }
    fclose(f);
    *out_len = (size_t)size;
    return buf;
}

static uint64_t f64_to_bits(double value) {
    uint64_t bits;
    memcpy(&bits, &value, sizeof(bits));
    return bits;
}

/* Documented round-trip tolerances: sub-micron on position/range (the km ->
 * meters reconstruction is within <= 1 ULP of the fit node), and correspondingly
 * tight on the light-time-derived transmit time and the microsecond-scaled
 * clock. */
#define POS_TOL_M 1.0e-6
#define RANGE_TOL_M 1.0e-6
#define TIME_TOL_S 1.0e-6
#define CLOCK_TOL_S 1.0e-9

static bool prediction_matches(const SidereonRangePrediction *a, const SidereonRangePrediction *b) {
    if (fabs(a->geometric_range_m - b->geometric_range_m) > RANGE_TOL_M) {
        return false;
    }
    if (fabs(a->transmit_time_j2000_s - b->transmit_time_j2000_s) > TIME_TOL_S) {
        return false;
    }
    if (a->has_sat_clock_s != b->has_sat_clock_s) {
        return false;
    }
    if (a->has_sat_clock_s && fabs(a->sat_clock_s - b->sat_clock_s) > CLOCK_TOL_S) {
        return false;
    }
    for (int k = 0; k < 3; k++) {
        if (fabs(a->sat_pos_ecef_m[k] - b->sat_pos_ecef_m[k]) > POS_TOL_M) {
            return false;
        }
    }
    return true;
}

static bool prediction_bit_identical(const SidereonRangePrediction *a,
                                     const SidereonRangePrediction *b) {
    if (f64_to_bits(a->geometric_range_m) != f64_to_bits(b->geometric_range_m)) {
        return false;
    }
    if (f64_to_bits(a->transmit_time_j2000_s) != f64_to_bits(b->transmit_time_j2000_s)) {
        return false;
    }
    if (a->has_sat_clock_s != b->has_sat_clock_s) {
        return false;
    }
    if (a->has_sat_clock_s && f64_to_bits(a->sat_clock_s) != f64_to_bits(b->sat_clock_s)) {
        return false;
    }
    for (int k = 0; k < 3; k++) {
        if (f64_to_bits(a->sat_pos_ecef_m[k]) != f64_to_bits(b->sat_pos_ecef_m[k])) {
            return false;
        }
    }
    return true;
}

#define REQUEST_COUNT 3

int main(int argc, char **argv) {
    if (argc < 2) {
        fprintf(stderr, "usage: %s <grg_sp3>\n", argv[0]);
        return 2;
    }
    const char *sp3_path = argv[1];

    size_t sp3_len = 0;
    uint8_t *sp3_bytes = read_file(sp3_path, &sp3_len);
    if (sp3_bytes == NULL) {
        fprintf(stderr, "FAIL: could not read SP3 fixture %s\n", sp3_path);
        return 1;
    }

    SidereonSp3 *sp3 = NULL;
    if (sidereon_sp3_load(sp3_bytes, sp3_len, &sp3) != SIDEREON_STATUS_OK) {
        free(sp3_bytes);
        return fail("sidereon_sp3_load", 1);
    }
    free(sp3_bytes);

    int rc = 1;
    SidereonPreciseEphemerisSample *samples = NULL;
    SidereonPreciseEphemerisSamples *source = NULL;

    /* (1) Extract the canonical samples: query the count, then fill. */
    size_t written = 0;
    size_t required = 0;
    if (sidereon_sp3_precise_ephemeris_samples(sp3, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != 0) {
        rc = fail("precise samples: count query", 1);
        goto cleanup;
    }
    if (required < 8) {
        rc = fail("precise samples: too few samples extracted", 1);
        goto cleanup;
    }
    samples = calloc(required, sizeof(*samples));
    if (samples == NULL) {
        rc = fail("precise samples: alloc", 1);
        goto cleanup;
    }
    if (sidereon_sp3_precise_ephemeris_samples(sp3, samples, required, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != required) {
        rc = fail("precise samples: fill", 1);
        goto cleanup;
    }

    /* (2) Rebuild a sample-backed source from the extracted samples. */
    if (sidereon_precise_ephemeris_samples_from_samples(samples, required, &source) !=
            SIDEREON_STATUS_OK ||
        source == NULL) {
        rc = fail("precise samples: from_samples", 1);
        goto cleanup;
    }

    /* Pick an interior sample so the satellite has coverage at and around its
     * epoch; predict at the node and at off-node epochs to exercise the
     * interpolation, not just the node values. */
    size_t mid = required / 2;
    const char *sat_id = samples[mid].sat.bytes;
    double t0 = samples[mid].epoch_j2000_s;
    double t_rx[REQUEST_COUNT] = {t0 - 300.0, t0, t0 + 300.0};
    double receiver[3] = {4027894.0, 307046.0, 4919474.0};

    SidereonRangePredictionRequest requests[REQUEST_COUNT];
    for (int i = 0; i < REQUEST_COUNT; i++) {
        requests[i].sat_id = sat_id;
        requests[i].receiver_ecef_m[0] = receiver[0];
        requests[i].receiver_ecef_m[1] = receiver[1];
        requests[i].receiver_ecef_m[2] = receiver[2];
        requests[i].t_rx_j2000_s = t_rx[i];
    }

    SidereonRangePrediction from_sp3[REQUEST_COUNT];
    SidereonRangePrediction from_samples[REQUEST_COUNT];
    if (sidereon_sp3_predict_ranges(sp3, requests, REQUEST_COUNT, NULL, from_sp3) !=
        SIDEREON_STATUS_OK) {
        rc = fail("predict_ranges: SP3 source", 1);
        goto cleanup;
    }
    if (sidereon_precise_ephemeris_samples_predict_ranges(source, requests, REQUEST_COUNT, NULL,
                                                          from_samples) != SIDEREON_STATUS_OK) {
        rc = fail("predict_ranges: samples source", 1);
        goto cleanup;
    }
    for (int i = 0; i < REQUEST_COUNT; i++) {
        if (!(from_sp3[i].geometric_range_m > 0.0)) {
            rc = fail("predict_ranges: non-positive range", 1);
            goto cleanup;
        }
        /* The sample-backed source and the SP3-parsed source must agree on both
         * the predicted range and the interpolated satellite state, within the
         * documented round-trip tolerance. */
        if (!prediction_matches(&from_sp3[i], &from_samples[i])) {
            rc = fail("predict_ranges: SP3 vs samples disagree beyond tolerance", 1);
            goto cleanup;
        }
    }

    /* (3) The one-call batch must be bit-identical to per-request length-1 calls
     * on the same source. */
    for (int i = 0; i < REQUEST_COUNT; i++) {
        SidereonRangePrediction single;
        if (sidereon_precise_ephemeris_samples_predict_ranges(source, &requests[i], 1, NULL,
                                                              &single) != SIDEREON_STATUS_OK) {
            rc = fail("predict_ranges: per-request call", 1);
            goto cleanup;
        }
        if (!prediction_bit_identical(&single, &from_samples[i])) {
            rc = fail("predict_ranges: batch != per-request", 1);
            goto cleanup;
        }
    }

    /* (4a) Validation error: no samples returns InvalidArgument, no handle. */
    SidereonPreciseEphemerisSamples *empty_handle = (SidereonPreciseEphemerisSamples *)0x1;
    if (sidereon_precise_ephemeris_samples_from_samples(NULL, 0, &empty_handle) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        empty_handle != NULL) {
        rc = fail("validation: empty sample set not rejected", 1);
        goto cleanup;
    }

    /* (4b) Validation error: a lone sample for a satellite cannot be
     * interpolated (needs at least two), so from_samples rejects it. */
    SidereonPreciseEphemerisSample lone = samples[mid];
    lone.clock_event = false;
    SidereonPreciseEphemerisSamples *lone_handle = (SidereonPreciseEphemerisSamples *)0x1;
    if (sidereon_precise_ephemeris_samples_from_samples(&lone, 1, &lone_handle) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        lone_handle != NULL) {
        rc = fail("validation: single-sample satellite not rejected", 1);
        goto cleanup;
    }

    /* (4c) Validation error: an absurd epoch (finite but far outside the i64
     * whole-second range) must be rejected with InvalidArgument, not silently
     * clamped by a saturating cast. Two samples so only the epoch is at fault. */
    SidereonPreciseEphemerisSample absurd[2] = {samples[mid], samples[mid]};
    absurd[0].epoch_j2000_s = 1.0e300;
    absurd[1].epoch_j2000_s = 1.0e300 + 900.0;
    SidereonPreciseEphemerisSamples *absurd_handle = (SidereonPreciseEphemerisSamples *)0x1;
    if (sidereon_precise_ephemeris_samples_from_samples(absurd, 2, &absurd_handle) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        absurd_handle != NULL) {
        rc = fail("validation: out-of-range epoch not rejected", 1);
        goto cleanup;
    }

    printf("precise_samples_smoke: OK (%zu samples, %d requests)\n", required, REQUEST_COUNT);
    rc = 0;

cleanup:
    sidereon_precise_ephemeris_samples_free(source);
    free(samples);
    sidereon_sp3_free(sp3);
    return rc;
}
