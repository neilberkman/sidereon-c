/*
 * Focused smoke for the 0.15 capability wave.
 */
#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sidereon.h"

static int fail(const char *what) {
    char message[512];
    size_t written = sidereon_last_error_message(message, sizeof(message));
    if (written > 0) {
        fprintf(stderr, "FAIL: %s: %s\n", what, message);
    } else {
        fprintf(stderr, "FAIL: %s\n", what);
    }
    return 1;
}

static int require_ok(SidereonStatus status, const char *what) {
    if (status != SIDEREON_STATUS_OK) {
        return fail(what);
    }
    return 0;
}

static bool close_abs(double actual, double expected, double tol) {
    return fabs(actual - expected) <= tol;
}

static bool close_rel(double actual, double expected, double rel) {
    double scale = fabs(expected) > 1.0 ? fabs(expected) : 1.0;
    return fabs(actual - expected) <= rel * scale;
}

static uint64_t f64_bits(double value) {
    uint64_t bits = 0;
    memcpy(&bits, &value, sizeof(bits));
    return bits;
}

static void set_sat_token(SidereonSatelliteToken *token, const char *text) {
    memset(token->bytes, 0, sizeof(token->bytes));
    memcpy(token->bytes, text, strlen(text));
}

static int test_error_metrics(void) {
    const double sigma = 3.0;
    const double cov[9] = {
        sigma * sigma, 0.0, 0.0,
        0.0, sigma * sigma, 0.0,
        0.0, 0.0, sigma * sigma,
    };
    SidereonPositionErrorMetrics metrics;
    SidereonErrorMetricsErrorKind err = SIDEREON_ERROR_METRICS_ERROR_KIND_NONE;
    if (require_ok(sidereon_error_metrics_from_enu_covariance_m2(cov, &metrics, &err),
                   "error metrics ENU") != 0) {
        return 1;
    }
    if (err != SIDEREON_ERROR_METRICS_ERROR_KIND_NONE) {
        return fail("error metrics unexpected error detail");
    }
    const double expected_cep = 1.177410 * sigma;
    if (!close_rel(metrics.cep_m.radius_m, expected_cep, 1.0e-6)) {
        fprintf(stderr, "FAIL: isotropic CEP %.17g expected %.17g\n", metrics.cep_m.radius_m,
                expected_cep);
        return 1;
    }
    const double expected_r95 = sqrt(-2.0 * log(1.0 - 0.95)) * sigma;
    if (!close_rel(metrics.r95_m.radius_m, expected_r95, 1.0e-12)) {
        return fail("isotropic R95");
    }
    if (!close_rel(metrics.drms_m, sqrt(2.0) * sigma, 1.0e-12)) {
        return fail("isotropic DRMS");
    }

    SidereonErrorEllipse ellipse;
    err = SIDEREON_ERROR_METRICS_ERROR_KIND_NONE;
    if (require_ok(sidereon_error_metrics_error_ellipse_from_enu_m2(cov, &ellipse, &err),
                   "error metrics ellipse from ENU") != 0) {
        return 1;
    }
    if (!close_rel(ellipse.semi_major_m, sigma, 1.0e-12) ||
        !close_rel(ellipse.semi_minor_m, sigma, 1.0e-12) ||
        !close_abs(ellipse.orientation_rad, 0.0, 1.0e-12)) {
        return fail("isotropic ellipse");
    }

    SidereonPercentileRadius radius;
    err = SIDEREON_ERROR_METRICS_ERROR_KIND_NONE;
    if (require_ok(sidereon_error_metrics_horizontal_radius_at(cov, 0.95, &radius, &err),
                   "error metrics horizontal radius") != 0) {
        return 1;
    }
    if (!close_rel(radius.radius_m, metrics.r95_m.radius_m, 1.0e-12) ||
        radius.approx_valid != metrics.r95_m.approx_valid) {
        return fail("horizontal radius helper");
    }
    err = SIDEREON_ERROR_METRICS_ERROR_KIND_NONE;
    if (require_ok(sidereon_error_metrics_spherical_radius_at(cov, 0.5, &radius, &err),
                   "error metrics spherical radius") != 0) {
        return 1;
    }
    if (!close_rel(radius.radius_m, metrics.sep_m.radius_m, 1.0e-12)) {
        return fail("spherical radius helper");
    }
    double vertical_radius = 0.0;
    err = SIDEREON_ERROR_METRICS_ERROR_KIND_NONE;
    if (require_ok(sidereon_error_metrics_vertical_radius_at(sigma * sigma, 0.5,
                                                             &vertical_radius, &err),
                   "error metrics vertical radius") != 0) {
        return 1;
    }
    if (!close_rel(vertical_radius, 0.6744897501960817 * sigma, 1.0e-12)) {
        return fail("vertical radius helper");
    }

    SidereonPositionCovariance position_covariance = {
        .ecef_m2 = {
            2.0 * sigma * sigma, 0.0, 0.0,
            0.0, 2.0 * sigma * sigma, 0.0,
            0.0, 0.0, 2.0 * sigma * sigma,
        },
        .enu_m2 = {
            sigma * sigma, 0.0, 0.0,
            0.0, sigma * sigma, 0.0,
            0.0, 0.0, sigma * sigma,
        },
    };
    SidereonPositionErrorMetrics position_metrics;
    err = SIDEREON_ERROR_METRICS_ERROR_KIND_NONE;
    if (require_ok(sidereon_error_metrics_from_position_covariance(
                       &position_covariance, &position_metrics, &err),
                   "error metrics position covariance") != 0) {
        return 1;
    }
    if (!close_rel(position_metrics.cep_m.radius_m, metrics.cep_m.radius_m, 1.0e-12) ||
        !close_rel(position_metrics.drms_m, metrics.drms_m, 1.0e-12)) {
        return fail("position covariance metrics");
    }

    const double elongated[9] = {
        9.0, 2.0, 0.0,
        2.0, 4.0, 0.0,
        0.0, 0.0, 1.44,
    };
    err = SIDEREON_ERROR_METRICS_ERROR_KIND_NONE;
    if (require_ok(sidereon_error_metrics_error_ellipse_from_enu_m2(elongated, &ellipse, &err),
                   "error metrics elongated ellipse") != 0) {
        return 1;
    }
    const double trace = 13.0;
    const double delta = sqrt((9.0 - 4.0) * (9.0 - 4.0) + 4.0 * 2.0 * 2.0);
    const double major_lambda = 0.5 * (trace + delta);
    const double minor_lambda = 0.5 * (trace - delta);
    if (!close_rel(ellipse.semi_major_m, sqrt(major_lambda), 1.0e-12) ||
        !close_rel(ellipse.semi_minor_m, sqrt(minor_lambda), 1.0e-12) ||
        !close_abs(ellipse.orientation_rad, 0.5 * atan2(4.0, 5.0), 1.0e-12)) {
        return fail("elongated ellipse oracle");
    }

    const double enu2[9] = {
        5.0, 0.25, 0.1,
        0.25, 2.0, -0.2,
        0.1, -0.2, 1.25,
    };
    const double ecef2[9] = {
        1.25, 0.1, -0.2,
        0.1, 5.0, 0.25,
        -0.2, 0.25, 2.0,
    };
    SidereonPositionErrorMetrics from_enu;
    SidereonPositionErrorMetrics from_ecef;
    const SidereonGeodetic receiver = {
        .lat_rad = 0.0,
        .lon_rad = 0.0,
        .height_m = 0.0,
    };
    err = SIDEREON_ERROR_METRICS_ERROR_KIND_NONE;
    if (require_ok(sidereon_error_metrics_from_enu_covariance_m2(enu2, &from_enu, &err),
                   "error metrics rotated ENU") != 0) {
        return 1;
    }
    err = SIDEREON_ERROR_METRICS_ERROR_KIND_NONE;
    if (require_ok(sidereon_error_metrics_from_ecef_covariance_m2(ecef2, receiver, &from_ecef,
                                                                  &err),
                   "error metrics rotated ECEF") != 0) {
        return 1;
    }
    if (!close_rel(from_ecef.cep_m.radius_m, from_enu.cep_m.radius_m, 1.0e-12) ||
        !close_rel(from_ecef.r95_m.radius_m, from_enu.r95_m.radius_m, 1.0e-12) ||
        !close_rel(from_ecef.drms_m, from_enu.drms_m, 1.0e-12)) {
        return fail("rotated ECEF agreement");
    }
    SidereonKinematicSolutionMetricsInput kinematic = {
        .position_m = {6378137.0, 0.0, 0.0},
        .position_covariance_m2 = {
            1.25, 0.1, -0.2,
            0.1, 5.0, 0.25,
            -0.2, 0.25, 2.0,
        },
    };
    SidereonPositionErrorMetrics from_kinematic;
    err = SIDEREON_ERROR_METRICS_ERROR_KIND_NONE;
    if (require_ok(sidereon_error_metrics_from_kinematic_solution(&kinematic, &from_kinematic,
                                                                  &err),
                   "error metrics kinematic") != 0) {
        return 1;
    }
    if (!close_rel(from_kinematic.cep_m.radius_m, from_enu.cep_m.radius_m, 1.0e-12) ||
        !close_rel(from_kinematic.r95_m.radius_m, from_enu.r95_m.radius_m, 1.0e-12) ||
        !close_rel(from_kinematic.drms_m, from_enu.drms_m, 1.0e-12)) {
        return fail("kinematic metrics agreement");
    }

    const double non_psd[9] = {
        1.0, 0.0, 0.0,
        0.0, -1.0, 0.0,
        0.0, 0.0, 1.0,
    };
    err = SIDEREON_ERROR_METRICS_ERROR_KIND_NONE;
    if (sidereon_error_metrics_from_enu_covariance_m2(non_psd, &metrics, &err) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        err != SIDEREON_ERROR_METRICS_ERROR_KIND_NOT_POSITIVE_SEMIDEFINITE) {
        return fail("non-PSD covariance typed error");
    }
    return 0;
}

