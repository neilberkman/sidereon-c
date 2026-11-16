#include "ppp_fixture.h"
#include "sidereon.h"

#include <assert.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

static double f64(uint64_t bits) {
    union {
        uint64_t bits;
        double value;
    } value = {.bits = bits};
    return value.value;
}

static uint64_t bits(double value) {
    union {
        uint64_t bits;
        double value;
    } result = {.value = value};
    return result.bits;
}

static uint8_t *read_file(const char *path, size_t *length) {
    FILE *file = fopen(path, "rb");
    assert(file != NULL);
    assert(fseek(file, 0, SEEK_END) == 0);
    long size = ftell(file);
    assert(size >= 0);
    rewind(file);
    uint8_t *data = malloc((size_t)size);
    assert(data != NULL);
    assert(fread(data, 1, (size_t)size, file) == (size_t)size);
    assert(fclose(file) == 0);
    *length = (size_t)size;
    return data;
}

static void fill_epochs(struct SidereonPppObservation *observations,
                        struct SidereonPppEpoch *epochs) {
    for (size_t i = 0; i < PPP_OBS_COUNT; ++i) {
        observations[i] = (struct SidereonPppObservation){
            .sat_id = PPP_OBS_SAT_IDS[i],
            .ambiguity_id = PPP_OBS_AMBIGUITY_IDS[i],
            .code_m = f64(PPP_OBS_CODE_BITS[i]),
            .phase_m = f64(PPP_OBS_PHASE_BITS[i]),
            .freq1_hz = f64(PPP_OBS_FREQ1_HZ_BITS[i]),
            .freq2_hz = f64(PPP_OBS_FREQ2_HZ_BITS[i]),
        };
    }
    for (size_t i = 0; i < PPP_EPOCH_COUNT; ++i) {
        epochs[i] = (struct SidereonPppEpoch){
            .civil = {.year = PPP_EPOCH_YEARS[i],
                      .month = PPP_EPOCH_MONTHS[i],
                      .day = PPP_EPOCH_DAYS[i],
                      .hour = PPP_EPOCH_HOURS[i],
                      .minute = PPP_EPOCH_MINUTES[i],
                      .second = f64(PPP_EPOCH_SECOND_BITS[i])},
            .jd_whole = f64(PPP_EPOCH_JD_WHOLE_BITS[i]),
            .jd_fraction = f64(PPP_EPOCH_JD_FRACTION_BITS[i]),
            .t_rx_j2000_s = f64(PPP_EPOCH_T_RX_J2000_S_BITS[i]),
            .observations = &observations[PPP_EPOCH_OBS_OFFSETS[i]],
            .observation_count = PPP_EPOCH_OBS_COUNTS[i],
        };
    }
}

static void fill_float_config(struct SidereonPppFloatConfig *config,
                              struct SidereonPppEpoch *epochs, double *clocks,
                              struct SidereonPppFloatMapEntry *ambiguities) {
    assert(sidereon_ppp_measurement_weights_init(&config->weights) == SIDEREON_STATUS_OK);
    assert(sidereon_ppp_troposphere_options_init(&config->tropo) == SIDEREON_STATUS_OK);
    assert(sidereon_ppp_range_corrections_init(&config->corrections) == SIDEREON_STATUS_OK);
    assert(sidereon_ppp_float_options_init(&config->options) == SIDEREON_STATUS_OK);
    config->epochs = epochs;
    config->epoch_count = PPP_EPOCH_COUNT;
    for (size_t axis = 0; axis < 3; ++axis) {
        config->initial_state.position_m[axis] = f64(PPP_INITIAL_POSITION_BITS[axis]);
    }
    for (size_t i = 0; i < PPP_EPOCH_COUNT; ++i) {
        clocks[i] = f64(PPP_INITIAL_CLOCK_BITS[i]);
    }
    config->initial_state.clocks_m = clocks;
    config->initial_state.clock_count = PPP_EPOCH_COUNT;
    for (size_t i = 0; i < PPP_INITIAL_AMBIGUITY_COUNT; ++i) {
        ambiguities[i].id = PPP_INITIAL_AMBIGUITY_IDS[i];
        ambiguities[i].value = f64(PPP_INITIAL_AMBIGUITY_BITS[i]);
    }
    config->initial_state.ambiguities_m = ambiguities;
    config->initial_state.ambiguity_count = PPP_INITIAL_AMBIGUITY_COUNT;
    config->initial_state.ztd_m = f64(PPP_INITIAL_ZTD_BITS);
    config->initial_state.tropo_gradient_north_m = 0.0;
    config->initial_state.tropo_gradient_east_m = 0.0;
    config->weights.code = f64(PPP_WEIGHT_CODE_BITS);
    config->weights.phase = f64(PPP_WEIGHT_PHASE_BITS);
    config->weights.elevation_weighting = PPP_WEIGHT_ELEVATION_WEIGHTING;
    config->tropo.enabled = PPP_TROPO_ENABLED;
    config->tropo.estimate_ztd = PPP_TROPO_ESTIMATE_ZTD;
    config->tropo.pressure_hpa = f64(PPP_TROPO_PRESSURE_HPA_BITS);
    config->tropo.temperature_k = f64(PPP_TROPO_TEMPERATURE_K_BITS);
    config->tropo.relative_humidity = f64(PPP_TROPO_RELATIVE_HUMIDITY_BITS);
    config->options.max_iterations = PPP_OPTS_MAX_ITERATIONS;
    config->options.position_tolerance_m = f64(PPP_OPTS_POSITION_TOLERANCE_BITS);
    config->options.clock_tolerance_m = f64(PPP_OPTS_CLOCK_TOLERANCE_BITS);
    config->options.ambiguity_tolerance_m = f64(PPP_OPTS_AMBIGUITY_TOLERANCE_BITS);
    config->options.ztd_tolerance_m = f64(PPP_OPTS_ZTD_TOLERANCE_BITS);
    config->residual_screen = PPP_RESIDUAL_SCREEN;
}

