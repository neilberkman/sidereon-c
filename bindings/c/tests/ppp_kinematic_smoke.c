#include "sidereon.h"

#include <assert.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

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
    SidereonPppKinematicConfig *config = NULL;
    SidereonPppKinematicConfig *config_copy = NULL;
    assert(sidereon_ppp_kinematic_config_default(&config) == SIDEREON_STATUS_OK);
    assert(sidereon_ppp_kinematic_config_default(&config_copy) == SIDEREON_STATUS_OK);
    bool equal = false;
    assert(sidereon_ppp_kinematic_config_equal(config, config_copy, &equal) == SIDEREON_STATUS_OK);
    assert(equal);
    SidereonPppKinematicConfigInfo config_info;
    assert(sidereon_ppp_kinematic_config_info(config, &config_info) == SIDEREON_STATUS_OK);
    assert(config_info.motion.kind == SIDEREON_PPP_KINEMATIC_MOTION_MODEL_KIND_HOLD);
    assert(config_info.process_noise.position.kind ==
           SIDEREON_PPP_KINEMATIC_POSITION_PROCESS_NOISE_KIND_RANDOM_WALK);
    assert(config_info.process_noise.position.spectral_density == 1.0);
    assert(config_info.process_noise.clock_white_m2_s == 100.0);
    assert(config_info.process_noise.ztd_random_walk_m2_s == 1e-6);
    assert(config_info.process_noise.ambiguity_hold_m2_s == 0.0);
    assert(config_info.new_ambiguity_variance_m2 == 1e8);
    assert(config_info.weights.code == 1.0 && config_info.weights.phase == 100.0);
    assert(!config_info.tropo_enabled && !config_info.estimate_ztd);

    double position[3] = {1.0, 2.0, 3.0};
    double covariance[25] = {0};
    for (size_t i = 0; i < 5; i++) {
        covariance[i * 5 + i] = 1.0;
    }
    SidereonPppKinematicState *state = NULL;
    assert(sidereon_ppp_kinematic_state_new(position, 4.0, 0.5, NULL, 0, covariance, 25, &state) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_ppp_kinematic_config_set_initial_state(config, state) == SIDEREON_STATUS_OK);
    SidereonPppKinematicMotionModel motion = {
        .kind = SIDEREON_PPP_KINEMATIC_MOTION_MODEL_KIND_CONSTANT_VELOCITY,
        .velocity_m_s = {1.0, 2.0, 3.0},
    };
    SidereonPppKinematicProcessNoise noise = {
        .position = {.kind = SIDEREON_PPP_KINEMATIC_POSITION_PROCESS_NOISE_KIND_RANDOM_WALK,
                     .spectral_density = 2.0},
        .clock_white_m2_s = 3.0,
        .ztd_random_walk_m2_s = 4.0,
        .ambiguity_hold_m2_s = 5.0,
    };
    assert(sidereon_ppp_kinematic_config_set_dynamics(config, motion, noise, 7.0) ==
           SIDEREON_STATUS_OK);
    const char *active[] = {"ambiguity-id-with-no-fixed-buffer"};
    assert(sidereon_ppp_predict_kinematic_state(state, 2.0, active, 1, config) ==
           SIDEREON_STATUS_OK);
    SidereonPppKinematicStateInfo state_info;
    assert(sidereon_ppp_kinematic_state_info(state, &state_info) == SIDEREON_STATUS_OK);
    assert(state_info.position_m[0] == 3.0 && state_info.position_m[1] == 6.0 &&
           state_info.position_m[2] == 9.0);
    assert(state_info.clock_m == 4.0 && state_info.ztd_residual_m == 0.5);
    assert(state_info.ambiguity_count == 1 && state_info.dimension == 6);
    double ambiguity = -1.0;
    assert(sidereon_ppp_kinematic_state_ambiguity_m(state, 0, &ambiguity) == SIDEREON_STATUS_OK);
    assert(ambiguity == 0.0);
    uint8_t text[128];
    size_t written = 0;
    size_t required = 0;
    assert(sidereon_ppp_kinematic_state_ambiguity_id(state, 0, text, sizeof(text), &written,
                                                     &required) == SIDEREON_STATUS_OK);
    assert(required == strlen(active[0]) && memcmp(text, active[0], required) == 0);
    double predicted_covariance[36];
    assert(sidereon_ppp_kinematic_state_covariance(state, predicted_covariance, 36, &written,
                                                   &required) == SIDEREON_STATUS_OK);
    assert(written == 36 && required == 36);
    assert(predicted_covariance[0] == 5.0);
    assert(predicted_covariance[7] == 5.0);
    assert(predicted_covariance[14] == 5.0);
    assert(predicted_covariance[21] == 7.0);
    assert(predicted_covariance[28] == 9.0);
    assert(predicted_covariance[35] == 17.0);

    size_t sp3_length = 0;
    uint8_t *sp3_bytes = read_file(argv[1], &sp3_length);
    SidereonSp3 *sp3 = NULL;
    assert(sidereon_sp3_load(sp3_bytes, sp3_length, &sp3) == SIDEREON_STATUS_OK);
    free(sp3_bytes);
    SidereonPppObservation observation = {
        .sat_id = "G01",
        .ambiguity_id = "ambiguity-id-with-no-fixed-buffer",
        .code_m = 0.0,
        .phase_m = 0.0,
        .freq1_hz = 1575420000.0,
        .freq2_hz = 1227600000.0,
    };
    SidereonPppEpoch epoch = {
        .civil = {.year = 2020, .month = 6, .day = 25},
        .jd_whole = 2459025.0,
        .jd_fraction = 0.5,
        .t_rx_j2000_s = 646315200.0,
        .observations = &observation,
        .observation_count = 1,
    };
    SidereonPppKinematicUpdateSummary *summary = NULL;
    assert(sidereon_ppp_correct_kinematic_state(sp3, 0, &epoch, state, config, &summary) ==
           SIDEREON_STATUS_OK);
    SidereonPppKinematicUpdateSummaryInfo summary_info;
    assert(sidereon_ppp_kinematic_update_summary_info(summary, &summary_info) ==
           SIDEREON_STATUS_OK);
    assert(summary_info.innovation_rms_m == 0.0 && summary_info.used_satellite_count == 0);
    assert(summary_info.unplaced_observation_count == 1);
    SidereonPppUnplacedObservationV2 unplaced;
    assert(sidereon_ppp_kinematic_update_summary_unplaced_observation(summary, 0, &unplaced) ==
           SIDEREON_STATUS_OK);
    assert(unplaced.epoch_index == 0);
    assert(unplaced.reason == SIDEREON_PPP_UNPLACED_OBSERVATION_REASON_CODE_NOT_POSITIVE);
    sidereon_ppp_kinematic_update_summary_free(summary);

    SidereonPppKinematicResult *result = NULL;
    SidereonPppKinematicResult *result_copy = NULL;
    assert(sidereon_ppp_solve_kinematic_ppp(sp3, &epoch, 1, config, &result) == SIDEREON_STATUS_OK);
    assert(sidereon_ppp_solve_kinematic_ppp(sp3, &epoch, 1, config, &result_copy) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_ppp_kinematic_result_equal(result, result_copy, &equal) == SIDEREON_STATUS_OK);
    assert(equal);
    size_t epoch_count = 0;
    assert(sidereon_ppp_kinematic_result_epoch_count(result, &epoch_count) == SIDEREON_STATUS_OK);
    assert(epoch_count == 1);
    SidereonPppKinematicEpochSolutionInfo solution;
    assert(sidereon_ppp_kinematic_result_epoch_info(result, 0, &solution) == SIDEREON_STATUS_OK);
    assert(solution.status == SIDEREON_PPP_KINEMATIC_EPOCH_STATUS_PREDICTED_ONLY);
    assert(solution.position_m[0] == 1.0 && solution.position_m[1] == 2.0 &&
           solution.position_m[2] == 3.0);
    assert(solution.clock_m == 4.0 && solution.ztd_residual_m == 0.5);
    assert(solution.used_satellite_count == 0 && solution.unplaced_observation_count == 1);
    assert(sidereon_ppp_kinematic_result_unplaced_observation(result, 0, 0, &unplaced) ==
           SIDEREON_STATUS_OK);
    assert(unplaced.reason == SIDEREON_PPP_UNPLACED_OBSERVATION_REASON_CODE_NOT_POSITIVE);
    sidereon_ppp_kinematic_result_free(result_copy);
    sidereon_ppp_kinematic_result_free(result);

    SidereonPppKinematicMotionModel invalid_motion = motion;
    invalid_motion.kind = 99;
    assert(sidereon_ppp_kinematic_config_set_dynamics(config, invalid_motion, noise, 7.0) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    result = (SidereonPppKinematicResult *)(uintptr_t)1;
    assert(sidereon_ppp_solve_kinematic_ppp(sp3, NULL, 0, config, &result) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    assert(result == NULL);
    SidereonEngineErrorInfo error;
    assert(sidereon_last_engine_error_info(&error) == SIDEREON_STATUS_OK);
    assert(error.family == SIDEREON_ENGINE_ERROR_FAMILY_PPP_KINEMATIC);
    uint8_t payload[512];
    assert(sidereon_last_engine_error_payload(payload, sizeof(payload), &written, &required) ==
           SIDEREON_STATUS_OK);
    payload[written] = 0;
    assert(strstr((char *)payload, "\"kind\":\"invalid_input\"") != NULL);
    assert(strstr((char *)payload, "\"field\":\"kinematic PPP epochs\"") != NULL);
    assert(strstr((char *)payload, "\"reason\":\"must not be empty\"") != NULL);

    sidereon_sp3_free(sp3);
    sidereon_ppp_kinematic_state_free(state);
    sidereon_ppp_kinematic_config_free(config_copy);
    sidereon_ppp_kinematic_config_free(config);
    return 0;
}
