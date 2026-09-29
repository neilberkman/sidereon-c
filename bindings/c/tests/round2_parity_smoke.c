/*
 * Round-2 parity smoke for local-core additions in the C binding:
 * covariance transport/propagation, CNAV/RINEX-4 record evaluation, SGP4 TLE
 * fitting, observation QC/lint/repair, EGM96/geoid batches, NMEA
 * parse/accumulate/GGA writing, space-weather tables, and NTRIP sans-IO.
 *
 * argv[1] must be SIDEREON_CORE_FIXTURES, normally
 * crates/sidereon-core/tests/fixtures from the local core checkout.
 */
#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sidereon.h"
#include "engine_pins_fixture.h"
#include "w4_r2p_pins.h"

#ifndef M_PI
#define M_PI 3.14159265358979323846
#endif

/* Binding codes of enums the header does not export (src/ntrip.rs,
 * src/space_weather.rs). The generated pins name these constants. */
enum {
    NTRIP_EVENT_CONNECTED = 0,
    NTRIP_EVENT_PAYLOAD = 1,
    NTRIP_EVENT_SOURCETABLE = 2,
    NTRIP_EVENT_REJECTED = 3,
    NTRIP_EVENT_STREAM_CORRUPTED = 4,
    NTRIP_EVENT_STREAM_ENDED = 5,
    NTRIP_STATE_IDLE = 0,
    NTRIP_STATE_AWAITING_STATUS = 1,
    NTRIP_STATE_AWAITING_HEADERS = 2,
    NTRIP_STATE_SOURCETABLE = 4,
    NTRIP_STATE_CLOSED = 5,
    SW_CLASS_OBSERVED = 0,
    SW_CLASS_INTERPOLATED = 1,
    SW_CLASS_DAILY_PREDICTED = 2,
    SW_CLASS_MONTHLY_PREDICTED = 3,
    SW_CLASS_NOT_OBSERVED = 4
};

enum {
    COV_FRAME_INERTIAL = 0,
    PROCESS_NOISE_NONE = 0,
    PROCESS_NOISE_RTN_ACCELERATION_PSD = 1,
    NAV_MESSAGE_GPS_CNAV = 1,
    NAV_MESSAGE_QZSS_CNAV = 3,
    NAV_MESSAGE_QZSS_CNAV2 = 4,
    NAV_MESSAGE_PREFER_MODERN = 1,
    GROUP_DELAY_CNAV_ISC_L1CA = 5,
    GROUP_DELAY_CNAV_ISC_L2C = 6,
    GROUP_DELAY_CNAV_ISC_L1CP = 10,
    CNAV_SIGNAL_L1CA = 0,
    CNAV_SIGNAL_L1CP = 4,
    NTRIP_VERSION_REV1 = 1,
    NTRIP_VERSION_REV2 = 2,
    NTRIP_STATE_STREAMING = 3
};

static int failures = 0;

