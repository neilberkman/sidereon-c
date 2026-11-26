#include "sidereon.h"
#include "orbit_models_pins.h"

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

static void vector(const double *actual, const uint64_t *expected, size_t count) {
    for (size_t i = 0; i < count; ++i) {
        exact(actual[i], expected[i]);
    }
}

static void error(SidereonEngineErrorFamily family, const char *kind,
                  const char *fields) {
    SidereonEngineErrorInfo info;
    assert(sidereon_last_engine_error_info(&info) == SIDEREON_STATUS_OK);
    assert(info.family == family);
    uint8_t *text = calloc(info.payload_len + 1, 1);
    assert(text);
    size_t written = 0, required = 0;
    assert(sidereon_last_engine_error_payload(text, info.payload_len, &written,
                                             &required) == SIDEREON_STATUS_OK);
    assert(written == info.payload_len && required == written);
    assert(strstr((char *)text, kind));
    assert(strstr((char *)text, fields));
    free(text);
}

static void attitude_and_shadow(void) {
    SidereonGnssBlock block;
    const uint8_t name[] = " block iir ";
    assert(sidereon_gnss_block_from_igs_name(name, sizeof(name) - 1, &block) ==
           SIDEREON_STATUS_OK);
    assert(block == SIDEREON_GNSS_BLOCK_GPS_IIR);
    size_t written, required;
    uint8_t label[11];
    assert(sidereon_gnss_block_igs_name(block, NULL, 0, &written, &required) ==
           SIDEREON_STATUS_OK);
    assert(written == 0 && required == 11);
    assert(sidereon_gnss_block_igs_name(block, label, 10, &written, &required) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    assert(written == 0 && required == 11);
    assert(sidereon_gnss_block_igs_name(block, label, 11, &written, &required) ==
           SIDEREON_STATUS_OK);
    assert(written == 11 && required == 11 && memcmp(label, "BLOCK IIR-B", 11) == 0);

    SidereonGnssAttitudeLaw *law = NULL, *same = NULL, *other = NULL;
    assert(sidereon_gnss_attitude_law_for_block(block, 0, &law) == SIDEREON_STATUS_OK);
    assert(sidereon_gnss_attitude_law_gps_iir(sidereon_gps_iir_default_yaw_rate_rad_s(),
                                           &same) == SIDEREON_STATUS_OK);
    assert(sidereon_gnss_attitude_law_orbit_normal(&other) == SIDEREON_STATUS_OK);
    bool equal;
    assert(sidereon_gnss_attitude_law_equal(law, same, &equal) == SIDEREON_STATUS_OK && equal);
    assert(sidereon_gnss_attitude_law_equal(law, other, &equal) == SIDEREON_STATUS_OK && !equal);
    sidereon_gnss_attitude_law_free(other);
    sidereon_gnss_attitude_law_free(same);
    sidereon_gnss_attitude_law_free(law);

    assert(sidereon_gnss_attitude_law_for_block(SIDEREON_GNSS_BLOCK_GPS_IIA, 0,
                                              &law) == SIDEREON_STATUS_INVALID_ARGUMENT);
    assert(law == NULL);
    error(SIDEREON_ENGINE_ERROR_FAMILY_GNSS_ATTITUDE, "\"kind\":\"no_default_yaw_rate\"",
          "\"fields\":{\"prn\":null}");
    assert(sidereon_gnss_block_from_igs_name((const uint8_t *)"BLOCK I", 7, &block) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    error(SIDEREON_ENGINE_ERROR_FAMILY_GNSS_BLOCK, "\"kind\":\"unsupported\"",
          "\"fields\":{\"name\":\"BLOCK I\"}");

    const double sat[] = {7000.0, 1.0, 0.0};
    const double sun[] = {150000000.0, 0.0, 0.0};
    const double moon[] = {-100000.0, 200000.0, 300000.0};
    SidereonEclipseStatus status;
    assert(sidereon_eclipse_status_with_moon(sat, sun, moon, &status) == SIDEREON_STATUS_OK);
    assert(status == SIDEREON_ECLIPSE_STATUS_SUNLIT);
    SidereonShadowGradient gradient;
    assert(sidereon_eclipse_shadow_fraction_with_gradient(sat, sun, &gradient) ==
           SIDEREON_STATUS_OK);
    assert(gradient.fraction == 0.0);
    assert(gradient.gradient_per_km[0] == 0.0 && gradient.gradient_per_km[1] == 0.0 &&
           gradient.gradient_per_km[2] == 0.0);
    const double zero[] = {0.0, 0.0, 0.0};
    assert(sidereon_eclipse_shadow_fraction_with_gradient(sat, zero, &gradient) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    error(SIDEREON_ENGINE_ERROR_FAMILY_SHADOW_GRADIENT, "\"kind\":\"eclipse\"",
          "\"fields\":{\"field\":\"sun_pos\",\"reason\":\"zero vector\"}");
}

static void variational(void) {
    SidereonForceModel *gravity = NULL, *empirical = NULL, *model = NULL;
    SidereonEmpiricalRtnTerm term = {
        .axis = SIDEREON_RTN_AXIS_RADIAL,
        .harmonic = SIDEREON_EMPIRICAL_HARMONIC_CONSTANT,
        .coefficient_km_s2 = 0.125,
    };
    assert(sidereon_two_body_gravity_new(8.0, &gravity) == SIDEREON_STATUS_OK);
    assert(sidereon_empirical_rtn_acceleration_new(&term, 1, &empirical) ==
           SIDEREON_STATUS_OK);
    const SidereonForceModel *components[] = {gravity, empirical};
    assert(sidereon_composite_force_model_new(components, 2, &model) == SIDEREON_STATUS_OK);
    sidereon_force_model_free(gravity);
    sidereon_force_model_free(empirical);
    const SidereonCartesianState state = {
        .epoch_s = 0.0, .position_km = {2.0, 0.0, 0.0}, .velocity_km_s = {0.0, 2.0, 0.0},
    };
    SidereonVector3 acceleration;
    assert(sidereon_force_model_acceleration(model, state, NULL, &acceleration) ==
           SIDEREON_STATUS_OK);
    assert(acceleration.xyz[0] == -1.875 && acceleration.xyz[1] == 0.0 &&
           acceleration.xyz[2] == 0.0);
    const double replacement = 0.25;
    assert(sidereon_force_model_acceleration_with_parameters(model, state, &replacement, 1, NULL,
                                                              &acceleration) == SIDEREON_STATUS_OK);
    assert(acceleration.xyz[0] == -1.75 && acceleration.xyz[1] == 0.0 &&
           acceleration.xyz[2] == 0.0);
    size_t count, written, required;
    assert(sidereon_force_model_parameter_count(model, &count) == SIDEREON_STATUS_OK);
    assert(count == 1);
    uint8_t name[64];
    double value;
    assert(sidereon_force_model_parameter(model, 0, name, sizeof(name), &written,
                                          &required, &value) == SIDEREON_STATUS_OK);
    assert(required == written && value == 0.125);
    assert(written == sizeof("empirical.radial.constant") - 1 &&
           memcmp(name, "empirical.radial.constant", written) == 0);
    SidereonVector3 derivative;
    assert(sidereon_force_model_parameter_partials(model, state, &value, 1, NULL,
                                                    &derivative, 1, &written, &required) ==
           SIDEREON_STATUS_OK);
    assert(written == 1 && required == 1);
    assert(derivative.xyz[0] == 1.0 && derivative.xyz[1] == 0.0 && derivative.xyz[2] == 0.0);

    size_t sensitivity = 0;
    SidereonVariationalEquations equations = {
        .force = model, .parameters = &value, .parameter_count = 1,
        .sensitivity_parameters = &sensitivity, .sensitivity_count = 1,
    };
    SidereonVariationalEpoch epoch = sidereon_variational_epoch_from_tdb_seconds(0.0, 0.0);
    assert(epoch.tdb_j2000_s == 0.0 && epoch.elapsed_tt_s == 0.0);
    SidereonVariationalTrajectory *result = NULL;
    assert(sidereon_propagate_variational(state, &epoch, 1, equations,
                                         sidereon_integrator_options_default(), NULL,
                                         &result) == SIDEREON_STATUS_OK);
    assert(sidereon_variational_trajectory_count(result, &count) == SIDEREON_STATUS_OK);
    assert(count == 1);
    SidereonVariationalState row;
    assert(sidereon_variational_trajectory_state(result, 0, &row) == SIDEREON_STATUS_OK);
    assert(row.state.position_km[0] == 2.0 && row.state.velocity_km_s[1] == 2.0);
    assert(row.sensitivity_columns == 1);
    for (size_t i = 0; i < 36; ++i) {
        assert(row.state_transition[i] == (i % 7 == 0 ? 1.0 : 0.0));
    }
    double sensitivities[6];
    assert(sidereon_variational_trajectory_sensitivity(result, 0, sensitivities, 6,
                                                       &written, &required) ==
           SIDEREON_STATUS_OK);
    assert(written == 6 && required == 6);
    for (size_t i = 0; i < 6; ++i) {
        assert(sensitivities[i] == 0.0);
    }
    sidereon_variational_trajectory_free(result);
    sidereon_force_model_free(model);
}

static void element_and_partial_values(void) {
    const SidereonMeanConversionOptions eh = sidereon_mean_conversion_options_eckstein_hechler_default();
    const SidereonMeanConversionOptions bl = sidereon_mean_conversion_options_brouwer_lyddane_default();
    assert(eh.tolerance == 1.0e-11 && eh.max_iterations == 100);
    assert(bl.tolerance == 1.0e-13 && bl.max_iterations == 200);
    SidereonCircularElements circle = {.a_km = 4.0, .ex = 0.25};
    assert(sidereon_circular_elements_eccentricity(circle) == 0.25);
    circle.ex = 0.0;
    SidereonAnalyticCartesian state;
    assert(sidereon_circular_elements_to_cartesian(circle, 4.0, &state) == SIDEREON_STATUS_OK);
    assert(state.position_km[0] == 4.0 && state.position_km[1] == 0.0 &&
           state.position_km[2] == 0.0 && state.velocity_km_s[0] == 0.0 &&
           state.velocity_km_s[1] == 1.0 && state.velocity_km_s[2] == 0.0);

    SidereonAnalyticKeplerianElements elements;
    const SidereonAnalyticCartesian reference = {{4.0, 0.0, 0.0}, {0.0, 1.0, 0.0}};
    assert(sidereon_analytic_keplerian_elements_from_cartesian(reference, 4.0, &elements) ==
           SIDEREON_STATUS_OK);
    assert(elements.a_km == 4.0 && elements.e == 0.0 && elements.i_rad == 0.0);
    const SidereonAnalyticKeplerianElements kepler = {.a_km = 4.0};
    assert(sidereon_analytic_keplerian_elements_to_cartesian(kepler, 4.0, &state) ==
           SIDEREON_STATUS_OK);
    assert(state.position_km[0] == 4.0 && state.position_km[1] == 0.0 &&
           state.position_km[2] == 0.0 && state.velocity_km_s[0] == 0.0 &&
           state.velocity_km_s[1] == 1.0 && state.velocity_km_s[2] == 0.0);

    uint8_t text[16];
    size_t written, required;
    assert(sidereon_analytic_theory_text(SIDEREON_ANALYTIC_THEORY_BROUWER_LYDDANE,
                                         text, sizeof(text), &written, &required) ==
           SIDEREON_STATUS_OK);
    assert(written == 15 && required == 15 && memcmp(text, "Brouwer-Lyddane", 15) == 0);

    const double matrix[] = {1, 2, 3, 4, 5, 6, 7, 8, 9};
    SidereonAccelerationPartials partials = sidereon_acceleration_partials_zeros();
    for (size_t i = 0; i < 9; ++i) {
        assert(partials.wrt_position[i] == 0.0 && partials.wrt_velocity[i] == 0.0);
    }
    assert(sidereon_acceleration_partials_position_only(matrix, &partials) == SIDEREON_STATUS_OK);
    for (size_t i = 0; i < 9; ++i) {
        assert(partials.wrt_position[i] == (double)(i + 1) && partials.wrt_velocity[i] == 0.0);
    }
}

static void empirical_coefficients(void) {
    SidereonForceModel *model = NULL, *modified = NULL, *all = NULL;
    assert(sidereon_empirical_rtn_acceleration_constant_terms(&model) == SIDEREON_STATUS_OK);
    assert(sidereon_empirical_rtn_acceleration_all_terms(&all) == SIDEREON_STATUS_OK);
    SidereonForceModel *empty = NULL;
    assert(sidereon_empirical_rtn_acceleration_new(NULL, 0, &empty) == SIDEREON_STATUS_OK);
    size_t empty_count;
    assert(sidereon_force_model_parameter_count(empty, &empty_count) == SIDEREON_STATUS_OK &&
           empty_count == 0);
    sidereon_force_model_free(empty);
    size_t count;
    assert(sidereon_force_model_parameter_count(model, &count) == SIDEREON_STATUS_OK && count == 3);
    assert(sidereon_force_model_parameter_count(all, &count) == SIDEREON_STATUS_OK && count == 9);
    sidereon_force_model_free(all);
    const SidereonEmpiricalRtnTerm term = {
        .axis = SIDEREON_RTN_AXIS_CROSS_TRACK, .harmonic = SIDEREON_EMPIRICAL_HARMONIC_SINE,
        .coefficient_km_s2 = 0.03125,
    };
    assert(sidereon_empirical_rtn_acceleration_with_term(model, term, &modified) ==
           SIDEREON_STATUS_OK);
    bool equal = true;
    assert(sidereon_empirical_rtn_acceleration_equal(model, modified, &equal) ==
           SIDEREON_STATUS_OK && !equal);
    SidereonOptionalOrbitScalar value;
    assert(sidereon_empirical_rtn_acceleration_coefficient_km_s2(
               modified, term.axis, term.harmonic, &value) == SIDEREON_STATUS_OK &&
           value.present && value.value == 0.03125);
    assert(sidereon_empirical_rtn_acceleration_coefficient_km_s2(
               model, term.axis, term.harmonic, &value) == SIDEREON_STATUS_OK && !value.present);
    SidereonEmpiricalRtnTerm terms[4];
    size_t written, required;
    assert(sidereon_empirical_rtn_acceleration_terms(modified, terms, 4, &written, &required) ==
           SIDEREON_STATUS_OK);
    assert(written == 4 && required == 4);
    assert(terms[3].axis == term.axis && terms[3].harmonic == term.harmonic &&
           terms[3].coefficient_km_s2 == 0.03125);
    uint8_t name[19];
    assert(sidereon_empirical_rtn_acceleration_parameter_name(
               term.axis, term.harmonic, name, sizeof(name), &written, &required) ==
           SIDEREON_STATUS_OK);
    assert(written == 19 && required == 19 && memcmp(name, "empirical.cross.sin", 19) == 0);
    sidereon_force_model_free(modified);
    sidereon_force_model_free(model);
}

static void force_configuration(void) {
    SidereonRtnAxis axes[3];
    SidereonEmpiricalHarmonic harmonics[3];
    assert(sidereon_rtn_axis_all(axes) == SIDEREON_STATUS_OK);
    assert(sidereon_empirical_harmonic_all(harmonics) == SIDEREON_STATUS_OK);
    assert(axes[0] == SIDEREON_RTN_AXIS_RADIAL && axes[1] == SIDEREON_RTN_AXIS_ALONG_TRACK &&
           axes[2] == SIDEREON_RTN_AXIS_CROSS_TRACK);
    assert(harmonics[0] == SIDEREON_EMPIRICAL_HARMONIC_CONSTANT &&
           harmonics[1] == SIDEREON_EMPIRICAL_HARMONIC_COSINE &&
           harmonics[2] == SIDEREON_EMPIRICAL_HARMONIC_SINE);

    const double values[] = {1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0};
    SidereonEcom2D2b1Coefficients coefficients;
    assert(sidereon_ecom2_d2b1_coefficients_from_array(values, &coefficients) ==
           SIDEREON_STATUS_OK);
    assert(coefficients.b0 == 1.0 && coefficients.b1c == 2.0 && coefficients.b1s == 3.0 &&
           coefficients.d0 == 4.0 && coefficients.d2c == 5.0 && coefficients.d2s == 6.0 &&
           coefficients.y0 == 7.0);
    assert(sidereon_ecom1_coefficient_count() == 9);
    assert(sidereon_ecom2_d2b1_coefficient_count() == 7);
    assert(sidereon_ecom2_d4b1_coefficient_count() == 9);
    SidereonRadiationShadowOptions shadow = sidereon_radiation_shadow_options_default();
    SidereonForceModel *model = NULL, *other = NULL;
    assert(sidereon_ecom2_solar_radiation_pressure_d2b1(coefficients, shadow, &model) ==
           SIDEREON_STATUS_OK);
    size_t n_d, n_b, count;
    assert(sidereon_ecom2_solar_radiation_pressure_n_d(model, &n_d) == SIDEREON_STATUS_OK);
    assert(sidereon_ecom2_solar_radiation_pressure_n_b(model, &n_b) == SIDEREON_STATUS_OK);
    assert(n_d == 1 && n_b == 1);
    assert(sidereon_ecom2_solar_radiation_pressure_coefficient_count(n_d, n_b, &count) ==
           SIDEREON_STATUS_OK && count == 7);
    assert(sidereon_ecom2_solar_radiation_pressure_with_moon_eclipse(model, false, &other) ==
           SIDEREON_STATUS_OK);
    bool enabled, equal;
    assert(sidereon_ecom2_solar_radiation_pressure_moon_eclipse_enabled(other, &enabled) ==
           SIDEREON_STATUS_OK && !enabled);
    assert(sidereon_ecom2_solar_radiation_pressure_equal(model, other, &equal) ==
           SIDEREON_STATUS_OK && !equal);
    sidereon_force_model_free(other);
    sidereon_force_model_free(model);

    SidereonStatePropagationConfig config;
    assert(sidereon_state_propagation_config_init(&config) == SIDEREON_STATUS_OK);
    config.epoch_s = 0.0;
    config.position_km[0] = 7000.0;
    config.velocity_km_s[1] = 7.5;
    double transition[36];
    assert(sidereon_state_transition_matrix_to_with_context(&config, 0.0, NULL, transition) ==
           SIDEREON_STATUS_OK);
    for (size_t i = 0; i < 36; ++i) {
        assert(transition[i] == (i % 7 == 0 ? 1.0 : 0.0));
    }
    const SidereonStatePropagationEnvironment environment = {.solar_shadow = &shadow};
    SidereonEphemeris *trajectory = NULL;
    const double target = 0.0;
    assert(sidereon_propagate_state_with_force_model(&config, &target, 1, &environment, NULL, 0,
                                                 &trajectory) == SIDEREON_STATUS_INVALID_ARGUMENT);
    assert(trajectory == NULL);
    char error_message[128];
    sidereon_last_error_message(error_message, sizeof(error_message));
    assert(strcmp(error_message,
                  "sidereon_propagate_state_with_force_model: radiation options require an additive force set") == 0);
}

static void orbit_values(void) {
    const double epoch = 646315200.25;
    exact(sidereon_tdb_minus_tt_seconds(epoch), ORBIT_TIME[0]);
    exact(sidereon_tdb_from_tt_j2000_seconds(epoch), ORBIT_TIME[1]);
    exact(sidereon_tt_from_tdb_j2000_seconds(epoch), ORBIT_TIME[2]);
    SidereonSunMoon bodies;
    assert(sidereon_sun_moon_gcrs_at_tdb_seconds(epoch, &bodies) == SIDEREON_STATUS_OK);
    vector(bodies.sun_m, ORBIT_SUN, 3);
    vector(bodies.moon_m, ORBIT_MOON, 3);

    const SidereonCartesianState state = {
        .epoch_s = epoch, .position_km = {7000.0, 20.0, 30.0},
        .velocity_km_s = {0.0, 7.5, 1.0},
    };
    const SidereonVector3 position = {{7000.0, 20.0, 30.0}};
    const SidereonVector3 velocity = {{0.0, 7.5, 1.0}};
    const SidereonVector3 sun = {{150000000.0, 20000000.0, -10000000.0}};
    const SidereonVector3 moon = {{-100000.0, 200000.0, 300000.0}};
    SidereonRadiationShadowOptions shadow = sidereon_radiation_shadow_options_default();
    assert(shadow.shadow_function == SIDEREON_SHADOW_FUNCTION_APPARENT_DISC);
    assert(shadow.earth_shadow_model == SIDEREON_EARTH_SHADOW_MODEL_WGS84_OBLATE);
    assert(shadow.moon_eclipse_enabled);
    SidereonForceModel *srp = NULL, *ecom = NULL;
    assert(sidereon_solar_radiation_pressure_new(1.25, 0.02, shadow, &srp) ==
           SIDEREON_STATUS_OK);
    SidereonVector3 accel;
    assert(sidereon_force_model_acceleration(srp, state, NULL, &accel) ==
           SIDEREON_STATUS_OK);
    vector(accel.xyz, ORBIT_SRP, 3);
    SidereonAccelerationPartials partials;
    const double cr = 1.25;
    assert(sidereon_force_model_acceleration_partials(srp, state, &cr, 1, NULL,
                                                      &partials) == SIDEREON_STATUS_OK);
    vector(partials.wrt_position, ORBIT_SRP_PARTIAL, 9);
    SidereonEcom1Coefficients coefficients = {.d0 = 1.0e-7};
    assert(sidereon_ecom1_solar_radiation_pressure_new(coefficients, shadow, &ecom) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_ecom1_solar_radiation_pressure_acceleration_with_sun_moon(
               ecom, position, velocity, sun, moon, &accel) == SIDEREON_STATUS_OK);
    vector(accel.xyz, ORBIT_ECOM, 3);
    assert(sidereon_ecom1_solar_radiation_pressure_acceleration_with_sun(
               ecom, position, velocity, sun, &accel) == SIDEREON_STATUS_INVALID_ARGUMENT);
    error(SIDEREON_ENGINE_ERROR_FAMILY_PROPAGATION, "\"kind\":\"invalid_input\"",
          "ECOM1 lunar eclipse is enabled; call acceleration_with_sun_moon");
    sidereon_force_model_free(ecom);
    sidereon_force_model_free(srp);

    SidereonEcomGeometry geometry;
    assert(sidereon_ecom_geometry(position, velocity, sun, &geometry) == SIDEREON_STATUS_OK);
    vector(geometry.dyb.d, ORBIT_ECOM_D, 3);
    SidereonAttitudeSample sample = {
        .position_m = {7000.0, 20.0, 30.0}, .velocity_m_s = {0.0, 7.5, 1.0},
        .sun_position_m = {150000000.0, 20000000.0, -10000000.0},
    };
    SidereonNominalYaw yaw;
    assert(sidereon_nominal_yaw(&sample, &yaw) == SIDEREON_STATUS_OK);
    exact(yaw.yaw_rad, ORBIT_YAW[0]);
    exact(yaw.beta_rad, ORBIT_YAW[1]);
    vector(yaw.body_x, ORBIT_BODY_X, 3);
    vector(yaw.body_y, ORBIT_BODY_Y, 3);
    vector(yaw.body_z, ORBIT_BODY_Z, 3);
    SidereonDybAxes dyb;
    assert(sidereon_dyb_axes(position.xyz, sun.xyz, &dyb) == SIDEREON_STATUS_OK);
    vector(dyb.d, ORBIT_DYB_D, 3);
    vector(dyb.y, ORBIT_DYB_Y, 3);
    vector(dyb.b, ORBIT_DYB_B, 3);

    const double eclipse_position[] = {-7000.0, 6378.0, 0.0};
    const double eclipse_sun[] = {150000000.0, 0.0, 0.0};
    SidereonShadowGradient eclipse;
    assert(sidereon_eclipse_shadow_fraction_with_model_and_gradient(
               eclipse_position, eclipse_sun, SIDEREON_EARTH_SHADOW_MODEL_WGS84_OBLATE,
               &eclipse) == SIDEREON_STATUS_OK);
    exact(eclipse.fraction, ORBIT_ECLIPSE[0]);
    vector(eclipse.gradient_per_km, ORBIT_ECLIPSE + 1, 3);

    SidereonZonalHarmonics gravity = sidereon_zonal_harmonics_earth();
    SidereonCircularElements circular = {7200.0, 0.001, 0.002, 0.5, 0.25, 0.125};
    SidereonEcksteinHechlerPropagator *eh = NULL;
    assert(sidereon_eckstein_hechler_propagator_from_mean_elements(
               epoch, circular, gravity, &eh) == SIDEREON_STATUS_OK);
    SidereonCartesianState output;
    assert(sidereon_eckstein_hechler_propagator_propagate(eh, epoch + 600.0, &output) ==
           SIDEREON_STATUS_OK);
    vector(output.position_km, ORBIT_ECKSTEIN_HECHLER, 3);
    vector(output.velocity_km_s, ORBIT_ECKSTEIN_HECHLER + 3, 3);
    assert(sidereon_eckstein_hechler_propagator_propagate(eh, NAN, &output) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    error(SIDEREON_ENGINE_ERROR_FAMILY_ANALYTIC_PROPAGATION, "\"kind\":\"non_finite\"",
          "\"fields\":{\"field\":\"epoch_tdb_seconds\"}");
    sidereon_eckstein_hechler_propagator_free(eh);

    SidereonAnalyticKeplerianElements kepler = {
        .a_km = 7200.0, .e = 0.01, .i_rad = 0.5, .raan_rad = 0.25,
        .arg_perigee_rad = 0.75, .mean_anomaly_rad = 0.125,
    };
    SidereonBrouwerLyddanePropagator *bl = NULL;
    assert(sidereon_brouwer_lyddane_propagator_from_mean_elements(
               epoch, kepler, gravity, 0.0, &bl) == SIDEREON_STATUS_OK);
    assert(sidereon_brouwer_lyddane_propagator_propagate(bl, epoch + 600.0, &output) ==
           SIDEREON_STATUS_OK);
    vector(output.position_km, ORBIT_BROUWER_LYDDANE, 3);
    vector(output.velocity_km_s, ORBIT_BROUWER_LYDDANE + 3, 3);
    sidereon_brouwer_lyddane_propagator_free(bl);

    SidereonIers2010EopSample eop[2] = {{
        .mjd_utc = 59025, .xp_arcsec = 0.1, .yp_arcsec = 0.2, .ut1_minus_utc_s = -0.2,
        .lod_s = 0.001, .dx_arcsec = 0.0001, .dy_arcsec = -0.0001, .has_pole_rates = true,
    }};
    eop[1] = eop[0];
    eop[1].mjd_utc = 59026;
    SidereonIers2010EarthOrientationProvider *provider = NULL;
    assert(sidereon_iers2010_earth_orientation_provider_new(eop, 2, &provider) ==
           SIDEREON_STATUS_OK);
    SidereonEarthOrientation orientation;
    assert(sidereon_iers2010_earth_orientation_provider_orientation_at_utc_mjd(
               provider, 59025, 43200.0, &orientation) == SIDEREON_STATUS_OK);
    assert(orientation.has_iers2010_quaternion);
    vector(orientation.iers2010_quaternion, ORBIT_QUATERNION, 4);
    sidereon_iers2010_earth_orientation_provider_free(provider);
}

static void products(const char *de_path, const char *field_path) {
    SidereonDeEphemeris *de = NULL;
    assert(sidereon_de_ephemeris_load(de_path, &de) == SIDEREON_STATUS_OK);
    uint32_t version;
    assert(sidereon_de_ephemeris_version(de, &version) == SIDEREON_STATUS_OK);
    assert(version == 440);
    SidereonDeCoverage interval;
    assert(sidereon_de_ephemeris_coverage_tdb_seconds(de, &interval) == SIDEREON_STATUS_OK);
    exact(interval.start_tdb_seconds, ORBIT_DE_COVERAGE[0]);
    exact(interval.end_tdb_seconds, ORBIT_DE_COVERAGE[1]);
    SidereonDeSunMoon bodies;
    assert(sidereon_de_ephemeris_sun_moon_at_tdb_seconds(de, interval.start_tdb_seconds + 3600.0,
                                                       &bodies) == SIDEREON_STATUS_OK);
    vector(bodies.sun.position_m, ORBIT_DE_SUN, 3);
    vector(bodies.sun.velocity_m_s, ORBIT_DE_SUN + 3, 3);
    assert(sidereon_de_ephemeris_sun_moon_at_tdb_seconds(de, NAN, &bodies) ==
           SIDEREON_STATUS_SOLVE);
    error(SIDEREON_ENGINE_ERROR_FAMILY_DE, "\"kind\":\"non_finite_epoch\"", "\"fields\":{}");
    sidereon_de_ephemeris_free(de);

    FILE *file = fopen(field_path, "rb");
    assert(file && fseek(file, 0, SEEK_END) == 0);
    const long size = ftell(file);
    assert(size > 0 && fseek(file, 0, SEEK_SET) == 0);
    const size_t n = (size_t)size;
    uint8_t *text = malloc(n);
    assert(text && fread(text, 1, n, file) == n);
    assert(fclose(file) == 0);
    SidereonIcgemGravityField *field = NULL;
    assert(sidereon_icgem_gravity_field_parse(text, n, &field) == SIDEREON_STATUS_OK);
    free(text);
    SidereonIcgemHeader header;
    assert(sidereon_icgem_gravity_field_header(field, &header) == SIDEREON_STATUS_OK);
    assert(header.format == SIDEREON_ICGEM_FORMAT_ICGEM2);
    SidereonSphericalHarmonicGravity *gravity = NULL;
    assert(sidereon_icgem_gravity_field_gravity(field, 2, 2, &gravity) == SIDEREON_STATUS_OK);
    sidereon_icgem_gravity_field_free(field);
    const SidereonVector3 position = {{7000.0, 20.0, 30.0}};
    double potential;
    assert(sidereon_spherical_harmonic_gravity_body_fixed_potential_at_km2_s2(
               gravity, position, 86400.0, &potential) == SIDEREON_STATUS_OK);
    exact(potential, ORBIT_ICGEM_POTENTIAL[0]);
    sidereon_spherical_harmonic_gravity_free(gravity);

    const uint8_t malformed[] = "modelname X\n";
    assert(sidereon_icgem_gravity_field_parse(malformed, sizeof(malformed) - 1, &field) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    assert(field == NULL);
    error(SIDEREON_ENGINE_ERROR_FAMILY_ICGEM, "\"kind\":\"missing_end_of_head\"", "\"fields\":{}");
}

int main(int argc, char **argv) {
    attitude_and_shadow();
    variational();
    force_configuration();
    element_and_partial_values();
    empirical_coefficients();
    if (argc == 3) {
        orbit_values();
        products(argv[1], argv[2]);
    }
    return 0;
}
