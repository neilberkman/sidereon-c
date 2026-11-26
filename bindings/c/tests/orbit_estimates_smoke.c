#include "sidereon.h"
#include "orbit_estimates_pins.h"

#include <assert.h>
#include <float.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static double from_bits(uint64_t bits) {
    double value;
    memcpy(&value, &bits, sizeof(value));
    return value;
}

static void exact(double value, uint64_t expected) {
    uint64_t actual;
    memcpy(&actual, &value, sizeof(actual));
    if (actual != expected) {
        fprintf(stderr, "actual %016llx expected %016llx\n",
                (unsigned long long)actual, (unsigned long long)expected);
        abort();
    }
}

static SidereonClockEpoch epoch(uint32_t scale, int64_t seconds) {
    SidereonClockEpoch result = {
        .scale = scale, .representation = SIDEREON_RINEX_CLOCK_INSTANT_REPRESENTATION_NANOS,
        .nanos_high = 0, .nanos_low = (uint64_t)seconds * UINT64_C(1000000000),
    };
    return result;
}

static void retained_error(SidereonEngineErrorFamily family, const char *kind,
                           const char *detail) {
    SidereonEngineErrorInfo info;
    assert(sidereon_last_engine_error_info(&info) == SIDEREON_STATUS_OK);
    assert(info.family == family);
    uint8_t *text = calloc(info.payload_len + 1, 1);
    assert(text);
    size_t written, required;
    assert(sidereon_last_engine_error_payload(text, info.payload_len, &written,
                                             &required) == SIDEREON_STATUS_OK);
    assert(written == info.payload_len && required == written);
    if (!strstr((char *)text, kind) || !strstr((char *)text, detail)) {
        fprintf(stderr, "actual %s expected %s %s\n", (char *)text, kind, detail);
        abort();
    }
    free(text);
}

static void legacy_message(const char *expected) {
    char text[512];
    assert(sidereon_last_error_message(text, sizeof(text)) == strlen(expected));
    assert(strcmp(text, expected) == 0);
}

