/*
 * Real-data RINEX RTK smoke test for the C binding.
 *
 * argv: <sp3> <wtzr_obs> <wtzz_obs>
 */
#include <math.h>
#include <stdbool.h>
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

static uint8_t *read_file(const char *path, size_t *out_len) {
    FILE *f = fopen(path, "rb");
    if (!f) {
        fprintf(stderr, "FAIL: cannot open %s\n", path);
        failures++;
        return NULL;
    }
    if (fseek(f, 0, SEEK_END) != 0) {
        fclose(f);
        failures++;
        return NULL;
    }
    long size = ftell(f);
    if (size < 0) {
        fclose(f);
        failures++;
        return NULL;
    }
    rewind(f);
    uint8_t *buf = (uint8_t *)malloc((size_t)size);
    if (!buf) {
        fclose(f);
        failures++;
        return NULL;
    }
    size_t got = fread(buf, 1, (size_t)size, f);
    fclose(f);
    if (got != (size_t)size) {
        free(buf);
        failures++;
        return NULL;
    }
    *out_len = got;
    return buf;
}

static SidereonSp3 *load_sp3(const char *path) {
    size_t len = 0;
    uint8_t *bytes = read_file(path, &len);
    if (!bytes) {
        return NULL;
    }
    SidereonSp3 *sp3 = NULL;
    check(sidereon_sp3_load(bytes, len, &sp3) == SIDEREON_STATUS_OK && sp3 != NULL,
          "sp3 load");
    free(bytes);
    return sp3;
}

static SidereonRinexObs *load_obs(const char *path) {
    size_t len = 0;
    uint8_t *bytes = read_file(path, &len);
    if (!bytes) {
        return NULL;
    }
    SidereonRinexObs *obs = NULL;
    check(sidereon_rinex_obs_parse(bytes, len, &obs) == SIDEREON_STATUS_OK && obs != NULL,
          "rinex obs parse");
    free(bytes);
    return obs;
}

static double norm3(const double v[3]) {
    return sqrt(v[0] * v[0] + v[1] * v[1] + v[2] * v[2]);
}

static double vector_error(const double got[3], const double expected[3]) {
    double diff[3] = {
        got[0] - expected[0],
        got[1] - expected[1],
        got[2] - expected[2],
    };
    return norm3(diff);
}

static int arp_position(const double marker[3], const SidereonRinexObs *obs, double out[3]) {
    SidereonRinexObsHeader header;
    memset(&header, 0, sizeof(header));
    SidereonStatus status = sidereon_rinex_obs_header(obs, &header);
    check(status == SIDEREON_STATUS_OK, "rinex obs header");
    if (status != SIDEREON_STATUS_OK) {
        return 0;
    }
    check(header.has_antenna_delta_hen_m, "rinex antenna delta present");
    if (!header.has_antenna_delta_hen_m) {
        return 0;
    }
    int zero_east_north = fabs(header.antenna_delta_hen_m[1]) < 1.0e-12 &&
                          fabs(header.antenna_delta_hen_m[2]) < 1.0e-12;
    check(zero_east_north, "rinex antenna east/north offsets zero");
    if (!zero_east_north) {
        return 0;
    }

    double marker_norm = norm3(marker);
    check(marker_norm > 0.0, "marker norm nonzero");
    if (!(marker_norm > 0.0)) {
        return 0;
    }
    double height_m = header.antenna_delta_hen_m[0];
    out[0] = marker[0] + height_m * marker[0] / marker_norm;
    out[1] = marker[1] + height_m * marker[1] / marker_norm;
    out[2] = marker[2] + height_m * marker[2] / marker_norm;
    return 1;
}

static void set_wtzr_single_frequency_config(SidereonRtkRinexStaticBaselineConfig *config,
                                             const double base_arp[3]) {
    check(sidereon_rtk_rinex_static_baseline_config_init(config) == SIDEREON_STATUS_OK,
          "static rinex config init");
    memcpy(config->base_m, base_arp, sizeof(config->base_m));
    config->arc_options.has_max_epochs = true;
    config->arc_options.max_epochs = 120;
    config->arc_options.include_prediction_time = false;
    config->model.code_sigma_m = 2.0;
    config->model.phase_sigma_m = 0.01;
    config->model.sagnac = true;
    config->model.stochastic = SIDEREON_RTK_STOCHASTIC_MODEL_SIMPLE;
    config->model.elevation_weighting = true;
    config->preprocessing.has_cycle_slip = true;
    config->preprocessing.cycle_slip = SIDEREON_RTK_CYCLE_SLIP_POLICY_SPLIT_ARC;
}

