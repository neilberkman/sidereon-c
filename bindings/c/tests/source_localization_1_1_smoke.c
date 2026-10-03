#include <math.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "sidereon.h"
#include "w6_source_localization_pins.h"

static uint64_t f64_to_bits(double value) {
    uint64_t bits;
    memcpy(&bits, &value, sizeof(bits));
    return bits;
}

static double bits_to_f64(uint64_t bits) {
    double value;
    memcpy(&value, &bits, sizeof(value));
    return value;
}

/* The sensors, arrival times and every expected value come from tests/valgen
 * (w6_source_localization): the inputs as it builds them, the results as
 * sidereon-core computes them. */
static void fill_sensors(SidereonSourceSensor *sensors, size_t count, size_t dimension,
                         const uint64_t positions[][3]) {
    for (size_t index = 0; index < count; index++) {
        sensors[index].dimension = dimension;
        for (size_t axis = 0; axis < 3; axis++) {
            sensors[index].position_m[axis] = bits_to_f64(positions[index][axis]);
        }
        sensors[index].has_propagation_speed_m_s = false;
        sensors[index].propagation_speed_m_s = 0.0;
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
    SidereonSourceSensor sensors[W6_SL_SENSOR_COUNT];
    fill_sensors(sensors, W6_SL_SENSOR_COUNT, 3, W6_SL_SENSOR_POSITION_BITS);
    double arrival_times_s[W6_SL_SENSOR_COUNT];
    for (size_t index = 0; index < W6_SL_SENSOR_COUNT; index++) {
        arrival_times_s[index] = bits_to_f64(W6_SL_ARRIVAL_BITS[index]);
    }
    const double speed_m_s = bits_to_f64(W6_SL_SPEED_BITS);

    SidereonSourceLocateOptions options;
    if (require_ok(sidereon_source_locate_options_init(&options), "initialize options") != 0) {
        return 1;
    }
    options.timing_sigma_s = 0.001;
    if (require_ok(sidereon_locate_source(sensors, W6_SL_SENSOR_COUNT, arrival_times_s,
                                          speed_m_s, &options,
                                          &legacy_solution),
                   "legacy source solve") != 0 ||
        require_ok(sidereon_locate_source_with(sensors, W6_SL_SENSOR_COUNT, arrival_times_s,
                                               speed_m_s, &options, false,
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
    if (legacy_summary.influence_count != W6_SL_LEGACY_INFLUENCE_COUNT ||
        lean_summary.influence_count != W6_SL_LEAN_INFLUENCE_COUNT ||
        legacy_summary.dimension != lean_summary.dimension ||
        legacy_summary.has_origin_time_s != lean_summary.has_origin_time_s) {
        sidereon_source_solution_free(lean_solution);
        sidereon_source_solution_free(legacy_solution);
        return fail("influence opt-out summary");
    }
    for (size_t axis = 0; axis < 3; axis++) {
        if (f64_to_bits(legacy_summary.position_m[axis]) != W6_SL_LEGACY_POSITION_BITS[axis]) {
            sidereon_source_solution_free(lean_solution);
            sidereon_source_solution_free(legacy_solution);
            return fail("legacy solve position");
        }
        if (f64_to_bits(legacy_summary.position_m[axis]) !=
            f64_to_bits(lean_summary.position_m[axis])) {
            sidereon_source_solution_free(lean_solution);
            sidereon_source_solution_free(legacy_solution);
            return fail("influence opt-out changed position bits");
        }
    }
    if (f64_to_bits(legacy_summary.origin_time_s) != W6_SL_LEGACY_ORIGIN_BITS ||
        f64_to_bits(legacy_summary.origin_time_s) != f64_to_bits(lean_summary.origin_time_s)) {
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

    SidereonSourceSensor seed_sensors[W6_SL_SEED_SENSOR_COUNT];
    fill_sensors(seed_sensors, W6_SL_SEED_SENSOR_COUNT, 2, W6_SL_SEED_SENSOR_POSITION_BITS);
    double seed_arrival_times_s[W6_SL_SEED_SENSOR_COUNT];
    for (size_t index = 0; index < W6_SL_SEED_SENSOR_COUNT; index++) {
        seed_arrival_times_s[index] = bits_to_f64(W6_SL_SEED_ARRIVAL_BITS[index]);
    }
    /* The 2D source (210, 170) m and emission time 2.75 s tests/valgen formed
     * the seed arrivals from: the guess recovers them to within 1e-8 m and
     * 1e-10 s, an accuracy check against that synthetic truth. */
    const double seed_source_m[2] = {210.0, 170.0};
    SidereonDop expected_dop;
    if (require_ok(sidereon_source_dop(seed_sensors, W6_SL_SEED_SENSOR_COUNT,
                                        seed_source_m, 2, speed_m_s, &expected_dop),
                   "DOP with disjoint inputs") != 0) {
        return 1;
    }
    union {
        max_align_t alignment;
        double position[2];
        SidereonDop dop;
    } dop_alias;
    memcpy(dop_alias.position, seed_source_m, sizeof dop_alias.position);
    if (require_ok(sidereon_source_dop(seed_sensors, W6_SL_SEED_SENSOR_COUNT,
                                        dop_alias.position, 2, speed_m_s, &dop_alias.dop),
                   "DOP with position/output overlap") != 0) {
        return 1;
    }
    if (f64_to_bits(dop_alias.dop.gdop) != f64_to_bits(expected_dop.gdop) ||
        f64_to_bits(dop_alias.dop.pdop) != f64_to_bits(expected_dop.pdop) ||
        f64_to_bits(dop_alias.dop.hdop) != f64_to_bits(expected_dop.hdop) ||
        f64_to_bits(dop_alias.dop.vdop) != f64_to_bits(expected_dop.vdop) ||
        f64_to_bits(dop_alias.dop.tdop) != f64_to_bits(expected_dop.tdop)) {
        return fail("DOP snapshots position before writing overlapping output");
    }
    SidereonSourceInitialGuess closed_form;
    SidereonSourceInitialGuess deprecated;
    if (require_ok(sidereon_closed_form_initial_guess(
                       seed_sensors, W6_SL_SEED_SENSOR_COUNT, seed_arrival_times_s, speed_m_s,
                       SIDEREON_SOURCE_SOLVE_MODE_TOA, 0, &closed_form),
                   "closed-form initializer") != 0 ||
        require_ok(sidereon_chan_ho_initial_guess(seed_sensors, W6_SL_SEED_SENSOR_COUNT,
                                                  seed_arrival_times_s, speed_m_s,
                                                  SIDEREON_SOURCE_SOLVE_MODE_TOA, 0, &deprecated),
                   "deprecated initializer alias") != 0) {
        return 1;
    }
    if (closed_form.dimension != W6_SL_GUESS_DIMENSION ||
        closed_form.has_origin_time_s != W6_SL_GUESS_HAS_ORIGIN ||
        f64_to_bits(closed_form.position_m[0]) != W6_SL_GUESS_POSITION_BITS[0] ||
        f64_to_bits(closed_form.position_m[1]) != W6_SL_GUESS_POSITION_BITS[1] ||
        f64_to_bits(closed_form.position_m[2]) != W6_SL_GUESS_POSITION_BITS[2] ||
        f64_to_bits(closed_form.origin_time_s) != W6_SL_GUESS_ORIGIN_BITS ||
        f64_to_bits(closed_form.residual_rms_s) != W6_SL_GUESS_RESIDUAL_RMS_BITS ||
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
