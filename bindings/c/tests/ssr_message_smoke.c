#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sidereon.h"

static int failures = 0;

static void check(int ok, const char *what) {
    if (!ok) {
        char message[512];
        size_t n = sidereon_last_error_message(message, sizeof(message));
        if (n == 0) {
            message[0] = '\0';
        }
        /* The last error is sticky and may come from an earlier call, so it is
           reported as context rather than as the cause of this failure. */
        if (message[0] != '\0') {
            fprintf(stderr, "FAIL: %s (last ABI error, may predate this check: %s)\n",
                    what, message);
        } else {
            fprintf(stderr, "FAIL: %s\n", what);
        }
        failures++;
    }
}

static void check_status(SidereonStatus got, SidereonStatus want, const char *what) {
    if (got != want) {
        fprintf(stderr, "FAIL: %s (got %s, want %s)\n", what,
                sidereon_status_message(got), sidereon_status_message(want));
        check(0, what);
    }
}

static void check_error_present(const char *what) {
    char message[512];
    check(sidereon_last_error_message(message, sizeof(message)) > 0, what);
}

static int hex_nibble(char c) {
    if (c >= '0' && c <= '9') {
        return c - '0';
    }
    if (c >= 'a' && c <= 'f') {
        return c - 'a' + 10;
    }
    if (c >= 'A' && c <= 'F') {
        return c - 'A' + 10;
    }
    return -1;
}

static size_t hex_to_bytes(const char *hex, uint8_t *out, size_t cap) {
    size_t chars = strlen(hex);
    size_t bytes = chars / 2;
    if ((chars % 2) != 0 || bytes > cap) {
        return 0;
    }
    for (size_t i = 0; i < bytes; i++) {
        int hi = hex_nibble(hex[2 * i]);
        int lo = hex_nibble(hex[2 * i + 1]);
        if (hi < 0 || lo < 0) {
            return 0;
        }
        out[i] = (uint8_t)((hi << 4) | lo);
    }
    return bytes;
}

static int header_equal(const SidereonRtcmSsrHeader *a,
                        const SidereonRtcmSsrHeader *b) {
    return a->epoch_time_s == b->epoch_time_s &&
           a->update_interval == b->update_interval &&
           a->multiple_message == b->multiple_message && a->iod_ssr == b->iod_ssr &&
           a->provider_id == b->provider_id && a->solution_id == b->solution_id &&
           a->has_satellite_reference_datum == b->has_satellite_reference_datum &&
           a->satellite_reference_datum == b->satellite_reference_datum &&
           a->has_dispersive_bias_consistency == b->has_dispersive_bias_consistency &&
           a->dispersive_bias_consistency == b->dispersive_bias_consistency &&
           a->has_mw_consistency == b->has_mw_consistency &&
           a->mw_consistency == b->mw_consistency && a->satellite_count == b->satellite_count;
}

static int info_equal(const SidereonRtcmSsrInfo *a, const SidereonRtcmSsrInfo *b) {
    return a->message_number == b->message_number && a->system == b->system &&
           a->kind == b->kind && header_equal(&a->header, &b->header) &&
           a->orbit_count == b->orbit_count && a->clock_count == b->clock_count &&
           a->ura_count == b->ura_count && a->code_bias_count == b->code_bias_count &&
           a->phase_bias_count == b->phase_bias_count;
}

static int orbit_equal(const SidereonRtcmSsrOrbitRecord *a,
                       const SidereonRtcmSsrOrbitRecord *b) {
    return a->satellite_id == b->satellite_id && a->iode == b->iode &&
           a->delta_radial == b->delta_radial && a->delta_along == b->delta_along &&
           a->delta_cross == b->delta_cross && a->dot_delta_radial == b->dot_delta_radial &&
           a->dot_delta_along == b->dot_delta_along && a->dot_delta_cross == b->dot_delta_cross;
}

static int clock_equal(const SidereonRtcmSsrClockRecord *a,
                       const SidereonRtcmSsrClockRecord *b) {
    return a->satellite_id == b->satellite_id && a->c0 == b->c0 && a->c1 == b->c1 &&
           a->c2 == b->c2;
}

