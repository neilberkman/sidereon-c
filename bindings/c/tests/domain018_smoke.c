/*
 * 0.18 domain exposure smoke: GNSS/INS fusion, deterministic scenarios, and
 * signal-analysis closed forms. Every numeric result is produced by
 * sidereon-core through the C ABI.
 */
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sidereon.h"

static int failures = 0;

static void check(int ok, const char *what) {
    if (!ok) {
        char msg[512];
        size_t n = sidereon_last_error_message(msg, sizeof(msg));
        if (n == 0) {
            msg[0] = '\0';
        }
        fprintf(stderr, "FAIL: %s (last_error: %s)\n", what, msg);
        failures++;
    }
}

static int close_abs(double actual, double expected, double tol) {
    return fabs(actual - expected) <= tol;
}

static uint64_t f64_bits(double value) {
    uint64_t bits = 0;
    memcpy(&bits, &value, sizeof(bits));
    return bits;
}

static void check_bits(double actual, uint64_t expected, const char *what) {
    check(f64_bits(actual) == expected, what);
}

static void check_vec_bits(const double *actual,
                           const uint64_t *expected,
                           size_t count,
                           const char *label) {
    char what[160];
    for (size_t i = 0; i < count; i++) {
        snprintf(what, sizeof(what), "%s[%zu] bits", label, i);
        check_bits(actual[i], expected[i], what);
    }
}

static void copy_state_position(const SidereonFusionState *state,
                                SidereonFusionLooseMeasurement *measurement,
                                double t_j2000_s,
                                const double *covariance,
                                size_t covariance_len) {
    memset(measurement, 0, sizeof(*measurement));
    measurement->t_j2000_s = t_j2000_s;
    measurement->position_ecef_m[0] = state->position_ecef_m[0];
    measurement->position_ecef_m[1] = state->position_ecef_m[1];
    measurement->position_ecef_m[2] = state->position_ecef_m[2];
    measurement->has_velocity = false;
    measurement->covariance = covariance;
    measurement->covariance_len = covariance_len;
    measurement->satellites_used = 4;
    measurement->solution_valid = true;
    measurement->fix_status = SIDEREON_FUSION_GNSS_FIX_STATUS_SINGLE;
}

static void identity_matrix(double *matrix, size_t dimension) {
    memset(matrix, 0, dimension * dimension * sizeof(double));
    for (size_t i = 0; i < dimension; i++) {
        matrix[i * dimension + i] = 1.0;
    }
}

static void set_zero_imu_spec(SidereonFusionFilterConfig *config) {
    config->imu_spec.accel_vrw_mps_sqrt_s = 0.0;
    config->imu_spec.gyro_arw_rad_sqrt_s = 0.0;
    config->imu_spec.accel_bias_instab_mps2 = 0.0;
    config->imu_spec.gyro_bias_instab_rps = 0.0;
    config->imu_spec.accel_bias_tau_s = INFINITY;
    config->imu_spec.gyro_bias_tau_s = INFINITY;
    config->imu_spec.has_accel_scale_instab_ppm = false;
    config->imu_spec.accel_scale_instab_ppm = 0.0;
    config->imu_spec.has_gyro_scale_instab_ppm = false;
    config->imu_spec.gyro_scale_instab_ppm = 0.0;
}

static void init_field_config(SidereonFusionFilterConfig *config) {
    check(sidereon_fusion_filter_config_init(config) == SIDEREON_STATUS_OK,
          "fusion field config init");
    set_zero_imu_spec(config);
}

static SidereonFusionFilter *create_field_filter(const SidereonFusionFilterConfig *config,
                                                 const SidereonFusionNavState *nav,
                                                 const char *label) {
    double diag[15];
    for (size_t i = 0; i < 15; i++) {
        diag[i] = 1.0;
    }
    SidereonFusionFilter *filter = NULL;
    check(sidereon_fusion_filter_create(nav, diag, 15, config, &filter) == SIDEREON_STATUS_OK &&
              filter != NULL,
          label);
    return filter;
}

static void init_position_velocity_fix(SidereonFusionLooseMeasurement *measurement,
                                       double t_j2000_s,
                                       const double position[3],
                                       const double velocity[3],
                                       const double *covariance,
                                       uint32_t fix_status) {
    memset(measurement, 0, sizeof(*measurement));
    measurement->t_j2000_s = t_j2000_s;
    memcpy(measurement->position_ecef_m, position, sizeof(measurement->position_ecef_m));
    measurement->has_velocity = true;
    memcpy(measurement->velocity_ecef_mps, velocity, sizeof(measurement->velocity_ecef_mps));
    measurement->covariance = covariance;
    measurement->covariance_len = 36;
    measurement->satellites_used = 8;
    measurement->solution_valid = true;
    measurement->fix_status = fix_status;
}

static void init_default_position_velocity_fix(SidereonFusionLooseMeasurement *measurement,
                                               const double *covariance,
                                               uint32_t fix_status) {
    const double position[3] = {6378138.0, 2.0, -3.0};
    const double velocity[3] = {0.4, -0.2, 0.1};
    init_position_velocity_fix(measurement, 0.0, position, velocity, covariance, fix_status);
}

static void check_filter_cov_diag_bits(SidereonFusionFilter *filter,
                                       size_t offset,
                                       const uint64_t *expected,
                                       size_t count,
                                       const char *label) {
    double covariance[225];
    size_t written = 0, required = 0;
    check(sidereon_fusion_filter_covariance(filter, covariance, 225, &written, &required) ==
              SIDEREON_STATUS_OK &&
              written == 225 && required == 225,
          label);
    char what[160];
    for (size_t i = 0; i < count; i++) {
        const size_t index = (offset + i) * 15 + (offset + i);
        snprintf(what, sizeof(what), "%s diag[%zu] bits", label, offset + i);
        check_bits(covariance[index], expected[i], what);
    }
}

