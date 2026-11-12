#include "sidereon.h"

#include <assert.h>
#include <math.h>
#include <stdint.h>
#include <string.h>

static int close_to(double left, double right, double tolerance) {
    return fabs(left - right) <= tolerance;
}

int main(void) {
    const double c = 299792458.0;
    const double f1 = 1575420000.0;
    const double f2 = 1227600000.0;
    SidereonPppCycleSlipObservation observations[2] = {
        {.satellite_id = "G01-lossless-prep-satellite",
         .ambiguity_id = "input-ambiguity-is-replaced-by-bare-satellite",
         .p1_m = 0.0,
         .p2_m = 0.0,
         .phi1_cyc = 5.0,
         .phi2_cyc = 0.0,
         .f1_hz = f1,
         .f2_hz = f2},
        {.satellite_id = "G01-lossless-prep-satellite",
         .ambiguity_id = "input-ambiguity-is-replaced-by-bare-satellite",
         .p1_m = 0.0,
         .p2_m = 0.0,
         .phi1_cyc = 5.0,
         .phi2_cyc = 0.0,
         .f1_hz = f1,
         .f2_hz = f2},
    };
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
    bool equal = false;
    assert(sidereon_ppp_dual_frequency_observation_equal(&observations[0], &observations[1],
                                                         &equal) == SIDEREON_STATUS_OK);
    assert(equal);
    assert(sidereon_ppp_dual_frequency_epoch_equal(&epochs[0], &epochs[0], &equal) ==
           SIDEREON_STATUS_OK);
    assert(equal);

    SidereonPppWideLanePrepOptions options;
    assert(sidereon_ppp_wide_lane_prep_options_new(2, 0.01, &options) == SIDEREON_STATUS_OK);
    assert(options.min_epochs == 2 && options.tolerance_cycles == 0.01);
    SidereonCycleSlipOptions slip = {
        .gf_threshold_m = 0.05, .mw_threshold_cycles = 4.0, .min_arc_gap_s = 1000.0};
    SidereonPppWideLanePrepResult *result = NULL;
    SidereonPppWideLanePrepResult *copy = NULL;
    assert(sidereon_ppp_prepare_widelane_fixed_epochs(epochs, 2, options,
                                                      SIDEREON_PPP_PREP_CYCLE_SLIP_POLICY_ERROR,
                                                      slip, &result) == SIDEREON_STATUS_OK);
    assert(sidereon_ppp_prepare_widelane_fixed_epochs(epochs, 2, options,
                                                      SIDEREON_PPP_PREP_CYCLE_SLIP_POLICY_ERROR,
                                                      slip, &copy) == SIDEREON_STATUS_OK);
    assert(sidereon_ppp_wide_lane_prep_result_equal(result, copy, &equal) == SIDEREON_STATUS_OK);
    assert(equal);

    SidereonPppWideLanePrepResultInfo info;
    assert(sidereon_ppp_wide_lane_prep_result_info(result, &info) == SIDEREON_STATUS_OK);
    assert(info.epoch_count == 2);
    assert(info.wavelength_count == 1 && info.offset_count == 1 && info.wide_lane_count == 1);
    assert(info.dropped_satellite_count == 0 && info.split_arc_count == 0);
    SidereonPppPreparedFloatEpochInfo epoch_info;
    assert(sidereon_ppp_wide_lane_prep_result_epoch_info(result, 1, &epoch_info) ==
           SIDEREON_STATUS_OK);
    assert(epoch_info.epoch_index == 1 && epoch_info.observation_count == 1);
    SidereonPppPreparedFloatObservationInfo observation_info;
    assert(sidereon_ppp_wide_lane_prep_result_observation_info(result, 0, 0, &observation_info) ==
           SIDEREON_STATUS_OK);
    const double expected_phase = f1 * c * 5.0 / (f1 * f1 - f2 * f2);
    assert(observation_info.code_m == 0.0);
    assert(close_to(observation_info.phase_m, expected_phase, 1e-15));
    double wavelength = 0.0;
    double offset = 0.0;
    int64_t wide_lane = 0;
    assert(sidereon_ppp_wide_lane_prep_result_wavelength_m(result, 0, &wavelength) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_ppp_wide_lane_prep_result_offset_m(result, 0, &offset) == SIDEREON_STATUS_OK);
    assert(sidereon_ppp_wide_lane_prep_result_wide_lane_cycles(result, 0, &wide_lane) ==
           SIDEREON_STATUS_OK);
    assert(close_to(wavelength, c / (f1 + f2), 1e-15));
    assert(close_to(offset, (f1 * f1 / (f1 * f1 - f2 * f2) - 1.0) * (c / f2) * 5.0, 1e-15));
    assert(wide_lane == 5);
    uint8_t text[128];
    size_t written = 0;
    size_t required = 0;
    assert(sidereon_ppp_wide_lane_prep_result_observation_satellite_id(
               result, 0, 0, text, sizeof(text), &written, &required) == SIDEREON_STATUS_OK);
    assert(required == strlen("G01-lossless-prep-satellite"));
    assert(memcmp(text, "G01-lossless-prep-satellite", required) == 0);
    assert(sidereon_ppp_wide_lane_prep_result_observation_ambiguity_id(
               result, 0, 0, text, sizeof(text), &written, &required) == SIDEREON_STATUS_OK);
    assert(required == strlen("G01-lossless-prep-satellite"));
    sidereon_ppp_wide_lane_prep_result_free(copy);
    sidereon_ppp_wide_lane_prep_result_free(result);

    observations[1].has_lli1 = true;
    observations[1].lli1 = 1;
    SidereonPppFloatCycleSlipObservation float_observations[2] = {
        {.satellite_id = "G01-lossless-prep-satellite",
         .ambiguity_id = "original-arc",
         .raw = &observations[0]},
        {.satellite_id = "G01-lossless-prep-satellite",
         .ambiguity_id = "original-arc",
         .raw = &observations[1]},
    };
    SidereonPppFloatCycleSlipEpoch float_epochs[2] = {
        {.has_gap_time_s = true,
         .gap_time_s = 0.0,
         .observations = &float_observations[0],
         .observation_count = 1},
        {.has_gap_time_s = true,
         .gap_time_s = 30.0,
         .observations = &float_observations[1],
         .observation_count = 1},
    };
    assert(sidereon_ppp_float_cycle_slip_observation_equal(
               &float_observations[0], &float_observations[0], &equal) == SIDEREON_STATUS_OK);
    assert(equal);
    assert(sidereon_ppp_float_cycle_slip_epoch_equal(&float_epochs[0], &float_epochs[0], &equal) ==
           SIDEREON_STATUS_OK);
    assert(equal);
    SidereonPppFloatCycleSlipTaggedResult *tagged = NULL;
    SidereonPppFloatCycleSlipTaggedResult *tagged_copy = NULL;
    assert(sidereon_ppp_split_float_cycle_slip_epochs(float_epochs, 2, slip, &tagged) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_ppp_split_float_cycle_slip_epochs(float_epochs, 2, slip, &tagged_copy) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_ppp_float_cycle_slip_tagged_result_equal(tagged, tagged_copy, &equal) ==
           SIDEREON_STATUS_OK);
    assert(equal);
    size_t tagged_count = 0;
    assert(sidereon_ppp_float_cycle_slip_tagged_result_epoch_count(tagged, &tagged_count) ==
           SIDEREON_STATUS_OK);
    assert(tagged_count == 2);
    assert(sidereon_ppp_float_cycle_slip_tagged_result_ambiguity_id(
               tagged, 1, 0, text, sizeof(text), &written, &required) == SIDEREON_STATUS_OK);
    assert(required == strlen("G01-lossless-prep-satellite#2"));
    assert(memcmp(text, "G01-lossless-prep-satellite#2", required) == 0);
    sidereon_ppp_float_cycle_slip_tagged_result_free(tagged_copy);
    sidereon_ppp_float_cycle_slip_tagged_result_free(tagged);

    observations[1].has_lli1 = false;
    options.min_epochs = 3;
    result = (SidereonPppWideLanePrepResult *)(uintptr_t)1;
    assert(sidereon_ppp_prepare_widelane_fixed_epochs(
               epochs, 2, options, SIDEREON_PPP_PREP_CYCLE_SLIP_POLICY_ERROR, slip, &result) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    assert(result == NULL);
    SidereonEngineErrorInfo error;
    assert(sidereon_last_engine_error_info(&error) == SIDEREON_STATUS_OK);
    assert(error.family == SIDEREON_ENGINE_ERROR_FAMILY_PPP_PREP);
    uint8_t payload[512];
    assert(sidereon_last_engine_error_payload(payload, sizeof(payload), &written, &required) ==
           SIDEREON_STATUS_OK);
    payload[written] = 0;
    assert(strstr((char *)payload, "\"kind\":\"too_few_wide_lane_epochs\"") != NULL);
    assert(strstr((char *)payload, "\"count\":2") != NULL);
    assert(strstr((char *)payload, "\"minimum\":3") != NULL);
    assert(strstr((char *)payload, "G01-lossless-prep-satellite") != NULL);
    result = (SidereonPppWideLanePrepResult *)(uintptr_t)1;
    assert(sidereon_ppp_prepare_widelane_fixed_epochs(epochs, 2, options, 99, slip, &result) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    assert(result == NULL);
    return 0;
}
