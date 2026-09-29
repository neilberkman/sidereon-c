/*
 * Track filter and smoother smoke. The checks cover the public C ABI flow:
 * config/filter handles, position innovations with NIS, gated rejection,
 * recorded updates, RTS smoothing, and tide force-model switches.
 */
#include <math.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sidereon.h"
#include "w6_track_pins.h"

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

/* Every expected value below is sidereon-core's own result for these inputs,
 * from tests/valgen (w6_track). */
static bool same_bits(const double *values, const uint64_t *expected, size_t count) {
    for (size_t i = 0; i < count; i++) {
        uint64_t bits = 0;
        memcpy(&bits, &values[i], sizeof(bits));
        if (bits != expected[i]) {
            return false;
        }
    }
    return true;
}

static int copy_filter_position(SidereonTrackFilter *filter, double *out, size_t len,
                                const char *what) {
    size_t written = 0;
    size_t required = 0;
    if (require_ok(sidereon_track_filter_position_m(filter, out, len, &written, &required),
                   what) != 0) {
        return 1;
    }
    if (written != len || required != len) {
        return fail(what);
    }
    return 0;
}

static int copy_filter_covariance(SidereonTrackFilter *filter, double *out, size_t len,
                                  const char *what) {
    size_t written = 0;
    size_t required = 0;
    if (require_ok(sidereon_track_filter_covariance(filter, out, len, &written, &required),
                   what) != 0) {
        return 1;
    }
    if (written != len || required != len) {
        return fail(what);
    }
    return 0;
}