static void check_info_common(const SidereonRtcmSsrInfo *info, uint16_t message_number,
                              SidereonRtcmSsrKind kind, size_t orbit_count, size_t clock_count,
                              size_t ura_count, size_t code_bias_count,
                              size_t phase_bias_count, const char *what) {
    check(info->message_number == message_number && info->system == SIDEREON_GNSS_SYSTEM_GPS &&
              info->kind == kind && info->header.epoch_time_s == 345600 &&
              info->header.update_interval == 2 && info->header.multiple_message &&
              info->header.iod_ssr == 9 && info->header.provider_id == 123 &&
              info->header.solution_id == 4 && info->header.satellite_count == 1 &&
              info->orbit_count == orbit_count && info->clock_count == clock_count &&
              info->ura_count == ura_count && info->code_bias_count == code_bias_count &&
              info->phase_bias_count == phase_bias_count,
          what);
}

static void check_copy_counts(size_t written, size_t required, size_t want, const char *what) {
    check(written == 0 && required == want, what);
}

#define TEST_ARRAY_1(fn, handle, type, want, label)                                      \
    do {                                                                                 \
        size_t _written = (size_t)-1;                                                    \
        size_t _required = (size_t)-1;                                                   \
        check_status((fn)((handle), NULL, 0, &_written, &_required), SIDEREON_STATUS_OK,  \
                     label " query");                                                   \
        check_copy_counts(_written, _required, (want), label " query counts");           \
        type _rows[8];                                                                    \
        _written = (size_t)-1;                                                            \
        _required = (size_t)-1;                                                           \
        check_status((fn)((handle), _rows, (want) - 1, &_written, &_required),            \
                     SIDEREON_STATUS_INVALID_ARGUMENT, label " short buffer");          \
        check_copy_counts(_written, _required, (want), label " short counts");            \
        check_error_present(label " short error");                                       \
        _written = (size_t)-1;                                                            \
        _required = (size_t)-1;                                                           \
        check_status((fn)((handle), _rows, (want), &_written, &_required),                \
                     SIDEREON_STATUS_OK, label " copy");                                \
        check(_written == (want) && _required == (want), label " copy counts");          \
    } while (0)

#define TEST_ARRAY_2(fn, handle, message_index, type, want, label)                         \
    do {                                                                                   \
        size_t _written = (size_t)-1;                                                      \
        size_t _required = (size_t)-1;                                                     \
        check_status((fn)((handle), (message_index), NULL, 0, &_written, &_required),     \
                     SIDEREON_STATUS_OK, label " query");                                \
        check_copy_counts(_written, _required, (want), label " query counts");            \
        type _rows[8];                                                                      \
        _written = (size_t)-1;                                                              \
        _required = (size_t)-1;                                                             \
        check_status((fn)((handle), (message_index), _rows, (want) - 1, &_written,         \
                          &_required), SIDEREON_STATUS_INVALID_ARGUMENT,                 \
                     label " short buffer");                                             \
        check_copy_counts(_written, _required, (want), label " short counts");             \
        check_error_present(label " short error");                                        \
        _written = (size_t)-1;                                                              \
        _required = (size_t)-1;                                                             \
        check_status((fn)((handle), (message_index), _rows, (want), &_written,             \
                          &_required), SIDEREON_STATUS_OK, label " copy");                \
        check(_written == (want) && _required == (want), label " copy counts");           \
    } while (0)

#define TEST_NESTED_1(fn, handle, record_index, type, want, label)                         \
    do {                                                                                   \
        size_t _written = (size_t)-1;                                                      \
        size_t _required = (size_t)-1;                                                     \
        check_status((fn)((handle), (record_index), NULL, 0, &_written, &_required),       \
                     SIDEREON_STATUS_OK, label " query");                                \
        check_copy_counts(_written, _required, (want), label " query counts");            \
        type _rows[8];                                                                      \
        _written = (size_t)-1;                                                              \
        _required = (size_t)-1;                                                             \
        check_status((fn)((handle), (record_index), _rows, (want) - 1, &_written,          \
                          &_required), SIDEREON_STATUS_INVALID_ARGUMENT,                 \
                     label " short buffer");                                             \
        check_copy_counts(_written, _required, (want), label " short counts");             \
        check_error_present(label " short error");                                        \
        _written = (size_t)-1;                                                              \
        _required = (size_t)-1;                                                             \
        check_status((fn)((handle), (record_index), _rows, (want), &_written,              \
                          &_required), SIDEREON_STATUS_OK, label " copy");                \
        check(_written == (want) && _required == (want), label " copy counts");           \
    } while (0)

