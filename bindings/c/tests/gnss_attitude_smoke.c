#include "sidereon.h"
#include "gnss_attitude_pins.h"

#include <assert.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static void exact(double value, uint64_t expected) {
    uint64_t actual;
    memcpy(&actual, &value, sizeof(actual));
    if (actual != expected) {
        fprintf(stderr, "actual %016llx expected %016llx\n",
                (unsigned long long)actual, (unsigned long long)expected);
        abort();
    }
}

static void vector(const double value[3], const uint64_t expected[3]) {
    for (size_t i = 0; i < 3; ++i) {
        exact(value[i], expected[i]);
    }
}

static double from_bits(uint64_t bits) {
    double value;
    memcpy(&value, &bits, sizeof(value));
    return value;
}

static void error(SidereonEngineErrorFamily family, const char *detail) {
    SidereonEngineErrorInfo info;
    assert(sidereon_last_engine_error_info(&info) == SIDEREON_STATUS_OK);
    assert(info.family == family);
    uint8_t text[512];
    size_t written = 0, required = 0;
    assert(info.payload_len < sizeof(text));
    assert(sidereon_last_engine_error_payload(text, info.payload_len, &written,
                                             &required) == SIDEREON_STATUS_OK);
    assert(written == info.payload_len && required == written);
    text[written] = 0;
    if (!strstr((const char *)text, detail)) {
        fprintf(stderr, "actual %s expected %s\n", (const char *)text, detail);
        abort();
    }
}

static void evaluate_law(const SidereonGnssAttitudeLaw *law,
                     const SidereonOrbitAttitudeTrajectory *trajectory,
                     const uint64_t *pins, SidereonAttitudeMode mode) {
    double epoch = from_bits(pins[0]);
    SidereonAttitudeSample sample;
    assert(sidereon_orbit_attitude_trajectory_sample(trajectory, epoch, &sample) ==
           SIDEREON_STATUS_OK);
    vector(sample.position_m, pins + 1);
    vector(sample.velocity_m_s, pins + 4);
    vector(sample.sun_position_m, pins + 7);

    SidereonGnssAttitude attitude;
    assert(sidereon_gnss_attitude(law, trajectory, epoch, &attitude) == SIDEREON_STATUS_OK);
    assert(attitude.mode == mode && attitude.has_dyb);
    exact(attitude.yaw_rad, pins[10]);
    exact(attitude.nominal_yaw_rad, pins[11]);
    exact(attitude.beta_rad, pins[12]);
    exact(attitude.orbit_angle_rad, pins[13]);
    vector(attitude.body_x, pins + 14);
    vector(attitude.body_y, pins + 17);
    vector(attitude.body_z, pins + 20);
    vector(attitude.dyb.d, pins + 23);
    vector(attitude.dyb.y, pins + 26);
    vector(attitude.dyb.b, pins + 29);
}

static void evaluate(const SidereonGnssAttitudeLaw *law,
                     const SidereonOrbitAttitudeTrajectory *trajectory,
                     const uint64_t *pins) {
    evaluate_law(law, trajectory, pins, SIDEREON_ATTITUDE_MODE_ORBIT_NORMAL);
}

static uint8_t *read_bytes(const char *path, size_t *length) {
    FILE *file = fopen(path, "rb");
    assert(file);
    assert(fseek(file, 0, SEEK_END) == 0);
    long bytes = ftell(file);
    assert(bytes > 0);
    *length = (size_t)bytes;
    assert(fseek(file, 0, SEEK_SET) == 0);
    uint8_t *data = malloc(*length);
    assert(data);
    assert(fread(data, 1, *length, file) == *length);
    assert(fclose(file) == 0);
    return data;
}

static SidereonClockEpoch gps_epoch(uint64_t seconds) {
    return (SidereonClockEpoch){
        .scale = SIDEREON_TIME_SCALE_GPST,
        .representation = SIDEREON_RINEX_CLOCK_INSTANT_REPRESENTATION_NANOS,
        .nanos_low = seconds * UINT64_C(1000000000),
    };
}

static void legacy_message(const char *expected) {
    char text[256];
    assert(sidereon_last_error_message(text, sizeof(text)) == strlen(expected));
    assert(strcmp(text, expected) == 0);
}