static int test_sidereal(void) {
    double period = 0.0;
    if (require_ok(sidereon_sidereal_repeat_period(SIDEREON_GNSS_SYSTEM_GPS, &period),
                   "sidereal repeat period") != 0) {
        return 1;
    }
    if (!close_abs(period, 86164.0905, 1.0e-9)) {
        return fail("sidereal repeat period value");
    }

    SidereonSiderealFilterOptions options;
    if (require_ok(sidereon_sidereal_filter_options_init(&options),
                   "sidereal options init") != 0) {
        return 1;
    }
    options.sample_interval_s = 1.0;
    options.prior_periods = 1;
    options.min_coverage = 2;
    options.template_method = SIDEREON_SIDEREAL_TEMPLATE_METHOD_MEAN;
    double series[2] = {10.0, 20.0};
    SidereonSiderealFilterOutput *output = NULL;
    if (require_ok(sidereon_sidereal_filter(series, 2, 2.0, &options, &output),
                   "sidereal filter") != 0) {
        return 1;
    }
    bool under[2] = {false, false};
    size_t written = 0;
    size_t required = 0;
    if (require_ok(sidereon_sidereal_filter_output_under_covered(output, under, 2, &written,
                                                                 &required),
                   "sidereal under-covered") != 0) {
        sidereon_sidereal_filter_output_free(output);
        return 1;
    }
    sidereon_sidereal_filter_output_free(output);
    if (written != 2 || required != 2 || !under[0] || !under[1]) {
        return fail("sidereal under-covered passthrough");
    }
    return 0;
}

