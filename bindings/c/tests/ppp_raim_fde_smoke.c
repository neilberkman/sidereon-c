#include "ppp_fixture.h"
#include "sidereon.h"

#include <assert.h>
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

int main(int argc, char **argv) {
    assert(argc == 2);
    size_t sp3_length = 0;
    uint8_t *sp3_bytes = read_file(argv[1], &sp3_length);
    SidereonSp3 *sp3 = NULL;
    assert(sidereon_sp3_load(sp3_bytes, sp3_length, &sp3) == SIDEREON_STATUS_OK);
    free(sp3_bytes);

    size_t observation_count = PPP_EPOCH_OBS_COUNTS[0];
    SidereonPppObservation *observations = calloc(observation_count, sizeof(*observations));
    assert(observations != NULL);
    size_t offset = PPP_EPOCH_OBS_OFFSETS[0];
    for (size_t index = 0; index < observation_count; ++index) {
        size_t source = offset + index;
        observations[index] = (SidereonPppObservation){
            .sat_id = PPP_OBS_SAT_IDS[source],
            .ambiguity_id = PPP_OBS_AMBIGUITY_IDS[source],
            .code_m = f64(PPP_OBS_CODE_BITS[source]),
            .phase_m = f64(PPP_OBS_PHASE_BITS[source]),
            .freq1_hz = f64(PPP_OBS_FREQ1_HZ_BITS[source]),
            .freq2_hz = f64(PPP_OBS_FREQ2_HZ_BITS[source]),
        };
    }
    SidereonPppEpoch epoch = {
        .civil = {.year = PPP_EPOCH_YEARS[0],
                  .month = PPP_EPOCH_MONTHS[0],
                  .day = PPP_EPOCH_DAYS[0],
                  .hour = PPP_EPOCH_HOURS[0],
                  .minute = PPP_EPOCH_MINUTES[0],
                  .second = f64(PPP_EPOCH_SECOND_BITS[0])},
        .jd_whole = f64(PPP_EPOCH_JD_WHOLE_BITS[0]),
        .jd_fraction = f64(PPP_EPOCH_JD_FRACTION_BITS[0]),
        .t_rx_j2000_s = f64(PPP_EPOCH_T_RX_J2000_S_BITS[0]),
        .observations = observations,
        .observation_count = observation_count,
    };

    SidereonPppFloatMapEntry ambiguities[PPP_INITIAL_AMBIGUITY_COUNT];
    for (size_t index = 0; index < PPP_INITIAL_AMBIGUITY_COUNT; ++index) {
        ambiguities[index] = (SidereonPppFloatMapEntry){
            .id = PPP_INITIAL_AMBIGUITY_IDS[index],
            .value = f64(PPP_INITIAL_AMBIGUITY_BITS[index]),
        };
    }
    double clock = f64(PPP_INITIAL_CLOCK_BITS[0]);
    SidereonPppFloatConfig config = {0};
    config.epochs = &epoch;
    config.epoch_count = 1;
    for (size_t axis = 0; axis < 3; ++axis) {
        config.initial_state.position_m[axis] = f64(PPP_INITIAL_POSITION_BITS[axis]);
    }
    config.initial_state.clocks_m = &clock;
    config.initial_state.clock_count = 1;
    config.initial_state.ambiguities_m = ambiguities;
    config.initial_state.ambiguity_count = PPP_INITIAL_AMBIGUITY_COUNT;
    config.initial_state.ztd_m = f64(PPP_INITIAL_ZTD_BITS);
    assert(sidereon_ppp_measurement_weights_init(&config.weights) == SIDEREON_STATUS_OK);
    assert(sidereon_ppp_troposphere_options_init(&config.tropo) == SIDEREON_STATUS_OK);
    assert(sidereon_ppp_range_corrections_init(&config.corrections) == SIDEREON_STATUS_OK);
    assert(sidereon_ppp_float_options_init(&config.options) == SIDEREON_STATUS_OK);
    config.weights.code = f64(PPP_WEIGHT_CODE_BITS);
    config.weights.phase = f64(PPP_WEIGHT_PHASE_BITS);
    config.weights.elevation_weighting = PPP_WEIGHT_ELEVATION_WEIGHTING;
    config.tropo.enabled = PPP_TROPO_ENABLED;
    config.tropo.estimate_ztd = PPP_TROPO_ESTIMATE_ZTD;
    config.tropo.pressure_hpa = f64(PPP_TROPO_PRESSURE_HPA_BITS);
    config.tropo.temperature_k = f64(PPP_TROPO_TEMPERATURE_K_BITS);
    config.tropo.relative_humidity = f64(PPP_TROPO_RELATIVE_HUMIDITY_BITS);
    config.options.max_iterations = PPP_OPTS_MAX_ITERATIONS;
    config.options.position_tolerance_m = f64(PPP_OPTS_POSITION_TOLERANCE_BITS);
    config.options.clock_tolerance_m = f64(PPP_OPTS_CLOCK_TOLERANCE_BITS);
    config.options.ambiguity_tolerance_m = f64(PPP_OPTS_AMBIGUITY_TOLERANCE_BITS);
    config.options.ztd_tolerance_m = f64(PPP_OPTS_ZTD_TOLERANCE_BITS);
    SidereonPppFloatConfigV2 config_v2 = {.legacy = config, .estimate_residual_ionosphere = false};

    SidereonPppRaimConfig raim;
    assert(sidereon_ppp_raim_config_default(&raim) == SIDEREON_STATUS_OK);
    raim.has_chi_square_threshold = true;
    raim.chi_square_threshold = 1e30;
    SidereonPppRaimFdeResult *result = NULL;
    assert(sidereon_ppp_solve_float_epoch_with_raim(sp3, &config_v2, raim, &result) ==
           SIDEREON_STATUS_OK);
    SidereonPppRaimFdeInfo info;
    assert(sidereon_ppp_raim_fde_result_info(result, &info) == SIDEREON_STATUS_OK);
    assert(info.status == SIDEREON_PPP_RAIM_FDE_STATUS_CLEAN);
    assert(info.excluded_satellite_count == 0);
    SidereonPppRaimResult *raim_result = NULL;
    assert(sidereon_ppp_raim_fde_result_raim(result, &raim_result) == SIDEREON_STATUS_OK);
    SidereonPppRaimResultInfo raim_info;
    assert(sidereon_ppp_raim_result_info(raim_result, &raim_info) == SIDEREON_STATUS_OK);
    assert(raim_info.status == SIDEREON_PPP_RAIM_STATUS_PASSED);
    assert(!raim_info.detected);
    sidereon_ppp_raim_result_free(raim_result);
    SidereonPppRaimFdeResult *same = NULL;
    assert(sidereon_ppp_raim_fde_float_epoch(sp3, &config_v2, raim, &same) == SIDEREON_STATUS_OK);
    bool equal = false;
    assert(sidereon_ppp_raim_fde_result_equal(result, same, &equal) == SIDEREON_STATUS_OK);
    assert(equal);
    sidereon_ppp_raim_fde_result_free(same);
    SidereonPppFloatSolution *solution = NULL;
    assert(sidereon_ppp_raim_fde_result_solution(result, &solution) == SIDEREON_STATUS_OK);
    double position[3];
    assert(sidereon_ppp_float_solution_position(solution, position, 3) == SIDEREON_STATUS_OK);
    assert(position[0] != 0.0 && position[1] != 0.0 && position[2] != 0.0);
    sidereon_ppp_float_solution_free(solution);
    sidereon_ppp_raim_fde_result_free(result);

    raim.false_alarm_probability = 0.0;
    result = NULL;
    assert(sidereon_ppp_raim_fde_float_epoch(sp3, &config_v2, raim, &result) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    assert(result == NULL);
    SidereonEngineErrorInfo error;
    assert(sidereon_last_engine_error_info(&error) == SIDEREON_STATUS_OK);
    assert(error.family == SIDEREON_ENGINE_ERROR_FAMILY_PPP_RAIM);

    free(observations);
    sidereon_sp3_free(sp3);
    return 0;
}
