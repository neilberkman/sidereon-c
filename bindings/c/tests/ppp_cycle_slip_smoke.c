#include "sidereon.h"

#include <assert.h>
#include <math.h>
#include <stdint.h>
#include <string.h>

static int close_to(double left, double right, double tolerance) {
    return fabs(left - right) <= tolerance;
}

static SidereonPppCycleSlipObservation observation(double geometry_free_m, int64_t lli1) {
    const double speed_of_light = 299792458.0;
    const double f1 = 1575420000.0;
    const double f2 = 1227600000.0;
    const double l1_m = 23000000.0 + geometry_free_m;
    const double l2_m = 23000000.0;
    SidereonPppCycleSlipObservation value = {
        .satellite_id = "G01-long-lossless-satellite-id",
        .ambiguity_id = "G01-long-lossless-ambiguity-id",
        .p1_m = 23000000.0,
        .p2_m = 23000000.0,
        .phi1_cyc = l1_m / (speed_of_light / f1),
        .phi2_cyc = l2_m / (speed_of_light / f2),
        .f1_hz = f1,
        .f2_hz = f2,
        .has_lli1 = lli1 >= 0,
        .lli1 = lli1,
    };
    return value;
}

int main(void) {
    SidereonPppCycleSlipConfig config;
    assert(sidereon_ppp_cycle_slip_config_default(&config) == SIDEREON_STATUS_OK);
    assert(config.melbourne_wubbena_threshold_cycles == 4.0);
    assert(config.geometry_free_threshold_m == 0.05);
    assert(config.minimum_arc_length == 2);
    assert(config.running_statistic_k_factor == 4.0);
    assert(config.maximum_gap_s == 300.0);
    assert(sidereon_ppp_cycle_slip_config_validate(config) == SIDEREON_STATUS_OK);
    SidereonPppCycleSlipConvertedError converted;
    assert(sidereon_ppp_cycle_slip_error_from_config(
               SIDEREON_PPP_CYCLE_SLIP_CONFIG_ERROR_KIND_INVALID_MINIMUM_ARC_LENGTH, &converted) ==
           SIDEREON_STATUS_OK);
    assert(converted.kind == SIDEREON_PPP_CYCLE_SLIP_ERROR_KIND_INVALID_CONFIG);
    assert(converted.has_config_cause &&
           converted.config_cause ==
               SIDEREON_PPP_CYCLE_SLIP_CONFIG_ERROR_KIND_INVALID_MINIMUM_ARC_LENGTH);
    const SidereonPppCycleSlipCarrierPhaseErrorKind carrier_inputs[4] = {
        SIDEREON_PPP_CYCLE_SLIP_CARRIER_PHASE_ERROR_KIND_EQUAL_FREQUENCIES,
        SIDEREON_PPP_CYCLE_SLIP_CARRIER_PHASE_ERROR_KIND_INVALID_FREQUENCY,
        SIDEREON_PPP_CYCLE_SLIP_CARRIER_PHASE_ERROR_KIND_INVALID_OBSERVATION,
        SIDEREON_PPP_CYCLE_SLIP_CARRIER_PHASE_ERROR_KIND_INVALID_THRESHOLD,
    };
    const SidereonPppCycleSlipErrorKind carrier_outputs[4] = {
        SIDEREON_PPP_CYCLE_SLIP_ERROR_KIND_EQUAL_FREQUENCIES,
        SIDEREON_PPP_CYCLE_SLIP_ERROR_KIND_INVALID_FREQUENCY,
        SIDEREON_PPP_CYCLE_SLIP_ERROR_KIND_NON_FINITE_OBSERVATION,
        SIDEREON_PPP_CYCLE_SLIP_ERROR_KIND_INVALID_CONFIG,
    };
    for (size_t i = 0; i < 4; i++) {
        assert(sidereon_ppp_cycle_slip_error_from_carrier_phase(carrier_inputs[i], &converted) ==
               SIDEREON_STATUS_OK);
        assert(converted.kind == carrier_outputs[i]);
    }

    SidereonPppRunningMeanVariance *running = NULL;
    SidereonPppRunningMeanVariance *empty = NULL;
    assert(sidereon_ppp_running_mean_variance_new(&running) == SIDEREON_STATUS_OK);
    assert(sidereon_ppp_running_mean_variance_new(&empty) == SIDEREON_STATUS_OK);
    bool equal = false;
    assert(sidereon_ppp_running_mean_variance_equal(running, empty, &equal) == SIDEREON_STATUS_OK);
    assert(equal);
    assert(sidereon_ppp_running_mean_variance_push(running, 1.0) == SIDEREON_STATUS_OK);
    assert(sidereon_ppp_running_mean_variance_push(running, 2.0) == SIDEREON_STATUS_OK);
    assert(sidereon_ppp_running_mean_variance_push(running, 3.0) == SIDEREON_STATUS_OK);
    SidereonPppRunningMeanVarianceInfo running_info;
    assert(sidereon_ppp_running_mean_variance_info(running, &running_info) == SIDEREON_STATUS_OK);
    assert(running_info.sample_count == 3);
    assert(running_info.mean == 2.0);
    assert(running_info.variance == 1.0);
    assert(running_info.has_standard_deviation && running_info.standard_deviation == 1.0);
    assert(sidereon_ppp_running_mean_variance_equal(running, empty, &equal) == SIDEREON_STATUS_OK);
    assert(!equal);
    sidereon_ppp_running_mean_variance_free(empty);
    sidereon_ppp_running_mean_variance_free(running);

    SidereonPppCycleSlipObservation first = observation(0.0, -1);
    double geometry_free = 0.0;
    assert(sidereon_ppp_geometry_free_m(&first, &geometry_free) == SIDEREON_STATUS_OK);
    assert(close_to(geometry_free, 0.0, 1e-8));
    double mw = 0.0;
    assert(sidereon_ppp_melbourne_wubbena_cycles(&first, &mw) == SIDEREON_STATUS_OK);
    const double wide_lane_wavelength = 299792458.0 / (first.f1_hz - first.f2_hz);
    const double expected_mw = first.phi1_cyc - first.phi2_cyc - first.p1_m / wide_lane_wavelength;
    assert(close_to(mw, expected_mw, 1e-8));

    SidereonPppSatelliteCycleSlipState *state = NULL;
    SidereonPppSatelliteCycleSlipState *state_copy = NULL;
    assert(sidereon_ppp_satellite_cycle_slip_state_new(&state) == SIDEREON_STATUS_OK);
    assert(sidereon_ppp_satellite_cycle_slip_state_new(&state_copy) == SIDEREON_STATUS_OK);
    assert(sidereon_ppp_satellite_cycle_slip_state_equal(state, state_copy, &equal) ==
           SIDEREON_STATUS_OK);
    assert(equal);
    SidereonPppGeometryFreeUpdate gf_update;
    assert(sidereon_ppp_update_geometry_free(state, &first, true, 10.0, NULL, config, &gf_update) ==
           SIDEREON_STATUS_OK);
    assert(!gf_update.slip && !gf_update.reset);
    SidereonPppCycleSlipObservation jumped = observation(0.25, -1);
    assert(sidereon_ppp_update_geometry_free(state, &jumped, true, 20.0, NULL, config,
                                             &gf_update) == SIDEREON_STATUS_OK);
    assert(close_to(gf_update.geometry_free_m, 0.25, 1e-8));
    assert(gf_update.slip && !gf_update.reset);
    SidereonPppMelbourneWubbenaUpdate mw_update;
    assert(sidereon_ppp_update_melbourne_wubbena(state, &first, config, &mw_update) ==
           SIDEREON_STATUS_OK);
    assert(close_to(mw_update.melbourne_wubbena_cycles, expected_mw, 1e-8));
    SidereonPppSatelliteCycleSlipStateInfo state_info;
    assert(sidereon_ppp_satellite_cycle_slip_state_info(state, &state_info) == SIDEREON_STATUS_OK);
    assert(state_info.has_previous_epoch_time_s && state_info.previous_epoch_time_s == 20.0);
    assert(state_info.has_previous_geometry_free_m);
    assert(state_info.melbourne_wubbena.sample_count == 1);

    SidereonPppCycleSlipObservation observations[2] = {first, first};
    observations[1].has_lli1 = true;
    observations[1].lli1 = 1;
    SidereonPppCycleSlipEpoch epochs[2] = {
        {.has_gap_time_s = true,
         .gap_time_s = 0.0,
         .observations = &observations[0],
         .observation_count = 1},
        {.has_gap_time_s = true,
         .gap_time_s = 30.0,
         .observations = &observations[1],
         .observation_count = 1},
    };
    SidereonPppCycleSlipDetection *detection = NULL;
    SidereonPppCycleSlipDetection *detection_copy = NULL;
    assert(sidereon_ppp_detect_cycle_slips(epochs, 2, config, &detection) == SIDEREON_STATUS_OK);
    assert(sidereon_ppp_detect_cycle_slips(epochs, 2, config, &detection_copy) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_ppp_cycle_slip_detection_equal(detection, detection_copy, &equal) ==
           SIDEREON_STATUS_OK);
    assert(equal);
    size_t epoch_count = 0;
    assert(sidereon_ppp_cycle_slip_detection_epoch_count(detection, &epoch_count) ==
           SIDEREON_STATUS_OK);
    assert(epoch_count == 2);
    SidereonPppCycleSlipFlagObservationInfo flag;
    assert(sidereon_ppp_cycle_slip_detection_observation_info(detection, 1, 0, &flag) ==
           SIDEREON_STATUS_OK);
    assert(flag.slip && flag.reason_count == 1);
    SidereonPppCycleSlipReason reason;
    assert(sidereon_ppp_cycle_slip_detection_observation_reason(detection, 1, 0, 0, &reason) ==
           SIDEREON_STATUS_OK);
    assert(reason == SIDEREON_PPP_CYCLE_SLIP_REASON_LLI);
    uint8_t id[64];
    size_t written = 0;
    size_t required = 0;
    assert(sidereon_ppp_cycle_slip_detection_observation_satellite_id(
               detection, 1, 0, id, sizeof(id), &written, &required) == SIDEREON_STATUS_OK);
    assert(required == strlen("G01-long-lossless-satellite-id"));
    assert(memcmp(id, "G01-long-lossless-satellite-id", required) == 0);
    sidereon_ppp_cycle_slip_detection_free(detection_copy);
    sidereon_ppp_cycle_slip_detection_free(detection);

    config.minimum_arc_length = 0;
    assert(sidereon_ppp_cycle_slip_config_validate(config) == SIDEREON_STATUS_INVALID_ARGUMENT);
    SidereonEngineErrorInfo error_info;
    assert(sidereon_last_engine_error_info(&error_info) == SIDEREON_STATUS_OK);
    assert(error_info.family == SIDEREON_ENGINE_ERROR_FAMILY_PPP_CYCLE_SLIP);
    uint8_t payload[512];
    assert(sidereon_last_engine_error_payload(payload, sizeof(payload), &written, &required) ==
           SIDEREON_STATUS_OK);
    payload[written] = 0;
    assert(strstr((char *)payload, "\"kind\":\"invalid_config\"") != NULL);
    assert(strstr((char *)payload, "\"kind\":\"invalid_minimum_arc_length\"") != NULL);
    assert(strstr((char *)payload, "minimum arc length must be nonzero") != NULL);

    config.minimum_arc_length = 2;
    assert(sidereon_ppp_update_geometry_free(state, &first, true, 5.0, NULL, config, &gf_update) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    assert(sidereon_last_engine_error_info(&error_info) == SIDEREON_STATUS_OK);
    assert(error_info.family == SIDEREON_ENGINE_ERROR_FAMILY_PPP_CYCLE_SLIP);
    assert(sidereon_last_engine_error_payload(payload, sizeof(payload), &written, &required) ==
           SIDEREON_STATUS_OK);
    payload[written] = 0;
    assert(strstr((char *)payload, "\"kind\":\"epochs_not_ordered\"") != NULL);
    assert(gf_update.geometry_free_m == 0.0 && !gf_update.slip && !gf_update.reset);

    assert(sidereon_ppp_satellite_cycle_slip_state_reset_arc(state) == SIDEREON_STATUS_OK);
    assert(sidereon_ppp_satellite_cycle_slip_state_info(state, &state_info) == SIDEREON_STATUS_OK);
    assert(!state_info.has_previous_epoch_time_s && state_info.melbourne_wubbena.sample_count == 0);
    sidereon_ppp_satellite_cycle_slip_state_free(state_copy);
    sidereon_ppp_satellite_cycle_slip_state_free(state);
    return 0;
}
