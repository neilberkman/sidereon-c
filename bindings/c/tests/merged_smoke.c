/*
 * Smoke coverage for the newly merged core features bound into the C binding:
 *
 *   1. NeQuick-G full slant integration (sidereon_nequick_g_stec_tecu /
 *      sidereon_nequick_g_delay_m over a SidereonNequickGRay).
 *   2. The standalone range RAIM/FDE design (sidereon_raim_fde_design and the
 *      result accessors), with a planted single outlier the loop must exclude.
 *   3. The sequential RTK baseline arc driver (sidereon_solve_rtk_arc), plus
 *      the static, wide-lane, and ionosphere-free RTK arc drivers over small
 *      synthetic arcs.
 *   4. The SPP-seeded PPP auto-initialization drivers
 *      (sidereon_solve_ppp_auto_init_float / _fixed) against the committed PPP
 *      SP3 fixture and observation epochs.
 *   5. RTCM 3 from-scratch construction (sidereon_rtcm_build_*), each message
 *      built from fields, encoded to a frame, and decoded back to confirm the
 *      construct -> encode -> decode loop round-trips.
 *
 * Every call delegates to sidereon-core. Each engine value is compared exactly
 * against sidereon-core's own result for the same inputs, written by
 * tests/valgen (bin w3_merged) into w3_merged_pins.h. argv[1] is the PPP SP3
 * fixture path used by the auto-init drivers.
 */
#include <math.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sidereon.h"
#include "ppp_fixture.h"
#include "w3_merged_pins.h"

static int failures = 0;

static void check(int ok, const char *what) {
    if (!ok) {
        char msg[512];
        size_t n = sidereon_last_error_message((char *)msg, sizeof(msg));
        if (n == 0) {
            msg[0] = '\0';
        }
        /* The last error is sticky and may come from an earlier call, so it is
           reported as context rather than as the cause of this failure. */
        if (msg[0] != '\0') {
            fprintf(stderr, "FAIL: %s (last ABI error, may predate this check: %s)\n",
                    what, msg);
        } else {
            fprintf(stderr, "FAIL: %s\n", what);
        }
        failures++;
    }
}

static uint64_t f64_bits(double value) {
    uint64_t bits;
    memcpy(&bits, &value, sizeof(bits));
    return bits;
}

static int bits_equal(const double *values, const uint64_t *expected, size_t count) {
    for (size_t i = 0; i < count; i++) {
        if (f64_bits(values[i]) != expected[i]) {
            return 0;
        }
    }
    return 1;
}

static double bits_to_f64(uint64_t bits) {
    double value;
    memcpy(&value, &bits, sizeof(value));
    return value;
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
    uint8_t *buf = (uint8_t *)malloc((size_t)size);
    if (buf == NULL) {
        fclose(f);
        return NULL;
    }
    size_t read = fread(buf, 1, (size_t)size, f);
    fclose(f);
    if (read != (size_t)size) {
        free(buf);
        return NULL;
    }
    *out_len = read;
    return buf;
}

/* ------------------------------------------------------------------ NeQuick */

static void test_nequick_slant(void) {
    SidereonNequickGRay ray;
    memset(&ray, 0, sizeof(ray));
    ray.month = 6;
    ray.utc_hours = 12.0;
    ray.station_lon_deg = 9.0;
    ray.station_lat_deg = 45.0;
    ray.station_height_m = 0.0;
    ray.satellite_lon_deg = 12.0;
    ray.satellite_lat_deg = 40.0;
    ray.satellite_height_m = 20000000.0;

    const double f_e1 = 1.57542e9;

    double stec = -1.0;
    check(sidereon_nequick_g_stec_tecu(0.0, 0.0, 0.0, &ray, &stec) == SIDEREON_STATUS_OK &&
              f64_bits(stec) == W3M_NEQUICK_STEC_DEFAULT,
          "nequick_g_stec_tecu default coeffs");

    double delay = -1.0;
    check(sidereon_nequick_g_delay_m(0.0, 0.0, 0.0, &ray, f_e1, &delay) == SIDEREON_STATUS_OK &&
              f64_bits(delay) == W3M_NEQUICK_DELAY_DEFAULT,
          "nequick_g_delay_m default coeffs");

    /* The delay is the dispersive map of the same slant TEC: delay =
     * stec_tecu * 40.3e16 / f^2 with stec_tecu = stec (TECU = 1e16 e/m^2). */
    double expected = stec * (40.3e16 / (f_e1 * f_e1));
    check(fabs(delay - expected) < 1e-6 * expected, "nequick_g delay matches stec mapping");

    double stec2 = -1.0;
    check(sidereon_nequick_g_stec_tecu(80.0, 0.1, 0.05, &ray, &stec2) == SIDEREON_STATUS_OK &&
              f64_bits(stec2) == W3M_NEQUICK_STEC_BROADCAST,
          "nequick_g_stec_tecu broadcast coeffs");
}

/* ------------------------------------------------------------------ RAIM/FDE */

static void test_raim_fde(void) {
    /* Six four-state range rows (last column is the receiver clock partial),
     * with a consistent truth and a single large outlier the loop must drop. */
    const double *dx_true = W3M_RAIM_DX_TRUE;
    const char *ids[6] = {"G01", "G02", "G03", "G04", "G05", "G06"};
    const int outlier = W3M_RAIM_OUTLIER; /* G03 */

    SidereonRangeFdeRow rows[6];
    for (int i = 0; i < 6; i++) {
        double residual = 0.0;
        for (int k = 0; k < 4; k++) {
            residual += W3M_RAIM_DESIGN[i][k] * dx_true[k];
        }
        if (i == outlier) {
            residual += W3M_RAIM_OUTLIER_M;
        }
        rows[i].id = ids[i];
        rows[i].residual_m = residual;
        rows[i].design_row = W3M_RAIM_DESIGN[i];
        rows[i].design_dim = 4;
        rows[i].weight = 1.0;
    }

    SidereonRangeFdeOptions options;
    check(sidereon_range_fde_options_init(&options) == SIDEREON_STATUS_OK &&
              f64_bits(options.p_fa) == W3M_RAIM_OPTIONS_P_FA &&
              options.max_exclusions == W3M_RAIM_OPTIONS_MAX_EXCLUSIONS &&
              options.min_redundancy == W3M_RAIM_OPTIONS_MIN_REDUNDANCY &&
              f64_bits(options.max_exclusion_rms_m) == W3M_RAIM_OPTIONS_MAX_EXCLUSION_RMS_M,
          "range_fde_options_init");

    SidereonRangeFdeResult *result = NULL;
    check(sidereon_raim_fde_design(rows, 6, &options, &result) == SIDEREON_STATUS_OK &&
              result != NULL,
          "raim_fde_design");
    if (!result) {
        return;
    }

    size_t dim = 0;
    check(sidereon_range_fde_result_state_dim(result, &dim) == SIDEREON_STATUS_OK && dim == W3M_RAIM_STATE_DIM,
          "range_fde_result_state_dim");

    double dx[4] = {0};
    size_t written = 0, required = 0;
    check(sidereon_range_fde_result_state_correction(result, dx, 4, &written, &required) ==
              SIDEREON_STATUS_OK &&
              written == W3M_RAIM_STATE_DIM && required == W3M_RAIM_STATE_DIM &&
              bits_equal(dx, W3M_RAIM_STATE_CORRECTION, W3M_RAIM_STATE_DIM),
          "range_fde_result_state_correction");
    double err = 0.0;
    for (int k = 0; k < 4; k++) {
        err += (dx[k] - dx_true[k]) * (dx[k] - dx_true[k]);
    }
    check(sqrt(err) < 1e-9, "range_fde recovers the clean state after exclusion");

    double cov[16] = {0};
    written = 0;
    required = 0;
    check(sidereon_range_fde_result_covariance(result, cov, 16, &written, &required) ==
              SIDEREON_STATUS_OK &&
              written == W3M_RAIM_COVARIANCE_LEN && required == W3M_RAIM_COVARIANCE_LEN &&
              bits_equal(cov, W3M_RAIM_COVARIANCE, W3M_RAIM_COVARIANCE_LEN),
          "range_fde_result_covariance");

    SidereonRangeChiSquareTest test;
    check(sidereon_range_fde_result_global_test(result, &test) == SIDEREON_STATUS_OK &&
              test.testable == W3M_RAIM_TEST_TESTABLE &&
              test.fault_detected == W3M_RAIM_TEST_FAULT_DETECTED &&
              test.has_threshold == W3M_RAIM_TEST_HAS_THRESHOLD &&
              f64_bits(test.threshold) == W3M_RAIM_TEST_THRESHOLD &&
              f64_bits(test.weighted_sum_squares) == W3M_RAIM_TEST_WEIGHTED_SUM_SQUARES &&
              test.dof == W3M_RAIM_TEST_DOF,
          "range_fde global test passes on the protected set");

    size_t iterations = 0;
    check(sidereon_range_fde_result_iterations(result, &iterations) == SIDEREON_STATUS_OK &&
              iterations == W3M_RAIM_ITERATIONS,
          "range_fde performed one exclusion");

    SidereonRtkId excluded[6];
    written = 0;
    required = 0;
    check(sidereon_range_fde_result_excluded(result, excluded, 6, &written, &required) ==
              SIDEREON_STATUS_OK &&
              written == W3M_RAIM_EXCLUDED_COUNT && required == W3M_RAIM_EXCLUDED_COUNT &&
              strcmp((const char *)excluded[0].bytes, W3M_RAIM_EXCLUDED[0]) == 0,
          "range_fde excluded the planted outlier");

    SidereonRangeFdeDiagnostic diag[6];
    written = 0;
    required = 0;
    check(sidereon_range_fde_result_diagnostics(result, diag, 6, &written, &required) ==
              SIDEREON_STATUS_OK &&
              written == W3M_RAIM_DIAGNOSTIC_COUNT && required == W3M_RAIM_DIAGNOSTIC_COUNT &&
              diag[outlier].excluded,
          "range_fde diagnostics flag the outlier");
    for (size_t i = 0; i < written && i < W3M_RAIM_DIAGNOSTIC_COUNT; i++) {
        check(strcmp((const char *)diag[i].id.bytes, ids[i]) == 0 &&
                  diag[i].excluded == W3M_RAIM_DIAGNOSTIC_EXCLUDED[i] &&
                  f64_bits(diag[i].post_fit_residual_m) == W3M_RAIM_DIAGNOSTIC_POST_FIT[i] &&
                  f64_bits(diag[i].normalized_residual) == W3M_RAIM_DIAGNOSTIC_NORMALIZED[i],
              "range_fde diagnostic row");
    }

    sidereon_range_fde_result_free(result);
}

