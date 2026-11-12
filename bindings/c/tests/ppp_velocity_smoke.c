#include "sidereon.h"

#include <assert.h>
#include <math.h>
#include <stddef.h>

int main(void) {
    struct SidereonPppVelocityConfig config;
    assert(sidereon_ppp_velocity_config_init(&config) == SIDEREON_STATUS_OK);
    assert(sidereon_ppp_velocity_config_init(NULL) == SIDEREON_STATUS_NULL_POINTER);
    assert(config.minimum_observations == 4);
    assert(!config.has_robust);
    struct SidereonPppVelocityRobustConfig robust;
    assert(sidereon_ppp_velocity_robust_config_init(&robust) == SIDEREON_STATUS_OK);
    assert(sidereon_ppp_velocity_robust_config_init(NULL) == SIDEREON_STATUS_NULL_POINTER);
    assert(robust.huber_k == 1.345);
    bool equal = false;
    assert(sidereon_ppp_velocity_config_equal(&config, &config, &equal) == SIDEREON_STATUS_OK);
    assert(equal);
    struct SidereonPppVelocityConfig unequal_config = config;
    unequal_config.minimum_observations = 5;
    assert(sidereon_ppp_velocity_config_equal(&config, &unequal_config, &equal) ==
           SIDEREON_STATUS_OK);
    assert(!equal);
    assert(sidereon_ppp_velocity_config_equal(NULL, &config, &equal) ==
           SIDEREON_STATUS_NULL_POINTER);
    assert(sidereon_ppp_velocity_robust_config_equal(&robust, &robust, &equal) ==
           SIDEREON_STATUS_OK);
    assert(equal);
    struct SidereonPppVelocityRobustConfig unequal_robust = robust;
    unequal_robust.max_outer++;
    assert(sidereon_ppp_velocity_robust_config_equal(&robust, &unequal_robust, &equal) ==
           SIDEREON_STATUS_OK);
    assert(!equal);
    assert(sidereon_ppp_velocity_robust_config_equal(NULL, &robust, &equal) ==
           SIDEREON_STATUS_NULL_POINTER);

    const double origin[3] = {0.0, 0.0, 0.0};
    struct SidereonPppReceiverVelocityState receiver;
    assert(sidereon_ppp_receiver_velocity_state_stationary_at(origin, &receiver) ==
           SIDEREON_STATUS_OK);
    assert(receiver.velocity_m_s[0] == 0.0);
    assert(receiver.clock_drift_m_s == 0.0);
    struct SidereonPppReceiverVelocityState stationary = receiver;
    assert(sidereon_ppp_receiver_velocity_state_equal(&receiver, &stationary, &equal) ==
           SIDEREON_STATUS_OK);
    assert(equal);
    stationary.clock_drift_m_s = 1.0;
    assert(sidereon_ppp_receiver_velocity_state_equal(&receiver, &stationary, &equal) ==
           SIDEREON_STATUS_OK);
    assert(!equal);
    assert(sidereon_ppp_receiver_velocity_state_equal(NULL, &stationary, &equal) ==
           SIDEREON_STATUS_NULL_POINTER);

    struct SidereonPppVelocityObservation prediction_observation = {
        .sat_id = "G01",
        .satellite_position_m = {20200000.0, 0.0, 0.0},
        .satellite_velocity_m_s = {10.0, 2.0, 3.0},
        .measured_range_rate_m_s = 0.0,
        .sigma_m_s = 0.1,
        .satellite_clock_drift_m_s = 0.1,
    };
    receiver.velocity_m_s[0] = 3.0;
    receiver.velocity_m_s[1] = -2.0;
    receiver.velocity_m_s[2] = 0.5;
    receiver.clock_drift_m_s = 0.25;
    struct SidereonPppRangeRatePrediction prediction;
    assert(sidereon_ppp_velocity_observation_equal(&prediction_observation, &prediction_observation,
                                                   &equal) == SIDEREON_STATUS_OK);
    assert(equal);
    struct SidereonPppVelocityObservation unequal_observation = prediction_observation;
    unequal_observation.sigma_m_s = 0.2;
    assert(sidereon_ppp_velocity_observation_equal(&prediction_observation, &unequal_observation,
                                                   &equal) == SIDEREON_STATUS_OK);
    assert(!equal);
    assert(sidereon_ppp_velocity_observation_equal(NULL, &unequal_observation, &equal) ==
           SIDEREON_STATUS_NULL_POINTER);
    assert(sidereon_ppp_predict_range_rate_m_s(&prediction_observation, &receiver, &prediction) ==
           SIDEREON_STATUS_OK);
    assert(prediction.los_unit[0] == 1.0);
    assert(prediction.los_unit[1] == 0.0);
    assert(prediction.range_rate_m_s == 7.15);
    assert(sidereon_ppp_predict_range_rate_m_s(NULL, &receiver, &prediction) ==
           SIDEREON_STATUS_NULL_POINTER);
    assert(sidereon_ppp_range_rate_prediction_equal(&prediction, &prediction, &equal) ==
           SIDEREON_STATUS_OK);
    assert(equal);
    struct SidereonPppRangeRatePrediction unequal_prediction = prediction;
    unequal_prediction.range_rate_m_s = 8.0;
    assert(sidereon_ppp_range_rate_prediction_equal(&prediction, &unequal_prediction, &equal) ==
           SIDEREON_STATUS_OK);
    assert(!equal);
    assert(sidereon_ppp_range_rate_prediction_equal(NULL, &prediction, &equal) ==
           SIDEREON_STATUS_NULL_POINTER);

    const double positions[5][3] = {{20200000.0, 0.0, 0.0},
                                    {0.0, 21100000.0, 0.0},
                                    {0.0, 0.0, 22300000.0},
                                    {-20900000.0, 19700000.0, 21500000.0},
                                    {18600000.0, -20400000.0, 23100000.0}};
    const double measured[5] = {-2.75, 2.25, -0.25, 2.796208544343477, -2.754479649598877};
    const char *ids[5] = {"G01", "G02", "G03", "G04", "G05"};
    struct SidereonPppVelocityObservation observations[5];
    for (size_t i = 0; i < 5; ++i) {
        observations[i] = (struct SidereonPppVelocityObservation){
            .sat_id = ids[i],
            .satellite_position_m = {positions[i][0], positions[i][1], positions[i][2]},
            .satellite_velocity_m_s = {0.0, 0.0, 0.0},
            .measured_range_rate_m_s = measured[i],
            .sigma_m_s = 0.1,
            .satellite_clock_drift_m_s = 0.0,
        };
    }
    struct SidereonPppVelocitySolution *solution = NULL;
    assert(sidereon_ppp_solve_velocity_least_squares(observations, 5, origin, &config, &solution) ==
           SIDEREON_STATUS_OK);
    double velocity[3];
    double clock_drift;
    double residual_rms;
    size_t robust_iterations;
    bool has_robust_scale;
    double robust_scale;
    assert(sidereon_ppp_velocity_solution_values(solution, velocity, &clock_drift, &residual_rms,
                                                 &robust_iterations, &has_robust_scale,
                                                 &robust_scale) == SIDEREON_STATUS_OK);
    assert(fabs(velocity[0] - 3.0) < 1e-12);
    assert(fabs(velocity[1] + 2.0) < 1e-12);
    assert(fabs(velocity[2] - 0.5) < 1e-12);
    assert(fabs(clock_drift - 0.25) < 1e-12);
    assert(residual_rms < 1e-12);
    assert(robust_iterations == 0);
    assert(!has_robust_scale);
    assert(robust_scale == 0.0);
    for (size_t axis = 0; axis < 3; ++axis) {
        velocity[axis] = 9.0;
    }
    clock_drift = 9.0;
    residual_rms = 9.0;
    robust_iterations = 9;
    has_robust_scale = true;
    assert(sidereon_ppp_velocity_solution_values(solution, velocity, &clock_drift, &residual_rms,
                                                 &robust_iterations, &has_robust_scale,
                                                 NULL) == SIDEREON_STATUS_NULL_POINTER);
    assert(velocity[0] == 0.0 && velocity[1] == 0.0 && velocity[2] == 0.0);
    assert(clock_drift == 0.0 && residual_rms == 0.0);
    assert(robust_iterations == 0 && !has_robust_scale);
    struct SidereonPppVelocitySolution *solution2 = NULL;
    assert(sidereon_ppp_solve_velocity_least_squares(observations, 5, origin, &config,
                                                     &solution2) == SIDEREON_STATUS_OK);
    assert(sidereon_ppp_velocity_solution_equal(solution, solution2, &equal) == SIDEREON_STATUS_OK);
    assert(equal);
    assert(sidereon_ppp_velocity_solution_equal(NULL, solution2, &equal) ==
           SIDEREON_STATUS_NULL_POINTER);
    size_t written = 99;
    size_t required = 99;
    assert(sidereon_ppp_velocity_solution_used_sats(solution, NULL, 0, &written, &required) ==
           SIDEREON_STATUS_OK);
    assert(written == 0 && required == 5);
    struct SidereonSatelliteToken used[5];
    assert(sidereon_ppp_velocity_solution_used_sats(solution, used, 5, &written, &required) ==
           SIDEREON_STATUS_OK);
    assert(written == 5 && required == 5 && used[0].bytes[0] == 'G' && used[0].bytes[1] == '0' &&
           used[0].bytes[2] == '1');
    assert(sidereon_ppp_velocity_solution_used_sats(NULL, used, 5, &written, &required) ==
           SIDEREON_STATUS_NULL_POINTER);
    assert(sidereon_ppp_velocity_solution_downweighted_sats(solution, NULL, 0, &written,
                                                            &required) == SIDEREON_STATUS_OK);
    assert(written == 0 && required == 0);
    assert(sidereon_ppp_velocity_solution_downweighted_sats(NULL, NULL, 0, &written, &required) ==
           SIDEREON_STATUS_NULL_POINTER);
    sidereon_ppp_velocity_solution_free(solution2);
    sidereon_ppp_velocity_solution_free(solution);

    solution = NULL;
    assert(sidereon_ppp_solve_velocity_least_squares(observations, 3, origin, &config, &solution) ==
           SIDEREON_STATUS_SOLVE);
    assert(solution == NULL);
    struct SidereonEngineErrorInfo error_info;
    assert(sidereon_last_engine_error_info(&error_info) == SIDEREON_STATUS_OK);
    assert(error_info.family == SIDEREON_ENGINE_ERROR_FAMILY_PPP_VELOCITY);
    assert(sidereon_ppp_solve_velocity_least_squares(NULL, 1, origin, &config, &solution) ==
           SIDEREON_STATUS_NULL_POINTER);
    return 0;
}