static int test_midas(void) {
    SidereonGeodeticPositionSample samples[5];
    const double rate[3] = {0.01, -0.02, 0.005};
    for (size_t i = 0; i < 5; i++) {
        double dt = (double)i;
        samples[i].epoch_year = 2020.0 + dt;
        samples[i].position_m[0] = rate[0] * dt;
        samples[i].position_m[1] = rate[1] * dt;
        samples[i].position_m[2] = rate[2] * dt;
        samples[i].has_covariance_m2 = false;
        memset(samples[i].covariance_m2, 0, sizeof(samples[i].covariance_m2));
    }
    SidereonGeodeticPositionSeries series = {
        .frame = SIDEREON_GEODETIC_TIME_SERIES_FRAME_ENU,
        .reference = {0.0, 0.0, 0.0},
        .samples = samples,
        .sample_count = 5,
    };
    SidereonMidasOptions options;
    if (require_ok(sidereon_geodetic_midas_options_init(&options), "MIDAS options") != 0) {
        return 1;
    }
    SidereonMidasVelocity velocity;
    if (require_ok(sidereon_geodetic_velocity_midas(&series, &options, &velocity),
                   "MIDAS velocity") != 0) {
        return 1;
    }
    for (int axis = 0; axis < 3; axis++) {
        if (!close_abs(velocity.rate_enu_m_per_yr[axis], rate[axis], 1.0e-12)) {
            return fail("MIDAS synthetic velocity");
        }
    }
    return 0;
}

