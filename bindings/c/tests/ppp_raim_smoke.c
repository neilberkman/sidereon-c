#include "sidereon.h"

#include <assert.h>
#include <math.h>
#include <stddef.h>
#include <stdlib.h>
#include <string.h>

static int close_to(double left, double right, double tolerance) {
    return fabs(left - right) <= tolerance;
}

int main(void) {
    SidereonPppRaimConfig config;
    assert(sidereon_ppp_raim_config_default(&config) == SIDEREON_STATUS_OK);
    assert(config.false_alarm_probability == 1e-3);
    assert(config.missed_detection_probability == 1e-3);
    assert(config.measurement_sigma_m == 1.0);
    assert(!config.has_chi_square_threshold);

    config.has_chi_square_threshold = true;
    config.chi_square_threshold = 100.0;
    const SidereonPppRaimResidual rows[2] = {
        {.epoch_index = 0,
         .satellite_id = "G01",
         .ambiguity_id = "arc-one-without-a-fixed-buffer-limit",
         .code_m = 1.0,
         .phase_m = 2.0,
         .code_weight = 2.0,
         .phase_weight = 0.5},
        {.epoch_index = 0,
         .satellite_id = "G02",
         .ambiguity_id = "G02",
         .code_m = -3.0,
         .phase_m = 4.0,
         .code_weight = 1.0,
         .phase_weight = 0.25},
    };
    SidereonPppRaimResult *result = NULL;
    assert(sidereon_ppp_raim_global_test(rows, 2, 1, config, &result) == SIDEREON_STATUS_OK);
    SidereonPppRaimResultInfo info;
    assert(sidereon_ppp_raim_result_info(result, &info) == SIDEREON_STATUS_OK);
    assert(info.status == SIDEREON_PPP_RAIM_STATUS_PASSED);
    assert(!info.detected);
    assert(info.test_statistic == 15.0);
    assert(info.threshold == 100.0);
    assert(info.redundancy == 3);
    assert(info.satellite_statistic_count == 0);

    SidereonPppRaimResult *equal_result = NULL;
    assert(sidereon_ppp_raim_global_test(rows, 2, 1, config, &equal_result) == SIDEREON_STATUS_OK);
    bool equal = false;
    assert(sidereon_ppp_raim_result_equal(result, equal_result, &equal) == SIDEREON_STATUS_OK);
    assert(equal);
    sidereon_ppp_raim_result_free(equal_result);
    sidereon_ppp_raim_result_free(result);

    const double inverse_sqrt_three = 0.57735026918962576451;
    const SidereonPppRaimGeometryRow geometry[5] = {
        {.satellite_id = "G01", .line_of_sight = {1.0, 0.0, 0.0}},
        {.satellite_id = "G02", .line_of_sight = {-1.0, 0.0, 0.0}},
        {.satellite_id = "G03", .line_of_sight = {0.0, 1.0, 0.0}},
        {.satellite_id = "G04", .line_of_sight = {0.0, 0.0, 1.0}},
        {.satellite_id = "G05",
         .line_of_sight = {inverse_sqrt_three, inverse_sqrt_three, inverse_sqrt_three}},
    };
    SidereonPppRaimGeometryRow geometry_copy = geometry[0];
    bool geometry_equal = false;
    assert(sidereon_ppp_raim_geometry_row_equal(&geometry[0], &geometry_copy, &geometry_equal) ==
           SIDEREON_STATUS_OK);
    assert(geometry_equal);
    geometry_copy.line_of_sight.e_x = 0.5;
    assert(sidereon_ppp_raim_geometry_row_equal(&geometry[0], &geometry_copy, &geometry_equal) ==
           SIDEREON_STATUS_OK);
    assert(!geometry_equal);
    const SidereonPppRaimResidual geometry_rows[5] = {
        {.satellite_id = "G01",
         .ambiguity_id = "G01",
         .code_m = 0.1,
         .phase_m = -0.1,
         .code_weight = 1.0,
         .phase_weight = 1.0},
        {.satellite_id = "G02",
         .ambiguity_id = "G02",
         .code_m = -0.1,
         .phase_m = 0.1,
         .code_weight = 1.0,
         .phase_weight = 1.0},
        {.satellite_id = "G03",
         .ambiguity_id = "G03",
         .code_m = 0.05,
         .phase_m = -0.05,
         .code_weight = 1.0,
         .phase_weight = 1.0},
        {.satellite_id = "G04",
         .ambiguity_id = "G04",
         .code_m = 3.0,
         .phase_m = 0.05,
         .code_weight = 1.0,
         .phase_weight = 1.0},
        {.satellite_id = "G05",
         .ambiguity_id = "G05",
         .code_m = 0.02,
         .phase_m = -0.02,
         .code_weight = 1.0,
         .phase_weight = 1.0},
    };
    config.chi_square_threshold = 1.0;
    result = NULL;
    assert(sidereon_ppp_raim_global_test_with_geometry(geometry_rows, 5, geometry, 5, config,
                                                       &result) == SIDEREON_STATUS_OK);
    assert(sidereon_ppp_raim_result_info(result, &info) == SIDEREON_STATUS_OK);
    assert(info.status == SIDEREON_PPP_RAIM_STATUS_FAULT_DETECTED);
    assert(info.detected);
    assert(info.test_statistic == 9.0483);
    assert(info.redundancy == 1);
    assert(info.satellite_statistic_count == 5);
    assert(info.has_most_likely_fault);
    size_t written = 0;
    size_t required = 0;
    unsigned char fault[4] = {0};
    assert(sidereon_ppp_raim_result_most_likely_fault(result, fault, sizeof(fault), &written,
                                                      &required) == SIDEREON_STATUS_OK);
    assert(written == 3 && required == 3 && memcmp(fault, "G04", 3) == 0);
    sidereon_ppp_raim_result_free(result);

    SidereonPppRaimIdentification *identification = NULL;
    assert(sidereon_ppp_raim_per_satellite_statistics(geometry_rows, 5, geometry, 5, config,
                                                      &identification) == SIDEREON_STATUS_OK);
    size_t statistic_count = 0;
    bool has_fault = false;
    assert(sidereon_ppp_raim_identification_info(identification, &statistic_count, &has_fault) ==
           SIDEREON_STATUS_OK);
    assert(statistic_count == 5 && has_fault);
    SidereonPppRaimSatelliteStatistic statistic;
    assert(sidereon_ppp_raim_identification_statistic(identification, 3, &statistic) ==
           SIDEREON_STATUS_OK);
    assert(statistic.code_testable && !statistic.phase_testable);
    assert(statistic.statistic == statistic.code);
    sidereon_ppp_raim_identification_free(identification);

    /* Independent NumPy/SciPy fixture: core tests/fixtures/ppp_raim_oracles.json. */
    const SidereonPppRaimGeometryRow oracle_geometry[6] = {
      {.satellite_id="G01", .line_of_sight={0.6344032590235116,0.46455589189480495,0.6178351950529927}},
      {.satellite_id="G02", .line_of_sight={-0.768437944530727,0.4248832010764284,0.4785158208966958}},
      {.satellite_id="G03", .line_of_sight={0.39884251403596477,-0.7774239880720021,0.48635027682493903}},
      {.satellite_id="G04", .line_of_sight={-0.6862316055536619,-0.5303441482709921,0.49781650026285734}},
      {.satellite_id="G05", .line_of_sight={0.6848692875740128,-0.4734637628313619,0.5538827711920927}},
      {.satellite_id="G06", .line_of_sight={-0.5179745434021694,0.7419074039750468,0.42576493081784084}}
    };
    const double weights[6] = {0.1, 0.2, 0.15, 0.3, 0.12, 0.25};
    SidereonPppRaimProtectionLevels protection;
    SidereonGeodetic receiver = {.lat_rad=0.9729221285256829, .lon_rad=0.21862976222017672, .height_m=0.0};
    config.has_chi_square_threshold = false;
    assert(sidereon_ppp_raim_protection_levels(oracle_geometry, 6, receiver, config, &protection) == SIDEREON_STATUS_OK);
    assert(close_to(protection.hpl_m, 57.887917962177994, 1e-8));
    assert(close_to(protection.vpl_m, 73.37084828641181, 1e-8));
    assert(sidereon_ppp_raim_protection_levels_with_weights(oracle_geometry, weights, 6, receiver, config, &protection) == SIDEREON_STATUS_OK);
    assert(close_to(protection.hpl_m, 314.13078984613486, 1e-8));
    assert(close_to(protection.vpl_m, 466.26302292463527, 1e-8));
    assert(sidereon_ppp_raim_fault_free_accuracy_bounds(oracle_geometry, weights, 6, receiver, config, &protection) == SIDEREON_STATUS_OK);
    assert(close_to(protection.hpl_m, 120.18361276551951, 1e-8));
    assert(close_to(protection.vpl_m, 163.34553880706616, 1e-8));
    assert(sidereon_ppp_raim_protection_levels(oracle_geometry, 4, receiver, config, &protection) == SIDEREON_STATUS_INVALID_ARGUMENT);
    assert(sidereon_ppp_raim_protection_levels_with_weights(oracle_geometry, weights, 6, receiver, config, NULL) == SIDEREON_STATUS_NULL_POINTER);
    assert(sidereon_ppp_raim_fault_free_accuracy_bounds(oracle_geometry, NULL, 6, receiver, config, &protection) == SIDEREON_STATUS_NULL_POINTER);
    assert(sidereon_ppp_raim_protection_levels_with_weights(oracle_geometry, weights, 6, receiver, config, (SidereonPppRaimProtectionLevels *)weights) == SIDEREON_STATUS_INVALID_ARGUMENT);
    config.has_chi_square_threshold = true;
    config.chi_square_threshold = 1e15;
    assert(sidereon_ppp_raim_protection_levels(oracle_geometry, 5, receiver, config, &protection) == SIDEREON_STATUS_INVALID_ARGUMENT);
    unsigned char domain_payload[512] = {0}; size_t domain_written = 0, domain_required = 0;
    assert(sidereon_last_engine_error_payload(domain_payload, sizeof(domain_payload)-1, &domain_written, &domain_required) == SIDEREON_STATUS_OK);
    assert(strstr((const char *)domain_payload, "\"fields\":{}") != NULL);
    assert(strstr((const char *)domain_payload, "unsupported_noncentrality") != NULL);
    /* The core supports at most 100000 residual degrees of freedom. */
    const size_t large_count = 100005;
    SidereonPppRaimGeometryRow *large_geometry = malloc(large_count * sizeof(*large_geometry));
    assert(large_geometry != NULL);
    for (size_t i = 0; i < large_count; ++i) large_geometry[i] = oracle_geometry[i % 6];
    config.has_chi_square_threshold = false;
    assert(sidereon_ppp_raim_protection_levels(large_geometry, large_count, receiver, config, &protection) == SIDEREON_STATUS_INVALID_ARGUMENT);
    free(large_geometry);
    memset(domain_payload, 0, sizeof(domain_payload));
    assert(sidereon_last_engine_error_payload(domain_payload, sizeof(domain_payload)-1, &domain_written, &domain_required) == SIDEREON_STATUS_OK);
    assert(strstr((const char *)domain_payload, "unsupported_dof") != NULL);
    assert(strstr((const char *)domain_payload, "\"fields\":{\"dof\":100001,\"max\":100000}") != NULL);
    config.has_chi_square_threshold = true;
    config.chi_square_threshold = 1.0;

    SidereonPppRaimResidual invalid = rows[0];
    invalid.code_weight = 0.0;
    result = NULL;
    assert(sidereon_ppp_raim_global_test(&invalid, 1, 0, config, &result) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    assert(result == NULL);
    SidereonEngineErrorInfo error;
    assert(sidereon_last_engine_error_info(&error) == SIDEREON_STATUS_OK);
    assert(error.family == SIDEREON_ENGINE_ERROR_FAMILY_PPP_RAIM);
    unsigned char payload[512] = {0};
    written = 0;
    required = 0;
    assert(sidereon_last_engine_error_payload(payload, sizeof(payload), &written, &required) ==
           SIDEREON_STATUS_OK);
    assert(written == required);
    assert(strstr((const char *)payload, "\"kind\":\"invalid_residual\"") != NULL);
    assert(strstr((const char *)payload, "\"field\":\"code_weight\"") != NULL);

    unsigned char text[128];
    assert(sidereon_ppp_raim_error_text(SIDEREON_PPP_RAIM_ERROR_KIND_SINGULAR_GEOMETRY, text,
                                        sizeof(text), &written, &required) == SIDEREON_STATUS_OK);
    assert(written == strlen("singular RAIM geometry"));
    /* Display mapping is checked here; no valid real input producing the
     * defensive numerical failure was found in the supported CDF domain. */
    assert(sidereon_ppp_raim_error_text(SIDEREON_PPP_RAIM_ERROR_KIND_NUMERICAL_PROTECTION_LEVEL, text,
                                        sizeof(text), &written, &required) == SIDEREON_STATUS_OK);
    assert(written == strlen("RAIM protection-level probability evaluation failed") &&
           memcmp(text, "RAIM protection-level probability evaluation failed", written) == 0);
    return 0;
}