static void test_signal_analysis(void) {
    SidereonSignalAnalysisModulation bpsk = {
        SIDEREON_SIGNAL_ANALYSIS_MODULATION_KIND_BPSK, 1.0, 0.0, 0.0};
    SidereonSignalAnalysisModulation boc = {
        SIDEREON_SIGNAL_ANALYSIS_MODULATION_KIND_BOC_SINE, 0.0, 1.0, 1.0};
    SidereonSignalAnalysisModulation boc_cosine = {
        SIDEREON_SIGNAL_ANALYSIS_MODULATION_KIND_BOC_COSINE, 0.0, 10.0, 5.0};
    SidereonSignalAnalysisModulation mboc = {
        SIDEREON_SIGNAL_ANALYSIS_MODULATION_KIND_MBOC611_OVER11, 0.0, 0.0, 0.0};
    SidereonSignalAnalysisModulation tmboc = {
        SIDEREON_SIGNAL_ANALYSIS_MODULATION_KIND_TMBOC614_OVER33, 0.0, 0.0, 0.0};
    SidereonSignalAnalysisModulation cboc_plus = {
        SIDEREON_SIGNAL_ANALYSIS_MODULATION_KIND_CBOC611_OVER11_PLUS, 0.0, 0.0, 0.0};
    SidereonSignalAnalysisModulation cboc_minus = {
        SIDEREON_SIGNAL_ANALYSIS_MODULATION_KIND_CBOC611_OVER11_MINUS, 0.0, 0.0, 0.0};
    double value = 0.0;

    check(sidereon_signal_analysis_psd(&bpsk, 0.0, &value) == SIDEREON_STATUS_OK &&
              close_abs(value, 9.775171065493646e-7, 1.0e-21),
          "signal BPSK(1) PSD");
    check(sidereon_signal_analysis_psd(&boc, 0.5 * 1023000.0, &value) == SIDEREON_STATUS_OK &&
              close_abs(value, 3.9617276106485926e-7, 1.0e-21),
          "signal BOCsin(1,1) PSD");
    check(sidereon_signal_analysis_psd(&boc_cosine, 0.5 * 1023000.0, &value) ==
                  SIDEREON_STATUS_OK &&
              close_abs(value, 1.80864807395667e-12, 1.0e-25),
          "signal BOCcos(10,5) PSD");
    check(sidereon_signal_analysis_psd(&mboc, 0.5 * 1023000.0, &value) ==
                  SIDEREON_STATUS_OK &&
              close_abs(value, 3.6078129341245042e-7, 1.0e-21),
          "signal MBOC PSD");
    check(sidereon_signal_analysis_psd(&tmboc, 0.5 * 1023000.0, &value) ==
                  SIDEREON_STATUS_OK &&
              close_abs(value, 3.4898413752831416e-7, 1.0e-21),
          "signal TMBOC PSD");
    check(sidereon_signal_analysis_psd(&cboc_plus, 0.5 * 1023000.0, &value) ==
                  SIDEREON_STATUS_OK &&
              close_abs(value, 3.9076953668363188e-7, 1.0e-21),
          "signal CBOC plus PSD");
    check(sidereon_signal_analysis_psd(&cboc_minus, 0.5 * 1023000.0, &value) ==
                  SIDEREON_STATUS_OK &&
              close_abs(value, 3.3079305014126891e-7, 1.0e-21),
          "signal CBOC minus PSD");
    check(sidereon_signal_analysis_fraction_power(&bpsk, 24000000.0, &value) ==
              SIDEREON_STATUS_OK &&
              close_abs(value, 0.99147813722178968, 1.0e-15),
          "signal fraction power");
    check(sidereon_signal_analysis_rms_bandwidth_hz(&boc, 24000000.0, &value) ==
              SIDEREON_STATUS_OK &&
              close_abs(value, 1978624.6068839289, 1.0e-9),
          "signal RMS bandwidth");

    SidereonSignalAnalysisSpectralSeparation ssc;
    check(sidereon_signal_analysis_spectral_separation(&bpsk, &boc, 24000000.0, &ssc) ==
              SIDEREON_STATUS_OK &&
              close_abs(ssc.hz, 1.629171137084864e-7, 1.0e-20) &&
              close_abs(ssc.db_hz, -67.880332926173509, 1.0e-12),
          "signal SSC");
    check(sidereon_signal_spectral_separation_coefficient_hz(&bpsk, &boc, 24000000.0,
                                                             &value) == SIDEREON_STATUS_OK &&
              close_abs(value, 1.629171137084864e-7, 1.0e-20),
          "signal SSC scalar hz");
    check(sidereon_signal_spectral_separation_coefficient_db_hz(&bpsk, &boc, 24000000.0,
                                                                &value) == SIDEREON_STATUS_OK &&
              close_abs(value, -67.880332926173509, 1.0e-12),
          "signal SSC scalar db hz");
    check(sidereon_signal_white_noise_spectral_separation_hz(&bpsk, 24000000.0, &value) ==
              SIDEREON_STATUS_OK &&
              close_abs(value, 4.1311589050907902e-8, 1.0e-20),
          "signal white-noise SSC scalar");

    SidereonSignalAnalysisInterference interference = {boc, 0.01};
    SidereonSignalAnalysisCn0Degradation degradation;
    check(sidereon_signal_analysis_effective_cn0_degradation(
              &bpsk, 45.0, 24000000.0, &interference, 1, &degradation) == SIDEREON_STATUS_OK &&
              close_abs(degradation.effective_cn0_hz, 31621.13351302073, 1.0e-8) &&
              close_abs(degradation.effective_cn0_db_hz, 44.999774338955795, 1.0e-12) &&
              close_abs(degradation.degradation_db, 0.00022566104420462807, 1.0e-15),
          "signal effective C/N0 degradation");

    SidereonSignalAnalysisDllTrackingOptions dll = {45.0, 1.0, 0.02, 0.5, 100000000.0};
    SidereonSignalAnalysisDllJitter jitter;
    check(sidereon_signal_analysis_dll_jitter(
              &bpsk, &dll, SIDEREON_SIGNAL_ANALYSIS_DLL_PROCESSING_COHERENT, &jitter) ==
              SIDEREON_STATUS_OK &&
              close_abs(jitter.chips, 0.0027925349810969391, 1.0e-15) &&
              close_abs(jitter.meters, 0.8183586764751074, 1.0e-12),
          "signal DLL jitter coherent");
    check(sidereon_signal_analysis_dll_lower_bound(&boc, &dll, &jitter) == SIDEREON_STATUS_OK &&
              close_abs(jitter.seconds, 2.2630212065471776e-10, 1.0e-21),
          "signal DLL lower bound");

    const double delays[3] = {0.0, 0.5, 1.0};
    SidereonSignalAnalysisMultipathOptions mp = {0.5, 1.0, 100000000.0};
    SidereonSignalAnalysisMultipathEnvelopePoint points[3];
    size_t written = 0, required = 0;
    check(sidereon_signal_analysis_multipath_envelope(&bpsk, &mp, delays, 3, points, 3, &written,
                                                       &required) == SIDEREON_STATUS_OK &&
              written == 3 && required == 3 &&
              close_abs(points[1].in_phase_chips, 0.16666443790427826, 1.0e-12) &&
              close_abs(points[1].anti_phase_chips, -0.20000709850498244, 1.0e-12) &&
              close_abs(points[1].running_average_chips, 0.10000354925249122, 1.0e-12),
          "signal multipath envelope");
}