static int test_gated_spike(void) {
    enum { DIM = 1, STATE_DIM = 2 * DIM };
    double initial_position[DIM] = {0.0};
    double initial_velocity[DIM] = {1.0};
    double initial_covariance[STATE_DIM * STATE_DIM] = {
        1.0, 0.0,
        0.0, 1.0,
    };
    SidereonTrackFilterConfig *config = NULL;
    SidereonTrackFilter *filter = NULL;
    SidereonTrackRtsHistoryBuilder *history = NULL;
    SidereonTrackRtsHistory *recorded = NULL;
    SidereonSmoothedTrack *smoothed = NULL;
    int rc = 1;

    if (require_ok(sidereon_track_filter_config_from_position_velocity(
                       SIDEREON_TRACK_COORDINATE_FRAME_CALLER_DEFINED_CARTESIAN, 0.0,
                       initial_position, initial_velocity, DIM, initial_covariance,
                       sizeof(initial_covariance) / sizeof(initial_covariance[0]), 0.1, &config),
                   "track config from position velocity") != 0 ||
        require_ok(sidereon_track_filter_new(config, &filter), "track filter new") != 0 ||
        require_ok(sidereon_track_rts_history_builder_from_filter(filter, &history),
                   "history from filter") != 0) {
        goto cleanup;
    }

    SidereonTrackPrediction prediction;
    if (require_ok(sidereon_track_filter_predict_recorded(filter, 1.0, history, &prediction),
                   "predict recorded") != 0 ||
        prediction.predicted.dimension != DIM ||
        prediction.predicted.state_dimension != STATE_DIM) {
        goto cleanup;
    }

    double predicted_position[DIM] = {0.0};
    double predicted_covariance[STATE_DIM * STATE_DIM] = {0.0};
    if (copy_filter_position(filter, predicted_position, DIM, "predicted position") != 0 ||
        copy_filter_covariance(filter, predicted_covariance, STATE_DIM * STATE_DIM,
                               "predicted covariance") != 0 ||
        !same_bits(predicted_position, W6_TRACK_SPIKE_PREDICTED_POSITION_BITS, DIM) ||
        !same_bits(predicted_covariance, W6_TRACK_SPIKE_PREDICTED_COVARIANCE_BITS,
                   STATE_DIM * STATE_DIM)) {
        goto cleanup;
    }

    double spike[DIM] = {100.0};
    double spike_covariance[DIM * DIM] = {0.01};
    double innovation[DIM] = {0.0};
    double innovation_covariance[DIM * DIM] = {0.0};
    SidereonTrackInnovation innovation_report;
    if (require_ok(sidereon_track_filter_position_innovation(
                       filter, spike, DIM, spike_covariance,
                       sizeof(spike_covariance) / sizeof(spike_covariance[0]), innovation, DIM,
                       innovation_covariance, DIM * DIM, &innovation_report),
                   "position innovation") != 0 ||
        innovation_report.dimension != DIM ||
        !same_bits(&innovation_report.nis, &W6_TRACK_SPIKE_NIS_BITS, 1) ||
        !same_bits(innovation, W6_TRACK_SPIKE_INNOVATION_BITS, DIM) ||
        !same_bits(innovation_covariance, W6_TRACK_SPIKE_INNOVATION_COVARIANCE_BITS, DIM * DIM)) {
        goto cleanup;
    }

    SidereonTrackGatedUpdate gated;
    if (require_ok(sidereon_track_filter_update_position_gated_recorded(
                       filter, spike, DIM, spike_covariance,
                       sizeof(spike_covariance) / sizeof(spike_covariance[0]), 0.95, history,
                       &gated),
                   "gated recorded update") != 0 ||
        gated.gate.in_gate != W6_TRACK_SPIKE_IN_GATE ||
        gated.has_update != W6_TRACK_SPIKE_HAS_UPDATE ||
        !same_bits(&gated.gate.nis, &W6_TRACK_SPIKE_GATE_NIS_BITS, 1) ||
        !same_bits(&gated.gate.threshold, &W6_TRACK_SPIKE_GATE_THRESHOLD_BITS, 1) ||
        gated.state.dimension != prediction.predicted.dimension) {
        goto cleanup;
    }

    double after_position[DIM] = {0.0};
    double after_covariance[STATE_DIM * STATE_DIM] = {0.0};
    if (copy_filter_position(filter, after_position, DIM, "after gated position") != 0 ||
        copy_filter_covariance(filter, after_covariance, STATE_DIM * STATE_DIM,
                               "after gated covariance") != 0 ||
        !same_bits(after_position, W6_TRACK_SPIKE_AFTER_POSITION_BITS, DIM) ||
        !same_bits(after_covariance, W6_TRACK_SPIKE_AFTER_COVARIANCE_BITS,
                   STATE_DIM * STATE_DIM) ||
        !close_abs(after_position[0], predicted_position[0], 0.0)) {
        goto cleanup;
    }
    for (size_t i = 0; i < STATE_DIM * STATE_DIM; i++) {
        if (!close_abs(after_covariance[i], predicted_covariance[i], 0.0)) {
            goto cleanup;
        }
    }

    if (require_ok(sidereon_track_rts_history_builder_finish(history, &recorded),
                   "finish history") != 0 ||
        recorded == NULL ||
        require_ok(sidereon_smooth_track_rts(recorded, &smoothed), "smooth history") != 0 ||
        smoothed == NULL) {
        goto cleanup;
    }

    size_t recorded_count = 0;
    size_t smoothed_count = 0;
    if (require_ok(sidereon_track_rts_history_epoch_count(recorded, &recorded_count),
                   "history count") != 0 ||
        require_ok(sidereon_smoothed_track_epoch_count(smoothed, &smoothed_count),
                   "smoothed count") != 0 ||
        recorded_count != W6_TRACK_SPIKE_RECORDED_COUNT ||
        smoothed_count != W6_TRACK_SPIKE_SMOOTHED_COUNT || smoothed_count != recorded_count) {
        goto cleanup;
    }

    double last_smoothed_position[DIM] = {0.0};
    size_t written = 0;
    size_t required = 0;
    if (require_ok(sidereon_smoothed_track_epoch_position_m(
                       smoothed, smoothed_count - 1, last_smoothed_position, DIM, &written,
                       &required),
                   "last smoothed position") != 0 ||
        written != DIM || required != DIM ||
        !same_bits(last_smoothed_position, W6_TRACK_SPIKE_LAST_SMOOTHED_POSITION_BITS, DIM) ||
        !close_abs(last_smoothed_position[0], predicted_position[0], 0.0)) {
        goto cleanup;
    }

    rc = 0;

cleanup:
    sidereon_smoothed_track_free(smoothed);
    sidereon_track_rts_history_free(recorded);
    sidereon_track_rts_history_builder_free(history);
    sidereon_track_filter_free(filter);
    sidereon_track_filter_config_free(config);
    return rc;
}

