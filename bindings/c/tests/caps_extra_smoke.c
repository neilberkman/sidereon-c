/*
 * Smoke coverage for the newer merged-core capabilities exposed through the C
 * ABI:
 *   - generic data-driven trust-region least squares: solve + leave-one-out
 *     (RAIM/FDE), with the result/summary/array accessors
 *   - Jacobian-derived covariance, Hessian trace, and the 2x2 error ellipse
 *   - DOP with an explicit ENU convention
 *   - residual-distribution statistics (moments + normality tests)
 *   - batch forward-observable prediction from SP3 and broadcast sources
 *   - rejection of out-of-range C-supplied enum discriminants
 *   - leap-second accessors (GPS-UTC, TAI-UTC)
 *   - embedded EGM96 geoid undulation and height conversions
 *   - ground-observer Sun/Moon geometry, illumination, rise/set, transits
 *
 * Structural and closed-form numeric checks (covariance entries, Hessian trace,
 * integer-exact moments, geoid round-trip, leap-second steps) run everywhere.
 * The bit-exact-vs-SciPy parity of the trust-region solver and the normality
 * statistics is pinned to linux-x86_64: there, if a host LAPACK is wired through
 * the environment, the host-LAPACK backend is exercised; the committed
 * scipy-fixture comparison is the engine's own concern at release.
 */
#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sidereon.h"

#include "broadcast_fixture.h"
#include "spp_fixture.h"
#include "w5_caps_extra_pins.h"

#if defined(__linux__) && defined(__x86_64__)
#define BITEXACT_PINNED 1
#else
#define BITEXACT_PINNED 0
#endif

static int fail(const char *what, int code) {
    char message[256];
    size_t written = sidereon_last_error_message(message, sizeof message);
    if (written > 0) {
        fprintf(stderr, "FAIL: %s: %s\n", what, message);
    } else {
        fprintf(stderr, "FAIL: %s\n", what);
    }
    return code;
}

static double bits_to_f64(uint64_t bits) {
    double value;
    memcpy(&value, &bits, sizeof value);
    return value;
}

/* Exact agreement with a pinned engine value (tests/valgen, bin
 * w5_caps_extra). */
static int same_bits(double got, uint64_t want) {
    uint64_t bits;
    memcpy(&bits, &got, sizeof bits);
    return bits == want;
}

static int approx(double got, double want, double tol) {
    return fabs(got - want) <= tol;
}

static uint8_t *read_file(const char *path, size_t *out_len) {
    FILE *f = fopen(path, "rb");
    if (f == NULL) {
        return NULL;
    }
    if (fseek(f, 0, SEEK_END) != 0) {
        fclose(f);
        return NULL;
    }
    long size = ftell(f);
    if (size < 0) {
        fclose(f);
        return NULL;
    }
    rewind(f);
    uint8_t *buf = malloc((size_t)size);
    if (buf == NULL) {
        fclose(f);
        return NULL;
    }
    if (fread(buf, 1, (size_t)size, f) != (size_t)size) {
        free(buf);
        fclose(f);
        return NULL;
    }
    fclose(f);
    *out_len = (size_t)size;
    return buf;
}

