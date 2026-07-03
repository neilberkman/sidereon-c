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

#ifndef M_PI
#define M_PI 3.14159265358979323846
#endif

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
    NTRIP_EVENT_CONNECTED = 0,
    NTRIP_EVENT_PAYLOAD = 1,
    NTRIP_EVENT_SOURCETABLE = 2,
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
        fprintf(stderr, "FAIL: %s (last_error: %s)\n", what, msg);
        failures++;
    }
}

static void check_close(double actual, double expected, double tol, const char *what) {
    if (!(isfinite(actual) && fabs(actual - expected) <= tol)) {
        fprintf(stderr, "FAIL: %s (got %.17g expected %.17g tol %.3g)\n", what, actual,
                expected, tol);
        failures++;
    }
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
              written == 0 && required == 2,
          "covariance transport size");
    check(sidereon_covariance_transport(&p0, &seg, 1, noise, out, 2, &written,
                                        &required) == SIDEREON_STATUS_OK &&
              written == 2 && required == 2,
          "covariance transport copy");
    check_close(out[0].values[0][0], 1.0, 0.0, "cov initial P00");
    check_close(out[1].values[0][0], 1.0003333333333333, 1e-15, "cov P00");
    check_close(out[1].values[1][1], 2.0006666666666666, 1e-15, "cov P11");
    check_close(out[1].values[2][2], 3.001, 1e-15, "cov P22");
    check_close(out[1].values[0][3], 5.0e-5, 1e-18, "cov P03");
    check_close(out[1].values[1][4], 1.0e-4, 1e-18, "cov P14");
    check_close(out[1].values[2][5], 1.5e-4, 1e-18, "cov P25");
    check_close(out[1].values[3][3], 0.01001, 1e-17, "cov P33");
    check_close(out[1].values[4][4], 0.02002, 1e-17, "cov P44");
    check_close(out[1].values[5][5], 0.03003, 1e-17, "cov P55");

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
    check(sidereon_covariance_ephemeris_count(eph, &count) == SIDEREON_STATUS_OK &&
              count == 1,
          "covariance ephemeris count");
    SidereonCovarianceMatrix6 at0;
    check(sidereon_covariance_ephemeris_covariance_at(eph, 0.0, &at0) == SIDEREON_STATUS_OK,
          "covariance at initial epoch");
    check_close(at0.values[2][2], 3.0, 0.0, "covariance initial pinned");
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
    size_t count = 0;
    check(sidereon_broadcast_ephemeris_record_count(nav, &count) == SIDEREON_STATUS_OK &&
              count == 4,
          "CNAV record count");
    SidereonBroadcastRecordInfo records[8];
    size_t written = 0, required = 0;
    check(sidereon_broadcast_ephemeris_records(nav, records, 8, &written, &required) ==
              SIDEREON_STATUS_OK &&
              written == 4 && required == 4,
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
        check(r.issue == 288 && r.issue_message == NAV_MESSAGE_QZSS_CNAV && r.week == 2425 &&
                  r.toe_week == 2425 && fabs(r.toe_tow_s - 86400.0) < 1e-12,
              "J02 CNAV issue/week");
        check(r.cnav.present && r.cnav.ura_ed_index == -8 && r.cnav.ura_ned0_index == -3 &&
                  r.cnav.ura_ned1_index == 0 && r.cnav.ura_ned2_index == 0,
              "J02 CNAV URA indices");
        check_close(r.cnav.adot_m_s, 7.648849487305e-02, 1e-15, "J02 CNAV adot");
        check_close(r.cnav.delta_n0_dot_rad_s2, -2.609579611854e-13, 1e-25,
                    "J02 CNAV dn dot");
        bool present = false;
        double ura = 0.0;
        check(sidereon_cnav_ura_nominal_m(0, &ura, &present) == SIDEREON_STATUS_OK &&
                  present,
              "CNAV URA nominal");
        check_close(ura, 2.0, 0.0, "CNAV URA index 0");
        double ned = 0.0;
        check(sidereon_cnav_ura_ned_m(&r.cnav, 2425, 86400.0, &ned, &present) ==
                  SIDEREON_STATUS_OK &&
                  present,
              "CNAV URA NED");
        check_close(ned, 0.707106781186548, 1e-15, "CNAV URA NED at toe");
        double delay = 0.0;
        check(sidereon_broadcast_ephemeris_record_group_delay(
                  nav, (size_t)j02_cnav, GROUP_DELAY_CNAV_ISC_L2C, &delay, &present) ==
                  SIDEREON_STATUS_OK &&
                  present,
              "CNAV L2C ISC");
        check_close(delay, -8.73114913702e-10, 1e-22, "CNAV L2C ISC value");
        double corr = 0.0;
        check(sidereon_broadcast_ephemeris_record_cnav_correction(
                  nav, (size_t)j02_cnav, CNAV_SIGNAL_L1CA, &corr, &present) ==
                  SIDEREON_STATUS_OK &&
                  present,
              "CNAV L1CA correction");
        check_close(corr, 3.201421350241e-10, 1e-22, "CNAV L1CA correction value");
    }
    if (j02_cnav2 >= 0) {
        SidereonBroadcastRecordInfo r = records[j02_cnav2];
        check(r.issue == 288 && r.issue_message == NAV_MESSAGE_QZSS_CNAV2 &&
                  r.cnav.present && fabs(r.cnav.transmission_time_sow - 82872.0) < 1e-12,
              "J02 CNAV2 issue/transmission");
        bool present = false;
        double delay = 0.0;
        check(sidereon_broadcast_ephemeris_record_group_delay(
                  nav, (size_t)j02_cnav2, GROUP_DELAY_CNAV_ISC_L1CP, &delay, &present) ==
                  SIDEREON_STATUS_OK &&
                  present,
              "CNAV2 L1CP ISC");
        check_close(delay, -1.164153218269e-10, 1e-22, "CNAV2 L1CP ISC value");
        double corr = 0.0;
        check(sidereon_broadcast_ephemeris_record_cnav_correction(
                  nav, (size_t)j02_cnav2, CNAV_SIGNAL_L1CP, &corr, &present) ==
                  SIDEREON_STATUS_OK &&
                  present,
              "CNAV2 L1CP correction");
        check_close(corr, 3.783497959375e-10, 1e-22, "CNAV2 L1CP correction value");
        SidereonBroadcastRecordInfo selected;
        check(sidereon_broadcast_ephemeris_select_by_issue(
                  nav, "J02", 288, NAV_MESSAGE_QZSS_CNAV2, j2000(2026, 6, 29, 0, 0, 0.0),
                  &selected, &present) == SIDEREON_STATUS_OK &&
                  present && selected.message == NAV_MESSAGE_QZSS_CNAV2 &&
                  selected.issue == 288,
              "select J02 CNAV2 by issue");
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
    int64_t unix_us[3] = {1781670172099296LL, 1781670772099296LL, 1781671372099296LL};
    double jd_frac[3] = {0.6825474454089999, 0.6894918899051845, 0.6964363344013691};
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
        samples[i].jd_whole = 2461208.0;
        samples[i].jd_fraction = jd_frac[i];
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
    check_close(stats.rms_position_km, 2.2775403160369087e-05, 1e-16,
                "fit RMS position");
    check_close(stats.max_position_km, 2.9996961729023241e-05, 1e-16,
                "fit max position");
    check(stats.has_rms_velocity_km_s, "fit velocity RMS present");
    check_close(stats.rms_velocity_km_s, 1.8207442306018301e-08, 1e-19,
                "fit RMS velocity");
    check_close(stats.tle_rms_position_km, 6.4837293406693109e-05, 1e-16,
                "fit source TLE RMS");
    check(stats.rms_position_km < stats.tle_rms_position_km && stats.status == 3,
          "fit residual improvement");
    SidereonTleLines lines;
    check(sidereon_sgp4_tle_fit_lines(fit, &lines) == SIDEREON_STATUS_OK, "fit lines");
    check(strncmp(lines.line1.bytes, "1 25544U", 8) == 0 && strncmp(lines.line2.bytes, "2 25544", 7) == 0,
          "fit TLE catalog lines");
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
    check(summary.total_epoch_records == 120 && summary.observation_epochs == 120 &&
              summary.event_records == 0 && summary.skipped_records == 0 &&
              summary.has_interval_s && summary.interval_s == 30.0 &&
              summary.missing_epochs == 0 && summary.data_gap_count == 0 &&
              summary.satellite_signal_count == 659 && summary.system_signal_count == 78,
          "observation QC oracle summary");

    size_t written = 0, required = 0;
    check(sidereon_observation_qc_clock_jumps(report, NULL, 0, &written, &required) ==
                  SIDEREON_STATUS_OK &&
              written == 0 && required == 0,
          "observation QC clock jumps empty");

    SidereonObservationQcCycleSlips cycle_slips;
    check(sidereon_observation_qc_cycle_slips(report, &cycle_slips) == SIDEREON_STATUS_OK &&
              cycle_slips.observations == 4135 && cycle_slips.total_slips == 27 &&
              cycle_slips.has_observations_per_slip && cycle_slips.system_count == 4,
          "observation QC cycle-slip summary");
    check_close(cycle_slips.observations_per_slip, 4135.0 / 27.0, 1e-12,
                "QC observations per slip");

    SidereonObservationQcSystemCycleSlip slip_systems[4];
    written = 0;
    required = 0;
    check(sidereon_observation_qc_cycle_slip_systems(report, NULL, 0, &written,
                                                     &required) == SIDEREON_STATUS_OK &&
              written == 0 && required == 4,
          "observation QC cycle-slip systems size");
    check(sidereon_observation_qc_cycle_slip_systems(report, slip_systems, 4, &written,
                                                     &required) == SIDEREON_STATUS_OK &&
              written == 4 && required == 4,
          "observation QC cycle-slip systems copy");
    int saw_gps_slip = 0;
    int saw_glonass_slip = 0;
    int saw_galileo_slip = 0;
    int saw_beidou_slip = 0;
    for (size_t i = 0; i < written; i++) {
        const SidereonObservationQcSystemCycleSlip *row = &slip_systems[i];
        if (row->system == SIDEREON_GNSS_SYSTEM_GPS) {
            saw_gps_slip = 1;
            check(row->observations == 1282 && row->slips == 4 &&
                      row->has_observations_per_slip,
                  "QC GPS cycle slips");
            check_close(row->observations_per_slip, 1282.0 / 4.0, 1e-12,
                        "QC GPS observations per slip");
        } else if (row->system == SIDEREON_GNSS_SYSTEM_GLONASS) {
            saw_glonass_slip = 1;
            check(row->observations == 784 && row->slips == 10 &&
                      row->has_observations_per_slip,
                  "QC GLONASS cycle slips");
            check_close(row->observations_per_slip, 784.0 / 10.0, 1e-12,
                        "QC GLONASS observations per slip");
        } else if (row->system == SIDEREON_GNSS_SYSTEM_GALILEO) {
            saw_galileo_slip = 1;
            check(row->observations == 1023 && row->slips == 9 &&
                      row->has_observations_per_slip,
                  "QC Galileo cycle slips");
            check_close(row->observations_per_slip, 1023.0 / 9.0, 1e-12,
                        "QC Galileo observations per slip");
        } else if (row->system == SIDEREON_GNSS_SYSTEM_BEI_DOU) {
            saw_beidou_slip = 1;
            check(row->observations == 1046 && row->slips == 4 &&
                      row->has_observations_per_slip,
                  "QC BeiDou cycle slips");
            check_close(row->observations_per_slip, 1046.0 / 4.0, 1e-12,
                        "QC BeiDou observations per slip");
        }
    }
    check(saw_gps_slip && saw_glonass_slip && saw_galileo_slip && saw_beidou_slip,
          "observation QC cycle-slip systems present");

    SidereonObservationQcSystemMultipath mp_systems[4];
    written = 0;
    required = 0;
    check(sidereon_observation_qc_multipath_systems(report, NULL, 0, &written,
                                                    &required) == SIDEREON_STATUS_OK &&
              written == 0 && required == 4,
          "observation QC multipath systems size");
    check(sidereon_observation_qc_multipath_systems(report, mp_systems, 4, &written,
                                                    &required) == SIDEREON_STATUS_OK &&
              written == 4 && required == 4,
          "observation QC multipath systems copy");
    int saw_gps_mp = 0;
    int saw_glonass_mp = 0;
    int saw_galileo_mp = 0;
    int saw_beidou_mp = 0;
    for (size_t i = 0; i < written; i++) {
        const SidereonObservationQcSystemMultipath *row = &mp_systems[i];
        check(row->has_mp1 && row->has_mp2, "QC multipath system has MP1 and MP2");
        if (row->system == SIDEREON_GNSS_SYSTEM_GPS) {
            saw_gps_mp = 1;
            check(row->mp1.n == 1282 && row->mp2.n == 1282, "QC GPS multipath counts");
            check_close(row->mp1.rms_m, 0.29240479301672934, 1e-12, "QC GPS MP1");
            check_close(row->mp2.rms_m, 0.28099636987578613, 1e-12, "QC GPS MP2");
        } else if (row->system == SIDEREON_GNSS_SYSTEM_GLONASS) {
            saw_glonass_mp = 1;
            check(row->mp1.n == 784 && row->mp2.n == 784, "QC GLONASS multipath counts");
            check_close(row->mp1.rms_m, 0.5186943851125804, 1e-12, "QC GLONASS MP1");
            check_close(row->mp2.rms_m, 0.3144151269762753, 1e-12, "QC GLONASS MP2");
        } else if (row->system == SIDEREON_GNSS_SYSTEM_GALILEO) {
            saw_galileo_mp = 1;
            check(row->mp1.n == 1023 && row->mp2.n == 1023,
                  "QC Galileo multipath counts");
            check_close(row->mp1.rms_m, 0.3864051258642349, 1e-12, "QC Galileo MP1");
            check_close(row->mp2.rms_m, 0.4834568175024186, 1e-12, "QC Galileo MP2");
        } else if (row->system == SIDEREON_GNSS_SYSTEM_BEI_DOU) {
            saw_beidou_mp = 1;
            check(row->mp1.n == 1046 && row->mp2.n == 1046, "QC BeiDou multipath counts");
            check_close(row->mp1.rms_m, 1.0173872172139768, 1e-12, "QC BeiDou MP1");
            check_close(row->mp2.rms_m, 1.1736185873490712, 1e-12, "QC BeiDou MP2");
        }
    }
    check(saw_gps_mp && saw_glonass_mp && saw_galileo_mp && saw_beidou_mp,
          "observation QC multipath systems present");

    written = 0;
    required = 0;
    check(sidereon_observation_qc_multipath_satellites(report, NULL, 0, &written,
                                                       &required) == SIDEREON_STATUS_OK &&
              written == 0 && required == 40,
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
            check(mp_sats[i].has_mp1 && mp_sats[i].has_mp2 && mp_sats[i].mp1.n == 120 &&
                      mp_sats[i].mp2.n == 120,
                  "QC G08 multipath counts");
            check_close(mp_sats[i].mp1.rms_m, 0.45634723176158526, 1e-12,
                        "QC G08 MP1");
            check_close(mp_sats[i].mp2.rms_m, 0.6360867914911793, 1e-12,
                        "QC G08 MP2");
        }
    }
    check(saw_g08_mp, "observation QC multipath satellite G08 present");
    free(mp_sats);

    size_t text_len = 0;
    char *text_report =
        copy_qc_report_string(report, sidereon_observation_qc_render_text, "QC render_text",
                              &text_len);
    check(text_len > 100 && strstr(text_report, "G   GPS") != NULL &&
              strstr(text_report, "R   GLONASS") != NULL &&
              strstr(text_report, "E   Galileo") != NULL &&
              strstr(text_report, "C   BeiDou") != NULL &&
              strstr(text_report, "S   SBAS") != NULL,
          "observation QC render_text constellation rows");
    free(text_report);

    size_t html_len = 0;
    char *html_report =
        copy_qc_report_string(report, sidereon_observation_qc_render_html, "QC render_html",
                              &html_len);
    check(html_len > 100 && strstr(html_report, "<td class=\"text\">GPS</td>") != NULL &&
              strstr(html_report, "<td>0.292</td>") != NULL &&
              strstr(html_report, "<td>1.174</td>") != NULL,
          "observation QC render_html smoke");
    free(html_report);

    size_t json_len = 0;
    char *json_report =
        copy_qc_report_string(report, sidereon_observation_qc_to_json, "QC to_json", &json_len);
    check(json_len > 100 && strstr(json_report, "\"cycle_slips\"") != NULL &&
              strstr(json_report, "\"multipath\"") != NULL &&
              strstr(json_report, "0.29240479301672934") != NULL,
          "observation QC to_json smoke");
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
    check(lsum.finding_count == 1 && lsum.error_count == 1 && lsum.warning_count == 0 &&
              lsum.decoded_from_crinex,
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
    check(rsum.finding_count == 0 && rsum.decoded_from_crinex, "repair remaining clean");
    written = 0;
    required = 0;
    check(sidereon_rinex_repair_crinex_text(repair, NULL, 0, &written, &required) ==
              SIDEREON_STATUS_OK &&
              required > 100,
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
              written == 3 && required == 3,
          "EGM96 undulations batch");
    check_close(out[0], 17.16, 1e-12, "EGM96 0 0");
    check_close(out[1], -102.69, 1e-12, "EGM96 0 80");
    check_close(out[2], 63.80, 1e-12, "EGM96 60 -30");
    double orthometric = 0.0;
    double ellipsoidal = 0.0;
    check(sidereon_egm96_orthometric_height_m(100.0, 0.0, 0.0, &orthometric) ==
              SIDEREON_STATUS_OK,
          "EGM96 orthometric conversion");
    check_close(orthometric, 82.84, 1e-12, "EGM96 orthometric value");
    check(sidereon_egm96_ellipsoidal_height_m(orthometric, 0.0, 0.0, &ellipsoidal) ==
              SIDEREON_STATUS_OK,
          "EGM96 ellipsoidal conversion");
    check_close(ellipsoidal, 100.0, 1e-12, "EGM96 ellipsoidal round trip");
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
    check(summary.sentence_count == 2 && summary.epoch_count == 2 && summary.skip_count == 0 &&
              summary.warning_count == 0,
          "NMEA summary counts");
    SidereonNmeaEpochSummary epochs[2];
    size_t written = 0, required = 0;
    check(sidereon_nmea_log_epochs(log, epochs, 2, &written, &required) ==
              SIDEREON_STATUS_OK &&
              written == 2,
          "NMEA epochs");
    check(epochs[0].has_position && epochs[0].sentence_count == 1 &&
              epochs[0].used_satellite_count == 0,
          "NMEA first epoch summary");
    check_close(epochs[0].position.lat_rad, 48.1173 * M_PI / 180.0, 1e-12,
                "NMEA latitude");
    check_close(epochs[0].position.lon_rad, 11.516666666666667 * M_PI / 180.0, 1e-12,
                "NMEA longitude");
    check_close(epochs[0].position.height_m, 592.3, 1e-12, "NMEA ellipsoidal height");
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
              summary.sentence_count == 2 && summary.epoch_count == 2,
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
              required == 70,
          "NMEA GGA size");
    uint8_t buf[96];
    check(sidereon_nmea_write_gga(&gga, buf, sizeof(buf), &written, &required) ==
              SIDEREON_STATUS_OK,
          "NMEA GGA write");
    const char *expected =
        "$GPGGA,123519.00,4807.038,N,01131.000,E,1,08,0.90,592.3,M,0.0,M,,*6F\r\n";
    check(written == strlen(expected) && memcmp(buf, expected, written) == 0,
          "NMEA GGA exact bytes");
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
    check(summary.day_count == 14 && summary.monthly_count == 2 && summary.skip_count == 0 &&
              summary.warning_count == 0,
          "space-weather fixture counts");
    SidereonSpaceWeatherCoverage coverage;
    check(sidereon_space_weather_table_coverage(table, &coverage) == SIDEREON_STATUS_OK,
          "space-weather coverage");
    check(coverage.has_last_observed_j2000_s && coverage.has_last_daily_predicted_j2000_s,
          "space-weather coverage flags");
    SidereonSpaceWeatherSample sample;
    check(sidereon_space_weather_table_sample_at(table, j2000(2026, 7, 1, 12, 0, 0.0),
                                                 &sample) == SIDEREON_STATUS_OK,
          "space-weather observed sample");
    check_close(sample.weather.f107, 202.6, 0.0, "space-weather f107");
    check_close(sample.weather.f107a, 145.9, 0.0, "space-weather f107a");
    check_close(sample.weather.ap, 12.0, 0.0, "space-weather ap");
    check(sample.class_ == 0 && !sample.ap_defaulted, "space-weather observed class");
    double ap[7] = {0.0};
    check(sidereon_space_weather_table_ap_array_at(table, j2000(2003, 10, 31, 13, 0, 0.0),
                                                   ap) == SIDEREON_STATUS_OK,
          "space-weather AP array");
    const double expected_ap[7] = {116.0, 154.0, 111.0, 154.0, 179.0, 183.125, 236.5};
    for (int i = 0; i < 7; i++) {
        check_close(ap[i], expected_ap[i], 0.0, "space-weather AP pinned");
    }
    SidereonSpaceWeatherDay day;
    bool present = false;
    check(sidereon_space_weather_table_day(table, 2026, 7, 1, &present, &day) ==
              SIDEREON_STATUS_OK &&
              present,
          "space-weather day lookup");
    check(day.has_ap_avg && day.ap_avg == 12 && day.has_f107_obs && day.f107_obs == 249.7,
          "space-weather day fields");
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
              required == 170,
          "NTRIP request size");
    uint8_t req[256];
    check(sidereon_ntrip_request_bytes(&cfg, req, sizeof(req), &written, &required) ==
              SIDEREON_STATUS_OK,
          "NTRIP request bytes");
    const char *expected_req =
        "GET /MOUNT HTTP/1.1\r\n"
        "Host: caster.example.test:2101\r\n"
        "Ntrip-Version: Ntrip/2.0\r\n"
        "User-Agent: NTRIP sidereon-test/0\r\n"
        "Authorization: Basic dXNlcjpwYXNz\r\n"
        "Connection: close\r\n\r\n";
    check(written == strlen(expected_req) && memcmp(req, expected_req, written) == 0,
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
    check(request_len > 0 && memcmp(request_bytes, "GET /MOUNT HTTP/1.0\r\n", 21) == 0,
          "NTRIP machine request bytes");
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
              event_count == 2,
          "NTRIP stream event count");
    SidereonNtripEventInfo info;
    check(sidereon_ntrip_events_event(events, 0, &info) == SIDEREON_STATUS_OK &&
              info.kind == NTRIP_EVENT_CONNECTED && info.version == NTRIP_VERSION_REV1,
          "NTRIP connected event");
    check(sidereon_ntrip_events_event(events, 1, &info) == SIDEREON_STATUS_OK &&
              info.kind == NTRIP_EVENT_PAYLOAD && info.payload_len == 3,
          "NTRIP payload event");
    check(sidereon_ntrip_events_payload(events, 1, NULL, 0, &written, &required) ==
              SIDEREON_STATUS_OK &&
              required == 3,
          "NTRIP payload size");
    uint8_t payload[3];
    check(sidereon_ntrip_events_payload(events, 1, payload, sizeof(payload), &written,
                                        &required) == SIDEREON_STATUS_OK &&
              memcmp(payload, "abc", 3) == 0,
          "NTRIP payload bytes");
    sidereon_ntrip_events_free(events);
    uint32_t state = 0;
    check(sidereon_ntrip_machine_state(machine, &state) == SIDEREON_STATUS_OK &&
              state == NTRIP_STATE_STREAMING,
          "NTRIP streaming state");
    SidereonNtripGgaPosition pos = {40.0, -105.0, 1600.0, 1, 10, 1.0};
    bool gga_present = false;
    SidereonNtripBytes *gga = NULL;
    check(sidereon_ntrip_machine_try_gga_message(machine, 5.0, &pos, 3661.239,
                                                 &gga_present, &gga) == SIDEREON_STATUS_OK &&
              gga_present && gga != NULL,
          "NTRIP machine GGA due");
    size_t gga_len = 0;
    uint8_t *gga_bytes = copy_ntrip_bytes(gga, &gga_len);
    const char *expected_gga =
        "$GPGGA,010101.23,4000.0000000,N,10500.0000000,W,1,10,1.00,1600.0,M,,,,*2A\r\n";
    check(gga_len == strlen(expected_gga) && memcmp(gga_bytes, expected_gga, gga_len) == 0,
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
              stsum.record_count == 3 && stsum.stream_count == 1,
          "NTRIP sourcetable summary");
    SidereonNtripStreamInfo stream;
    check(sidereon_ntrip_sourcetable_streams(st, &stream, 1, &written, &required) ==
              SIDEREON_STATUS_OK &&
              written == 1,
          "NTRIP sourcetable stream");
    check(strcmp(stream.mountpoint, "MOUNT") == 0 && stream.has_lat_deg &&
              fabs(stream.lat_deg - 40.1) < 1e-12 && stream.has_bitrate &&
              stream.bitrate == 9600,
          "NTRIP stream fields");
    check(sidereon_ntrip_sourcetable_to_text(st, NULL, 0, &written, &required) ==
              SIDEREON_STATUS_OK &&
              required == 258,
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