/* -------------------------------------------------------------------- RTK arc */

typedef struct {
    const char *id;
    double pos[3];
    int64_t cycles;
} ArcSat;

static void test_rtk_arc(void) {
    const double c = 299792458.0;
    const double f_l1 = 1575420000.0;
    const double lambda = c / f_l1;

    const ArcSat sats[5] = {
        {"G01", {15000000.0, 7000000.0, 21000000.0}, 0},
        {"G02", {-12000000.0, 18000000.0, 19000000.0}, 4},
        {"G03", {20000000.0, -10000000.0, 17000000.0}, -7},
        {"G04", {-19000000.0, -13000000.0, 20000000.0}, 9},
        {"G05", {9000000.0, 22000000.0, 16000000.0}, -3},
    };
    double base[3] = {-2700000.0, -4300000.0, 3850000.0};
    double baseline[3] = {12.0, -7.0, 5.0};
    double rover[3] = {base[0] + baseline[0], base[1] + baseline[1], base[2] + baseline[2]};

    SidereonRtkArcObservation base_obs[5];
    SidereonRtkArcObservation rover_obs[5];
    SidereonRtkArcPositionEntry positions[5];
    SidereonRtkFloatMapEntry wavelengths[7];
    SidereonRtkFloatMapEntry offsets[7];
    memset(base_obs, 0, sizeof(base_obs));
    memset(rover_obs, 0, sizeof(rover_obs));
    memset(wavelengths, 0, sizeof(wavelengths));
    memset(offsets, 0, sizeof(offsets));
    for (int i = 0; i < 5; i++) {
        double db = 0.0, dr = 0.0;
        for (int k = 0; k < 3; k++) {
            db += (sats[i].pos[k] - base[k]) * (sats[i].pos[k] - base[k]);
            dr += (sats[i].pos[k] - rover[k]) * (sats[i].pos[k] - rover[k]);
        }
        db = sqrt(db);
        dr = sqrt(dr);
        base_obs[i].sat_id = sats[i].id;
        base_obs[i].ambiguity_id = sats[i].id;
        base_obs[i].code_m = db;
        base_obs[i].phase_m = db;
        rover_obs[i].sat_id = sats[i].id;
        rover_obs[i].ambiguity_id = sats[i].id;
        rover_obs[i].code_m = dr;
        rover_obs[i].phase_m = dr + (double)sats[i].cycles * lambda;
        positions[i].id = sats[i].id;
        for (int k = 0; k < 3; k++) {
            positions[i].pos[k] = sats[i].pos[k];
        }
        wavelengths[i].id = sats[i].id;
        wavelengths[i].value = lambda;
        offsets[i].id = sats[i].id;
        offsets[i].value = 0.0;
    }
    wavelengths[5].id = "G02@rover#1";
    wavelengths[5].value = lambda;
    offsets[5].id = "G02@rover#1";
    offsets[5].value = 0.0;
    wavelengths[6].id = "G02@rover#2";
    wavelengths[6].value = lambda;
    offsets[6].id = "G02@rover#2";
    offsets[6].value = 0.0;

    SidereonRtkArcEpoch epochs[2];
    for (int e = 0; e < 2; e++) {
        memset(&epochs[e], 0, sizeof(epochs[e]));
        epochs[e].base = base_obs;
        epochs[e].base_count = 5;
        epochs[e].rover = rover_obs;
        epochs[e].rover_count = 5;
        epochs[e].satellite_positions = positions;
        epochs[e].satellite_position_count = 5;
    }

    SidereonRtkArcConfig config;
    memset(&config, 0, sizeof(config));
    for (int k = 0; k < 3; k++) {
        config.base_m[k] = base[k];
        config.initial_baseline_m[k] = 0.0;
    }
    config.reference_mode = SIDEREON_RTK_ARC_REFERENCE_MODE_AUTO;
    sidereon_rtk_measurement_model_init(&config.model);
    config.baseline_prior_sigma_m = 100.0;
    config.ambiguity_prior_sigma_m = 100.0;
    config.wavelengths_m = wavelengths;
    config.wavelength_count = 5;
    config.offsets_m = offsets;
    config.offset_count = 5;
    sidereon_rtk_arc_update_options_init(&config.update_options);
    config.update_options.report_residuals = true;
    SidereonRtkArcSolution *sol = NULL;
    check(sidereon_solve_rtk_arc(epochs, 2, &config, &sol) == SIDEREON_STATUS_OK && sol != NULL,
          "solve_rtk_arc");
    if (!sol) {
        return;
    }

    size_t epoch_count = 0;
    check(sidereon_rtk_arc_solution_epoch_count(sol, &epoch_count) == SIDEREON_STATUS_OK &&
              epoch_count == W3M_ARC_EPOCH_COUNT,
          "rtk_arc epoch count");

    SidereonRtkArcEpochMetadata meta;
    check(sidereon_rtk_arc_solution_epoch_metadata(sol, 1, &meta) == SIDEREON_STATUS_OK &&
              meta.used_satellite_count == W3M_ARC_E1_USED_SATELLITE_COUNT &&
              meta.sd_ambiguity_count == W3M_ARC_E1_SD_AMBIGUITY_COUNT &&
              meta.fixed_id_count == W3M_ARC_E1_FIXED_ID_COUNT &&
              meta.integer_fixed == W3M_ARC_E1_INTEGER_FIXED &&
              meta.residual_count == W3M_ARC_E1_RESIDUAL_COUNT &&
              bits_equal(meta.reported_baseline_m, W3M_ARC_E1_REPORTED_BASELINE, 3),
          "rtk_arc epoch metadata");

    double err = 0.0;
    for (int k = 0; k < 3; k++) {
        err += (meta.reported_baseline_m[k] - baseline[k]) * (meta.reported_baseline_m[k] - baseline[k]);
    }
    check(sqrt(err) < 2.0, "rtk_arc recovers the planted baseline");

    SidereonSatelliteToken used[8];
    size_t written = 0, required = 0;
    check(sidereon_rtk_arc_solution_epoch_used_satellites(sol, 1, used, 8, &written, &required) ==
              SIDEREON_STATUS_OK &&
              written == W3M_ARC_E1_USED_SATELLITE_COUNT &&
              required == W3M_ARC_E1_USED_SATELLITE_COUNT,
          "rtk_arc used satellites");
    for (size_t i = 0; i < written && i < W3M_ARC_E1_USED_SATELLITE_COUNT; i++) {
        check(strcmp((const char *)used[i].bytes, W3M_ARC_E1_USED_SATELLITES[i]) == 0,
              "rtk_arc used satellite id");
    }

    SidereonRtkAmbiguity amb[8];
    written = 0;
    required = 0;
    check(sidereon_rtk_arc_solution_epoch_sd_ambiguities(sol, 1, amb, 8, &written, &required) ==
              SIDEREON_STATUS_OK &&
              written == required && required == W3M_ARC_E1_SD_AMBIGUITY_COUNT,
          "rtk_arc sd ambiguities");
    for (size_t i = 0; i < written && i < W3M_ARC_E1_SD_AMBIGUITY_COUNT; i++) {
        check(strcmp((const char *)amb[i].id.bytes, W3M_ARC_E1_SD_AMBIGUITY_IDS[i]) == 0 &&
                  f64_bits(amb[i].value_m) == W3M_ARC_E1_SD_AMBIGUITIES[i],
              "rtk_arc sd ambiguity");
    }

    SidereonRtkId fixed_ids[8];
    written = 0;
    required = 0;
    check(sidereon_rtk_arc_solution_epoch_string_ids(
              sol, 1, SIDEREON_RTK_ARC_EPOCH_ID_LIST_FIXED_IDS, fixed_ids, 8, &written, &required) ==
              SIDEREON_STATUS_OK &&
              written == W3M_ARC_E1_FIXED_ID_COUNT && required == W3M_ARC_E1_FIXED_ID_COUNT,
          "rtk_arc fixed id list");
    for (size_t i = 0; i < written && i < W3M_ARC_E1_FIXED_ID_COUNT; i++) {
        check(strcmp((const char *)fixed_ids[i].bytes, W3M_ARC_E1_FIXED_IDS[i]) == 0,
              "rtk_arc fixed id");
    }

    SidereonRtkArcReferenceOut refs[4];
    written = 0;
    required = 0;
    check(sidereon_rtk_arc_solution_references(sol, refs, 4, &written, &required) ==
              SIDEREON_STATUS_OK &&
              written == required && required == W3M_ARC_REFERENCE_COUNT &&
              strcmp((const char *)refs[0].system.bytes, W3M_ARC_REFERENCE0_SYSTEM) == 0 &&
              strcmp((const char *)refs[0].reference_id.bytes, W3M_ARC_REFERENCE0_ID) == 0,
          "rtk_arc references");

    double final_baseline[3] = {0};
    check(sidereon_rtk_arc_solution_final_baseline(sol, final_baseline, 3) == SIDEREON_STATUS_OK &&
              bits_equal(final_baseline, W3M_ARC_FINAL_BASELINE, 3),
          "rtk_arc final baseline");

    size_t final_epochs = 0;
    check(sidereon_rtk_arc_solution_final_epoch_count(sol, &final_epochs) == SIDEREON_STATUS_OK &&
              final_epochs == W3M_ARC_FINAL_EPOCH_COUNT,
          "rtk_arc final epoch count");

    sidereon_rtk_arc_solution_free(sol);

    SidereonRtkStaticArcConfig static_config;
    memset(&static_config, 0, sizeof(static_config));
    static_config.arc = config;
    sidereon_rtk_float_options_init(&static_config.float_options);
    sidereon_rtk_fixed_options_init(&static_config.fixed_options);
    sidereon_rtk_residual_validation_options_init(&static_config.residual_options);

    SidereonRtkStaticArcSolution *static_sol = NULL;
    check(sidereon_solve_static_rtk_arc(epochs, 2, &static_config, &static_sol) ==
                  SIDEREON_STATUS_OK &&
              static_sol != NULL,
          "solve_static_rtk_arc");
    if (static_sol) {
        double float_baseline[3] = {0};
        double fixed_baseline[3] = {0};
        check(sidereon_rtk_static_arc_solution_float_baseline_ecef(static_sol, float_baseline, 3) ==
                      SIDEREON_STATUS_OK &&
                  bits_equal(float_baseline, W3M_STATIC_FLOAT_BASELINE, 3),
              "static_rtk_arc float baseline");
        check(sidereon_rtk_static_arc_solution_fixed_baseline_ecef(static_sol, fixed_baseline, 3) ==
                      SIDEREON_STATUS_OK &&
                  bits_equal(fixed_baseline, W3M_STATIC_FIXED_BASELINE, 3),
              "static_rtk_arc fixed baseline");

        SidereonRtkFixedMetadata fixed_meta;
        check(sidereon_rtk_static_arc_solution_fixed_metadata(static_sol, &fixed_meta) ==
                      SIDEREON_STATUS_OK &&
                  fixed_meta.fixed_ambiguity_count == W3M_STATIC_FIXED_AMBIGUITY_COUNT &&
                  fixed_meta.geometry_quality.tier == W3M_STATIC_FIXED_GEOMETRY_TIER &&
                  fixed_meta.geometry_quality.redundancy ==
                      W3M_STATIC_FIXED_GEOMETRY_REDUNDANCY &&
                  fixed_meta.geometry_quality.rank == W3M_STATIC_FIXED_GEOMETRY_RANK &&
                  fixed_meta.geometry_quality.covariance_validated ==
                      W3M_STATIC_FIXED_GEOMETRY_COVARIANCE_VALIDATED &&
                  fixed_meta.integer_status == W3M_STATIC_INTEGER_STATUS,
              "static_rtk_arc fixed metadata");

        SidereonGeometryQuality static_geometry;
        check(sidereon_rtk_static_arc_solution_geometry_quality(static_sol, &static_geometry) ==
                      SIDEREON_STATUS_OK &&
                  static_geometry.tier == W3M_STATIC_GEOMETRY_TIER &&
                  static_geometry.redundancy == W3M_STATIC_GEOMETRY_REDUNDANCY &&
                  static_geometry.rank == W3M_STATIC_GEOMETRY_RANK &&
                  f64_bits(static_geometry.condition_number) ==
                      W3M_STATIC_GEOMETRY_CONDITION_NUMBER &&
                  f64_bits(static_geometry.gdop) == W3M_STATIC_GEOMETRY_GDOP &&
                  static_geometry.raim_checkable == W3M_STATIC_GEOMETRY_RAIM_CHECKABLE &&
                  static_geometry.covariance_validated ==
                      W3M_STATIC_GEOMETRY_COVARIANCE_VALIDATED,
              "static_rtk_arc geometry quality");

        SidereonRtkFixedAmbiguity static_fixed[8];
        written = 0;
        required = 0;
        check(sidereon_rtk_static_arc_solution_fixed_ambiguities(static_sol, static_fixed, 8, &written,
                                                                 &required) == SIDEREON_STATUS_OK &&
                  written == required && required == W3M_STATIC_FIXED_AMBIGUITY_COUNT,
              "static_rtk_arc fixed ambiguities");

        SidereonRtkAmbiguitySatelliteOut ambiguity_sats[8];
        written = 0;
        required = 0;
        check(sidereon_rtk_static_arc_solution_ambiguity_satellites(
                  static_sol, ambiguity_sats, 8, &written, &required) == SIDEREON_STATUS_OK &&
                  written == required && required == W3M_STATIC_AMBIGUITY_SATELLITE_COUNT,
              "static_rtk_arc ambiguity satellites");

        sidereon_rtk_static_arc_solution_free(static_sol);
    }

    SidereonRtkArcObservation pre_base_obs[2][5];
    SidereonRtkArcObservation pre_rover_obs[2][5];
    SidereonRtkArcEpoch pre_epochs[2];
    for (int e = 0; e < 2; e++) {
        memcpy(pre_base_obs[e], base_obs, sizeof(base_obs));
        memcpy(pre_rover_obs[e], rover_obs, sizeof(rover_obs));
        memset(&pre_epochs[e], 0, sizeof(pre_epochs[e]));
        pre_epochs[e].base = pre_base_obs[e];
        pre_epochs[e].base_count = 5;
        pre_epochs[e].rover = pre_rover_obs[e];
        pre_epochs[e].rover_count = 5;
        pre_epochs[e].satellite_positions = positions;
        pre_epochs[e].satellite_position_count = 5;
    }
    pre_rover_obs[1][1].has_lli = true;
    pre_rover_obs[1][1].lli = 1;

    SidereonRtkArcConfig split_config = config;
    split_config.wavelength_count = 7;
    split_config.offset_count = 7;
    split_config.preprocessing.has_cycle_slip = true;
    split_config.preprocessing.cycle_slip = SIDEREON_RTK_CYCLE_SLIP_POLICY_SPLIT_ARC;
    split_config.preprocessing.has_hatch_window_cap = true;
    split_config.preprocessing.hatch_window_cap = 8;
    split_config.preprocessing.has_elevation_mask_deg = true;
    split_config.preprocessing.elevation_mask_deg = -20.0;

    SidereonRtkArcSolution *split_sol = NULL;
    check(sidereon_solve_rtk_arc(pre_epochs, 2, &split_config, &split_sol) == SIDEREON_STATUS_OK &&
              split_sol != NULL,
          "solve_rtk_arc preprocessing split mask");
    if (split_sol) {
        SidereonRtkArcSplitArc split_arcs[8];
        written = 0;
        required = 0;
        check(sidereon_rtk_arc_solution_split_cycle_slip_arcs(
                  split_sol, split_arcs, 8, &written, &required) == SIDEREON_STATUS_OK &&
                  written == required && required == W3M_SPLIT_ARC_COUNT,
              "rtk_arc split cycle-slip arc metadata");
        for (size_t i = 0; i < written && i < W3M_SPLIT_ARC_COUNT; i++) {
            check(split_arcs[i].receiver == W3M_SPLIT_ARC_RECEIVERS[i] &&
                      strcmp((const char *)split_arcs[i].satellite_id.bytes,
                             W3M_SPLIT_ARC_SATELLITES[i]) == 0 &&
                      strcmp((const char *)split_arcs[i].ambiguity_id.bytes,
                             W3M_SPLIT_ARC_AMBIGUITIES[i]) == 0 &&
                      split_arcs[i].start_epoch_index == W3M_SPLIT_ARC_STARTS[i] &&
                      split_arcs[i].end_epoch_index == W3M_SPLIT_ARC_ENDS[i] &&
                      split_arcs[i].n_epochs == W3M_SPLIT_ARC_N_EPOCHS[i],
                  "rtk_arc split cycle-slip arc content");
        }

        SidereonSatelliteToken masked[5];
        written = 0;
        required = 0;
        check(sidereon_rtk_arc_solution_elevation_masked_sats(split_sol, masked, 5, &written,
                                                              &required) == SIDEREON_STATUS_OK &&
                  written == required && required == W3M_SPLIT_MASKED_COUNT &&
                  strcmp((const char *)masked[0].bytes, W3M_SPLIT_MASKED[0]) == 0,
              "rtk_arc elevation masked satellites metadata");

        double covariance[128];
        written = 0;
        required = 0;
        check(sidereon_rtk_arc_solution_measurement_covariance(split_sol, NULL, 0, &written,
                                                               &required) == SIDEREON_STATUS_OK &&
                  written == 0 && required == W3M_SPLIT_COVARIANCE_LEN && required <= 128,
              "rtk_arc covariance metadata sizing");
        if (required > 0 && required <= 128) {
            written = 0;
            size_t required_again = 0;
            check(sidereon_rtk_arc_solution_measurement_covariance(
                      split_sol, covariance, 128, &written, &required_again) == SIDEREON_STATUS_OK &&
                      written == required_again && required_again == required &&
                      bits_equal(covariance, W3M_SPLIT_COVARIANCE, W3M_SPLIT_COVARIANCE_LEN),
                  "rtk_arc covariance metadata values");
        }

        sidereon_rtk_arc_solution_free(split_sol);
    }

    SidereonRtkArcConfig drop_config = config;
    drop_config.preprocessing.has_cycle_slip = true;
    drop_config.preprocessing.cycle_slip = SIDEREON_RTK_CYCLE_SLIP_POLICY_DROP_SATELLITE;
    SidereonRtkArcSolution *drop_sol = NULL;
    check(sidereon_solve_rtk_arc(pre_epochs, 2, &drop_config, &drop_sol) == SIDEREON_STATUS_OK &&
              drop_sol != NULL,
          "solve_rtk_arc preprocessing drop");
    if (drop_sol) {
        SidereonSatelliteToken dropped[5];
        written = 0;
        required = 0;
        check(sidereon_rtk_arc_solution_dropped_sats(drop_sol, dropped, 5, &written, &required) ==
                      SIDEREON_STATUS_OK &&
                  written == required && required == W3M_DROP_COUNT &&
                  strcmp((const char *)dropped[0].bytes, W3M_DROPPED[0]) == 0,
              "rtk_arc dropped satellites metadata");
        sidereon_rtk_arc_solution_free(drop_sol);
    }
}