static int test_clock_power_law(void) {
    double adev_slope = 0.0;
    double mdev_slope = 0.0;
    int variance_exp = 0;
    if (require_ok(sidereon_clock_power_law_noise_slopes(
                       SIDEREON_POWER_LAW_NOISE_TYPE_WHITE_FM, &adev_slope, &mdev_slope,
                       &variance_exp),
                   "WhiteFM slopes") != 0) {
        return 1;
    }
    if (adev_slope != -0.5 || mdev_slope != -0.5 || variance_exp != -1) {
        return fail("WhiteFM slope exact");
    }

    SidereonPowerLawNoiseOptions options;
    if (require_ok(sidereon_clock_power_law_noise_options_init(1.0, 0.5, &options),
                   "power-law options") != 0) {
        return 1;
    }
    options.slope_tolerance = 1.0e-12;
    options.scatter_tolerance = 1.0e-12;
    SidereonAllanPoint short_curve[1] = {{1.0, 1.0, 1}};
    SidereonPowerLawNoiseFit *fit = NULL;
    if (require_ok(sidereon_clock_fit_power_law_noise(short_curve, 1, short_curve, 1, &options,
                                                      &fit),
                   "power-law short fit") != 0) {
        return 1;
    }
    SidereonPowerLawOctave octave;
    size_t written = 0;
    size_t required = 0;
    if (require_ok(sidereon_clock_power_law_noise_fit_octaves(fit, &octave, 1, &written,
                                                              &required),
                   "power-law octaves") != 0) {
        sidereon_clock_power_law_noise_fit_free(fit);
        return 1;
    }
    sidereon_clock_power_law_noise_fit_free(fit);
    if (written != 1 || required != 1 ||
        octave.dominance_kind != SIDEREON_POWER_LAW_OCTAVE_DOMINANCE_KIND_FLAGGED ||
        octave.flag != SIDEREON_POWER_LAW_OCTAVE_FLAG_UNDER_SAMPLED) {
        return fail("power-law under-sampled flag");
    }
    return 0;
}

static int copy_ephemeris_states(SidereonEphemeris *eph, SidereonCartesianState *states,
                                 size_t count) {
    size_t written = 0;
    size_t required = 0;
    if (require_ok(sidereon_ephemeris_states(eph, states, count, &written, &required),
                   "ephemeris states") != 0) {
        return 1;
    }
    if (written != count || required != count) {
        return fail("ephemeris state count");
    }
    return 0;
}

static int test_composite_propagation(void) {
    double start = 0.0;
    if (require_ok(sidereon_civil_to_j2000_seconds(2026, 6, 1, 0, 0, 0.0, &start),
                   "civil to J2000") != 0) {
        return 1;
    }
    double epochs[3] = {start, start + 60.0, start + 120.0};
    SidereonStatePropagationConfig legacy;
    SidereonStatePropagationConfig composite;
    if (require_ok(sidereon_state_propagation_config_init(&legacy), "legacy propagation init") !=
            0 ||
        require_ok(sidereon_state_propagation_config_init(&composite),
                   "composite propagation init") != 0) {
        return 1;
    }
    legacy.epoch_s = start;
    legacy.position_km[0] = 7078.0;
    legacy.position_km[1] = -30.0;
    legacy.position_km[2] = 820.0;
    legacy.velocity_km_s[0] = 0.20;
    legacy.velocity_km_s[1] = 7.35;
    legacy.velocity_km_s[2] = 1.05;
    legacy.force_model = SIDEREON_PROPAGATION_FORCE_MODEL_TWO_BODY_J2;
    legacy.initial_step_s = 10.0;
    legacy.max_step_s = 60.0;
    composite = legacy;
    composite.force_model = SIDEREON_PROPAGATION_FORCE_MODEL_COMPOSITE;
    composite.force_components.has_two_body = true;
    composite.force_components.has_zonal = true;
    composite.force_components.zonal_max_degree = 2;
    composite.force_components.has_third_body = false;
    composite.force_components.has_solar_radiation_pressure = false;
    composite.force_components.has_relativity = false;

    SidereonEphemeris *legacy_eph = NULL;
    SidereonEphemeris *composite_eph = NULL;
    if (require_ok(sidereon_propagate_state(&legacy, epochs, 3, &legacy_eph),
                   "legacy propagation") != 0 ||
        require_ok(sidereon_propagate_state(&composite, epochs, 3, &composite_eph),
                   "composite propagation") != 0) {
        sidereon_ephemeris_free(legacy_eph);
        sidereon_ephemeris_free(composite_eph);
        return 1;
    }
    SidereonCartesianState a[3];
    SidereonCartesianState b[3];
    int rc = copy_ephemeris_states(legacy_eph, a, 3) || copy_ephemeris_states(composite_eph, b, 3);
    sidereon_ephemeris_free(legacy_eph);
    sidereon_ephemeris_free(composite_eph);
    if (rc != 0) {
        return 1;
    }
    if (memcmp(a, b, sizeof(a)) != 0) {
        return fail("composite propagation bit-for-bit parity");
    }
    return 0;
}

