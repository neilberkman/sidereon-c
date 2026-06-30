/*
 * Focused C smoke for the SPP robustness + integrity surface that brings the C
 * binding to parity with the Elixir interface:
 *
 *   1. Fault detection and exclusion (sidereon_fde_solve_broadcast): a clean
 *      set converges with zero exclusions, and a deliberately corrupted
 *      satellite is detected and excluded, leaving it out of the surviving
 *      solution's used set.
 *   2. A robust (Huber/IRLS) solve via sidereon_solve_spp_v2 reports the
 *      reweighting it performed.
 *   3. A coarse-search cold start via the same V2 policy converges from a
 *      deliberately poor initial guess.
 *   4. The precise-with-broadcast fallback picks broadcast when no precise
 *      product covers the epoch (an empty product set and a wrong-epoch SP3).
 *
 * Every solve delegates to the engine; this program only marshals inputs and
 * asserts the surfaced provenance. Build/run is driven by tests/run_smoke.sh,
 * which passes the GRG SP3, the broadcast NAV, and a wrong-epoch SP3 as argv.
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

static int fail(const char *what, int code) {
    char message[512];
    size_t written = sidereon_last_error_message(message, sizeof(message));
    if (written > 0) {
        fprintf(stderr, "FAIL: %s: %s\n", what, message);
    } else {
        fprintf(stderr, "FAIL: %s\n", what);
    }
    return code;
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

static bool token_equals(const SidereonSatelliteToken *token, const char *expected) {
    return strncmp(token->bytes, expected, sizeof(token->bytes)) == 0;
}

/* True when the solution's used-satellite set contains the given token. */
static bool used_contains(const SidereonSppSolution *sol, const char *token) {
    size_t count = 0;
    if (sidereon_spp_solution_used_sat_count(sol, &count) != SIDEREON_STATUS_OK) {
        return false;
    }
    if (count == 0) {
        return false;
    }
    SidereonSatelliteToken *ids = calloc(count, sizeof(*ids));
    if (ids == NULL) {
        return false;
    }
    size_t written = 0;
    size_t required = 0;
    bool found = false;
    if (sidereon_spp_solution_used_sat_ids(sol, ids, count, &written, &required) ==
        SIDEREON_STATUS_OK) {
        for (size_t i = 0; i < written; i++) {
            if (token_equals(&ids[i], token)) {
                found = true;
                break;
            }
        }
    }
    free(ids);
    return found;
}

static void fill_broadcast_inputs(SidereonObservation *obs, SidereonSppInputs *inputs) {
    for (size_t i = 0; i < BC_OBS_COUNT; i++) {
        obs[i].sat_id = BC_SAT_IDS[i];
        obs[i].pseudorange_m = bits_to_f64(BC_PSEUDORANGE_BITS[i]);
    }
    inputs->observations = obs;
    inputs->observation_count = BC_OBS_COUNT;
    inputs->t_rx_j2000_s = bits_to_f64(BC_T_RX_J2000_S_BITS);
    inputs->t_rx_second_of_day_s = bits_to_f64(BC_T_RX_SOD_S_BITS);
    inputs->day_of_year = bits_to_f64(BC_DOY_BITS);
    for (int i = 0; i < 4; i++) {
        inputs->initial_guess[i] = bits_to_f64(BC_INITIAL_GUESS_BITS[i]);
        inputs->klobuchar_alpha[i] = 0.0;
        inputs->klobuchar_beta[i] = 0.0;
    }
    inputs->ionosphere = false;
    inputs->troposphere = true;
    inputs->pressure_hpa = bits_to_f64(BC_PRESSURE_HPA_BITS);
    inputs->temperature_k = bits_to_f64(BC_TEMPERATURE_K_BITS);
    inputs->relative_humidity = bits_to_f64(BC_RELATIVE_HUMIDITY_BITS);
    inputs->with_geodetic = true;
}

/* (1) FDE over broadcast: clean set excludes nothing, corrupted satellite is
 * detected and excluded. */