static void set_dual_obs(SidereonRtkDualFrequencyObservation *obs, const char *id, double p1_m,
                         double p2_m, double phi1_cycles, double phi2_cycles) {
    memset(obs, 0, sizeof(*obs));
    obs->ambiguity_id = id;
    obs->p1_m = p1_m;
    obs->p2_m = p2_m;
    obs->phi1_cycles = phi1_cycles;
    obs->phi2_cycles = phi2_cycles;
    obs->f1_hz = 1575420000.0;
    obs->f2_hz = 1227600000.0;
}

static void set_dual_sat(SidereonRtkDualFrequencySatelliteObservation *obs, const char *id,
                         double base_p1_m, double base_p2_m, double base_phi1_cycles,
                         double rover_p1_m, double rover_p2_m, double rover_phi1_cycles) {
    memset(obs, 0, sizeof(*obs));
    obs->sat_id = id;
    set_dual_obs(&obs->base, id, base_p1_m, base_p2_m, base_phi1_cycles, 0.0);
    set_dual_obs(&obs->rover, id, rover_p1_m, rover_p2_m, rover_phi1_cycles, 0.0);
}

static void test_rtk_dual_arc_drivers(void) {
    SidereonRtkArcPositionEntry positions[4] = {
        {"G01", {14350000.0, 3190000.0, 21440000.0}},
        {"G02", {20000000.0, 3000000.0, 18000000.0}},
        {"G03", {9000000.0, 9000000.0, 22000000.0}},
        {"G04", {16000000.0, -4000000.0, 21000000.0}},
    };

    SidereonRtkDualFrequencySatelliteObservation observations[3][4];
    for (int e = 0; e < 3; e++) {
        set_dual_sat(&observations[e][0], "G01", 20000020.0, 20000022.0, 2.0, 20000050.0,
                     20000052.5, 5.0);
        set_dual_sat(&observations[e][1], "G02", 20000010.0, 20000012.0, 1.0, 20000042.0,
                     20000044.5, 7.0);
        set_dual_sat(&observations[e][2], "G03", 19999980.0, 19999982.0, -2.0, 20000005.0,
                     20000007.5, 0.0);
        set_dual_sat(&observations[e][3], "G04", 20000040.0, 20000042.0, 4.0, 20000073.0,
                     20000075.5, 8.0);
    }

    const char *keys[3] = {"000", "001", "002"};
    SidereonRtkDualFrequencyArcEpoch epochs[3];
    for (int e = 0; e < 3; e++) {
        memset(&epochs[e], 0, sizeof(epochs[e]));
        epochs[e].jd_whole = 2460100.5;
        epochs[e].jd_fraction = 0.25;
        epochs[e].epoch_sort_key = keys[e];
        epochs[e].has_gap_time_s = true;
        epochs[e].gap_time_s = (double)e;
        epochs[e].observations = observations[e];
        epochs[e].observation_count = 4;
        epochs[e].satellite_positions = positions;
        epochs[e].satellite_position_count = 4;
    }

    SidereonRtkWideLaneArcConfig wl_config;
    memset(&wl_config, 0, sizeof(wl_config));
    wl_config.base_m[0] = 3512900.0;
    wl_config.base_m[1] = 780500.0;
    wl_config.base_m[2] = 5248700.0;
    wl_config.reference_mode = SIDEREON_RTK_ARC_REFERENCE_MODE_AUTO;
    wl_config.options.min_epochs = 2;
    wl_config.options.tolerance_cycles = 0.5;
    wl_config.options.skip_short_fragments = false;

    SidereonRtkWideLaneArcSolution *wl_sol = NULL;
    check(sidereon_fix_wide_lane_rtk_arc(epochs, 3, &wl_config, &wl_sol) == SIDEREON_STATUS_OK &&
              wl_sol != NULL,
          "fix_wide_lane_rtk_arc");
    if (!wl_sol) {
        return;
    }

    size_t epoch_count = 0;
    check(sidereon_rtk_wide_lane_arc_solution_epoch_count(wl_sol, &epoch_count) ==
                  SIDEREON_STATUS_OK &&
              epoch_count == W3M_WL_EPOCH_COUNT,
          "wide_lane_rtk_arc epoch count");

    SidereonGeometryQuality wl_geometry;
    check(sidereon_rtk_wide_lane_arc_solution_geometry_quality(wl_sol, &wl_geometry) ==
                  SIDEREON_STATUS_OK &&
              wl_geometry.tier == W3M_WL_GEOMETRY_TIER &&
              wl_geometry.rank == W3M_WL_GEOMETRY_RANK &&
              wl_geometry.redundancy == W3M_WL_GEOMETRY_REDUNDANCY &&
              f64_bits(wl_geometry.condition_number) == W3M_WL_GEOMETRY_CONDITION_NUMBER &&
              f64_bits(wl_geometry.gdop) == W3M_WL_GEOMETRY_GDOP &&
              wl_geometry.raim_checkable == W3M_WL_GEOMETRY_RAIM_CHECKABLE &&
              wl_geometry.covariance_validated == W3M_WL_GEOMETRY_COVARIANCE_VALIDATED,
          "wide_lane_rtk_arc geometry quality");

    SidereonRtkWideLaneCycle cycles[8];
    size_t written = 0;
    size_t required = 0;
    check(sidereon_rtk_wide_lane_arc_solution_wide_lane_cycles(wl_sol, cycles, 8, &written,
                                                               &required) == SIDEREON_STATUS_OK &&
              written == required && required == W3M_WL_CYCLE_COUNT,
          "wide_lane_rtk_arc cycles");
    for (size_t i = 0; i < written && i < W3M_WL_CYCLE_COUNT; i++) {
        check(strcmp((const char *)cycles[i].id.bytes, W3M_WL_CYCLE_IDS[i]) == 0 &&
                  cycles[i].cycles == W3M_WL_CYCLES[i],
              "wide_lane_rtk_arc cycle");
    }
    /* The ionosphere-free preparation below takes every wide-lane cycle. */
    const size_t cycle_count = written;

    SidereonRtkArcReferenceOut refs[4];
    written = 0;
    required = 0;
    check(sidereon_rtk_wide_lane_arc_solution_references(wl_sol, refs, 4, &written, &required) ==
                  SIDEREON_STATUS_OK &&
              written == required && required == W3M_WL_REFERENCE_COUNT,
          "wide_lane_rtk_arc references");

    SidereonRtkIonosphereFreeArcConfig if_config;
    memset(&if_config, 0, sizeof(if_config));
    if_config.base_m[0] = wl_config.base_m[0];
    if_config.base_m[1] = wl_config.base_m[1];
    if_config.base_m[2] = wl_config.base_m[2];
    if_config.reference_mode = SIDEREON_RTK_ARC_REFERENCE_MODE_AUTO;
    if_config.apply_troposphere = false;

    SidereonRtkIonosphereFreeArcSolution *if_sol = NULL;
    check(sidereon_prepare_ionosphere_free_rtk_arc(epochs, 3, cycles, cycle_count, &if_config,
                                                   &if_sol) == SIDEREON_STATUS_OK &&
              if_sol != NULL,
          "prepare_ionosphere_free_rtk_arc");
    if (if_sol) {
        epoch_count = 0;
        check(sidereon_rtk_ionosphere_free_arc_solution_epoch_count(if_sol, &epoch_count) ==
                      SIDEREON_STATUS_OK &&
                  epoch_count == W3M_IF_EPOCH_COUNT,
              "ionosphere_free_rtk_arc epoch count");

        SidereonRtkMapValue wavelengths[8];
        written = 0;
        required = 0;
        check(sidereon_rtk_ionosphere_free_arc_solution_wavelengths_m(if_sol, wavelengths, 8,
                                                                      &written, &required) ==
                      SIDEREON_STATUS_OK &&
                  written == required && required == W3M_IF_WAVELENGTH_COUNT &&
                  f64_bits(wavelengths[0].value) == W3M_IF_WAVELENGTHS[0],
              "ionosphere_free_rtk_arc wavelengths");

        SidereonRtkMapValue offsets[8];
        written = 0;
        required = 0;
        check(sidereon_rtk_ionosphere_free_arc_solution_offsets_m(if_sol, offsets, 8, &written,
                                                                  &required) == SIDEREON_STATUS_OK &&
                  written == required && required == W3M_IF_OFFSET_COUNT &&
                  f64_bits(offsets[0].value) == W3M_IF_OFFSETS[0],
              "ionosphere_free_rtk_arc offsets");

        SidereonRtkArcEpochOutMetadata meta;
        check(sidereon_rtk_ionosphere_free_arc_solution_epoch_metadata(if_sol, 0, &meta) ==
                      SIDEREON_STATUS_OK &&
                  meta.base_count == W3M_IF_E0_BASE_COUNT &&
                  meta.rover_count == W3M_IF_E0_ROVER_COUNT &&
                  meta.satellite_position_count == W3M_IF_E0_POSITION_COUNT,
              "ionosphere_free_rtk_arc epoch metadata");

        SidereonRtkArcObservationOut base_obs[8];
        written = 0;
        required = 0;
        check(sidereon_rtk_ionosphere_free_arc_solution_epoch_base_observations(
                  if_sol, 0, base_obs, 8, &written, &required) == SIDEREON_STATUS_OK &&
                  written == required && required == meta.base_count &&
                  strcmp((const char *)base_obs[0].sat_id.bytes, W3M_IF_E0_BASE0_SAT) == 0,
              "ionosphere_free_rtk_arc base observations");

        SidereonRtkArcPositionOut pos[8];
        written = 0;
        required = 0;
        check(sidereon_rtk_ionosphere_free_arc_solution_epoch_satellite_positions(
                  if_sol, 0, pos, 8, &written, &required) == SIDEREON_STATUS_OK &&
                  written == required && required == meta.satellite_position_count,
              "ionosphere_free_rtk_arc positions");

        sidereon_rtk_ionosphere_free_arc_solution_free(if_sol);
    }

    sidereon_rtk_wide_lane_arc_solution_free(wl_sol);
}

