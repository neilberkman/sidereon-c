#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "sidereon.h"

static uint64_t f64_to_bits(double value) {
    uint64_t bits;
    memcpy(&bits, &value, sizeof(bits));
    return bits;
}

static double distance_to_source(const SidereonSourceSensor *sensor, const double *source_m) {
    double squared_distance_m2 = 0.0;
    for (size_t axis = 0; axis < sensor->dimension; axis++) {
        double delta_m = source_m[axis] - sensor->position_m[axis];
        squared_distance_m2 += delta_m * delta_m;
    }
    return sqrt(squared_distance_m2);
}

static void fill_arrivals(const SidereonSourceSensor *sensors, size_t sensor_count,
                          const double *source_m, double origin_time_s,
                          double propagation_speed_m_s, double *arrival_times_s) {
    for (size_t index = 0; index < sensor_count; index++) {
        arrival_times_s[index] =
            origin_time_s + distance_to_source(&sensors[index], source_m) / propagation_speed_m_s;
    }
}

static int require_ok(SidereonStatus status, const char *operation) {
    if (status == SIDEREON_STATUS_OK) {
        return 0;
    }
    char message[512];
    size_t written = sidereon_last_error_message(message, sizeof(message));
    fprintf(stderr, "FAIL: %s%s%s\n", operation, written == 0 ? "" : ": ",
            written == 0 ? "" : message);
    return 1;
}

static int fail(const char *message) {
    fprintf(stderr, "FAIL: %s\n", message);
    return 1;
}

int main(void) {
    SidereonSourceSolution *legacy_solution = NULL;
    SidereonSourceSolution *lean_solution = NULL;
    SidereonSourceSensor sensors[5] = {
        {3, {0.0, 0.0, 0.0}, false, 0.0},
        {3, {1200.0, 0.0, 0.0}, false, 0.0},
        {3, {0.0, 900.0, 0.0}, false, 0.0},
        {3, {0.0, 0.0, 700.0}, false, 0.0},
        {3, {1100.0, 800.0, 600.0}, false, 0.0},
    };
    double source_m[3] = {320.0, 260.0, 180.0};
    double arrival_times_s[5];
    fill_arrivals(sensors, 5, source_m, 12.5, 343.0, arrival_times_s);
    const double noise_s[5] = {0.00031, -0.00022, 0.00017, -0.00008, 0.00041};
    for (size_t index = 0; index < 5; index++) {
        arrival_times_s[index] += noise_s[index];
    }

    SidereonSourceLocateOptions options;
    if (require_ok(sidereon_source_locate_options_init(&options), "initialize options") != 0) {
        return 1;
    }
    options.timing_sigma_s = 0.001;
    if (require_ok(sidereon_locate_source(sensors, 5, arrival_times_s, 343.0, &options,
                                          &legacy_solution),
                   "legacy source solve") != 0 ||
        require_ok(sidereon_locate_source_with(sensors, 5, arrival_times_s, 343.0, &options, false,
                                               &lean_solution),
                   "source solve without influence") != 0) {
        sidereon_source_solution_free(lean_solution);
        sidereon_source_solution_free(legacy_solution);
        return 1;
    }

    SidereonSourceSolutionSummary legacy_summary;
    SidereonSourceSolutionSummary lean_summary;
    if (require_ok(sidereon_source_solution_summary(legacy_solution, &legacy_summary),
                   "legacy summary") != 0 ||
        require_ok(sidereon_source_solution_summary(lean_solution, &lean_summary),
                   "lean summary") != 0) {
        sidereon_source_solution_free(lean_solution);
        sidereon_source_solution_free(legacy_solution);
        return 1;
    }
    if (legacy_summary.influence_count != 5 || lean_summary.influence_count != 0 ||
        legacy_summary.dimension != lean_summary.dimension ||
        legacy_summary.has_origin_time_s != lean_summary.has_origin_time_s) {
        sidereon_source_solution_free(lean_solution);
        sidereon_source_solution_free(legacy_solution);
        return fail("influence opt-out summary");
    }
    for (size_t axis = 0; axis < 3; axis++) {
        if (f64_to_bits(legacy_summary.position_m[axis]) !=
            f64_to_bits(lean_summary.position_m[axis])) {
            sidereon_source_solution_free(lean_solution);
            sidereon_source_solution_free(legacy_solution);
            return fail("influence opt-out changed position bits");
        }
    }
    if (f64_to_bits(legacy_summary.origin_time_s) != f64_to_bits(lean_summary.origin_time_s)) {
        sidereon_source_solution_free(lean_solution);
        sidereon_source_solution_free(legacy_solution);
        return fail("influence opt-out changed origin-time bits");
    }
    size_t written = SIZE_MAX;
    size_t required = SIZE_MAX;
    if (require_ok(sidereon_source_solution_influences(lean_solution, NULL, 0, &written, &required),
                   "lean influence query") != 0 ||
        written != 0 || required != 0) {
        sidereon_source_solution_free(lean_solution);
        sidereon_source_solution_free(legacy_solution);
        return fail("influence opt-out did not return an empty list");
    }
    sidereon_source_solution_free(lean_solution);
    sidereon_source_solution_free(legacy_solution);

    SidereonSourceSensor seed_sensors[4] = {
        {2, {0.0, 0.0, 0.0}, false, 0.0},
        {2, {700.0, 0.0, 0.0}, false, 0.0},
        {2, {0.0, 600.0, 0.0}, false, 0.0},
        {2, {650.0, 550.0, 0.0}, false, 0.0},
    };
    double seed_source_m[2] = {210.0, 170.0};
    double seed_arrival_times_s[4];
    fill_arrivals(seed_sensors, 4, seed_source_m, 2.75, 343.0, seed_arrival_times_s);
    SidereonSourceInitialGuess closed_form;
    SidereonSourceInitialGuess deprecated;
    if (require_ok(sidereon_closed_form_initial_guess(
                       seed_sensors, 4, seed_arrival_times_s, 343.0,
                       SIDEREON_SOURCE_SOLVE_MODE_TOA, 0, &closed_form),
                   "closed-form initializer") != 0 ||
        require_ok(sidereon_chan_ho_initial_guess(seed_sensors, 4, seed_arrival_times_s, 343.0,
                                                  SIDEREON_SOURCE_SOLVE_MODE_TOA, 0, &deprecated),
                   "deprecated initializer alias") != 0) {
        return 1;
    }
    if (closed_form.dimension != 2 || !closed_form.has_origin_time_s ||
        fabs(closed_form.position_m[0] - seed_source_m[0]) >= 1.0e-8 ||
        fabs(closed_form.position_m[1] - seed_source_m[1]) >= 1.0e-8 ||
        fabs(closed_form.origin_time_s - 2.75) >= 1.0e-10 ||
        closed_form.dimension != deprecated.dimension ||
        closed_form.has_origin_time_s != deprecated.has_origin_time_s) {
        return fail("clean 2D initializer result");
    }
    for (size_t axis = 0; axis < 3; axis++) {
        if (f64_to_bits(closed_form.position_m[axis]) != f64_to_bits(deprecated.position_m[axis])) {
            return fail("initializer symbols changed position bits");
        }
    }
    if (f64_to_bits(closed_form.origin_time_s) != f64_to_bits(deprecated.origin_time_s) ||
        f64_to_bits(closed_form.residual_rms_s) != f64_to_bits(deprecated.residual_rms_s)) {
        return fail("initializer symbols disagree");
    }

    puts("source_localization_1_1_smoke: OK");
    return 0;
}