static int exercise_fde(SidereonBroadcastEphemeris *broadcast) {
    SidereonObservation obs[BC_OBS_COUNT];
    SidereonSppInputs inputs;
    fill_broadcast_inputs(obs, &inputs);

    SidereonFdeOptions options;
    if (sidereon_fde_options_init(&options) != SIDEREON_STATUS_OK) {
        return fail("fde: sidereon_fde_options_init", 1);
    }
    options.p_fa = 1.0e-3;
    options.unit_weights = true;
    options.max_iterations = BC_OBS_COUNT; /* generous exclusion budget */

    /* Clean run: the set is self-consistent, so RAIM excludes nothing. */
    SidereonFdeSolution *clean = NULL;
    if (sidereon_fde_solve_broadcast(broadcast, &inputs, &options, &clean) != SIDEREON_STATUS_OK) {
        return fail("fde: clean solve", 1);
    }
    size_t clean_iters = 99;
    size_t clean_written = 99;
    size_t clean_required = 99;
    if (sidereon_fde_solution_iterations(clean, &clean_iters) != SIDEREON_STATUS_OK ||
        clean_iters != 0 ||
        sidereon_fde_solution_excluded_sats(clean, NULL, 0, &clean_written, &clean_required) !=
            SIDEREON_STATUS_OK ||
        clean_written != 0 || clean_required != 0) {
        sidereon_fde_solution_free(clean);
        return fail("fde: clean run excluded a satellite", 1);
    }
    SidereonSppSolution *clean_sol = NULL;
    if (sidereon_fde_solution_solution(clean, &clean_sol) != SIDEREON_STATUS_OK) {
        sidereon_fde_solution_free(clean);
        return fail("fde: clean surviving solution", 1);
    }
    size_t clean_used = 0;
    if (sidereon_spp_solution_used_sat_count(clean_sol, &clean_used) != SIDEREON_STATUS_OK ||
        clean_used < 5) {
        sidereon_spp_solution_free(clean_sol);
        sidereon_fde_solution_free(clean);
        return fail("fde: clean used-sat count", 1);
    }
    /* Pick a satellite that the clean solve actually uses (some are dropped by
     * the elevation mask), so corrupting it lands in the RAIM residual set. */
    const char *bad_sat = NULL;
    size_t bad_index = 0;
    for (size_t i = 0; i < BC_OBS_COUNT; i++) {
        if (used_contains(clean_sol, BC_SAT_IDS[i])) {
            bad_sat = BC_SAT_IDS[i];
            bad_index = i;
            break;
        }
    }
    sidereon_spp_solution_free(clean_sol);
    sidereon_fde_solution_free(clean);
    if (bad_sat == NULL) {
        return fail("fde: no used satellite to corrupt", 1);
    }

    /* Corrupted run: bias the chosen used satellite's pseudorange by 200 m. The
     * clean set is self-consistent (RAIM passes with zero exclusions above), so the
     * blunder is detected and the corrupted satellite is excluded. A single
     * least-squares pass can spread one bias across several residuals (RAIM
     * swamping), so the loop may drop another satellite as well; the test asserts
     * the corrupted satellite is among the excluded and is gone from the surviving
     * used set, not that it is dropped first. A gross blunder would mask the
     * outlier entirely, so a moderate bias is used. */
    SidereonObservation corrupt_obs[BC_OBS_COUNT];
    memcpy(corrupt_obs, obs, sizeof(obs));
    corrupt_obs[bad_index].pseudorange_m += 200.0;
    SidereonSppInputs corrupt_inputs = inputs;
    corrupt_inputs.observations = corrupt_obs;

    SidereonFdeSolution *fixed = NULL;
    if (sidereon_fde_solve_broadcast(broadcast, &corrupt_inputs, &options, &fixed) !=
        SIDEREON_STATUS_OK) {
        return fail("fde: corrupted solve", 1);
    }
    size_t iters = 0;
    if (sidereon_fde_solution_iterations(fixed, &iters) != SIDEREON_STATUS_OK || iters < 1) {
        sidereon_fde_solution_free(fixed);
        return fail("fde: corrupted run performed no exclusion", 1);
    }
    /* Read the excluded tokens via the query-then-fill contract. */
    size_t written = 0;
    size_t required = 0;
    if (sidereon_fde_solution_excluded_sats(fixed, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        required < 1 || written != 0) {
        sidereon_fde_solution_free(fixed);
        return fail("fde: excluded count query", 1);
    }
    SidereonSatelliteToken *excluded = calloc(required, sizeof(*excluded));
    if (excluded == NULL) {
        sidereon_fde_solution_free(fixed);
        return fail("fde: excluded alloc", 1);
    }
    if (sidereon_fde_solution_excluded_sats(fixed, excluded, required, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != required) {
        free(excluded);
        sidereon_fde_solution_free(fixed);
        return fail("fde: excluded fill", 1);
    }
    bool excluded_bad = false;
    for (size_t i = 0; i < written; i++) {
        if (token_equals(&excluded[i], bad_sat)) {
            excluded_bad = true;
            break;
        }
    }
    free(excluded);
    if (!excluded_bad) {
        sidereon_fde_solution_free(fixed);
        return fail("fde: corrupted satellite not excluded", 1);
    }

    SidereonSppSolution *survivor = NULL;
    if (sidereon_fde_solution_solution(fixed, &survivor) != SIDEREON_STATUS_OK) {
        sidereon_fde_solution_free(fixed);
        return fail("fde: surviving solution", 1);
    }
    if (used_contains(survivor, bad_sat)) {
        sidereon_spp_solution_free(survivor);
        sidereon_fde_solution_free(fixed);
        return fail("fde: excluded satellite still in surviving used set", 1);
    }
    size_t survivor_used = 0;
    if (sidereon_spp_solution_used_sat_count(survivor, &survivor_used) != SIDEREON_STATUS_OK ||
        survivor_used < 4) {
        sidereon_spp_solution_free(survivor);
        sidereon_fde_solution_free(fixed);
        return fail("fde: surviving used-sat count", 1);
    }
    sidereon_spp_solution_free(survivor);
    sidereon_fde_solution_free(fixed);

    printf("fde: clean iterations = 0, corrupted excluded %s after %zu iteration(s)\n", bad_sat,
           iters);
    return 0;
}

/* Fill the GRG L0_minimal SPP inputs (geometry + clock + Sagnac only, no iono,
 * no tropo), the reference configuration whose converged answer is the frozen
 * SPP_EXPECTED_X. */
static void fill_grg_inputs(SidereonObservation *observations, SidereonSppInputs *base) {
    for (size_t i = 0; i < SPP_OBS_COUNT; i++) {
        observations[i].sat_id = SPP_SAT_IDS[i];
        observations[i].pseudorange_m = bits_to_f64(SPP_PSEUDORANGE_BITS[i]);
    }
    base->observations = observations;
    base->observation_count = SPP_OBS_COUNT;
    base->t_rx_j2000_s = bits_to_f64(SPP_T_RX_J2000_S_BITS);
    base->t_rx_second_of_day_s = bits_to_f64(SPP_T_RX_SOD_S_BITS);
    base->day_of_year = bits_to_f64(SPP_DOY_BITS);
    for (int i = 0; i < 4; i++) {
        base->initial_guess[i] = bits_to_f64(SPP_INITIAL_GUESS_BITS[i]);
        base->klobuchar_alpha[i] = 0.0;
        base->klobuchar_beta[i] = 0.0;
    }
    base->ionosphere = false;
    base->troposphere = false;
    base->pressure_hpa = bits_to_f64(SPP_PRESSURE_HPA_BITS);
    base->temperature_k = bits_to_f64(SPP_TEMPERATURE_K_BITS);
    base->relative_humidity = bits_to_f64(SPP_RELATIVE_HUMIDITY_BITS);
    base->with_geodetic = true;
}

/* (2) Robust Huber/IRLS reweighting and (3) coarse-search cold start, each a
 * separate V2 solve against the GRG SP3. The engine composes robust and coarse
 * as independent controls, but they answer different needs, so they are exercised
 * apart (the Elixir interface treats them as mutually exclusive). */
static int exercise_robust_and_coarse(const SidereonSp3 *sp3) {
    /* (2) Robust: a good initial guess, max_outer = 2 (the warm start plus one
     * reweighted solve), so the metadata reports outer_iterations == 1 and a
     * finite final robust scale. */
    SidereonObservation robust_obs[SPP_OBS_COUNT];
    SidereonSppInputsV2 robust_inputs;
    if (sidereon_spp_inputs_v2_init(&robust_inputs) != SIDEREON_STATUS_OK) {
        return fail("robust: sidereon_spp_inputs_v2_init", 1);
    }
    fill_grg_inputs(robust_obs, &robust_inputs.base);
    robust_inputs.robust_enabled = true;
    robust_inputs.robust.max_outer = 2;

    SidereonSppSolution *robust_sol = NULL;
    if (sidereon_solve_spp_v2(sp3, &robust_inputs, &robust_sol) != SIDEREON_STATUS_OK) {
        return fail("robust: sidereon_solve_spp_v2", 1);
    }
    SidereonSppMetadata metadata;
    if (sidereon_spp_solution_metadata(robust_sol, &metadata) != SIDEREON_STATUS_OK ||
        metadata.outer_iterations != 1 || !metadata.has_final_robust_scale_m ||
        !isfinite(metadata.final_robust_scale_m)) {
        sidereon_spp_solution_free(robust_sol);
        return fail("robust: reweighting not reported", 1);
    }
    sidereon_spp_solution_free(robust_sol);

    /* (3) Coarse search: wipe the initial guess to the geocenter so the warm start
     * is useless, and require the near-surface coarse seeds to find the basin.
     * Success is recovering the frozen reference position, proving the cold start
     * converged to the true basin rather than a wrong local minimum. */
    SidereonObservation coarse_obs[SPP_OBS_COUNT];
    SidereonSppInputsV2 coarse_inputs;
    if (sidereon_spp_inputs_v2_init(&coarse_inputs) != SIDEREON_STATUS_OK) {
        return fail("coarse: sidereon_spp_inputs_v2_init", 1);
    }
    fill_grg_inputs(coarse_obs, &coarse_inputs.base);
    coarse_inputs.policy.coarse_search_enabled = true;
    coarse_inputs.policy.coarse_search_seeds = 24;
    for (int i = 0; i < 4; i++) {
        coarse_inputs.base.initial_guess[i] = 0.0;
    }

    SidereonSppSolution *coarse_sol = NULL;
    if (sidereon_solve_spp_v2(sp3, &coarse_inputs, &coarse_sol) != SIDEREON_STATUS_OK) {
        return fail("coarse: sidereon_solve_spp_v2", 1);
    }
    double position[3];
    if (sidereon_spp_solution_position(coarse_sol, position, 3) != SIDEREON_STATUS_OK) {
        sidereon_spp_solution_free(coarse_sol);
        return fail("coarse: position readout", 1);
    }
    double dx = position[0] - bits_to_f64(SPP_EXPECTED_X_BITS[0]);
    double dy = position[1] - bits_to_f64(SPP_EXPECTED_X_BITS[1]);
    double dz = position[2] - bits_to_f64(SPP_EXPECTED_X_BITS[2]);
    double dpos = sqrt(dx * dx + dy * dy + dz * dz);
    if (!(dpos < 1.0e-3)) {
        sidereon_spp_solution_free(coarse_sol);
        return fail("coarse: cold start did not recover the reference position", 1);
    }
    sidereon_spp_solution_free(coarse_sol);

    printf("robust: outer_iterations = %zu, final_robust_scale = %.6f m\n",
           metadata.outer_iterations, metadata.final_robust_scale_m);
    printf("coarse: cold start recovered reference position (dpos = %.3e m)\n", dpos);
    return 0;
}

/* (4) Fallback picks broadcast when no precise product covers the epoch: an empty
 * product set, then a wrong-epoch SP3. */
static int exercise_fallback_broadcast(SidereonBroadcastEphemeris *broadcast,
                                       const char *wrong_epoch_sp3_path) {
    SidereonObservation obs[BC_OBS_COUNT];
    SidereonSppInputs inputs;
    fill_broadcast_inputs(obs, &inputs);
    SidereonStalenessPolicy policy = sidereon_staleness_policy_days(3.0);

    /* Empty precise set: nothing to select, so broadcast produces the fix. */
    SidereonSourcedSolution *fb_empty = NULL;
    if (sidereon_solve_with_fallback(NULL, 0, broadcast, &inputs, policy, &fb_empty) !=
        SIDEREON_FALLBACK_STATUS_OK) {
        return fail("fallback: empty-set solve", 1);
    }
    SidereonFixSourceKind kind = SIDEREON_FIX_SOURCE_KIND_PRECISE;
    if (sidereon_sourced_solution_source_kind(fb_empty, &kind) != SIDEREON_STATUS_OK ||
        kind != SIDEREON_FIX_SOURCE_KIND_BROADCAST) {
        sidereon_sourced_solution_free(fb_empty);
        return fail("fallback: empty set did not pick broadcast", 1);
    }
    sidereon_sourced_solution_free(fb_empty);

    /* Wrong-epoch precise product: covers a different day, so broadcast wins. */
    size_t len = 0;
    uint8_t *bytes = read_file(wrong_epoch_sp3_path, &len);
    if (bytes == NULL) {
        return fail("fallback: read wrong-epoch SP3", 1);
    }
    SidereonSp3 *wrong = NULL;
    if (sidereon_sp3_load(bytes, len, &wrong) != SIDEREON_STATUS_OK) {
        free(bytes);
        return fail("fallback: load wrong-epoch SP3", 1);
    }
    free(bytes);

    const SidereonSp3 *wrong_set[1] = {wrong};
    SidereonSourcedSolution *fb_wrong = NULL;
    if (sidereon_solve_with_fallback(wrong_set, 1, broadcast, &inputs, policy, &fb_wrong) !=
        SIDEREON_FALLBACK_STATUS_OK) {
        sidereon_sp3_free(wrong);
        return fail("fallback: wrong-epoch solve", 1);
    }
    kind = SIDEREON_FIX_SOURCE_KIND_PRECISE;
    if (sidereon_sourced_solution_source_kind(fb_wrong, &kind) != SIDEREON_STATUS_OK ||
        kind != SIDEREON_FIX_SOURCE_KIND_BROADCAST) {
        sidereon_sourced_solution_free(fb_wrong);
        sidereon_sp3_free(wrong);
        return fail("fallback: wrong epoch did not pick broadcast", 1);
    }
    sidereon_sourced_solution_free(fb_wrong);
    sidereon_sp3_free(wrong);

    printf("fallback: empty set and wrong-epoch SP3 both selected broadcast\n");
    return 0;
}

int main(int argc, char **argv) {
    if (argc < 4) {
        fprintf(stderr, "usage: %s <grg_sp3> <broadcast_nav> <wrong_epoch_sp3>\n", argv[0]);
        return 2;
    }
    const char *sp3_path = argv[1];
    const char *nav_path = argv[2];
    const char *wrong_epoch_sp3_path = argv[3];

    /* Load the GRG SP3 for the robust + coarse-search exercises. */
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

    /* Parse the broadcast NAV for the FDE + fallback exercises. */
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
        rc = exercise_fde(broadcast);
    }
    if (rc == 0) {
        rc = exercise_robust_and_coarse(sp3);
    }
    if (rc == 0) {
        rc = exercise_fallback_broadcast(broadcast, wrong_epoch_sp3_path);
    }

    sidereon_broadcast_ephemeris_free(broadcast);
    sidereon_sp3_free(sp3);

    if (rc == 0) {
        printf("robustness smoke: OK\n");
    }
    return rc;
}