/* --------------------------------------------------------------- PPP auto-init */

static void fill_ppp_epochs(SidereonPppObservation observations[PPP_OBS_COUNT],
                            SidereonPppEpoch epochs[PPP_EPOCH_COUNT]) {
    for (size_t i = 0; i < PPP_OBS_COUNT; i++) {
        observations[i].sat_id = PPP_OBS_SAT_IDS[i];
        observations[i].ambiguity_id = PPP_OBS_AMBIGUITY_IDS[i];
        observations[i].code_m = bits_to_f64(PPP_OBS_CODE_BITS[i]);
        observations[i].phase_m = bits_to_f64(PPP_OBS_PHASE_BITS[i]);
        observations[i].freq1_hz = bits_to_f64(PPP_OBS_FREQ1_HZ_BITS[i]);
        observations[i].freq2_hz = bits_to_f64(PPP_OBS_FREQ2_HZ_BITS[i]);
        observations[i].code1_signal = NULL;
        observations[i].code2_signal = NULL;
        observations[i].phase1_signal = NULL;
        observations[i].phase2_signal = NULL;
        observations[i].has_glonass_channel = false;
        observations[i].glonass_channel = 0;
    }
    for (size_t i = 0; i < PPP_EPOCH_COUNT; i++) {
        epochs[i].civil.year = PPP_EPOCH_YEARS[i];
        epochs[i].civil.month = PPP_EPOCH_MONTHS[i];
        epochs[i].civil.day = PPP_EPOCH_DAYS[i];
        epochs[i].civil.hour = PPP_EPOCH_HOURS[i];
        epochs[i].civil.minute = PPP_EPOCH_MINUTES[i];
        epochs[i].civil.second = bits_to_f64(PPP_EPOCH_SECOND_BITS[i]);
        epochs[i].jd_whole = bits_to_f64(PPP_EPOCH_JD_WHOLE_BITS[i]);
        epochs[i].jd_fraction = bits_to_f64(PPP_EPOCH_JD_FRACTION_BITS[i]);
        epochs[i].t_rx_j2000_s = bits_to_f64(PPP_EPOCH_T_RX_J2000_S_BITS[i]);
        epochs[i].observations = &observations[PPP_EPOCH_OBS_OFFSETS[i]];
        epochs[i].observation_count = PPP_EPOCH_OBS_COUNTS[i];
    }
}

