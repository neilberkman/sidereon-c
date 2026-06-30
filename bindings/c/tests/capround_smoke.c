/*
 * Smoke coverage for the capability-parity round added to the C binding:
 * Galileo NeQuick-G ionosphere, rv<->COE element conversions, observational
 * geometry (sub-solar / terminator / parallactic angle / visual magnitude /
 * sub-observer point), geoid undulation + orthometric/ellipsoidal height (built
 * in grid and a loaded GeoidGrid), Instant::from_utc_civil, moving-baseline RTK,
 * and RTCM 3 decode/encode (typed message accessors + framing). Every call
 * delegates to sidereon-core; this program only checks the FFI marshaling and
 * that the engine produces sane numbers.
 *
 * The RTCM stream below was generated from sidereon-core's own encoder (one
 * 1006, 1008, 1019, 1020, and 1077 message, each framed with a real CRC-24Q) so
 * the decode path is exercised against bytes the engine itself produced.
 *
 * No argv; this program is fully self-contained.
 */
#include <math.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sidereon.h"

static int failures = 0;

static void check(int ok, const char *what) {
    if (!ok) {
        char msg[512];
        size_t n = sidereon_last_error_message((char *)msg, sizeof(msg));
        if (n == 0) {
            msg[0] = '\0';
        }
        fprintf(stderr, "FAIL: %s (last_error: %s)\n", what, msg);
        failures++;
    }
}

static const unsigned char RTCM_STREAM[] = {
    0xd3, 0x00, 0x15, 0x3e, 0xe7, 0xd3, 0x03, 0x02, 0xaa, 0x3c, 0x6d, 0x18,
    0x3e, 0x46, 0x05, 0xff, 0x0c, 0x02, 0xef, 0x2b, 0x54, 0x84, 0x3a, 0x98,
    0xd8, 0xb4, 0x87, 0xd3, 0x00, 0x1b, 0x3f, 0x07, 0xd3, 0x0b, 0x54, 0x52,
    0x4d, 0x35, 0x39, 0x38, 0x30, 0x30, 0x2e, 0x30, 0x30, 0x01, 0x0a, 0x31,
    0x34, 0x34, 0x30, 0x38, 0x31, 0x32, 0x33, 0x34, 0x35, 0x63, 0xb9, 0x0f,
    0xd3, 0x00, 0x3d, 0x3f, 0xb2, 0x07, 0xb0, 0x7f, 0xfb, 0x2a, 0x03, 0xe8,
    0x00, 0xff, 0xfd, 0x00, 0xc0, 0xe4, 0x2a, 0xff, 0x91, 0x00, 0xde, 0xff,
    0xed, 0x29, 0x79, 0xff, 0xce, 0x00, 0x3d, 0x09, 0x00, 0x00, 0x3c, 0xa0,
    0xee, 0xbb, 0x00, 0x03, 0xe8, 0x00, 0x07, 0x00, 0x08, 0x7a, 0x23, 0xff,
    0xf7, 0x00, 0x0f, 0x42, 0x3f, 0x01, 0x4d, 0xff, 0xf4, 0x21, 0xcf, 0xff,
    0xfc, 0x00, 0xfd, 0x00, 0x76, 0x82, 0x1c, 0xd3, 0x00, 0x2d, 0x3f, 0xc1,
    0x51, 0xa0, 0xc8, 0x9e, 0x80, 0x04, 0xd2, 0x00, 0x02, 0xc5, 0xc1, 0x00,
    0x10, 0xe1, 0x80, 0x04, 0x47, 0xb1, 0x00, 0x00, 0xde, 0x80, 0x00, 0x29,
    0xa0, 0x00, 0xc5, 0x00, 0x01, 0xb8, 0x87, 0x45, 0x78, 0xd2, 0xc4, 0x00,
    0x0f, 0x12, 0x01, 0x60, 0x00, 0x2a, 0x00, 0x20, 0xe3, 0x6b, 0xd3, 0x00,
    0x24, 0x43, 0x57, 0xd3, 0x00, 0x07, 0x89, 0x00, 0x00, 0x00, 0x00, 0x80,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x51, 0x82,
    0x00, 0xfe, 0x70, 0x0c, 0x0e, 0x7f, 0xfd, 0xb5, 0xc6, 0x44, 0xb0, 0x00,
    0x14, 0x43, 0x2c, 0x82,
};
static const size_t RTCM_STREAM_LEN = 220;