static void scoring_equality(void) {
    SidereonClockEpoch epochs_a[] = {epoch(SIDEREON_TIME_SCALE_GPST, 60)};
    SidereonClockEpoch epochs_b[] = {epoch(SIDEREON_TIME_SCALE_GPST, 60)};
    const SidereonScoreEpochs left_epochs = {true, epochs_a, 1};
    const SidereonScoreEpochs right_epochs = {true, epochs_b, 1};
    bool equal;
    assert(sidereon_score_epochs_equal(left_epochs, right_epochs, &equal) ==
           SIDEREON_STATUS_OK && equal);
    epochs_b[0] = epoch(SIDEREON_TIME_SCALE_GPST, 61);
    assert(sidereon_score_epochs_equal(left_epochs, right_epochs, &equal) ==
           SIDEREON_STATUS_OK && !equal);
    epochs_b[0] = epochs_a[0];
    const SidereonScoreEpochs invalid = {.count = 1};
    assert(sidereon_score_epochs_equal(invalid, right_epochs, &equal) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    legacy_message("sidereon_score_epochs_equal: epoch count requires explicit instants");

    const uint8_t label_a[] = "IGS14", label_b[] = "IGS14";
    const SidereonFrameLabel left_labels[] = {{label_a, 5}, {(const uint8_t *)"IGS20", 5}};
    SidereonFrameLabel right_labels[] = {{label_b, 5}, {(const uint8_t *)"IGS20", 5}};
    const SidereonFrameLabelSet left_set = {left_labels, 2}, right_set = {right_labels, 2};
    const SidereonSp3FrameReconciliationOptions left_frame = {&left_set, 1, false};
    const SidereonSp3FrameReconciliationOptions right_frame = {&right_set, 1, false};
    assert(sidereon_sp3_frame_reconciliation_options_equal(left_frame, right_frame, &equal) ==
           SIDEREON_STATUS_OK && equal);
    right_labels[1].data = (const uint8_t *)"IGS08";
    assert(sidereon_sp3_frame_reconciliation_options_equal(left_frame, right_frame, &equal) ==
           SIDEREON_STATUS_OK && !equal);
    right_labels[1] = left_labels[1];

    const double left_edges[] = {0.0, 60.0};
    double right_edges[] = {0.0, 60.0};
    const SidereonSisreWeights left_weights = {1.0, 0.25};
    SidereonSisreWeights right_weights = {1.0, 0.25};
    SidereonOrbitScoreOptions left = sidereon_orbit_score_options_default(left_edges, 2);
    SidereonOrbitScoreOptions right = sidereon_orbit_score_options_default(right_edges, 2);
    left.epochs = left_epochs;
    right.epochs = right_epochs;
    left.sisre_weights = &left_weights;
    right.sisre_weights = &right_weights;
    left.frame_reconciliation = left_frame;
    right.frame_reconciliation = right_frame;
    assert(sidereon_orbit_score_options_equal(left, right, &equal) ==
           SIDEREON_STATUS_OK && equal);
    right_edges[1] = 120.0;
    assert(sidereon_orbit_score_options_equal(left, right, &equal) ==
           SIDEREON_STATUS_OK && !equal);
    right_edges[1] = 60.0;
    right_weights.radial = 0.5;
    assert(sidereon_orbit_score_options_equal(left, right, &equal) ==
           SIDEREON_STATUS_OK && !equal);
    right_weights.radial = 2.0;
    assert(sidereon_orbit_score_options_equal(left, right, &equal) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    retained_error(SIDEREON_ENGINE_ERROR_FAMILY_ORBIT_SCORE, "invalid_input",
                   "{\"field\":\"sisre_weights.radial\",\"reason\":\"not within 0..=1\"}");

    SidereonTrajectorySample left_sample = {
        .satellite = {{"G03"}}, .epoch = epochs_a[0], .position_ecef_m = {1.0, 2.0, 3.0},
        .has_clock_s = false, .clock_s = NAN,
    };
    SidereonTrajectorySample right_sample = left_sample;
    right_sample.clock_s = 1.0;
    assert(sidereon_trajectory_sample_equal(left_sample, right_sample, &equal) ==
           SIDEREON_STATUS_OK && equal);
    right_sample.has_clock_s = true;
    assert(sidereon_trajectory_sample_equal(left_sample, right_sample, &equal) ==
           SIDEREON_STATUS_OK && !equal);
    left_sample.has_clock_s = true;
    left_sample.clock_s = 1.0;
    assert(sidereon_trajectory_sample_equal(left_sample, right_sample, &equal) ==
           SIDEREON_STATUS_OK && equal);
}

static void scoring_conversions(void) {
    const SidereonVector3 position = {{26560000.0, 0.0, 0.0}};
    const SidereonVector3 fixed_velocity = {{0.0, 2000.0, 0.0}};
    SidereonVector3 velocity;
    assert(sidereon_inertial_velocity_from_earth_fixed(position, fixed_velocity, &velocity) ==
           SIDEREON_STATUS_OK);
    assert(velocity.xyz[0] == 0.0 &&
           velocity.xyz[1] == 2000.0 + 7.2921151467e-5 * 26560000.0 && velocity.xyz[2] == 0.0);
    SidereonOrbitRegime regime;
    bool present;
    assert(sidereon_orbit_regime_from_inertial_state(position, velocity, &present, &regime) ==
           SIDEREON_STATUS_OK && present && regime == SIDEREON_ORBIT_REGIME_MEO);
    SidereonSisreWeights weights;
    assert(sidereon_sisre_weights_montenbruck_2018(SIDEREON_GNSS_SYSTEM_GPS, regime, &present,
                                                  &weights) == SIDEREON_STATUS_OK && present);
    assert(weights.radial == 0.98 && weights.transverse_normal_squared == 1.0 / 49.0);
    bool equal;
    assert(sidereon_sisre_weight_selection_equal(NULL, NULL, &equal) ==
           SIDEREON_STATUS_OK && equal);
    assert(sidereon_sisre_weight_selection_equal(&weights, &weights, &equal) ==
           SIDEREON_STATUS_OK && equal);
    assert(sidereon_sisre_weight_selection_equal(NULL, &weights, &equal) ==
           SIDEREON_STATUS_OK && !equal);
}

static void sampled_orbit(void) {
    const SidereonVector3 diff = {{2.0, 3.0, 6.0}};
    const SidereonVector3 position = {{4.0, 0.0, 0.0}};
    const SidereonVector3 velocity = {{0.0, 2.0, 0.0}};
    SidereonRtnComponents components;
    assert(sidereon_rtn_components(diff, position, velocity, &components) ==
           SIDEREON_STATUS_OK);
    assert(components.radial_m == 2.0 && components.transverse_m == 3.0 &&
           components.normal_m == 6.0);
    const SidereonRtnComponents hand = {3.0, 4.0, 0.0};
    const SidereonSisreWeights weights = {1.0, 1.0};
    double sisre;
    assert(sidereon_sisre_weights_orbit_only_m(weights, hand, &sisre) == SIDEREON_STATUS_OK);
    assert(sisre == 5.0);
    assert(sidereon_sisre_weights_with_clock_m(weights, hand, 3.0, &sisre) ==
           SIDEREON_STATUS_OK && sisre == 4.0);

    uint8_t text[24];
    size_t text_written, text_required;
    assert(sidereon_orbit_regime_text(SIDEREON_ORBIT_REGIME_MEO, text, sizeof(text),
                                     &text_written, &text_required) == SIDEREON_STATUS_OK);
    assert(text_written == 3 && text_required == 3 && memcmp(text, "MEO", 3) == 0);
    assert(sidereon_scored_product_text(SIDEREON_SCORED_PRODUCT_REFERENCE, text, sizeof(text),
                                        &text_written, &text_required) == SIDEREON_STATUS_OK);
    assert(text_written == 17 && text_required == 17 && memcmp(text, "reference product", 17) == 0);

    const SidereonClockEpoch instant = epoch(SIDEREON_TIME_SCALE_GPST, 60);
    const SidereonTrajectorySample node = {
        .satellite = {.bytes = "G03"}, .epoch = instant,
        .position_ecef_m = {10.0, 20.0, 30.0}, .has_clock_s = true, .clock_s = 0.125,
    };
    const uint8_t frame[] = "IGS20";
    SidereonOrbitSource *trajectory = NULL, *other = NULL;
    assert(sidereon_orbit_trajectory_new(frame, sizeof(frame) - 1, instant, &node, 1,
                                         &trajectory) == SIDEREON_STATUS_OK);
    SidereonOrbitSourceState state;
    assert(sidereon_orbit_source_state_at_j2000_s(trajectory, "G03", 60, &state) ==
           SIDEREON_STATUS_OK);
    assert(state.position_ecef_m[0] == 10.0 && state.position_ecef_m[1] == 20.0 &&
           state.position_ecef_m[2] == 30.0 && state.clock_s.present &&
           state.clock_s.value == 0.125);
    SidereonOptionalOrbitScalar origin;
    assert(sidereon_orbit_source_prediction_origin_j2000_s(trajectory, "G03", &origin) ==
           SIDEREON_STATUS_OK && origin.present && origin.value == 60);
    assert(sidereon_orbit_trajectory_with_clock_relativity_included(trajectory, true,
                                                                   &other) == SIDEREON_STATUS_OK);
    bool equal = false;
    assert(sidereon_orbit_trajectory_equal(trajectory, trajectory, &equal) ==
           SIDEREON_STATUS_OK && equal);
    assert(sidereon_orbit_trajectory_equal(trajectory, other, &equal) ==
           SIDEREON_STATUS_OK && !equal);
    SidereonOrbitSourceInfo info;
    assert(sidereon_orbit_source_info(other, &info) == SIDEREON_STATUS_OK);
    assert(info.time_scale == SIDEREON_TIME_SCALE_GPST && info.clock_includes_relativity);
    assert(info.reference_point == SIDEREON_ORBIT_REFERENCE_POINT_CENTER_OF_MASS);
    assert(info.frame == SIDEREON_ORBIT_FRAME_LABEL && !info.has_sp3_time_system);
    uint8_t label[5];
    assert(sidereon_orbit_source_frame_label(other, label, 5, &text_written, &text_required) ==
           SIDEREON_STATUS_OK);
    assert(text_written == 5 && text_required == 5 && memcmp(label, "IGS20", 5) == 0);
    SidereonClockEpoch output_epoch;
    assert(sidereon_orbit_trajectory_epochs(other, &output_epoch, 1,
                                            &text_written, &text_required) == SIDEREON_STATUS_OK);
    assert(text_written == 1 && text_required == 1);
    assert(output_epoch.nanos_low == UINT64_C(60000000000) &&
           output_epoch.scale == SIDEREON_TIME_SCALE_GPST);
    SidereonSatelliteToken token;
    assert(sidereon_orbit_trajectory_satellites(other, &token, 1,
                                                &text_written, &text_required) == SIDEREON_STATUS_OK);
    assert(text_written == 1 && text_required == 1 && strcmp(token.bytes, "G03") == 0);
    sidereon_orbit_source_free(other);
    sidereon_orbit_source_free(trajectory);
    const SidereonClockEpoch invalid_origin = epoch(SIDEREON_TIME_SCALE_UTC, 60);
    assert(sidereon_orbit_trajectory_new(frame, sizeof(frame) - 1, invalid_origin, &node, 1,
                                         &trajectory) == SIDEREON_STATUS_INVALID_ARGUMENT);
    assert(trajectory == NULL);
    retained_error(SIDEREON_ENGINE_ERROR_FAMILY_ORBIT_SCORE,
                   "\"kind\":\"instant_time_system_mismatch\"", "\"found\":\"GPST\"");
}

static void estimates(void) {
    SidereonInertialPositionSample samples[8];
    for (size_t i = 0; i < 8; ++i) {
        samples[i].epoch = epoch(SIDEREON_TIME_SCALE_TDB, (int64_t)i * 60);
        for (size_t k = 0; k < 3; ++k) {
            samples[i].position_gcrf_m[k] = from_bits(ESTIMATE_OBSERVATIONS[3 * i + k]);
        }
    }
    SidereonOrbitParameterFitOptions options = sidereon_orbit_parameter_fit_options_default();
    assert(options.force_model == SIDEREON_PROPAGATION_FORCE_MODEL_EARTH_PHASE_A &&
           options.solar_shadow == NULL && options.earth_radiation_pressure == NULL &&
           options.mu_km3_s2 == NULL && options.drag == NULL && options.space_weather == NULL &&
           options.additional_forces == NULL && options.additional_force_count == 0 &&
           options.estimated_parameters == NULL && options.estimated_parameter_count == 0 &&
           options.propagation_context == NULL && options.observation_sigma_m == 1.0 &&
           options.max_iterations == 25 && options.convergence_threshold == 1e-3 &&
           options.max_condition_number == 1e12 && options.min_ledger_samples == 3);
    assert(options.integrator_options.integrator == SIDEREON_PROPAGATION_INTEGRATOR_DP54 &&
           options.integrator_options.abs_tol == 1e-9 && options.integrator_options.rel_tol == 1e-12 &&
           options.integrator_options.initial_step_s == 60.0 && options.integrator_options.min_step_s == 1e-6 &&
           options.integrator_options.max_step_s == 3600.0 && options.integrator_options.max_steps == 1000000);
    options.force_model = SIDEREON_PROPAGATION_FORCE_MODEL_TWO_BODY;
    SidereonOrbitParameterFit *fit = NULL;
    assert(sidereon_fit_inertial_orbit_parameters(samples, 8, "G03", NULL, &options,
                                                  &fit) == SIDEREON_STATUS_OK);
    SidereonOrbitParameterFitSummary summary;
    assert(sidereon_orbit_parameter_fit_summary(fit, &summary) == SIDEREON_STATUS_OK);
    assert(summary.residual_stats.n == 8 && summary.force_parameter_count == 0);
    assert(summary.parameter_count == 6 && summary.iterations == ESTIMATE_ITERATIONS);
    exact(summary.residual_stats.rms_3d_m, ESTIMATE_FIT[0]);
    exact(summary.seed_rms_3d_m, ESTIMATE_FIT[1]);
    exact(summary.condition_number, ESTIMATE_FIT[2]);
    for (size_t i = 0; i < 3; ++i) {
        exact(summary.initial_state.position_km[i], ESTIMATE_FIT_INITIAL[i]);
        exact(summary.initial_state.velocity_km_s[i], ESTIMATE_FIT_INITIAL[i + 3]);
    }
    const SidereonOrbitParameterName x = {(const uint8_t *)"x_km", 4};
    const SidereonOrbitParameterName y = {(const uint8_t *)"y_km", 4};
    SidereonOptionalOrbitScalar value;
    assert(sidereon_orbit_parameter_fit_sigma(fit, x, &value) == SIDEREON_STATUS_OK &&
           value.present);
    exact(value.value, ESTIMATE_FIT[3]);
    assert(sidereon_orbit_parameter_fit_correlation(fit, x, y, &value) ==
           SIDEREON_STATUS_OK && value.present);
    exact(value.value, ESTIMATE_FIT[4]);

    size_t written, required;
    assert(sidereon_orbit_parameter_fit_residuals(fit, NULL, 0, &written, &required) ==
           SIDEREON_STATUS_OK && written == 0 && required == 8);
    assert(sidereon_orbit_parameter_fit_covariance_matrix(fit, false, NULL, 0, &written,
                                                          &required) == SIDEREON_STATUS_OK);
    assert(written == 0 && required == 36);
    SidereonEstimatedForceParameter parameter;
    assert(sidereon_orbit_parameter_fit_force_parameter(fit, x, &parameter) ==
           SIDEREON_STATUS_OK && !parameter.present);
    uint8_t label[4];
    assert(sidereon_orbit_parameter_fit_parameter_name(fit, 0, label, 4, &written,
                                                        &required) == SIDEREON_STATUS_OK);
    assert(written == 4 && required == 4 && memcmp(label, "x_km", 4) == 0);
    assert(sidereon_state_parameter_name_count() == 6);
    assert(sidereon_state_parameter_name(2, label, 4, &written, &required) ==
           SIDEREON_STATUS_OK);
    assert(written == 4 && required == 4 && memcmp(label, "z_km", 4) == 0);
    SidereonOrbitParameterCovariance covariance;
    assert(sidereon_orbit_parameter_fit_covariance(fit, &covariance) == SIDEREON_STATUS_OK);
    assert(covariance.estimated && covariance.dimension == 6);
    exact(covariance.variance_factor, ESTIMATE_FIT_STATS[0]);
    exact((double)covariance.degrees_of_freedom, ESTIMATE_FIT_STATS[1]);
    exact(summary.residual_stats.radial_rms_m, ESTIMATE_FIT_STATS[2]);
    exact(summary.residual_stats.along_rms_m, ESTIMATE_FIT_STATS[3]);
    exact(summary.residual_stats.cross_rms_m, ESTIMATE_FIT_STATS[4]);
    assert(!summary.residual_stats.low_sample_count && strcmp(summary.satellite.bytes, "G03") == 0);
    exact(summary.initial_state.epoch_s, ESTIMATE_FIT_INITIAL[6]);
    double matrix[36], correlation[36];
    assert(sidereon_orbit_parameter_fit_covariance_matrix(fit, false, matrix, 36, &written,
                                                          &required) == SIDEREON_STATUS_OK);
    assert(written == 36 && required == 36);
    assert(sidereon_orbit_parameter_fit_covariance_matrix(fit, true, correlation, 36, &written,
                                                          &required) == SIDEREON_STATUS_OK);
    assert(written == 36 && required == 36);
    for (size_t i = 0; i < 36; ++i) {
        exact(matrix[i], ESTIMATE_COVARIANCE[i]);
        exact(correlation[i], ESTIMATE_CORRELATION[i]);
    }
    SidereonOrbitRtnResidual residuals[8];
    assert(sidereon_orbit_parameter_fit_residuals(fit, residuals, 8, &written, &required) ==
           SIDEREON_STATUS_OK && written == 8 && required == 8);
    for (size_t i = 0; i < 8; ++i) {
        exact(residuals[i].variational_epoch.tdb_j2000_s, ESTIMATE_RESIDUALS[5 * i]);
        exact(residuals[i].variational_epoch.elapsed_tt_s, ESTIMATE_RESIDUALS[5 * i + 1]);
        exact(residuals[i].radial_m, ESTIMATE_RESIDUALS[5 * i + 2]);
        exact(residuals[i].along_m, ESTIMATE_RESIDUALS[5 * i + 3]);
        exact(residuals[i].cross_m, ESTIMATE_RESIDUALS[5 * i + 4]);
    }

    SidereonOrbitParameterFit *other = NULL;
    assert(sidereon_fit_inertial_orbit_parameters_with_validity(
               samples, 8, "G03", NULL, &options, true, &other) == SIDEREON_STATUS_OK);
    SidereonOrbitParameterFitSummary other_summary;
    assert(sidereon_orbit_parameter_fit_summary(other, &other_summary) == SIDEREON_STATUS_OK);
    exact(other_summary.residual_stats.rms_3d_m, ESTIMATE_FIT[0]);
    bool equal;
    assert(sidereon_orbit_parameter_fit_equal(fit, other, &equal) == SIDEREON_STATUS_OK && equal);
    sidereon_orbit_parameter_fit_free(other);

    const SidereonEmpiricalRtnTermSelector term = {
        SIDEREON_RTN_AXIS_RADIAL, SIDEREON_EMPIRICAL_HARMONIC_CONSTANT,
    };
    SidereonForceModel *empirical = NULL;
    assert(sidereon_empirical_rtn_acceleration_with_zero_terms(&term, 1, &empirical) ==
           SIDEREON_STATUS_OK);
    const SidereonForceModel *forces[] = {empirical};
    const char expected_name[] = "empirical.radial.constant";
    const SidereonOrbitParameterName empirical_name = {
        (const uint8_t *)expected_name, sizeof(expected_name) - 1,
    };
    SidereonOrbitParameterFitOptions with_force = options;
    with_force.additional_forces = forces;
    with_force.additional_force_count = 1;
    with_force.estimated_parameters = &empirical_name;
    with_force.estimated_parameter_count = 1;
    assert(sidereon_fit_inertial_orbit_parameters(samples, 8, "G03", NULL, &with_force,
                                                  &other) == SIDEREON_STATUS_OK);
    assert(sidereon_orbit_parameter_fit_force_parameter(other, empirical_name, &parameter) ==
           SIDEREON_STATUS_OK && parameter.present);
    exact(parameter.initial_value, ESTIMATE_FITTED_FORCE[0]);
    exact(parameter.value, ESTIMATE_FITTED_FORCE[1]);
    assert(parameter.sigma.present);
    exact(parameter.sigma.value, ESTIMATE_FITTED_FORCE[2]);
    uint8_t force_name[sizeof(expected_name) - 1];
    assert(sidereon_orbit_parameter_fit_parameter_name(other, 6, force_name, sizeof(force_name),
                                                        &written, &required) == SIDEREON_STATUS_OK);
    assert(written == sizeof(force_name) && required == written &&
           memcmp(force_name, expected_name, written) == 0);
    assert(sidereon_orbit_parameter_fit_force_parameter_at(
               other, 0, NULL, 0, &written, &required, &parameter) == SIDEREON_STATUS_OK);
    assert(written == 0 && required == sizeof(force_name));
    assert(sidereon_orbit_parameter_fit_force_parameter_at(
               other, 0, force_name, sizeof(force_name) - 1, &written, &required, &parameter) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    legacy_message("sidereon_orbit_parameter_fit_force_parameter_at: out_name needs room for 25 entries");
    assert(written == 0 && required == sizeof(force_name));
    memset(force_name, 0, sizeof(force_name));
    assert(sidereon_orbit_parameter_fit_force_parameter_at(
               other, 0, force_name, sizeof(force_name), &written, &required, &parameter) ==
           SIDEREON_STATUS_OK);
    assert(written == sizeof(force_name) && required == written &&
           memcmp(force_name, expected_name, written) == 0 && parameter.present && parameter.sigma.present);
    exact(parameter.initial_value, ESTIMATE_FITTED_FORCE[0]);
    exact(parameter.value, ESTIMATE_FITTED_FORCE[1]);
    exact(parameter.sigma.value, ESTIMATE_FITTED_FORCE[2]);
    assert(sidereon_orbit_parameter_fit_force_parameter_at(
               other, 1, NULL, 0, &written, &required, &parameter) == SIDEREON_STATUS_INVALID_ARGUMENT);
    assert(written == 0 && required == 0);
    legacy_message("sidereon_orbit_parameter_fit_force_parameter_at: force parameter index 1 out of range");
    assert(sidereon_orbit_parameter_fit_equal(fit, other, &equal) == SIDEREON_STATUS_OK && !equal);
    sidereon_orbit_parameter_fit_free(other);
    sidereon_orbit_parameter_fit_free(fit);

    SidereonOrbitParameterFitOptions tides = options;
    assert(tides.gravity_tide_system == SIDEREON_GRAVITY_TIDE_SYSTEM_TIDE_FREE);
    tides.force_model = SIDEREON_PROPAGATION_FORCE_MODEL_COMPOSITE;
    tides.force_components.has_zonal = true;
    tides.force_components.zonal_max_degree = 3;
    tides.force_components.has_solid_earth_tide = true;
    tides.gravity_tide_system = SIDEREON_GRAVITY_TIDE_SYSTEM_ZERO_TIDE;
    assert(sidereon_fit_inertial_orbit_parameters(samples, 8, "G03", NULL, &tides, &fit) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_orbit_parameter_fit_summary(fit, &summary) == SIDEREON_STATUS_OK);
    exact(summary.residual_stats.rms_3d_m, ESTIMATE_ZERO_TIDE_FIT[0]);
    for (size_t i = 0; i < 3; ++i) {
        exact(summary.initial_state.position_km[i], ESTIMATE_ZERO_TIDE_FIT[1 + i]);
        exact(summary.initial_state.velocity_km_s[i], ESTIMATE_ZERO_TIDE_FIT[4 + i]);
    }
    sidereon_orbit_parameter_fit_free(fit);
    tides.gravity_tide_system = 99;
    assert(sidereon_fit_inertial_orbit_parameters(samples, 8, "G03", NULL, &tides, &fit) ==
           SIDEREON_STATUS_INVALID_ARGUMENT && fit == NULL);
    legacy_message("sidereon_fit_inertial_orbit_parameters: invalid gravity tide-system selector");

    assert(sidereon_fit_inertial_orbit_parameters(samples, 2, "G03", NULL, &options, &fit) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_orbit_parameter_fit_covariance(fit, &covariance) == SIDEREON_STATUS_OK);
    assert(!covariance.estimated && covariance.dimension == 0 &&
           covariance.variance_factor == 0.0 && covariance.degrees_of_freedom == 0);
    sidereon_orbit_parameter_fit_free(fit);

    const SidereonEngineErrorFamily family = SIDEREON_ENGINE_ERROR_FAMILY_ORBIT_PARAMETER_FIT;
    assert(sidereon_fit_inertial_orbit_parameters(samples, 2, "G03", NULL, &with_force, &fit) ==
           SIDEREON_STATUS_SOLVE);
    retained_error(family, "\"kind\":\"singular_normal_equations\"",
                   "\"fields\":{\"satellite\":\"G03\",\"condition_number\":"
                   "{\"decimal\":\"inf\",\"bits_hex\":\"7ff0000000000000\"},\"weakest\":[]}");
    sidereon_force_model_free(empirical);

    SidereonOrbitParameterFitOptions refused = options;
    refused.max_condition_number = 1.5;
    assert(sidereon_fit_inertial_orbit_parameters(samples, 8, "G03", NULL, &refused, &fit) ==
           SIDEREON_STATUS_SOLVE);
    retained_error(family, ESTIMATE_ERROR_ILL_CONDITIONED, "");
    refused = options;
    refused.max_iterations = 1;
    assert(sidereon_fit_inertial_orbit_parameters(samples, 8, "G03", NULL, &refused, &fit) ==
           SIDEREON_STATUS_SOLVE);
    retained_error(family, ESTIMATE_ERROR_DID_NOT_CONVERGE, "");

    SidereonForceModel *srp = NULL;
    assert(sidereon_solar_radiation_pressure_new(1.25, DBL_TRUE_MIN,
               sidereon_radiation_shadow_options_default(), &srp) == SIDEREON_STATUS_OK);
    const SidereonForceModel *radiation[] = {srp};
    const SidereonOrbitParameterName cr = {(const uint8_t *)"srp.cr", 6};
    refused = options;
    refused.additional_forces = radiation;
    refused.additional_force_count = 1;
    refused.estimated_parameters = &cr;
    refused.estimated_parameter_count = 1;
    assert(sidereon_fit_inertial_orbit_parameters(samples, 8, "G03", NULL, &refused, &fit) ==
           SIDEREON_STATUS_SOLVE);
    retained_error(family, "\"kind\":\"unobservable_parameter\"",
                   "\"fields\":{\"satellite\":\"G03\",\"name\":\"srp.cr\"}");
    sidereon_force_model_free(srp);

    assert(sidereon_fit_inertial_orbit_parameters(samples, 1, "G03", NULL, &options, &fit) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    retained_error(family, "\"kind\":\"fit\"",
                   "\"fields\":{\"cause\":{\"kind\":\"too_few_samples\",\"fields\":"
                   "{\"satellite\":\"G03\",\"got\":1,\"required\":2}}}");
    const SidereonCartesianState seed = {
        from_bits(ESTIMATE_FIT_INITIAL[6]), {7000.0, 20.0, 30.0}, {0.0, 7.5, 1.0},
    };
    samples[7].position_gcrf_m[0] = 1e200;
    assert(sidereon_fit_inertial_orbit_parameters(samples, 8, "G03", &seed, &options, &fit) ==
           SIDEREON_STATUS_SOLVE);
    retained_error(family, "\"kind\":\"numerical_failure\"",
                   "\"fields\":{\"satellite\":\"G03\",\"quantity\":\"sum of squared residuals\"}");
    samples[7].position_gcrf_m[0] = from_bits(ESTIMATE_OBSERVATIONS[21]);

    SidereonOrbitParameterFitOptions nested = options;
    nested.observation_sigma_m = 0.0;
    assert(sidereon_fit_inertial_orbit_parameters(samples, 8, "G03", NULL, &nested, &fit) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    retained_error(family, "\"kind\":\"fit\"",
                   "\"fields\":{\"cause\":{\"kind\":\"invalid_option\",\"fields\":"
                   "{\"field\":\"observation_sigma_m\",\"reason\":\"not finite and positive\"}}}");
    SidereonInertialPositionSample duplicate[] = {samples[0], samples[0]};
    assert(sidereon_fit_inertial_orbit_parameters(duplicate, 2, "G03", NULL, &options, &fit) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    retained_error(family, "\"kind\":\"fit\"",
                   "\"fields\":{\"cause\":{\"kind\":\"non_monotonic_epochs\",\"fields\":"
                   "{\"satellite\":\"G03\"}}}");
    SidereonCartesianState wrong_seed = seed;
    wrong_seed.epoch_s = 1.0;
    assert(sidereon_fit_inertial_orbit_parameters(samples, 8, "G03", &wrong_seed, &options, &fit) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    retained_error(family, "\"kind\":\"fit\"",
                   "\"fields\":{\"cause\":{\"kind\":\"invalid_epoch\",\"fields\":"
                   "{\"satellite\":\"G03\",\"reason\":\"initial-state seed epoch must match the first sample\"}}}");
    duplicate[1] = samples[1];
    duplicate[1].position_gcrf_m[0] = NAN;
    assert(sidereon_fit_inertial_orbit_parameters(duplicate, 2, "G03", NULL, &options, &fit) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    retained_error(family, "\"kind\":\"fit\"",
                   "\"fields\":{\"cause\":{\"kind\":\"invalid_observation\",\"fields\":"
                   "{\"satellite\":\"G03\",\"reason\":\"position components must be finite\"}}}");
    nested = options;
    nested.integrator_options.max_steps = 0;
    assert(sidereon_fit_inertial_orbit_parameters(samples, 8, "G03", &seed, &nested, &fit) ==
           SIDEREON_STATUS_SOLVE);
    retained_error(family, "\"kind\":\"fit\"",
                   "\"fields\":{\"cause\":{\"kind\":\"propagation\",\"fields\":"
                   "{\"satellite\":\"G03\",\"cause\":{\"kind\":\"max_steps_exceeded\",\"fields\":{}}}}}");

    const SidereonEarthRadiationPressureOptions earth = {
        .cr = 1.25, .area_to_mass_m2_kg = 0.02, .moon_eclipse_enabled = false,
    };
    nested = options;
    nested.force_model = SIDEREON_PROPAGATION_FORCE_MODEL_COMPOSITE;
    nested.earth_radiation_pressure = &earth;
    assert(sidereon_fit_inertial_orbit_parameters(samples, 8, "G03", NULL, &nested, &fit) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_orbit_parameter_fit_summary(fit, &summary) == SIDEREON_STATUS_OK);
    exact(summary.residual_stats.rms_3d_m, ESTIMATE_RADIATION_FIT[0]);
    for (size_t i = 0; i < 3; ++i) {
        exact(summary.initial_state.position_km[i], ESTIMATE_RADIATION_FIT[1 + i]);
        exact(summary.initial_state.velocity_km_s[i], ESTIMATE_RADIATION_FIT[4 + i]);
    }
    sidereon_orbit_parameter_fit_free(fit);

    nested = options;
    nested.force_model = SIDEREON_PROPAGATION_FORCE_MODEL_COMPOSITE;
    nested.force_components.has_two_body = false;
    SidereonCartesianState radial_seed = {
        from_bits(ESTIMATE_FIT_INITIAL[6]), {7000.0, 0.0, 0.0}, {0.0, 0.0, 0.0},
    };
    SidereonInertialPositionSample radial_samples[] = {
        {samples[0].epoch, {7000000.0, 0.0, 0.0}},
        {samples[1].epoch, {7000000.0, 0.0, 0.0}},
    };
    assert(sidereon_fit_inertial_orbit_parameters(radial_samples, 2, "G03", &radial_seed,
                                                  &nested, &fit) == SIDEREON_STATUS_SOLVE);
    retained_error(family, "\"kind\":\"fit\"",
                   "\"fields\":{\"cause\":{\"kind\":\"rtn_frame\",\"fields\":"
                   "{\"satellite\":\"G03\",\"cause\":{\"kind\":\"parallel_position_velocity\",\"fields\":{}}}}}");
    radial_seed.position_km[0] = 0.0;
    radial_samples[0].position_gcrf_m[0] = 0.0;
    radial_samples[1].position_gcrf_m[0] = 0.0;
    assert(sidereon_fit_inertial_orbit_parameters(radial_samples, 2, "G03", &radial_seed,
                                                  &nested, &fit) == SIDEREON_STATUS_SOLVE);
    retained_error(family, "\"kind\":\"fit\"",
                   "\"fields\":{\"cause\":{\"kind\":\"rtn_frame\",\"fields\":"
                   "{\"satellite\":\"G03\",\"cause\":{\"kind\":\"zero_position\",\"fields\":{}}}}}");

    options.estimated_parameters = &x;
    options.estimated_parameter_count = 1;
    assert(sidereon_fit_inertial_orbit_parameters(samples, 8, "G03", NULL, &options,
                                                  &fit) == SIDEREON_STATUS_INVALID_ARGUMENT);
    assert(fit == NULL);
    retained_error(SIDEREON_ENGINE_ERROR_FAMILY_ORBIT_PARAMETER_FIT,
                   "\"kind\":\"invalid_parameter\"",
                   "\"fields\":{\"name\":\"x_km\",\"reason\":\"is the name of an initial-state parameter\"}");
    legacy_message("sidereon_fit_inertial_orbit_parameters: estimated parameter \"x_km\" is the name of an initial-state parameter");
}

static void zero_score(SidereonErrorStats stats, bool present) {
    assert(stats.present == present && stats.rms_m == 0.0 && stats.mean_m == 0.0 &&
           stats.max_abs_m == 0.0);
}

static void full_coverage(SidereonScoreCoverage coverage, size_t clocked) {
    assert(coverage.epochs == ESTIMATE_EPOCHS && coverage.scored == ESTIMATE_SCORED &&
           coverage.clocked == clocked && coverage.outside_horizon_bins == ESTIMATE_EPOCHS - ESTIMATE_SCORED &&
           coverage.no_prediction_origin == 0 && coverage.source_satellite_missing == 0 &&
           coverage.source_epoch_missing == 0 && coverage.reference_satellite_missing == 0 &&
           coverage.reference_epoch_missing == 0);
}

static void space_weather_without_drag(const SidereonSp3 *product) {
    const char weather[] =
        "DATE,BSRN,ND,KP1,KP2,KP3,KP4,KP5,KP6,KP7,KP8,KP_SUM,AP1,AP2,AP3,AP4,AP5,AP6,"
        "AP7,AP8,AP_AVG,CP,C9,ISN,F10.7_OBS,F10.7_ADJ,F10.7_DATA_TYPE,F10.7_OBS_CENTER81,"
        "F10.7_OBS_LAST81,F10.7_ADJ_CENTER81,F10.7_ADJ_LAST81\n"
        "2024-05-09,2556,1,23,27,30,33,40,50,47,37,287,9,12,15,18,27,48,39,22,24,1.2,5,"
        "120,165.1,162.0,OBS,150.1,149.8,147.0,146.6\n";
    SidereonSpaceWeatherTable *table = NULL;
    assert(sidereon_space_weather_table_parse_csv(
               (const uint8_t *)weather, sizeof(weather) - 1, &table) == SIDEREON_STATUS_OK);
    const SidereonStatePropagationEnvironment environment = {.space_weather = table};
    SidereonOrbitFitReport *report = NULL;
    assert(sidereon_fit_sp3_precise_orbit_with_context(product, "G03", NULL, &environment, &report) ==
           SIDEREON_STATUS_SOLVE);
    assert(report == NULL);
    retained_error(SIDEREON_ENGINE_ERROR_FAMILY_ORBIT_FIT, "\"kind\":\"propagation\"",
                   "\"fields\":{\"satellite\":\"G03\",\"cause\":{\"kind\":\"invalid_input\","
                   "\"fields\":{\"message\":\"space weather source without drag\"}}}");
    sidereon_space_weather_table_free(table);
}

static void score_product(const char *path) {
    FILE *file = fopen(path, "rb");
    assert(file && fseek(file, 0, SEEK_END) == 0);
    long size = ftell(file);
    assert(size > 0 && fseek(file, 0, SEEK_SET) == 0);
    uint8_t *data = malloc((size_t)size);
    assert(data && fread(data, 1, (size_t)size, file) == (size_t)size);
    assert(fclose(file) == 0);
    SidereonSp3 *product = NULL;
    assert(sidereon_sp3_load(data, (size_t)size, &product) == SIDEREON_STATUS_OK);
    free(data);
    space_weather_without_drag(product);

    SidereonOrbitParameterFitOptions fit_options = sidereon_orbit_parameter_fit_options_default();
    fit_options.force_model = SIDEREON_PROPAGATION_FORCE_MODEL_TWO_BODY;
    SidereonOrbitParameterFit *sp3_fit = NULL, *permissive = NULL;
    assert(sidereon_fit_sp3_ecef_orbit_parameters(
               product, NULL, "G03", NULL, &fit_options, (SidereonOrbitParameterFit **)product) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    legacy_message("sidereon_fit_sp3_ecef_orbit_parameters: out overlaps product");
    assert(sidereon_fit_sp3_ecef_orbit_parameters_with_validity(
               product, NULL, "G03", NULL, &fit_options, false, (SidereonOrbitParameterFit **)product) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    legacy_message("sidereon_fit_sp3_ecef_orbit_parameters_with_validity: out overlaps product");
    assert(sidereon_fit_sp3_ecef_orbit_parameters(product, NULL, "G03", NULL, &fit_options,
                                                  &sp3_fit) == SIDEREON_STATUS_OK);
    assert(sidereon_fit_sp3_ecef_orbit_parameters_with_validity(
               product, NULL, "G03", NULL, &fit_options, true, &permissive) == SIDEREON_STATUS_OK);
    SidereonOrbitParameterFit *fits[] = {sp3_fit, permissive};
    for (size_t i = 0; i < 2; ++i) {
        SidereonOrbitParameterFitSummary fit;
        assert(sidereon_orbit_parameter_fit_summary(fits[i], &fit) == SIDEREON_STATUS_OK);
        exact(fit.residual_stats.rms_3d_m, ESTIMATE_SP3_FIT[0]);
        exact(fit.condition_number, ESTIMATE_SP3_FIT[1]);
        exact(fit.initial_state.epoch_s, ESTIMATE_SP3_FIT[2]);
        for (size_t k = 0; k < 3; ++k) {
            exact(fit.initial_state.position_km[k], ESTIMATE_SP3_FIT[3 + k]);
            exact(fit.initial_state.velocity_km_s[k], ESTIMATE_SP3_FIT[6 + k]);
        }
        sidereon_orbit_parameter_fit_free(fits[i]);
    }

    SidereonIers2010EopSample eop[2] = {{.mjd_utc = 59020}, {.mjd_utc = 59021}};
    SidereonIers2010EarthOrientationProvider *iers = NULL, *permissive_iers = NULL;
    assert(sidereon_iers2010_earth_orientation_provider_new(eop, 2, &iers) == SIDEREON_STATUS_OK);
    assert(sidereon_fit_sp3_ecef_orbit_parameters(product, iers, "G03", NULL, &fit_options,
                                                  &sp3_fit) == SIDEREON_STATUS_INVALID_ARGUMENT);
    retained_error(SIDEREON_ENGINE_ERROR_FAMILY_ORBIT_PARAMETER_FIT, "\"kind\":\"fit\"",
                   "\"fields\":{\"cause\":{\"kind\":\"frame\",\"fields\":"
                   "{\"satellite\":\"G03\",\"cause\":{\"kind\":\"ut1_outside_coverage\",\"fields\":"
                   "{\"reason\":\"after_coverage\"}}}}}");
    assert(sidereon_iers2010_earth_orientation_provider_with_validity(iers, true, &permissive_iers) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_fit_sp3_ecef_orbit_parameters(product, permissive_iers, "G03", NULL, &fit_options,
                                                  &sp3_fit) == SIDEREON_STATUS_INVALID_ARGUMENT);
    retained_error(SIDEREON_ENGINE_ERROR_FAMILY_ORBIT_PARAMETER_FIT, "\"kind\":\"fit\"",
                   "\"fields\":{\"cause\":{\"kind\":\"ut1_validity_mismatch\",\"fields\":"
                   "{\"fit\":\"strict\",\"provider\":\"orientation\",\"provider_mode\":\"permissive\"}}}");
    sidereon_iers2010_earth_orientation_provider_free(permissive_iers);
    sidereon_iers2010_earth_orientation_provider_free(iers);

    SidereonOrbitSource *source = NULL;
    const SidereonClockEpoch cutoff = epoch(SIDEREON_TIME_SCALE_GPST, 646315200);
    assert(sidereon_predicted_sp3_with_cutoff(product, cutoff, &source) == SIDEREON_STATUS_OK);
    const double edges[] = {0.0, 1800.0, 7200.0};
    SidereonOrbitScoreOptions options = sidereon_orbit_score_options_default(edges, 3);
    assert(!options.epochs.explicit_instants && options.epochs.instants == NULL &&
           options.epochs.count == 0 && options.horizon_bin_edges_s == edges &&
           options.horizon_bin_edge_count == 3 && options.triad == SIDEREON_TRIAD_VELOCITY_INERTIAL &&
           options.velocity_half_step_s == 0.5 && options.sisre_weights == NULL &&
           options.clock_datum == SIDEREON_CLOCK_DATUM_EPOCH_SYSTEM_MEDIAN &&
           options.gaps == SIDEREON_GAP_POLICY_COUNT && !options.frame_reconciliation.helmert &&
           options.frame_reconciliation.asserted_equivalent_label_sets == NULL &&
           options.frame_reconciliation.label_set_count == 0);
    const SidereonSisreWeights weights = {1.0, 0.02};
    options.sisre_weights = &weights;
    const char *satellites[] = {"G03"};
    SidereonOrbitScoreReport *report = NULL;
    assert(sidereon_score_orbit(source, product, satellites, 1, &options, &report) ==
           SIDEREON_STATUS_OK);
    SidereonOrbitScoreSummary result;
    assert(sidereon_orbit_score_report_summary(report, &result) == SIDEREON_STATUS_OK);
    full_coverage(result.coverage, 0);
    assert(result.frame.kind == SIDEREON_FRAME_AGREEMENT_KIND_SAME_LABEL);
    assert(result.source_reference_point == SIDEREON_ORBIT_REFERENCE_POINT_CENTER_OF_MASS);
    assert(result.satellite_count == 1 && result.system_count == 1);
    size_t written, required;
    SidereonHorizonBinStats bins[2];
    assert(sidereon_orbit_score_report_satellite_bins(report, 0, bins, 2, &written,
                                                      &required) == SIDEREON_STATUS_OK);
    assert(written == 2 && required == 2);
    assert(bins[0].count == ESTIMATE_BIN0_COUNT && bins[1].count == ESTIMATE_BIN1_COUNT);
    assert(bins[0].horizon_start_s == 0.0 && bins[0].horizon_end_s == 1800.0);
    assert(bins[1].horizon_start_s == 1800.0 && bins[1].horizon_end_s == 7200.0);
    for (size_t i = 0; i < 2; ++i) {
        assert(bins[i].clock_count == 0);
        zero_score(bins[i].radial, true);
        zero_score(bins[i].transverse, true);
        zero_score(bins[i].normal, true);
        zero_score(bins[i].orbit_3d, true);
        zero_score(bins[i].sisre_orbit, true);
        zero_score(bins[i].clock, false);
        zero_score(bins[i].sisre, false);
    }

    SidereonSatelliteScore satellite;
    assert(sidereon_orbit_score_report_per_satellite(report, &satellite, 1, &written,
                                                     &required) == SIDEREON_STATUS_OK);
    assert(written == 1 && required == 1 && strcmp(satellite.satellite.bytes, "G03") == 0);
    assert(satellite.prediction_origin_j2000_s.present &&
           satellite.prediction_origin_j2000_s.value == 646315200.0);
    assert(satellite.bin_count == 2 && !satellite.has_orbit_regime &&
           satellite.has_sisre_weights && satellite.sisre_weights.radial == 1.0 &&
           satellite.sisre_weights.transverse_normal_squared == 0.02);
    full_coverage(satellite.coverage, 0);
    SidereonSystemScore system;
    assert(sidereon_orbit_score_report_per_system(report, &system, 1, &written, &required) ==
           SIDEREON_STATUS_OK);
    assert(written == 1 && required == 1 && system.system == SIDEREON_GNSS_SYSTEM_GPS);
    assert(system.bin_count == 2);
    full_coverage(system.coverage, 0);
    assert(sidereon_orbit_score_report_system_bins(report, 0, bins, 2, &written, &required) ==
           SIDEREON_STATUS_OK);
    assert(written == 2 && required == 2 && bins[0].count == ESTIMATE_BIN0_COUNT);
    double copied_edges[3];
    assert(sidereon_orbit_score_report_horizon_bin_edges_s(report, copied_edges, 3, &written,
                                                           &required) == SIDEREON_STATUS_OK);
    assert(written == 3 && required == 3 && copied_edges[0] == 0.0 &&
           copied_edges[1] == 1800.0 && copied_edges[2] == 7200.0);
    bool equal;
    assert(sidereon_orbit_score_report_equal(report, report, &equal) ==
           SIDEREON_STATUS_OK && equal);
    SidereonOrbitScoreReport *unadjusted = NULL;
    options.clock_datum = SIDEREON_CLOCK_DATUM_UNADJUSTED;
    assert(sidereon_score_orbit(source, product, satellites, 1, &options, &unadjusted) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_orbit_score_report_equal(report, unadjusted, &equal) ==
           SIDEREON_STATUS_OK && !equal);
    assert(sidereon_orbit_score_report_summary(unadjusted, &result) == SIDEREON_STATUS_OK);
    full_coverage(result.coverage, ESTIMATE_SCORED);
    assert(sidereon_orbit_score_report_satellite_bins(unadjusted, 0, bins, 2, &written,
                                                      &required) == SIDEREON_STATUS_OK);
    for (size_t i = 0; i < 2; ++i) {
        assert(bins[i].clock_count == bins[i].count);
        zero_score(bins[i].clock, true);
        zero_score(bins[i].sisre, true);
    }
    sidereon_orbit_score_report_free(unadjusted);
    sidereon_orbit_score_report_free(report);
    sidereon_orbit_source_free(source);
    sidereon_sp3_free(product);
}

static void score_error(const char *kind, const char *fields) {
    char expected[512];
    int length = snprintf(expected, sizeof(expected),
                          "\"error\":{\"kind\":\"%s\",\"fields\":%s}", kind, fields);
    assert(length > 0 && (size_t)length < sizeof(expected));
    retained_error(SIDEREON_ENGINE_ERROR_FAMILY_ORBIT_SCORE, expected, "");
}

static SidereonSp3 *linear_sp3(const char *satellite, const char *time_system, double step_km,
                              bool polar, bool predicted, const char *frame) {
    char text[2048];
    int length = snprintf(text, sizeof(text),
        "#cP2020  6 25  0  0  0.00000000      11 ORBIT %5s FIT  IGS\n"
        "## 2111 345600.00000000   900.00000000 59025 0.0000000000000\n"
        "+    1   %3s  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0\n"
        "%%c M  cc %3s ccc cccc cccc cccc cccc ccccc ccccc ccccc ccccc\n",
        frame, satellite, time_system);
    assert(length > 0 && (size_t)length < sizeof(text));
    size_t offset = (size_t)length;
    for (size_t i = 0; i < 11; ++i) {
        length = snprintf(text + offset, sizeof(text) - offset,
                          "*  2020  6 25 %2zu %2zu  0.00000000\nP%3s%14.6f%14.6f%14.6f%14.6f%20s\n",
                          i / 4, i % 4 * 15, satellite, polar ? 0.0 : 26560.0, ((double)i - 5.0) * step_km,
                          polar ? 26560.0 : 0.0, 0.0, predicted ? "P" : "");
        assert(length > 0 && (size_t)length < sizeof(text) - offset);
        offset += (size_t)length;
    }
    memcpy(text + offset, "EOF\n", 4);
    SidereonSp3 *product = NULL;
    assert(sidereon_sp3_load((const uint8_t *)text, offset + 4, &product) == SIDEREON_STATUS_OK);
    return product;
}

static SidereonOrbitSource *sampled_source(const char *frame, SidereonClockEpoch origin,
                                          SidereonTrajectorySample node) {
    SidereonOrbitSource *source = NULL;
    assert(sidereon_orbit_trajectory_new((const uint8_t *)frame, strlen(frame), origin, &node, 1,
                                         &source) == SIDEREON_STATUS_OK);
    return source;
}

static void score_refusals(void) {
    const SidereonClockEpoch origin = epoch(SIDEREON_TIME_SCALE_GPST, 646315200);
    SidereonClockEpoch query = epoch(SIDEREON_TIME_SCALE_GPST, 646319700);
    const double edges[] = {0.0, 7200.0};
    SidereonOrbitScoreOptions options = sidereon_orbit_score_options_default(edges, 2);
    options.epochs = (SidereonScoreEpochs){true, &query, 1};
    options.gaps = SIDEREON_GAP_POLICY_REFUSE;
    const char *gps[] = {"G03"};
    const char *qzss[] = {"J01"};
    const SidereonTrajectorySample node = {
        .satellite = {.bytes = "G03"}, .epoch = query, .position_ecef_m = {26560000.0, 0.0, 0.0},
    };
    SidereonSp3 *gps_reference = linear_sp3(gps[0], "GPS", 1800.0, false, false, "IGS14");
    SidereonSp3 *irn_reference = linear_sp3(gps[0], "IRN", 1800.0, false, false, "IGS14");
    SidereonSp3 *qzss_reference = linear_sp3(qzss[0], "GPS", 1800.0, false, false, "IGS14");
    SidereonSp3 *fixed_reference = linear_sp3(gps[0], "GPS", 0.0, false, false, "IGS14");
    SidereonSp3 *polar_reference = linear_sp3(gps[0], "GPS", 0.0, true, false, "IGS14");
    SidereonOrbitScoreReport *report = NULL;
    SidereonOrbitSource *source = sampled_source("IGS14", origin, node);

    assert(sidereon_score_orbit(source, irn_reference, gps, 1, &options, &report) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    score_error("time_system_label_mismatch", "{\"orbit_source\":\"GPS\",\"reference\":\"IRN\"}");
    query.scale = SIDEREON_TIME_SCALE_UTC;
    assert(sidereon_score_orbit(source, gps_reference, gps, 1, &options, &report) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    score_error("instant_time_system_mismatch",
                "{\"field\":\"epochs\",\"index\":0,\"found\":\"UTC\",\"expected\":\"GPST\"}");
    query.scale = SIDEREON_TIME_SCALE_GPST;

    assert(sidereon_score_orbit(source, gps_reference, qzss, 1, &options, &report) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    score_error("missing_satellite", "{\"product\":1,\"satellite\":\"J01\"}");
    query = epoch(SIDEREON_TIME_SCALE_GPST, 646319800);
    assert(sidereon_score_orbit(source, gps_reference, gps, 1, &options, &report) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    score_error("missing_epoch", "{\"product\":0,\"satellite\":\"G03\",\"epoch_index\":0,"
                                 "\"cause\":{\"kind\":\"epoch_out_of_range\",\"fields\":{}}}");
    query = node.epoch;

    assert(sidereon_score_orbit(source, fixed_reference, gps, 1, &options, &report) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    score_error("undetermined_orbit_regime", "{\"satellite\":\"G03\"}");
    options.triad = SIDEREON_TRIAD_VELOCITY_EARTH_FIXED;
    assert(sidereon_score_orbit(source, polar_reference, gps, 1, &options, &report) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    score_error("degenerate_reference_state", "{\"satellite\":\"G03\",\"epoch_index\":0}");
    options.triad = SIDEREON_TRIAD_VELOCITY_INERTIAL;
    sidereon_orbit_source_free(source);

    assert(sidereon_predicted_sp3_with_cutoff(qzss_reference, origin, &source) == SIDEREON_STATUS_OK);
    assert(sidereon_score_orbit(source, qzss_reference, qzss, 1, &options, &report) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    score_error("no_sisre_weights", "{\"satellite\":\"J01\",\"regime\":\"MEO\"}");
    sidereon_orbit_source_free(source);

    assert(sidereon_predicted_sp3_from_prediction_flags(gps_reference, &source) == SIDEREON_STATUS_OK);
    SidereonOptionalOrbitScalar prediction_origin;
    assert(sidereon_orbit_source_prediction_origin_j2000_s(source, "G03", &prediction_origin) ==
           SIDEREON_STATUS_OK && prediction_origin.present && prediction_origin.value == 646324200.0);
    SidereonSp3 *copy = NULL;
    assert(sidereon_predicted_sp3_product(source, &copy) == SIDEREON_STATUS_OK);
    size_t epochs;
    assert(sidereon_sp3_epoch_count(copy, &epochs) == SIDEREON_STATUS_OK && epochs == 11);
    sidereon_sp3_free(copy);
    uint8_t label[3];
    size_t written, required;
    assert(sidereon_orbit_source_sp3_time_system(source, label, sizeof(label), &written, &required) ==
           SIDEREON_STATUS_OK && written == 3 && required == 3 && memcmp(label, "GPS", 3) == 0);
    sidereon_orbit_source_free(source);

    SidereonSp3 *predicted_reference = linear_sp3(gps[0], "GPS", 1800.0, false, true, "IGS14");
    assert(sidereon_predicted_sp3_from_prediction_flags(predicted_reference, &source) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_score_orbit(source, gps_reference, gps, 1, &options, &report) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    score_error("no_prediction_origin", "{\"satellite\":\"G03\"}");
    sidereon_orbit_source_free(source);
    sidereon_sp3_free(predicted_reference);

    source = sampled_source("IGS20", origin, node);
    assert(sidereon_score_orbit(source, gps_reference, gps, 1, &options, &report) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    score_error("frame_mismatch", "{\"orbit_source\":\"IGS20\",\"reference\":\"IGS14\"}");
    sidereon_orbit_source_free(source);

    SidereonClockEpoch utc_origin = origin;
    utc_origin.scale = SIDEREON_TIME_SCALE_UTC;
    SidereonTrajectorySample utc_node = node;
    utc_node.epoch.scale = SIDEREON_TIME_SCALE_UTC;
    source = sampled_source("IGS14", utc_origin, utc_node);
    assert(sidereon_score_orbit(source, gps_reference, gps, 1, &options, &report) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    score_error("time_system_mismatch", "{\"orbit_source\":\"UTC\",\"reference\":\"GPST\"}");
    legacy_message("sidereon_score_orbit: orbit source time system UTC does not match reference time system GPST");
    sidereon_orbit_source_free(source);

    SidereonTrajectorySample enormous = node;
    enormous.has_clock_s = true;
    enormous.clock_s = DBL_MAX;
    source = sampled_source("IGS14", origin, enormous);
    assert(sidereon_score_orbit(source, gps_reference, gps, 1, &options, &report) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    score_error("non_finite_result", "{\"satellite\":\"G03\"}");
    sidereon_orbit_source_free(source);

    SidereonSp3 *new_frame = linear_sp3(gps[0], "GPS", 1800.0, false, false, "IGS20");
    const SidereonSisreWeights weights = {1.0, 0.02};
    options.sisre_weights = &weights;
    source = sampled_source("IGS14", origin, node);
    options.frame_reconciliation.helmert = true;
    assert(sidereon_score_orbit(source, new_frame, gps, 1, &options, &report) ==
           SIDEREON_STATUS_OK);
    SidereonOrbitScoreSummary summary;
    assert(sidereon_orbit_score_report_summary(report, &summary) == SIDEREON_STATUS_OK);
    assert(summary.frame.kind == SIDEREON_FRAME_AGREEMENT_KIND_HELMERT &&
           summary.frame.from == SIDEREON_TERRESTRIAL_FRAME_ITRF2014 &&
           summary.frame.to == SIDEREON_TERRESTRIAL_FRAME_ITRF2020 && summary.coverage.scored == 1);
    sidereon_orbit_score_report_free(report);
    const SidereonFrameLabel labels[] = {
        {(const uint8_t *)"IGS14", 5}, {(const uint8_t *)"IGS20", 5},
    };
    const SidereonFrameLabelSet equivalent = {labels, 2};
    options.frame_reconciliation.helmert = false;
    options.frame_reconciliation.asserted_equivalent_label_sets = &equivalent;
    options.frame_reconciliation.label_set_count = 1;
    assert(sidereon_score_orbit(source, new_frame, gps, 1, &options, &report) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_orbit_score_report_summary(report, &summary) == SIDEREON_STATUS_OK);
    assert(summary.frame.kind == SIDEREON_FRAME_AGREEMENT_KIND_ASSERTED_EQUIVALENT &&
           summary.frame.from == 0 && summary.frame.to == 0 && summary.coverage.scored == 1);
    sidereon_orbit_score_report_free(report);
    assert(sidereon_score_orbit(source, new_frame, NULL, 0, &options, &report) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_orbit_score_report_summary(report, &summary) == SIDEREON_STATUS_OK);
    assert(summary.satellite_count == 0 && summary.system_count == 0 &&
           summary.coverage.epochs == 0 && summary.coverage.scored == 0 &&
           summary.coverage.clocked == 0 && summary.coverage.outside_horizon_bins == 0 &&
           summary.coverage.no_prediction_origin == 0 && summary.coverage.source_satellite_missing == 0 &&
           summary.coverage.source_epoch_missing == 0 && summary.coverage.reference_satellite_missing == 0 &&
           summary.coverage.reference_epoch_missing == 0);
    sidereon_orbit_score_report_free(report);
    sidereon_orbit_source_free(source);
    options.frame_reconciliation.asserted_equivalent_label_sets = NULL;
    options.frame_reconciliation.label_set_count = 0;

    enormous = node;
    enormous.position_ecef_m[0] = DBL_MAX;
    source = sampled_source("IGS14", origin, enormous);
    options.frame_reconciliation.helmert = true;
    assert(sidereon_score_orbit(source, new_frame, gps, 1, &options, &report) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    score_error("frame_transform", "{\"cause\":{\"kind\":\"invalid_input\",\"fields\":"
                                   "{\"field\":\"x_m\",\"reason\":\"must be finite\"}}}");
    sidereon_orbit_source_free(source);
    sidereon_sp3_free(new_frame);

    assert(report == NULL);
    sidereon_sp3_free(gps_reference);
    sidereon_sp3_free(irn_reference);
    sidereon_sp3_free(qzss_reference);
    sidereon_sp3_free(fixed_reference);
    sidereon_sp3_free(polar_reference);
}

int main(int argc, char **argv) {
    sampled_orbit();
    scoring_conversions();
    scoring_equality();
    score_refusals();
    if (argc == 2) {
        estimates();
        score_product(argv[1]);
    }
    return 0;
}