static void set_wtzr_wide_lane_config(SidereonRtkRinexWideLaneFixedConfig *config,
                                      const double base_arp[3]) {
    check(sidereon_rtk_rinex_wide_lane_fixed_config_init(config) == SIDEREON_STATUS_OK,
          "wide lane rinex config init");
    memcpy(config->base_m, base_arp, sizeof(config->base_m));
    config->arc_options.has_max_epochs = true;
    config->arc_options.max_epochs = 120;
    config->arc_options.include_prediction_time = false;
    config->model.code_sigma_m = 2.0;
    config->model.phase_sigma_m = 0.01;
    config->model.sagnac = true;
    config->model.stochastic = SIDEREON_RTK_STOCHASTIC_MODEL_SIMPLE;
    config->model.elevation_weighting = true;
}

static void test_static_rinex_rtk(const SidereonSp3 *sp3, const SidereonRinexObs *base_obs,
                                  const SidereonRinexObs *rover_obs,
                                  const double base_arp[3], const double truth[3]) {
    SidereonRtkRinexStaticBaselineConfig config;
    set_wtzr_single_frequency_config(&config, base_arp);

    SidereonRtkStaticArcSolution *solution = NULL;
    check(sidereon_solve_static_rinex_rtk_baseline(sp3, base_obs, rover_obs, &config,
                                                   &solution) == SIDEREON_STATUS_OK &&
              solution != NULL,
          "solve static rinex rtk baseline");
    if (!solution) {
        return;
    }

    double float_baseline[3] = {0.0, 0.0, 0.0};
    double fixed_baseline[3] = {0.0, 0.0, 0.0};
    check(sidereon_rtk_static_arc_solution_float_baseline_ecef(solution, float_baseline, 3) ==
              SIDEREON_STATUS_OK,
          "static float baseline");
    check(sidereon_rtk_static_arc_solution_fixed_baseline_ecef(solution, fixed_baseline, 3) ==
              SIDEREON_STATUS_OK,
          "static fixed baseline");

    SidereonRtkFloatMetadata float_meta;
    SidereonRtkFixedMetadata fixed_meta;
    check(sidereon_rtk_static_arc_solution_float_metadata(solution, &float_meta) ==
              SIDEREON_STATUS_OK,
          "static float metadata");
    check(sidereon_rtk_static_arc_solution_fixed_metadata(solution, &fixed_meta) ==
              SIDEREON_STATUS_OK,
          "static fixed metadata");

    double float_err = vector_error(float_baseline, truth);
    double fixed_err = vector_error(fixed_baseline, truth);
    printf("static rinex rtk float baseline = %.9f %.9f %.9f, error = %.9f m\n",
           float_baseline[0], float_baseline[1], float_baseline[2], float_err);
    printf("static rinex rtk fixed baseline = %.9f %.9f %.9f, error = %.9f m\n",
           fixed_baseline[0], fixed_baseline[1], fixed_baseline[2], fixed_err);

    check(float_meta.ambiguity_count > 0 && float_meta.n_observations > 0,
          "static float metadata counts");
    check(fixed_meta.integer_status == SIDEREON_RTK_INTEGER_STATUS_NOT_FIXED,
          "static integer status not fixed");
    check(fixed_meta.has_integer_ratio && fixed_meta.integer_ratio < 3.0,
          "static ratio rejected");
    check(float_err < 0.08, "static float baseline within dm");
    check(fixed_err < 0.01, "static fixed reported baseline within cm");

    size_t written = 99;
    size_t required = 99;
    check(sidereon_rtk_static_arc_solution_split_cycle_slip_arcs(
              solution, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
              written == 0 && required == 4,
          "static split cycle slip arcs");

    written = 99;
    required = 99;
    check(sidereon_rtk_static_arc_solution_ambiguity_ids(solution, NULL, 0, &written,
                                                         &required) == SIDEREON_STATUS_OK &&
              written == 0 && required > 0,
          "static ambiguity id query");

    sidereon_rtk_static_arc_solution_free(solution);
}

static void test_wide_lane_rinex_rtk(const SidereonSp3 *sp3, const SidereonRinexObs *base_obs,
                                     const SidereonRinexObs *rover_obs,
                                     const double base_arp[3], const double truth[3]) {
    SidereonRtkRinexWideLaneFixedConfig config;
    set_wtzr_wide_lane_config(&config, base_arp);

    SidereonRtkWideLaneFixedRinexSolution *solution = NULL;
    check(sidereon_solve_wide_lane_fixed_rinex_rtk_baseline(
              sp3, base_obs, rover_obs, &config, &solution) == SIDEREON_STATUS_OK &&
              solution != NULL,
          "solve wide lane rinex rtk baseline");
    if (!solution) {
        return;
    }

    double float_baseline[3] = {0.0, 0.0, 0.0};
    double fixed_baseline[3] = {0.0, 0.0, 0.0};
    check(sidereon_rtk_wide_lane_fixed_rinex_solution_float_baseline_ecef(
              solution, float_baseline, 3) == SIDEREON_STATUS_OK,
          "wide lane float baseline");
    check(sidereon_rtk_wide_lane_fixed_rinex_solution_fixed_baseline_ecef(
              solution, fixed_baseline, 3) == SIDEREON_STATUS_OK,
          "wide lane fixed baseline");

    SidereonRtkFloatMetadata float_meta;
    SidereonRtkFixedMetadata fixed_meta;
    SidereonRtkWideLaneFixedRinexMetadata rinex_meta;
    check(sidereon_rtk_wide_lane_fixed_rinex_solution_float_metadata(solution, &float_meta) ==
              SIDEREON_STATUS_OK,
          "wide lane float metadata");
    check(sidereon_rtk_wide_lane_fixed_rinex_solution_fixed_metadata(solution, &fixed_meta) ==
              SIDEREON_STATUS_OK,
          "wide lane fixed metadata");
    check(sidereon_rtk_wide_lane_fixed_rinex_solution_metadata(solution, &rinex_meta) ==
              SIDEREON_STATUS_OK,
          "wide lane combined metadata");

    double float_err = vector_error(float_baseline, truth);
    double fixed_err = vector_error(fixed_baseline, truth);
    printf("wide lane rinex rtk float baseline = %.9f %.9f %.9f, error = %.9f m\n",
           float_baseline[0], float_baseline[1], float_baseline[2], float_err);
    printf("wide lane rinex rtk fixed baseline = %.9f %.9f %.9f, error = %.9f m\n",
           fixed_baseline[0], fixed_baseline[1], fixed_baseline[2], fixed_err);

    check(float_meta.ambiguity_count > 0 && float_meta.n_observations > 0,
          "wide lane float metadata counts");
    check(fixed_meta.integer_status == SIDEREON_RTK_INTEGER_STATUS_FIXED,
          "wide lane integer status fixed");
    check(fixed_meta.has_integer_ratio && fixed_meta.integer_ratio > 3.0,
          "wide lane ratio accepted");
    check(rinex_meta.wide_lane_fixed && rinex_meta.wide_lane_ambiguity_count > 0,
          "wide lane metadata fixed");
    check(float_err < 0.1, "wide lane float baseline within dm");
    check(fixed_err < 0.01, "wide lane fixed baseline within cm");

    size_t written = 99;
    size_t required = 99;
    check(sidereon_rtk_wide_lane_fixed_rinex_solution_wide_lane_cycles(
              solution, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
              written == 0 && required == rinex_meta.wide_lane_ambiguity_count &&
              required > 0,
          "wide lane cycle query");

    sidereon_rtk_wide_lane_fixed_rinex_solution_free(solution);
}

int main(int argc, char **argv) {
    if (argc != 4) {
        fprintf(stderr, "usage: %s <sp3> <wtzr_obs> <wtzz_obs>\n", argv[0]);
        return 2;
    }

    static const double WTZR_MARKER_M[3] = {
        4075580.3111,
        931854.0543,
        4801568.2808,
    };
    static const double WTZZ_MARKER_M[3] = {
        4075579.1913,
        931853.3696,
        4801569.1897,
    };

    SidereonSp3 *sp3 = load_sp3(argv[1]);
    SidereonRinexObs *base_obs = load_obs(argv[2]);
    SidereonRinexObs *rover_obs = load_obs(argv[3]);

    if (sp3 && base_obs && rover_obs) {
        double base_arp[3] = {0.0, 0.0, 0.0};
        double rover_arp[3] = {0.0, 0.0, 0.0};
        double truth[3] = {0.0, 0.0, 0.0};
        if (arp_position(WTZR_MARKER_M, base_obs, base_arp) &&
            arp_position(WTZZ_MARKER_M, rover_obs, rover_arp)) {
            truth[0] = rover_arp[0] - base_arp[0];
            truth[1] = rover_arp[1] - base_arp[1];
            truth[2] = rover_arp[2] - base_arp[2];
            printf("truth arp baseline = %.9f %.9f %.9f m\n", truth[0], truth[1], truth[2]);

            test_static_rinex_rtk(sp3, base_obs, rover_obs, base_arp, truth);
            test_wide_lane_rinex_rtk(sp3, base_obs, rover_obs, base_arp, truth);
        }
    }

    sidereon_rinex_obs_free(rover_obs);
    sidereon_rinex_obs_free(base_obs);
    sidereon_sp3_free(sp3);

    if (failures != 0) {
        fprintf(stderr, "rtk_rinex_smoke: %d failure(s)\n", failures);
        return 1;
    }
    return 0;
}