static void test_nequick(void) {
    double delay = -1.0;
    /* Zero broadcast coefficients select the Galileo default ionisation level;
     * the delay must be finite and positive for a satellite above the horizon. */
    check(sidereon_galileo_nequick_g_native(0.0, 0.0, 0.0, 45.0, 9.0, 30.0,
                                            43200.0, 80.0, 1.57542e9, &delay) ==
              SIDEREON_STATUS_OK &&
              delay > 0.0 && isfinite(delay),
          "galileo_nequick_g_native default coeffs");

    double delay2 = -1.0;
    check(sidereon_galileo_nequick_g_native(80.0, 0.1, 0.05, 45.0, 9.0, 30.0,
                                            43200.0, 80.0, 1.57542e9, &delay2) ==
              SIDEREON_STATUS_OK &&
              delay2 > 0.0,
          "galileo_nequick_g_native broadcast coeffs");
}

static void test_elements(void) {
    /* A near-circular LEO state; rv2coe then coe2rv must round-trip. */
    double r[3] = {-6045.0, -3490.0, 2500.0};
    double v[3] = {-3.457, 6.618, 2.533};
    const double mu = 398600.4418;

    SidereonClassicalElements coe;
    check(sidereon_rv2coe(r, v, mu, &coe) == SIDEREON_STATUS_OK && coe.ecc >= 0.0 &&
              coe.a > 0.0,
          "rv2coe");

    double r2[3] = {0.0, 0.0, 0.0};
    double v2[3] = {0.0, 0.0, 0.0};
    check(sidereon_coe2rv(&coe, mu, r2, v2) == SIDEREON_STATUS_OK, "coe2rv");

    double dr = 0.0, dv = 0.0;
    for (int i = 0; i < 3; i++) {
        dr += (r[i] - r2[i]) * (r[i] - r2[i]);
        dv += (v[i] - v2[i]) * (v[i] - v2[i]);
    }
    check(sqrt(dr) < 1e-6 && sqrt(dv) < 1e-9, "rv2coe/coe2rv round-trip");
}

static void test_observation(void) {
    /* Sun straight down the +x axis: sub-solar point on the equator at lon 0. */
    double sun[3] = {1.4959787e11, 0.0, 0.0};
    SidereonSurfacePoint sp;
    check(sidereon_sub_solar_point(sun, &sp) == SIDEREON_STATUS_OK &&
              fabs(sp.latitude_deg) < 1e-9 && fabs(sp.longitude_deg) < 1e-9,
          "sub_solar_point");

    double termlat = 999.0;
    check(sidereon_terminator_latitude_deg(sp.latitude_deg, sp.longitude_deg, 90.0,
                                           &termlat) == SIDEREON_STATUS_OK &&
              isfinite(termlat),
          "terminator_latitude_deg");

    double q = 999.0;
    check(sidereon_parallactic_angle_deg(40.0, 0.0, 20.0, &q) == SIDEREON_STATUS_OK &&
              fabs(q) < 1e-9,
          "parallactic_angle_deg on the meridian");

    double mag = 999.0;
    check(sidereon_satellite_visual_magnitude(1000.0, 0.0, 5.0, 1000.0, &mag) ==
              SIDEREON_STATUS_OK &&
              fabs(mag - 5.0) < 1e-9,
          "satellite_visual_magnitude at reference range, zero phase");

    double obs[3] = {1.0, 0.0, 0.0};
    SidereonSurfacePoint sub;
    check(sidereon_sub_observer_point(obs, 0.0, 90.0, 0.0, &sub) == SIDEREON_STATUS_OK &&
              isfinite(sub.latitude_deg) && isfinite(sub.longitude_deg),
          "sub_observer_point");
}

