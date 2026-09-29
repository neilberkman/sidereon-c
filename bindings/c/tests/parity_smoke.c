/*
 * Focused C smoke for the capability-parity additions that bring the C binding
 * level with the Elixir interface. Each exercise delegates to the engine; this
 * program only marshals inputs and asserts the surfaced structure:
 *
 *   1. SP3-backed geometry (sidereon_sp3_geometry_visible /
 *      _visibility_series / _passes).
 *   2. Predicted observables from SP3 and broadcast sources
 *      (sidereon_sp3_observables / sidereon_broadcast_observables) and the
 *      options initializer.
 *   3. Broadcast-source velocity (sidereon_solve_velocity_broadcast), reusing
 *      the shared SidereonVelocitySolution accessors.
 *   4. Reduced-orbit fit / evaluate / drift, including piecewise fit /
 *      evaluate / drift, fed real ECEF samples interpolated from the SP3
 *      product.
 *   5. NRLMSISE-00 neutral-atmosphere density (sidereon_atmosphere_nrlmsise00),
 *      including the out-of-domain rejection.
 *
 * Build/run is driven by tests/run_smoke.sh, which passes the GRG SP3 and the
 * broadcast NAV as argv.
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
#include "w5_parity_pins.h"

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

static uint64_t f64_to_bits(double value) {
    uint64_t bits;
    memcpy(&bits, &value, sizeof(bits));
    return bits;
}

/* Exact agreement with a pinned engine value (tests/valgen, bin w5_parity). */
static bool same_bits(double value, uint64_t want) {
    return f64_to_bits(value) == want;
}