static int test_recorded_fix_smoothing(void) {
    enum { DIM = 3, STATE_DIM = 2 * DIM };
    double initial_position[DIM] = {0.0, 0.0, 0.0};
    double position_covariance[DIM * DIM] = {
        1.0, 0.0, 0.0,
        0.0, 1.0, 0.0,
        0.0, 0.0, 1.0,
    };
    double fix[DIM] = {1.0, 0.0, 0.0};
    double fix_covariance[DIM * DIM] = {
        0.25, 0.0, 0.0,
        0.0, 0.25, 0.0,
        0.0, 0.0, 0.25,
    };
    SidereonTrackFilter *filter = NULL;
    SidereonTrackRtsHistoryBuilder *history = NULL;
    SidereonTrackRtsHistory *recorded = NULL;
    SidereonSmoothedTrack *smoothed = NULL;
    int rc = 1;

    if (require_ok(sidereon_track_filter_new_from_position(
                       SIDEREON_TRACK_COORDINATE_FRAME_ECEF, 0.0, initial_position, DIM,
                       position_covariance,
                       sizeof(position_covariance) / sizeof(position_covariance[0]), 25.0, 0.05,
                       &filter),
                   "track filter from position") != 0 ||
        require_ok(sidereon_track_rts_history_builder_from_filter(filter, &history),
                   "history from filter") != 0) {
        goto cleanup;
    }

    SidereonTrackPrediction prediction;
    SidereonTrackUpdate update;
    if (require_ok(sidereon_track_filter_predict_recorded(filter, 1.0, history, &prediction),
                   "predict recorded") != 0 ||
        require_ok(sidereon_track_filter_update_position_recorded(
                       filter, fix, DIM, fix_covariance,
                       sizeof(fix_covariance) / sizeof(fix_covariance[0]), history, &update),
                   "recorded position update") != 0 ||
        update.updated.frame != W6_TRACK_FIX_UPDATED_FRAME ||
        update.innovation.dimension != W6_TRACK_FIX_INNOVATION_DIMENSION ||
        !same_bits(&update.innovation.nis, &W6_TRACK_FIX_NIS_BITS, 1) ||
        update.updated.dimension != prediction.predicted.dimension) {
        goto cleanup;
    }

    double after_position[DIM] = {0.0, 0.0, 0.0};
    if (copy_filter_position(filter, after_position, DIM, "updated position") != 0 ||
        !same_bits(after_position, W6_TRACK_FIX_UPDATED_POSITION_BITS, DIM)) {
        goto cleanup;
    }

    if (require_ok(sidereon_track_rts_history_builder_finish(history, &recorded),
                   "finish history") != 0 ||
        require_ok(sidereon_smooth_track_rts(recorded, &smoothed), "smooth history") != 0) {
        goto cleanup;
    }

    size_t recorded_count = 0;
    size_t smoothed_count = 0;
    if (require_ok(sidereon_track_rts_history_epoch_count(recorded, &recorded_count),
                   "recorded count") != 0 ||
        require_ok(sidereon_smoothed_track_epoch_count(smoothed, &smoothed_count),
                   "smoothed count") != 0 ||
        recorded_count != W6_TRACK_FIX_RECORDED_COUNT ||
        smoothed_count != W6_TRACK_FIX_SMOOTHED_COUNT) {
        goto cleanup;
    }

    SidereonSmoothedTrackEpoch first_epoch;
    size_t written = 0;
    size_t required = 0;
    if (require_ok(sidereon_smoothed_track_epoch(smoothed, 0, &first_epoch),
                   "first smoothed epoch") != 0 ||
        first_epoch.has_rts_gain_to_next != W6_TRACK_FIX_FIRST_HAS_GAIN ||
        require_ok(sidereon_smoothed_track_epoch_rts_gain_to_next(
                       smoothed, 0, NULL, 0, &written, &required),
                   "first RTS gain query") != 0 ||
        required != W6_TRACK_FIX_FIRST_GAIN_LEN ||
        required != first_epoch.state.state_dimension * first_epoch.state.state_dimension) {
        goto cleanup;
    }

    double covariance[STATE_DIM * STATE_DIM] = {0.0};
    if (require_ok(sidereon_smoothed_track_epoch_covariance(
                       smoothed, 0, covariance, sizeof(covariance) / sizeof(covariance[0]),
                       &written, &required),
                   "smoothed covariance") != 0 ||
        written != required || required != STATE_DIM * STATE_DIM ||
        !same_bits(covariance, W6_TRACK_FIX_FIRST_COVARIANCE_BITS, STATE_DIM * STATE_DIM)) {
        goto cleanup;
    }

    if (require_ok(sidereon_smoothed_track_epoch_rts_gain_to_next(
                       smoothed, smoothed_count - 1, NULL, 0, &written, &required),
                   "last RTS gain query") != 0 ||
        required != W6_TRACK_FIX_LAST_GAIN_LEN) {
        goto cleanup;
    }

    rc = 0;

cleanup:
    sidereon_smoothed_track_free(smoothed);
    sidereon_track_rts_history_free(recorded);
    sidereon_track_rts_history_builder_free(history);
    sidereon_track_filter_free(filter);
    return rc;
}