static void test_ppp_auto_init(const char *sp3_path) {
    size_t sp3_len = 0;
    uint8_t *sp3_bytes = read_file(sp3_path, &sp3_len);
    check(sp3_bytes != NULL, "ppp sp3 read_file");
    if (!sp3_bytes) {
        return;
    }
    SidereonSp3 *sp3 = NULL;
    check(sidereon_sp3_load(sp3_bytes, sp3_len, &sp3) == SIDEREON_STATUS_OK && sp3 != NULL,
          "ppp sp3 load");
    free(sp3_bytes);
    if (!sp3) {
        return;
    }

    static SidereonPppObservation observations[PPP_OBS_COUNT];
    static SidereonPppEpoch epochs[PPP_EPOCH_COUNT];
    fill_ppp_epochs(observations, epochs);

    SidereonPppRangeCorrections corrections;
    sidereon_ppp_range_corrections_init(&corrections);
    corrections.receiver_antenna = NULL;

    /* A float config that reuses the fixture epochs and settings; the auto-init
     * driver ignores the embedded initial_state and seeds itself. */
    SidereonPppFloatConfig float_config;
    memset(&float_config, 0, sizeof(float_config));
    float_config.epochs = epochs;
    float_config.epoch_count = PPP_EPOCH_COUNT;
    sidereon_ppp_measurement_weights_init(&float_config.weights);
    float_config.weights.code = bits_to_f64(PPP_WEIGHT_CODE_BITS);
    float_config.weights.phase = bits_to_f64(PPP_WEIGHT_PHASE_BITS);
    sidereon_ppp_troposphere_options_init(&float_config.tropo);
    float_config.corrections = corrections;
    sidereon_ppp_float_options_init(&float_config.options);
    float_config.residual_screen = false;
    /* initial_state is required to be structurally present even though auto-init
     * overrides it; point its arrays at valid storage. */
    static double initial_clocks[PPP_EPOCH_COUNT];
    float_config.initial_state.clocks_m = initial_clocks;
    float_config.initial_state.clock_count = PPP_EPOCH_COUNT;
    float_config.initial_state.ambiguities_m = NULL;
    float_config.initial_state.ambiguity_count = 0;

    SidereonPppAutoInitOptions options;
    check(sidereon_ppp_auto_init_options_init(&options) == SIDEREON_STATUS_OK &&
              options.has_initial_guess == W3M_PPP_AUTO_HAS_INITIAL_GUESS &&
              f64_bits(options.spp_pressure_hpa) == W3M_PPP_AUTO_SPP_PRESSURE_HPA,
          "ppp_auto_init_options_init");

    size_t sp3_epochs_before = 0;
    check(sidereon_sp3_epoch_count(sp3, &sp3_epochs_before) == SIDEREON_STATUS_OK,
          "ppp alias control reads live sp3 before refusal");
    check(sidereon_solve_ppp_auto_init_float(
              sp3, &float_config, &options,
              (SidereonPppFloatSolution **)(void *)sp3) == SIDEREON_STATUS_INVALID_ARGUMENT,
          "ppp auto-init float refuses output overlapping live sp3 handle");
    size_t sp3_epochs_after = 0;
    check(sidereon_sp3_epoch_count(sp3, &sp3_epochs_after) == SIDEREON_STATUS_OK &&
              sp3_epochs_after == sp3_epochs_before,
          "ppp auto-init float overlap refusal preserves sp3 handle");

    SidereonPppFloatSolution *float_sol = NULL;
    check(sidereon_solve_ppp_auto_init_float(sp3, &float_config, &options, &float_sol) ==
              SIDEREON_STATUS_OK &&
              float_sol != NULL,
          "solve_ppp_auto_init_float");
    if (float_sol) {
        double position[3] = {0};
        check(sidereon_ppp_float_solution_position(float_sol, position, 3) == SIDEREON_STATUS_OK &&
                  bits_equal(position, W3M_PPP_AUTO_FLOAT_POSITION, 3),
              "ppp auto-init float position");
        sidereon_ppp_float_solution_free(float_sol);
    }

    union {
        SidereonPppFloatConfig config;
        SidereonPppFloatSolution *solution;
    } aliased_float;
    aliased_float.config = float_config;
    check(sidereon_solve_ppp_auto_init_float(sp3, &aliased_float.config, &options,
                                             &aliased_float.solution) == SIDEREON_STATUS_OK &&
              aliased_float.solution != NULL,
          "ppp auto-init float snapshots config before writing aliased output");
    if (aliased_float.solution) {
        double position[3] = {0};
        check(sidereon_ppp_float_solution_position(aliased_float.solution, position, 3) ==
                  SIDEREON_STATUS_OK &&
                  bits_equal(position, W3M_PPP_AUTO_FLOAT_POSITION, 3),
              "ppp auto-init float aliased output matches disjoint result");
        sidereon_ppp_float_solution_free(aliased_float.solution);
    }

    SidereonPppFixedConfig fixed_config;
    memset(&fixed_config, 0, sizeof(fixed_config));
    fixed_config.epochs = epochs;
    fixed_config.epoch_count = PPP_EPOCH_COUNT;
    sidereon_ppp_measurement_weights_init(&fixed_config.weights);
    fixed_config.weights.code = bits_to_f64(PPP_WEIGHT_CODE_BITS);
    fixed_config.weights.phase = bits_to_f64(PPP_WEIGHT_PHASE_BITS);
    sidereon_ppp_troposphere_options_init(&fixed_config.tropo);
    fixed_config.corrections = corrections;
    sidereon_ppp_float_options_init(&fixed_config.options);
    sidereon_ppp_fixed_ambiguity_options_init(&fixed_config.ambiguity);

    static SidereonPppFloatMapEntry wavelengths[PPP_FIXED_AMBIGUITY_COUNT];
    static SidereonPppFloatMapEntry offsets[PPP_FIXED_AMBIGUITY_COUNT];
    for (size_t i = 0; i < PPP_FIXED_AMBIGUITY_COUNT; i++) {
        wavelengths[i].id = PPP_WAVELENGTH_IDS[i];
        wavelengths[i].value = bits_to_f64(PPP_WAVELENGTH_BITS[i]);
        offsets[i].id = PPP_OFFSET_IDS[i];
        offsets[i].value = bits_to_f64(PPP_OFFSET_BITS[i]);
    }
    fixed_config.ambiguity.wavelengths_m = wavelengths;
    fixed_config.ambiguity.wavelength_count = PPP_FIXED_AMBIGUITY_COUNT;
    fixed_config.ambiguity.offsets_m = offsets;
    fixed_config.ambiguity.offset_count = PPP_FIXED_AMBIGUITY_COUNT;
    fixed_config.ambiguity.ratio_threshold = bits_to_f64(PPP_FIXED_RATIO_THRESHOLD_BITS);

    check(sidereon_solve_ppp_auto_init_fixed(
              sp3, &float_config, &fixed_config, &options,
              (SidereonPppFixedSolution **)(void *)sp3) == SIDEREON_STATUS_INVALID_ARGUMENT,
          "ppp auto-init fixed refuses output overlapping live sp3 handle");
    check(sidereon_sp3_epoch_count(sp3, &sp3_epochs_after) == SIDEREON_STATUS_OK &&
              sp3_epochs_after == sp3_epochs_before,
          "ppp auto-init fixed overlap refusal preserves sp3 handle");

    SidereonPppFixedSolution *fixed_sol = NULL;
    check(sidereon_solve_ppp_auto_init_fixed(sp3, &float_config, &fixed_config, &options,
                                             &fixed_sol) == SIDEREON_STATUS_OK &&
              fixed_sol != NULL,
          "solve_ppp_auto_init_fixed");
    if (fixed_sol) {
        double position[3] = {0};
        check(sidereon_ppp_fixed_solution_position(fixed_sol, position, 3) == SIDEREON_STATUS_OK &&
                  bits_equal(position, W3M_PPP_AUTO_FIXED_POSITION, 3),
              "ppp auto-init fixed position");
        sidereon_ppp_fixed_solution_free(fixed_sol);
    }

    union {
        SidereonPppFixedConfig config;
        SidereonPppFixedSolution *solution;
    } aliased_fixed;
    aliased_fixed.config = fixed_config;
    check(sidereon_solve_ppp_auto_init_fixed(sp3, &float_config, &aliased_fixed.config,
                                             &options, &aliased_fixed.solution) ==
                  SIDEREON_STATUS_OK &&
              aliased_fixed.solution != NULL,
          "ppp auto-init fixed snapshots config before writing aliased output");
    if (aliased_fixed.solution) {
        double position[3] = {0};
        check(sidereon_ppp_fixed_solution_position(aliased_fixed.solution, position, 3) ==
                  SIDEREON_STATUS_OK &&
                  bits_equal(position, W3M_PPP_AUTO_FIXED_POSITION, 3),
              "ppp auto-init fixed aliased output matches disjoint result");
        sidereon_ppp_fixed_solution_free(aliased_fixed.solution);
    }

    sidereon_sp3_free(sp3);
}