static const char *scenario_json =
    "{"
    "\"schema_version\":1,"
    "\"seed\":5855056816869359901,"
    "\"epochs\":{\"start_j2000_s\":0.0,\"count\":2,\"cadence_s\":30.0},"
    "\"receiver\":{\"kind\":\"static_geodetic\",\"position\":{\"lat_rad\":0.0,\"lon_rad\":0.0,"
    "\"height_m\":0.0}},"
    "\"constellation\":{\"kind\":\"synthetic_keplerian\",\"satellites\":["
    "{\"satellite_id\":{\"system\":\"Gps\",\"prn\":1},\"semi_major_axis_m\":26560000.0,"
    "\"eccentricity\":0.0,\"inclination_rad\":0.0,\"raan_rad\":0.0,\"arg_perigee_rad\":0.0,"
    "\"mean_anomaly_rad\":0.0,\"epoch_j2000_s\":0.0,\"clock_bias_s\":0.0,"
    "\"clock_drift_s_s\":0.0},"
    "{\"satellite_id\":{\"system\":\"Gps\",\"prn\":2},\"semi_major_axis_m\":26560000.0,"
    "\"eccentricity\":0.0,\"inclination_rad\":0.0,\"raan_rad\":0.0,\"arg_perigee_rad\":0.0,"
    "\"mean_anomaly_rad\":1.0471975511965976,\"epoch_j2000_s\":0.0,\"clock_bias_s\":0.0,"
    "\"clock_drift_s_s\":0.0},"
    "{\"satellite_id\":{\"system\":\"Gps\",\"prn\":3},\"semi_major_axis_m\":26560000.0,"
    "\"eccentricity\":0.0,\"inclination_rad\":0.0,\"raan_rad\":0.0,\"arg_perigee_rad\":0.0,"
    "\"mean_anomaly_rad\":-1.0471975511965976,\"epoch_j2000_s\":0.0,\"clock_bias_s\":0.0,"
    "\"clock_drift_s_s\":0.0},"
    "{\"satellite_id\":{\"system\":\"Gps\",\"prn\":4},\"semi_major_axis_m\":26560000.0,"
    "\"eccentricity\":0.0,\"inclination_rad\":1.5707963267948966,"
    "\"raan_rad\":0.0,\"arg_perigee_rad\":0.0,\"mean_anomaly_rad\":1.0471975511965976,"
    "\"epoch_j2000_s\":0.0,\"clock_bias_s\":0.0,\"clock_drift_s_s\":0.0},"
    "{\"satellite_id\":{\"system\":\"Gps\",\"prn\":5},\"semi_major_axis_m\":26560000.0,"
    "\"eccentricity\":0.0,\"inclination_rad\":1.5707963267948966,"
    "\"raan_rad\":0.0,\"arg_perigee_rad\":0.0,\"mean_anomaly_rad\":-1.0471975511965976,"
    "\"epoch_j2000_s\":0.0,\"clock_bias_s\":0.0,\"clock_drift_s_s\":0.0}"
    "]},"
    "\"signals\":[{\"system\":\"Gps\",\"code_observable\":\"C1C\",\"phase_observable\":\"L1C\","
    "\"doppler_observable\":\"D1C\",\"carrier_hz\":1575420000.0,"
    "\"carrier_phase_bias_cycles\":0.0}],"
    "\"error_budget\":{\"receiver_clock\":{\"enabled\":true,\"bias_s\":1.0e-7,"
    "\"drift_s_s\":1.0e-10,\"power_law_coefficients\":[1.0e-24,1.0e-26,1.0e-22,"
    "1.0e-26,1.0e-28]},\"satellite_clock\":{\"enabled\":false,\"bias_s\":0.0,"
    "\"drift_s_s\":0.0,\"power_law_coefficients\":[0.0,0.0,0.0,0.0,0.0]},"
    "\"ionosphere\":{\"kind\":\"off\"},\"troposphere\":{\"kind\":\"off\"},"
    "\"thermal_noise\":{\"enabled\":true,\"pseudorange_sigma_m\":0.25,"
    "\"carrier_phase_sigma_m\":0.002,\"doppler_sigma_hz\":0.02},"
    "\"multipath\":{\"enabled\":true,\"amplitude_m\":0.15,\"reflector_height_m\":1.25,"
    "\"phase_rad\":0.3},\"elevation_mask_deg\":-5.0}}";

static void test_scenario(void) {
    SidereonScenarioSimulation *first = NULL;
    SidereonScenarioSimulation *second = NULL;
    const size_t len = strlen(scenario_json);
    check(sidereon_scenario_simulate_json((const uint8_t *)scenario_json, len, &first) ==
              SIDEREON_STATUS_OK &&
              first != NULL,
          "scenario simulate first");
    check(sidereon_scenario_simulate_json((const uint8_t *)scenario_json, len, &second) ==
              SIDEREON_STATUS_OK &&
              second != NULL,
          "scenario simulate second");
    if (first == NULL || second == NULL) {
        sidereon_scenario_simulation_free(first);
        sidereon_scenario_simulation_free(second);
        return;
    }

    SidereonScenarioSummary a, b;
    check(sidereon_scenario_simulation_summary(first, &a) == SIDEREON_STATUS_OK,
          "scenario summary first");
    check(sidereon_scenario_simulation_summary(second, &b) == SIDEREON_STATUS_OK,
          "scenario summary second");
    check(a.determinism_fingerprint == b.determinism_fingerprint &&
              a.observation_count == b.observation_count && a.json_len == b.json_len,
          "scenario deterministic summary");
    /* The fingerprint and payload length stamp the engine version by design,
     * so they change each release; the invariants are determinism (checked
     * above) and the structural counts. */
    check(a.observation_count == 10 && a.epoch_offset_count == 3,
          "scenario pinned summary");

    uint8_t *json_a = (uint8_t *)malloc(a.json_len);
    uint8_t *json_b = (uint8_t *)malloc(b.json_len);
    size_t written = 0, required = 0;
    check(json_a != NULL && json_b != NULL, "scenario json alloc");
    if (json_a != NULL && json_b != NULL) {
        check(sidereon_scenario_simulation_json(first, json_a, a.json_len, &written, &required) ==
                  SIDEREON_STATUS_OK &&
                  written == a.json_len && required == a.json_len,
              "scenario json first");
        check(sidereon_scenario_simulation_json(second, json_b, b.json_len, &written, &required) ==
                  SIDEREON_STATUS_OK &&
                  written == b.json_len && required == b.json_len,
              "scenario json second");
        check(memcmp(json_a, json_b, a.json_len) == 0, "scenario deterministic bytes");
    }

    SidereonScenarioObservation observations[16];
    check(sidereon_scenario_observations(first, observations, 16, &written, &required) ==
              SIDEREON_STATUS_OK &&
              written == a.observation_count && required == a.observation_count,
          "scenario observations");
    SidereonScenarioTerms terms[16];
    check(sidereon_scenario_terms(first, terms, 16, &written, &required) == SIDEREON_STATUS_OK &&
              written == a.observation_count && required == a.observation_count,
          "scenario terms");
    if (a.observation_count > 0) {
        check(close_abs(observations[0].pseudorange_m, 20181892.897856534, 1.0e-8) &&
                  close_abs(observations[0].carrier_phase_cycles, 106056563.40292211, 1.0e-8) &&
                  close_abs(observations[0].doppler_hz, -0.089520544733728835, 1.0e-14) &&
                  close_abs(terms[0].geometric_range_m, 20181863.00040463, 1.0e-8) &&
                  close_abs(terms[0].thermal_noise_m, -0.1410676061858952, 1.0e-14),
              "scenario pinned first rows");
    }
    free(json_a);
    free(json_b);
    sidereon_scenario_simulation_free(first);
    sidereon_scenario_simulation_free(second);
}