static int test_tide_force_options(void) {
    SidereonStatePropagationConfig config;
    if (require_ok(sidereon_state_propagation_config_init(&config), "propagation config") != 0) {
        return 1;
    }
    config.epoch_s = 0.0;
    config.position_km[0] = 7078.0;
    config.position_km[1] = -30.0;
    config.position_km[2] = 820.0;
    config.velocity_km_s[0] = 0.20;
    config.velocity_km_s[1] = 7.35;
    config.velocity_km_s[2] = 1.05;
    config.force_model = SIDEREON_PROPAGATION_FORCE_MODEL_COMPOSITE;
    config.integrator = SIDEREON_PROPAGATION_INTEGRATOR_RK4;
    config.initial_step_s = 30.0;
    config.max_step_s = 30.0;
    config.force_components.has_solid_earth_tide = true;
    config.force_components.has_solid_earth_pole_tide = true;

    double times[] = {0.0, 60.0};
    const size_t time_count = sizeof(times) / sizeof(times[0]);
    SidereonEphemeris *ephemeris = NULL;
    if (require_ok(sidereon_propagate_state(&config, times, time_count, &ephemeris),
                   "tide force propagation") != 0) {
        return 1;
    }

    int rc = 1;
    size_t count = 0;
    size_t written = 0;
    size_t required = 0;
    SidereonCartesianState states[sizeof(times) / sizeof(times[0])];
    if (require_ok(sidereon_ephemeris_epoch_count(ephemeris, &count), "ephemeris count") != 0 ||
        count != time_count ||
        require_ok(sidereon_ephemeris_states(ephemeris, states, time_count, &written, &required),
                   "ephemeris states") != 0 ||
        written != time_count || required != time_count) {
        goto cleanup;
    }
    for (size_t i = 0; i < time_count; i++) {
        for (size_t axis = 0; axis < sizeof(states[i].position_km) / sizeof(states[i].position_km[0]);
             axis++) {
            if (!isfinite(states[i].position_km[axis]) || !isfinite(states[i].velocity_km_s[axis])) {
                goto cleanup;
            }
        }
    }

    rc = 0;

cleanup:
    sidereon_ephemeris_free(ephemeris);
    return rc;
}

int main(void) {
    if (test_gated_spike() != 0) {
        return 1;
    }
    if (test_recorded_fix_smoothing() != 0) {
        return 1;
    }
    if (test_tide_force_options() != 0) {
        return 1;
    }
    printf("track_smoke: OK\n");
    return 0;
}