static int build_two_epoch_precise_samples(SidereonPreciseEphemerisSample samples[2]) {
    double start = 0.0;
    if (require_ok(sidereon_civil_to_j2000_seconds(2026, 6, 1, 0, 0, 0.0, &start),
                   "orbit start epoch") != 0) {
        return 1;
    }
    double epochs[2] = {start, start + 600.0};
    SidereonStatePropagationConfig cfg;
    if (require_ok(sidereon_state_propagation_config_init(&cfg), "orbit propagation init") != 0) {
        return 1;
    }
    cfg.epoch_s = start;
    cfg.position_km[0] = 7078.0;
    cfg.position_km[1] = 0.0;
    cfg.position_km[2] = 820.0;
    cfg.velocity_km_s[0] = 0.15;
    cfg.velocity_km_s[1] = 7.35;
    cfg.velocity_km_s[2] = 1.00;
    cfg.force_model = SIDEREON_PROPAGATION_FORCE_MODEL_TWO_BODY;
    cfg.initial_step_s = 10.0;
    cfg.max_step_s = 60.0;
    SidereonEphemeris *eph = NULL;
    if (require_ok(sidereon_propagate_state(&cfg, epochs, 2, &eph), "orbit truth propagation") !=
        0) {
        return 1;
    }
    SidereonCartesianState states[2];
    if (copy_ephemeris_states(eph, states, 2) != 0) {
        sidereon_ephemeris_free(eph);
        return 1;
    }
    sidereon_ephemeris_free(eph);

    SidereonTimeScales ts[2];
    if (require_ok(sidereon_timescales_from_utc(2026, 6, 1, 0, 0, 0.0, &ts[0]),
                   "orbit timescales 0") != 0 ||
        require_ok(sidereon_timescales_from_utc(2026, 6, 1, 0, 10, 0.0, &ts[1]),
                   "orbit timescales 1") != 0) {
        return 1;
    }
    for (size_t i = 0; i < 2; i++) {
        double itrs_km[3] = {0.0, 0.0, 0.0};
        if (require_ok(sidereon_frame_gcrs_to_itrs(states[i].position_km, &ts[i], false,
                                                   itrs_km),
                       "GCRS to ITRS") != 0) {
            return 1;
        }
        set_sat_token(&samples[i].sat, "G11");
        samples[i].time_scale = SIDEREON_TIME_SCALE_UTC;
        samples[i].epoch_j2000_s = epochs[i];
        samples[i].position_ecef_m[0] = itrs_km[0] * 1000.0;
        samples[i].position_ecef_m[1] = itrs_km[1] * 1000.0;
        samples[i].position_ecef_m[2] = itrs_km[2] * 1000.0;
        samples[i].has_clock_s = false;
        samples[i].clock_s = 0.0;
        samples[i].clock_event = false;
    }
    return 0;
}