static void test_geoid(void) {
    double n = 999.0;
    check(sidereon_geoid_undulation(0.7, 0.1, &n) == SIDEREON_STATUS_OK && isfinite(n),
          "geoid_undulation");

    double ortho = 0.0, ellip = 0.0;
    check(sidereon_orthometric_height_m(100.0, 0.7, 0.1, &ortho) == SIDEREON_STATUS_OK,
          "orthometric_height_m");
    check(sidereon_ellipsoidal_height_m(ortho, 0.7, 0.1, &ellip) == SIDEREON_STATUS_OK &&
              fabs(ellip - 100.0) < 1e-9,
          "ellipsoidal_height_m round-trip");

    /* A tiny 2x2 regional grid loaded from text; bilinear midpoint is the mean. */
    const char *grid_text =
        "# lat_min lon_min dlat dlon n_lat n_lon\n"
        "0 0 1 1 2 2\n"
        "0 10 20 30\n";
    SidereonGeoidGrid *grid = NULL;
    check(sidereon_geoid_grid_from_text((const uint8_t *)grid_text, strlen(grid_text),
                                        &grid) == SIDEREON_STATUS_OK &&
              grid != NULL,
          "geoid_grid_from_text");
    if (grid) {
        double mid = -1.0;
        check(sidereon_geoid_grid_undulation_deg(grid, 0.5, 0.5, &mid) ==
                  SIDEREON_STATUS_OK &&
                  fabs(mid - 15.0) < 1e-9,
              "geoid_grid_undulation_deg midpoint");
        double midr = -1.0;
        check(sidereon_geoid_grid_undulation_rad(grid, 0.5 * M_PI / 180.0,
                                                 0.5 * M_PI / 180.0, &midr) ==
                  SIDEREON_STATUS_OK &&
                  fabs(midr - mid) < 1e-9,
              "geoid_grid_undulation_rad matches deg");
        sidereon_geoid_grid_free(grid);
    }

    /* Build the same grid from samples directly. */
    double values[4] = {0.0, 10.0, 20.0, 30.0};
    SidereonGeoidGrid *grid2 = NULL;
    check(sidereon_geoid_grid_new(0.0, 0.0, 1.0, 1.0, 2, 2, values, 4, &grid2) ==
              SIDEREON_STATUS_OK,
          "geoid_grid_new");
    sidereon_geoid_grid_free(grid2);
}

static void test_instant(void) {
    double jd_whole = 0.0, jd_fraction = 0.0, j2000 = 0.0;
    check(sidereon_instant_from_utc_civil(2020, 6, 25, 12, 0, 0.0, &jd_whole,
                                          &jd_fraction, &j2000) == SIDEREON_STATUS_OK,
          "instant_from_utc_civil");
    /* 2020-06-25 is well after J2000 (2000-01-01); the JD should be near 2.459e6. */
    check(jd_whole + jd_fraction > 2.459e6 && jd_whole + jd_fraction < 2.46e6 &&
              isfinite(j2000),
          "instant_from_utc_civil produces a sane Julian date");
}

/* One double-difference epoch geometry shared by the moving-baseline test. */
typedef struct {
    const char *id;
    double pos[3];
    int64_t cycles;
} MbSat;