#define TEST_NESTED_2(fn, handle, message_index, record_index, type, want, label)           \
    do {                                                                                   \
        size_t _written = (size_t)-1;                                                      \
        size_t _required = (size_t)-1;                                                     \
        check_status((fn)((handle), (message_index), (record_index), NULL, 0,              \
                          &_written, &_required), SIDEREON_STATUS_OK, label " query");   \
        check_copy_counts(_written, _required, (want), label " query counts");             \
        type _rows[8];                                                                      \
        _written = (size_t)-1;                                                              \
        _required = (size_t)-1;                                                             \
        check_status((fn)((handle), (message_index), (record_index), _rows, (want) - 1,     \
                          &_written, &_required), SIDEREON_STATUS_INVALID_ARGUMENT,       \
                     label " short buffer");                                             \
        check_copy_counts(_written, _required, (want), label " short counts");             \
        check_error_present(label " short error");                                        \
        _written = (size_t)-1;                                                              \
        _required = (size_t)-1;                                                             \
        check_status((fn)((handle), (message_index), (record_index), _rows, (want),         \
                          &_written, &_required), SIDEREON_STATUS_OK, label " copy");     \
        check(_written == (want) && _required == (want), label " copy counts");            \
    } while (0)

static void compare_combined(const uint8_t *frame, size_t frame_len) {
    uint8_t body[256];
    size_t body_written = (size_t)-1;
    size_t body_required = (size_t)-1;
    size_t decoded_frame_len = 0;
    check_status(sidereon_rtcm_decode_frame(frame, frame_len, body, sizeof(body), &body_written,
                                            &body_required, &decoded_frame_len),
                 SIDEREON_STATUS_OK, "combined frame body decode");
    check(body_written > 0 && body_required == body_written && decoded_frame_len == frame_len,
          "combined frame body counts");

    SidereonRtcmMessages *messages = NULL;
    SidereonSsrMessage *bare = NULL;
    check_status(sidereon_rtcm_decode_messages(frame, frame_len, &messages), SIDEREON_STATUS_OK,
                 "combined framed decode");
    check_status(sidereon_ssr_message_decode(body, body_written, &bare), SIDEREON_STATUS_OK,
                 "combined bare decode");
    check(messages != NULL && bare != NULL, "combined handles");
    if (messages == NULL || bare == NULL) {
        sidereon_rtcm_messages_free(messages);
        sidereon_ssr_message_free(bare);
        return;
    }

    size_t message_count = 0;
    check_status(sidereon_rtcm_messages_count(messages, &message_count), SIDEREON_STATUS_OK,
                 "combined message count");
    check(message_count == 1, "combined message count value");
    SidereonRtcmSsrInfo framed_info;
    SidereonRtcmSsrInfo bare_info;
    check_status(sidereon_rtcm_message_ssr_info(messages, 0, &framed_info), SIDEREON_STATUS_OK,
                 "combined framed info");
    check_status(sidereon_ssr_message_info(bare, &bare_info), SIDEREON_STATUS_OK,
                 "combined bare info");
    check(info_equal(&framed_info, &bare_info) && framed_info.message_number == 1060 &&
              framed_info.kind == SIDEREON_RTCM_SSR_KIND_COMBINED_ORBIT_CLOCK &&
              framed_info.orbit_count > 0 && framed_info.clock_count > 0,
          "combined info exact equality");

    TEST_ARRAY_2(sidereon_rtcm_message_ssr_orbits, messages, 0,
                 SidereonRtcmSsrOrbitRecord, framed_info.orbit_count, "combined framed orbits");
    TEST_ARRAY_1(sidereon_ssr_message_orbits, bare, SidereonRtcmSsrOrbitRecord,
                 bare_info.orbit_count, "combined bare orbits");
    TEST_ARRAY_2(sidereon_rtcm_message_ssr_clocks, messages, 0,
                 SidereonRtcmSsrClockRecord, framed_info.clock_count, "combined framed clocks");
    TEST_ARRAY_1(sidereon_ssr_message_clocks, bare, SidereonRtcmSsrClockRecord,
                 bare_info.clock_count, "combined bare clocks");

    SidereonRtcmSsrOrbitRecord framed_orbits[8];
    SidereonRtcmSsrOrbitRecord bare_orbits[8];
    size_t framed_written = 0;
    size_t framed_required = 0;
    size_t bare_written = 0;
    size_t bare_required = 0;
    check_status(sidereon_rtcm_message_ssr_orbits(messages, 0, framed_orbits, 8, &framed_written,
                                                  &framed_required),
                 SIDEREON_STATUS_OK, "combined framed orbit copy");
    check_status(sidereon_ssr_message_orbits(bare, bare_orbits, 8, &bare_written, &bare_required),
                 SIDEREON_STATUS_OK, "combined bare orbit copy");
    int orbit_fields_equal = framed_written == bare_written && framed_required == bare_required;
    for (size_t i = 0; i < framed_written && orbit_fields_equal; i++) {
        orbit_fields_equal = orbit_equal(&framed_orbits[i], &bare_orbits[i]);
    }
    check(orbit_fields_equal, "combined orbit fields exact equality");

    SidereonRtcmSsrClockRecord framed_clocks[8];
    SidereonRtcmSsrClockRecord bare_clocks[8];
    check_status(sidereon_rtcm_message_ssr_clocks(messages, 0, framed_clocks, 8, &framed_written,
                                                  &framed_required),
                 SIDEREON_STATUS_OK, "combined framed clock copy");
    check_status(sidereon_ssr_message_clocks(bare, bare_clocks, 8, &bare_written, &bare_required),
                 SIDEREON_STATUS_OK, "combined bare clock copy");
    int clock_fields_equal = framed_written == bare_written && framed_required == bare_required;
    for (size_t i = 0; i < framed_written && clock_fields_equal; i++) {
        clock_fields_equal = clock_equal(&framed_clocks[i], &bare_clocks[i]);
    }
    check(clock_fields_equal, "combined clock fields exact equality");

    size_t null_written = 77;
    size_t null_required = 88;
    check_status(sidereon_ssr_message_orbits(bare, NULL, 0, &null_written, &null_required),
                 SIDEREON_STATUS_OK, "combined bare null query");
    check(null_written == 0 && null_required == bare_info.orbit_count,
          "combined bare null query counts");
    check_status(sidereon_rtcm_message_ssr_orbits(messages, 0, NULL, 0, NULL, &null_required),
                 SIDEREON_STATUS_NULL_POINTER, "combined null out_written");
    check_error_present("combined null out_written error");

    sidereon_rtcm_messages_free(messages);
    sidereon_ssr_message_free(bare);
}