/* ------------------------------------------------------------- RTCM construct */

/* Build one message, encode it to a frame, decode the frame back, and return the
 * decoded list (or NULL). The caller frees it. */
static SidereonRtcmMessages *roundtrip(SidereonRtcmMessages *built, const char *what) {
    if (!built) {
        check(0, what);
        return NULL;
    }
    uint8_t frame[256];
    size_t written = 0, required = 0;
    int ok = sidereon_rtcm_message_to_frame(built, 0, frame, sizeof(frame), &written, &required) ==
                 SIDEREON_STATUS_OK &&
             written == required && required > 0;
    sidereon_rtcm_messages_free(built);
    check(ok, what);
    if (!ok) {
        return NULL;
    }
    SidereonRtcmMessages *decoded = NULL;
    check(sidereon_rtcm_decode_messages(frame, written, &decoded) == SIDEREON_STATUS_OK &&
              decoded != NULL,
          "rtcm construct decode");
    if (decoded) {
        size_t count = 0;
        check(sidereon_rtcm_messages_count(decoded, &count) == SIDEREON_STATUS_OK && count == 1,
              "rtcm construct decode count");
    }
    return decoded;
}

/* Each constructed message is compared, after encode and decode, with the
 * fields it was built from. The captured 1046 frame is compared with
 * sidereon-core's decode of it (tests/valgen, bin w3_merged). */