static void test_moving_baseline(void) {
    const double c = 299792458.0;
    const double f_l1 = 1575420000.0;
    const double lambda = c / f_l1;

    const MbSat sats[5] = {
        {"G01", {15000000.0, 7000000.0, 21000000.0}, 0},
        {"G02", {-12000000.0, 18000000.0, 19000000.0}, 4},
        {"G03", {20000000.0, -10000000.0, 17000000.0}, -7},
        {"G04", {-19000000.0, -13000000.0, 20000000.0}, 9},
        {"G05", {9000000.0, 22000000.0, 16000000.0}, -3},
    };
    double base[3] = {-2700000.0, -4300000.0, 3850000.0};
    double baseline[3] = {12.0, -7.0, 5.0};
    double rover[3] = {base[0] + baseline[0], base[1] + baseline[1], base[2] + baseline[2]};

    SidereonRtkSatMeasurement refrow;
    SidereonRtkSatMeasurement nonrow[4];
    SidereonRtkSatMeasurement *rows[5] = {&refrow, &nonrow[0], &nonrow[1], &nonrow[2],
                                          &nonrow[3]};
    char ids[5][8];
    for (int i = 0; i < 5; i++) {
        SidereonRtkSatMeasurement *row = rows[i];
        memset(row, 0, sizeof(*row));
        snprintf(ids[i], sizeof(ids[i]), "%s", sats[i].id);
        row->sat_id = ids[i];
        row->sd_ambiguity_id = ids[i];
        double db = 0.0, dr = 0.0;
        for (int k = 0; k < 3; k++) {
            db += (sats[i].pos[k] - base[k]) * (sats[i].pos[k] - base[k]);
            dr += (sats[i].pos[k] - rover[k]) * (sats[i].pos[k] - rover[k]);
        }
        db = sqrt(db);
        dr = sqrt(dr);
        row->base_code_m = db;
        row->base_phase_m = db;
        row->rover_code_m = dr;
        row->rover_phase_m = dr + (double)sats[i].cycles * lambda;
        for (int k = 0; k < 3; k++) {
            row->base_tx_pos[k] = sats[i].pos[k];
            row->rover_tx_pos[k] = sats[i].pos[k];
            row->pos[k] = sats[i].pos[k];
        }
    }

    SidereonRtkEpoch epoch;
    memset(&epoch, 0, sizeof(epoch));
    epoch.references = &refrow;
    epoch.reference_count = 1;
    epoch.nonref = nonrow;
    epoch.nonref_count = 4;
    epoch.has_velocity_mps = false;
    epoch.dt_s = 0.0;

    const char *amb_ids[4] = {"G02", "G03", "G04", "G05"};
    SidereonRtkAmbiguitySatellite amb_sats[4];
    SidereonRtkFloatMapEntry wavelengths[4];
    SidereonRtkFloatMapEntry offsets[4];
    for (int i = 0; i < 4; i++) {
        amb_sats[i].id = amb_ids[i];
        amb_sats[i].sat_id = amb_ids[i];
        wavelengths[i].id = amb_ids[i];
        wavelengths[i].value = lambda;
        offsets[i].id = amb_ids[i];
        offsets[i].value = 0.0;
    }

    SidereonRtkMeasurementModel model;
    sidereon_rtk_measurement_model_init(&model);
    SidereonRtkFloatOptions fopts;
    sidereon_rtk_float_options_init(&fopts);
    SidereonRtkFixedOptions xopts;
    sidereon_rtk_fixed_options_init(&xopts);

    SidereonMovingBaselineEpoch mb_epoch;
    memset(&mb_epoch, 0, sizeof(mb_epoch));
    for (int k = 0; k < 3; k++) {
        mb_epoch.base_position_m[k] = base[k];
    }
    mb_epoch.epoch = epoch;
    mb_epoch.ambiguity_ids = amb_ids;
    mb_epoch.ambiguity_id_count = 4;
    mb_epoch.ambiguity_satellites = amb_sats;
    mb_epoch.ambiguity_satellite_count = 4;
    mb_epoch.wavelengths_m = wavelengths;
    mb_epoch.wavelength_count = 4;
    mb_epoch.offsets_m = offsets;
    mb_epoch.offset_count = 4;
    mb_epoch.float_only_systems = NULL;
    mb_epoch.float_only_system_count = 0;

    SidereonMovingBaselineConfig config;
    memset(&config, 0, sizeof(config));
    config.epochs = &mb_epoch;
    config.epoch_count = 1;
    config.model = model;
    config.float_options = fopts;
    config.fixed_options = xopts;
    config.warm_start = false;
    config.receiver_antenna = NULL;

    SidereonMovingBaselineSolution *sol = NULL;
    check(sidereon_solve_moving_baseline(&config, &sol) == SIDEREON_STATUS_OK &&
              sol != NULL,
          "solve_moving_baseline");
    if (sol) {
        size_t n = 0;
        check(sidereon_moving_baseline_solution_epoch_count(sol, &n) ==
                  SIDEREON_STATUS_OK &&
                  n == 1,
              "moving_baseline epoch count");
        SidereonMovingBaselineEpochSummary summary;
        check(sidereon_moving_baseline_solution_epoch(sol, 0, &summary) ==
                  SIDEREON_STATUS_OK,
              "moving_baseline epoch summary");
        /* The perfect synthetic geometry recovers the planted baseline closely. */
        double err = 0.0;
        for (int k = 0; k < 3; k++) {
            err += (summary.baseline_m[k] - baseline[k]) * (summary.baseline_m[k] - baseline[k]);
        }
        check(sqrt(err) < 0.5 && summary.baseline_length_m > 0.0,
              "moving_baseline recovers the planted baseline");
        sidereon_moving_baseline_solution_free(sol);
    }
}

