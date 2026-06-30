#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "constellation_fixture.h"
#include "prop_fixture.h"
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
    if (f == NULL) {
        check(0, "open input");
        return NULL;
    }
    if (fseek(f, 0, SEEK_END) != 0) {
        fclose(f);
        check(0, "seek input");
        return NULL;
    }
    long size = ftell(f);
    if (size < 0) {
        fclose(f);
        check(0, "tell input");
        return NULL;
    }
    rewind(f);
    uint8_t *buf = (uint8_t *)malloc((size_t)size + 1);
    if (buf == NULL) {
        fclose(f);
        check(0, "allocate input");
        return NULL;
    }
    size_t got = fread(buf, 1, (size_t)size, f);
    fclose(f);
    buf[got] = 0;
    *out_len = got;
    return buf;
}

static SidereonSp3 *load_sp3(const char *path) {
    size_t len = 0;
    uint8_t *bytes = read_file(path, &len);
    if (bytes == NULL) {
        return NULL;
    }
    SidereonSp3 *sp3 = NULL;
    check(sidereon_sp3_load(bytes, len, &sp3) == SIDEREON_STATUS_OK && sp3 != NULL,
          "sp3 load");
    free(bytes);
    return sp3;
}

static SidereonTle *load_tle(void) {
    SidereonTle *tle = NULL;
    check(sidereon_tle_load(PROP_TLE_LINE1, PROP_TLE_LINE2, PROP_TLE_OPSMODE, &tle) ==
              SIDEREON_STATUS_OK &&
              tle != NULL,
          "tle load");
    return tle;
}

static void test_forces_doppler_covariance(void) {
    double position[3] = {7000.0, -1210.0, 1300.0};
    double velocity[3] = {0.2, 7.2, 1.0};
    double twobody[3] = {0.0, 0.0, 0.0};
    double j2[3] = {0.0, 0.0, 0.0};
    check(sidereon_force_twobody_acceleration(position, velocity, twobody) ==
              SIDEREON_STATUS_OK &&
              isfinite(twobody[0]) && isfinite(twobody[1]) && isfinite(twobody[2]),
          "twobody force");
    check(sidereon_force_j2_acceleration(position, velocity, j2) == SIDEREON_STATUS_OK &&
              isfinite(j2[0]) && isfinite(j2[1]) && isfinite(j2[2]),
          "j2 force");

    SidereonTimeScales ts;
    check(sidereon_timescales_from_utc(2020, 6, 24, 0, 0, 0.0, &ts) == SIDEREON_STATUS_OK,
          "timescales for doppler");
    SidereonDopplerRangeRate range_rate;
    SidereonDopplerShift shift;
    check(sidereon_doppler_range_rate_and_ratio(position, velocity, 51.5, -0.1, 0.08, &ts,
                                                &range_rate) == SIDEREON_STATUS_OK &&
              isfinite(range_rate.range_rate_km_s) && isfinite(range_rate.doppler_ratio),
          "doppler range rate");
    check(sidereon_doppler_shift(position, velocity, 51.5, -0.1, 0.08, &ts, 1575.42e6, &shift) ==
              SIDEREON_STATUS_OK &&
              isfinite(shift.range_rate_km_s) && isfinite(shift.doppler_hz) &&
              isfinite(shift.doppler_ratio),
          "doppler shift");

    double cov[9] = {1.0, 0.1, 0.0, 0.1, 2.0, 0.2, 0.0, 0.2, 3.0};
    double eci[9] = {0.0};
    bool flag = false;
    check(sidereon_rtn_to_eci_covariance(cov, position, velocity, eci) == SIDEREON_STATUS_OK &&
              isfinite(eci[0]) && isfinite(eci[8]),
          "rtn covariance to eci");
    check(sidereon_covariance_is_symmetric(cov, &flag) == SIDEREON_STATUS_OK && flag,
          "covariance symmetric");
    check(sidereon_covariance_is_positive_semidefinite(cov, &flag) == SIDEREON_STATUS_OK && flag,
          "covariance positive semidefinite");
}