static void init_nav(SidereonFusionNavState *nav) {
    memset(nav, 0, sizeof(*nav));
    nav->position_ecef_m[0] = 6378137.0;
    nav->attitude_body_to_ecef[0] = 1.0;
    nav->attitude_body_to_ecef[4] = 1.0;
    nav->attitude_body_to_ecef[8] = 1.0;
}

static void test_fusion(void) {
    SidereonFusionFilterConfig config;
    check(sidereon_fusion_filter_config_init(&config) == SIDEREON_STATUS_OK,
          "fusion config init");
    config.time_sync_imu_capacity = 8;
    config.time_sync_checkpoint_capacity = 4;

    SidereonFusionNavState nav;
    init_nav(&nav);
    double diag[15];
    for (size_t i = 0; i < 15; i++) {
        diag[i] = 10.0;
    }

    SidereonFusionFilter *filter = NULL;
    check(sidereon_fusion_filter_create(&nav, diag, 15, &config, &filter) == SIDEREON_STATUS_OK &&
              filter != NULL,
          "fusion create EKF");

    SidereonFusionFilterConfig ukf_config = config;
    ukf_config.filter_kind = SIDEREON_FUSION_FILTER_KIND_UKF;
    SidereonFusionFilter *ukf_filter = NULL;
    check(sidereon_fusion_filter_create(&nav, diag, 15, &ukf_config, &ukf_filter) ==
              SIDEREON_STATUS_OK &&
              ukf_filter != NULL,
          "fusion create UKF");
    sidereon_fusion_filter_free(ukf_filter);

    if (filter == NULL) {
        return;
    }

    SidereonFusionImuSample sample;
    memset(&sample, 0, sizeof(sample));
    sample.kind = SIDEREON_FUSION_IMU_SAMPLE_KIND_INCREMENT;
    sample.t_j2000_s = 1.0;
    sample.dt_s = 1.0;
    sample.delta_velocity_mps[0] = 9.7803253359;
    check(sidereon_fusion_filter_propagate(filter, &sample) == SIDEREON_STATUS_OK,
          "fusion propagate first");
    SidereonFusionState state1;
    check(sidereon_fusion_filter_state(filter, &state1) == SIDEREON_STATUS_OK,
          "fusion state after first propagate");

    sample.t_j2000_s = 2.0;
    check(sidereon_fusion_filter_propagate(filter, &sample) == SIDEREON_STATUS_OK,
          "fusion propagate second");

    double loose_cov[9] = {1.0e12, 0.0, 0.0, 0.0, 1.0e12, 0.0, 0.0, 0.0, 1.0e12};
    SidereonFusionLooseMeasurement late;
    copy_state_position(&state1, &late, 1.5, loose_cov, 9);
    SidereonFusionTimeSyncUpdate sync_update;
    check(sidereon_fusion_filter_update_loose_time_sync(filter, &late, &sync_update) ==
              SIDEREON_STATUS_OK &&
              sync_update.update.rows == 3 && sync_update.replayed_imu_segments == 3 &&
              sync_update.late_measurement &&
              close_abs(sync_update.update.nis, 8.3946053370110733e-20, 1.0e-30) &&
              close_abs(sync_update.current_epoch_j2000_s, 2.0, 1.0e-12),
          "fusion loose time-sync update");

    SidereonFusionState state2;
    check(sidereon_fusion_filter_state(filter, &state2) == SIDEREON_STATUS_OK,
          "fusion state after time-sync");
    check(close_abs(state2.position_ecef_m[0], 6378136.9999999311, 1.0e-8) &&
              close_abs(state2.position_ecef_m[1], -0.0010252141450324513, 1.0e-15) &&
              close_abs(state2.position_ecef_m[2], 0.0, 1.0e-15) &&
              close_abs(state2.tight_clock_bias_m, 0.0, 1.0e-15) &&
              close_abs(state2.tight_clock_drift_m_s, 0.0, 1.0e-15) &&
              state2.covariance_dimension == 15,
          "fusion pinned state");

    size_t written = 0, required = 0;
    check(sidereon_fusion_filter_encode_state(filter, NULL, 0, &written, &required) ==
              SIDEREON_STATUS_OK &&
              required > 0 && written == 0,
          "fusion encode query");
    uint8_t *bytes = (uint8_t *)malloc(required);
    check(bytes != NULL, "fusion encode alloc");
    if (bytes != NULL) {
        const size_t byte_count = required;
        check(sidereon_fusion_filter_encode_state(filter, bytes, byte_count, &written, &required) ==
                  SIDEREON_STATUS_OK &&
                  written == byte_count && required == byte_count,
              "fusion encode bytes");
        check(sidereon_fusion_filter_restore_state(filter, bytes, byte_count) ==
                  SIDEREON_STATUS_OK,
              "fusion restore bytes");
        uint8_t *roundtrip = (uint8_t *)malloc(byte_count);
        check(roundtrip != NULL, "fusion roundtrip alloc");
        if (roundtrip != NULL) {
            check(sidereon_fusion_filter_encode_state(filter, roundtrip, byte_count, &written,
                                                      &required) == SIDEREON_STATUS_OK &&
                      written == byte_count && required == byte_count &&
                      memcmp(bytes, roundtrip, byte_count) == 0,
                  "fusion encode restore byte roundtrip");
            free(roundtrip);
        }
        free(bytes);
    }

    SidereonFusionTimeSyncStatus status;
    check(sidereon_fusion_filter_time_sync_status(filter, &status) == SIDEREON_STATUS_OK &&
              status.imu_capacity == 8 && status.checkpoint_capacity == 4,
          "fusion time-sync status");
    sidereon_fusion_filter_free(filter);
}