static int test_sparse_orbit_fit(void) {
    SidereonPreciseEphemerisSample samples[2];
    if (build_two_epoch_precise_samples(samples) != 0) {
        return 1;
    }
    SidereonOrbitFitOptions options;
    if (require_ok(sidereon_orbit_fit_options_init(&options), "orbit fit options") != 0) {
        return 1;
    }
    options.force_model = SIDEREON_PROPAGATION_FORCE_MODEL_TWO_BODY;
    options.initial_step_s = 10.0;
    options.max_step_s = 60.0;
    options.solver_gtol = 1.0e-15;
    options.solver_ftol = 1.0e-15;
    options.solver_xtol = 1.0e-15;
    options.solver_max_nfev = 1200;
    SidereonOrbitFitReport *report = NULL;
    if (require_ok(sidereon_fit_precise_ephemeris_sample_orbit(samples, 2, "G11", &options,
                                                               &report),
                   "sparse orbit fit") != 0) {
        return 1;
    }
    SidereonOrbitFitSolution fit;
    size_t written = 0;
    size_t required = 0;
    if (require_ok(sidereon_orbit_fit_report_fits(report, &fit, 1, &written, &required),
                   "orbit fit solution") != 0) {
        sidereon_orbit_fit_report_free(report);
        return 1;
    }
    if (written != 1 || required != 1 ||
        fit.covariance.kind != SIDEREON_ORBIT_FIT_COVARIANCE_KIND_UNBOUNDED ||
        fit.geometry_quality.tier != SIDEREON_OBSERVABILITY_TIER_ZERO_REDUNDANCY) {
        sidereon_orbit_fit_report_free(report);
        return fail("sparse orbit unbounded covariance");
    }
    SidereonOrbitSatelliteResidualEntry ledger;
    written = 0;
    required = 0;
    if (require_ok(sidereon_orbit_fit_report_satellite_ledger(report, &ledger, 1, &written,
                                                              &required),
                   "orbit satellite ledger") != 0) {
        sidereon_orbit_fit_report_free(report);
        return 1;
    }
    sidereon_orbit_fit_report_free(report);
    if (written != 1 || required != 1 || ledger.stats.n != 2 || !ledger.stats.low_sample_count) {
        return fail("sparse orbit low-sample ledger");
    }
    return 0;
}

static int test_baarda_constants(void) {
    SidereonWTestNoncentrality constants;
    if (require_ok(sidereon_wtest_noncentrality(0.001, 0.20, &constants),
                   "w-test noncentrality") != 0) {
        return 1;
    }
    if (!close_rel(constants.delta0, 4.132147965064809, 1.0e-14)) {
        fprintf(stderr, "FAIL: delta0 %.17g expected %.17g\n", constants.delta0,
                4.132147965064809);
        return 1;
    }
    if (constants.lambda0 != constants.delta0 * constants.delta0) {
        return fail("lambda0 identity");
    }
    return 0;
}

static int test_reliability_design(void) {
    const double h1[2] = {1.0, 0.0};
    const double h2[2] = {0.0, 1.0};
    const double h3[2] = {0.0, 1.0};
    const SidereonRangeReliabilityRow rows[3] = {
        {.id = "rx_clock", .design_row = h1, .design_dim = 2, .sigma_m = 1.0},
        {.id = "range_a", .design_row = h2, .design_dim = 2, .sigma_m = 1.0},
        {.id = "range_b", .design_row = h3, .design_dim = 2, .sigma_m = 1.0},
    };
    SidereonReliabilityOptions options;
    if (require_ok(sidereon_reliability_options_init(&options), "reliability options") != 0) {
        return 1;
    }
    SidereonReliabilityReport *report = NULL;
    if (require_ok(sidereon_reliability_design(rows, 3, &options, &report),
                   "reliability design") != 0) {
        return 1;
    }
    SidereonReliabilitySummary summary;
    if (require_ok(sidereon_reliability_report_summary(report, &summary),
                   "reliability summary") != 0) {
        sidereon_reliability_report_free(report);
        return 1;
    }
    SidereonObservationReliability obs[3];
    size_t written = 0;
    size_t required = 0;
    if (require_ok(sidereon_reliability_report_observations(report, obs, 3, &written,
                                                            &required),
                   "reliability observations") != 0) {
        sidereon_reliability_report_free(report);
        return 1;
    }
    sidereon_reliability_report_free(report);
    if (written != 3 || required != 3 || summary.dof != 1 || summary.n_uncheckable != 1) {
        return fail("reliability counts");
    }
    double sum = obs[0].redundancy + obs[1].redundancy + obs[2].redundancy;
    if (!close_abs(sum, summary.sum_redundancy, 2.0e-14) ||
        !close_abs(summary.sum_redundancy, (double)summary.dof, 2.0e-14)) {
        return fail("reliability redundancy sum");
    }
    if (!obs[0].uncheckable || obs[0].has_mdb_m || obs[0].has_external_enu_m ||
        obs[0].has_bias_to_noise) {
        return fail("reliability uncheckable row");
    }
    if (obs[1].uncheckable || !obs[1].has_mdb_m || !obs[1].has_bias_to_noise) {
        return fail("reliability checkable row");
    }
    return 0;
}