static void make_frame(const uint8_t *body, size_t body_len, uint8_t *frame, size_t frame_cap,
                       size_t *frame_len, const char *what) {
    size_t written = (size_t)-1;
    size_t required = (size_t)-1;
    check_status(sidereon_rtcm_encode_frame(body, body_len, frame, frame_cap, &written, &required),
                 SIDEREON_STATUS_OK, what);
    check(written > 0 && required == written, "SSR generated frame counts");
    *frame_len = written;
}

static void compare_code(const uint8_t *body, size_t body_len) {
    uint8_t frame[256];
    size_t frame_len = 0;
    make_frame(body, body_len, frame, sizeof(frame), &frame_len, "code-bias frame encode");
    SidereonRtcmMessages *messages = NULL;
    SidereonSsrMessage *bare = NULL;
    check_status(sidereon_rtcm_decode_messages(frame, frame_len, &messages), SIDEREON_STATUS_OK,
                 "code-bias framed decode");
    check_status(sidereon_ssr_message_decode(body, body_len, &bare), SIDEREON_STATUS_OK,
                 "code-bias bare decode");
    check(messages != NULL && bare != NULL, "code-bias handles");
    if (messages == NULL || bare == NULL) {
        sidereon_rtcm_messages_free(messages);
        sidereon_ssr_message_free(bare);
        return;
    }

    SidereonRtcmSsrInfo framed_info;
    SidereonRtcmSsrInfo bare_info;
    check_status(sidereon_rtcm_message_ssr_info(messages, 0, &framed_info), SIDEREON_STATUS_OK,
                 "code-bias framed info");
    check_status(sidereon_ssr_message_info(bare, &bare_info), SIDEREON_STATUS_OK,
                 "code-bias bare info");
    check(info_equal(&framed_info, &bare_info), "code-bias info exact equality");
    check_info_common(&bare_info, 1059, SIDEREON_RTCM_SSR_KIND_CODE_BIAS, 0, 0, 0, 1, 0,
                      "code-bias expected info");
    check(!bare_info.header.has_satellite_reference_datum &&
              !bare_info.header.has_dispersive_bias_consistency &&
              !bare_info.header.has_mw_consistency,
          "code-bias optional header flags");

    TEST_ARRAY_2(sidereon_rtcm_message_ssr_code_biases, messages, 0,
                 SidereonRtcmSsrCodeBiasRecord, 1, "code-bias framed records");
    TEST_ARRAY_1(sidereon_ssr_message_code_biases, bare, SidereonRtcmSsrCodeBiasRecord, 1,
                 "code-bias bare records");
    TEST_NESTED_2(sidereon_rtcm_message_ssr_code_bias_signals, messages, 0, 0,
                  SidereonRtcmSsrCodeBiasSignal, 2, "code-bias framed signals");
    TEST_NESTED_1(sidereon_ssr_message_code_bias_signals, bare, 0,
                  SidereonRtcmSsrCodeBiasSignal, 2, "code-bias bare signals");

    SidereonRtcmSsrCodeBiasRecord framed_record;
    SidereonRtcmSsrCodeBiasRecord bare_record;
    size_t written = 0;
    size_t required = 0;
    check_status(sidereon_rtcm_message_ssr_code_biases(messages, 0, &framed_record, 1, &written,
                                                       &required),
                 SIDEREON_STATUS_OK, "code-bias framed record copy");
    check_status(sidereon_ssr_message_code_biases(bare, &bare_record, 1, &written, &required),
                 SIDEREON_STATUS_OK, "code-bias bare record copy");
    check(framed_record.satellite_id == bare_record.satellite_id &&
              framed_record.signal_count == bare_record.signal_count &&
              bare_record.satellite_id == 3 && bare_record.signal_count == 2,
          "code-bias record fields exact equality");

    SidereonRtcmSsrCodeBiasSignal framed_signals[2];
    SidereonRtcmSsrCodeBiasSignal bare_signals[2];
    check_status(sidereon_rtcm_message_ssr_code_bias_signals(messages, 0, 0, framed_signals, 2,
                                                             &written, &required),
                 SIDEREON_STATUS_OK, "code-bias framed signal copy");
    check_status(sidereon_ssr_message_code_bias_signals(bare, 0, bare_signals, 2, &written,
                                                        &required),
                 SIDEREON_STATUS_OK, "code-bias bare signal copy");
    check(framed_signals[0].signal_id == bare_signals[0].signal_id &&
              framed_signals[0].bias == bare_signals[0].bias &&
              framed_signals[1].signal_id == bare_signals[1].signal_id &&
              framed_signals[1].bias == bare_signals[1].bias &&
              bare_signals[0].signal_id == 1 && bare_signals[0].bias == -1234 &&
              bare_signals[1].signal_id == 9 && bare_signals[1].bias == 2345,
          "code-bias raw signed fields exact equality");

    written = 41;
    required = 42;
    check_status(sidereon_ssr_message_code_bias_signals(bare, 4, bare_signals, 2, &written,
                                                        &required),
                 SIDEREON_STATUS_INVALID_ARGUMENT, "code-bias bad record index");
    check(written == 0 && required == 0, "code-bias bad index counts");
    check_error_present("code-bias bad index error");
    check_status(sidereon_ssr_message_info(bare, NULL), SIDEREON_STATUS_NULL_POINTER,
                 "code-bias null info out");
    check_error_present("code-bias null info error");

    sidereon_rtcm_messages_free(messages);
    sidereon_ssr_message_free(bare);
}