static bool same_bits_n(const double *values, const uint64_t *want, size_t n) {
    for (size_t i = 0; i < n; i++) {
        if (!same_bits(values[i], want[i])) {
            return false;
        }
    }
    return true;
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

static bool token_nonempty(const SidereonSatelliteToken *token) {
    return token->bytes[0] != '\0';
}

/* (1) SP3-backed geometry: visible at one epoch, then visible-count series and
 * passes over a short window. The receiver is the frozen SPP reference position
 * and the epoch is the SPP receive time, both inside the GRG SP3 coverage. */
static int exercise_geometry(const SidereonSp3 *sp3) {
    double receiver[3] = {
        bits_to_f64(SPP_EXPECTED_X_BITS[0]),
        bits_to_f64(SPP_EXPECTED_X_BITS[1]),
        bits_to_f64(SPP_EXPECTED_X_BITS[2]),
    };
    double t_rx = bits_to_f64(SPP_T_RX_J2000_S_BITS);
    const double mask_deg = 10.0;

    /* visible: query count, then fill. */
    size_t written = 0;
    size_t required = 0;
    if (sidereon_sp3_geometry_visible(sp3, receiver, t_rx, mask_deg, NULL, 0, NULL, 0, &written,
                                      &required) != SIDEREON_STATUS_OK ||
        written != 0) {
        return fail("geometry: visible count query", 1);
    }
    if (required != W5_PARITY_VISIBLE_COUNT) {
        return fail("geometry: visible count differs from the engine's", 1);
    }
    SidereonGeometryVisible *rows = calloc(required, sizeof(*rows));
    if (rows == NULL) {
        return fail("geometry: visible alloc", 1);
    }
    if (sidereon_sp3_geometry_visible(sp3, receiver, t_rx, mask_deg, NULL, 0, rows, required,
                                      &written, &required) != SIDEREON_STATUS_OK ||
        written != required) {
        free(rows);
        return fail("geometry: visible fill", 1);
    }
    for (size_t i = 0; i < written; i++) {
        if (!token_nonempty(&rows[i].satellite) ||
            strcmp(rows[i].satellite.bytes, W5_PARITY_VISIBLE_SATELLITES[i]) != 0 ||
            !same_bits(rows[i].elevation_deg, W5_PARITY_VISIBLE_ELEVATION_DEG_BITS[i]) ||
            !same_bits(rows[i].azimuth_deg, W5_PARITY_VISIBLE_AZIMUTH_DEG_BITS[i])) {
            free(rows);
            return fail("geometry: visible row differs from the engine's", 1);
        }
    }
    size_t n_visible = written;
    free(rows);

    /* The GPS-only count is the engine's; it cannot exceed the full count. */
    uint32_t gps_only[1] = {(uint32_t)SIDEREON_GNSS_SYSTEM_GPS};
    size_t gps_written = 0;
    size_t gps_required = 0;
    if (sidereon_sp3_geometry_visible(sp3, receiver, t_rx, mask_deg, gps_only, 1, NULL, 0,
                                      &gps_written, &gps_required) != SIDEREON_STATUS_OK ||
        gps_required != W5_PARITY_VISIBLE_GPS_COUNT || gps_required > n_visible) {
        return fail("geometry: GPS-filtered count exceeds total", 1);
    }

    /* visibility_series over a 30-minute window at 600 s steps (4 samples). */
    double window_end = t_rx + 1800.0;
    size_t series_written = 0;
    size_t series_required = 0;
    if (sidereon_sp3_geometry_visibility_series(sp3, receiver, t_rx, window_end, 600, mask_deg, NULL,
                                                0, NULL, 0, &series_written, &series_required) !=
            SIDEREON_STATUS_OK ||
        series_required != W5_PARITY_SERIES_COUNT) {
        return fail("geometry: visibility series count", 1);
    }
    SidereonVisibilitySeriesPoint *series = calloc(series_required, sizeof(*series));
    if (series == NULL) {
        return fail("geometry: visibility series alloc", 1);
    }
    if (sidereon_sp3_geometry_visibility_series(sp3, receiver, t_rx, window_end, 600, mask_deg, NULL,
                                                0, series, series_required, &series_written,
                                                &series_required) != SIDEREON_STATUS_OK ||
        series_written != series_required) {
        free(series);
        return fail("geometry: visibility series fill", 1);
    }
    for (size_t i = 0; i < series_written; i++) {
        if (series[i].n_visible != W5_PARITY_SERIES_N_VISIBLE[i]) {
            free(series);
            return fail("geometry: visibility series count differs from the engine's", 1);
        }
    }
    free(series);

    /* passes over the same window: a valid call (the count may legitimately be
     * zero if every visible satellite is already up at both ends). */
    size_t pass_written = 0;
    size_t pass_required = 0;
    if (sidereon_sp3_geometry_passes(sp3, receiver, t_rx, window_end, 600, mask_deg, NULL, 0, NULL,
                                     0, &pass_written, &pass_required) != SIDEREON_STATUS_OK ||
        pass_required != W5_PARITY_PASS_COUNT) {
        return fail("geometry: passes count", 1);
    }

    printf("geometry: %zu visible (>= %.0f deg), %zu series samples, %zu pass(es)\n", n_visible,
           mask_deg, series_required, pass_required);
    return 0;
}

/* Predict one satellite's observables from a source; light validity checks. */
static int check_observables(const SidereonPredictedObservables *obs) {
    if (!isfinite(obs->geometric_range_m) || obs->geometric_range_m <= 0.0) {
        return 1;
    }
    double norm = sqrt(obs->los_unit[0] * obs->los_unit[0] + obs->los_unit[1] * obs->los_unit[1] +
                       obs->los_unit[2] * obs->los_unit[2]);
    if (fabs(norm - 1.0) > 1.0e-6) {
        return 1;
    }
    if (!isfinite(obs->range_rate_m_s) || !isfinite(obs->doppler_hz) ||
        !isfinite(obs->elevation_deg) || !isfinite(obs->azimuth_deg)) {
        return 1;
    }
    return 0;
}

/* (2) Observables from SP3 and broadcast sources plus the options initializer. */
static int exercise_observables(const SidereonSp3 *sp3, const SidereonBroadcastEphemeris *broadcast,
                                SidereonObservablesOptions *out_options) {
    SidereonObservablesOptions options;
    if (sidereon_observables_options_init(&options) != SIDEREON_STATUS_OK ||
        !same_bits(options.carrier_hz, W5_PARITY_OPTIONS_CARRIER_HZ_BITS) ||
        options.light_time != W5_PARITY_OPTIONS_LIGHT_TIME ||
        options.sagnac != W5_PARITY_OPTIONS_SAGNAC) {
        return fail("observables: options init defaults", 1);
    }
    *out_options = options;

    double sp3_receiver[3] = {
        bits_to_f64(SPP_EXPECTED_X_BITS[0]),
        bits_to_f64(SPP_EXPECTED_X_BITS[1]),
        bits_to_f64(SPP_EXPECTED_X_BITS[2]),
    };
    double sp3_t_rx = bits_to_f64(SPP_T_RX_J2000_S_BITS);
    SidereonPredictedObservables sp3_obs;
    if (sidereon_sp3_observables(sp3, SPP_SAT_IDS[0], sp3_receiver, sp3_t_rx, NULL, &sp3_obs) !=
            SIDEREON_STATUS_OK ||
        check_observables(&sp3_obs) != 0 ||
        !same_bits(sp3_obs.geometric_range_m, W5_PARITY_SP3_OBS_RANGE_M_BITS) ||
        !same_bits(sp3_obs.range_rate_m_s, W5_PARITY_SP3_OBS_RANGE_RATE_M_S_BITS) ||
        !same_bits(sp3_obs.doppler_hz, W5_PARITY_SP3_OBS_DOPPLER_HZ_BITS) ||
        !same_bits(sp3_obs.elevation_deg, W5_PARITY_SP3_OBS_ELEVATION_DEG_BITS) ||
        !same_bits(sp3_obs.azimuth_deg, W5_PARITY_SP3_OBS_AZIMUTH_DEG_BITS) ||
        !same_bits_n(sp3_obs.los_unit, W5_PARITY_SP3_OBS_LOS_UNIT_BITS, 3)) {
        return fail("observables: sp3 predict", 1);
    }

    /* Broadcast source at its own epoch and approximate receiver position; find
     * one satellite the navigation message covers. */
    double bc_receiver[3] = {
        bits_to_f64(BC_INITIAL_GUESS_BITS[0]),
        bits_to_f64(BC_INITIAL_GUESS_BITS[1]),
        bits_to_f64(BC_INITIAL_GUESS_BITS[2]),
    };
    double bc_t_rx = bits_to_f64(BC_T_RX_J2000_S_BITS);
    int predicted = 0;
    if (W5_PARITY_BROADCAST_OBS_OK_COUNT != BC_OBS_COUNT) {
        return fail("observables: pinned broadcast row count", 1);
    }
    for (size_t i = 0; i < BC_OBS_COUNT; i++) {
        SidereonPredictedObservables bc_obs;
        bool ok = sidereon_broadcast_observables(broadcast, BC_SAT_IDS[i], bc_receiver, bc_t_rx,
                                                 &options, &bc_obs) == SIDEREON_STATUS_OK;
        if (ok != W5_PARITY_BROADCAST_OBS_OK[i]) {
            return fail("observables: broadcast outcome differs from the engine's", 1);
        }
        if (ok) {
            if (check_observables(&bc_obs) != 0 ||
                !same_bits(bc_obs.geometric_range_m, W5_PARITY_BROADCAST_OBS_RANGE_M_BITS[i]) ||
                !same_bits(bc_obs.range_rate_m_s, W5_PARITY_BROADCAST_OBS_RANGE_RATE_M_S_BITS[i])) {
                return fail("observables: broadcast row differs from the engine's", 1);
            }
            predicted++;
        }
    }

    printf("observables: sp3 range %.1f km, broadcast predicted %d satellite(s)\n",
           sp3_obs.geometric_range_m / 1000.0, predicted);
    return 0;
}

/* (3) Broadcast velocity: synthesize self-consistent range-rate observations
 * from the broadcast predictor (static receiver, zero clock drift), then solve.
 * Asserts a finite recovered velocity over at least four satellites. */
static int exercise_broadcast_velocity(const SidereonBroadcastEphemeris *broadcast,
                                       const SidereonObservablesOptions *options) {
    double receiver[3] = {
        bits_to_f64(BC_INITIAL_GUESS_BITS[0]),
        bits_to_f64(BC_INITIAL_GUESS_BITS[1]),
        bits_to_f64(BC_INITIAL_GUESS_BITS[2]),
    };
    double t_rx = bits_to_f64(BC_T_RX_J2000_S_BITS);

    SidereonVelocityObservation obs[BC_OBS_COUNT];
    size_t n = 0;
    for (size_t i = 0; i < BC_OBS_COUNT; i++) {
        SidereonPredictedObservables predicted;
        if (sidereon_broadcast_observables(broadcast, BC_SAT_IDS[i], receiver, t_rx, options,
                                           &predicted) != SIDEREON_STATUS_OK) {
            continue;
        }
        obs[n].sat_id = BC_SAT_IDS[i];
        obs[n].value = predicted.range_rate_m_s;
        obs[n].carrier_hz = options->carrier_hz;
        obs[n].sat_clock_drift_s_s = 0.0;
        n++;
    }
    if (n < 4) {
        return fail("broadcast velocity: too few predictable satellites", 1);
    }

    SidereonVelocityOptions velocity_options;
    if (sidereon_velocity_options_init(&velocity_options) != SIDEREON_STATUS_OK) {
        return fail("broadcast velocity: options init", 1);
    }
    velocity_options.observable = (uint32_t)SIDEREON_VELOCITY_OBSERVABLE_RANGE_RATE;

    SidereonVelocitySolution *solution = NULL;
    if (sidereon_solve_velocity_broadcast(broadcast, obs, n, receiver, t_rx, &velocity_options,
                                          &solution) != SIDEREON_STATUS_OK) {
        return fail("broadcast velocity: solve", 1);
    }
    double velocity[3];
    double speed = 0.0;
    double drift = 0.0;
    size_t used = 0;
    if (sidereon_velocity_solution_velocity(solution, velocity, 3) != SIDEREON_STATUS_OK ||
        sidereon_velocity_solution_speed(solution, &speed) != SIDEREON_STATUS_OK ||
        sidereon_velocity_solution_clock_drift(solution, &drift) != SIDEREON_STATUS_OK ||
        sidereon_velocity_solution_used_sat_count(solution, &used) != SIDEREON_STATUS_OK) {
        sidereon_velocity_solution_free(solution);
        return fail("broadcast velocity: readout", 1);
    }
    if (!same_bits_n(velocity, W5_PARITY_VELOCITY_M_S_BITS, 3) ||
        !same_bits(speed, W5_PARITY_SPEED_M_S_BITS) ||
        !same_bits(drift, W5_PARITY_CLOCK_DRIFT_S_S_BITS) || used != W5_PARITY_USED_SAT_COUNT) {
        sidereon_velocity_solution_free(solution);
        return fail("broadcast velocity: non-finite or under-determined", 1);
    }
    sidereon_velocity_solution_free(solution);

    printf("broadcast velocity: solved over %zu sats, speed %.4f m/s\n", used, speed);
    return 0;
}

/* (4) Reduced orbit: fit a circular mean-element model to ECEF samples
 * interpolated from one SP3 satellite over the first samples of the product,
 * then evaluate position / position+velocity and a drift report. The calendar
 * epochs are paired with each interpolated sample (the GRG SP3 starts at
 * 2020-06-24 00:00:00 with 15-minute spacing), so the fit is self-consistent. */
#define RO_SAMPLE_COUNT 9
static int exercise_reduced_orbit(const SidereonSp3 *sp3) {
    /* Read the first sample epochs (seconds since J2000) from the SP3. */
    size_t epoch_total = 0;
    size_t epoch_required = 0;
    if (sidereon_sp3_epochs_j2000_seconds(sp3, NULL, 0, &epoch_total, &epoch_required) !=
            SIDEREON_STATUS_OK ||
        epoch_required < RO_SAMPLE_COUNT) {
        return fail("reduced orbit: not enough SP3 epochs", 1);
    }
    double epochs[RO_SAMPLE_COUNT];
    size_t written = 0;
    size_t required = 0;
    /* The variable-length contract fills the supplied prefix and reports the
     * full count, so read all epochs into a full-size buffer and keep the
     * leading RO_SAMPLE_COUNT. */
    double *all = calloc(epoch_required, sizeof(*all));
    if (all == NULL) {
        return fail("reduced orbit: epoch alloc", 1);
    }
    if (sidereon_sp3_epochs_j2000_seconds(sp3, all, epoch_required, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written < RO_SAMPLE_COUNT) {
        free(all);
        return fail("reduced orbit: epoch fill", 1);
    }
    for (size_t i = 0; i < RO_SAMPLE_COUNT; i++) {
        epochs[i] = all[i];
    }
    free(all);

    SidereonEcefSample samples[RO_SAMPLE_COUNT];
    for (size_t i = 0; i < RO_SAMPLE_COUNT; i++) {
        double position_m[3];
        double clock_s[1];
        size_t interp_written = 0;
        if (sidereon_sp3_interpolate(sp3, SPP_SAT_IDS[0], &epochs[i], 1, position_m, 3, clock_s, 1,
                                     &interp_written) != SIDEREON_STATUS_OK ||
            interp_written != 1) {
            return fail("reduced orbit: SP3 interpolation", 1);
        }
        /* GRG SP3 first epoch is 2020-06-24 00:00:00, 900 s spacing. */
        long step_s = (long)i * 900;
        SidereonCalendarEpoch epoch = {0};
        epoch.year = 2020;
        epoch.month = 6;
        epoch.day = 24;
        epoch.hour = (int)(step_s / 3600);
        epoch.minute = (int)((step_s % 3600) / 60);
        epoch.second = (double)(step_s % 60);
        samples[i].epoch = epoch;
        samples[i].x_m = position_m[0];
        samples[i].y_m = position_m[1];
        samples[i].z_m = position_m[2];
    }

    SidereonReducedOrbitElements elements;
    SidereonReducedOrbitFitStats stats;
    if (sidereon_reduced_orbit_fit(samples, RO_SAMPLE_COUNT, (uint32_t)SIDEREON_TIME_SCALE_GPST,
                                   (uint32_t)SIDEREON_REDUCED_ORBIT_MODEL_CIRCULAR_SECULAR,
                                   &elements, &stats) != SIDEREON_STATUS_OK) {
        return fail("reduced orbit: fit", 1);
    }
    if (stats.n_samples != W5_PARITY_FIT_N_SAMPLES ||
        !same_bits(stats.rms_m, W5_PARITY_FIT_RMS_M_BITS) ||
        !same_bits(stats.max_m, W5_PARITY_FIT_MAX_M_BITS) ||
        !same_bits(elements.a_m, W5_PARITY_FIT_A_M_BITS) ||
        !same_bits(elements.mean_motion_rad_s, W5_PARITY_FIT_MEAN_MOTION_RAD_S_BITS)) {
        return fail("reduced orbit: fit stats out of range", 1);
    }

    /* Evaluate position and position+velocity at the first sample epoch. */
    double position[3];
    if (sidereon_reduced_orbit_position(&elements, &samples[0].epoch,
                                        (uint32_t)SIDEREON_TIME_SCALE_GPST,
                                        (uint32_t)SIDEREON_REDUCED_ORBIT_FRAME_ECEF, position, 3) !=
        SIDEREON_STATUS_OK) {
        return fail("reduced orbit: position", 1);
    }
    if (!same_bits_n(position, W5_PARITY_POSITION_ECEF_BITS, 3)) {
        return fail("reduced orbit: position differs from the engine's", 1);
    }
    double eval_pos[3];
    double eval_vel[3];
    if (sidereon_reduced_orbit_position_velocity(
            &elements, &samples[0].epoch, (uint32_t)SIDEREON_TIME_SCALE_GPST,
            (uint32_t)SIDEREON_REDUCED_ORBIT_FRAME_GCRS, eval_pos, eval_vel) != SIDEREON_STATUS_OK) {
        return fail("reduced orbit: position+velocity", 1);
    }
    if (!same_bits_n(eval_vel, W5_PARITY_VELOCITY_GCRS_BITS, 3)) {
        return fail("reduced orbit: velocity differs from the engine's", 1);
    }

    /* Drift report against the same truth samples. */
    SidereonReducedOrbitDriftReport *report = NULL;
    if (sidereon_reduced_orbit_drift(&elements, samples, RO_SAMPLE_COUNT,
                                     (uint32_t)SIDEREON_TIME_SCALE_GPST, 1.0e9, &report) !=
        SIDEREON_STATUS_OK) {
        return fail("reduced orbit: drift", 1);
    }
    size_t drift_written = 0;
    size_t drift_required = 0;
    if (sidereon_reduced_orbit_drift_report_entries(report, NULL, 0, &drift_written,
                                                    &drift_required) != SIDEREON_STATUS_OK ||
        drift_required != W5_PARITY_DRIFT_ENTRY_COUNT) {
        sidereon_reduced_orbit_drift_report_free(report);
        return fail("reduced orbit: drift entry count", 1);
    }
    SidereonReducedOrbitDriftEntry *entries = calloc(drift_required, sizeof(*entries));
    if (entries == NULL) {
        sidereon_reduced_orbit_drift_report_free(report);
        return fail("reduced orbit: drift entry alloc", 1);
    }
    if (sidereon_reduced_orbit_drift_report_entries(report, entries, drift_required, &drift_written,
                                                    &drift_required) != SIDEREON_STATUS_OK ||
        drift_written != drift_required) {
        free(entries);
        sidereon_reduced_orbit_drift_report_free(report);
        return fail("reduced orbit: drift entry fill", 1);
    }
    for (size_t i = 0; i < drift_written; i++) {
        if (!same_bits(entries[i].error_m, W5_PARITY_DRIFT_ERROR_M_BITS[i])) {
            free(entries);
            sidereon_reduced_orbit_drift_report_free(report);
            return fail("reduced orbit: drift entry differs from the engine's", 1);
        }
    }
    free(entries);

    SidereonReducedOrbitDriftSummary summary;
    if (sidereon_reduced_orbit_drift_report_summary(report, &summary) != SIDEREON_STATUS_OK ||
        !same_bits(summary.max_m, W5_PARITY_DRIFT_MAX_M_BITS) ||
        !same_bits(summary.rms_m, W5_PARITY_DRIFT_RMS_M_BITS) ||
        summary.has_threshold_crossing != W5_PARITY_DRIFT_HAS_CROSSING) {
        sidereon_reduced_orbit_drift_report_free(report);
        return fail("reduced orbit: drift summary", 1);
    }
    sidereon_reduced_orbit_drift_report_free(report);

    /* Threshold-crossing path: a 0 m threshold is crossed by the first entry with
     * positive error, so threshold_index addresses a real report entry whose
     * epoch is the crossing (no fabricated epoch). */
    SidereonReducedOrbitDriftReport *crossed = NULL;
    if (sidereon_reduced_orbit_drift(&elements, samples, RO_SAMPLE_COUNT,
                                     (uint32_t)SIDEREON_TIME_SCALE_GPST, 0.0, &crossed) !=
        SIDEREON_STATUS_OK) {
        return fail("reduced orbit: drift crossing", 1);
    }
    SidereonReducedOrbitDriftSummary crossed_summary;
    if (sidereon_reduced_orbit_drift_report_summary(crossed, &crossed_summary) !=
            SIDEREON_STATUS_OK ||
        crossed_summary.has_threshold_crossing != W5_PARITY_CROSSED_HAS_CROSSING ||
        crossed_summary.threshold_index != W5_PARITY_CROSSED_THRESHOLD_INDEX) {
        sidereon_reduced_orbit_drift_report_free(crossed);
        return fail("reduced orbit: drift crossing summary", 1);
    }
    SidereonReducedOrbitDriftEntry crossing_entries[RO_SAMPLE_COUNT];
    size_t crossing_written = 0;
    size_t crossing_required = 0;
    if (sidereon_reduced_orbit_drift_report_entries(crossed, crossing_entries, RO_SAMPLE_COUNT,
                                                    &crossing_written, &crossing_required) !=
            SIDEREON_STATUS_OK ||
        crossing_written != RO_SAMPLE_COUNT ||
        !same_bits(crossing_entries[crossed_summary.threshold_index].error_m,
                   W5_PARITY_DRIFT_ERROR_M_BITS[crossed_summary.threshold_index])) {
        sidereon_reduced_orbit_drift_report_free(crossed);
        return fail("reduced orbit: drift crossing entry", 1);
    }
    sidereon_reduced_orbit_drift_report_free(crossed);

    SidereonReducedOrbitPiecewise *piecewise = NULL;
    if (sidereon_reduced_orbit_fit_piecewise(
            samples, RO_SAMPLE_COUNT, (uint32_t)SIDEREON_TIME_SCALE_GPST,
            (uint32_t)SIDEREON_REDUCED_ORBIT_MODEL_CIRCULAR_SECULAR, &samples[0].epoch,
            &samples[RO_SAMPLE_COUNT - 1].epoch, 3600, &piecewise) != SIDEREON_STATUS_OK) {
        return fail("piecewise reduced orbit: fit", 1);
    }
    SidereonReducedOrbitPiecewiseInfo pinfo;
    if (sidereon_reduced_orbit_piecewise_info(piecewise, &pinfo) != SIDEREON_STATUS_OK ||
        pinfo.n_segments != W5_PARITY_PIECEWISE_N_SEGMENTS ||
        pinfo.segment_s != W5_PARITY_PIECEWISE_SEGMENT_S ||
        pinfo.model != (uint32_t)SIDEREON_REDUCED_ORBIT_MODEL_CIRCULAR_SECULAR ||
        pinfo.scale != (uint32_t)SIDEREON_TIME_SCALE_GPST) {
        sidereon_reduced_orbit_piecewise_free(piecewise);
        return fail("piecewise reduced orbit: info", 1);
    }
    size_t seg_written = 0;
    size_t seg_required = 0;
    if (sidereon_reduced_orbit_piecewise_segments(piecewise, NULL, 0, &seg_written,
                                                  &seg_required) != SIDEREON_STATUS_OK ||
        seg_written != 0 || seg_required != pinfo.n_segments) {
        sidereon_reduced_orbit_piecewise_free(piecewise);
        return fail("piecewise reduced orbit: segment count", 1);
    }
    SidereonReducedOrbitPiecewiseSegment *segments = calloc(seg_required, sizeof(*segments));
    if (segments == NULL) {
        sidereon_reduced_orbit_piecewise_free(piecewise);
        return fail("piecewise reduced orbit: segment alloc", 1);
    }
    if (sidereon_reduced_orbit_piecewise_segments(piecewise, segments, seg_required, &seg_written,
                                                  &seg_required) != SIDEREON_STATUS_OK ||
        seg_written != pinfo.n_segments) {
        free(segments);
        sidereon_reduced_orbit_piecewise_free(piecewise);
        return fail("piecewise reduced orbit: segment fill", 1);
    }
    for (size_t i = 0; i < seg_written; i++) {
        if (segments[i].stats.n_samples != W5_PARITY_PIECEWISE_SEGMENT_N_SAMPLES[i] ||
            !same_bits(segments[i].stats.rms_m, W5_PARITY_PIECEWISE_SEGMENT_RMS_M_BITS[i]) ||
            !same_bits(segments[i].elements.a_m, W5_PARITY_PIECEWISE_SEGMENT_A_M_BITS[i])) {
            free(segments);
            sidereon_reduced_orbit_piecewise_free(piecewise);
            return fail("piecewise reduced orbit: segment values", 1);
        }
    }

    size_t selected_index = 0;
    SidereonReducedOrbitPiecewiseSegment selected;
    if (sidereon_reduced_orbit_piecewise_select_segment(piecewise, &samples[4].epoch,
                                                        &selected_index,
                                                        &selected) != SIDEREON_STATUS_OK ||
        selected_index != W5_PARITY_SELECTED_INDEX ||
        !same_bits(selected.stats.rms_m, W5_PARITY_SELECTED_RMS_M_BITS)) {
        free(segments);
        sidereon_reduced_orbit_piecewise_free(piecewise);
        return fail("piecewise reduced orbit: select segment", 1);
    }
    free(segments);

    double pposition[3];
    if (sidereon_reduced_orbit_piecewise_position(
            piecewise, &samples[4].epoch, (uint32_t)SIDEREON_REDUCED_ORBIT_FRAME_ECEF, pposition,
            3) != SIDEREON_STATUS_OK ||
        !same_bits_n(pposition, W5_PARITY_PIECEWISE_POSITION_ECEF_BITS, 3)) {
        sidereon_reduced_orbit_piecewise_free(piecewise);
        return fail("piecewise reduced orbit: position", 1);
    }
    double ppos[3];
    double pvel[3];
    if (sidereon_reduced_orbit_piecewise_position_velocity(
            piecewise, &samples[4].epoch, (uint32_t)SIDEREON_REDUCED_ORBIT_FRAME_GCRS, ppos,
            pvel) != SIDEREON_STATUS_OK) {
        sidereon_reduced_orbit_piecewise_free(piecewise);
        return fail("piecewise reduced orbit: position+velocity", 1);
    }
    if (!same_bits_n(pvel, W5_PARITY_PIECEWISE_VELOCITY_GCRS_BITS, 3)) {
        sidereon_reduced_orbit_piecewise_free(piecewise);
        return fail("piecewise reduced orbit: velocity differs from the engine's", 1);
    }

    SidereonReducedOrbitDriftReport *preport = NULL;
    if (sidereon_reduced_orbit_piecewise_drift(piecewise, samples, RO_SAMPLE_COUNT, 1.0e9,
                                               &preport) != SIDEREON_STATUS_OK) {
        sidereon_reduced_orbit_piecewise_free(piecewise);
        return fail("piecewise reduced orbit: drift", 1);
    }
    SidereonReducedOrbitDriftEntry pentries[RO_SAMPLE_COUNT];
    size_t pdrift_written = 0;
    size_t pdrift_required = 0;
    if (sidereon_reduced_orbit_drift_report_entries(preport, pentries, RO_SAMPLE_COUNT,
                                                    &pdrift_written,
                                                    &pdrift_required) != SIDEREON_STATUS_OK ||
        pdrift_written != W5_PARITY_PIECEWISE_DRIFT_ENTRY_COUNT ||
        pdrift_required != W5_PARITY_PIECEWISE_DRIFT_ENTRY_COUNT) {
        sidereon_reduced_orbit_drift_report_free(preport);
        sidereon_reduced_orbit_piecewise_free(piecewise);
        return fail("piecewise reduced orbit: drift entries", 1);
    }
    SidereonReducedOrbitDriftSummary psummary;
    if (sidereon_reduced_orbit_drift_report_summary(preport, &psummary) != SIDEREON_STATUS_OK ||
        !same_bits(psummary.max_m, W5_PARITY_PIECEWISE_DRIFT_MAX_M_BITS) ||
        !same_bits(psummary.rms_m, W5_PARITY_PIECEWISE_DRIFT_RMS_M_BITS) ||
        psummary.has_threshold_crossing != W5_PARITY_PIECEWISE_DRIFT_HAS_CROSSING) {
        sidereon_reduced_orbit_drift_report_free(preport);
        sidereon_reduced_orbit_piecewise_free(piecewise);
        return fail("piecewise reduced orbit: drift summary", 1);
    }
    sidereon_reduced_orbit_drift_report_free(preport);
    sidereon_reduced_orbit_piecewise_free(piecewise);

    printf("reduced orbit: a = %.1f km, fit rms %.3e m, drift rms %.3e m, crossing index %zu, "
           "piecewise segments %zu\n",
           elements.a_m / 1000.0, stats.rms_m, summary.rms_m, crossed_summary.threshold_index,
           pinfo.n_segments);
    return 0;
}

/* (5) NRLMSISE-00: a representative thermospheric point yields a positive
 * density and temperature, and an out-of-domain altitude is rejected. */
static int exercise_atmosphere(void) {
    /* Source the quiet-Sun f107/f107a/ap defaults from the binding (which sources
     * them from sidereon_core::astro::atmosphere::{DEFAULT_F107, DEFAULT_F107A,
     * DEFAULT_AP}) rather than hardcoding them here. */
    SidereonAtmosphereInput input = sidereon_atmosphere_input_default();
    input.year = 2020;
    input.doy = 175;
    input.sec = 43200.0;
    input.alt_km = 400.0;
    input.lat_deg = 55.0;
    input.lon_deg = 12.0;

    SidereonAtmosphereOutput output;
    if (sidereon_atmosphere_nrlmsise00(&input, &output) != SIDEREON_STATUS_OK ||
        !same_bits(output.density_kg_m3, W5_PARITY_ATMOSPHERE_DENSITY_KG_M3_BITS) ||
        !same_bits(output.temperature_k, W5_PARITY_ATMOSPHERE_TEMPERATURE_K_BITS)) {
        return fail("atmosphere: nrlmsise00 nominal", 1);
    }

    /* The lst toggle: supplying the same local solar time the core derives
     * (sec/3600 + lon/15, wrapped to [0,24)) must reproduce the has_lst=false
     * result bit-for-bit. */
    double lst = input.sec / 3600.0 + input.lon_deg / 15.0;
    lst = fmod(fmod(lst, 24.0) + 24.0, 24.0);
    SidereonAtmosphereInput with_lst = input;
    with_lst.has_lst = true;
    with_lst.lst = lst;
    SidereonAtmosphereOutput lst_output;
    if (sidereon_atmosphere_nrlmsise00(&with_lst, &lst_output) != SIDEREON_STATUS_OK ||
        f64_to_bits(lst_output.density_kg_m3) != f64_to_bits(output.density_kg_m3) ||
        f64_to_bits(lst_output.temperature_k) != f64_to_bits(output.temperature_k)) {
        return fail("atmosphere: supplied lst matches derived", 1);
    }

    /* Ap-history mode: marshalling an Ap array through the binding must solve. */
    SidereonAtmosphereInput aph = input;
    aph.has_ap_array = true;
    for (size_t i = 0; i < SIDEREON_ATMOSPHERE_AP_ARRAY_LEN; i++) {
        aph.ap_array[i] = 100.0;
    }
    SidereonAtmosphereOutput aph_output;
    if (sidereon_atmosphere_nrlmsise00(&aph, &aph_output) != SIDEREON_STATUS_OK ||
        !same_bits(aph_output.density_kg_m3, W5_PARITY_ATMOSPHERE_AP_HISTORY_DENSITY_KG_M3_BITS) ||
        !same_bits(aph_output.temperature_k, W5_PARITY_ATMOSPHERE_AP_HISTORY_TEMPERATURE_K_BITS)) {
        return fail("atmosphere: nrlmsise00 ap-history", 1);
    }

    SidereonAtmosphereInput bad = input;
    bad.alt_km = 5000.0; /* above the documented 1000 km domain */
    SidereonAtmosphereOutput discard;
    if (W5_PARITY_ATMOSPHERE_5000_KM_OK ||
        sidereon_atmosphere_nrlmsise00(&bad, &discard) != SIDEREON_STATUS_INVALID_ARGUMENT) {
        return fail("atmosphere: out-of-domain not rejected", 1);
    }

    printf("atmosphere: density %.3e kg/m^3, temperature %.1f K (lst toggle + ap-history + "
           "out-of-domain rejected)\n",
           output.density_kg_m3, output.temperature_k);
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

    SidereonObservablesOptions options;
    int rc = 0;
    if (rc == 0) {
        rc = exercise_geometry(sp3);
    }
    if (rc == 0) {
        rc = exercise_observables(sp3, broadcast, &options);
    }
    if (rc == 0) {
        rc = exercise_broadcast_velocity(broadcast, &options);
    }
    if (rc == 0) {
        rc = exercise_reduced_orbit(sp3);
    }
    if (rc == 0) {
        rc = exercise_atmosphere();
    }

    sidereon_broadcast_ephemeris_free(broadcast);
    sidereon_sp3_free(sp3);

    if (rc == 0) {
        printf("parity smoke: OK\n");
    }
    return rc;
}