static void test_fusion_field_mode(void) {
    double cov6[36];
    identity_matrix(cov6, 6);

    SidereonFusionNavState nav;
    init_nav(&nav);

    SidereonFusionFilterConfig config;
    init_field_config(&config);
    check(config.loose_fix_status_weighting.single_sigma_multiplier == 1.0 &&
              config.loose_fix_status_weighting.float_sigma_multiplier == 1.0 &&
              config.loose_fix_status_weighting.fixed_sigma_multiplier == 1.0 &&
              !config.has_loose_stationary_updates && !config.has_loose_non_holonomic &&
              config.time_sync_imu_capacity > 0,
          "fusion field defaults omitted");
    const uint64_t imu_to_body_bits[9] = {
        UINT64_C(0x3FF0000000000000), UINT64_C(0x0000000000000000),
        UINT64_C(0x0000000000000000), UINT64_C(0x0000000000000000),
        UINT64_C(0x3FF0000000000000), UINT64_C(0x0000000000000000),
        UINT64_C(0x0000000000000000), UINT64_C(0x0000000000000000),
        UINT64_C(0x3FF0000000000000),
    };
    check_vec_bits(config.imu_to_body_dcm, imu_to_body_bits, 9, "fusion field imu_to_body");

    SidereonFusionFilter *filter = create_field_filter(&config, &nav, "fusion field create");
    if (filter != NULL) {
        SidereonFusionLooseMeasurement fix;
        init_default_position_velocity_fix(&fix, cov6, SIDEREON_FUSION_GNSS_FIX_STATUS_SINGLE);
        SidereonFusionUpdate update;
        check(sidereon_fusion_filter_update_loose(filter, &fix, &update) ==
                  SIDEREON_STATUS_OK &&
                  update.applied && update.rows == 6 && update.accepted_rows == 6 &&
                  update.rejected_rows == 0,
              "fusion field default loose update");
        check_bits(update.nis, UINT64_C(0x401C6B851EB851E9),
                   "fusion field default NIS bits");

        SidereonFusionState state;
        check(sidereon_fusion_filter_state(filter, &state) == SIDEREON_STATUS_OK,
              "fusion field default state");
        const uint64_t position_bits[3] = {
            UINT64_C(0x415854A660000000),
            UINT64_C(0x3FEFFFFFFFFFFFFF),
            UINT64_C(0xBFF7FFFFFFFFFFFF),
        };
        const uint64_t velocity_bits[3] = {
            UINT64_C(0x3FC9999999999999),
            UINT64_C(0xBFB9999999999999),
            UINT64_C(0x3FA9999999999999),
        };
        const uint64_t cov_bits[6] = {
            UINT64_C(0x3FDFFFFFFFFFFFFF), UINT64_C(0x3FDFFFFFFFFFFFFF),
            UINT64_C(0x3FDFFFFFFFFFFFFF), UINT64_C(0x3FDFFFFFFFFFFFFF),
            UINT64_C(0x3FDFFFFFFFFFFFFF), UINT64_C(0x3FDFFFFFFFFFFFFF),
        };
        check_vec_bits(state.position_ecef_m, position_bits, 3, "fusion field position");
        check_vec_bits(state.velocity_ecef_mps, velocity_bits, 3, "fusion field velocity");
        check_filter_cov_diag_bits(filter, 0, cov_bits, 6, "fusion field covariance");
    }
    sidereon_fusion_filter_free(filter);

    init_field_config(&config);
    config.has_loose_stationary_updates = true;
    config.loose_stationary_updates.detector.window_len = 1;
    config.loose_stationary_updates.detector.max_specific_force_norm_error_mps2 = 100.0;
    config.loose_stationary_updates.detector.max_body_rate_wrt_ecef_norm_rps = 1.0;
    config.loose_stationary_updates.zero_velocity_sigma_mps = 0.5;
    config.loose_stationary_updates.zero_angular_rate_sigma_rps = 0.05;
    filter = create_field_filter(&config, &nav, "fusion stationary create");
    if (filter != NULL) {
        SidereonFusionImuSample sample;
        memset(&sample, 0, sizeof(sample));
        sample.kind = SIDEREON_FUSION_IMU_SAMPLE_KIND_INCREMENT;
        sample.t_j2000_s = 1.0;
        sample.dt_s = 1.0;
        check(sidereon_fusion_filter_propagate(filter, &sample) == SIDEREON_STATUS_OK,
              "fusion stationary propagate");
        SidereonFusionUpdate update;
        bool present = false;
        check(sidereon_fusion_filter_update_stationary(filter, &update, &present) ==
                  SIDEREON_STATUS_OK &&
                  present && update.applied && update.rows == 6 && update.accepted_rows == 6 &&
                  update.rejected_rows == 0,
              "fusion stationary update");
        check_bits(update.nis, UINT64_C(0x404541AF8E65B9FC),
                   "fusion stationary NIS bits");
        SidereonFusionState state;
        check(sidereon_fusion_filter_state(filter, &state) == SIDEREON_STATUS_OK,
              "fusion stationary state");
        const uint64_t station_velocity_bits[3] = {
            UINT64_C(0xBFF16320EDFCD4C0),
            UINT64_C(0xBDE64EF6EFBB7204),
            UINT64_C(0x0000000000000000),
        };
        const uint64_t gyro_bias_bits[3] = {
            UINT64_C(0x0000000000000000),
            UINT64_C(0x0000000000000000),
            UINT64_C(0xBF131173B6B2C903),
        };
        const uint64_t stationary_cov_bits[6] = {
            UINT64_C(0x3FCC71C76E2F216E), UINT64_C(0x3FCC71C6F3FF694D),
            UINT64_C(0x3FCC71C6F3B73AFD), UINT64_C(0x3FF00A36E71A6702),
            UINT64_C(0x3FF00A36E71A6702), UINT64_C(0x3FF00A36E71A2CB0),
        };
        check_vec_bits(state.velocity_ecef_mps, station_velocity_bits, 3,
                       "fusion stationary velocity");
        check_vec_bits(state.gyro_bias_rps, gyro_bias_bits, 3,
                       "fusion stationary gyro bias");
        check_filter_cov_diag_bits(filter, 3, stationary_cov_bits, 6,
                                   "fusion stationary covariance");
    }
    sidereon_fusion_filter_free(filter);

    init_field_config(&config);
    SidereonFusionFilter *plain = create_field_filter(&config, &nav, "fusion no stationary create");
    if (plain != NULL) {
        SidereonFusionUpdate update;
        bool present = true;
        check(sidereon_fusion_filter_update_stationary(plain, &update, &present) ==
                  SIDEREON_STATUS_OK &&
                  !present,
              "fusion stationary absent without config");
    }
    sidereon_fusion_filter_free(plain);

    init_field_config(&config);
    config.loose_fix_status_weighting.single_sigma_multiplier = 3.0;
    config.loose_fix_status_weighting.float_sigma_multiplier = 2.0;
    config.loose_fix_status_weighting.fixed_sigma_multiplier = 1.0;
    const uint32_t statuses[3] = {
        SIDEREON_FUSION_GNSS_FIX_STATUS_SINGLE,
        SIDEREON_FUSION_GNSS_FIX_STATUS_FLOAT,
        SIDEREON_FUSION_GNSS_FIX_STATUS_FIXED,
    };
    const uint64_t nis_bits[3] = {
        UINT64_C(0x3FF6BC6A7EF9DB22),
        UINT64_C(0x4006BC6A7EF9DB22),
        UINT64_C(0x401C6B851EB851E9),
    };
    const uint64_t diag_bits[3][6] = {
        {UINT64_C(0x3FECCCCCCCCCCCCD), UINT64_C(0x3FECCCCCCCCCCCCD),
         UINT64_C(0x3FECCCCCCCCCCCCD), UINT64_C(0x3FECCCCCCCCCCCCD),
         UINT64_C(0x3FECCCCCCCCCCCCD), UINT64_C(0x3FECCCCCCCCCCCCD)},
        {UINT64_C(0x3FE999999999999A), UINT64_C(0x3FE999999999999A),
         UINT64_C(0x3FE999999999999A), UINT64_C(0x3FE999999999999A),
         UINT64_C(0x3FE999999999999A), UINT64_C(0x3FE999999999999A)},
        {UINT64_C(0x3FDFFFFFFFFFFFFF), UINT64_C(0x3FDFFFFFFFFFFFFF),
         UINT64_C(0x3FDFFFFFFFFFFFFF), UINT64_C(0x3FDFFFFFFFFFFFFF),
         UINT64_C(0x3FDFFFFFFFFFFFFF), UINT64_C(0x3FDFFFFFFFFFFFFF)},
    };
    double cov_x[3] = {0.0, 0.0, 0.0};
    for (size_t i = 0; i < 3; i++) {
        filter = create_field_filter(&config, &nav, "fusion fix weighting create");
        if (filter == NULL) {
            continue;
        }
        SidereonFusionLooseMeasurement fix;
        init_default_position_velocity_fix(&fix, cov6, statuses[i]);
        SidereonFusionUpdate update;
        check(sidereon_fusion_filter_update_loose(filter, &fix, &update) ==
                  SIDEREON_STATUS_OK &&
                  update.applied && update.rows == 6,
              "fusion fix weighting update");
        check_bits(update.nis, nis_bits[i], "fusion fix weighting NIS bits");
        double covariance[225];
        size_t written = 0, required = 0;
        check(sidereon_fusion_filter_covariance(filter, covariance, 225, &written, &required) ==
                  SIDEREON_STATUS_OK &&
                  written == 225 && required == 225,
              "fusion fix weighting covariance");
        cov_x[i] = covariance[0];
        check_filter_cov_diag_bits(filter, 0, diag_bits[i], 6, "fusion fix weighting diag");
        sidereon_fusion_filter_free(filter);
    }
    check(cov_x[2] < cov_x[1] && cov_x[1] < cov_x[0],
          "fusion fix weighting covariance order");

    SidereonFusionVelocityMatchState states[3] = {
        {0.0, {0.0, 0.0, 0.0}, {1.0, 0.0, 0.0}},
        {1.0, {1.0, 0.0, 0.0}, {1.0, 0.0, 0.0}},
        {2.0, {2.0, 0.0, 0.0}, {1.0, 0.0, 0.0}},
    };
    const double first_good_position[3] = {4.0, 1.0, 0.0};
    const double first_good_velocity[3] = {2.0, 0.0, 0.0};
    SidereonFusionLooseMeasurement first_good;
    init_position_velocity_fix(&first_good, 2.0, first_good_position, first_good_velocity, cov6,
                               SIDEREON_FUSION_GNSS_FIX_STATUS_SINGLE);
    SidereonFusionVelocityMatchingConfig vm_config = {5.0};
    SidereonFusionVelocityMatchState matched[3];
    SidereonFusionVelocityMatchedTrajectory trajectory;
    size_t written = 0, required = 0;
    check(sidereon_fusion_velocity_match_outage(states, 3, &first_good, &vm_config, matched, 3,
                                                &written, &required, &trajectory) ==
                  SIDEREON_STATUS_OK &&
              written == 3 && required == 3 && trajectory.state_count == 3,
          "fusion velocity matching outage");
    const uint64_t endpoint_position_bits[3] = {
        UINT64_C(0x4000000000000000),
        UINT64_C(0x3FF0000000000000),
        UINT64_C(0x0000000000000000),
    };
    const uint64_t endpoint_velocity_bits[3] = {
        UINT64_C(0x3FF0000000000000),
        UINT64_C(0x0000000000000000),
        UINT64_C(0x0000000000000000),
    };
    const uint64_t middle_position_bits[3] = {
        UINT64_C(0x3FFC000000000000),
        UINT64_C(0x3FE0000000000000),
        UINT64_C(0x0000000000000000),
    };
    const uint64_t middle_velocity_bits[3] = {
        UINT64_C(0x4002000000000000),
        UINT64_C(0x3FE8000000000000),
        UINT64_C(0x0000000000000000),
    };
    check_vec_bits(trajectory.endpoint_position_correction_ecef_m, endpoint_position_bits, 3,
                   "fusion velocity matching endpoint position");
    check_vec_bits(trajectory.endpoint_velocity_correction_ecef_mps, endpoint_velocity_bits, 3,
                   "fusion velocity matching endpoint velocity");
    check_vec_bits(matched[1].position_ecef_m, middle_position_bits, 3,
                   "fusion velocity matching middle position");
    check_vec_bits(matched[1].velocity_ecef_mps, middle_velocity_bits, 3,
                   "fusion velocity matching middle velocity");

    init_field_config(&config);
    config.imu_to_body_dcm[0] = 0.0;
    config.imu_to_body_dcm[1] = -1.0;
    config.imu_to_body_dcm[2] = 0.0;
    config.imu_to_body_dcm[3] = 1.0;
    config.imu_to_body_dcm[4] = 0.0;
    config.imu_to_body_dcm[5] = 0.0;
    config.imu_to_body_dcm[6] = 0.0;
    config.imu_to_body_dcm[7] = 0.0;
    config.imu_to_body_dcm[8] = 1.0;
    config.has_loose_non_holonomic = true;
    config.loose_non_holonomic.lateral_velocity_sigma_mps = 0.5;
    config.loose_non_holonomic.vertical_velocity_sigma_mps = 0.5;
    config.loose_non_holonomic.min_speed_mps = 0.1;
    config.loose_non_holonomic.max_body_rate_wrt_ecef_norm_rps = 1.0;
    nav.velocity_ecef_mps[0] = 2.0;
    nav.velocity_ecef_mps[1] = 0.4;
    nav.velocity_ecef_mps[2] = -0.2;
    filter = create_field_filter(&config, &nav, "fusion non-holonomic create");
    if (filter != NULL) {
        SidereonFusionUpdate update;
        bool present = false;
        check(sidereon_fusion_filter_update_non_holonomic(filter, &update, &present) ==
                  SIDEREON_STATUS_OK &&
                  present && update.applied && update.rows == 2 && update.accepted_rows == 2 &&
                  update.rejected_rows == 0,
              "fusion non-holonomic update");
        check_bits(update.nis, UINT64_C(0x3FA3813813813814),
                   "fusion non-holonomic NIS bits");
        SidereonFusionState state;
        check(sidereon_fusion_filter_state(filter, &state) == SIDEREON_STATUS_OK,
              "fusion non-holonomic state");
        const uint64_t nhc_velocity_bits[3] = {
            UINT64_C(0x4000000000000000),
            UINT64_C(0x3FD4B94B94B94B95),
            UINT64_C(0xBFC4B94B94B94B95),
        };
        check_vec_bits(state.velocity_ecef_mps, nhc_velocity_bits, 3,
                       "fusion non-holonomic velocity");
    }
    sidereon_fusion_filter_free(filter);
}