/* (1) HEADLINE: generic data-driven trust-region least squares. */
static int exercise_trls(void) {
    /* Consistent overdetermined linear system y = 1 + 2x at (0,1),(1,3),(2,5):
     * the least-squares minimizer is exactly [1, 2]. */
    double a[6] = {1.0, 0.0, 1.0, 1.0, 1.0, 2.0};
    double b[3] = {1.0, 3.0, 5.0};
    double x0[2] = {0.0, 0.0};

    SidereonDataProblem problem;
    if (sidereon_data_problem_init(SIDEREON_TRLS_KIND_LINEAR, &problem) != SIDEREON_STATUS_OK) {
        return fail("trls: problem init", 1);
    }
    problem.a = a;
    problem.a_len = 6;
    problem.b = b;
    problem.b_len = 3;
    problem.m = 3;
    problem.n = 2;
    problem.x0 = x0;
    problem.x0_len = 2;

    SidereonTrlsSolution *sol = NULL;
    if (sidereon_solve_data_problem(&problem, &sol) != SIDEREON_STATUS_OK || sol == NULL) {
        return fail("trls: solve linear", 1);
    }

    SidereonTrlsSummary summary;
    if (sidereon_trls_solution_summary(sol, &summary) != SIDEREON_STATUS_OK) {
        sidereon_trls_solution_free(sol);
        return fail("trls: summary", 1);
    }
    if (summary.success != W5_CAPS_EXTRA_TRLS_SUCCESS || summary.n != 2 || summary.m != 3) {
        sidereon_trls_solution_free(sol);
        return fail("trls: summary shape/success", 1);
    }

    double x[2] = {0.0, 0.0};
    size_t written = 0, required = 0;
    if (sidereon_trls_solution_x(sol, NULL, 0, &written, &required) != SIDEREON_STATUS_OK ||
        required != 2) {
        sidereon_trls_solution_free(sol);
        return fail("trls: x query count", 1);
    }
    if (sidereon_trls_solution_x(sol, x, 2, &written, &required) != SIDEREON_STATUS_OK ||
        written != 2) {
        sidereon_trls_solution_free(sol);
        return fail("trls: x copy", 1);
    }
    /* The engine's minimizer; the closed form is exactly [1, 2]. */
    if (!same_bits(x[0], W5_CAPS_EXTRA_TRLS_X_BITS[0]) ||
        !same_bits(x[1], W5_CAPS_EXTRA_TRLS_X_BITS[1]) || !approx(x[0], 1.0, 1e-6) ||
        !approx(x[1], 2.0, 1e-6)) {
        sidereon_trls_solution_free(sol);
        return fail("trls: linear minimizer", 1);
    }

    /* Residuals, gradient, and Jacobian are reachable with the same contract. */
    double residuals[3] = {0};
    double grad[2] = {0};
    double jac[6] = {0};
    if (sidereon_trls_solution_residuals(sol, residuals, 3, &written, &required) !=
            SIDEREON_STATUS_OK ||
        sidereon_trls_solution_gradient(sol, grad, 2, &written, &required) != SIDEREON_STATUS_OK ||
        sidereon_trls_solution_jacobian(sol, jac, 6, &written, &required) != SIDEREON_STATUS_OK) {
        sidereon_trls_solution_free(sol);
        return fail("trls: residuals/gradient/jacobian", 1);
    }
    for (size_t i = 0; i < 3; ++i) {
        if (!same_bits(residuals[i], W5_CAPS_EXTRA_TRLS_RESIDUALS_BITS[i])) {
            sidereon_trls_solution_free(sol);
            return fail("trls: residuals", 1);
        }
    }
    /* The consistent system fits exactly, so the cost (and every residual) is ~0. */
    if (!same_bits(summary.cost, W5_CAPS_EXTRA_TRLS_COST_BITS) || !approx(summary.cost, 0.0, 1e-12)) {
        sidereon_trls_solution_free(sol);
        return fail("trls: consistent-fit cost", 1);
    }
    sidereon_trls_solution_free(sol);

    /* Polynomial kind: degree-1 fit recovers the same coefficients. */
    double t[3] = {0.0, 1.0, 2.0};
    double y[3] = {1.0, 3.0, 5.0};
    double poly_x0[2] = {0.0, 0.0};
    SidereonDataProblem poly;
    if (sidereon_data_problem_init(SIDEREON_TRLS_KIND_POLYNOMIAL, &poly) != SIDEREON_STATUS_OK) {
        return fail("trls: poly init", 1);
    }
    poly.degree = 1;
    poly.t = t;
    poly.t_len = 3;
    poly.y = y;
    poly.y_len = 3;
    poly.x0 = poly_x0;
    poly.x0_len = 2;
    SidereonTrlsSolution *poly_sol = NULL;
    if (sidereon_solve_data_problem(&poly, &poly_sol) != SIDEREON_STATUS_OK || poly_sol == NULL) {
        return fail("trls: solve polynomial", 1);
    }
    double poly_coeffs[2] = {0};
    if (sidereon_trls_solution_x(poly_sol, poly_coeffs, 2, &written, &required) !=
        SIDEREON_STATUS_OK) {
        sidereon_trls_solution_free(poly_sol);
        return fail("trls: poly x", 1);
    }
    if (!same_bits(poly_coeffs[0], W5_CAPS_EXTRA_TRLS_POLY_X_BITS[0]) ||
        !same_bits(poly_coeffs[1], W5_CAPS_EXTRA_TRLS_POLY_X_BITS[1]) ||
        !approx(poly_coeffs[0], 1.0, 1e-6) || !approx(poly_coeffs[1], 2.0, 1e-6)) {
        sidereon_trls_solution_free(poly_sol);
        return fail("trls: poly coefficients", 1);
    }
    sidereon_trls_solution_free(poly_sol);

    /* Leave-one-out (RAIM/FDE) over the linear problem. */
    SidereonTrlsDropOne *report = NULL;
    if (sidereon_solve_data_problem_drop_one(&problem, &report) != SIDEREON_STATUS_OK ||
        report == NULL) {
        return fail("trls: drop-one solve", 1);
    }
    size_t drop_count = 0;
    if (sidereon_trls_drop_one_count(report, &drop_count) != SIDEREON_STATUS_OK ||
        drop_count != W5_CAPS_EXTRA_TRLS_DROP_COUNT) {
        sidereon_trls_drop_one_free(report);
        return fail("trls: drop-one count", 1);
    }
    SidereonTrlsSummary base_summary;
    if (sidereon_trls_drop_one_base_summary(report, &base_summary) != SIDEREON_STATUS_OK ||
        base_summary.success != W5_CAPS_EXTRA_TRLS_DROP_BASE_SUCCESS) {
        sidereon_trls_drop_one_free(report);
        return fail("trls: drop-one base summary", 1);
    }
    double cost_delta[3] = {0};
    if (sidereon_trls_drop_one_cost_delta(report, cost_delta, 3, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != 3) {
        sidereon_trls_drop_one_free(report);
        return fail("trls: drop-one cost delta", 1);
    }
    for (size_t i = 0; i < 3; ++i) {
        SidereonTrlsSummary drop_summary;
        double drop_x[2] = {0};
        if (sidereon_trls_drop_one_drop_summary(report, i, &drop_summary) != SIDEREON_STATUS_OK ||
            sidereon_trls_drop_one_drop_x(report, i, drop_x, 2, &written, &required) !=
                SIDEREON_STATUS_OK) {
            sidereon_trls_drop_one_free(report);
            return fail("trls: drop-one per-row accessors", 1);
        }
        if (!same_bits(cost_delta[i], W5_CAPS_EXTRA_TRLS_DROP_COST_DELTA_BITS[i]) ||
            !same_bits(drop_x[0], W5_CAPS_EXTRA_TRLS_DROP_X_BITS[2 * i]) ||
            !same_bits(drop_x[1], W5_CAPS_EXTRA_TRLS_DROP_X_BITS[2 * i + 1])) {
            sidereon_trls_drop_one_free(report);
            return fail("trls: drop-one values", 1);
        }
    }
    sidereon_trls_drop_one_free(report);

#if BITEXACT_PINNED
    /* linux-x86_64: if a host LAPACK is wired through the environment, exercise
     * the bit-exact backend; otherwise it is unreachable here and we skip it.
     * The committed scipy-fixture comparison lives with the engine. */
    if (getenv("TRUST_REGION_LEAST_SQUARES_LAPACK_PATH") != NULL) {
        SidereonDataProblem host = problem;
        host.backend = SIDEREON_TRLS_BACKEND_HOST_LAPACK;
        SidereonTrlsSolution *host_sol = NULL;
        if (sidereon_solve_data_problem(&host, &host_sol) != SIDEREON_STATUS_OK ||
            host_sol == NULL) {
            return fail("trls: host-lapack solve", 1);
        }
        double host_x[2] = {0};
        if (sidereon_trls_solution_x(host_sol, host_x, 2, &written, &required) !=
            SIDEREON_STATUS_OK) {
            sidereon_trls_solution_free(host_sol);
            return fail("trls: host-lapack minimizer", 1);
        }
        /* The host backend's result is the LAPACK library the environment
         * names, so an exact pin exists only when tests/valgen ran with that
         * same library (W5_CAPS_EXTRA_TRLS_HOST_LAPACK_SOLVED). A header
         * generated without one carries no host value; the minimizer is then
         * checked against the closed-form solution [1, 2] of the consistent
         * system y = 1 + 2x. */
        bool host_ok = W5_CAPS_EXTRA_TRLS_HOST_LAPACK_SOLVED
                           ? same_bits(host_x[0], W5_CAPS_EXTRA_TRLS_HOST_LAPACK_X_BITS[0]) &&
                                 same_bits(host_x[1], W5_CAPS_EXTRA_TRLS_HOST_LAPACK_X_BITS[1])
                           : fabs(host_x[0] - 1.0) <= 1e-9 && fabs(host_x[1] - 2.0) <= 1e-9;
        if (!host_ok) {
            sidereon_trls_solution_free(host_sol);
            return fail("trls: host-lapack minimizer", 1);
        }
        sidereon_trls_solution_free(host_sol);
        printf("trls: host-lapack backend exercised\n");
    }
#endif

    printf("trls: linear/polynomial solve + leave-one-out OK\n");
    return 0;
}

/* (2) Covariance / Hessian trace / error ellipse. */
static int exercise_covariance(void) {
    /* Design matrix [[1,0],[1,1],[1,2]] -> J^T J = [[3,3],[3,5]],
     * (J^T J)^-1 = [[5/6, -1/2], [-1/2, 1/2]], trace(J^T J) = 8. */
    double jac[6] = {1.0, 0.0, 1.0, 1.0, 1.0, 2.0};

    double cov[4] = {0};
    size_t written = 0, required = 0;
    if (sidereon_normal_covariance(jac, 3, 2, 1.0, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        required != 4) {
        return fail("covariance: normal query count", 1);
    }
    if (sidereon_normal_covariance(jac, 3, 2, 1.0, cov, 4, &written, &required) !=
        SIDEREON_STATUS_OK) {
        return fail("covariance: normal copy", 1);
    }
    for (size_t i = 0; i < 4; ++i) {
        if (!same_bits(cov[i], W5_CAPS_EXTRA_NORMAL_COVARIANCE_BITS[i])) {
            return fail("covariance: normal entries differ from the engine's", 1);
        }
    }
    if (!approx(cov[0], 5.0 / 6.0, 1e-9) || !approx(cov[1], -0.5, 1e-9) ||
        !approx(cov[2], -0.5, 1e-9) || !approx(cov[3], 0.5, 1e-9)) {
        return fail("covariance: normal entries", 1);
    }

    double trace = 0.0;
    if (sidereon_hessian_trace(jac, 3, 2, &trace) != SIDEREON_STATUS_OK ||
        !same_bits(trace, W5_CAPS_EXTRA_HESSIAN_TRACE_BITS) || !approx(trace, 8.0, 1e-12)) {
        return fail("covariance: hessian trace", 1);
    }

    /* covariance_from_jacobian scales (J^T J)^-1 by s_sq = 2*cost/(m-n).
     * With cost = 3, m-n = 1, s_sq = 6 -> cov = [[5,-3],[-3,3]]. */
    double jac_cov[4] = {0};
    if (sidereon_covariance_from_jacobian(jac, 3, 2, 3.0, jac_cov, 4, &written, &required) !=
        SIDEREON_STATUS_OK) {
        return fail("covariance: from jacobian", 1);
    }
    for (size_t i = 0; i < 4; ++i) {
        if (!same_bits(jac_cov[i], W5_CAPS_EXTRA_JACOBIAN_COVARIANCE_BITS[i])) {
            return fail("covariance: from jacobian entries differ from the engine's", 1);
        }
    }
    if (!approx(jac_cov[0], 5.0, 1e-9) || !approx(jac_cov[3], 3.0, 1e-9)) {
        return fail("covariance: from jacobian entries", 1);
    }

    /* Error ellipse: diag(4,1) at confidence with chi-square scale 1 -> axes 2, 1. */
    double block[4] = {4.0, 0.0, 0.0, 1.0};
    double confidence = 1.0 - exp(-0.5); /* chi_square_scale = -2 ln(1-conf) = 1 */
    SidereonErrorEllipse2 ellipse;
    if (sidereon_error_ellipse_2x2(block, confidence, &ellipse) != SIDEREON_STATUS_OK) {
        return fail("covariance: error ellipse", 1);
    }
    if (!same_bits(ellipse.chi_square_scale, W5_CAPS_EXTRA_ELLIPSE_CHI_SQUARE_SCALE_BITS) ||
        !same_bits(ellipse.semi_major, W5_CAPS_EXTRA_ELLIPSE_SEMI_MAJOR_BITS) ||
        !same_bits(ellipse.semi_minor, W5_CAPS_EXTRA_ELLIPSE_SEMI_MINOR_BITS) ||
        !approx(ellipse.chi_square_scale, 1.0, 1e-9) || !approx(ellipse.semi_major, 2.0, 1e-9) ||
        !approx(ellipse.semi_minor, 1.0, 1e-9)) {
        return fail("covariance: ellipse axes", 1);
    }

    printf("covariance: normal + hessian trace + jacobian + error ellipse OK\n");
    return 0;
}

/* (3) DOP with explicit ENU convention. */
static int exercise_dop_convention(void) {
    SidereonGeodetic receiver = {0.6, 0.1, 100.0};
    const double az[4] = {0.0, 90.0, 180.0, 270.0};
    const double el[4] = {15.0, 30.0, 45.0, 70.0};
    SidereonLineOfSight los[4];
    for (int i = 0; i < 4; ++i) {
        if (sidereon_line_of_sight_from_az_el_deg(az[i], el[i], receiver, &los[i]) !=
            SIDEREON_STATUS_OK) {
            return fail("dop convention: los build", 1);
        }
    }
    double weights[4] = {1.0, 1.0, 1.0, 1.0};

    SidereonDop geodetic;
    SidereonDop geocentric;
    if (sidereon_dop_with_convention(los, weights, 4, receiver,
                                     SIDEREON_ENU_CONVENTION_GEODETIC_NORMAL,
                                     &geodetic) != SIDEREON_STATUS_OK) {
        return fail("dop convention: geodetic-normal", 1);
    }
    if (sidereon_dop_with_convention(los, weights, 4, receiver,
                                     SIDEREON_ENU_CONVENTION_GEOCENTRIC_RADIAL,
                                     &geocentric) != SIDEREON_STATUS_OK) {
        return fail("dop convention: geocentric-radial", 1);
    }
    if (!same_bits(geodetic.gdop, W5_CAPS_EXTRA_DOP_GEODETIC_GDOP_BITS) ||
        !same_bits(geodetic.pdop, W5_CAPS_EXTRA_DOP_GEODETIC_PDOP_BITS) ||
        !same_bits(geodetic.hdop, W5_CAPS_EXTRA_DOP_GEODETIC_HDOP_BITS) ||
        !same_bits(geodetic.vdop, W5_CAPS_EXTRA_DOP_GEODETIC_VDOP_BITS) ||
        !same_bits(geodetic.tdop, W5_CAPS_EXTRA_DOP_GEODETIC_TDOP_BITS) ||
        !same_bits(geocentric.gdop, W5_CAPS_EXTRA_DOP_GEOCENTRIC_GDOP_BITS) ||
        !same_bits(geocentric.pdop, W5_CAPS_EXTRA_DOP_GEOCENTRIC_PDOP_BITS) ||
        !same_bits(geocentric.hdop, W5_CAPS_EXTRA_DOP_GEOCENTRIC_HDOP_BITS) ||
        !same_bits(geocentric.vdop, W5_CAPS_EXTRA_DOP_GEOCENTRIC_VDOP_BITS) ||
        !same_bits(geocentric.tdop, W5_CAPS_EXTRA_DOP_GEOCENTRIC_TDOP_BITS)) {
        return fail("dop convention: engine values", 1);
    }
    /* GDOP/PDOP/TDOP are identical between conventions; HDOP/VDOP may differ. */
    if (!isfinite(geodetic.gdop) || !approx(geodetic.gdop, geocentric.gdop, 1e-9) ||
        !approx(geodetic.tdop, geocentric.tdop, 1e-9)) {
        return fail("dop convention: GDOP/TDOP invariance", 1);
    }
    printf("dop convention: gdop %.3f (geodetic) == %.3f (geocentric) OK\n", geodetic.gdop,
           geocentric.gdop);
    return 0;
}

/* (4) Residual-distribution statistics. */
static int exercise_residual_stats(void) {
    /* mean 5, population variance 4 by construction. */
    double x[8] = {2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0};

    SidereonResidualMoments moments;
    if (sidereon_residual_moments(x, 8, true, false, &moments) != SIDEREON_STATUS_OK) {
        return fail("stats: moments", 1);
    }
    if (!same_bits(moments.mean, W5_CAPS_EXTRA_MOMENTS_MEAN_BITS) ||
        !same_bits(moments.variance, W5_CAPS_EXTRA_MOMENTS_VARIANCE_BITS) ||
        !approx(moments.mean, 5.0, 1e-12) || !approx(moments.variance, 4.0, 1e-12)) {
        return fail("stats: moments mean/variance", 1);
    }

    double skew = 0.0, kurt = 0.0;
    if (sidereon_residual_skewness(x, 8, true, &skew) != SIDEREON_STATUS_OK ||
        sidereon_residual_kurtosis(x, 8, true, false, &kurt) != SIDEREON_STATUS_OK) {
        return fail("stats: skewness/kurtosis", 1);
    }
    if (!same_bits(skew, W5_CAPS_EXTRA_SKEWNESS_BITS) ||
        !same_bits(kurt, W5_CAPS_EXTRA_KURTOSIS_BITS)) {
        return fail("stats: skewness/kurtosis finite", 1);
    }

    SidereonJarqueBera jb;
    SidereonShapiroWilk sw;
    if (sidereon_residual_jarque_bera(x, 8, &jb) != SIDEREON_STATUS_OK ||
        sidereon_residual_shapiro_wilk(x, 8, &sw) != SIDEREON_STATUS_OK) {
        return fail("stats: jarque-bera/shapiro-wilk", 1);
    }
    if (!same_bits(jb.statistic, W5_CAPS_EXTRA_JARQUE_BERA_STATISTIC_BITS) ||
        !same_bits(jb.p_value, W5_CAPS_EXTRA_JARQUE_BERA_P_VALUE_BITS) ||
        !same_bits(sw.w, W5_CAPS_EXTRA_SHAPIRO_WILK_W_BITS) ||
        !same_bits(sw.p_value, W5_CAPS_EXTRA_SHAPIRO_WILK_P_VALUE_BITS)) {
        return fail("stats: test ranges", 1);
    }

    /* Too few samples for Shapiro-Wilk is rejected as an invalid argument. */
    double tiny[2] = {1.0, 2.0};
    SidereonShapiroWilk sw_tiny;
    if (W5_CAPS_EXTRA_SHAPIRO_WILK_TWO_SAMPLES_OK ||
        sidereon_residual_shapiro_wilk(tiny, 2, &sw_tiny) != SIDEREON_STATUS_INVALID_ARGUMENT) {
        return fail("stats: shapiro-wilk underflow not rejected", 1);
    }

    printf("stats: moments + jarque-bera + shapiro-wilk (W=%.4f) OK\n", sw.w);
    return 0;
}

/* (5) Batch forward-observable prediction from an SP3 product. */
static int exercise_predict_batch(const SidereonSp3 *sp3) {
    double receiver[3] = {
        bits_to_f64(SPP_EXPECTED_X_BITS[0]),
        bits_to_f64(SPP_EXPECTED_X_BITS[1]),
        bits_to_f64(SPP_EXPECTED_X_BITS[2]),
    };
    double t_rx = bits_to_f64(SPP_T_RX_J2000_S_BITS);

    enum { N = 6 };
    SidereonPredictRequest requests[N];
    for (int i = 0; i < N; ++i) {
        requests[i].sat_id = SPP_SAT_IDS[i];
        requests[i].receiver_ecef_m[0] = receiver[0];
        requests[i].receiver_ecef_m[1] = receiver[1];
        requests[i].receiver_ecef_m[2] = receiver[2];
        requests[i].t_rx_j2000_s = t_rx;
    }

    SidereonPredictedObservables out[N];
    bool ok[N];
    if (sidereon_sp3_observables_batch(sp3, requests, N, NULL, out, ok) != SIDEREON_STATUS_OK) {
        return fail("predict batch: sp3", 1);
    }
    int predicted = 0;
    for (int i = 0; i < N; ++i) {
        if (ok[i] != W5_CAPS_EXTRA_SP3_BATCH_OK[i] ||
            !same_bits(out[i].geometric_range_m, W5_CAPS_EXTRA_SP3_BATCH_RANGE_M_BITS[i]) ||
            !same_bits(out[i].elevation_deg, W5_CAPS_EXTRA_SP3_BATCH_ELEVATION_DEG_BITS[i])) {
            return fail("predict batch: row differs from the engine's", 1);
        }
        predicted += ok[i] ? 1 : 0;
    }
    printf("predict batch: %d of %d satellites predicted OK\n", predicted, N);
    return 0;
}

/* (5b) Batch forward-observable prediction from a broadcast source. Mirror of
 * exercise_predict_batch for sidereon_broadcast_observables_batch. */
static int exercise_broadcast_batch(const SidereonBroadcastEphemeris *broadcast) {
    double receiver[3] = {
        bits_to_f64(BC_INITIAL_GUESS_BITS[0]),
        bits_to_f64(BC_INITIAL_GUESS_BITS[1]),
        bits_to_f64(BC_INITIAL_GUESS_BITS[2]),
    };
    double t_rx = bits_to_f64(BC_T_RX_J2000_S_BITS);

    SidereonPredictRequest requests[BC_OBS_COUNT];
    for (size_t i = 0; i < BC_OBS_COUNT; ++i) {
        requests[i].sat_id = BC_SAT_IDS[i];
        requests[i].receiver_ecef_m[0] = receiver[0];
        requests[i].receiver_ecef_m[1] = receiver[1];
        requests[i].receiver_ecef_m[2] = receiver[2];
        requests[i].t_rx_j2000_s = t_rx;
    }

    SidereonPredictedObservables out[BC_OBS_COUNT];
    bool ok[BC_OBS_COUNT];
    if (sidereon_broadcast_observables_batch(broadcast, requests, BC_OBS_COUNT, NULL, out, ok) !=
        SIDEREON_STATUS_OK) {
        return fail("broadcast batch: call", 1);
    }
    int predicted = 0;
    if (W5_CAPS_EXTRA_BROADCAST_BATCH_OK_COUNT != BC_OBS_COUNT) {
        return fail("broadcast batch: pinned row count", 1);
    }
    for (size_t i = 0; i < BC_OBS_COUNT; ++i) {
        if (ok[i] != W5_CAPS_EXTRA_BROADCAST_BATCH_OK[i] ||
            !same_bits(out[i].geometric_range_m, W5_CAPS_EXTRA_BROADCAST_BATCH_RANGE_M_BITS[i]) ||
            !same_bits(out[i].elevation_deg, W5_CAPS_EXTRA_BROADCAST_BATCH_ELEVATION_DEG_BITS[i])) {
            return fail("broadcast batch: row differs from the engine's", 1);
        }
        predicted += ok[i] ? 1 : 0;
    }
    printf("broadcast batch: %d of %d satellites predicted OK\n", predicted, (int)BC_OBS_COUNT);
    return 0;
}

/* (5c) Out-of-range C-supplied enum discriminants must be rejected with
 * SIDEREON_STATUS_INVALID_ARGUMENT, never transmuted into an invalid Rust enum
 * (undefined behavior). The enum-typed problem fields and the convention
 * argument cross the boundary as uint32_t, so a bogus integer is well-defined to
 * pass and must be caught by the binding. */
static int reject_bad_problem(const SidereonDataProblem *bad, const char *what) {
    SidereonTrlsSolution *sol = NULL;
    if (sidereon_solve_data_problem(bad, &sol) != SIDEREON_STATUS_INVALID_ARGUMENT || sol != NULL) {
        sidereon_trls_solution_free(sol);
        return fail(what, 1);
    }
    return 0;
}

static int exercise_invalid_enum(void) {
    const uint32_t bogus = 4242;

    /* The initializer rejects an out-of-range residual kind. */
    SidereonDataProblem probe;
    if (sidereon_data_problem_init(bogus, &probe) != SIDEREON_STATUS_INVALID_ARGUMENT) {
        return fail("invalid enum: data_problem_init bogus kind", 1);
    }

    /* Build an otherwise-valid linear problem, then corrupt one enum field at a
     * time and confirm the solve refuses each. */
    double a[6] = {1.0, 0.0, 1.0, 1.0, 1.0, 2.0};
    double b[3] = {1.0, 3.0, 5.0};
    double x0[2] = {0.0, 0.0};
    SidereonDataProblem base;
    if (sidereon_data_problem_init(SIDEREON_TRLS_KIND_LINEAR, &base) != SIDEREON_STATUS_OK) {
        return fail("invalid enum: base init", 1);
    }
    base.a = a;
    base.a_len = 6;
    base.b = b;
    base.b_len = 3;
    base.m = 3;
    base.n = 2;
    base.x0 = x0;
    base.x0_len = 2;

    SidereonDataProblem bad = base;
    bad.kind = bogus;
    if (reject_bad_problem(&bad, "invalid enum: solve accepted bogus kind")) {
        return 1;
    }
    bad = base;
    bad.loss = bogus;
    if (reject_bad_problem(&bad, "invalid enum: solve accepted bogus loss")) {
        return 1;
    }
    bad = base;
    bad.x_scale_mode = bogus;
    if (reject_bad_problem(&bad, "invalid enum: solve accepted bogus x_scale_mode")) {
        return 1;
    }
    bad = base;
    bad.backend = bogus;
    if (reject_bad_problem(&bad, "invalid enum: solve accepted bogus backend")) {
        return 1;
    }

    /* A valid problem still solves, confirming the rejections above are about the
     * corrupted discriminants and not the shared inputs. */
    SidereonTrlsSolution *sol = NULL;
    if (sidereon_solve_data_problem(&base, &sol) != SIDEREON_STATUS_OK || sol == NULL) {
        return fail("invalid enum: valid control solve", 1);
    }
    sidereon_trls_solution_free(sol);

    /* dop_with_convention rejects an out-of-range convention. */
    SidereonGeodetic receiver = {0.6, 0.1, 100.0};
    SidereonLineOfSight los[4];
    for (int i = 0; i < 4; ++i) {
        if (sidereon_line_of_sight_from_az_el_deg((double)(i * 90), 30.0, receiver, &los[i]) !=
            SIDEREON_STATUS_OK) {
            return fail("invalid enum: los build", 1);
        }
    }
    double weights[4] = {1.0, 1.0, 1.0, 1.0};
    SidereonDop dop;
    if (sidereon_dop_with_convention(los, weights, 4, receiver, bogus, &dop) !=
        SIDEREON_STATUS_INVALID_ARGUMENT) {
        return fail("invalid enum: dop_with_convention bogus convention", 1);
    }

    printf("invalid enum: bogus kind/loss/x_scale/backend/convention rejected OK\n");
    return 0;
}

/* (6) Leap-second accessors. */
static int exercise_leap_seconds(void) {
    /* 2020-06-25 (DOY 177) UTC Julian date: TAI-UTC = 37, GPS-UTC = 18. */
    double jd_utc = 2459025.5;
    double gps = 0.0, tai = 0.0;
    if (sidereon_gps_utc_offset_s(jd_utc, &gps) != SIDEREON_STATUS_OK ||
        sidereon_tai_utc_offset_s(jd_utc, &tai) != SIDEREON_STATUS_OK) {
        return fail("leap seconds: accessors", 1);
    }
    /* TAI - GPS = 19 s is the fixed GPS time origin offset. */
    if (!same_bits(gps, W5_CAPS_EXTRA_GPS_UTC_S_BITS) ||
        !same_bits(tai, W5_CAPS_EXTRA_TAI_UTC_S_BITS) || tai - gps != 19.0) {
        return fail("leap seconds: expected steps", 1);
    }
    printf("leap seconds: GPS-UTC %.0f, TAI-UTC %.0f OK\n", gps, tai);
    return 0;
}

/* (7) Embedded EGM96 geoid. */
static int exercise_egm96(void) {
    double lat = 40.0 * M_PI / 180.0;
    double lon = -105.0 * M_PI / 180.0;
    double n = 0.0;
    if (sidereon_egm96_undulation(lat, lon, &n) != SIDEREON_STATUS_OK) {
        return fail("egm96: undulation", 1);
    }
    if (!same_bits(n, W5_CAPS_EXTRA_EGM96_UNDULATION_M_BITS)) {
        return fail("egm96: undulation range", 1);
    }

    double ellipsoidal = 1600.0;
    double ortho = 0.0, back = 0.0;
    if (sidereon_egm96_orthometric_height_m(ellipsoidal, lat, lon, &ortho) != SIDEREON_STATUS_OK ||
        sidereon_egm96_ellipsoidal_height_m(ortho, lat, lon, &back) != SIDEREON_STATUS_OK) {
        return fail("egm96: height conversions", 1);
    }
    if (!same_bits(ortho, W5_CAPS_EXTRA_EGM96_ORTHOMETRIC_M_BITS) ||
        !same_bits(back, W5_CAPS_EXTRA_EGM96_ELLIPSOIDAL_M_BITS)) {
        return fail("egm96: heights differ from the engine's", 1);
    }
    /* h = H + N and H = h - N must round-trip, and differ by the undulation. */
    if (!approx(back, ellipsoidal, 1e-6) || !approx(ellipsoidal - ortho, n, 1e-6)) {
        return fail("egm96: height round-trip", 1);
    }
    printf("egm96: undulation %.2f m, ortho/ellipsoid round-trip OK\n", n);
    return 0;
}

/* (8) Ground-observer Sun/Moon geometry. */
static int exercise_sun_moon(void) {
    SidereonGeodeticStation station = {40.0, -105.0, 1.6};
    /* 2024-01-01T00:00:00Z. */
    int64_t start_us = (int64_t)1704067200 * 1000000;
    int64_t end_us = start_us + (int64_t)48 * 3600 * 1000000; /* +48 h */

    SidereonBodyAzEl sun;
    SidereonBodyAzEl moon;
    if (sidereon_sun_az_el(&station, start_us, &sun) != SIDEREON_STATUS_OK ||
        sidereon_moon_az_el(&station, start_us, &moon) != SIDEREON_STATUS_OK) {
        return fail("sun/moon: az-el", 1);
    }
    if (!same_bits(sun.azimuth_deg, W5_CAPS_EXTRA_SUN_AZIMUTH_DEG_BITS) ||
        !same_bits(sun.elevation_deg, W5_CAPS_EXTRA_SUN_ELEVATION_DEG_BITS) ||
        !same_bits(sun.range_km, W5_CAPS_EXTRA_SUN_RANGE_KM_BITS)) {
        return fail("sun/moon: sun geometry", 1);
    }
    if (!same_bits(moon.elevation_deg, W5_CAPS_EXTRA_MOON_ELEVATION_DEG_BITS) ||
        !same_bits(moon.range_km, W5_CAPS_EXTRA_MOON_RANGE_KM_BITS)) {
        return fail("sun/moon: moon geometry", 1);
    }

    SidereonMoonIllumination illum;
    if (sidereon_moon_illumination(&station, start_us, &illum) != SIDEREON_STATUS_OK) {
        return fail("sun/moon: illumination", 1);
    }
    if (!same_bits(illum.illuminated_fraction, W5_CAPS_EXTRA_MOON_ILLUMINATED_FRACTION_BITS) ||
        !same_bits(illum.phase_angle_deg, W5_CAPS_EXTRA_MOON_PHASE_ANGLE_DEG_BITS)) {
        return fail("sun/moon: illumination range", 1);
    }

    double moon_el = 0.0;
    if (sidereon_moon_elevation_deg(&station, start_us, &moon_el) != SIDEREON_STATUS_OK ||
        !same_bits(moon_el, W5_CAPS_EXTRA_MOON_ELEVATION_DEG_BITS)) {
        return fail("sun/moon: moon elevation agreement", 1);
    }

    /* An invalid-but-finite station (latitude out of [-90, 90]) must be rejected
     * with INVALID_ARGUMENT, not trip an internal panic. */
    SidereonGeodeticStation bad_station = {200.0, -105.0, 1.6};
    double bad_el = 1.0;
    if (W5_CAPS_EXTRA_MOON_BAD_STATION_OK ||
        sidereon_moon_elevation_deg(&bad_station, start_us, &bad_el) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        bad_el != 0.0) {
        return fail("sun/moon: moon elevation bad-station rejection", 1);
    }

    /* Moonrise/moonset crossings over the window (two-call count + copy). */
    size_t written = 0, required = 0;
    if (sidereon_find_moon_elevation_crossings(&station, start_us, end_us, NULL, NULL, 0, &written,
                                               &required) != SIDEREON_STATUS_OK ||
        required != W5_CAPS_EXTRA_MOON_CROSSING_TIMES_US_COUNT) {
        return fail("sun/moon: crossings query", 1);
    }
    if (required > 0) {
        SidereonMoonElevationCrossing *crossings =
            malloc(required * sizeof *crossings);
        if (crossings == NULL) {
            return fail("sun/moon: crossings alloc", 1);
        }
        if (sidereon_find_moon_elevation_crossings(&station, start_us, end_us, NULL, crossings,
                                                   required, &written, &required) !=
                SIDEREON_STATUS_OK ||
            written != required) {
            free(crossings);
            return fail("sun/moon: crossings copy", 1);
        }
        for (size_t i = 0; i < written; ++i) {
            if (crossings[i].time_unix_us != W5_CAPS_EXTRA_MOON_CROSSING_TIMES_US[i]) {
                free(crossings);
                return fail("sun/moon: crossing time differs from the engine's", 1);
            }
        }
        free(crossings);
    }

    /* Moon meridian transits over the window. */
    size_t t_written = 0, t_required = 0;
    if (sidereon_find_moon_transits(&station, start_us, end_us, 300.0, 1.0, NULL, 0, &t_written,
                                    &t_required) != SIDEREON_STATUS_OK ||
        t_required != W5_CAPS_EXTRA_MOON_TRANSIT_TIMES_US_COUNT) {
        return fail("sun/moon: transits query", 1);
    }
    if (t_required > 0) {
        SidereonMoonTransit *transits = malloc(t_required * sizeof *transits);
        if (transits == NULL) {
            return fail("sun/moon: transits alloc", 1);
        }
        if (sidereon_find_moon_transits(&station, start_us, end_us, 300.0, 1.0, transits, t_required,
                                        &t_written, &t_required) != SIDEREON_STATUS_OK ||
            t_written != t_required) {
            free(transits);
            return fail("sun/moon: transits copy", 1);
        }
        for (size_t i = 0; i < t_written; ++i) {
            if (transits[i].time_unix_us != W5_CAPS_EXTRA_MOON_TRANSIT_TIMES_US[i]) {
                free(transits);
                return fail("sun/moon: transit time differs from the engine's", 1);
            }
        }
        free(transits);
    }

    printf("sun/moon: sun el %.1f deg, moon illum %.0f%%, %zu crossing(s), %zu transit(s) OK\n",
           sun.elevation_deg, illum.illuminated_fraction * 100.0, required, t_required);
    return 0;
}

int main(int argc, char **argv) {
    if (argc < 3) {
        fprintf(stderr, "usage: %s <grg_sp3> <broadcast_nav>\n", argv[0]);
        return 2;
    }
    const char *sp3_path = argv[1];
    const char *nav_path = argv[2];

    size_t sp3_len = 0;
    uint8_t *sp3_bytes = read_file(sp3_path, &sp3_len);
    if (sp3_bytes == NULL) {
        return fail("read GRG SP3", 1);
    }
    SidereonSp3 *sp3 = NULL;
    if (sidereon_sp3_load(sp3_bytes, sp3_len, &sp3) != SIDEREON_STATUS_OK) {
        free(sp3_bytes);
        return fail("sidereon_sp3_load GRG", 1);
    }
    free(sp3_bytes);

    size_t nav_len = 0;
    uint8_t *nav_bytes = read_file(nav_path, &nav_len);
    if (nav_bytes == NULL) {
        sidereon_sp3_free(sp3);
        return fail("read broadcast NAV", 1);
    }
    SidereonBroadcastEphemeris *broadcast = NULL;
    if (sidereon_broadcast_ephemeris_parse_nav(nav_bytes, nav_len, &broadcast) !=
        SIDEREON_STATUS_OK) {
        free(nav_bytes);
        sidereon_sp3_free(sp3);
        return fail("sidereon_broadcast_ephemeris_parse_nav", 1);
    }
    free(nav_bytes);

    int rc = 0;
    if (rc == 0) {
        rc = exercise_trls();
    }
    if (rc == 0) {
        rc = exercise_covariance();
    }
    if (rc == 0) {
        rc = exercise_dop_convention();
    }
    if (rc == 0) {
        rc = exercise_residual_stats();
    }
    if (rc == 0) {
        rc = exercise_predict_batch(sp3);
    }
    if (rc == 0) {
        rc = exercise_broadcast_batch(broadcast);
    }
    if (rc == 0) {
        rc = exercise_invalid_enum();
    }
    if (rc == 0) {
        rc = exercise_leap_seconds();
    }
    if (rc == 0) {
        rc = exercise_egm96();
    }
    if (rc == 0) {
        rc = exercise_sun_moon();
    }

    sidereon_broadcast_ephemeris_free(broadcast);
    sidereon_sp3_free(sp3);

    if (rc == 0) {
        printf("caps extra smoke: OK\n");
    }
    return rc;
}
