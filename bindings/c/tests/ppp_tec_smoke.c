#include "sidereon.h"

#include <assert.h>
#include <math.h>
#include <stddef.h>
#include <string.h>

static int close_to(double left, double right, double tolerance) {
    return fabs(left - right) <= tolerance;
}

int main(void) {
    const double pi = 3.14159265358979323846264338327950288;
    SidereonTecConfig config;
    assert(sidereon_tec_config_default(&config) == SIDEREON_STATUS_OK);
    assert(config.shell_height_m == 350000.0);
    assert(config.earth_radius_m == 6371000.0);
    assert(sidereon_tec_config_validate(config) == SIDEREON_STATUS_OK);

    double mapping = 0.0;
    assert(sidereon_tec_thin_shell_mapping_function(pi / 3.0, config, &mapping) ==
           SIDEREON_STATUS_OK);
    assert(close_to(mapping, 1.1356600608194427, 1e-14));

    SidereonTecIonosphericPiercePoint point;
    assert(sidereon_tec_ionospheric_pierce_point(0.0, 0.0, pi / 3.0, pi / 2.0, config, &point) ==
           SIDEREON_STATUS_OK);
    assert(close_to(point.latitude_rad, 0.0, 1e-15));
    assert(close_to(point.longitude_rad, 0.029813697146944318, 1e-14));
    assert(point.shell_height_m == 350000.0);

    const double f1_hz = 1575420000.0;
    const double f2_hz = 1227600000.0;
    const double denominator =
        SIDEREON_TEC_GROUP_DELAY_COEFFICIENT * (1.0 / (f1_hz * f1_hz) - 1.0 / (f2_hz * f2_hz));
    const double wavelength1_m = 299792458.0 / f1_hz;
    SidereonPppDualFrequencyObservation raw = {
        .satellite_id = "satellite-id-with-no-fixed-output-buffer",
        .ambiguity_id = "arc-id-with-no-fixed-output-buffer",
        .p1_m = 0.0,
        .p2_m = -denominator * 20.0,
        .phi1_cyc = (-denominator * 25.0) / wavelength1_m,
        .phi2_cyc = 0.0,
        .f1_hz = f1_hz,
        .f2_hz = f2_hz,
    };
    double scalar = 0.0;
    assert(sidereon_tec_code_geometry_free_m(&raw, &scalar) == SIDEREON_STATUS_OK);
    assert(close_to(scalar, denominator * 20.0, 1e-15));
    assert(sidereon_tec_phase_geometry_free_m(&raw, &scalar) == SIDEREON_STATUS_OK);
    assert(close_to(scalar, -denominator * 25.0, 1e-15));
    assert(sidereon_tec_slant_from_code_geometry_free_m(denominator * 20.0, f1_hz, f2_hz,
                                                        &scalar) == SIDEREON_STATUS_OK);
    assert(close_to(scalar, 20.0, 1e-12));

    SidereonTecCodeSlantEstimate *code_estimate = NULL;
    assert(sidereon_tec_estimate_code_slant(&raw, &code_estimate) == SIDEREON_STATUS_OK);
    SidereonTecCodeSlantEstimateInfo code_info;
    assert(sidereon_tec_code_slant_estimate_info(code_estimate, &code_info) == SIDEREON_STATUS_OK);
    assert(close_to(code_info.slant_tec_tecu, 20.0, 1e-12));
    size_t written = 0;
    size_t required = 0;
    unsigned char identifier[64] = {0};
    assert(sidereon_tec_code_slant_estimate_satellite_id(code_estimate, identifier,
                                                         sizeof(identifier), &written,
                                                         &required) == SIDEREON_STATUS_OK);
    assert(required == strlen("satellite-id-with-no-fixed-output-buffer"));
    SidereonTecCodeSlantEstimate *same_code = NULL;
    assert(sidereon_tec_estimate_code_slant(&raw, &same_code) == SIDEREON_STATUS_OK);
    bool equal = false;
    assert(sidereon_tec_code_slant_estimate_equal(code_estimate, same_code, &equal) ==
           SIDEREON_STATUS_OK);
    assert(equal);
    sidereon_tec_code_slant_estimate_free(same_code);
    sidereon_tec_code_slant_estimate_free(code_estimate);

    SidereonTecObservation observations[2] = {
        {.observation = raw, .elevation_rad = pi / 3.0, .azimuth_rad = pi / 2.0},
        {.observation = raw, .elevation_rad = pi / 3.0, .azimuth_rad = pi / 2.0},
    };
    SidereonTecEpoch epochs[2] = {
        {.time_s = 0.0,
         .receiver_latitude_rad = 0.0,
         .receiver_longitude_rad = 0.0,
         .observations = &observations[0],
         .observation_count = 1},
        {.time_s = 30.0,
         .receiver_latitude_rad = 0.0,
         .receiver_longitude_rad = 0.0,
         .observations = &observations[1],
         .observation_count = 1},
    };
    assert(sidereon_tec_epoch_equal(&epochs[0], &epochs[0], &equal) == SIDEREON_STATUS_OK);
    assert(equal);
    SidereonTecEstimate *estimate = NULL;
    assert(sidereon_tec_estimate(epochs, 2, config, &estimate) == SIDEREON_STATUS_OK);
    size_t arc_count = 0;
    assert(sidereon_tec_estimate_arc_count(estimate, &arc_count) == SIDEREON_STATUS_OK);
    assert(arc_count == 1);
    double arc_bias = 0.0;
    size_t sample_count = 0;
    assert(sidereon_tec_estimate_arc_info(estimate, 0, &arc_bias, &sample_count) ==
           SIDEREON_STATUS_OK);
    assert(close_to(arc_bias, 5.0, 1e-12));
    assert(sample_count == 2);
    SidereonTecEstimateSample estimate_sample;
    assert(sidereon_tec_estimate_sample(estimate, 0, 0, &estimate_sample) == SIDEREON_STATUS_OK);
    assert(close_to(estimate_sample.leveled_slant_tec_tecu, 20.0, 1e-12));
    assert(close_to(estimate_sample.vertical_tec_tecu, 20.0 / mapping, 1e-12));
    sidereon_tec_estimate_free(estimate);

    SidereonTecLevelingSample samples[2] = {
        {.code_slant_tec_tecu = 20.0, .phase_slant_tec_tecu = 25.0, .elevation_rad = pi / 3.0},
        {.code_slant_tec_tecu = 22.0, .phase_slant_tec_tecu = 27.0, .elevation_rad = pi / 3.0},
    };
    SidereonTecLevelingResult *result = NULL;
    assert(sidereon_tec_level_slant_arc(samples, 2, config, &result) == SIDEREON_STATUS_OK);
    assert(result != NULL);
    double bias = 0.0;
    size_t count = 0;
    assert(sidereon_tec_leveling_result_info(result, &bias, &count) == SIDEREON_STATUS_OK);
    assert(bias == 5.0);
    assert(count == 2);
    SidereonTecLeveledSample output[2];
    assert(sidereon_tec_leveling_result_samples(result, output, 2, &written, &required) ==
           SIDEREON_STATUS_OK);
    assert(written == 2 && required == 2);
    assert(output[0].leveled_slant_tec_tecu == 20.0);

    SidereonTecLevelingResult *same = NULL;
    assert(sidereon_tec_level_slant_arc(samples, 2, config, &same) == SIDEREON_STATUS_OK);
    assert(sidereon_tec_leveling_result_equal(result, same, &equal) == SIDEREON_STATUS_OK);
    assert(equal);
    sidereon_tec_leveling_result_free(same);
    sidereon_tec_leveling_result_free(result);

    config.shell_height_m = 0.0;
    assert(sidereon_tec_config_validate(config) == SIDEREON_STATUS_INVALID_ARGUMENT);
    SidereonEngineErrorInfo error;
    assert(sidereon_last_engine_error_info(&error) == SIDEREON_STATUS_OK);
    assert(error.family == SIDEREON_ENGINE_ERROR_FAMILY_TEC);
    unsigned char payload[512] = {0};
    written = 0;
    required = 0;
    assert(sidereon_last_engine_error_payload(payload, sizeof(payload), &written, &required) ==
           SIDEREON_STATUS_OK);
    assert(strstr((const char *)payload, "\"kind\":\"invalid_shell_height\"") != NULL);
    unsigned char error_text[128];
    assert(sidereon_tec_error_text(SIDEREON_TEC_ERROR_KIND_INVALID_SHELL_HEIGHT, error_text,
                                   sizeof(error_text), &written, &required) == SIDEREON_STATUS_OK);
    assert(written == strlen("TEC shell height must be positive and finite"));
    return 0;
}