static void compare_phase(const uint8_t *body, size_t body_len) {
    uint8_t frame[256];
    size_t frame_len = 0;
    make_frame(body, body_len, frame, sizeof(frame), &frame_len, "phase-bias frame encode");
    SidereonRtcmMessages *messages = NULL;
    SidereonSsrMessage *bare = NULL;
    check_status(sidereon_rtcm_decode_messages(frame, frame_len, &messages), SIDEREON_STATUS_OK,
                 "phase-bias framed decode");
    check_status(sidereon_ssr_message_decode(body, body_len, &bare), SIDEREON_STATUS_OK,
                 "phase-bias bare decode");
    check(messages != NULL && bare != NULL, "phase-bias handles");
    if (messages == NULL || bare == NULL) {
        sidereon_rtcm_messages_free(messages);
        sidereon_ssr_message_free(bare);
        return;
    }

    SidereonRtcmSsrInfo framed_info;
    SidereonRtcmSsrInfo bare_info;
    check_status(sidereon_rtcm_message_ssr_info(messages, 0, &framed_info), SIDEREON_STATUS_OK,
                 "phase-bias framed info");
    check_status(sidereon_ssr_message_info(bare, &bare_info), SIDEREON_STATUS_OK,
                 "phase-bias bare info");
    check(info_equal(&framed_info, &bare_info), "phase-bias info exact equality");
    check_info_common(&bare_info, 1265, SIDEREON_RTCM_SSR_KIND_PHASE_BIAS, 0, 0, 0, 0, 1,
                      "phase-bias expected info");
    check(bare_info.header.has_dispersive_bias_consistency &&
              bare_info.header.dispersive_bias_consistency &&
              bare_info.header.has_mw_consistency && !bare_info.header.mw_consistency,
          "phase-bias optional header flags");

    TEST_ARRAY_2(sidereon_rtcm_message_ssr_phase_biases, messages, 0,
                 SidereonRtcmSsrPhaseBiasRecord, 1, "phase-bias framed records");
    TEST_ARRAY_1(sidereon_ssr_message_phase_biases, bare, SidereonRtcmSsrPhaseBiasRecord, 1,
                 "phase-bias bare records");
    TEST_NESTED_2(sidereon_rtcm_message_ssr_phase_bias_signals, messages, 0, 0,
                  SidereonRtcmSsrPhaseBiasSignal, 2, "phase-bias framed signals");
    TEST_NESTED_1(sidereon_ssr_message_phase_bias_signals, bare, 0,
                  SidereonRtcmSsrPhaseBiasSignal, 2, "phase-bias bare signals");

    SidereonRtcmSsrPhaseBiasRecord framed_record;
    SidereonRtcmSsrPhaseBiasRecord bare_record;
    size_t written = 0;
    size_t required = 0;
    check_status(sidereon_rtcm_message_ssr_phase_biases(messages, 0, &framed_record, 1, &written,
                                                        &required),
                 SIDEREON_STATUS_OK, "phase-bias framed record copy");
    check_status(sidereon_ssr_message_phase_biases(bare, &bare_record, 1, &written, &required),
                 SIDEREON_STATUS_OK, "phase-bias bare record copy");
    check(framed_record.satellite_id == bare_record.satellite_id &&
              framed_record.yaw_angle == bare_record.yaw_angle &&
              framed_record.yaw_rate == bare_record.yaw_rate &&
              framed_record.signal_count == bare_record.signal_count &&
              bare_record.satellite_id == 3 && bare_record.yaw_angle == 127 &&
              bare_record.yaw_rate == -12 && bare_record.signal_count == 2,
          "phase-bias record raw fields exact equality");

    SidereonRtcmSsrPhaseBiasSignal framed_signals[2];
    SidereonRtcmSsrPhaseBiasSignal bare_signals[2];
    check_status(sidereon_rtcm_message_ssr_phase_bias_signals(messages, 0, 0, framed_signals, 2,
                                                              &written, &required),
                 SIDEREON_STATUS_OK, "phase-bias framed signal copy");
    check_status(sidereon_ssr_message_phase_bias_signals(bare, 0, bare_signals, 2, &written,
                                                         &required),
                 SIDEREON_STATUS_OK, "phase-bias bare signal copy");
    check(framed_signals[0].signal_id == bare_signals[0].signal_id &&
              framed_signals[0].integer_indicator == bare_signals[0].integer_indicator &&
              framed_signals[0].wide_lane_integer_indicator ==
                  bare_signals[0].wide_lane_integer_indicator &&
              framed_signals[0].discontinuity_counter == bare_signals[0].discontinuity_counter &&
              framed_signals[0].bias == bare_signals[0].bias &&
              framed_signals[1].signal_id == bare_signals[1].signal_id &&
              framed_signals[1].integer_indicator == bare_signals[1].integer_indicator &&
              framed_signals[1].wide_lane_integer_indicator ==
                  bare_signals[1].wide_lane_integer_indicator &&
              framed_signals[1].discontinuity_counter == bare_signals[1].discontinuity_counter &&
              framed_signals[1].bias == bare_signals[1].bias &&
              bare_signals[0].signal_id == 1 && bare_signals[0].integer_indicator == 1 &&
              bare_signals[0].wide_lane_integer_indicator == 2 &&
              bare_signals[0].discontinuity_counter == 3 && bare_signals[0].bias == -123456 &&
              bare_signals[1].signal_id == 9 && bare_signals[1].integer_indicator == 0 &&
              bare_signals[1].wide_lane_integer_indicator == 1 &&
              bare_signals[1].discontinuity_counter == 4 && bare_signals[1].bias == 234567,
          "phase-bias raw signed and indicator fields exact equality");

    written = 41;
    required = 42;
    check_status(sidereon_ssr_message_phase_bias_signals(bare, 4, bare_signals, 2, &written,
                                                         &required),
                 SIDEREON_STATUS_INVALID_ARGUMENT, "phase-bias bad record index");
    check(written == 0 && required == 0, "phase-bias bad index counts");
    check_error_present("phase-bias bad index error");
    check_status(sidereon_rtcm_message_ssr_phase_biases(messages, 0, NULL, 0, NULL, &required),
                 SIDEREON_STATUS_NULL_POINTER, "phase-bias null out_written");
    check_error_present("phase-bias null out_written error");

    sidereon_rtcm_messages_free(messages);
    sidereon_ssr_message_free(bare);
}