static void test_rtcm(void) {
    SidereonRtcmMessages *messages = NULL;
    check(sidereon_rtcm_decode_messages(RTCM_STREAM, RTCM_STREAM_LEN, &messages) ==
              SIDEREON_STATUS_OK &&
              messages != NULL,
          "rtcm_decode_messages");
    if (!messages) {
        return;
    }

    size_t count = 0;
    check(sidereon_rtcm_messages_count(messages, &count) == SIDEREON_STATUS_OK &&
              count == 5,
          "rtcm_messages_count");

    /* Message 0: 1006 station coordinates. */
    SidereonRtcmMessageKind kind;
    uint16_t number = 0;
    check(sidereon_rtcm_message_kind(messages, 0, &kind, &number) == SIDEREON_STATUS_OK &&
              kind == SIDEREON_RTCM_MESSAGE_KIND_STATION_COORDINATES && number == 1006,
          "rtcm_message_kind 1006");
    SidereonRtcmStationCoordinates station;
    check(sidereon_rtcm_message_station_coordinates(messages, 0, &station) ==
              SIDEREON_STATUS_OK &&
              station.reference_station_id == 2003 && station.has_antenna_height &&
              fabs(station.antenna_height_m - 1.5) < 1e-9 && fabs(station.x_m) > 1000.0,
          "rtcm station coordinates fields");

    /* Message 1: 1008 antenna descriptor (with strings). */
    check(sidereon_rtcm_message_kind(messages, 1, &kind, &number) == SIDEREON_STATUS_OK &&
              kind == SIDEREON_RTCM_MESSAGE_KIND_ANTENNA_DESCRIPTOR && number == 1008,
          "rtcm_message_kind 1008");
    SidereonRtcmAntennaDescriptor antenna;
    check(sidereon_rtcm_message_antenna_descriptor(messages, 1, &antenna) ==
              SIDEREON_STATUS_OK &&
              antenna.has_antenna_serial_number && !antenna.has_receiver_type,
          "rtcm antenna descriptor fields");
    char descriptor[64];
    size_t written = 0, required = 0;
    check(sidereon_rtcm_message_antenna_string(
              messages, 1, SIDEREON_RTCM_ANTENNA_STRING_FIELD_ANTENNA_DESCRIPTOR,
              (uint8_t *)descriptor, sizeof(descriptor), &written, &required) ==
              SIDEREON_STATUS_OK &&
              required == strlen("TRM59800.00") && written == required,
          "rtcm antenna descriptor string");
    descriptor[written] = '\0';
    check(strcmp(descriptor, "TRM59800.00") == 0, "rtcm antenna descriptor string value");

    /* Message 2: 1019 GPS ephemeris. */
    SidereonRtcmGpsEphemeris gps;
    check(sidereon_rtcm_message_gps_ephemeris(messages, 2, &gps) == SIDEREON_STATUS_OK &&
              gps.satellite_id == 8 && gps.week_number == 123 && gps.a_f0 == 12345,
          "rtcm gps ephemeris fields");

    /* Message 3: 1020 GLONASS ephemeris. */
    SidereonRtcmGlonassEphemeris glo;
    check(sidereon_rtcm_message_glonass_ephemeris(messages, 3, &glo) ==
              SIDEREON_STATUS_OK &&
              glo.satellite_id == 5 && glo.frequency_channel == 8 && glo.m_n_t == 700,
          "rtcm glonass ephemeris fields");

    /* Message 4: 1077 GPS MSM7 observation. */
    SidereonRtcmMsmInfo msm;
    check(sidereon_rtcm_message_msm_info(messages, 4, &msm) == SIDEREON_STATUS_OK &&
              msm.message_number == 1077 && msm.kind == SIDEREON_RTCM_MSM_KIND_MSM7 &&
              msm.system == SIDEREON_GNSS_SYSTEM_GPS && msm.satellite_count == 1 &&
              msm.signal_count == 1 && msm.header.reference_station_id == 2003,
          "rtcm msm info");
    SidereonRtcmMsmSatellite msm_sats[4];
    written = 0;
    required = 0;
    check(sidereon_rtcm_message_msm_satellites(messages, 4, msm_sats, 4, &written,
                                               &required) == SIDEREON_STATUS_OK &&
              required == 1 && written == 1 && msm_sats[0].id == 8 &&
              msm_sats[0].has_extended_info,
          "rtcm msm satellites");
    SidereonRtcmMsmSignal msm_sigs[4];
    written = 0;
    required = 0;
    check(sidereon_rtcm_message_msm_signals(messages, 4, msm_sigs, 4, &written,
                                            &required) == SIDEREON_STATUS_OK &&
              required == 1 && written == 1 && msm_sigs[0].signal_id == 2 &&
              msm_sigs[0].has_fine_phase_range_rate,
          "rtcm msm signals");

    /* Re-encode message 0 to a frame and confirm it matches the first 27 bytes of
     * the input stream (the 1006 frame), proving the encode path round-trips. */
    uint8_t reframe[64];
    written = 0;
    required = 0;
    check(sidereon_rtcm_message_to_frame(messages, 0, reframe, sizeof(reframe), &written,
                                         &required) == SIDEREON_STATUS_OK &&
              written == required && required == 27 &&
              memcmp(reframe, RTCM_STREAM, 27) == 0,
          "rtcm message_to_frame round-trips the 1006 frame");

    /* Message-body encode (without the frame) for message 0. */
    uint8_t body[64];
    written = 0;
    required = 0;
    check(sidereon_rtcm_message_encode(messages, 0, body, sizeof(body), &written,
                                       &required) == SIDEREON_STATUS_OK &&
              written == required && required == 21,
          "rtcm message_encode body length");

    sidereon_rtcm_messages_free(messages);

    /* Frame scanner over the whole stream: five frames, first frame_len 27. */
    SidereonRtcmFrames *frames = NULL;
    check(sidereon_rtcm_scan_frames(RTCM_STREAM, RTCM_STREAM_LEN, &frames) ==
              SIDEREON_STATUS_OK &&
              frames != NULL,
          "rtcm_scan_frames");
    if (frames) {
        size_t fcount = 0;
        check(sidereon_rtcm_frames_count(frames, &fcount) == SIDEREON_STATUS_OK &&
                  fcount == 5,
              "rtcm_frames_count");
        size_t flen = 0;
        check(sidereon_rtcm_frame_len(frames, 0, &flen) == SIDEREON_STATUS_OK &&
                  flen == 27,
              "rtcm_frame_len");
        uint8_t fbody[64];
        written = 0;
        required = 0;
        check(sidereon_rtcm_frame_body(frames, 0, fbody, sizeof(fbody), &written,
                                       &required) == SIDEREON_STATUS_OK &&
                  required == 21,
              "rtcm_frame_body");
        sidereon_rtcm_frames_free(frames);
    }

    /* encode_frame + decode_frame on an arbitrary body round-trips byte-exactly. */
    const uint8_t payload[5] = {0x10, 0x20, 0x30, 0x40, 0x50};
    uint8_t frame[16];
    written = 0;
    required = 0;
    check(sidereon_rtcm_encode_frame(payload, sizeof(payload), frame, sizeof(frame),
                                     &written, &required) == SIDEREON_STATUS_OK &&
              written == required && required == sizeof(payload) + 6,
          "rtcm_encode_frame");
    uint8_t out_body[16];
    size_t body_written = 0, body_required = 0, frame_len = 0;
    check(sidereon_rtcm_decode_frame(frame, written, out_body, sizeof(out_body),
                                     &body_written, &body_required, &frame_len) ==
              SIDEREON_STATUS_OK &&
              body_required == sizeof(payload) && body_written == sizeof(payload) &&
              frame_len == written && memcmp(out_body, payload, sizeof(payload)) == 0,
          "rtcm_decode_frame round-trips the body");
}

int main(void) {
    test_nequick();
    test_elements();
    test_observation();
    test_geoid();
    test_instant();
    test_moving_baseline();
    test_rtcm();

    if (failures != 0) {
        fprintf(stderr, "capround_smoke: %d failure(s)\n", failures);
        return 1;
    }
    printf("capround_smoke: all checks passed\n");
    return 0;
}