static void check(int ok, const char *what) {
    if (!ok) {
        char msg[512];
        size_t n = sidereon_last_error_message(msg, sizeof(msg));
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

static void check_exact_bits(double got, uint64_t expected, const char *what) {
    uint64_t bits = 0;
    memcpy(&bits, &got, sizeof(bits));
    if (bits != expected) {
        fprintf(stderr, "FAIL: %s: got %016llx, expected %016llx\n", what,
                (unsigned long long)bits, (unsigned long long)expected);
        failures++;
    }
}

/* FNV-1a 64 of a byte string; the generated pins carry the same hash of the
 * engine's text. */
static uint64_t fnv1a64(const uint8_t *data, size_t len) {
    uint64_t hash = UINT64_C(0xcbf29ce484222325);
    for (size_t i = 0; i < len; i++) {
        hash ^= data[i];
        hash *= UINT64_C(0x00000100000001b3);
    }
    return hash;
}

static void check_text(const void *got, size_t got_len, const char *expected,
                       size_t expected_len, const char *what) {
    check(got_len == expected_len && memcmp(got, expected, expected_len) == 0, what);
}

static char *join_path(const char *root, const char *rel) {
    size_t a = strlen(root);
    size_t b = strlen(rel);
    int need_slash = a > 0 && root[a - 1] != '/';
    char *out = (char *)malloc(a + (size_t)need_slash + b + 1);
    if (!out) {
        fprintf(stderr, "FAIL: malloc path\n");
        exit(2);
    }
    memcpy(out, root, a);
    size_t pos = a;
    if (need_slash) {
        out[pos++] = '/';
    }
    memcpy(out + pos, rel, b);
    out[pos + b] = '\0';
    return out;
}

static uint8_t *read_file(const char *path, size_t *out_len) {
    FILE *f = fopen(path, "rb");
    if (!f) {
        fprintf(stderr, "FAIL: open %s\n", path);
        exit(2);
    }
    if (fseek(f, 0, SEEK_END) != 0) {
        fprintf(stderr, "FAIL: seek %s\n", path);
        exit(2);
    }
    long len = ftell(f);
    if (len < 0) {
        fprintf(stderr, "FAIL: tell %s\n", path);
        exit(2);
    }
    rewind(f);
    uint8_t *buf = (uint8_t *)malloc((size_t)len + 1);
    if (!buf) {
        fprintf(stderr, "FAIL: malloc file\n");
        exit(2);
    }
    size_t got = fread(buf, 1, (size_t)len, f);
    fclose(f);
    if (got != (size_t)len) {
        fprintf(stderr, "FAIL: read %s\n", path);
        exit(2);
    }
    buf[got] = 0;
    *out_len = got;
    return buf;
}

static int token_equals(const SidereonSatelliteToken *token, const char *expected) {
    return strncmp((const char *)token->bytes, expected, sizeof(token->bytes)) == 0;
}

typedef SidereonStatus (*ObservationQcStringFn)(const SidereonObservationQcReport *report,
                                                uint8_t *out, size_t len,
                                                size_t *out_written,
                                                size_t *out_required);

static char *copy_qc_report_string(SidereonObservationQcReport *report, ObservationQcStringFn fn,
                                   const char *what, size_t *out_len) {
    size_t written = 0, required = 0;
    check(fn(report, NULL, 0, &written, &required) == SIDEREON_STATUS_OK && written == 0 &&
              required > 0,
          what);
    char *out = (char *)malloc(required + 1);
    if (!out) {
        fprintf(stderr, "FAIL: malloc QC string\n");
        exit(2);
    }
    check(fn(report, (uint8_t *)out, required, &written, &required) == SIDEREON_STATUS_OK &&
              written == required,
          what);
    out[written] = '\0';
    *out_len = written;
    return out;
}

static uint8_t *copy_ntrip_bytes(const SidereonNtripBytes *bytes, size_t *out_len) {
    size_t written = 0, required = 0;
    check(sidereon_ntrip_bytes(bytes, NULL, 0, &written, &required) == SIDEREON_STATUS_OK,
          "ntrip bytes size");
    uint8_t *out = (uint8_t *)malloc(required + 1);
    if (!out) {
        fprintf(stderr, "FAIL: malloc ntrip bytes\n");
        exit(2);
    }
    check(sidereon_ntrip_bytes(bytes, out, required, &written, &required) ==
              SIDEREON_STATUS_OK &&
              written == required,
          "ntrip bytes copy");
    out[written] = 0;
    *out_len = written;
    return out;
}

static double j2000(int y, int m, int d, int h, int min, double s) {
    double out = 0.0;
    check(sidereon_civil_to_j2000_seconds(y, m, d, h, min, s, &out) == SIDEREON_STATUS_OK,
          "civil to j2000");
    return out;
}

static void test_covariance(void) {
    SidereonCovarianceMatrix6 p0;
    memset(&p0, 0, sizeof(p0));
    p0.values[0][0] = 1.0;
    p0.values[1][1] = 2.0;
    p0.values[2][2] = 3.0;
    p0.values[3][3] = 0.01;
    p0.values[4][4] = 0.02;
    p0.values[5][5] = 0.03;

    SidereonCovarianceTransportSegment seg;
    memset(&seg, 0, sizeof(seg));
    for (int i = 0; i < 6; i++) {
        seg.stm.values[i][i] = 1.0;
    }
    seg.dt_seconds = 10.0;
    seg.q_rotation_state.epoch_s = 0.0;
    seg.q_rotation_state.position_km[0] = 7000.0;
    seg.q_rotation_state.velocity_km_s[1] = 7.5;

    SidereonProcessNoise noise = {PROCESS_NOISE_RTN_ACCELERATION_PSD, 1.0e-6, 2.0e-6,
                                  3.0e-6};
    SidereonCovarianceMatrix6 out[2];
    size_t written = 0, required = 0;
    check(sidereon_covariance_transport(&p0, &seg, 1, noise, out, 1, &written,
                                        &required) == SIDEREON_STATUS_INVALID_ARGUMENT &&
              written == 0 && required == W4_R2P_COV_TRANSPORT_COUNT,
          "covariance transport size");
    check(sidereon_covariance_transport(&p0, &seg, 1, noise, out, 2, &written,
                                        &required) == SIDEREON_STATUS_OK &&
              written == W4_R2P_COV_TRANSPORT_COUNT && required == W4_R2P_COV_TRANSPORT_COUNT,
          "covariance transport copy");
    /* The first covariance is the one supplied; the second is the engine's
     * transport of it (tests/valgen w4_r2p). */
    check(memcmp(&out[0], &p0, sizeof(p0)) == 0, "cov initial equals P0");
    for (int i = 0; i < 6; i++) {
        for (int j = 0; j < 6; j++) {
            check_exact_bits(out[1].values[i][j], W4_R2P_COV_TRANSPORT_1_BITS[i * 6 + j],
                             "cov transported entry");
        }
    }

    SidereonStatePropagationConfig cfg;
    check(sidereon_state_propagation_config_init(&cfg) == SIDEREON_STATUS_OK,
          "state propagation config init");
    cfg.epoch_s = 0.0;
    cfg.position_km[0] = 7000.0;
    cfg.position_km[1] = 0.0;
    cfg.position_km[2] = 0.0;
    cfg.velocity_km_s[0] = 0.0;
    cfg.velocity_km_s[1] = 7.5;
    cfg.velocity_km_s[2] = 0.0;
    cfg.force_model = SIDEREON_PROPAGATION_FORCE_MODEL_TWO_BODY;
    double epochs[1] = {0.0};
    SidereonCovariancePropagationOptions opts;
    opts.input_frame = COV_FRAME_INERTIAL;
    opts.output_frame = COV_FRAME_INERTIAL;
    opts.process_noise.kind = PROCESS_NOISE_NONE;
    opts.process_noise.q_radial_km2_s3 = 0.0;
    opts.process_noise.q_transverse_km2_s3 = 0.0;
    opts.process_noise.q_normal_km2_s3 = 0.0;
    SidereonCovarianceEphemeris *eph = NULL;
    check(sidereon_propagate_covariance(&cfg, &p0, epochs, 1, opts, &eph) ==
              SIDEREON_STATUS_OK &&
              eph != NULL,
          "propagate covariance");
    size_t count = 0;
    /* One node per requested epoch; the node at the initial epoch holds the
     * supplied covariance. */
    check(sidereon_covariance_ephemeris_count(eph, &count) == SIDEREON_STATUS_OK &&
              count == sizeof(epochs) / sizeof(epochs[0]),
          "covariance ephemeris count");
    SidereonCovarianceMatrix6 at0;
    check(sidereon_covariance_ephemeris_covariance_at(eph, 0.0, &at0) == SIDEREON_STATUS_OK,
          "covariance at initial epoch");
    check(memcmp(&at0, &p0, sizeof(p0)) == 0, "covariance at initial epoch equals P0");
    sidereon_covariance_ephemeris_free(eph);
}

static void test_cnav(const char *fixtures) {
    char *path = join_path(fixtures, "nav/BRD400DLR_S_20261800000_01H_MN_trim.rnx");
    size_t len = 0;
    uint8_t *bytes = read_file(path, &len);
    SidereonBroadcastEphemeris *nav = NULL;
    check(sidereon_broadcast_ephemeris_parse_nav(bytes, len, &nav) == SIDEREON_STATUS_OK &&
              nav != NULL,
          "parse RINEX-4 CNAV NAV");
    /* The trim holds eight frames: G01 and G03 LNAV and CNAV, J02 LNAV, CNAV
     * and CNV2, and a BeiDou C19 CNV2 frame the reader does not decode. The
     * record count and every J02 value below are sidereon-core's own reading
     * of the file (tests/valgen w4_r2p). */
    size_t count = 0;
    check(sidereon_broadcast_ephemeris_record_count(nav, &count) == SIDEREON_STATUS_OK &&
              count == W4_R2P_CNAV_RECORD_COUNT,
          "CNAV record count");
    SidereonBroadcastRecordInfo records[W4_R2P_CNAV_RECORD_COUNT];
    size_t written = 0, required = 0;
    check(sidereon_broadcast_ephemeris_records(nav, records, W4_R2P_CNAV_RECORD_COUNT, &written,
                                               &required) == SIDEREON_STATUS_OK &&
              written == W4_R2P_CNAV_RECORD_COUNT && required == W4_R2P_CNAV_RECORD_COUNT,
          "CNAV records copy");
    int j02_cnav = -1;
    int j02_cnav2 = -1;
    for (size_t i = 0; i < written; i++) {
        if (strcmp(records[i].sat_id.bytes, "J02") == 0) {
            if (records[i].message == NAV_MESSAGE_QZSS_CNAV) {
                j02_cnav = (int)i;
            } else if (records[i].message == NAV_MESSAGE_QZSS_CNAV2) {
                j02_cnav2 = (int)i;
            }
        }
    }
    check(j02_cnav >= 0 && j02_cnav2 >= 0, "find J02 CNAV records");
    if (j02_cnav >= 0) {
        SidereonBroadcastRecordInfo r = records[j02_cnav];
        /* A record without an issue of data reads issue 0 and message 0
         * (src/broadcast.rs broadcast_record_to_c). */
        check(r.has_issue == W4_R2P_J02_CNAV_HAS_ISSUE &&
                  (r.has_issue || (r.issue == 0 && r.issue_message == 0)) &&
                  r.week == W4_R2P_J02_CNAV_WEEK && r.toe_week == W4_R2P_J02_CNAV_TOE_WEEK,
              "J02 CNAV issue/week");
        check_exact_bits(r.toe_tow_s, W4_R2P_J02_CNAV_TOE_TOW_S_BITS, "J02 CNAV toe tow");
        check(r.cnav.present == W4_R2P_J02_CNAV_PRESENT &&
                  r.cnav.ura_ed_index == W4_R2P_J02_CNAV_URA_ED_INDEX &&
                  r.cnav.ura_ned0_index == W4_R2P_J02_CNAV_URA_NED0_INDEX &&
                  r.cnav.ura_ned1_index == W4_R2P_J02_CNAV_URA_NED1_INDEX &&
                  r.cnav.ura_ned2_index == W4_R2P_J02_CNAV_URA_NED2_INDEX,
              "J02 CNAV URA indices");
        check_exact_bits(r.cnav.adot_m_s, W4_R2P_J02_CNAV_ADOT_BITS, "J02 CNAV adot");
        check_exact_bits(r.cnav.delta_n0_dot_rad_s2, W4_R2P_J02_CNAV_DN0_DOT_BITS,
                         "J02 CNAV dn dot");
        bool present = false;
        double ura = 0.0;
        check(sidereon_cnav_ura_nominal_m(0, &ura, &present) == SIDEREON_STATUS_OK &&
                  present == W4_R2P_CNAV_URA_NOMINAL_0_PRESENT,
              "CNAV URA nominal");
        check_exact_bits(ura, W4_R2P_CNAV_URA_NOMINAL_0_BITS, "CNAV URA index 0");
        double ned = 0.0;
        check(sidereon_cnav_ura_ned_m(&r.cnav, 2425, 86400.0, &ned, &present) ==
                  SIDEREON_STATUS_OK &&
                  present == W4_R2P_J02_CNAV_URA_NED_PRESENT,
              "CNAV URA NED");
        check_exact_bits(ned, W4_R2P_J02_CNAV_URA_NED_BITS, "CNAV URA NED at toe");
        double delay = 0.0;
        check(sidereon_broadcast_ephemeris_record_group_delay(
                  nav, (size_t)j02_cnav, GROUP_DELAY_CNAV_ISC_L2C, &delay, &present) ==
                  SIDEREON_STATUS_OK &&
                  present == W4_R2P_J02_CNAV_ISC_L2C_PRESENT,
              "CNAV L2C ISC");
        check_exact_bits(delay, W4_R2P_J02_CNAV_ISC_L2C_BITS, "CNAV L2C ISC value");
        double corr = 0.0;
        check(sidereon_broadcast_ephemeris_record_cnav_correction(
                  nav, (size_t)j02_cnav, CNAV_SIGNAL_L1CA, &corr, &present) ==
                  SIDEREON_STATUS_OK &&
                  present == W4_R2P_J02_CNAV_L1CA_CORRECTION_PRESENT,
              "CNAV L1CA correction");
        check_exact_bits(corr, W4_R2P_J02_CNAV_L1CA_CORRECTION_BITS,
                         "CNAV L1CA correction value");
    }
    if (j02_cnav2 >= 0) {
        SidereonBroadcastRecordInfo r = records[j02_cnav2];
        check(r.has_issue == W4_R2P_J02_CNAV2_HAS_ISSUE &&
                  (r.has_issue || (r.issue == 0 && r.issue_message == 0)) &&
                  r.cnav.present == W4_R2P_J02_CNAV2_PRESENT,
              "J02 CNAV2 issue/transmission");
        check_exact_bits(r.cnav.transmission_time_sow, W4_R2P_J02_CNAV2_TRANSMISSION_SOW_BITS,
                         "J02 CNAV2 transmission time");
        bool present = false;
        double delay = 0.0;
        check(sidereon_broadcast_ephemeris_record_group_delay(
                  nav, (size_t)j02_cnav2, GROUP_DELAY_CNAV_ISC_L1CP, &delay, &present) ==
                  SIDEREON_STATUS_OK &&
                  present == W4_R2P_J02_CNAV2_ISC_L1CP_PRESENT,
              "CNAV2 L1CP ISC");
        check_exact_bits(delay, W4_R2P_J02_CNAV2_ISC_L1CP_BITS, "CNAV2 L1CP ISC value");
        double corr = 0.0;
        check(sidereon_broadcast_ephemeris_record_cnav_correction(
                  nav, (size_t)j02_cnav2, CNAV_SIGNAL_L1CP, &corr, &present) ==
                  SIDEREON_STATUS_OK &&
                  present == W4_R2P_J02_CNAV2_L1CP_CORRECTION_PRESENT,
              "CNAV2 L1CP correction");
        check_exact_bits(corr, W4_R2P_J02_CNAV2_L1CP_CORRECTION_BITS,
                         "CNAV2 L1CP correction value");
        SidereonBroadcastRecordInfo selected;
        check(sidereon_broadcast_ephemeris_select_by_issue(
                  nav, "J02", 288, NAV_MESSAGE_QZSS_CNAV2, j2000(2026, 6, 29, 0, 0, 0.0),
                  &selected, &present) == SIDEREON_STATUS_OK &&
                  present == W4_R2P_J02_CNAV2_SELECT_BY_ISSUE_PRESENT,
              "J02 CNAV2 select by issue");
    }
    uint32_t pref = 0;
    check(sidereon_broadcast_ephemeris_set_nav_message_preference(nav, NAV_MESSAGE_PREFER_MODERN) ==
              SIDEREON_STATUS_OK,
          "set modern NAV preference");
    check(sidereon_broadcast_ephemeris_nav_message_preference(nav, &pref) ==
              SIDEREON_STATUS_OK &&
              pref == NAV_MESSAGE_PREFER_MODERN,
          "read modern NAV preference");
    sidereon_broadcast_ephemeris_free(nav);
    free(bytes);
    free(path);
}

static void test_tle_fit(void) {
    const char *tle_text =
        "ISS\n"
        "1 25544U 98067A   26168.18949189  .00009113  00000+0  17172-3 0  9996\n"
        "2 25544  51.6332 300.0813 0004737 195.1146 164.9702 15.49273435571752\n";
    SidereonTleFile *file = NULL;
    check(sidereon_parse_tle_file((const uint8_t *)tle_text, strlen(tle_text),
                                  SIDEREON_TLE_OPS_MODE_IMPROVED, &file) ==
              SIDEREON_STATUS_OK &&
              file != NULL,
          "parse fit TLE");
    SidereonTle *tle = NULL;
    check(sidereon_tle_file_satellite(file, 0, &tle) == SIDEREON_STATUS_OK && tle != NULL,
          "get fit TLE");
    /* 2026-06-17 04:22:52.099296, 04:32:52.099296 and 04:42:52.099296 UTC. */
    int64_t unix_us[3] = {1781670172099296LL, 1781670772099296LL, 1781671372099296LL};
    const double minute[3] = {22.0, 32.0, 42.0};
    SidereonTlePropagation *prop = NULL;
    check(sidereon_tle_propagate(tle, unix_us, 3, &prop) == SIDEREON_STATUS_OK &&
              prop != NULL,
          "propagate TLE for fit samples");
    SidereonTemeState states[3];
    size_t written = 0, required = 0;
    check(sidereon_tle_propagation_states(prop, states, 3, &written, &required) ==
              SIDEREON_STATUS_OK &&
              written == 3,
          "copy fit sample states");
    SidereonSgp4FitSample samples[3];
    memset(samples, 0, sizeof(samples));
    for (int i = 0; i < 3; i++) {
        /* Each sample is labelled with the split Julian date SGP4 propagated
         * it at: midnight plus the day fraction summed from the civil fields
         * in the engine's order. A single-double JD (unix / 86400 +
         * 2440587.5) is 3.7 to 12.7 microseconds away from these instants,
         * which put centimetres of timing error into every residual. */
        samples[i].jd_whole = 2461208.5;
        samples[i].jd_fraction = 4.0 / 24.0 + minute[i] / 1440.0 + 52.0 / 86400.0 +
                                 99296.0 / 86400000000.0;
        memcpy(samples[i].position_teme_km, states[i].position_km, sizeof(states[i].position_km));
        memcpy(samples[i].velocity_teme_km_s, states[i].velocity_km_s,
               sizeof(states[i].velocity_km_s));
        samples[i].has_velocity_teme_km_s = true;
    }
    SidereonSgp4FitConfig cfg;
    check(sidereon_sgp4_fit_config_init(&cfg) == SIDEREON_STATUS_OK, "fit config init");
    cfg.has_max_nfev = true;
    cfg.max_nfev = 80;
    cfg.catalog_number = 25544;
    cfg.classification[0] = 'U';
    cfg.classification[1] = '\0';
    cfg.element_set_number = 999;
    cfg.rev_at_epoch = 57175;
    SidereonSgp4TleFit *fit = NULL;
    check(sidereon_sgp4_fit_tle(samples, 3, &cfg, &fit) == SIDEREON_STATUS_OK && fit != NULL,
          "fit TLE");
    SidereonSgp4FitStatistics stats;
    check(sidereon_sgp4_tle_fit_statistics(fit, &stats) == SIDEREON_STATUS_OK,
          "fit statistics");
    /* With every sample labelled with the instant it was propagated at, the
     * fit recovers the source elements: the residual is nanometres of solver
     * noise, and the fitted TLE lines, re-parsed, reproduce the samples
     * exactly. Every expected value is the engine's own fit of these samples
     * (sidereon_core::astro::sgp4::fit_tle), written by tests/pingen. The
     * fitted lines carry the source epoch, B*, inclination, RAAN,
     * eccentricity, argument of perigee, mean anomaly and mean motion; the
     * first derivative of mean motion, which SGP4 does not use, is not
     * fitted. */
    check_exact_bits(stats.rms_position_km, PIN_FIT_RMS_POSITION_KM_BITS, "fit RMS position");
    check_exact_bits(stats.max_position_km, PIN_FIT_MAX_POSITION_KM_BITS, "fit max position");
    /* tests/pingen fails unless the engine reports a velocity RMS. */
    check(stats.has_rms_velocity_km_s, "fit velocity RMS present");
    check_exact_bits(stats.rms_velocity_km_s, PIN_FIT_RMS_VELOCITY_KM_S_BITS,
                     "fit RMS velocity");
    check_exact_bits(stats.tle_rms_position_km, PIN_FIT_TLE_RMS_POSITION_KM_BITS,
                     "fit TLE lines reproduce the samples");
    check(stats.status == PIN_FIT_STATUS && stats.nfev == PIN_FIT_NFEV &&
              stats.njev == PIN_FIT_NJEV &&
              stats.seed_refine_passes == PIN_FIT_SEED_REFINE_PASSES,
          "fit solver path");
    SidereonTleLines lines;
    check(sidereon_sgp4_tle_fit_lines(fit, &lines) == SIDEREON_STATUS_OK, "fit lines");
    check(strcmp(lines.line1.bytes, PIN_FIT_LINE1) == 0 &&
              strcmp(lines.line2.bytes, PIN_FIT_LINE2) == 0,
          "fit TLE lines");
    sidereon_sgp4_tle_fit_free(fit);
    sidereon_tle_propagation_free(prop);
    sidereon_tle_free(tle);
    sidereon_tle_file_free(file);
}

static void test_qc(const char *fixtures) {
    char *obs_path = join_path(fixtures, "obs/ESBC00DNK_R_20201770000_01D_30S_MO_120epoch.rnx");
    size_t obs_len = 0;
    uint8_t *obs = read_file(obs_path, &obs_len);
    SidereonObservationQcOptions opts;
    check(sidereon_observation_qc_options_init(&opts) == SIDEREON_STATUS_OK,
          "QC options init");
    SidereonObservationQcReport *report = NULL;
    check(sidereon_observation_qc_parse(obs, obs_len, &opts, &report) == SIDEREON_STATUS_OK &&
              report != NULL,
          "observation QC parse");
    SidereonObservationQcSummary summary;
    check(sidereon_observation_qc_summary(report, &summary) == SIDEREON_STATUS_OK,
          "observation QC summary");
    check(summary.total_epoch_records == W4_R2P_QC_TOTAL_EPOCH_RECORDS &&
              summary.observation_epochs == W4_R2P_QC_OBSERVATION_EPOCHS &&
              summary.event_records == W4_R2P_QC_EVENT_RECORDS &&
              summary.skipped_records == W4_R2P_QC_SKIPPED_RECORDS &&
              summary.has_interval_s == W4_R2P_QC_HAS_INTERVAL_S &&
              summary.missing_epochs == W4_R2P_QC_MISSING_EPOCHS &&
              summary.data_gap_count == W4_R2P_QC_DATA_GAP_COUNT &&
              summary.satellite_signal_count == PIN_QC_SATELLITE_SIGNAL_COUNT &&
              summary.system_signal_count == PIN_QC_SYSTEM_SIGNAL_COUNT,
          "observation QC oracle summary");
    check_exact_bits(summary.interval_s, W4_R2P_QC_INTERVAL_S_BITS, "observation QC interval");

    size_t written = 0, required = 0;
    check(sidereon_observation_qc_clock_jumps(report, NULL, 0, &written, &required) ==
                  SIDEREON_STATUS_OK &&
              written == 0 && required == W4_R2P_QC_CLOCK_JUMP_COUNT,
          "observation QC clock jump count");

    SidereonObservationQcCycleSlips cycle_slips;
    check(sidereon_observation_qc_cycle_slips(report, &cycle_slips) == SIDEREON_STATUS_OK &&
              cycle_slips.observations == PIN_QC_CYCLE_SLIP_OBSERVATIONS &&
              cycle_slips.total_slips == PIN_QC_CYCLE_SLIP_TOTAL &&
              cycle_slips.has_observations_per_slip == W4_R2P_QC_HAS_OBSERVATIONS_PER_SLIP &&
              cycle_slips.system_count == PIN_QC_CYCLE_SLIP_SYSTEM_COUNT,
          "observation QC cycle-slip summary");
    check_exact_bits(cycle_slips.observations_per_slip, W4_R2P_QC_OBSERVATIONS_PER_SLIP_BITS,
                     "QC observations per slip");

    SidereonObservationQcSystemCycleSlip slip_systems[PIN_QC_CYCLE_SLIP_SYSTEM_COUNT];
    written = 0;
    required = 0;
    check(sidereon_observation_qc_cycle_slip_systems(report, NULL, 0, &written,
                                                     &required) == SIDEREON_STATUS_OK &&
              written == 0 && required == PIN_QC_CYCLE_SLIP_SYSTEM_COUNT,
          "observation QC cycle-slip systems size");
    check(sidereon_observation_qc_cycle_slip_systems(report, slip_systems,
                                                     PIN_QC_CYCLE_SLIP_SYSTEM_COUNT, &written,
                                                     &required) == SIDEREON_STATUS_OK &&
              written == PIN_QC_CYCLE_SLIP_SYSTEM_COUNT &&
              required == PIN_QC_CYCLE_SLIP_SYSTEM_COUNT,
          "observation QC cycle-slip systems copy");
    int saw_gps_slip = 0;
    int saw_glonass_slip = 0;
    int saw_galileo_slip = 0;
    int saw_beidou_slip = 0;
    for (size_t i = 0; i < written; i++) {
        const SidereonObservationQcSystemCycleSlip *row = &slip_systems[i];
        if (row->system == SIDEREON_GNSS_SYSTEM_GPS) {
            saw_gps_slip = 1;
            check(row->observations == PIN_QC_SLIPS_GPS_OBSERVATIONS &&
                      row->slips == PIN_QC_SLIPS_GPS_SLIPS &&
                      row->has_observations_per_slip ==
                          W4_R2P_QC_SLIPS_GPS_HAS_OBSERVATIONS_PER_SLIP,
                  "QC GPS cycle slips");
            check_exact_bits(row->observations_per_slip,
                             W4_R2P_QC_SLIPS_GPS_OBSERVATIONS_PER_SLIP_BITS,
                             "QC GPS observations per slip");
        } else if (row->system == SIDEREON_GNSS_SYSTEM_GLONASS) {
            saw_glonass_slip = 1;
            /* The 136 R09 and R12 records that also carry G3 values count: a
             * code on a carrier with no frequency no longer drops the whole
             * satellite-epoch. */
            check(row->observations == PIN_QC_SLIPS_GLONASS_OBSERVATIONS &&
                      row->slips == PIN_QC_SLIPS_GLONASS_SLIPS &&
                      row->has_observations_per_slip ==
                          W4_R2P_QC_SLIPS_GLONASS_HAS_OBSERVATIONS_PER_SLIP,
                  "QC GLONASS cycle slips");
            check_exact_bits(row->observations_per_slip,
                             W4_R2P_QC_SLIPS_GLONASS_OBSERVATIONS_PER_SLIP_BITS,
                             "QC GLONASS observations per slip");
        } else if (row->system == SIDEREON_GNSS_SYSTEM_GALILEO) {
            saw_galileo_slip = 1;
            check(row->observations == PIN_QC_SLIPS_GALILEO_OBSERVATIONS &&
                      row->slips == PIN_QC_SLIPS_GALILEO_SLIPS &&
                      row->has_observations_per_slip ==
                          W4_R2P_QC_SLIPS_GALILEO_HAS_OBSERVATIONS_PER_SLIP,
                  "QC Galileo cycle slips");
            check_exact_bits(row->observations_per_slip,
                             W4_R2P_QC_SLIPS_GALILEO_OBSERVATIONS_PER_SLIP_BITS,
                             "QC Galileo observations per slip");
        } else if (row->system == SIDEREON_GNSS_SYSTEM_BEI_DOU) {
            saw_beidou_slip = 1;
            check(row->observations == PIN_QC_SLIPS_BEIDOU_OBSERVATIONS &&
                      row->slips == PIN_QC_SLIPS_BEIDOU_SLIPS &&
                      row->has_observations_per_slip ==
                          W4_R2P_QC_SLIPS_BEIDOU_HAS_OBSERVATIONS_PER_SLIP,
                  "QC BeiDou cycle slips");
            check_exact_bits(row->observations_per_slip,
                             W4_R2P_QC_SLIPS_BEIDOU_OBSERVATIONS_PER_SLIP_BITS,
                             "QC BeiDou observations per slip");
        }
    }
    check(saw_gps_slip && saw_glonass_slip && saw_galileo_slip && saw_beidou_slip,
          "observation QC cycle-slip systems present");

    SidereonObservationQcSystemMultipath mp_systems[PIN_QC_MULTIPATH_SYSTEM_COUNT];
    written = 0;
    required = 0;
    check(sidereon_observation_qc_multipath_systems(report, NULL, 0, &written,
                                                    &required) == SIDEREON_STATUS_OK &&
              written == 0 && required == PIN_QC_MULTIPATH_SYSTEM_COUNT,
          "observation QC multipath systems size");
    check(sidereon_observation_qc_multipath_systems(report, mp_systems,
                                                    PIN_QC_MULTIPATH_SYSTEM_COUNT, &written,
                                                    &required) == SIDEREON_STATUS_OK &&
              written == PIN_QC_MULTIPATH_SYSTEM_COUNT &&
              required == PIN_QC_MULTIPATH_SYSTEM_COUNT,
          "observation QC multipath systems copy");
    int saw_gps_mp = 0;
    int saw_glonass_mp = 0;
    int saw_galileo_mp = 0;
    int saw_beidou_mp = 0;
    for (size_t i = 0; i < written; i++) {
        const SidereonObservationQcSystemMultipath *row = &mp_systems[i];
        if (row->system == SIDEREON_GNSS_SYSTEM_GPS) {
            saw_gps_mp = 1;
            check(row->has_mp1 == W4_R2P_QC_MP_GPS_HAS_MP1 &&
                      row->has_mp2 == W4_R2P_QC_MP_GPS_HAS_MP2,
                  "QC GPS multipath presence");
            check(row->mp1.n == PIN_QC_MP_GPS_MP1_N && row->mp2.n == PIN_QC_MP_GPS_MP2_N, "QC GPS multipath counts");
            check_exact_bits(row->mp1.rms_m, PIN_QC_MP_GPS_MP1_RMS_M_BITS, "QC GPS MP1");
            check_exact_bits(row->mp2.rms_m, PIN_QC_MP_GPS_MP2_RMS_M_BITS, "QC GPS MP2");
        } else if (row->system == SIDEREON_GNSS_SYSTEM_GLONASS) {
            saw_glonass_mp = 1;
            check(row->has_mp1 == W4_R2P_QC_MP_GLONASS_HAS_MP1 &&
                      row->has_mp2 == W4_R2P_QC_MP_GLONASS_HAS_MP2,
                  "QC GLONASS multipath presence");
            check(row->mp1.n == PIN_QC_MP_GLONASS_MP1_N && row->mp2.n == PIN_QC_MP_GLONASS_MP2_N, "QC GLONASS multipath counts");
            check_exact_bits(row->mp1.rms_m, PIN_QC_MP_GLONASS_MP1_RMS_M_BITS, "QC GLONASS MP1");
            check_exact_bits(row->mp2.rms_m, PIN_QC_MP_GLONASS_MP2_RMS_M_BITS, "QC GLONASS MP2");
        } else if (row->system == SIDEREON_GNSS_SYSTEM_GALILEO) {
            saw_galileo_mp = 1;
            check(row->has_mp1 == W4_R2P_QC_MP_GALILEO_HAS_MP1 &&
                      row->has_mp2 == W4_R2P_QC_MP_GALILEO_HAS_MP2,
                  "QC GALILEO multipath presence");
            check(row->mp1.n == PIN_QC_MP_GALILEO_MP1_N && row->mp2.n == PIN_QC_MP_GALILEO_MP2_N,
                  "QC Galileo multipath counts");
            check_exact_bits(row->mp1.rms_m, PIN_QC_MP_GALILEO_MP1_RMS_M_BITS, "QC Galileo MP1");
            check_exact_bits(row->mp2.rms_m, PIN_QC_MP_GALILEO_MP2_RMS_M_BITS, "QC Galileo MP2");
        } else if (row->system == SIDEREON_GNSS_SYSTEM_BEI_DOU) {
            saw_beidou_mp = 1;
            check(row->has_mp1 == W4_R2P_QC_MP_BEIDOU_HAS_MP1 &&
                      row->has_mp2 == W4_R2P_QC_MP_BEIDOU_HAS_MP2,
                  "QC BEIDOU multipath presence");
            check(row->mp1.n == PIN_QC_MP_BEIDOU_MP1_N && row->mp2.n == PIN_QC_MP_BEIDOU_MP2_N, "QC BeiDou multipath counts");
            check_exact_bits(row->mp1.rms_m, PIN_QC_MP_BEIDOU_MP1_RMS_M_BITS, "QC BeiDou MP1");
            check_exact_bits(row->mp2.rms_m, PIN_QC_MP_BEIDOU_MP2_RMS_M_BITS, "QC BeiDou MP2");
        }
    }
    check(saw_gps_mp && saw_glonass_mp && saw_galileo_mp && saw_beidou_mp,
          "observation QC multipath systems present");

    written = 0;
    required = 0;
    check(sidereon_observation_qc_multipath_satellites(report, NULL, 0, &written,
                                                       &required) == SIDEREON_STATUS_OK &&
              written == 0 && required == PIN_QC_MULTIPATH_SATELLITE_COUNT,
          "observation QC multipath satellites size");
    SidereonObservationQcSatelliteMultipath *mp_sats =
        (SidereonObservationQcSatelliteMultipath *)malloc(required * sizeof(*mp_sats));
    if (!mp_sats) {
        fprintf(stderr, "FAIL: malloc QC multipath satellites\n");
        exit(2);
    }
    check(sidereon_observation_qc_multipath_satellites(report, mp_sats, required, &written,
                                                       &required) == SIDEREON_STATUS_OK &&
              written == required,
          "observation QC multipath satellites copy");
    int saw_g08_mp = 0;
    for (size_t i = 0; i < written; i++) {
        if (token_equals(&mp_sats[i].sat_id, "G08")) {
            saw_g08_mp = 1;
            check(mp_sats[i].has_mp1 == W4_R2P_QC_MP_G08_HAS_MP1 &&
                      mp_sats[i].has_mp2 == W4_R2P_QC_MP_G08_HAS_MP2 &&
                      mp_sats[i].mp1.n == PIN_QC_MP_G08_MP1_N &&
                      mp_sats[i].mp2.n == PIN_QC_MP_G08_MP2_N,
                  "QC G08 multipath counts");
            check_exact_bits(mp_sats[i].mp1.rms_m, PIN_QC_MP_G08_MP1_RMS_M_BITS,
                             "QC G08 MP1");
            check_exact_bits(mp_sats[i].mp2.rms_m, PIN_QC_MP_G08_MP2_RMS_M_BITS,
                             "QC G08 MP2");
        }
    }
    check(saw_g08_mp, "observation QC multipath satellite G08 present");
    free(mp_sats);

    /* The rendered text, HTML and JSON reports are sidereon-core's
     * (render_text, render_html, and the report serialized with serde_json),
     * compared by length and FNV-1a 64 of their bytes (tests/valgen w4_r2p). */
    size_t text_len = 0;
    char *text_report =
        copy_qc_report_string(report, sidereon_observation_qc_render_text, "QC render_text",
                              &text_len);
    check(text_len == W4_R2P_QC_RENDER_TEXT_LEN &&
              fnv1a64((const uint8_t *)text_report, text_len) == W4_R2P_QC_RENDER_TEXT_FNV1A64,
          "observation QC render_text bytes");
    free(text_report);

    size_t html_len = 0;
    char *html_report =
        copy_qc_report_string(report, sidereon_observation_qc_render_html, "QC render_html",
                              &html_len);
    check(html_len == W4_R2P_QC_RENDER_HTML_LEN &&
              fnv1a64((const uint8_t *)html_report, html_len) == W4_R2P_QC_RENDER_HTML_FNV1A64,
          "observation QC render_html bytes");
    free(html_report);

    size_t json_len = 0;
    char *json_report =
        copy_qc_report_string(report, sidereon_observation_qc_to_json, "QC to_json", &json_len);
    check(json_len == W4_R2P_QC_TO_JSON_LEN &&
              fnv1a64((const uint8_t *)json_report, json_len) == W4_R2P_QC_TO_JSON_FNV1A64,
          "observation QC to_json bytes");
    free(json_report);

    sidereon_observation_qc_report_free(report);
    free(obs);
    free(obs_path);

    char *crx_path = join_path(fixtures, "obs/ESBC00DNK_R_20201770000_01D_30S_MO_trim.crx");
    size_t crx_len = 0;
    uint8_t *crx = read_file(crx_path, &crx_len);
    SidereonRinexLintReport *lint = NULL;
    check(sidereon_rinex_lint_obs(crx, crx_len, &lint) == SIDEREON_STATUS_OK && lint != NULL,
          "lint CRINEX OBS bytes");
    SidereonRinexLintSummary lsum;
    check(sidereon_rinex_lint_summary(lint, &lsum) == SIDEREON_STATUS_OK,
          "lint summary");
    check(lsum.finding_count == W4_R2P_LINT_FINDING_COUNT &&
              lsum.error_count == W4_R2P_LINT_ERROR_COUNT &&
              lsum.warning_count == W4_R2P_LINT_WARNING_COUNT &&
              lsum.decoded_from_crinex == W4_R2P_LINT_DECODED_FROM_CRINEX,
          "lint CRINEX H08 summary");
    sidereon_rinex_lint_report_free(lint);

    SidereonRinexRepairOptions ropts;
    check(sidereon_rinex_repair_options_init(&ropts) == SIDEREON_STATUS_OK,
          "repair options init");
    ropts.set_interval = true;
    ropts.set_time_of_last_obs = true;
    ropts.set_obs_counts = true;
    ropts.drop_empty_records = true;
    ropts.drop_unsupported = true;
    SidereonRinexRepair *repair = NULL;
    check(sidereon_rinex_repair_obs(crx, crx_len, &ropts, &repair) == SIDEREON_STATUS_OK &&
              repair != NULL,
          "repair CRINEX OBS bytes");
    SidereonRinexLintSummary rsum;
    check(sidereon_rinex_repair_summary(repair, &rsum) == SIDEREON_STATUS_OK,
          "repair remaining summary");
    check(rsum.finding_count == W4_R2P_REPAIR_REMAINING_FINDING_COUNT &&
              rsum.decoded_from_crinex == W4_R2P_REPAIR_DECODED_FROM_CRINEX,
          "repair remaining summary");
    written = 0;
    required = 0;
    check(sidereon_rinex_repair_crinex_text(repair, NULL, 0, &written, &required) ==
              SIDEREON_STATUS_OK &&
              required == W4_R2P_REPAIR_CRINEX_LEN,
          "repair CRINEX output size");
    sidereon_rinex_repair_free(repair);
    free(crx);
    free(crx_path);
}

static void test_geoid(void) {
    SidereonGeoidPoint pts[3] = {{0.0, 0.0}, {0.0, 80.0}, {60.0, -30.0}};
    double out[3] = {0.0, 0.0, 0.0};
    size_t written = 0, required = 0;
    check(sidereon_egm96_undulations_deg(pts, 3, out, 3, &written, &required) ==
              SIDEREON_STATUS_OK &&
              written == W4_R2P_EGM96_POINT_COUNT && required == W4_R2P_EGM96_POINT_COUNT,
          "EGM96 undulations batch");
    check_exact_bits(out[0], W4_R2P_EGM96_UNDULATION_0_BITS, "EGM96 0 0");
    check_exact_bits(out[1], W4_R2P_EGM96_UNDULATION_1_BITS, "EGM96 0 80");
    check_exact_bits(out[2], W4_R2P_EGM96_UNDULATION_2_BITS, "EGM96 60 -30");
    double orthometric = 0.0;
    double ellipsoidal = 0.0;
    check(sidereon_egm96_orthometric_height_m(100.0, 0.0, 0.0, &orthometric) ==
              SIDEREON_STATUS_OK,
          "EGM96 orthometric conversion");
    check_exact_bits(orthometric, W4_R2P_EGM96_ORTHOMETRIC_BITS, "EGM96 orthometric value");
    check(sidereon_egm96_ellipsoidal_height_m(orthometric, 0.0, 0.0, &ellipsoidal) ==
              SIDEREON_STATUS_OK,
          "EGM96 ellipsoidal conversion");
    check_exact_bits(ellipsoidal, W4_R2P_EGM96_ELLIPSOIDAL_BITS, "EGM96 ellipsoidal round trip");
}

static void test_nmea(void) {
    const char *text =
        "$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47\r\n"
        "$GPGGA,123520,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*4D\r\n";
    SidereonNmeaLog *log = NULL;
    check(sidereon_nmea_parse((const uint8_t *)text, strlen(text), &log) ==
              SIDEREON_STATUS_OK &&
              log != NULL,
          "NMEA parse");
    SidereonNmeaSummary summary;
    check(sidereon_nmea_log_summary(log, &summary) == SIDEREON_STATUS_OK,
          "NMEA summary");
    check(summary.sentence_count == W4_R2P_NMEA_SENTENCE_COUNT &&
              summary.epoch_count == W4_R2P_NMEA_EPOCH_COUNT &&
              summary.skip_count == W4_R2P_NMEA_SKIP_COUNT &&
              summary.warning_count == W4_R2P_NMEA_WARNING_COUNT,
          "NMEA summary counts");
    SidereonNmeaEpochSummary epochs[W4_R2P_NMEA_EPOCH_COUNT];
    size_t written = 0, required = 0;
    check(sidereon_nmea_log_epochs(log, epochs, W4_R2P_NMEA_EPOCH_COUNT, &written, &required) ==
              SIDEREON_STATUS_OK &&
              written == W4_R2P_NMEA_EPOCH_COUNT,
          "NMEA epochs");
    check(epochs[0].has_position == W4_R2P_NMEA_EPOCH0_HAS_POSITION &&
              epochs[0].sentence_count == W4_R2P_NMEA_EPOCH0_SENTENCE_COUNT &&
              epochs[0].used_satellite_count == W4_R2P_NMEA_EPOCH0_USED_SATELLITE_COUNT,
          "NMEA first epoch summary");
    check_exact_bits(epochs[0].position.lat_rad, W4_R2P_NMEA_EPOCH0_LAT_RAD_BITS,
                     "NMEA latitude");
    check_exact_bits(epochs[0].position.lon_rad, W4_R2P_NMEA_EPOCH0_LON_RAD_BITS,
                     "NMEA longitude");
    check_exact_bits(epochs[0].position.height_m, W4_R2P_NMEA_EPOCH0_HEIGHT_M_BITS,
                     "NMEA ellipsoidal height");
    sidereon_nmea_log_free(log);

    SidereonNmeaAccumulator *acc = NULL;
    check(sidereon_nmea_accumulator_new(&acc) == SIDEREON_STATUS_OK && acc != NULL,
          "NMEA accumulator new");
    SidereonNmeaChunkSummary chunk;
    check(sidereon_nmea_accumulator_push(acc, (const uint8_t *)text, 80, &chunk) ==
              SIDEREON_STATUS_OK,
          "NMEA accumulator push split 1");
    check(sidereon_nmea_accumulator_push(acc, (const uint8_t *)text + 80,
                                         strlen(text) - 80, &chunk) == SIDEREON_STATUS_OK,
          "NMEA accumulator push split 2");
    check(sidereon_nmea_accumulator_finish(acc, &chunk) == SIDEREON_STATUS_OK,
          "NMEA accumulator finish");
    check(sidereon_nmea_accumulator_summary(acc, &summary) == SIDEREON_STATUS_OK &&
              summary.sentence_count == W4_R2P_NMEA_ACC_SENTENCE_COUNT &&
              summary.epoch_count == W4_R2P_NMEA_ACC_EPOCH_COUNT,
          "NMEA accumulator summary");
    sidereon_nmea_accumulator_free(acc);

    SidereonNmeaGgaOptions gga;
    memset(&gga, 0, sizeof(gga));
    gga.talker[0] = 'G';
    gga.talker[1] = 'P';
    gga.utc_seconds_of_day = 12.0 * 3600.0 + 35.0 * 60.0 + 19.0;
    gga.position.lat_rad = 48.1173 * M_PI / 180.0;
    gga.position.lon_rad = 11.516666666666667 * M_PI / 180.0;
    gga.position.height_m = 592.3;
    gga.quality = 1;
    gga.satellites_used = 8;
    gga.hdop = 0.9;
    gga.coordinate_decimals = 3;
    check(sidereon_nmea_write_gga(&gga, NULL, 0, &written, &required) ==
              SIDEREON_STATUS_OK &&
              required == W4_R2P_NMEA_GGA_LEN,
          "NMEA GGA size");
    uint8_t buf[W4_R2P_NMEA_GGA_LEN];
    check(sidereon_nmea_write_gga(&gga, buf, sizeof(buf), &written, &required) ==
              SIDEREON_STATUS_OK,
          "NMEA GGA write");
    check_text(buf, written, W4_R2P_NMEA_GGA, W4_R2P_NMEA_GGA_LEN, "NMEA GGA exact bytes");
}

static void test_space_weather(const char *fixtures) {
    char *csv_path = join_path(fixtures, "space_weather/SW-All-20260702-trim.csv");
    size_t len = 0;
    uint8_t *csv = read_file(csv_path, &len);
    SidereonSpaceWeatherTable *table = NULL;
    check(sidereon_space_weather_table_parse(csv, len, &table) == SIDEREON_STATUS_OK &&
              table != NULL,
          "space-weather parse CSV bytes");
    SidereonSpaceWeatherTableSummary summary;
    check(sidereon_space_weather_table_summary(table, &summary) == SIDEREON_STATUS_OK,
          "space-weather summary");
    check(summary.day_count == W4_R2P_SW_DAY_COUNT &&
              summary.monthly_count == W4_R2P_SW_MONTHLY_COUNT &&
              summary.skip_count == W4_R2P_SW_SKIP_COUNT &&
              summary.warning_count == W4_R2P_SW_WARNING_COUNT,
          "space-weather fixture counts");
    SidereonSpaceWeatherCoverage coverage;
    check(sidereon_space_weather_table_coverage(table, &coverage) == SIDEREON_STATUS_OK,
          "space-weather coverage");
    check(coverage.has_last_observed_j2000_s == W4_R2P_SW_HAS_LAST_OBSERVED &&
              coverage.has_last_daily_predicted_j2000_s == W4_R2P_SW_HAS_LAST_DAILY_PREDICTED,
          "space-weather coverage flags");
    SidereonSpaceWeatherSample sample;
    check(sidereon_space_weather_table_sample_at(table, j2000(2026, 7, 1, 12, 0, 0.0),
                                                 &sample) == SIDEREON_STATUS_OK,
          "space-weather observed sample");
    check_exact_bits(sample.weather.f107, W4_R2P_SW_SAMPLE_F107_BITS, "space-weather f107");
    check_exact_bits(sample.weather.f107a, W4_R2P_SW_SAMPLE_F107A_BITS, "space-weather f107a");
    check_exact_bits(sample.weather.ap, W4_R2P_SW_SAMPLE_AP_BITS, "space-weather ap");
    check(sample.class_ == W4_R2P_SW_SAMPLE_CLASS &&
              sample.ap_defaulted == W4_R2P_SW_SAMPLE_AP_DEFAULTED,
          "space-weather observed class");
    double ap[7] = {0.0};
    check(sidereon_space_weather_table_ap_array_at(table, j2000(2003, 10, 31, 13, 0, 0.0),
                                                   ap) == SIDEREON_STATUS_OK,
          "space-weather AP array");
    for (int i = 0; i < 7; i++) {
        check_exact_bits(ap[i], W4_R2P_SW_AP_ARRAY_BITS[i], "space-weather AP array");
    }
    SidereonSpaceWeatherDay day;
    bool present = false;
    check(sidereon_space_weather_table_day(table, 2026, 7, 1, &present, &day) ==
              SIDEREON_STATUS_OK &&
              present == W4_R2P_SW_DAY_PRESENT,
          "space-weather day lookup");
    check(day.has_ap_avg == W4_R2P_SW_DAY_HAS_AP_AVG && day.ap_avg == W4_R2P_SW_DAY_AP_AVG &&
              day.has_f107_obs == W4_R2P_SW_DAY_HAS_F107_OBS,
          "space-weather day fields");
    check_exact_bits(day.f107_obs, W4_R2P_SW_DAY_F107_OBS_BITS, "space-weather day F10.7");
    size_t written = 0, required = 0;
    check(sidereon_space_weather_table_to_csv(table, NULL, 0, &written, &required) ==
              SIDEREON_STATUS_OK &&
              required == len,
          "space-weather CSV round-trip size");

    SidereonDecayConfig dcfg;
    check(sidereon_decay_config_init(&dcfg) == SIDEREON_STATUS_OK, "decay config init");
    SidereonSpaceWeather quiet;
    check(sidereon_space_weather_default(&quiet) == SIDEREON_STATUS_OK,
          "default space weather");
    check(sidereon_drag_parameters_from_bc_factor(0.8, quiet, 100.0, &dcfg.drag) ==
              SIDEREON_STATUS_OK,
          "decay drag");
    dcfg.abs_tol = 1.0e-8;
    dcfg.rel_tol = 1.0e-10;
    dcfg.initial_step_s = 5.0;
    dcfg.min_step_s = 1.0e-6;
    dcfg.max_step_s = 30.0;
    dcfg.max_steps = 200000;
    dcfg.scan_step_s = 60.0;
    dcfg.crossing_tolerance_s = 2.0;
    dcfg.max_duration_s = 50000.0;
    dcfg.max_scan_samples = 2000;
    SidereonCartesianState initial;
    memset(&initial, 0, sizeof(initial));
    initial.epoch_s = j2000(2003, 10, 30, 12, 0, 0.0);
    const double radius = 6378.137 + 125.0;
    initial.position_km[0] = radius;
    initial.velocity_km_s[1] = sqrt(398600.4418 / radius);
    SidereonDecayEstimate fixed, sourced;
    check(sidereon_estimate_decay(&initial, &dcfg, &fixed) == SIDEREON_STATUS_OK,
          "fixed decay estimate");
    check(sidereon_estimate_decay_with_space_weather_table(&initial, &dcfg, table, &sourced) ==
              SIDEREON_STATUS_OK,
          "table-backed decay estimate");
    check(sourced.time_to_decay_s > 0.0 && sourced.time_to_decay_s < fixed.time_to_decay_s,
          "table-backed decay direction");
    sidereon_space_weather_table_free(table);
    free(csv);
    free(csv_path);
}

static void test_ntrip(void) {
    SidereonNtripConfig cfg;
    memset(&cfg, 0, sizeof(cfg));
    cfg.host = "caster.example.test";
    cfg.port = 2101;
    cfg.mountpoint = "MOUNT";
    cfg.version = NTRIP_VERSION_REV2;
    cfg.has_credentials = true;
    cfg.username = "user";
    cfg.password = "pass";
    cfg.user_agent_product = "sidereon-test/0";
    size_t written = 0, required = 0;
    check(sidereon_ntrip_request_bytes(&cfg, NULL, 0, &written, &required) ==
              SIDEREON_STATUS_OK &&
              required == W4_R2P_NTRIP_REQUEST_LEN,
          "NTRIP request size");
    uint8_t req[W4_R2P_NTRIP_REQUEST_LEN];
    check(sidereon_ntrip_request_bytes(&cfg, req, sizeof(req), &written, &required) ==
              SIDEREON_STATUS_OK,
          "NTRIP request bytes");
    check_text(req, written, W4_R2P_NTRIP_REQUEST, W4_R2P_NTRIP_REQUEST_LEN,
               "NTRIP exact request");

    SidereonNtripMachine *machine = NULL;
    cfg.version = NTRIP_VERSION_REV1;
    cfg.has_credentials = false;
    cfg.has_gga_interval_s = true;
    cfg.gga_interval_s = 10.0;
    check(sidereon_ntrip_machine_new(&cfg, &machine) == SIDEREON_STATUS_OK && machine != NULL,
          "NTRIP machine new");
    SidereonNtripBytes *request_handle = NULL;
    check(sidereon_ntrip_machine_connection_request(machine, &request_handle) ==
              SIDEREON_STATUS_OK &&
              request_handle != NULL,
          "NTRIP machine request");
    size_t request_len = 0;
    uint8_t *request_bytes = copy_ntrip_bytes(request_handle, &request_len);
    check_text(request_bytes, request_len, W4_R2P_NTRIP_MACHINE_REQUEST,
               W4_R2P_NTRIP_MACHINE_REQUEST_LEN, "NTRIP machine request bytes");
    free(request_bytes);
    sidereon_ntrip_bytes_free(request_handle);

    const uint8_t wire[] = "ICY 200 OK\r\n\r\nabc";
    SidereonNtripEvents *events = NULL;
    check(sidereon_ntrip_machine_push(machine, wire, sizeof(wire) - 1, &events) ==
              SIDEREON_STATUS_OK &&
              events != NULL,
          "NTRIP machine push stream");
    size_t event_count = 0;
    check(sidereon_ntrip_events_count(events, &event_count) == SIDEREON_STATUS_OK &&
              event_count == W4_R2P_NTRIP_EVENT_COUNT,
          "NTRIP stream event count");
    SidereonNtripEventInfo info;
    check(sidereon_ntrip_events_event(events, 0, &info) == SIDEREON_STATUS_OK &&
              info.kind == W4_R2P_NTRIP_EVENT0_KIND && info.version == W4_R2P_NTRIP_EVENT0_VERSION,
          "NTRIP connected event");
    check(sidereon_ntrip_events_event(events, 1, &info) == SIDEREON_STATUS_OK &&
              info.kind == W4_R2P_NTRIP_EVENT1_KIND &&
              info.payload_len == W4_R2P_NTRIP_EVENT1_PAYLOAD_LEN,
          "NTRIP payload event");
    check(sidereon_ntrip_events_payload(events, 1, NULL, 0, &written, &required) ==
              SIDEREON_STATUS_OK &&
              required == W4_R2P_NTRIP_EVENT1_PAYLOAD_LEN,
          "NTRIP payload size");
    uint8_t payload[W4_R2P_NTRIP_EVENT1_PAYLOAD_LEN];
    check(sidereon_ntrip_events_payload(events, 1, payload, sizeof(payload), &written,
                                        &required) == SIDEREON_STATUS_OK,
          "NTRIP payload copy");
    check_text(payload, written, W4_R2P_NTRIP_EVENT1_PAYLOAD, W4_R2P_NTRIP_EVENT1_PAYLOAD_LEN,
               "NTRIP payload bytes");
    sidereon_ntrip_events_free(events);
    uint32_t state = 0;
    check(sidereon_ntrip_machine_state(machine, &state) == SIDEREON_STATUS_OK &&
              state == W4_R2P_NTRIP_STATE,
          "NTRIP streaming state");
    SidereonNtripGgaPosition pos = {40.0, -105.0, 1600.0, 1, 10, 1.0};
    bool gga_present = false;
    SidereonNtripBytes *gga = NULL;
    check(sidereon_ntrip_machine_try_gga_message(machine, 5.0, &pos, 3661.239,
                                                 &gga_present, &gga) == SIDEREON_STATUS_OK &&
              gga_present == W4_R2P_NTRIP_GGA_PRESENT && gga != NULL,
          "NTRIP machine GGA due");
    size_t gga_len = 0;
    uint8_t *gga_bytes = copy_ntrip_bytes(gga, &gga_len);
    check_text(gga_bytes, gga_len, W4_R2P_NTRIP_GGA, W4_R2P_NTRIP_GGA_LEN,
               "NTRIP GGA exact bytes");
    free(gga_bytes);
    sidereon_ntrip_bytes_free(gga);
    sidereon_ntrip_machine_free(machine);

    const char *table_text =
        "STR;MOUNT;ID;RTCM 3;1004(1);2;GPS;NET;USA;40.1;-105.2;1;0;gen;none;B;N;9600;misc;with;semis\r\n"
        "CAS;caster.example.test;2101;Caster;Op;0;USA;40.0;-105.0;backup.example.test;2102;cas misc\r\n"
        "NET;NET;Op;D;Y;https://net;https://str;https://reg;net misc\r\n"
        "ENDSOURCETABLE\r\n";
    SidereonNtripSourcetable *st = NULL;
    check(sidereon_ntrip_sourcetable_parse((const uint8_t *)table_text, strlen(table_text),
                                           &st) == SIDEREON_STATUS_OK &&
              st != NULL,
          "NTRIP sourcetable parse");
    SidereonNtripSourcetableSummary stsum;
    check(sidereon_ntrip_sourcetable_summary(st, &stsum) == SIDEREON_STATUS_OK &&
              stsum.record_count == W4_R2P_NTRIP_SOURCETABLE_RECORD_COUNT &&
              stsum.stream_count == W4_R2P_NTRIP_SOURCETABLE_STREAM_COUNT,
          "NTRIP sourcetable summary");
    SidereonNtripStreamInfo stream;
    check(sidereon_ntrip_sourcetable_streams(st, &stream, 1, &written, &required) ==
              SIDEREON_STATUS_OK &&
              written == 1,
          "NTRIP sourcetable stream");
    check(strcmp(stream.mountpoint, W4_R2P_NTRIP_STREAM_MOUNTPOINT) == 0 &&
              stream.has_lat_deg == W4_R2P_NTRIP_STREAM_HAS_LAT_DEG &&
              stream.has_bitrate == W4_R2P_NTRIP_STREAM_HAS_BITRATE &&
              stream.bitrate == W4_R2P_NTRIP_STREAM_BITRATE,
          "NTRIP stream fields");
    check_exact_bits(stream.lat_deg, W4_R2P_NTRIP_STREAM_LAT_DEG_BITS, "NTRIP stream latitude");
    check(sidereon_ntrip_sourcetable_to_text(st, NULL, 0, &written, &required) ==
              SIDEREON_STATUS_OK &&
              required == W4_R2P_NTRIP_SOURCETABLE_TEXT_LEN,
          "NTRIP sourcetable to_text size");
    sidereon_ntrip_sourcetable_free(st);
}

int main(int argc, char **argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: %s SIDEREON_CORE_FIXTURES\n", argv[0]);
        return 2;
    }
    test_covariance();
    test_cnav(argv[1]);
    test_tle_fit();
    test_qc(argv[1]);
    test_geoid();
    test_nmea();
    test_space_weather(argv[1]);
    test_ntrip();
    return failures == 0 ? 0 : 1;
}