static void test_time_metadata(void) {
    uint8_t abbrev[8];
    size_t written = 0;
    size_t required = 0;
    check(sidereon_time_scale_abbrev(SIDEREON_TIME_SCALE_GPST, abbrev, sizeof(abbrev), &written,
                                     &required) == SIDEREON_STATUS_OK &&
              written == 4 && required == 4 && memcmp(abbrev, "GPST", 4) == 0,
          "time scale abbrev");

    double leap_seconds = 0.0;
    check(sidereon_leap_seconds(2020, 1, 1, &leap_seconds) == SIDEREON_STATUS_OK &&
              leap_seconds >= 37.0,
          "leap seconds");

    SidereonLeapSecondTableInfo leap_info;
    check(sidereon_leap_second_table_info(&leap_info) == SIDEREON_STATUS_OK &&
              leap_info.entries > 0 && leap_info.source_len > 0,
          "leap table info");
    check(sidereon_leap_second_table_source(NULL, 0, &written, &required) ==
              SIDEREON_STATUS_OK &&
              written == 0 && required == leap_info.source_len,
          "leap table source query");

    SidereonUt1CoverageInfo ut1_info;
    check(sidereon_ut1_coverage_info(&ut1_info) == SIDEREON_STATUS_OK && ut1_info.entries > 0 &&
              ut1_info.source_len > 0 && ut1_info.first_jd_tt < ut1_info.last_jd_tt,
          "ut1 coverage info");
    bool covered = false;
    double mid_jd_tt = 0.5 * (ut1_info.first_jd_tt + ut1_info.last_jd_tt);
    check(sidereon_ut1_coverage_covers_jd_tt(mid_jd_tt, &covered) == SIDEREON_STATUS_OK &&
              covered,
          "ut1 coverage contains midpoint");
    check(sidereon_ut1_coverage_source(NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
              written == 0 && required == ut1_info.source_len,
          "ut1 coverage source query");

    SidereonGnssWeekTow wt;
    SidereonGnssWeekTow normalized;
    check(sidereon_gnss_week_tow_new(SIDEREON_TIME_SCALE_GPST, 100, 604805.0, &wt) ==
              SIDEREON_STATUS_OK,
          "gnss week tow new");
    check(sidereon_gnss_week_tow_normalized(&wt, &normalized) == SIDEREON_STATUS_OK &&
              normalized.week == 101 && fabs(normalized.tow_s - 5.0) < 1.0e-9,
          "gnss week tow normalized");
    uint32_t unrolled = 0;
    check(sidereon_gnss_week_tow_unrolled_week(&normalized, 2, &unrolled) ==
              SIDEREON_STATUS_OK &&
              unrolled == 2149,
          "gnss week tow unrolled");
    bool present = false;
    int64_t jdn = 0;
    uint32_t week = 0;
    check(sidereon_gnss_week_epoch_julian_day_number(SIDEREON_TIME_SCALE_GPST, &present, &jdn) ==
              SIDEREON_STATUS_OK &&
              present && jdn > 0,
          "gnss week epoch");
    check(sidereon_gnss_week_from_calendar(SIDEREON_TIME_SCALE_GPST, 2020, 6, 24, &present,
                                           &week) == SIDEREON_STATUS_OK &&
              present && week > 2000,
          "gnss week from calendar");
    double sow = 0.0;
    double split_week = 0.0;
    check(sidereon_gnss_seconds_of_week_from_calendar(2020, 6, 24, 1, 2, 3, &sow) ==
              SIDEREON_STATUS_OK &&
              isfinite(sow),
          "gnss seconds of week");
    check(sidereon_gnss_week_and_seconds_of_week(604800.0 * 3.0 + 42.0, &split_week, &sow) ==
              SIDEREON_STATUS_OK &&
              split_week == 3.0 && fabs(sow - 42.0) < 1.0e-9,
          "gnss week and seconds");
}

static void test_coverage_grid(SidereonTle *tle) {
    const SidereonTle *tles[1] = {tle};
    SidereonGroundStation stations[2] = {
        {PROP_STATION_LATITUDE_DEG, PROP_STATION_LONGITUDE_DEG, PROP_STATION_ALTITUDE_M},
        {40.7128, -74.0060, 10.0},
    };
    SidereonCoverageGrid *grid = NULL;
    check(sidereon_coverage_look_angles(tles, 1, stations, 2, PROP_EPOCHS_UNIX_US[0], &grid) ==
              SIDEREON_STATUS_OK &&
              grid != NULL,
          "coverage look angles");
    if (grid == NULL) {
        return;
    }
    size_t sats = 0;
    size_t station_count = 0;
    check(sidereon_coverage_grid_dimensions(grid, &sats, &station_count) == SIDEREON_STATUS_OK &&
              sats == 1 && station_count == 2,
          "coverage dimensions");
    SidereonCoverageLookAngle look;
    check(sidereon_coverage_grid_look_angle(grid, 0, 0, &look) == SIDEREON_STATUS_OK &&
              look.ok && isfinite(look.azimuth_deg) && isfinite(look.elevation_deg) &&
              isfinite(look.range_km),
          "coverage cell");
    bool mask[2] = {false, false};
    size_t written = 0;
    size_t required = 0;
    check(sidereon_coverage_grid_visible_mask(grid, -90.0, mask, 2, &written, &required) ==
              SIDEREON_STATUS_OK &&
              written == 2 && required == 2,
          "coverage visible mask");
    size_t counts[2] = {0, 0};
    check(sidereon_coverage_grid_access_counts(grid, -90.0, counts, 2, &written, &required) ==
              SIDEREON_STATUS_OK &&
              written == 2 && required == 2 && counts[0] == 1,
          "coverage access counts");
    double max_el[2] = {0.0, 0.0};
    check(sidereon_coverage_grid_max_elevation_deg(grid, max_el, 2, &written, &required) ==
              SIDEREON_STATUS_OK &&
              written == 2 && required == 2 && isfinite(max_el[0]),
          "coverage max elevation");
    sidereon_coverage_grid_free(grid);
}

static void test_constellation_helpers(SidereonSp3 *sp3) {
    bool present = false;
    uint16_t u16_value = 0;
    int8_t i8_value = 0;
    check(sidereon_constellation_galileo_prn_for_gsat(210, &present, &u16_value) ==
              SIDEREON_STATUS_OK &&
              present && u16_value == 1,
          "galileo gsat helper");
    check(sidereon_constellation_glonass_slot_for_number(730, &present, &u16_value) ==
              SIDEREON_STATUS_OK &&
              present && u16_value == 1,
          "glonass slot helper");
    check(sidereon_constellation_glonass_fdma_channel(1, &present, &i8_value) ==
              SIDEREON_STATUS_OK &&
              present && i8_value == 1,
          "glonass fdma helper");

    SidereonConstellation *base = NULL;
    SidereonConstellation *with_status = NULL;
    check(sidereon_constellation_build(SIDEREON_GNSS_SYSTEM_GPS, CONSTELLATION_GPS_OPS_JSON,
                                       CONSTELLATION_GPS_OPS_JSON_LEN, NULL, 0, &base) ==
              SIDEREON_STATUS_OK &&
              base != NULL,
          "base catalog build");
    check(sidereon_constellation_build(SIDEREON_GNSS_SYSTEM_GPS, CONSTELLATION_GPS_OPS_JSON,
                                       CONSTELLATION_GPS_OPS_JSON_LEN, CONSTELLATION_NAVCEN_HTML,
                                       CONSTELLATION_NAVCEN_HTML_LEN, &with_status) ==
              SIDEREON_STATUS_OK &&
              with_status != NULL,
          "status catalog build");
    if (base == NULL || with_status == NULL) {
        sidereon_constellation_free(base);
        sidereon_constellation_free(with_status);
        return;
    }

    const char *all_ids[] = {"G03", "G05", "G13", "G19"};
    check(sidereon_constellation_validate_against_sp3_ids_strict(base, all_ids, 4) ==
              SIDEREON_STATUS_OK,
          "constellation strict ids");
    SidereonConstellationValidation *sp3_validation = NULL;
    check(sidereon_constellation_validate_against_sp3(with_status, sp3, &sp3_validation) ==
              SIDEREON_STATUS_OK &&
              sp3_validation != NULL,
          "constellation validate sp3");
    sidereon_constellation_validation_free(sp3_validation);

    SidereonConstellationDiff *diff = NULL;
    check(sidereon_constellation_diff(base, with_status, &diff) == SIDEREON_STATUS_OK &&
              diff != NULL,
          "constellation diff");
    if (diff != NULL) {
        bool changed = false;
        SidereonConstellationDiffCounts counts;
        check(sidereon_constellation_diff_changed(diff, &changed) == SIDEREON_STATUS_OK &&
                  changed,
              "constellation diff changed");
        check(sidereon_constellation_diff_counts(diff, &counts) == SIDEREON_STATUS_OK &&
                  counts.usability_changed >= 1,
              "constellation diff counts");
        SidereonConstellationBoolChange usability[4];
        size_t written = 0;
        size_t required = 0;
        check(sidereon_constellation_diff_usability_changed(diff, usability, 4, &written,
                                                            &required) == SIDEREON_STATUS_OK &&
                  written == counts.usability_changed && required == counts.usability_changed,
              "constellation diff usability");
        SidereonConstellationRecord empty_records[1];
        check(sidereon_constellation_diff_added(diff, empty_records, 1, &written, &required) ==
                  SIDEREON_STATUS_OK &&
                  written == 0 && required == 0,
              "constellation diff added empty");
        sidereon_constellation_diff_free(diff);
    }

    SidereonConstellationDiff *same = NULL;
    check(sidereon_constellation_diff(base, base, &same) == SIDEREON_STATUS_OK && same != NULL,
          "constellation same diff");
    if (same != NULL) {
        bool changed = true;
        check(sidereon_constellation_diff_changed(same, &changed) == SIDEREON_STATUS_OK &&
                  !changed,
              "constellation same diff unchanged");
        sidereon_constellation_diff_free(same);
    }
    sidereon_constellation_free(with_status);
    sidereon_constellation_free(base);
}

static void test_piecewise_sources(SidereonSp3 *sp3, SidereonTle *tle) {
    SidereonReducedOrbitSourceFitOptions fit_options;
    memset(&fit_options, 0, sizeof(fit_options));
    fit_options.sampling.t0 = (SidereonCalendarEpoch){2020, 6, 24, 0, 0, 0.0};
    fit_options.sampling.t1 = (SidereonCalendarEpoch){2020, 6, 24, 3, 0, 0.0};
    fit_options.sampling.cadence_s = 900.0;
    fit_options.model = SIDEREON_REDUCED_ORBIT_MODEL_CIRCULAR_SECULAR;

    SidereonReducedOrbitPiecewise *sp3_piecewise = NULL;
    SidereonReducedOrbitPiecewiseSourceFitStats sp3_stats;
    check(sidereon_reduced_orbit_fit_piecewise_sp3_source(sp3, "G01", &fit_options, 3600.0,
                                                          &sp3_piecewise, &sp3_stats) ==
              SIDEREON_STATUS_OK &&
              sp3_piecewise != NULL && sp3_stats.requested_samples >= 4 &&
              sp3_stats.used_samples >= 4,
          "piecewise sp3 source fit");
    if (sp3_piecewise != NULL) {
        SidereonReducedOrbitPiecewiseInfo info;
        check(sidereon_reduced_orbit_piecewise_info(sp3_piecewise, &info) == SIDEREON_STATUS_OK &&
                  info.n_segments > 0,
              "piecewise sp3 source info");
        SidereonReducedOrbitSourceDriftOptions drift_options;
        memset(&drift_options, 0, sizeof(drift_options));
        drift_options.sampling.t0 = (SidereonCalendarEpoch){2020, 6, 24, 0, 0, 0.0};
        drift_options.sampling.t1 = (SidereonCalendarEpoch){2020, 6, 24, 4, 0, 0.0};
        drift_options.sampling.cadence_s = 900.0;
        drift_options.threshold_m = 1.0e9;
        SidereonReducedOrbitDriftReport *report = NULL;
        check(sidereon_reduced_orbit_piecewise_drift_sp3_source(sp3_piecewise, sp3, "G01",
                                                                &drift_options, &report) ==
                  SIDEREON_STATUS_OK &&
                  report != NULL,
              "piecewise sp3 source drift");
        if (report != NULL) {
            SidereonReducedOrbitDriftSummary summary;
            check(sidereon_reduced_orbit_drift_report_summary(report, &summary) ==
                      SIDEREON_STATUS_OK &&
                      isfinite(summary.max_m) && isfinite(summary.rms_m),
                  "piecewise sp3 source drift summary");
            sidereon_reduced_orbit_drift_report_free(report);
        }
        sidereon_reduced_orbit_piecewise_free(sp3_piecewise);
    }

    fit_options.sampling.t0 = (SidereonCalendarEpoch){2018, 7, 3, 19, 30, 0.0};
    fit_options.sampling.t1 = (SidereonCalendarEpoch){2018, 7, 3, 22, 30, 0.0};
    fit_options.sampling.cadence_s = 600.0;

    SidereonReducedOrbitPiecewise *tle_piecewise = NULL;
    SidereonReducedOrbitPiecewiseSourceFitStats tle_stats;
    check(sidereon_reduced_orbit_fit_piecewise_tle_source(tle, &fit_options, 3600.0,
                                                          &tle_piecewise, &tle_stats) ==
              SIDEREON_STATUS_OK &&
              tle_piecewise != NULL && tle_stats.used_samples >= 4,
          "piecewise tle source fit");
    if (tle_piecewise != NULL) {
        SidereonReducedOrbitSourceDriftOptions drift_options;
        memset(&drift_options, 0, sizeof(drift_options));
        drift_options.sampling.t0 = (SidereonCalendarEpoch){2018, 7, 3, 19, 30, 0.0};
        drift_options.sampling.t1 = (SidereonCalendarEpoch){2018, 7, 3, 22, 30, 0.0};
        drift_options.sampling.cadence_s = 600.0;
        drift_options.threshold_m = 1.0e9;
        SidereonReducedOrbitDriftReport *report = NULL;
        check(sidereon_reduced_orbit_piecewise_drift_tle_source(tle_piecewise, tle,
                                                                &drift_options, &report) ==
                  SIDEREON_STATUS_OK &&
                  report != NULL,
              "piecewise tle source drift");
        sidereon_reduced_orbit_drift_report_free(report);
        sidereon_reduced_orbit_piecewise_free(tle_piecewise);
    }
}

int main(int argc, char **argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: %s <sp3>\n", argv[0]);
        return 2;
    }

    SidereonSp3 *sp3 = load_sp3(argv[1]);
    SidereonTle *tle = load_tle();
    test_forces_doppler_covariance();
    test_time_metadata();
    if (tle != NULL) {
        test_coverage_grid(tle);
    }
    if (sp3 != NULL) {
        test_constellation_helpers(sp3);
    }
    if (sp3 != NULL && tle != NULL) {
        test_piecewise_sources(sp3, tle);
    }
    sidereon_tle_free(tle);
    sidereon_sp3_free(sp3);
    return failures == 0 ? 0 : 1;
}