static void test_rtcm_construct(void) {
    /* 1006 station coordinates. */
    SidereonRtcmStationCoordinates station;
    memset(&station, 0, sizeof(station));
    station.message_number = 1006;
    station.reference_station_id = 2003;
    station.itrf_realization_year = 1;
    station.gps_indicator = true;
    station.ecef_x = 38403690L;
    station.ecef_y = 6863060L;
    station.ecef_z = 50208700L;
    station.has_antenna_height = true;
    station.antenna_height = 15000;
    SidereonRtcmMessages *built = NULL;
    check(sidereon_rtcm_build_station_coordinates(&station, &built) == SIDEREON_STATUS_OK,
          "rtcm_build_station_coordinates");
    SidereonRtcmMessages *decoded = roundtrip(built, "rtcm station construct round-trip");
    if (decoded) {
        SidereonRtcmStationCoordinates got;
        check(sidereon_rtcm_message_station_coordinates(decoded, 0, &got) == SIDEREON_STATUS_OK &&
                  got.message_number == station.message_number &&
                  got.reference_station_id == station.reference_station_id &&
                  got.itrf_realization_year == station.itrf_realization_year &&
                  got.gps_indicator == station.gps_indicator &&
                  got.ecef_x == station.ecef_x && got.ecef_y == station.ecef_y &&
                  got.ecef_z == station.ecef_z &&
                  got.has_antenna_height == station.has_antenna_height &&
                  got.antenna_height == station.antenna_height,
              "rtcm station construct fields");
        sidereon_rtcm_messages_free(decoded);
    }

    /* 1008 antenna descriptor (descriptor + serial). */
    const uint16_t antenna_message = 1008;
    const char *antenna_descriptor = "TRM59800.00";
    built = NULL;
    check(sidereon_rtcm_build_antenna_descriptor(antenna_message, 2003, 1, antenna_descriptor,
                                                 "1440812345", NULL, NULL, NULL, &built) ==
              SIDEREON_STATUS_OK,
          "rtcm_build_antenna_descriptor");
    decoded = roundtrip(built, "rtcm antenna construct round-trip");
    if (decoded) {
        SidereonRtcmAntennaDescriptor got;
        check(sidereon_rtcm_message_antenna_descriptor(decoded, 0, &got) == SIDEREON_STATUS_OK &&
                  got.message_number == antenna_message && got.has_antenna_serial_number &&
                  !got.has_receiver_type,
              "rtcm antenna construct fields");
        char descriptor[64];
        size_t written = 0, required = 0;
        check(sidereon_rtcm_message_antenna_string(
                  decoded, 0, SIDEREON_RTCM_ANTENNA_STRING_FIELD_ANTENNA_DESCRIPTOR,
                  (uint8_t *)descriptor, sizeof(descriptor), &written, &required) ==
                  SIDEREON_STATUS_OK &&
                  required == strlen(antenna_descriptor),
              "rtcm antenna construct descriptor string");
        descriptor[written] = '\0';
        check(strcmp(descriptor, antenna_descriptor) == 0,
              "rtcm antenna construct descriptor value");
        sidereon_rtcm_messages_free(decoded);
    }

    /* 1019 GPS ephemeris. */
    SidereonRtcmGpsEphemeris gps;
    memset(&gps, 0, sizeof(gps));
    gps.satellite_id = 8;
    gps.week_number = 123;
    gps.a_f0 = 12345;
    gps.t_oe = 7200;
    gps.sqrt_a = UINT64_C(2702336448);
    built = NULL;
    check(sidereon_rtcm_build_gps_ephemeris(&gps, &built) == SIDEREON_STATUS_OK,
          "rtcm_build_gps_ephemeris");
    decoded = roundtrip(built, "rtcm gps ephemeris construct round-trip");
    if (decoded) {
        SidereonRtcmGpsEphemeris got;
        check(sidereon_rtcm_message_gps_ephemeris(decoded, 0, &got) == SIDEREON_STATUS_OK &&
                  got.satellite_id == gps.satellite_id && got.week_number == gps.week_number &&
                  got.a_f0 == gps.a_f0 && got.t_oe == gps.t_oe && got.sqrt_a == gps.sqrt_a,
              "rtcm gps ephemeris construct fields");
        sidereon_rtcm_messages_free(decoded);
    }

    /* 1020 GLONASS ephemeris. */
    SidereonRtcmGlonassEphemeris glo;
    memset(&glo, 0, sizeof(glo));
    glo.satellite_id = 5;
    glo.frequency_channel = 8;
    glo.m_n_t = 700;
    glo.t_b = 30;
    built = NULL;
    check(sidereon_rtcm_build_glonass_ephemeris(&glo, &built) == SIDEREON_STATUS_OK,
          "rtcm_build_glonass_ephemeris");
    decoded = roundtrip(built, "rtcm glonass ephemeris construct round-trip");
    if (decoded) {
        SidereonRtcmGlonassEphemeris got;
        check(sidereon_rtcm_message_glonass_ephemeris(decoded, 0, &got) == SIDEREON_STATUS_OK &&
                  got.satellite_id == glo.satellite_id &&
                  got.frequency_channel == glo.frequency_channel && got.m_n_t == glo.m_n_t &&
                  got.t_b == glo.t_b,
              "rtcm glonass ephemeris construct fields");
        sidereon_rtcm_messages_free(decoded);
    }

    /* 1042 BeiDou ephemeris. */
    SidereonRtcmBeidouEphemeris bds;
    memset(&bds, 0, sizeof(bds));
    bds.satellite_id = 19;
    bds.week_number = 902;
    bds.aode = 17;
    bds.t_oc = 12000;
    bds.a_f1 = 12345;
    bds.a_f0 = -45678;
    bds.sqrt_a = UINT64_C(2852448983);
    bds.t_oe = 12000;
    built = NULL;
    check(sidereon_rtcm_build_beidou_ephemeris(&bds, &built) == SIDEREON_STATUS_OK,
          "rtcm_build_beidou_ephemeris");
    decoded = roundtrip(built, "rtcm beidou ephemeris construct round-trip");
    if (decoded) {
        SidereonRtcmBeidouEphemeris got;
        check(sidereon_rtcm_message_beidou_ephemeris(decoded, 0, &got) == SIDEREON_STATUS_OK &&
                  got.satellite_id == bds.satellite_id && got.week_number == bds.week_number &&
                  got.aode == bds.aode && got.t_oc == bds.t_oc && got.a_f1 == bds.a_f1 &&
                  got.a_f0 == bds.a_f0 && got.sqrt_a == bds.sqrt_a && got.t_oe == bds.t_oe,
              "rtcm beidou ephemeris construct fields");
        sidereon_rtcm_messages_free(decoded);
    }

    /* 1044 QZSS ephemeris. */
    SidereonRtcmQzssEphemeris qzs;
    memset(&qzs, 0, sizeof(qzs));
    qzs.satellite_id = 3;
    qzs.week_number = 123;
    qzs.iode = 11;
    qzs.t_oc = 7200;
    qzs.a_f0 = 23456;
    qzs.sqrt_a = UINT64_C(2702336448);
    qzs.t_oe = 3600;
    qzs.codes_on_l2 = 1;
    built = NULL;
    check(sidereon_rtcm_build_qzss_ephemeris(&qzs, &built) == SIDEREON_STATUS_OK,
          "rtcm_build_qzss_ephemeris");
    decoded = roundtrip(built, "rtcm qzss ephemeris construct round-trip");
    if (decoded) {
        SidereonRtcmQzssEphemeris got;
        check(sidereon_rtcm_message_qzss_ephemeris(decoded, 0, &got) == SIDEREON_STATUS_OK &&
                  got.satellite_id == qzs.satellite_id && got.week_number == qzs.week_number &&
                  got.iode == qzs.iode && got.t_oc == qzs.t_oc && got.a_f0 == qzs.a_f0 &&
                  got.codes_on_l2 == qzs.codes_on_l2 && got.sqrt_a == qzs.sqrt_a &&
                  got.t_oe == qzs.t_oe,
              "rtcm qzss ephemeris construct fields");
        sidereon_rtcm_messages_free(decoded);
    }

    /* 1045 Galileo F/NAV ephemeris. */
    SidereonRtcmGalileoFnavEphemeris gal_fnav;
    memset(&gal_fnav, 0, sizeof(gal_fnav));
    gal_fnav.satellite_id = 12;
    gal_fnav.week_number = 1402;
    gal_fnav.iod_nav = 7;
    gal_fnav.sisa = 42;
    gal_fnav.t_oc = 5150;
    gal_fnav.a_f1 = -151;
    gal_fnav.a_f0 = -471483;
    gal_fnav.sqrt_a = UINT64_C(2852448983);
    gal_fnav.t_oe = 5150;
    built = NULL;
    check(sidereon_rtcm_build_galileo_fnav_ephemeris(&gal_fnav, &built) ==
              SIDEREON_STATUS_OK,
          "rtcm_build_galileo_fnav_ephemeris");
    decoded = roundtrip(built, "rtcm galileo fnav ephemeris construct round-trip");
    if (decoded) {
        SidereonRtcmGalileoFnavEphemeris got;
        check(sidereon_rtcm_message_galileo_fnav_ephemeris(decoded, 0, &got) ==
                      SIDEREON_STATUS_OK &&
                  got.satellite_id == gal_fnav.satellite_id &&
                  got.week_number == gal_fnav.week_number && got.iod_nav == gal_fnav.iod_nav &&
                  got.sisa == gal_fnav.sisa && got.t_oc == gal_fnav.t_oc &&
                  got.a_f1 == gal_fnav.a_f1 && got.a_f0 == gal_fnav.a_f0 &&
                  got.sqrt_a == gal_fnav.sqrt_a && got.t_oe == gal_fnav.t_oe,
              "rtcm galileo fnav ephemeris construct fields");
        sidereon_rtcm_messages_free(decoded);
    }

    /* 1046 Galileo I/NAV ephemeris. */
    SidereonRtcmGalileoInavEphemeris gal_inav;
    memset(&gal_inav, 0, sizeof(gal_inav));
    gal_inav.satellite_id = 3;
    gal_inav.week_number = 1402;
    gal_inav.iod_nav = 7;
    gal_inav.sisa_index = 107;
    gal_inav.t_oc = 5150;
    gal_inav.a_f1 = -151;
    gal_inav.a_f0 = -471483;
    gal_inav.sqrt_a = UINT64_C(2852448983);
    gal_inav.t_oe = 5150;
    gal_inav.bgd_e5a_e1 = 5;
    gal_inav.bgd_e5b_e1 = 7;
    built = NULL;
    check(sidereon_rtcm_build_galileo_inav_ephemeris(&gal_inav, &built) ==
              SIDEREON_STATUS_OK,
          "rtcm_build_galileo_inav_ephemeris");
    decoded = roundtrip(built, "rtcm galileo inav ephemeris construct round-trip");
    if (decoded) {
        SidereonRtcmGalileoInavEphemeris got;
        check(sidereon_rtcm_message_galileo_inav_ephemeris(decoded, 0, &got) ==
                      SIDEREON_STATUS_OK &&
                  got.satellite_id == gal_inav.satellite_id &&
                  got.week_number == gal_inav.week_number && got.iod_nav == gal_inav.iod_nav &&
                  got.sisa_index == gal_inav.sisa_index && got.t_oc == gal_inav.t_oc &&
                  got.a_f1 == gal_inav.a_f1 && got.a_f0 == gal_inav.a_f0 &&
                  got.sqrt_a == gal_inav.sqrt_a && got.t_oe == gal_inav.t_oe &&
                  got.bgd_e5a_e1 == gal_inav.bgd_e5a_e1 &&
                  got.bgd_e5b_e1 == gal_inav.bgd_e5b_e1,
              "rtcm galileo inav ephemeris construct fields");
        sidereon_rtcm_messages_free(decoded);
    }

    /* Real captured 1046 Galileo I/NAV frame. */
    const uint8_t *real_1046 = W3M_REAL_1046;
    const size_t real_1046_len = sizeof(W3M_REAL_1046);
    decoded = NULL;
    check(sidereon_rtcm_decode_messages(real_1046, real_1046_len, &decoded) ==
              SIDEREON_STATUS_OK,
          "rtcm real galileo inav decode");
    if (decoded) {
        SidereonRtcmMessageKind kind;
        uint16_t message_number = 0;
        check(sidereon_rtcm_message_kind(decoded, 0, &kind, &message_number) ==
                      SIDEREON_STATUS_OK &&
                  kind == W3M_REAL_1046_KIND &&
                  message_number == W3M_REAL_1046_MESSAGE_NUMBER,
              "rtcm real galileo inav kind");
        SidereonRtcmGalileoInavEphemeris got;
        check(sidereon_rtcm_message_galileo_inav_ephemeris(decoded, 0, &got) ==
                      SIDEREON_STATUS_OK &&
                  got.satellite_id == W3M_REAL_1046_SATELLITE_ID &&
                  got.week_number == W3M_REAL_1046_WEEK_NUMBER &&
                  got.iod_nav == W3M_REAL_1046_IOD_NAV &&
                  got.sqrt_a == W3M_REAL_1046_SQRT_A &&
                  got.eccentricity == W3M_REAL_1046_ECCENTRICITY,
              "rtcm real galileo inav fields");
        uint8_t frame[96];
        size_t written = 0, required = 0;
        check(sidereon_rtcm_message_to_frame(decoded, 0, frame, sizeof(frame), &written,
                                             &required) == SIDEREON_STATUS_OK &&
                  written == real_1046_len && required == real_1046_len &&
                  memcmp(frame, real_1046, real_1046_len) == 0,
              "rtcm real galileo inav frame round-trip");
        sidereon_rtcm_messages_free(decoded);
    }

    /* 1077 GPS MSM7: one satellite, one signal cell. */
    SidereonRtcmMsmInfo info;
    memset(&info, 0, sizeof(info));
    info.message_number = 1077;
    info.system = SIDEREON_GNSS_SYSTEM_GPS;
    info.kind = SIDEREON_RTCM_MSM_KIND_MSM7;
    info.header.reference_station_id = 2003;
    info.header.epoch_time = 100000;
    SidereonRtcmMsmSatellite sat;
    memset(&sat, 0, sizeof(sat));
    sat.id = 8;
    sat.has_rough_range_ms = true;
    sat.rough_range_ms = 75;
    sat.rough_range_mod1 = 512;
    sat.has_extended_info = true;
    sat.extended_info = 3;
    sat.has_rough_phase_range_rate = true;
    sat.rough_phase_range_rate_m_s = -100;
    SidereonRtcmMsmSignal sig;
    memset(&sig, 0, sizeof(sig));
    sig.satellite_id = 8;
    sig.signal_id = 2;
    /* MSM7 carries every signal field. */
    sig.has_fine_pseudorange = true;
    sig.fine_pseudorange = 1234;
    sig.has_fine_phase_range = true;
    sig.fine_phase_range = -5678;
    sig.has_lock_time_indicator = true;
    sig.lock_time_indicator = 200;
    sig.has_half_cycle_ambiguity = true;
    sig.half_cycle_ambiguity = false;
    sig.has_cnr = true;
    sig.cnr = 720;
    sig.has_fine_phase_range_rate = true;
    sig.fine_phase_range_rate = 42;
    built = NULL;
    check(sidereon_rtcm_build_msm(&info, &sat, 1, &sig, 1, &built) == SIDEREON_STATUS_OK,
          "rtcm_build_msm");
    decoded = roundtrip(built, "rtcm msm construct round-trip");
    if (decoded) {
        SidereonRtcmMsmInfo got;
        check(sidereon_rtcm_message_msm_info(decoded, 0, &got) == SIDEREON_STATUS_OK &&
                  got.message_number == info.message_number && got.system == info.system &&
                  got.kind == info.kind &&
                  got.header.reference_station_id == info.header.reference_station_id &&
                  got.header.epoch_time == info.header.epoch_time && got.satellite_count == 1 &&
                  got.signal_count == 1,
              "rtcm msm construct info");
        SidereonRtcmMsmSatellite got_sat[2];
        size_t written = 0, required = 0;
        check(sidereon_rtcm_message_msm_satellites(decoded, 0, got_sat, 2, &written, &required) ==
                  SIDEREON_STATUS_OK &&
                  written == 1 && got_sat[0].id == sat.id &&
                  got_sat[0].has_rough_range_ms == sat.has_rough_range_ms &&
                  got_sat[0].rough_range_ms == sat.rough_range_ms &&
                  got_sat[0].rough_range_mod1 == sat.rough_range_mod1 &&
                  got_sat[0].has_extended_info == sat.has_extended_info &&
                  got_sat[0].extended_info == sat.extended_info &&
                  got_sat[0].rough_phase_range_rate_m_s == sat.rough_phase_range_rate_m_s,
              "rtcm msm construct satellites");
        SidereonRtcmMsmSignal got_sig[2];
        written = 0;
        required = 0;
        check(sidereon_rtcm_message_msm_signals(decoded, 0, got_sig, 2, &written, &required) ==
                  SIDEREON_STATUS_OK &&
                  written == 1 && got_sig[0].signal_id == sig.signal_id &&
                  got_sig[0].has_fine_pseudorange == sig.has_fine_pseudorange &&
                  got_sig[0].fine_pseudorange == sig.fine_pseudorange &&
                  got_sig[0].has_fine_phase_range == sig.has_fine_phase_range &&
                  got_sig[0].fine_phase_range == sig.fine_phase_range &&
                  got_sig[0].has_lock_time_indicator == sig.has_lock_time_indicator &&
                  got_sig[0].lock_time_indicator == sig.lock_time_indicator &&
                  got_sig[0].has_half_cycle_ambiguity == sig.has_half_cycle_ambiguity &&
                  got_sig[0].half_cycle_ambiguity == sig.half_cycle_ambiguity &&
                  got_sig[0].has_cnr == sig.has_cnr &&
                  got_sig[0].cnr == sig.cnr &&
                  got_sig[0].has_fine_phase_range_rate == sig.has_fine_phase_range_rate &&
                  got_sig[0].fine_phase_range_rate == sig.fine_phase_range_rate,
              "rtcm msm construct signals");
        sidereon_rtcm_messages_free(decoded);
    }
}

int main(int argc, char **argv) {
    test_nequick_slant();
    test_raim_fde();
    test_rtk_arc();
    test_rtk_dual_arc_drivers();
    test_rtcm_construct();
    if (argc > 1) {
        test_ppp_auto_init(argv[1]);
    } else {
        check(0, "merged_smoke requires the PPP SP3 fixture path as argv[1]");
    }

    if (failures != 0) {
        fprintf(stderr, "merged_smoke: %d failure(s)\n", failures);
        return 1;
    }
    printf("merged_smoke: all checks passed\n");
    return 0;
}