static void compare_ura(const uint8_t *body, size_t body_len) {
    uint8_t frame[256];
    size_t frame_len = 0;
    make_frame(body, body_len, frame, sizeof(frame), &frame_len, "URA frame encode");
    SidereonRtcmMessages *messages = NULL;
    SidereonSsrMessage *bare = NULL;
    check_status(sidereon_rtcm_decode_messages(frame, frame_len, &messages), SIDEREON_STATUS_OK,
                 "URA framed decode");
    check_status(sidereon_ssr_message_decode(body, body_len, &bare), SIDEREON_STATUS_OK,
                 "URA bare decode");
    check(messages != NULL && bare != NULL, "URA handles");
    if (messages == NULL || bare == NULL) {
        sidereon_rtcm_messages_free(messages);
        sidereon_ssr_message_free(bare);
        return;
    }

    SidereonRtcmSsrInfo framed_info;
    SidereonRtcmSsrInfo bare_info;
    check_status(sidereon_rtcm_message_ssr_info(messages, 0, &framed_info), SIDEREON_STATUS_OK,
                 "URA framed info");
    check_status(sidereon_ssr_message_info(bare, &bare_info), SIDEREON_STATUS_OK,
                 "URA bare info");
    check(info_equal(&framed_info, &bare_info), "URA info exact equality");
    check_info_common(&bare_info, 1061, SIDEREON_RTCM_SSR_KIND_URA, 0, 0, 1, 0, 0,
                      "URA expected info");
    check(!bare_info.header.has_satellite_reference_datum &&
              !bare_info.header.has_dispersive_bias_consistency &&
              !bare_info.header.has_mw_consistency,
          "URA optional header flags");

    TEST_ARRAY_2(sidereon_rtcm_message_ssr_ura, messages, 0, SidereonRtcmSsrUraRecord, 1,
                 "URA framed records");
    TEST_ARRAY_1(sidereon_ssr_message_ura, bare, SidereonRtcmSsrUraRecord, 1,
                 "URA bare records");
    SidereonRtcmSsrUraRecord framed_record;
    SidereonRtcmSsrUraRecord bare_record;
    size_t written = 0;
    size_t required = 0;
    check_status(sidereon_rtcm_message_ssr_ura(messages, 0, &framed_record, 1, &written, &required),
                 SIDEREON_STATUS_OK, "URA framed record copy");
    check_status(sidereon_ssr_message_ura(bare, &bare_record, 1, &written, &required),
                 SIDEREON_STATUS_OK, "URA bare record copy");
    check(framed_record.satellite_id == bare_record.satellite_id &&
              framed_record.ura_index == bare_record.ura_index && bare_record.satellite_id == 3 &&
              bare_record.ura_index == 41,
          "URA fields exact equality");
    sidereon_rtcm_messages_free(messages);
    sidereon_ssr_message_free(bare);
}