static int test_sbas_protection_levels(void) {
    SidereonSbasKMultipliers precision;
    SidereonSbasKMultipliers enroute;
    if (require_ok(sidereon_sbas_k_multipliers_precision_approach(&precision),
                   "SBAS precision K") != 0 ||
        require_ok(sidereon_sbas_k_multipliers_en_route_npa(&enroute), "SBAS enroute K") != 0) {
        return 1;
    }
    if (f64_bits(precision.k_h) != f64_bits(6.0) ||
        f64_bits(precision.k_v) != f64_bits(5.33) ||
        f64_bits(enroute.k_h) != f64_bits(6.18) ||
        f64_bits(enroute.k_v) != f64_bits(5.33)) {
        return fail("SBAS K constants");
    }

    const double deg_to_rad = 0.017453292519943295;
    const SidereonGeodetic receiver = {.lat_rad = 0.0, .lon_rad = 0.0, .height_m = 0.0};
    const double az_el[5][2] = {
        {15.0, 15.0},
        {80.0, 70.0},
        {155.0, 25.0},
        {230.0, 55.0},
        {310.0, 35.0},
    };
    const char *ids[5] = {"G01", "G02", "G03", "G04", "G05"};
    SidereonSbasProtectionRow rows[5];
    for (size_t i = 0; i < 5; i++) {
        if (require_ok(sidereon_line_of_sight_from_az_el_deg(az_el[i][0], az_el[i][1],
                                                             receiver, &rows[i].line_of_sight),
                       "SBAS line of sight") != 0) {
            return 1;
        }
        rows[i].sat_id = ids[i];
        rows[i].system = SIDEREON_GNSS_SYSTEM_GPS;
        rows[i].elevation_rad = az_el[i][1] * deg_to_rad;
    }
    const uint32_t clocks[1] = {SIDEREON_GNSS_SYSTEM_GPS};
    const SidereonSbasProtectionGeometry geometry = {
        .rows = rows,
        .row_count = 5,
        .receiver = receiver,
        .clock_systems = clocks,
        .clock_system_count = 1,
    };
    const double sigmas[5] = {2.0, 1.0, 1.5, 1.2, 1.8};
    SidereonSbasSisError sis[5];
    for (size_t i = 0; i < 5; i++) {
        sis[i].sat_id = ids[i];
        sis[i].sigma_flt_m = sigmas[i];
        sis[i].sigma_uire_m = 0.0;
        sis[i].sigma_air_m = 0.0;
        sis[i].sigma_tropo_m = 0.0;
    }
    const SidereonSbasErrorModel model = {.rows = sis, .row_count = 5};
    SidereonSbasProtection pl;
    SidereonSbasPlError err = SIDEREON_SBAS_PL_ERROR_NONE;
    if (require_ok(sidereon_sbas_protection_levels(&geometry, &model, precision, &pl, &err),
                   "SBAS protection levels") != 0) {
        return 1;
    }
    if (err != SIDEREON_SBAS_PL_ERROR_NONE) {
        return fail("SBAS PL error detail");
    }
    if (!close_rel(pl.hpl_m, 9.064491010405014, 1.0e-12) ||
        !close_rel(pl.vpl_m, 13.664070819648263, 1.0e-12)) {
        fprintf(stderr, "FAIL: SBAS PL hpl=%.17g vpl=%.17g\n", pl.hpl_m, pl.vpl_m);
        return 1;
    }
    return 0;
}

int main(void) {
    if (test_error_metrics() != 0 || test_sidereal() != 0 || test_midas() != 0 ||
        test_clock_power_law() != 0 || test_composite_propagation() != 0 ||
        test_sparse_orbit_fit() != 0 || test_baarda_constants() != 0 ||
        test_reliability_design() != 0 || test_sbas_protection_levels() != 0) {
        return 1;
    }
    printf("cap015: error metrics, sidereal, MIDAS, clock noise, composite propagation, orbit fit, reliability, SBAS PL\n");
    return 0;
}