static void test_fusion_recorded_rts(void) {
    SidereonFusionFilterConfig config;
    check(sidereon_fusion_filter_config_init(&config) == SIDEREON_STATUS_OK,
          "fusion recorded config init");
    config.has_loose_innovation_gate = true;
    config.loose_innovation_gate.threshold_sigma = 4.0;
    config.loose_innovation_gate.min_rows = 2;
    config.has_loose_measurement_reweighting = true;
    config.loose_measurement_reweighting.k0_sigma = 2.0;
    config.loose_measurement_reweighting.k1_sigma = 5.0;
    config.has_loose_prediction_adaptation = true;
    config.loose_prediction_adaptation.threshold = 1.0;
    config.loose_prediction_adaptation.outlier_gate_probability = 0.99;

    SidereonFusionNavState nav;
    init_nav(&nav);
    double diag[15];
    for (size_t i = 0; i < 15; i++) {
        diag[i] = 1.0;
    }

    SidereonFusionFilter *filter = NULL;
    check(sidereon_fusion_filter_create(&nav, diag, 15, &config, &filter) ==
              SIDEREON_STATUS_OK &&
              filter != NULL,
          "fusion recorded create");
    if (filter == NULL) {
        return;
    }

    SidereonFusionRtsHistoryBuilder *builder = NULL;
    check(sidereon_fusion_rts_history_builder_from_filter(filter, &builder) ==
              SIDEREON_STATUS_OK &&
              builder != NULL,
          "fusion recorded history from filter");
    if (builder == NULL) {
        sidereon_fusion_filter_free(filter);
        return;
    }

    SidereonFusionImuSample sample;
    memset(&sample, 0, sizeof(sample));
    sample.kind = SIDEREON_FUSION_IMU_SAMPLE_KIND_RATE;
    sample.t_j2000_s = 1.0;
    check(sidereon_fusion_filter_propagate_recorded(filter, &sample, builder) ==
              SIDEREON_STATUS_OK,
          "fusion recorded propagate");

    double loose_cov[9] = {0.5, 0.0, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.5};
    SidereonFusionLooseMeasurement measurement;
    memset(&measurement, 0, sizeof(measurement));
    measurement.t_j2000_s = 1.0;
    measurement.position_ecef_m[0] = 6378137.35;
    measurement.position_ecef_m[1] = 0.2;
    measurement.position_ecef_m[2] = -0.1;
    measurement.covariance = loose_cov;
    measurement.covariance_len = 9;
    measurement.satellites_used = 7;
    measurement.solution_valid = true;
    measurement.fix_status = SIDEREON_FUSION_GNSS_FIX_STATUS_SINGLE;

    SidereonFusionUpdate update;
    check(sidereon_fusion_filter_update_loose_recorded(filter, &measurement, builder, &update) ==
              SIDEREON_STATUS_OK &&
              update.applied && update.rows == 3 && update.accepted_rows == 3 &&
              update.rejected_rows == 0,
          "fusion recorded loose update");
    check_bits(update.nis, UINT64_C(0x400A42AD3B07976F), "fusion recorded NIS bits");

    SidereonFusionState state;
    check(sidereon_fusion_filter_state(filter, &state) == SIDEREON_STATUS_OK,
          "fusion recorded state");
    check_bits(state.position_ecef_m[0], UINT64_C(0x415854A602757FB6),
               "fusion recorded state x bits");
    check_bits(state.position_ecef_m[1], UINT64_C(0x3FC7B6B11D7FA0D8),
               "fusion recorded state y bits");
    check_bits(state.position_ecef_m[2], UINT64_C(0xBFB7B6B11D5C2B22),
               "fusion recorded state z bits");

    SidereonFusionRtsHistory *history = NULL;
    check(sidereon_fusion_rts_history_builder_finish(builder, &history) ==
              SIDEREON_STATUS_OK &&
              history != NULL,
          "fusion recorded history finish");
    size_t count = 0;
    check(sidereon_fusion_rts_history_epoch_count(history, &count) == SIDEREON_STATUS_OK &&
              count == 2,
          "fusion recorded history count");

    SidereonFusionRtsEpoch epoch0, epoch1;
    check(sidereon_fusion_rts_history_epoch(history, 0, &epoch0) == SIDEREON_STATUS_OK &&
              epoch0.covariance_dimension == 15 && epoch0.augmented_dimension == 17 &&
              !epoch0.has_transition_from_previous,
          "fusion recorded epoch 0 summary");
    check(sidereon_fusion_rts_history_epoch(history, 1, &epoch1) == SIDEREON_STATUS_OK &&
              epoch1.covariance_dimension == 15 && epoch1.augmented_dimension == 17 &&
              epoch1.has_transition_from_previous,
          "fusion recorded epoch 1 summary");

    size_t written = 0, required = 0;
    double transition[225];
    check(sidereon_fusion_rts_history_epoch_transition_from_previous(
              history, 0, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
              written == 0 && required == 0,
          "fusion recorded first transition absent");
    check(sidereon_fusion_rts_history_epoch_transition_from_previous(
              history, 1, transition, 225, &written, &required) == SIDEREON_STATUS_OK &&
              written == 225 && required == 225,
          "fusion recorded transition copy");
    check_bits(transition[0], UINT64_C(0x3FF000019D17A15A),
               "fusion recorded transition xx bits");
    check_bits(transition[16], UINT64_C(0x3FEFFFFE650C7E2C),
               "fusion recorded transition yy bits");
    check_bits(transition[32], UINT64_C(0x3FEFFFFE639F13D3),
               "fusion recorded transition zz bits");

    SidereonSmoothedFusionTrajectory *smoothed = NULL;
    check(sidereon_smooth_fusion_rts(history, &smoothed) == SIDEREON_STATUS_OK &&
              smoothed != NULL,
          "fusion recorded smooth");
    check(sidereon_smoothed_fusion_trajectory_epoch_count(smoothed, &count) ==
              SIDEREON_STATUS_OK &&
              count == 2,
          "fusion recorded smoothed count");

    SidereonSmoothedFusionEpoch smoothed0, smoothed1;
    check(sidereon_smoothed_fusion_trajectory_epoch(smoothed, 0, &smoothed0) ==
              SIDEREON_STATUS_OK &&
              smoothed0.covariance_dimension == 17 && smoothed0.correction_len == 17 &&
              smoothed0.has_rts_gain_to_next,
          "fusion recorded smoothed epoch 0 summary");
    check(sidereon_smoothed_fusion_trajectory_epoch(smoothed, 1, &smoothed1) ==
              SIDEREON_STATUS_OK &&
              smoothed1.covariance_dimension == 17 && smoothed1.correction_len == 17 &&
              !smoothed1.has_rts_gain_to_next,
          "fusion recorded smoothed epoch 1 summary");

    double position[3];
    check(sidereon_smoothed_fusion_trajectory_epoch_position_ecef_m(
              smoothed, 0, position, 3, &written, &required) == SIDEREON_STATUS_OK &&
              written == 3 && required == 3,
          "fusion recorded smoothed position 0");
    check_bits(position[0], UINT64_C(0x415854A6AFB47DAB),
               "fusion recorded smoothed 0 x bits");
    check_bits(position[1], UINT64_C(0x3FB5122C16E56642),
               "fusion recorded smoothed 0 y bits");
    check_bits(position[2], UINT64_C(0xBFA5122C1780E0A5),
               "fusion recorded smoothed 0 z bits");
    check(sidereon_smoothed_fusion_trajectory_epoch_position_ecef_m(
              smoothed, 1, position, 3, &written, &required) == SIDEREON_STATUS_OK &&
              written == 3 && required == 3,
          "fusion recorded smoothed position 1");
    check_bits(position[0], UINT64_C(0x415854A602757FB6),
               "fusion recorded smoothed 1 x bits");
    check_bits(position[1], UINT64_C(0x3FC7B6B11D7FA0D8),
               "fusion recorded smoothed 1 y bits");
    check_bits(position[2], UINT64_C(0xBFB7B6B11D5C2B22),
               "fusion recorded smoothed 1 z bits");

    double correction[17];
    check(sidereon_smoothed_fusion_trajectory_epoch_error_state_correction(
              smoothed, 0, correction, 17, &written, &required) == SIDEREON_STATUS_OK &&
              written == 17 && required == 17,
          "fusion recorded smoothed correction");
    check_bits(correction[0], UINT64_C(0xBFFBED1F6AC3E068),
               "fusion recorded correction x bits");
    check_bits(correction[1], UINT64_C(0xBFB5122C16E56642),
               "fusion recorded correction y bits");
    check_bits(correction[2], UINT64_C(0x3FA5122C1780E0A5),
               "fusion recorded correction z bits");

    double covariance[289];
    check(sidereon_smoothed_fusion_trajectory_epoch_covariance(
              smoothed, 0, covariance, 289, &written, &required) == SIDEREON_STATUS_OK &&
              written == 289 && required == 289,
          "fusion recorded smoothed covariance");
    check_bits(covariance[0], UINT64_C(0x3FFDC64F219100F6),
               "fusion recorded covariance xx bits");
    check_bits(covariance[18], UINT64_C(0x3FFA44D611536A90),
               "fusion recorded covariance yy bits");
    check_bits(covariance[36], UINT64_C(0x3FFA44D6119F127C),
               "fusion recorded covariance zz bits");

    double rts_gain[289];
    check(sidereon_smoothed_fusion_trajectory_epoch_rts_gain_to_next(
              smoothed, 0, rts_gain, 289, &written, &required) == SIDEREON_STATUS_OK &&
              written == 289 && required == 289,
          "fusion recorded RTS gain");
    check(sidereon_smoothed_fusion_trajectory_epoch_rts_gain_to_next(
              smoothed, 1, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
              written == 0 && required == 0,
          "fusion recorded final RTS gain absent");

    sidereon_smoothed_fusion_trajectory_free(smoothed);
    sidereon_fusion_rts_history_free(history);
    sidereon_fusion_rts_history_builder_free(builder);
    sidereon_fusion_filter_free(filter);
}

int main(void) {
    test_signal_analysis();
    test_scenario();
    test_fusion();
    test_fusion_field_mode();
    test_fusion_recorded_rts();
    if (failures != 0) {
        fprintf(stderr, "domain018_smoke failures: %d\n", failures);
        return 1;
    }
    return 0;
}
