/*
 * Focused smoke for the 0.13 C binding additions:
 *
 *   1. Batched observable states and cached precise-ephemeris interpolants.
 *   2. Estimation and detection primitives.
 *   3. ToA/TDOA source localization, DOP, and CRLB.
 */
#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sidereon.h"

static int fail(const char *what) {
    char message[512];
    size_t written = sidereon_last_error_message(message, sizeof(message));
    if (written > 0) {
        fprintf(stderr, "FAIL: %s: %s\n", what, message);
    } else {
        fprintf(stderr, "FAIL: %s\n", what);
    }
    return 1;
}

static int require_ok(SidereonStatus status, const char *what) {
    if (status != SIDEREON_STATUS_OK) {
        return fail(what);
    }
    return 0;
}

static bool last_error_contains(const char *needle) {
    char message[512];
    size_t written = sidereon_last_error_message(message, sizeof(message));
    return written > 0 && strstr(message, needle) != NULL;
}

static bool close_abs(double actual, double expected, double tol) {
    return fabs(actual - expected) <= tol;
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

static bool state_rows_match(const double *a_pos, const double *b_pos, bool a_has_clock,
                             bool b_has_clock, double a_clock, double b_clock, double pos_tol_m,
                             double clock_tol_s) {
    for (int axis = 0; axis < 3; axis++) {
        if (!close_abs(a_pos[axis], b_pos[axis], pos_tol_m)) {
            return false;
        }
    }
    if (a_has_clock != b_has_clock) {
        return false;
    }
    if (a_has_clock && !close_abs(a_clock, b_clock, clock_tol_s)) {
        return false;
    }
    return true;
}

static int exercise_observable_states(const char *sp3_path) {
    enum { COUNT = 3, SHARED_COUNT = 2 };
    int rc = 1;
    size_t sp3_len = 0;
    uint8_t *sp3_bytes = read_file(sp3_path, &sp3_len);
    SidereonSp3 *sp3 = NULL;
    SidereonPreciseEphemerisSample *samples = NULL;
    SidereonPreciseEphemerisSamples *sample_source = NULL;
    SidereonPreciseEphemerisInterpolant *interp_sp3 = NULL;
    SidereonPreciseEphemerisInterpolant *interp_samples = NULL;
    SidereonPreciseEphemerisInterpolant *interp_source = NULL;

    if (sp3_bytes == NULL) {
        fprintf(stderr, "FAIL: could not read SP3 fixture %s\n", sp3_path);
        return 1;
    }
    if (require_ok(sidereon_sp3_load(sp3_bytes, sp3_len, &sp3), "sp3 load") != 0) {
        goto cleanup;
    }
    free(sp3_bytes);
    sp3_bytes = NULL;

    size_t written = 0;
    size_t required = 0;
    if (require_ok(sidereon_sp3_precise_ephemeris_samples(sp3, NULL, 0, &written, &required),
                   "precise sample count") != 0) {
        goto cleanup;
    }
    if (required < 8) {
        rc = fail("observable states: insufficient SP3 samples");
        goto cleanup;
    }
    samples = calloc(required, sizeof(*samples));
    if (samples == NULL) {
        rc = fail("observable states: sample allocation");
        goto cleanup;
    }
    if (require_ok(sidereon_sp3_precise_ephemeris_samples(sp3, samples, required, &written,
                                                          &required),
                   "precise sample fill") != 0) {
        goto cleanup;
    }
    if (require_ok(sidereon_precise_ephemeris_samples_from_samples(samples, required,
                                                                   &sample_source),
                   "sample source build") != 0) {
        goto cleanup;
    }
    if (require_ok(sidereon_precise_ephemeris_interpolant_from_sp3(sp3, &interp_sp3),
                   "interpolant from sp3") != 0 ||
        require_ok(sidereon_precise_ephemeris_interpolant_from_samples(samples, required,
                                                                       &interp_samples),
                   "interpolant from samples") != 0 ||
        require_ok(sidereon_precise_ephemeris_interpolant_from_precise_ephemeris_samples(
                       sample_source, &interp_source),
                   "interpolant from sample source") != 0) {
        goto cleanup;
    }

    size_t mid = required / 2;
    const char *sat_id = (const char *)samples[mid].sat.bytes;
    double t0 = samples[mid].epoch_j2000_s;
    const char *satellites[COUNT] = {sat_id, sat_id, sat_id};
    double epochs[COUNT] = {t0 - 300.0, t0, INFINITY};
    double sp3_pos[COUNT * 3];
    double interp_pos[COUNT * 3];
    double sample_pos[COUNT * 3];
    double source_pos[COUNT * 3];
    double sp3_clock[COUNT];
    double interp_clock[COUNT];
    double sample_clock[COUNT];
    double source_clock[COUNT];
    bool sp3_has_clock[COUNT];
    bool interp_has_clock[COUNT];
    bool sample_has_clock[COUNT];
    bool source_has_clock[COUNT];
    SidereonObservableStateElementStatus sp3_status[COUNT];
    SidereonObservableStateElementStatus interp_status[COUNT];
    SidereonObservableStateElementStatus sample_status[COUNT];
    SidereonObservableStateElementStatus source_status[COUNT];
    SidereonStatus sp3_result[COUNT];
    SidereonStatus interp_result[COUNT];
    SidereonStatus sample_result[COUNT];
    SidereonStatus source_result[COUNT];

    if (require_ok(sidereon_sp3_observable_states_at_j2000_s(
                       sp3, satellites, epochs, COUNT, sp3_pos, sp3_clock, sp3_has_clock,
                       sp3_status, sp3_result),
                   "sp3 observable states") != 0 ||
        require_ok(sidereon_precise_ephemeris_interpolant_observable_states_at_j2000_s(
                       interp_sp3, satellites, epochs, COUNT, interp_pos, interp_clock,
                       interp_has_clock, interp_status, interp_result),
                   "interpolant observable states") != 0 ||
        require_ok(sidereon_precise_ephemeris_interpolant_observable_states_at_j2000_s(
                       interp_samples, satellites, epochs, COUNT, sample_pos, sample_clock,
                       sample_has_clock, sample_status, sample_result),
                   "sample interpolant observable states") != 0 ||
        require_ok(sidereon_precise_ephemeris_samples_observable_states_at_j2000_s(
                       sample_source, satellites, epochs, COUNT, source_pos, source_clock,
                       source_has_clock, source_status, source_result),
                   "sample source observable states") != 0) {
        goto cleanup;
    }

    for (size_t i = 0; i < 2; i++) {
        if (sp3_status[i] != SIDEREON_OBSERVABLE_STATE_ELEMENT_STATUS_VALID ||
            interp_status[i] != SIDEREON_OBSERVABLE_STATE_ELEMENT_STATUS_VALID ||
            sample_status[i] != SIDEREON_OBSERVABLE_STATE_ELEMENT_STATUS_VALID ||
            source_status[i] != SIDEREON_OBSERVABLE_STATE_ELEMENT_STATUS_VALID ||
            sp3_result[i] != SIDEREON_STATUS_OK || interp_result[i] != SIDEREON_STATUS_OK ||
            sample_result[i] != SIDEREON_STATUS_OK || source_result[i] != SIDEREON_STATUS_OK) {
            rc = fail("observable states: valid row status");
            goto cleanup;
        }
        if (!state_rows_match(&sp3_pos[i * 3], &interp_pos[i * 3], sp3_has_clock[i],
                              interp_has_clock[i], sp3_clock[i], interp_clock[i], 1.0e-9,
                              1.0e-12) ||
            !state_rows_match(&sp3_pos[i * 3], &sample_pos[i * 3], sp3_has_clock[i],
                              sample_has_clock[i], sp3_clock[i], sample_clock[i], 1.0e-6,
                              1.0e-9) ||
            !state_rows_match(&sample_pos[i * 3], &source_pos[i * 3], sample_has_clock[i],
                              source_has_clock[i], sample_clock[i], source_clock[i], 0.0, 0.0)) {
            rc = fail("observable states: source agreement");
            goto cleanup;
        }
    }
    if (sp3_status[2] != SIDEREON_OBSERVABLE_STATE_ELEMENT_STATUS_ERROR ||
        sp3_result[2] != SIDEREON_STATUS_INVALID_ARGUMENT || !isnan(sp3_pos[6]) ||
        !isnan(sp3_pos[7]) || !isnan(sp3_pos[8]) || sp3_has_clock[2]) {
        rc = fail("observable states: invalid element sentinel");
        goto cleanup;
    }

    const char *shared_sats[SHARED_COUNT] = {sat_id, sat_id};
    double shared_pos[SHARED_COUNT * 3];
    double shared_clock[SHARED_COUNT];
    bool shared_has_clock[SHARED_COUNT];
    SidereonObservableStateElementStatus shared_status[SHARED_COUNT];
    SidereonStatus shared_result[SHARED_COUNT];
    if (require_ok(sidereon_sp3_observable_states_at_shared_j2000_s(
                       sp3, shared_sats, SHARED_COUNT, t0, shared_pos, shared_clock,
                       shared_has_clock, shared_status, shared_result),
                   "shared observable states") != 0) {
        goto cleanup;
    }
    for (size_t i = 0; i < SHARED_COUNT; i++) {
        if (shared_status[i] != SIDEREON_OBSERVABLE_STATE_ELEMENT_STATUS_VALID ||
            shared_result[i] != SIDEREON_STATUS_OK ||
            !state_rows_match(&sp3_pos[3], &shared_pos[i * 3], sp3_has_clock[1],
                              shared_has_clock[i], sp3_clock[1], shared_clock[i], 0.0, 0.0)) {
            rc = fail("observable states: shared epoch agreement");
            goto cleanup;
        }
    }

    double missing[3];
    if (require_ok(sidereon_observable_state_missing_position_ecef_m(missing, 3),
                   "missing sentinel") != 0 ||
        !isnan(missing[0]) || !isnan(missing[1]) || !isnan(missing[2])) {
        rc = fail("observable states: missing sentinel accessor");
        goto cleanup;
    }

    printf("cap013_observable_states_smoke: OK (%zu samples)\n", required);
    rc = 0;

cleanup:
    sidereon_precise_ephemeris_interpolant_free(interp_source);
    sidereon_precise_ephemeris_interpolant_free(interp_samples);
    sidereon_precise_ephemeris_interpolant_free(interp_sp3);
    sidereon_precise_ephemeris_samples_free(sample_source);
    free(samples);
    sidereon_sp3_free(sp3);
    free(sp3_bytes);
    return rc;
}

static int exercise_estimation_primitives(void) {
    SidereonAlphaBetaGains gains;
    SidereonScalarKalmanGains kalman;
    if (require_ok(sidereon_alpha_beta_steady_state_gains(0.4, &gains),
                   "alpha-beta gains") != 0 ||
        require_ok(sidereon_kalman_cv_steady_state_gains(0.4, 2.0, 9.0, &kalman),
                   "kalman gains") != 0) {
        return 1;
    }
    if (!close_abs(kalman.position_gain, gains.alpha, 1.0e-9) ||
        !close_abs(kalman.rate_gain * 2.0, gains.beta, 1.0e-9)) {
        return fail("estimation primitives: gain relation");
    }

    SidereonAlphaBetaState state = {10.0, 1.0};
    SidereonAlphaBetaStep step;
    if (require_ok(sidereon_alpha_beta_filter_step(&state, 14.0, 2.0, &gains, &step),
                   "alpha-beta step") != 0) {
        return 1;
    }
    if (!close_abs(step.predicted.level, 12.0, 0.0) ||
        !close_abs(step.predicted.rate, 1.0, 0.0) ||
        !close_abs(step.innovation, 2.0, 0.0) ||
        !close_abs(step.updated.level, 12.0 + gains.alpha * 2.0, 1.0e-15) ||
        !close_abs(step.updated.rate, 1.0 + gains.beta, 1.0e-15)) {
        return fail("estimation primitives: alpha-beta equations");
    }

    double value = 0.0;
    if (require_ok(sidereon_normalized_innovation(2.0, 4.0, &value),
                   "normalized innovation") != 0 ||
        !close_abs(value, 1.0, 0.0) ||
        require_ok(sidereon_nis(2.0, 4.0, &value), "nis") != 0 ||
        !close_abs(value, 1.0, 0.0) ||
        require_ok(sidereon_nis_expected_value(3, &value), "nis expected") != 0 ||
        !close_abs(value, 3.0, 0.0)) {
        return fail("estimation primitives: innovation values");
    }

    double threshold = 0.0;
    SidereonNisGate gate;
    if (require_ok(sidereon_nis_gate_threshold(1, 0.95, &threshold), "nis threshold") != 0 ||
        threshold < 3.84 || threshold > 3.85 ||
        require_ok(sidereon_nis_gate_test(2.0, 4.0, 1, 0.95, &gate), "nis gate") != 0 ||
        !gate.in_gate || !close_abs(gate.nis, 1.0, 0.0) ||
        !close_abs(gate.threshold, threshold, 0.0) || gate.dof != 1) {
        return fail("estimation primitives: nis gate");
    }

    double mad_constant = 0.0;
    double values[3] = {1.0, 2.0, 100.0};
    if (require_ok(sidereon_mad_gaussian_consistency(&mad_constant), "mad constant") != 0 ||
        !close_abs(mad_constant, 1.482602218505602, 1.0e-15) ||
        require_ok(sidereon_mad_spread(values, 3, 0.0, &value), "mad spread") != 0 ||
        !close_abs(value, mad_constant, 1.0e-15)) {
        return fail("estimation primitives: mad");
    }

    if (require_ok(sidereon_ewma_update(10.0, 14.0, 0.25, &value), "ewma") != 0 ||
        !close_abs(value, 11.0, 0.0) ||
        require_ok(sidereon_ewma_update_power_of_two(10.0, 14.0, 2, &value),
                   "ewma power of two") != 0 ||
        !close_abs(value, 11.0, 0.0)) {
        return fail("estimation primitives: ewma");
    }

    double multiplier = 0.0;
    double pfa = 0.0;
    double cfar_threshold = 0.0;
    if (require_ok(sidereon_cfar_ca_multiplier_from_pfa(16, 1.0e-3, &multiplier),
                   "cfar multiplier") != 0 ||
        require_ok(sidereon_cfar_ca_pfa_from_multiplier(16, multiplier, &pfa),
                   "cfar pfa") != 0 ||
        !close_abs(pfa, 1.0e-3, 1.0e-15) ||
        require_ok(sidereon_cfar_ca_threshold(16, 1.0e-3, 2.5, &cfar_threshold),
                   "cfar threshold") != 0 ||
        !close_abs(cfar_threshold, multiplier * 2.5, 1.0e-12) ||
        require_ok(sidereon_cfar_ca_false_alarm_probability(16, cfar_threshold, 2.5, &pfa),
                   "cfar false alarm") != 0 ||
        !close_abs(pfa, 1.0e-3, 1.0e-15)) {
        return fail("estimation primitives: cfar");
    }

    printf("cap013_estimation_primitives_smoke: OK\n");
    return 0;
}

static double sensor_speed(const SidereonSourceSensor *sensor, double default_speed) {
    return sensor->has_propagation_speed_m_s ? sensor->propagation_speed_m_s : default_speed;
}

static double distance_to_source(const SidereonSourceSensor *sensor, const double *source) {
    double sum = 0.0;
    for (size_t axis = 0; axis < sensor->dimension; axis++) {
        double delta = source[axis] - sensor->position_m[axis];
        sum += delta * delta;
    }
    return sqrt(sum);
}

static void fill_arrivals(const SidereonSourceSensor *sensors, size_t count, const double *source,
                          double origin, double default_speed, double *out_times) {
    for (size_t i = 0; i < count; i++) {
        out_times[i] = origin + distance_to_source(&sensors[i], source) /
                                    sensor_speed(&sensors[i], default_speed);
    }
}

static bool vec_close3(const double *actual, const double *expected, size_t dimension, double tol) {
    for (size_t axis = 0; axis < dimension; axis++) {
        if (!close_abs(actual[axis], expected[axis], tol)) {
            return false;
        }
    }
    return true;
}

static int exercise_source_localization(void) {
    SidereonSourceSensor toa_sensors[5] = {
        {3, {0.0, 0.0, 0.0}, false, 0.0},
        {3, {2.0, 0.0, 0.0}, false, 0.0},
        {3, {0.0, 2.0, 0.0}, false, 0.0},
        {3, {0.0, 0.0, 2.0}, false, 0.0},
        {3, {2.0, 2.0, 2.0}, false, 0.0},
    };
    double toa_source[3] = {0.4, 0.6, 0.5};
    double toa_times[5];
    fill_arrivals(toa_sensors, 5, toa_source, 1.25, 1.0, toa_times);

    SidereonSourceInitialGuess guess;
    if (require_ok(sidereon_chan_ho_initial_guess(toa_sensors, 5, toa_times, 1.0,
                                                  SIDEREON_SOURCE_SOLVE_MODE_TOA, 0, &guess),
                   "chan-ho toa") != 0) {
        return 1;
    }
    if (guess.dimension != 3 || !guess.has_origin_time_s ||
        !vec_close3(guess.position_m, toa_source, 3, 1.0e-7) ||
        !close_abs(guess.origin_time_s, 1.25, 1.0e-10) || guess.residual_rms_s > 1.0e-10) {
        return fail("source localization: toa seed");
    }

    SidereonSourceLocateOptions options;
    if (require_ok(sidereon_source_locate_options_init(&options), "source options init") != 0) {
        return 1;
    }
    options.timing_sigma_s = 0.001;

    SidereonSourceSolution *solution = NULL;
    if (require_ok(sidereon_locate_source(toa_sensors, 5, toa_times, 1.0, &options, &solution),
                   "locate source toa") != 0) {
        return 1;
    }
    SidereonSourceSolutionSummary summary;
    if (require_ok(sidereon_source_solution_summary(solution, &summary), "source summary") != 0) {
        sidereon_source_solution_free(solution);
        return 1;
    }
    if (summary.dimension != 3 || !summary.has_origin_time_s || !summary.has_covariance ||
        summary.residual_count != 5 || summary.influence_count != 5 ||
        summary.geometry_quality.tier != SIDEREON_OBSERVABILITY_TIER_NOMINAL ||
        summary.geometry_quality.redundancy != 1 || summary.geometry_quality.rank != 4 ||
        !summary.geometry_quality.raim_checkable ||
        !summary.geometry_quality.covariance_validated ||
        !isfinite(summary.geometry_quality.condition_number) ||
        !isfinite(summary.geometry_quality.gdop) || summary.geometry_quality.gdop <= 0.0 ||
        !vec_close3(summary.position_m, toa_source, 3, 1.0e-7) ||
        !close_abs(summary.origin_time_s, 1.25, 1.0e-10)) {
        sidereon_source_solution_free(solution);
        return fail("source localization: toa summary");
    }

    bool covariance_available = false;
    SidereonSourceCovariance covariance;
    if (require_ok(sidereon_source_solution_covariance(solution, &covariance,
                                                       &covariance_available),
                   "source covariance") != 0 ||
        !covariance_available || covariance.dimension != 3 || covariance.state_dimension != 4) {
        sidereon_source_solution_free(solution);
        return fail("source localization: covariance");
    }

    size_t written = 0;
    size_t required = 0;
    if (require_ok(sidereon_source_solution_residuals(solution, NULL, 0, &written, &required),
                   "source residual count") != 0 ||
        written != 0 || required != 5) {
        sidereon_source_solution_free(solution);
        return fail("source localization: residual count");
    }
    SidereonSourceResidual residuals[5];
    if (require_ok(sidereon_source_solution_residuals(solution, residuals, 5, &written,
                                                      &required),
                   "source residual fill") != 0 ||
        written != 5) {
        sidereon_source_solution_free(solution);
        return 1;
    }
    for (size_t i = 0; i < 5; i++) {
        if (residuals[i].sensor_index != i || residuals[i].has_reference_sensor_index ||
            fabs(residuals[i].residual_s) > 1.0e-10) {
            sidereon_source_solution_free(solution);
            return fail("source localization: residual values");
        }
    }

    SidereonSourceSensorInfluence influences[5];
    if (require_ok(sidereon_source_solution_influences(solution, influences, 5, &written,
                                                       &required),
                   "source influence fill") != 0 ||
        written != 5 || required != 5) {
        sidereon_source_solution_free(solution);
        return 1;
    }
    for (size_t i = 0; i < 5; i++) {
        if (influences[i].sensor_index != i || !isfinite(influences[i].score)) {
            sidereon_source_solution_free(solution);
            return fail("source localization: influence values");
        }
    }
    sidereon_source_solution_free(solution);
    solution = NULL;

    SidereonSourceSensor tdoa_sensors[4] = {
        {2, {0.0, 0.0, 0.0}, false, 0.0},
        {2, {1000.0, 0.0, 0.0}, false, 0.0},
        {2, {0.0, 800.0, 0.0}, false, 0.0},
        {2, {900.0, 900.0, 0.0}, false, 0.0},
    };
    double tdoa_source[2] = {300.0, 260.0};
    double tdoa_times[4];
    fill_arrivals(tdoa_sensors, 4, tdoa_source, 4.0, 340.0, tdoa_times);
    options.mode = SIDEREON_SOURCE_SOLVE_MODE_TDOA;
    options.reference_sensor = 0;
    if (require_ok(sidereon_locate_source(tdoa_sensors, 4, tdoa_times, 340.0, &options,
                                          &solution),
                   "locate source tdoa") != 0) {
        return 1;
    }
    if (require_ok(sidereon_source_solution_summary(solution, &summary),
                   "source tdoa summary") != 0) {
        sidereon_source_solution_free(solution);
        return 1;
    }
    if (summary.dimension != 2 || summary.residual_count != 3 ||
        !vec_close3(summary.position_m, tdoa_source, 2, 1.0e-7) ||
        !close_abs(summary.origin_time_s, 4.0, 1.0e-9)) {
        sidereon_source_solution_free(solution);
        return fail("source localization: tdoa summary");
    }
    sidereon_source_solution_free(solution);

    SidereonSourceSensor dop_sensors[4] = {
        {2, {100.0, 0.0, 0.0}, false, 0.0},
        {2, {-100.0, 0.0, 0.0}, false, 0.0},
        {2, {0.0, 100.0, 0.0}, false, 0.0},
        {2, {0.0, -100.0, 0.0}, false, 0.0},
    };
    double dop_source[2] = {0.0, 0.0};
    SidereonDop dop;
    SidereonSourceCrlb crlb;
    if (require_ok(sidereon_source_dop(dop_sensors, 4, dop_source, 2, 10.0, &dop),
                   "source dop") != 0 ||
        !close_abs(dop.pdop, 10.0, 1.0e-12) || !close_abs(dop.hdop, 10.0, 1.0e-12) ||
        !close_abs(dop.vdop, 0.0, 0.0) || !close_abs(dop.tdop, 0.5, 1.0e-12) ||
        !close_abs(dop.gdop, sqrt(100.25), 1.0e-12) ||
        require_ok(sidereon_source_crlb(dop_sensors, 4, dop_source, 2, 10.0, 0.01, &crlb),
                   "source crlb") != 0 ||
        !close_abs(crlb.covariance.position_m2[0], 0.005, 1.0e-15) ||
        !close_abs(crlb.covariance.position_m2[4], 0.005, 1.0e-15) ||
        !crlb.covariance.has_origin_time_s2 ||
        !close_abs(crlb.covariance.origin_time_s2, 0.000025, 1.0e-18)) {
        return fail("source localization: dop crlb");
    }

    SidereonSourceSensor singular_sensors[4] = {
        {2, {0.0, 0.0, 0.0}, false, 0.0},
        {2, {100.0, 0.0, 0.0}, false, 0.0},
        {2, {200.0, 0.0, 0.0}, false, 0.0},
        {2, {300.0, 0.0, 0.0}, false, 0.0},
    };
    double singular_source[2] = {50.0, 0.0};
    if (sidereon_source_dop(singular_sensors, 4, singular_source, 2, 300.0, &dop) !=
            SIDEREON_STATUS_SOLVE ||
        !last_error_contains("singular")) {
        return fail("source localization: singular geometry status");
    }

    printf("cap013_source_localization_smoke: OK\n");
    return 0;
}

int main(int argc, char **argv) {
    if (argc < 2) {
        fprintf(stderr, "usage: %s <grg_sp3>\n", argv[0]);
        return 2;
    }
    if (exercise_observable_states(argv[1]) != 0) {
        return 1;
    }
    if (exercise_estimation_primitives() != 0) {
        return 1;
    }
    if (exercise_source_localization() != 0) {
        return 1;
    }
    return 0;
}