static void fill_fixed_config(struct SidereonPppFixedConfig *config,
                              const struct SidereonPppFloatConfig *float_config,
                              struct SidereonPppFloatMapEntry *wavelengths,
                              struct SidereonPppFloatMapEntry *offsets) {
    config->epochs = float_config->epochs;
    config->epoch_count = float_config->epoch_count;
    config->weights = float_config->weights;
    config->tropo = float_config->tropo;
    config->corrections = float_config->corrections;
    config->options = float_config->options;
    assert(sidereon_ppp_fixed_ambiguity_options_init(&config->ambiguity) == SIDEREON_STATUS_OK);
    for (size_t i = 0; i < PPP_FIXED_AMBIGUITY_COUNT; ++i) {
        wavelengths[i] = (struct SidereonPppFloatMapEntry){.id = PPP_WAVELENGTH_IDS[i],
                                                           .value = f64(PPP_WAVELENGTH_BITS[i])};
        offsets[i] = (struct SidereonPppFloatMapEntry){.id = PPP_OFFSET_IDS[i],
                                                       .value = f64(PPP_OFFSET_BITS[i])};
    }
    config->ambiguity.wavelengths_m = wavelengths;
    config->ambiguity.wavelength_count = PPP_FIXED_AMBIGUITY_COUNT;
    config->ambiguity.offsets_m = offsets;
    config->ambiguity.offset_count = PPP_FIXED_AMBIGUITY_COUNT;
    config->ambiguity.ratio_threshold = f64(PPP_FIXED_RATIO_THRESHOLD_BITS);
}

static void assert_position(const struct SidereonPppFloatSolution *solution,
                            const uint64_t expected[3]) {
    double position[3];
    assert(sidereon_ppp_float_solution_position(solution, position, 3) == SIDEREON_STATUS_OK);
    for (size_t axis = 0; axis < 3; ++axis) {
        assert(bits(position[axis]) == expected[axis]);
    }
}