static void test_failures(const uint8_t *valid_body, size_t valid_body_len) {
    uint8_t invalid_body[1] = {0};
    SidereonSsrMessage *message = (SidereonSsrMessage *)(uintptr_t)1;
    check_status(sidereon_ssr_message_decode(invalid_body, sizeof(invalid_body), &message),
                 SIDEREON_STATUS_SP3_PARSE, "invalid bare SSR decode");
    check(message == NULL, "invalid bare SSR clears output");
    check_error_present("invalid bare SSR error");
    check_status(sidereon_ssr_message_decode(valid_body, valid_body_len, NULL),
                 SIDEREON_STATUS_NULL_POINTER, "bare SSR null output handle");
    check_error_present("bare SSR null output error");
    sidereon_ssr_message_free(NULL);
    sidereon_rtcm_messages_free(NULL);
}

int main(void) {
    /*
     * This combined GPS 1060 frame literal is copied from phaseb_smoke.c,
     * the repository's committed SSR fixture. The body used for the bare
     * route is derived at runtime through sidereon_rtcm_decode_frame.
     */
    static const char *const combined_frame_hex =
        "d3003c4245438a3040000827968003270026dffea30000f7fff6ffff0000530000000000003e87fff8effc94002c7ffff57fffc80003004128000000000000625cf0";

    /*
     * These are the encoded bodies of the published sidereon-core 1.2.0
     * src/rtcm/ssr.rs test helper's SsrMessage values: code biases
     * [(1, -1234), (9, 2345)] and phase biases with yaw 127/-12 and signal
     * rows (1,1,2,3,-123456), (9,0,1,4,234567). The URA body uses the same
     * helper header and its test value (satellite 3, URA 41). They are frozen
     * public-fixture bytes, not binding-side parsing or mapping logic.
     */
    static const char *const code_body_hex = "423546002c803da021883d97249290";
    static const char *const phase_body_hex = "4f1546002c803da408623ffa071f0ee024a1ca2380";
    static const char *const ura_body_hex = "425546002c803da021d2";

    uint8_t combined_frame[256];
    uint8_t code_body[256];
    uint8_t phase_body[256];
    uint8_t ura_body[256];
    size_t combined_frame_len = hex_to_bytes(combined_frame_hex, combined_frame,
                                             sizeof(combined_frame));
    size_t code_body_len = hex_to_bytes(code_body_hex, code_body, sizeof(code_body));
    size_t phase_body_len = hex_to_bytes(phase_body_hex, phase_body, sizeof(phase_body));
    size_t ura_body_len = hex_to_bytes(ura_body_hex, ura_body, sizeof(ura_body));
    check(combined_frame_len > 0 && code_body_len == 15 && phase_body_len == 21 &&
              ura_body_len == 10,
          "SSR fixture hex decode");

    compare_combined(combined_frame, combined_frame_len);
    compare_code(code_body, code_body_len);
    compare_phase(phase_body, phase_body_len);
    compare_ura(ura_body, ura_body_len);
    test_failures(phase_body, phase_body_len);

    if (failures != 0) {
        fprintf(stderr, "ssr_message_smoke: %d failure(s)\n", failures);
        return 1;
    }
    puts("ssr_message_smoke: ok");
    return 0;
}
