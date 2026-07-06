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
        check(byte_count == 13872, "fusion pinned encoded length");
        free(bytes);
    }

    SidereonFusionTimeSyncStatus status;
    check(sidereon_fusion_filter_time_sync_status(filter, &status) == SIDEREON_STATUS_OK &&
              status.imu_capacity == 8 && status.checkpoint_capacity == 4,
          "fusion time-sync status");
    sidereon_fusion_filter_free(filter);
}

int main(void) {
    test_signal_analysis();
    test_scenario();
    test_fusion();
    if (failures != 0) {
        fprintf(stderr, "domain018_smoke failures: %d\n", failures);
        return 1;
    }
    return 0;
}