int main(int argc, char **argv) {
    assert(argc == 2);
    size_t sp3_length = 0;
    uint8_t *sp3_bytes = read_file(argv[1], &sp3_length);
    struct SidereonSp3 *sp3 = NULL;
    assert(sidereon_sp3_load(sp3_bytes, sp3_length, &sp3) == SIDEREON_STATUS_OK);
    free(sp3_bytes);

    struct SidereonPppObservation *observations = calloc(PPP_OBS_COUNT, sizeof(*observations));
    struct SidereonPppEpoch *epochs = calloc(PPP_EPOCH_COUNT, sizeof(*epochs));
    double *clocks = calloc(PPP_EPOCH_COUNT, sizeof(*clocks));
    assert(observations != NULL && epochs != NULL && clocks != NULL);
    fill_epochs(observations, epochs);
    struct SidereonPppFloatMapEntry initial_ambiguities[PPP_INITIAL_AMBIGUITY_COUNT];
    struct SidereonPppFloatConfig float_config = {0};
    fill_float_config(&float_config, epochs, clocks, initial_ambiguities);
    struct SidereonPppFloatConfigV2 float_v2 = {.legacy = float_config,
                                                .estimate_residual_ionosphere = false};

    bool equal = false;
    struct SidereonPppAutoInitOptions auto_init;
    assert(sidereon_ppp_auto_init_options_init(&auto_init) == SIDEREON_STATUS_OK);
    assert(sidereon_ppp_auto_init_options_equal(&auto_init, &auto_init, &equal) ==
           SIDEREON_STATUS_OK);
    assert(equal);
    struct SidereonPppAutoInitOptions unequal_auto_init = auto_init;
    unequal_auto_init.spp_pressure_hpa += 1.0;
    assert(sidereon_ppp_auto_init_options_equal(&auto_init, &unequal_auto_init, &equal) ==
           SIDEREON_STATUS_OK);
    assert(!equal);
    assert(sidereon_ppp_auto_init_options_equal(NULL, &auto_init, &equal) ==
           SIDEREON_STATUS_NULL_POINTER);
    assert(sidereon_ppp_observation_equal(&observations[0], &observations[0], &equal) ==
           SIDEREON_STATUS_OK);
    assert(equal);
    struct SidereonPppObservation unequal_observation = observations[0];
    unequal_observation.code_m += 1.0;
    assert(sidereon_ppp_observation_equal(&observations[0], &unequal_observation, &equal) ==
           SIDEREON_STATUS_OK);
    assert(!equal);
    assert(sidereon_ppp_observation_equal(NULL, &observations[0], &equal) ==
           SIDEREON_STATUS_NULL_POINTER);
    assert(sidereon_ppp_epoch_equal(&epochs[0], &epochs[0], &equal) == SIDEREON_STATUS_OK);
    assert(equal);
    struct SidereonPppEpoch unequal_epoch = epochs[0];
    unequal_epoch.t_rx_j2000_s += 1.0;
    assert(sidereon_ppp_epoch_equal(&epochs[0], &unequal_epoch, &equal) == SIDEREON_STATUS_OK);
    assert(!equal);
    assert(sidereon_ppp_epoch_equal(NULL, &epochs[0], &equal) == SIDEREON_STATUS_NULL_POINTER);
    assert(sidereon_ppp_measurement_weights_equal(&float_config.weights, &float_config.weights,
                                                  &equal) == SIDEREON_STATUS_OK);
    assert(equal);
    struct SidereonPppMeasurementWeights unequal_weights = float_config.weights;
    unequal_weights.code += 1.0;
    assert(sidereon_ppp_measurement_weights_equal(&float_config.weights, &unequal_weights,
                                                  &equal) == SIDEREON_STATUS_OK);
    assert(!equal);
    assert(sidereon_ppp_measurement_weights_equal(NULL, &unequal_weights, &equal) ==
           SIDEREON_STATUS_NULL_POINTER);
    assert(sidereon_ppp_float_options_equal(&float_config.options, &float_config.options, &equal) ==
           SIDEREON_STATUS_OK);
    assert(equal);
    struct SidereonPppFloatOptions unequal_options = float_config.options;
    unequal_options.max_iterations++;
    assert(sidereon_ppp_float_options_equal(&float_config.options, &unequal_options, &equal) ==
           SIDEREON_STATUS_OK);
    assert(!equal);
    assert(sidereon_ppp_float_options_equal(NULL, &unequal_options, &equal) ==
           SIDEREON_STATUS_NULL_POINTER);
    assert(sidereon_ppp_troposphere_options_equal(&float_config.tropo, &float_config.tropo,
                                                  &equal) == SIDEREON_STATUS_OK);
    assert(equal);
    struct SidereonPppTroposphereOptions unequal_tropo = float_config.tropo;
    unequal_tropo.pressure_hpa += 1.0;
    assert(sidereon_ppp_troposphere_options_equal(&float_config.tropo, &unequal_tropo, &equal) ==
           SIDEREON_STATUS_OK);
    assert(!equal);
    assert(sidereon_ppp_troposphere_options_equal(NULL, &unequal_tropo, &equal) ==
           SIDEREON_STATUS_NULL_POINTER);
    assert(sidereon_ppp_range_corrections_equal(
               &float_config.corrections, &float_config.corrections, &equal) == SIDEREON_STATUS_OK);
    assert(equal);
    struct SidereonPppRangeCorrections unequal_corrections = float_config.corrections;
    unequal_corrections.sat_clock_relativity = true;
    assert(sidereon_ppp_range_corrections_equal(&float_config.corrections, &unequal_corrections,
                                                &equal) == SIDEREON_STATUS_OK);
    assert(!equal);
    assert(sidereon_ppp_range_corrections_equal(NULL, &unequal_corrections, &equal) ==
           SIDEREON_STATUS_NULL_POINTER);
    assert(sidereon_ppp_float_config_v2_equal(&float_v2, &float_v2, &equal) == SIDEREON_STATUS_OK);
    assert(equal);
    struct SidereonPppFloatConfigV2 unequal_float_v2 = float_v2;
    unequal_float_v2.estimate_residual_ionosphere = true;
    assert(sidereon_ppp_float_config_v2_equal(&float_v2, &unequal_float_v2, &equal) ==
           SIDEREON_STATUS_OK);
    assert(!equal);
    assert(sidereon_ppp_float_config_v2_equal(NULL, &float_v2, &equal) ==
           SIDEREON_STATUS_NULL_POINTER);

    struct SidereonPppFloatSolution *float_solution = NULL;
    assert(sidereon_solve_ppp_float_v2(sp3, &float_v2, &float_solution) == SIDEREON_STATUS_OK);
    assert_position(float_solution, PPP_EXPECTED_FLOAT_POSITION_BITS);
    assert(sidereon_ppp_float_solution_equal(float_solution, float_solution, &equal) ==
           SIDEREON_STATUS_OK);
    assert(equal);
    assert(sidereon_ppp_float_solution_equal(NULL, float_solution, &equal) ==
           SIDEREON_STATUS_NULL_POINTER);
    assert(
        sidereon_ppp_float_solution_equal(float_solution, float_solution, (bool *)float_solution) ==
        SIDEREON_STATUS_INVALID_ARGUMENT);
    bool variance_estimated = false;
    assert(sidereon_ppp_float_solution_posterior_variance_factor_estimated(float_solution, &variance_estimated) == SIDEREON_STATUS_OK);
    assert(variance_estimated);
    assert(sidereon_ppp_float_solution_posterior_variance_factor_estimated(NULL, &variance_estimated) == SIDEREON_STATUS_NULL_POINTER);
    assert(sidereon_ppp_float_solution_posterior_variance_factor_estimated(float_solution, (bool *)float_solution) == SIDEREON_STATUS_INVALID_ARGUMENT);
    double float_posterior = 0.0;
    double decorrelation = 0.0;
    assert(sidereon_ppp_float_solution_posterior_variance_factor(
               float_solution, &float_posterior) == SIDEREON_STATUS_OK);
    assert(sidereon_ppp_float_solution_decorrelation_time_epochs(float_solution, &decorrelation) ==
           SIDEREON_STATUS_OK);
    size_t output_written = 0;
    size_t output_required = 0;
    assert(sidereon_ppp_float_solution_residuals(float_solution, NULL, 0, &output_written,
                                                 &output_required) == SIDEREON_STATUS_OK);
    struct SidereonPppFloatResidual *float_residuals =
        calloc(output_required, sizeof(*float_residuals));
    assert(float_residuals != NULL);
    assert(sidereon_ppp_float_solution_residuals(float_solution, float_residuals, output_required,
                                                 &output_written,
                                                 &output_required) == SIDEREON_STATUS_OK);
    assert(bits(float_posterior) == UINT64_C(0x40ac13b508520b65));
    assert(bits(decorrelation) == UINT64_C(0x4058dff24728af16));
    assert(output_written == 1282);
    assert(bits(float_residuals[0].code_weight) == UINT64_C(0x3ff0000000000000));
    assert(bits(float_residuals[0].phase_weight) == UINT64_C(0x4059000000000000));
    free(float_residuals);
    assert(sidereon_ppp_float_solution_residual_ionosphere(float_solution, NULL, 0, &output_written,
                                                           &output_required) == SIDEREON_STATUS_OK);
    assert(output_required == 0);
    assert(sidereon_ppp_float_solution_posterior_variance_factor(NULL, &float_posterior) ==
           SIDEREON_STATUS_NULL_POINTER);
    assert(sidereon_ppp_float_solution_decorrelation_time_epochs(NULL, &decorrelation) ==
           SIDEREON_STATUS_NULL_POINTER);
    assert(sidereon_ppp_float_solution_residuals(NULL, NULL, 0, &output_written,
                                                 &output_required) == SIDEREON_STATUS_NULL_POINTER);
    assert(sidereon_ppp_float_solution_residual_ionosphere(
               NULL, NULL, 0, &output_written, &output_required) == SIDEREON_STATUS_NULL_POINTER);
    struct SidereonPppFloatSolution *bad_float = NULL;
    assert(sidereon_solve_ppp_float_v2(NULL, &float_v2, &bad_float) ==
           SIDEREON_STATUS_NULL_POINTER);
    assert(bad_float == NULL);
    struct SidereonPppFloatConfigV2 ionosphere_float_v2 = float_v2;
    ionosphere_float_v2.estimate_residual_ionosphere = true;
    struct SidereonPppFloatSolution *ionosphere_float_solution = NULL;
    assert(sidereon_solve_ppp_float_v2(sp3, &ionosphere_float_v2, &ionosphere_float_solution) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_ppp_float_solution_residual_ionosphere(ionosphere_float_solution, NULL, 0,
                                                           &output_written,
                                                           &output_required) == SIDEREON_STATUS_OK);
    assert(output_required == PPP_FIXED_AMBIGUITY_COUNT);
    struct SidereonPppResidualIonosphere *ionosphere = calloc(output_required, sizeof(*ionosphere));
    assert(ionosphere != NULL);
    assert(sidereon_ppp_float_solution_residual_ionosphere(ionosphere_float_solution, ionosphere,
                                                           output_required, &output_written,
                                                           &output_required) == SIDEREON_STATUS_OK);
    assert(output_written == 12);
    assert(bits(ionosphere[0].value_m) == UINT64_C(0xc0a2d39c42e31c7a));
    free(ionosphere);

    struct SidereonPppFloatConfigV2 epoch_v2 = float_v2;
    epoch_v2.legacy.epoch_count = 2;
    struct SidereonPppFloatSolution *epoch_solution = NULL;
    assert(sidereon_solve_ppp_float_epoch(sp3, &epoch_v2, &epoch_solution) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    assert(epoch_solution == NULL);
    epoch_v2.legacy.epoch_count = 1;
    epoch_v2.legacy.initial_state.clock_count = 1;
    assert(sidereon_solve_ppp_float_epoch(sp3, &epoch_v2, &epoch_solution) == SIDEREON_STATUS_OK);
    double epoch_position[3];
    assert(sidereon_ppp_float_solution_position(epoch_solution, epoch_position, 3) ==
           SIDEREON_STATUS_OK);
    static const uint64_t expected_epoch_position[3] = {
        UINT64_C(0x414b544e74870998), UINT64_C(0x412040e0b9b37caf), UINT64_C(0x4153f620b2ff23c7)};
    for (size_t axis = 0; axis < 3; ++axis) {
        if (bits(epoch_position[axis]) != expected_epoch_position[axis]) {
            fprintf(stderr, "epoch_position[%zu] = UINT64_C(0x%016llx)\n", axis,
                    (unsigned long long)bits(epoch_position[axis]));
            return 2;
        }
    }
    assert(sidereon_ppp_float_solution_equal(float_solution, epoch_solution, &equal) ==
           SIDEREON_STATUS_OK);
    assert(!equal);
    sidereon_ppp_float_solution_free(epoch_solution);

    struct SidereonPppFloatMapEntry wavelengths[PPP_FIXED_AMBIGUITY_COUNT];
    struct SidereonPppFloatMapEntry offsets[PPP_FIXED_AMBIGUITY_COUNT];
    struct SidereonPppFixedConfig fixed_config = {0};
    fill_fixed_config(&fixed_config, &float_config, wavelengths, offsets);
    struct SidereonPppFixedConfigV2 fixed_v2 = {.legacy = fixed_config,
                                                .estimate_residual_ionosphere = false};
    assert(sidereon_ppp_fixed_ambiguity_options_equal(
               &fixed_config.ambiguity, &fixed_config.ambiguity, &equal) == SIDEREON_STATUS_OK);
    assert(equal);
    struct SidereonPppFixedAmbiguityOptions unequal_ambiguity = fixed_config.ambiguity;
    unequal_ambiguity.ratio_threshold += 1.0;
    assert(sidereon_ppp_fixed_ambiguity_options_equal(&fixed_config.ambiguity, &unequal_ambiguity,
                                                      &equal) == SIDEREON_STATUS_OK);
    assert(!equal);
    assert(sidereon_ppp_fixed_ambiguity_options_equal(NULL, &unequal_ambiguity, &equal) ==
           SIDEREON_STATUS_NULL_POINTER);
    assert(sidereon_ppp_fixed_config_v2_equal(&fixed_v2, &fixed_v2, &equal) == SIDEREON_STATUS_OK);
    assert(equal);
    struct SidereonPppFixedConfigV2 unequal_fixed_v2 = fixed_v2;
    unequal_fixed_v2.estimate_residual_ionosphere = true;
    assert(sidereon_ppp_fixed_config_v2_equal(&fixed_v2, &unequal_fixed_v2, &equal) ==
           SIDEREON_STATUS_OK);
    assert(!equal);
    assert(sidereon_ppp_fixed_config_v2_equal(NULL, &fixed_v2, &equal) ==
           SIDEREON_STATUS_NULL_POINTER);

    struct SidereonPppFixedSolution *fixed_solution = NULL;
    assert(sidereon_solve_ppp_fixed_v2(sp3, float_solution, &fixed_v2, &fixed_solution) ==
           SIDEREON_STATUS_OK);
    double fixed_position[3];
    assert(sidereon_ppp_fixed_solution_position(fixed_solution, fixed_position, 3) ==
           SIDEREON_STATUS_OK);
    for (size_t axis = 0; axis < 3; ++axis) {
        assert(bits(fixed_position[axis]) == PPP_EXPECTED_FIXED_POSITION_BITS[axis]);
    }
    assert(sidereon_ppp_fixed_solution_equal(fixed_solution, fixed_solution, &equal) ==
           SIDEREON_STATUS_OK);
    assert(equal);
    assert(sidereon_ppp_fixed_solution_equal(NULL, fixed_solution, &equal) ==
           SIDEREON_STATUS_NULL_POINTER);
    double first_wavelength = wavelengths[0].value;
    wavelengths[0].value *= 1.01;
    struct SidereonPppFixedSolution *unequal_fixed_solution = NULL;
    assert(sidereon_solve_ppp_fixed_v2(sp3, float_solution, &fixed_v2, &unequal_fixed_solution) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_ppp_fixed_solution_equal(fixed_solution, unequal_fixed_solution, &equal) ==
           SIDEREON_STATUS_OK);
    assert(!equal);
    wavelengths[0].value = first_wavelength;
    variance_estimated = false;
    assert(sidereon_ppp_fixed_solution_posterior_variance_factor_estimated(fixed_solution, &variance_estimated) == SIDEREON_STATUS_OK);
    assert(variance_estimated);
    assert(sidereon_ppp_fixed_solution_posterior_variance_factor_estimated(NULL, &variance_estimated) == SIDEREON_STATUS_NULL_POINTER);
    assert(sidereon_ppp_fixed_solution_posterior_variance_factor_estimated(fixed_solution, (bool *)fixed_solution) == SIDEREON_STATUS_INVALID_ARGUMENT);
    double fixed_posterior = 0.0;
    assert(sidereon_ppp_fixed_solution_posterior_variance_factor(
               fixed_solution, &fixed_posterior) == SIDEREON_STATUS_OK);
    assert(bits(fixed_posterior) == UINT64_C(0x40abf2ae09d2fd4b));
    assert(sidereon_ppp_fixed_solution_residual_ionosphere(fixed_solution, NULL, 0, &output_written,
                                                           &output_required) == SIDEREON_STATUS_OK);
    assert(output_required == 0);
    assert(sidereon_ppp_fixed_solution_posterior_variance_factor(NULL, &fixed_posterior) ==
           SIDEREON_STATUS_NULL_POINTER);
    assert(sidereon_ppp_fixed_solution_residual_ionosphere(
               NULL, NULL, 0, &output_written, &output_required) == SIDEREON_STATUS_NULL_POINTER);
    struct SidereonPppFixedSolution *bad_fixed = NULL;
    assert(sidereon_solve_ppp_fixed_v2(NULL, float_solution, &fixed_v2, &bad_fixed) ==
           SIDEREON_STATUS_NULL_POINTER);
    assert(bad_fixed == NULL);
    struct SidereonPppFixedConfigV2 ionosphere_fixed_v2 = fixed_v2;
    ionosphere_fixed_v2.estimate_residual_ionosphere = true;
    struct SidereonPppFixedSolution *ionosphere_fixed_solution = NULL;
    assert(sidereon_solve_ppp_fixed_v2(sp3, ionosphere_float_solution, &ionosphere_fixed_v2,
                                       &ionosphere_fixed_solution) == SIDEREON_STATUS_OK);
    assert(sidereon_ppp_fixed_solution_residual_ionosphere(ionosphere_fixed_solution, NULL, 0,
                                                           &output_written,
                                                           &output_required) == SIDEREON_STATUS_OK);
    assert(output_required == 12);

    struct SidereonPppAmbiguitySearch *search = NULL;
    struct SidereonPppAmbiguitySearch *search2 = NULL;
    assert(sidereon_ppp_fixed_solution_ambiguity_search(fixed_solution, &search) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_ppp_fixed_solution_ambiguity_search(fixed_solution, &search2) ==
           SIDEREON_STATUS_OK);
    struct SidereonPppAmbiguitySearch *bad_search = NULL;
    assert(sidereon_ppp_fixed_solution_ambiguity_search(NULL, &bad_search) ==
           SIDEREON_STATUS_NULL_POINTER);
    assert(bad_search == NULL);
    assert(sidereon_ppp_ambiguity_search_equal(search, search, &equal) == SIDEREON_STATUS_OK);
    assert(equal);
    struct SidereonPppAmbiguitySearch *unequal_search = NULL;
    assert(sidereon_ppp_fixed_solution_ambiguity_search(unequal_fixed_solution, &unequal_search) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_ppp_ambiguity_search_equal(search, unequal_search, &equal) ==
           SIDEREON_STATUS_OK);
    assert(!equal);
    assert(sidereon_ppp_ambiguity_search_equal(NULL, search, &equal) ==
           SIDEREON_STATUS_NULL_POINTER);
    size_t written = 0;
    size_t required = 0;
    assert(sidereon_ppp_ambiguity_search_float_cycles(search, NULL, 0, &written, &required) ==
           SIDEREON_STATUS_OK);
    assert(required == PPP_FIXED_AMBIGUITY_COUNT);
    struct SidereonPppFloatAmbiguityCycles cycles[PPP_FIXED_AMBIGUITY_COUNT];
    assert(sidereon_ppp_ambiguity_search_float_cycles(search, cycles, required, &written,
                                                      &required) == SIDEREON_STATUS_OK);
    assert(written == PPP_FIXED_AMBIGUITY_COUNT);
    assert(sidereon_ppp_ambiguity_search_float_cycles(NULL, cycles, required, &written,
                                                      &required) == SIDEREON_STATUS_NULL_POINTER);
    double covariance[PPP_FIXED_AMBIGUITY_COUNT * PPP_FIXED_AMBIGUITY_COUNT];
    assert(sidereon_ppp_ambiguity_search_covariance_cycles(
               search, covariance, PPP_FIXED_AMBIGUITY_COUNT * PPP_FIXED_AMBIGUITY_COUNT, &written,
               &required) == SIDEREON_STATUS_OK);
    assert(written == PPP_FIXED_AMBIGUITY_COUNT * PPP_FIXED_AMBIGUITY_COUNT);
    assert(sidereon_ppp_ambiguity_search_covariance_cycles(
               NULL, covariance, 1, &written, &required) == SIDEREON_STATUS_NULL_POINTER);
    double inverse[PPP_FIXED_AMBIGUITY_COUNT * PPP_FIXED_AMBIGUITY_COUNT];
    assert(sidereon_ppp_ambiguity_search_covariance_inverse_cycles(
               search, inverse, PPP_FIXED_AMBIGUITY_COUNT * PPP_FIXED_AMBIGUITY_COUNT, &written,
               &required) == SIDEREON_STATUS_OK);
    assert(sidereon_ppp_ambiguity_search_covariance_inverse_cycles(
               NULL, inverse, 1, &written, &required) == SIDEREON_STATUS_NULL_POINTER);
    static const uint64_t expected_cycle0 = UINT64_C(0x402d3a503df72a82);
    static const uint64_t expected_covariance0 = UINT64_C(0x3fb1d49bd8be8c8e);
    static const uint64_t expected_inverse0 = UINT64_C(0x40c17c34b60c4572);
    if (bits(cycles[0].value_cycles) != expected_cycle0 ||
        bits(covariance[0]) != expected_covariance0 || bits(inverse[0]) != expected_inverse0) {
        fprintf(stderr,
                "search literals: cycle UINT64_C(0x%016llx), covariance UINT64_C(0x%016llx), "
                "inverse UINT64_C(0x%016llx)\n",
                (unsigned long long)bits(cycles[0].value_cycles),
                (unsigned long long)bits(covariance[0]), (unsigned long long)bits(inverse[0]));
        return 3;
    }
    sidereon_ppp_ambiguity_search_free(search);
    sidereon_ppp_ambiguity_search_free(search2);
    sidereon_ppp_ambiguity_search_free(unequal_search);

    struct SidereonPppCorrectionLookup *lookup = NULL;
    struct SidereonPppCorrectionLookup *lookup2 = NULL;
    assert(sidereon_ppp_correction_lookup_default(&lookup) == SIDEREON_STATUS_OK);
    assert(sidereon_ppp_correction_lookup_default(NULL) == SIDEREON_STATUS_NULL_POINTER);
    struct SidereonPppCorrections *built_corrections = NULL;
    struct SidereonPppCorrectionLookup *bad_lookup = NULL;
    struct SidereonPppCorrectionsOptions correction_options = {0};
    const double receiver[3] = {f64(PPP_INITIAL_POSITION_BITS[0]),
                                f64(PPP_INITIAL_POSITION_BITS[1]),
                                f64(PPP_INITIAL_POSITION_BITS[2])};
    assert(sidereon_ppp_corrections_build(sp3, NULL, 0, receiver, &correction_options,
                                          &built_corrections) == SIDEREON_STATUS_OK);
    struct SidereonPppCorrectionLookup *from_corrections = NULL;
    assert(sidereon_ppp_correction_lookup_from_corrections(built_corrections, &from_corrections) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_ppp_correction_lookup_equal(lookup, from_corrections, &equal) ==
           SIDEREON_STATUS_OK);
    assert(equal);
    assert(sidereon_ppp_correction_lookup_from_corrections(NULL, &bad_lookup) ==
           SIDEREON_STATUS_NULL_POINTER);
    assert(sidereon_build_ppp_lookup(sp3, epochs, 1, receiver, &correction_options, &lookup2) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_build_ppp_lookup(NULL, epochs, 1, receiver, &correction_options, &bad_lookup) ==
           SIDEREON_STATUS_NULL_POINTER);
    assert(bad_lookup == NULL);
    sidereon_ppp_correction_lookup_free(lookup2);
    assert(sidereon_build_ppp_lookup_with_validity(sp3, epochs, 1, receiver, &correction_options,
                                                   SIDEREON_STATION_TIDE_VALIDITY_MODE_STRICT,
                                                   &lookup2) == SIDEREON_STATUS_OK);
    assert(sidereon_build_ppp_lookup_with_validity(sp3, epochs, 1, receiver, &correction_options,
                                                   999, &bad_lookup) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    assert(bad_lookup == NULL);
    assert(sidereon_ppp_correction_lookup_equal(lookup, lookup, &equal) == SIDEREON_STATUS_OK);
    assert(equal);
    assert(sidereon_ppp_correction_lookup_equal(NULL, lookup, &equal) ==
           SIDEREON_STATUS_NULL_POINTER);
    enum SidereonPppCorrectionsDegradeReason degraded;
    struct SidereonPppCorrectionLookupFlags flags;
    assert(sidereon_ppp_correction_lookup_info(lookup, &degraded, &flags) == SIDEREON_STATUS_OK);
    assert(degraded == SIDEREON_PPP_CORRECTIONS_DEGRADE_REASON_NONE && !flags.tide_enabled &&
           !flags.code_bias_enabled);
    assert(sidereon_ppp_correction_lookup_info(NULL, &degraded, &flags) ==
           SIDEREON_STATUS_NULL_POINTER);
    assert(sidereon_ppp_correction_lookup_info(lookup,
                                               (enum SidereonPppCorrectionsDegradeReason *)lookup,
                                               &flags) == SIDEREON_STATUS_INVALID_ARGUMENT);
#define CHECK_EMPTY_ACCESSOR(function, type)                                                       \
    do {                                                                                           \
        assert(function(lookup, NULL, 0, &written, &required) == SIDEREON_STATUS_OK);              \
        assert(written == 0 && required == 0);                                                     \
        type value;                                                                                \
        assert(function(NULL, &value, 1, &written, &required) == SIDEREON_STATUS_NULL_POINTER);    \
    } while (0)
    CHECK_EMPTY_ACCESSOR(sidereon_ppp_correction_lookup_tide, struct SidereonEpochVectorCorrection);
    CHECK_EMPTY_ACCESSOR(sidereon_ppp_correction_lookup_pole_tide,
                         struct SidereonEpochVectorCorrection);
    CHECK_EMPTY_ACCESSOR(sidereon_ppp_correction_lookup_ocean_loading,
                         struct SidereonEpochVectorCorrection);
    CHECK_EMPTY_ACCESSOR(sidereon_ppp_correction_lookup_windup, struct SidereonSatScalarCorrection);
    CHECK_EMPTY_ACCESSOR(sidereon_ppp_correction_lookup_sat_pcv,
                         struct SidereonSatScalarCorrection);
    CHECK_EMPTY_ACCESSOR(sidereon_ppp_correction_lookup_code_bias,
                         struct SidereonSatScalarCorrection);
    CHECK_EMPTY_ACCESSOR(sidereon_ppp_correction_lookup_sat_pco_ecef,
                         struct SidereonSatVectorCorrection);
#undef CHECK_EMPTY_ACCESSOR

    sidereon_ppp_correction_lookup_free(lookup);
    sidereon_ppp_correction_lookup_free(lookup2);
    sidereon_ppp_correction_lookup_free(from_corrections);
    sidereon_ppp_corrections_free(built_corrections);
    sidereon_ppp_fixed_solution_free(fixed_solution);
    sidereon_ppp_fixed_solution_free(unequal_fixed_solution);
    sidereon_ppp_fixed_solution_free(ionosphere_fixed_solution);
    sidereon_ppp_float_solution_free(float_solution);
    sidereon_ppp_float_solution_free(ionosphere_float_solution);
    sidereon_sp3_free(sp3);
    free(clocks);
    free(epochs);
    free(observations);
    return 0;
}