static void earth_fixed(const SidereonGnssAttitudeLaw *law, const char *sp3_path,
                        const char *nav_path) {
    size_t size;
    uint8_t *bytes = read_bytes(sp3_path, &size);
    SidereonSp3 *sp3 = NULL, *reference = NULL;
    assert(sidereon_sp3_load(bytes, size, &sp3) == SIDEREON_STATUS_OK);
    assert(sidereon_sp3_load(bytes, size, &reference) == SIDEREON_STATUS_OK);
    free(bytes);

    SidereonIers2010EopSample eop[2] = {{
        .mjd_utc = 59025, .xp_arcsec = 0.1, .yp_arcsec = 0.2, .ut1_minus_utc_s = -0.2,
        .lod_s = 0.001, .dx_arcsec = 0.0001, .dy_arcsec = -0.0001, .has_pole_rates = true,
    }};
    eop[1] = eop[0];
    eop[1].mjd_utc = 59026;
    SidereonIers2010EarthOrientationProvider *iers = NULL;
    assert(sidereon_iers2010_earth_orientation_provider_new(eop, 2, &iers) ==
           SIDEREON_STATUS_OK);

    SidereonOrbitAttitudeTrajectory *trajectory = NULL;
    assert(sidereon_orbit_attitude_trajectory_from_sp3(sp3, "G03", iers, &trajectory) ==
           SIDEREON_STATUS_OK);
    sidereon_iers2010_earth_orientation_provider_free(iers);

    SidereonOrbitSource *source = NULL, *flagged = NULL;
    const SidereonClockEpoch origin = gps_epoch(0);
    assert(sidereon_predicted_sp3_with_cutoff(sp3, origin, (SidereonOrbitSource **)sp3) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    legacy_message("sidereon_predicted_sp3_with_cutoff: out overlaps product");
    assert(sidereon_predicted_sp3_from_prediction_flags(sp3, (SidereonOrbitSource **)sp3) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    legacy_message("sidereon_predicted_sp3_from_prediction_flags: out overlaps product");
    assert(sidereon_predicted_sp3_with_cutoff(sp3, origin, &source) == SIDEREON_STATUS_OK);
    assert(sidereon_predicted_sp3_from_prediction_flags(sp3, &flagged) == SIDEREON_STATUS_OK);
    sidereon_sp3_free(sp3);

    const double horizon[] = {0.0, 7200.0};
    const SidereonOrbitScoreOptions options = sidereon_orbit_score_options_default(horizon, 2);
    SidereonOrbitScoreReport *report = NULL;
    SidereonOrbitScoreSummary summary;
    assert(sidereon_score_orbit(source, reference, NULL, 0, &options, &report) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_orbit_score_report_summary(report, &summary) == SIDEREON_STATUS_OK);
    assert(summary.frame.kind == SIDEREON_FRAME_AGREEMENT_KIND_SAME_LABEL &&
           summary.source_reference_point == SIDEREON_ORBIT_REFERENCE_POINT_CENTER_OF_MASS);
    sidereon_orbit_score_report_free(report);
    assert(sidereon_score_orbit(flagged, reference, NULL, 0, &options, &report) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_orbit_score_report_summary(report, &summary) == SIDEREON_STATUS_OK);
    assert(summary.frame.kind == SIDEREON_FRAME_AGREEMENT_KIND_SAME_LABEL);
    sidereon_orbit_score_report_free(report);
    sidereon_orbit_source_free(flagged);
    assert(sidereon_orbit_attitude_trajectory_with_velocity_half_step_s(trajectory, 10.0) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_orbit_attitude_trajectory_with_velocity_half_step_s(trajectory, 1e-9) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    error(SIDEREON_ENGINE_ERROR_FAMILY_ORBIT_SCORE,
          "\"error\":{\"kind\":\"invalid_input\",\"fields\":{\"field\":\"velocity_half_step_s\","
          "\"reason\":\"below the 1e-8 s resolution of SP3 epochs\"}}");
    evaluate(law, trajectory, ATTITUDE_SP3);
    sidereon_orbit_attitude_trajectory_free(trajectory);

    assert(sidereon_orbit_attitude_trajectory_from_orbit_source(source, "G03", NULL,
                                                              &trajectory) == SIDEREON_STATUS_OK);
    sidereon_orbit_source_free(source);
    evaluate(law, trajectory, ATTITUDE_PREDICTED);
    sidereon_orbit_attitude_trajectory_free(trajectory);
    bytes = read_bytes(nav_path, &size);
    SidereonBroadcastEphemeris *nav = NULL;
    assert(sidereon_broadcast_ephemeris_parse_nav(bytes, size, &nav) == SIDEREON_STATUS_OK);
    free(bytes);
    assert(sidereon_orbit_attitude_trajectory_from_broadcast(nav, "G03", NULL,
                                                            &trajectory) == SIDEREON_STATUS_OK);
    assert(sidereon_broadcast_orbit_new(nav, origin, (SidereonOrbitSource **)nav) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    legacy_message("sidereon_broadcast_orbit_new: out overlaps ephemeris");
    assert(sidereon_broadcast_orbit_new(nav, origin, &source) == SIDEREON_STATUS_OK);
    assert(sidereon_broadcast_ephemeris_set_nav_message_preference(
               nav, SIDEREON_NAV_MESSAGE_PREFERENCE_PREFER_MODERN) == SIDEREON_STATUS_OK);
    uint32_t preference;
    assert(sidereon_broadcast_ephemeris_nav_message_preference(nav, &preference) ==
           SIDEREON_STATUS_OK && preference == SIDEREON_NAV_MESSAGE_PREFERENCE_PREFER_MODERN);
    sidereon_broadcast_ephemeris_free(nav);
    evaluate(law, trajectory, ATTITUDE_NAV);
    sidereon_orbit_attitude_trajectory_free(trajectory);

    assert(sidereon_orbit_attitude_trajectory_from_orbit_source(source, "G03", NULL,
                                                              &trajectory) == SIDEREON_STATUS_OK);
    SidereonOrbitSourceInfo info;
    assert(sidereon_orbit_source_info(source, &info) == SIDEREON_STATUS_OK);
    assert(info.time_scale == SIDEREON_TIME_SCALE_GPST && info.clock_includes_relativity &&
           info.frame == SIDEREON_ORBIT_FRAME_BROADCAST_DATUMS &&
           info.reference_point == SIDEREON_ORBIT_REFERENCE_POINT_ANTENNA_PHASE_CENTER);
    assert(sidereon_score_orbit(source, reference, NULL, 0, &options, &report) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_orbit_score_report_summary(report, &summary) == SIDEREON_STATUS_OK);
    assert(summary.frame.kind == SIDEREON_FRAME_AGREEMENT_KIND_BROADCAST_DATUMS &&
           summary.source_reference_point == SIDEREON_ORBIT_REFERENCE_POINT_ANTENNA_PHASE_CENTER);
    sidereon_orbit_score_report_free(report);
    sidereon_sp3_free(reference);
    sidereon_orbit_source_free(source);
    evaluate(law, trajectory, ATTITUDE_NAV);
    sidereon_orbit_attitude_trajectory_free(trajectory);

    SidereonTrajectorySample nodes[3] = {0};
    for (size_t i = 0; i < 3; ++i) {
        memcpy(nodes[i].satellite.bytes, "G03", 4);
        nodes[i].epoch = gps_epoch(646315199 + i);
        nodes[i].position_ecef_m[0] = 26000000.0;
        nodes[i].position_ecef_m[1] = 100.0 * (double)i;
        nodes[i].position_ecef_m[2] = 1000000.0;
    }
    assert(sidereon_orbit_trajectory_new((const uint8_t *)"IGS14", 5, origin, nodes, 3,
                                         &source) == SIDEREON_STATUS_OK);
    assert(sidereon_orbit_trajectory_with_clock_relativity_included(
               source, true, (SidereonOrbitSource **)source) == SIDEREON_STATUS_INVALID_ARGUMENT);
    legacy_message("sidereon_orbit_trajectory_with_clock_relativity_included: out overlaps source");
    assert(sidereon_orbit_attitude_trajectory_from_orbit_source(source, "G03", NULL,
                                                              &trajectory) == SIDEREON_STATUS_OK);
    assert(sidereon_orbit_attitude_trajectory_with_velocity_half_step_s(trajectory, 1.0) ==
           SIDEREON_STATUS_OK);
    sidereon_orbit_source_free(source);
    evaluate(law, trajectory, ATTITUDE_SAMPLED);
    sidereon_orbit_attitude_trajectory_free(trajectory);
}

static SidereonOrbitAttitudeTrajectory *eckstein_hechler(void) {
    SidereonZonalHarmonics gravity = sidereon_zonal_harmonics_earth();
    SidereonCircularElements elements = {7200.0, 0.001, 0.002, 0.5, 0.25, 0.125};
    SidereonEcksteinHechlerPropagator *eh = NULL;
    assert(sidereon_eckstein_hechler_propagator_from_mean_elements(
               646315200.25, elements, gravity, &eh) == SIDEREON_STATUS_OK);
    SidereonOrbitAttitudeTrajectory *trajectory = NULL;
    assert(sidereon_orbit_attitude_trajectory_from_eckstein_hechler(eh, &trajectory) ==
           SIDEREON_STATUS_OK);
    sidereon_eckstein_hechler_propagator_free(eh);
    return trajectory;
}

static void analytic(const SidereonGnssAttitudeLaw *law, const char *de_path) {
    SidereonOrbitAttitudeTrajectory *trajectory = eckstein_hechler();
    evaluate(law, trajectory, ATTITUDE_EH);
    if (de_path) {
        SidereonDeEphemeris *de = NULL;
        assert(sidereon_de_ephemeris_load(de_path, &de) == SIDEREON_STATUS_OK);
        assert(sidereon_orbit_attitude_trajectory_with_de_ephemeris(trajectory, de) ==
               SIDEREON_STATUS_OK);
        sidereon_de_ephemeris_free(de);
        evaluate(law, trajectory, ATTITUDE_DE);
        assert(sidereon_orbit_attitude_trajectory_with_de_ephemeris(trajectory, NULL) ==
               SIDEREON_STATUS_OK);
        evaluate(law, trajectory, ATTITUDE_EH);
    }
    sidereon_orbit_attitude_trajectory_free(trajectory);

    SidereonZonalHarmonics gravity = sidereon_zonal_harmonics_earth();
    SidereonAnalyticKeplerianElements elements = {
        .a_km = 7200.0, .e = 0.01, .i_rad = 0.5, .raan_rad = 0.25,
        .arg_perigee_rad = 0.75, .mean_anomaly_rad = 0.125,
    };
    SidereonBrouwerLyddanePropagator *bl = NULL;
    assert(sidereon_brouwer_lyddane_propagator_from_mean_elements(
               646315200.25, elements, gravity, 0.0, &bl) == SIDEREON_STATUS_OK);
    assert(sidereon_orbit_attitude_trajectory_from_brouwer_lyddane(bl, &trajectory) ==
           SIDEREON_STATUS_OK);
    sidereon_brouwer_lyddane_propagator_free(bl);
    evaluate(law, trajectory, ATTITUDE_BL);
    sidereon_orbit_attitude_trajectory_free(trajectory);
}

static void dense_coverage(const SidereonDenseOutput *dense, const uint64_t expected[2]) {
    SidereonDenseOutputCoverage coverage;
    assert(sidereon_dense_output_coverage_tdb_seconds(dense, &coverage) == SIDEREON_STATUS_OK);
    assert(coverage.has_coverage);
    exact(coverage.start_tdb_seconds, expected[0]);
    exact(coverage.end_tdb_seconds, expected[1]);
}

static void numerical(const SidereonGnssAttitudeLaw *law) {
    SidereonStatePropagationConfig config;
    assert(sidereon_state_propagation_config_init(&config) == SIDEREON_STATUS_OK);
    config.epoch_s = 646315200.25;
    const double position[] = {7000.0, 20.0, 30.0};
    const double velocity[] = {0.0, 7.5, 1.0};
    memcpy(config.position_km, position, sizeof(position));
    memcpy(config.velocity_km_s, velocity, sizeof(velocity));
    double end = config.epoch_s + 600.0;
    SidereonCartesianState final_state;
    SidereonDenseOutput *dense = NULL;
    assert(sidereon_propagate_state_with_dense_output(&config, end, NULL, NULL, 0, &final_state, &dense) ==
           SIDEREON_STATUS_OK);
    exact(final_state.epoch_s, ATTITUDE_FINAL[0]);
    vector(final_state.position_km, ATTITUDE_FINAL + 1);
    vector(final_state.velocity_km_s, ATTITUDE_FINAL + 4);
    dense_coverage(dense, ATTITUDE_DENSE_COVERAGE_FORWARD);
    assert(sidereon_dense_output_coverage_tdb_seconds(dense, NULL) == SIDEREON_STATUS_NULL_POINTER);
    assert(sidereon_dense_output_coverage_tdb_seconds(dense, (SidereonDenseOutputCoverage *)dense) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    SidereonOrbitAttitudeTrajectory *trajectory = NULL;
    assert(sidereon_orbit_attitude_trajectory_from_dense_output(dense, &trajectory) ==
           SIDEREON_STATUS_OK);
    sidereon_dense_output_free(dense);
    evaluate(law, trajectory, ATTITUDE_DENSE);

    SidereonGnssAttitude attitude;
    assert(sidereon_gnss_attitude(law, trajectory, 0.0, &attitude) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    error(SIDEREON_ENGINE_ERROR_FAMILY_GNSS_ATTITUDE,
          "\"error\":{\"kind\":\"trajectory_unavailable\",\"fields\":"
          "{\"t_s\":{\"decimal\":\"0\",\"bits_hex\":\"0000000000000000\"}}}");
    SidereonAttitudeSample sample;
    assert(sidereon_orbit_attitude_trajectory_sample(trajectory, 0.0, &sample) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    error(SIDEREON_ENGINE_ERROR_FAMILY_GNSS_ATTITUDE,
          "\"error\":{\"kind\":\"trajectory_unavailable\",\"fields\":"
          "{\"t_s\":{\"decimal\":\"0\",\"bits_hex\":\"0000000000000000\"}}}");
    sidereon_orbit_attitude_trajectory_free(trajectory);

    assert(sidereon_propagate_state_with_dense_output(&config, config.epoch_s - 600.0, NULL,
                                                    NULL, 0, &final_state, &dense) == SIDEREON_STATUS_OK);
    dense_coverage(dense, ATTITUDE_DENSE_COVERAGE_BACKWARD);
    sidereon_dense_output_free(dense);

    assert(sidereon_propagate_state_with_dense_output(&config, config.epoch_s, NULL,
                                                    NULL, 0, &final_state, &dense) == SIDEREON_STATUS_OK);
    assert(final_state.epoch_s == config.epoch_s);
    assert(memcmp(final_state.position_km, position, sizeof(position)) == 0);
    assert(memcmp(final_state.velocity_km_s, velocity, sizeof(velocity)) == 0);
    SidereonDenseOutputCoverage coverage = {true, 1.0, 2.0};
    assert(sidereon_dense_output_coverage_tdb_seconds(dense, &coverage) == SIDEREON_STATUS_OK);
    assert(!coverage.has_coverage);
    assert(coverage.start_tdb_seconds == 0.0 && coverage.end_tdb_seconds == 0.0);
    assert(sidereon_orbit_attitude_trajectory_from_dense_output(dense, &trajectory) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_orbit_attitude_trajectory_sample(trajectory, config.epoch_s, &sample) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    sidereon_orbit_attitude_trajectory_free(trajectory);
    sidereon_dense_output_free(dense);

    config.integrator = SIDEREON_PROPAGATION_INTEGRATOR_RK4;
    assert(sidereon_propagate_state_with_dense_output(&config, end, NULL, NULL, 0, &final_state, &dense) ==
           SIDEREON_STATUS_OK);
    assert(dense == NULL && final_state.epoch_s == end);
    assert(sidereon_dense_output_coverage_tdb_seconds(dense, &coverage) == SIDEREON_STATUS_NULL_POINTER);

    assert(sidereon_propagate_state_with_dense_output(&config, NAN, NULL, NULL, 0, &final_state, &dense) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    assert(dense == NULL);
    error(SIDEREON_ENGINE_ERROR_FAMILY_PROPAGATION,
          "\"error\":{\"kind\":\"invalid_input\",\"fields\":"
          "{\"message\":\"t_end_tdb_seconds not finite\",\"reason\":\"t_end_tdb_seconds not finite\"}}");

    config.integrator = SIDEREON_PROPAGATION_INTEGRATOR_DP54;
    config.has_drag = true;
    SidereonIers2010EopSample samples[2] = {{.mjd_utc = 59020}, {.mjd_utc = 59021}};
    SidereonIers2010EarthOrientationProvider *iers = NULL;
    assert(sidereon_iers2010_earth_orientation_provider_new(samples, 2, &iers) == SIDEREON_STATUS_OK);
    const SidereonPropagationContext context = {.earth_orientation = iers};
    const SidereonStatePropagationEnvironment environment = {.context = &context};
    assert(sidereon_propagate_state_with_dense_output(&config, end, &environment, NULL, 0, &final_state,
                                                      &dense) == SIDEREON_STATUS_UT1_OUTSIDE_COVERAGE);
    error(SIDEREON_ENGINE_ERROR_FAMILY_PROPAGATION,
          "\"error\":{\"kind\":\"ut1_outside_coverage\",\"fields\":{\"reason\":\"after_coverage\"}}");
    assert(dense == NULL);
    sidereon_iers2010_earth_orientation_provider_free(iers);

}

static void attitude_laws(void) {
    SidereonOrbitAttitudeTrajectory *trajectory = eckstein_hechler();
    SidereonGnssAttitudeLaw *laws[8] = {0};
    double iia_rate;
    assert(sidereon_gps_iia_default_yaw_rate_rad_s(1, &iia_rate) == SIDEREON_STATUS_OK);
    assert(sidereon_gnss_attitude_law_gps_iia(
               iia_rate, sidereon_gps_iia_default_yaw_bias_rad(),
               sidereon_gps_shadow_limit_rad(), &laws[0]) == SIDEREON_STATUS_OK);
    assert(sidereon_gnss_attitude_law_gps_iir(
               sidereon_gps_iir_default_yaw_rate_rad_s(), &laws[1]) == SIDEREON_STATUS_OK);
    assert(sidereon_gnss_attitude_law_gps_iif(
               sidereon_gps_iif_default_yaw_rate_rad_s(), sidereon_gps_iif_default_yaw_bias_rad(),
               sidereon_gps_shadow_limit_rad(), &laws[2]) == SIDEREON_STATUS_OK);
    assert(sidereon_gnss_attitude_law_glonass_m(
               sidereon_glonass_m_default_yaw_rate_rad_s(), sidereon_glonass_m_shadow_limit_rad(),
               &laws[3]) == SIDEREON_STATUS_OK);
    assert(sidereon_gnss_attitude_law_galileo_iov(
               sidereon_galileo_iov_beta_x_rad(), sidereon_galileo_iov_beta_y_rad(),
               &laws[4]) == SIDEREON_STATUS_OK);
    assert(sidereon_gnss_attitude_law_cosine_turn(sidereon_galileo_foc_turn(), &laws[5]) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_gnss_attitude_law_beidou_orbit_normal_switch(
               sidereon_beidou2_ys_to_on_beta_rad(), sidereon_beidou2_on_to_ys_beta_rad(),
               &laws[6]) == SIDEREON_STATUS_OK);
    assert(sidereon_gnss_attitude_law_beidou3_meo_unknown_manufacturer(
               sidereon_beidou3_cast_meo_turn(), sidereon_beidou3_secm_beta0_rad(),
               &laws[7]) == SIDEREON_STATUS_OK);
    for (size_t i = 0; i < 8; ++i) {
        evaluate_law(laws[i], trajectory, ATTITUDE_NOMINAL, SIDEREON_ATTITUDE_MODE_NOMINAL);
        sidereon_gnss_attitude_law_free(laws[i]);
    }

    SidereonGnssAttitudeLaw *law = NULL;
    assert(sidereon_gnss_attitude_law_fixed_beta_yaw_steering(1.5, &law) ==
           SIDEREON_STATUS_OK);
    evaluate_law(law, trajectory, ATTITUDE_FIXED_BETA, SIDEREON_ATTITUDE_MODE_MODIFIED_YAW_STEERING);
    sidereon_gnss_attitude_law_free(law);
    sidereon_orbit_attitude_trajectory_free(trajectory);
}

static void attitude_law_errors(void) {
    SidereonOrbitAttitudeTrajectory *trajectory = eckstein_hechler();
    const double epoch = 646315800.25;
    SidereonGnssAttitudeLaw *law = NULL;
    SidereonGnssAttitude value;

    assert(sidereon_gnss_attitude_law_fixed_beta_yaw_steering(0.0, &law) == SIDEREON_STATUS_OK);
    assert(sidereon_gnss_attitude(law, trajectory, epoch, &value) == SIDEREON_STATUS_INVALID_ARGUMENT);
    error(SIDEREON_ENGINE_ERROR_FAMILY_GNSS_ATTITUDE,
          "\"error\":{\"kind\":\"invalid_parameter\",\"fields\":{\"field\":\"beta0_rad\","
          "\"value\":{\"decimal\":\"0\",\"bits_hex\":\"0000000000000000\"}}}");
    sidereon_gnss_attitude_law_free(law);

    assert(sidereon_gnss_attitude_law_beidou3_meo_unknown_manufacturer(
               sidereon_beidou3_cast_meo_turn(), 1.5, &law) == SIDEREON_STATUS_OK);
    assert(sidereon_gnss_attitude(law, trajectory, epoch, &value) == SIDEREON_STATUS_INVALID_ARGUMENT);
    error(SIDEREON_ENGINE_ERROR_FAMILY_GNSS_ATTITUDE,
          "\"error\":{\"kind\":\"beidou_manufacturer_required\",\"fields\":"
          "{\"t_s\":{\"decimal\":\"646315800.25\",\"bits_hex\":\"41c342ff8c200000\"}}}");
    sidereon_gnss_attitude_law_free(law);

    const SidereonCosineTurnParameters turn = {
        .beta_limit_rad = 1e-9, .start_angle = SIDEREON_COSINE_TURN_START_ANGLE_ORBIT_ANGLE,
        .start_angle_rad = 0.1, .period_s = 1e7,
    };
    assert(sidereon_gnss_attitude_law_cosine_turn(turn, &law) == SIDEREON_STATUS_OK);
    assert(sidereon_gnss_attitude(law, trajectory, epoch, &value) == SIDEREON_STATUS_INVALID_ARGUMENT);
    error(SIDEREON_ENGINE_ERROR_FAMILY_GNSS_ATTITUDE,
          "\"error\":{\"kind\":\"turn_end_not_found\",\"fields\":"
          "{\"t_s\":{\"decimal\":\"646315800.25\",\"bits_hex\":\"41c342ff8c200000\"}}}");
    sidereon_gnss_attitude_law_free(law);

    assert(sidereon_gnss_attitude_law_gps_iia(1000.0, 0.0, 1.6, &law) == SIDEREON_STATUS_OK);
    assert(sidereon_gnss_attitude(law, trajectory, epoch, &value) == SIDEREON_STATUS_INVALID_ARGUMENT);
    error(SIDEREON_ENGINE_ERROR_FAMILY_GNSS_ATTITUDE,
          "\"error\":{\"kind\":\"outside_validated_domain\",\"fields\":"
          "{\"t_s\":{\"decimal\":\"646315800.25\",\"bits_hex\":\"41c342ff8c200000\"},"
          "\"reason\":\"GPS IIA maximum yaw rate above 1e5 times the orbital rate, where the shadow "
          "crossing's yaw is set only to the rate times the uncertainty of the shadow entry\"}}");
    sidereon_gnss_attitude_law_free(law);
    sidereon_orbit_attitude_trajectory_free(trajectory);
}

int main(int argc, char **argv) {
    assert(argc == 3 || argc == 4);
    SidereonGnssAttitudeLaw *law = NULL;
    assert(sidereon_gnss_attitude_law_orbit_normal(&law) == SIDEREON_STATUS_OK);
    earth_fixed(law, argv[1], argv[2]);
    analytic(law, argc == 4 ? argv[3] : NULL);
    numerical(law);
    attitude_laws();
    attitude_law_errors();
    sidereon_gnss_attitude_law_free(law);
    return 0;
}
