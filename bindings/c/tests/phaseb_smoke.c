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

static void check_close(double got, double want, double tol, const char *what) {
    check(isfinite(got) && fabs(got - want) <= tol, what);
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
        return NULL;
    }
    long size = ftell(f);
    if (size < 0) {
        fclose(f);
        return NULL;
    }
    rewind(f);
    uint8_t *buf = (uint8_t *)malloc((size_t)size + 1);
    if (!buf) {
        fclose(f);
        return NULL;
    }
    size_t got = fread(buf, 1, (size_t)size, f);
    fclose(f);
    buf[got] = 0;
    *out_len = got;
    return buf;
}

static int hex_nibble(char c) {
    if (c >= '0' && c <= '9') {
        return c - '0';
    }
    if (c >= 'a' && c <= 'f') {
        return 10 + c - 'a';
    }
    if (c >= 'A' && c <= 'F') {
        return 10 + c - 'A';
    }
    return -1;
}

static size_t hex_to_bytes(const char *hex, uint8_t *out, size_t cap) {
    size_t n = strlen(hex);
    if ((n % 2) != 0 || cap < n / 2) {
        return 0;
    }
    for (size_t i = 0; i < n; i += 2) {
        int hi = hex_nibble(hex[i]);
        int lo = hex_nibble(hex[i + 1]);
        if (hi < 0 || lo < 0) {
            return 0;
        }
        out[i / 2] = (uint8_t)((hi << 4) | lo);
    }
    return n / 2;
}

static SidereonSp3 *load_sp3(const char *path) {
    size_t len = 0;
    uint8_t *bytes = read_file(path, &len);
    if (!bytes) {
        return NULL;
    }
    SidereonSp3 *sp3 = NULL;
    check(sidereon_sp3_load(bytes, len, &sp3) == SIDEREON_STATUS_OK && sp3 != NULL,
          "phaseb sp3 load");
    free(bytes);
    return sp3;
}

static SidereonSpk *load_spk(const char *path) {
    size_t len = 0;
    uint8_t *bytes = read_file(path, &len);
    if (!bytes) {
        return NULL;
    }
    SidereonSpk *spk = NULL;
    check(sidereon_spk_load(bytes, len, &spk) == SIDEREON_STATUS_OK && spk != NULL,
          "phaseb spk load");
    free(bytes);
    return spk;
}

static double first_sp3_epoch(const SidereonSp3 *sp3) {
    size_t written = 0;
    size_t required = 0;
    check(sidereon_sp3_epochs_j2000_seconds(sp3, NULL, 0, &written, &required) ==
              SIDEREON_STATUS_OK &&
              required > 0,
          "phaseb sp3 epoch query");
    double *epochs = (double *)calloc(required, sizeof(*epochs));
    if (!epochs) {
        check(0, "phaseb sp3 epoch allocation");
        return 0.0;
    }
    check(sidereon_sp3_epochs_j2000_seconds(sp3, epochs, required, &written, &required) ==
              SIDEREON_STATUS_OK &&
              written > 0,
          "phaseb sp3 epoch copy");
    double first = epochs[0];
    free(epochs);
    return first;
}

static void test_labels(void) {
    uint8_t buf[32];
    size_t written = 0;
    size_t required = 0;
    memset(buf, 0, sizeof(buf));
    check(sidereon_gnss_system_label(SIDEREON_GNSS_SYSTEM_GPS, buf, sizeof(buf), &written,
                                     &required) == SIDEREON_STATUS_OK &&
              written == 3 && required == 3 && memcmp(buf, "GPS", 3) == 0,
          "gnss system label delegates to core");
    memset(buf, 0, sizeof(buf));
    check(sidereon_carrier_band_label(SIDEREON_CARRIER_BAND_L1, buf, sizeof(buf), &written,
                                      &required) == SIDEREON_STATUS_OK &&
              written == 2 && required == 2 && memcmp(buf, "l1", 2) == 0,
          "carrier band label delegates to core");
}

static SidereonClassicalElements sample_coe(void) {
    SidereonClassicalElements coe;
    memset(&coe, 0, sizeof(coe));
    coe.a = 7000.0;
    coe.ecc = 0.01;
    coe.p = coe.a * (1.0 - coe.ecc * coe.ecc);
    coe.incl = 0.3;
    coe.raan = 0.2;
    coe.argp = 0.4;
    coe.nu = 0.5;
    coe.arglat = NAN;
    coe.truelon = NAN;
    coe.lonper = NAN;
    coe.orbit_type = SIDEREON_ORBIT_TYPE_ELLIPTICAL_INCLINED;
    return coe;
}

static void test_anomaly_and_equinoctial(void) {
    const double mu = 398600.4418;
    double ecc = 0.1;
    double mean = 0.75;
    double e_anom = 0.0;
    double mean_round = 0.0;
    double true_anom = 0.0;
    double true_round = 0.0;
    SidereonKeplerSolution solved;
    check(sidereon_mean_to_eccentric_anomaly(mean, ecc, &e_anom) == SIDEREON_STATUS_OK,
          "mean to eccentric anomaly");
    check(sidereon_eccentric_to_mean_anomaly(e_anom, ecc, &mean_round) == SIDEREON_STATUS_OK,
          "eccentric to mean anomaly");
    check_close(mean_round, mean, 1e-13, "anomaly mean roundtrip");
    check(sidereon_eccentric_to_true_anomaly(e_anom, ecc, &true_anom) == SIDEREON_STATUS_OK,
          "eccentric to true anomaly");
    check(sidereon_true_to_eccentric_anomaly(true_anom, ecc, &true_round) == SIDEREON_STATUS_OK,
          "true to eccentric anomaly");
    check_close(true_round, e_anom, 1e-13, "anomaly eccentric roundtrip");
    check(sidereon_solve_kepler(mean, ecc, &solved) == SIDEREON_STATUS_OK &&
              solved.iterations > 0,
          "solve kepler");
    check_close(solved.anomaly_rad, e_anom, 1e-13, "solve kepler value");

    SidereonClassicalElements coe = sample_coe();
    SidereonClassicalElements propagated;
    check(sidereon_propagate_kepler(&coe, mu, 0.0, &propagated) == SIDEREON_STATUS_OK,
          "propagate kepler");
    check_close(propagated.nu, coe.nu, 1e-13, "propagate kepler zero dt");

    SidereonEquinoctialElements eq;
    SidereonClassicalElements coe_from_eq;
    SidereonModifiedEquinoctialElements mee;
    SidereonClassicalElements coe_from_mee;
    double r[3];
    double v[3];
    double r2[3];
    double v2[3];
    check(sidereon_coe2eq(&coe, SIDEREON_RETROGRADE_FACTOR_PROGRADE, &eq) ==
              SIDEREON_STATUS_OK,
          "coe to equinoctial");
    check(sidereon_eq2coe(&eq, &coe_from_eq) == SIDEREON_STATUS_OK, "equinoctial to coe");
    check_close(coe_from_eq.a, coe.a, 1e-9, "equinoctial a roundtrip");
    check(sidereon_coe2mee(&coe, SIDEREON_RETROGRADE_FACTOR_PROGRADE, &mee) ==
              SIDEREON_STATUS_OK,
          "coe to modified equinoctial");
    check(sidereon_mee2coe(&mee, &coe_from_mee) == SIDEREON_STATUS_OK,
          "modified equinoctial to coe");
    check_close(coe_from_mee.ecc, coe.ecc, 1e-12, "modified equinoctial ecc roundtrip");
    check(sidereon_coe2rv(&coe, mu, r, v) == SIDEREON_STATUS_OK, "coe to rv");
    check(sidereon_rv2eq(r, v, mu, SIDEREON_RETROGRADE_FACTOR_PROGRADE, &eq) ==
              SIDEREON_STATUS_OK,
          "rv to equinoctial");
    check(sidereon_eq2rv(&eq, mu, r2, v2) == SIDEREON_STATUS_OK, "equinoctial to rv");
    check_close(r2[0], r[0], 1e-6, "equinoctial rv x");
    check(sidereon_rv2mee(r, v, mu, SIDEREON_RETROGRADE_FACTOR_PROGRADE, &mee) ==
              SIDEREON_STATUS_OK,
          "rv to modified equinoctial");
    check(sidereon_mee2rv(&mee, mu, r2, v2) == SIDEREON_STATUS_OK, "modified equinoctial to rv");
    check_close(v2[1], v[1], 1e-9, "modified equinoctial rv vy");
}

static void test_angles_and_relative(void) {
    double x[3] = {1.0, 0.0, 0.0};
    double y[3] = {0.0, 1.0, 0.0};
    double z[3] = {0.0, 0.0, 1.0};
    double out = 0.0;
    check(sidereon_angular_separation_deg(x, y, &out) == SIDEREON_STATUS_OK,
          "angular separation vectors");
    check_close(out, 90.0, 1e-12, "angular separation vectors value");
    check(sidereon_angular_separation_coords_deg(0.0, 0.0, 90.0, 0.0, &out) ==
              SIDEREON_STATUS_OK,
          "angular separation coords");
    check_close(out, 90.0, 1e-12, "angular separation coords value");
    check(sidereon_position_angle_deg(0.0, 0.0, 90.0, 0.0, &out) == SIDEREON_STATUS_OK,
          "position angle");
    check_close(out, 90.0, 1e-12, "position angle value");
    check(sidereon_beta_angle_deg(z, x, &out) == SIDEREON_STATUS_OK, "beta angle");
    check_close(out, 0.0, 1e-12, "beta angle value");

    SidereonCartesianState chief = {0};
    chief.position_km[0] = 7000.0;
    chief.velocity_km_s[1] = 7.5;
    SidereonCartesianState deputy = chief;
    deputy.position_km[0] += 1.0;
    deputy.position_km[1] += 2.0;
    deputy.velocity_km_s[1] += 0.01;
    double rotation[9];
    check(sidereon_relative_rotation(SIDEREON_RELATIVE_FRAME_RTN, &chief, rotation, 9) ==
              SIDEREON_STATUS_OK &&
              isfinite(rotation[0]),
          "relative frame rotation");
    SidereonCartesianState rel;
    SidereonCartesianState recovered;
    check(sidereon_relative_state(&chief, &deputy, &rel) == SIDEREON_STATUS_OK,
          "relative state");
    check(sidereon_absolute_from_relative(&chief, &rel, &recovered) == SIDEREON_STATUS_OK,
          "absolute from relative");
    check_close(recovered.position_km[1], deputy.position_km[1], 1e-9,
                "relative absolute roundtrip");
    double n = 0.0;
    double stm[36];
    check(sidereon_relative_mean_motion_circular(7000.0, &n) == SIDEREON_STATUS_OK && n > 0.0,
          "relative mean motion circular");
    check(sidereon_relative_mean_motion_from_state(&chief, &out) == SIDEREON_STATUS_OK &&
              out > 0.0,
          "relative mean motion state");
    check(sidereon_cw_stm(n, 0.0, stm, 36) == SIDEREON_STATUS_OK, "cw stm");
    check_close(stm[0], 1.0, 1e-15, "cw stm identity");
    check(sidereon_cw_propagate(&rel, n, 0.0, &recovered) == SIDEREON_STATUS_OK,
          "cw propagate");
    check_close(recovered.position_km[0], rel.position_km[0], 1e-15, "cw zero dt");
}

static void test_observe_and_almanac(SidereonSpk *spk) {
    SidereonGeodeticStation station = {51.4779, 0.0, 0.0};
    const int64_t jan1_2025 = 1735689600000000LL;
    const int64_t feb15_2025 = 1739577600000000LL;
    const int64_t apr1_2025 = 1743465600000000LL;
    SidereonObserveOptions options;
    SidereonBodyObservation obs;
    check(sidereon_observe_options_init(&options) == SIDEREON_STATUS_OK, "observe options init");
    check(sidereon_observe(&station, jan1_2025, SIDEREON_OBSERVE_TARGET_KIND_SUN, NULL, 0, NULL,
                           NULL, &options, &obs) == SIDEREON_STATUS_OK &&
              isfinite(obs.apparent.right_ascension_deg) && obs.apparent.distance_km > 0.0,
          "observe sun");
    check(sidereon_observe(&station, jan1_2025, SIDEREON_OBSERVE_TARGET_KIND_MOON, NULL, 0, NULL,
                           NULL, &options, &obs) == SIDEREON_STATUS_OK &&
              isfinite(obs.horizontal.azimuth_deg),
          "observe moon");
    if (spk) {
        check(sidereon_observe_spk_body(&station, jan1_2025, spk, 4, &obs) ==
                  SIDEREON_STATUS_OK &&
                  obs.astrometric.distance_km > 0.0,
              "observe spk body");
    }

    size_t written = 0;
    size_t required = 0;
    check(sidereon_almanac_seasons(NULL, jan1_2025, apr1_2025, 86400.0, 60.0, NULL, 0,
                                   &written, &required) == SIDEREON_STATUS_OK &&
              required >= 1,
          "almanac seasons query");
    SidereonSeasonEvent seasons[4];
    check(sidereon_almanac_seasons(NULL, jan1_2025, apr1_2025, 86400.0, 60.0, seasons, 4,
                                   &written, &required) == SIDEREON_STATUS_OK &&
              written >= 1,
          "almanac seasons fill");
    check(sidereon_almanac_moon_phases(NULL, jan1_2025, feb15_2025, 21600.0, 60.0, NULL, 0,
                                       &written, &required) == SIDEREON_STATUS_OK &&
              required >= 1,
          "almanac moon phases query");
    SidereonMoonPhaseEvent phases[8];
    check(sidereon_almanac_moon_phases(NULL, jan1_2025, feb15_2025, 21600.0, 60.0, phases, 8,
                                       &written, &required) == SIDEREON_STATUS_OK &&
              written >= 1,
          "almanac moon phases fill");
    SidereonPlanetaryEvent planet_events[4];
    check(sidereon_almanac_planetary_events(
              spk, SIDEREON_PLANET_MARS, SIDEREON_PLANETARY_EVENT_KIND_OPPOSITION, jan1_2025,
              feb15_2025, 21600.0, 60.0, planet_events, 4, &written, &required) ==
              SIDEREON_STATUS_OK,
          "almanac planetary events");
    SidereonMeridianTransit transits[4];
    check(sidereon_almanac_meridian_transits(NULL, SIDEREON_TRANSIT_BODY_KIND_SUN, 0, &station,
                                             jan1_2025, jan1_2025 + 86400000000LL, 3600.0, 10.0,
                                             transits, 4, &written, &required) ==
              SIDEREON_STATUS_OK &&
              written >= 1,
          "almanac meridian transits");
    SidereonAlmanacEclipseEvent eclipses[4];
    check(sidereon_almanac_lunar_solar_eclipses(NULL, jan1_2025, apr1_2025, 86400.0, 60.0,
                                                eclipses, 4, &written, &required) ==
              SIDEREON_STATUS_OK,
          "almanac eclipses");
}

static void test_drag_decay(void) {
    SidereonSpaceWeather weather;
    SidereonDragParameters drag;
    check(sidereon_space_weather_default(&weather) == SIDEREON_STATUS_OK, "space weather default");
    check(sidereon_drag_parameters_from_area_mass(2.2, 20.0, 100.0, weather, 90.0, &drag) ==
              SIDEREON_STATUS_OK &&
              drag.bc_factor_m2_kg > 0.0,
          "drag parameters area mass");
    SidereonCartesianState state = {0};
    state.position_km[0] = 6778.0;
    state.velocity_km_s[1] = 7.67;
    double accel[3];
    check(sidereon_drag_force_acceleration(&drag, &state, accel) == SIDEREON_STATUS_OK &&
              isfinite(accel[0]),
          "drag force acceleration");
    SidereonDecayConfig config;
    SidereonDecayEstimate estimate;
    check(sidereon_decay_config_init(&config) == SIDEREON_STATUS_OK, "decay config init");
    config.drag = drag;
    config.reentry_altitude_km = 500.0;
    check(sidereon_estimate_decay(&state, &config, &estimate) == SIDEREON_STATUS_OK &&
              estimate.time_to_decay_s == 0.0,
          "estimate decay initial below threshold");
}

static void test_ephemeris_sample(SidereonSp3 *sp3) {
    double t = first_sp3_epoch(sp3);
    const char *sats[] = {"G01"};
    size_t written = 0;
    size_t required = 0;
    check(sidereon_sp3_ephemeris_sample(sp3, sats, 1, t, t, 60.0, NULL, 0, &written,
                                        &required) == SIDEREON_STATUS_OK &&
              required == 1,
          "sp3 ephemeris sample query");
    SidereonEphemerisSampleRow row;
    check(sidereon_sp3_ephemeris_sample(sp3, sats, 1, t, t, 60.0, &row, 1, &written,
                                        &required) == SIDEREON_STATUS_OK &&
              written == 1 && row.status == SIDEREON_EPHEMERIS_SAMPLE_STATUS_VALID &&
              row.has_position_ecef_m,
          "sp3 ephemeris sample fill");
}

static void test_terrain(const char *dted_root, const char *dted_tile) {
    SidereonDtedLookupOptions options;
    check(sidereon_dted_lookup_options_init(&options) == SIDEREON_STATUS_OK &&
              options.interpolation == SIDEREON_DTED_INTERPOLATION_BILINEAR,
          "dted lookup options init");
    SidereonDtedTerrain *terrain = NULL;
    double h = 0.0;
    check(sidereon_dted_terrain_new(dted_root, &terrain) == SIDEREON_STATUS_OK && terrain != NULL,
          "dted terrain new");
    if (terrain) {
        check(sidereon_dted_terrain_height_m_with_options(terrain, -106.5, 36.5, &options, &h) ==
                  SIDEREON_STATUS_OK &&
                  isfinite(h),
              "dted terrain height");
        sidereon_dted_terrain_free(terrain);
    }
    SidereonDtedTile *tile = NULL;
    int16_t elev = 0;
    check(sidereon_dted_tile_load(dted_tile, &tile) == SIDEREON_STATUS_OK && tile != NULL,
          "dted tile load");
    if (tile) {
        check(sidereon_dted_tile_get_elevation(tile, -106.5, 36.5, &elev) ==
                  SIDEREON_STATUS_OK,
              "dted tile elevation");
        sidereon_dted_tile_free(tile);
    }
}

static const uint8_t EDGE_BIA[] =
    "%=BIA 1.00 TST\n"
    "+FILE/REFERENCE\n"
    " DESCRIPTION EDGE CASE PRODUCT\n"
    "-FILE/REFERENCE\n"
    "+BIAS/DESCRIPTION\n"
    " BIAS_MODE ABSOLUTE\n"
    " TIME_SYSTEM G\n"
    " SATELLITE_CLOCK_REFERENCE_OBSERVABLES G C1W C2W\n"
    " SATELLITE_CLOCK_REFERENCE_OBSERVABLES E C1C C5Q\n"
    " OBSERVATION_SAMPLING 30\n"
    " PARAMETER_SPACING 86400\n"
    " DETERMINATION_METHOD TEST\n"
    "-BIAS/DESCRIPTION\n"
    "+BIAS/SOLUTION 11\n"
    "*BIAS SVN_ PRN STATION__ OBS1 OBS2 BIAS_START____ BIAS_END______ UNIT "
    "__ESTIMATED_VALUE____ _STD_DEV___\n"
    " OSB  G063 G             C1C       2020:001:00000 2020:002:00000 ns      "
    "1.000000000000E-01 1.00000E-02\n"
    " OSB  G063 G01           C1C       2020:001:00000 2020:002:00000 ns     "
    "-1.234567890000E+00 2.00000E-02    8.640000000000E-01 1.00000E-02\n"
    " OSB  G063 G01           C1W       2020:001:00000 2020:002:00000 ns      "
    "5.600000000000E-01 2.00000E-02\n"
    " DSB  G063 G01           C1C  C1W  2020:001:00000 2020:002:00000 ns     "
    "-1.794567890000E+00 3.00000E-02\n"
    " ISB  G063 G01           C1C  C2W  2020:001:00000 2020:002:00000 ns      "
    "2.500000000000E-01 4.00000E-02\n"
    " OSB  G063 G01           L1C       2020:001:00000 2020:002:00000 cyc    "
    "-1.050000000000E-01 1.00000E-02\n"
    " OSB       G   ALGO      C1C       2020:001:00000 2020:002:00000 ns      "
    "3.100000000000E+00 5.00000E-02\n"
    " OSB       E   ALGO      C1C       2020:001:00000 2020:002:00000 ns      "
    "4.200000000000E+00 6.00000E-02\n"
    " OSB  G063 G01 ALGO      C1C       2020:001:00000 2020:002:00000 ns      "
    "9.900000000000E+00 7.00000E-02\n"
    " OSB  E011 E11           C1C       2020:001:00000 2020:002:00000 ns      "
    "1.500000000000E+00 2.00000E-02\n"
    " OSB  G063 G01           C2W       2020:002:00000 2020:003:86399 ns     "
    "-3.000000000000E-01 2.00000E-02\n"
    "-BIAS/SOLUTION\n";

static void test_biases(const char *dcb_path, const char *bia_gz_path) {
    SidereonBiasSet *set = NULL;
    check(sidereon_bias_sinex_parse(EDGE_BIA, sizeof(EDGE_BIA) - 1, &set) ==
              SIDEREON_STATUS_OK &&
              set != NULL,
          "bias sinex parse");
    if (set) {
        size_t count = 0;
        SidereonBiasMode mode = SIDEREON_BIAS_MODE_UNSPECIFIED;
        uint32_t scale = 0;
        SidereonBiasEpoch epoch = {2020, 1, 0};
        bool present = false;
        double value = 0.0;
        check(sidereon_bias_set_record_count(set, &count) == SIDEREON_STATUS_OK && count == 11,
              "bias record count");
        check(sidereon_bias_set_mode(set, &mode, &scale) == SIDEREON_STATUS_OK &&
                  mode == SIDEREON_BIAS_MODE_ABSOLUTE && scale == SIDEREON_TIME_SCALE_GPST,
              "bias mode");
        check(sidereon_bias_set_code_osb_seconds(set, "G01", "C1C", epoch, &present, &value) ==
                  SIDEREON_STATUS_OK &&
                  present,
              "bias code osb lookup");
        check_close(value, -1.234567890000e-9, 1e-18, "bias code osb value");
        check(sidereon_bias_set_phase_osb_cycles(set, "G01", "L1C", epoch, &present, &value) ==
                  SIDEREON_STATUS_OK &&
                  present,
              "bias phase osb lookup");
        check_close(value, -0.105, 1e-15, "bias phase osb value");
        check(sidereon_bias_set_code_dsb_seconds(set, "G01", "C1C", "C1W", epoch, &present,
                                                 &value) == SIDEREON_STATUS_OK &&
                  present,
              "bias code dsb lookup");
        check_close(value, -1.794567890000e-9, 1e-18, "bias code dsb value");
        sidereon_bias_set_free(set);
    }

    SidereonCodeDcbOptions opts;
    memset(&opts, 0, sizeof(opts));
    opts.obs1 = "P1";
    opts.obs2 = "C1";
    opts.year = 2026;
    opts.month = 6;
    opts.time_scale = SIDEREON_TIME_SCALE_GPST;
    SidereonBiasSet *dcb = NULL;
    check(sidereon_code_dcb_load(dcb_path, &opts, &dcb) == SIDEREON_STATUS_OK && dcb != NULL,
          "code dcb load");
    if (dcb) {
        SidereonBiasEpoch epoch = {2026, 153, 0};
        bool present = false;
        double value = 0.0;
        check(sidereon_bias_set_code_dsb_seconds(dcb, "G01", "C1W", "C1C", epoch, &present,
                                                 &value) == SIDEREON_STATUS_OK &&
                  present,
              "code dcb lookup");
        check_close(value, 0.626e-9, 1e-18, "code dcb value");
        sidereon_bias_set_free(dcb);
    }

    SidereonBiasSet *lossy = NULL;
    check(sidereon_bias_sinex_load_lossy(bia_gz_path, &lossy) == SIDEREON_STATUS_OK &&
              lossy != NULL,
          "bias sinex gz lossy load");
    sidereon_bias_set_free(lossy);
}

static void test_sbas(void) {
    static const char *const hex =
        "5366819010029EE7ED83018202819BBE1A08BF8008FFA00000004066C0";
    uint8_t body[29];
    size_t body_len = hex_to_bytes(hex, body, sizeof(body));
    check(body_len == sizeof(body), "sbas hex decode");
    SidereonSbasBlock *block = NULL;
    SidereonSbasMessageInfo info;
    check(sidereon_sbas_block_decode(body, body_len, SIDEREON_SBAS_WIRE_FORM_BODY226, &block) ==
              SIDEREON_STATUS_OK &&
              block != NULL,
          "sbas block decode");
    if (block) {
        check(sidereon_sbas_block_info(block, &info) == SIDEREON_STATUS_OK &&
                  info.kind == SIDEREON_SBAS_MESSAGE_KIND_LONG_TERM_CORRECTIONS &&
                  info.long_term_count > 0,
              "sbas block info");
        uint8_t encoded[29];
        size_t written = 0;
        size_t required = 0;
        check(sidereon_sbas_block_encode(block, encoded, sizeof(encoded), &written, &required) ==
                  SIDEREON_STATUS_OK &&
                  written == sizeof(encoded) && required == sizeof(encoded) &&
                  memcmp(encoded, body, sizeof(encoded)) == 0,
              "sbas block encode");
        SidereonSbasCorrectionStore *store = NULL;
        check(sidereon_sbas_store_new(&store) == SIDEREON_STATUS_OK && store != NULL,
              "sbas store new");
        if (store) {
            SidereonGnssWeekTow epoch = {SIDEREON_TIME_SCALE_GPST, 2400, 20.0};
            check(sidereon_sbas_store_ingest(store, block, "S20", &epoch) == SIDEREON_STATUS_OK,
                  "sbas store ingest");
            bool present = true;
            SidereonSatelliteToken geo;
            check(sidereon_sbas_store_preferred_geo(store, 0.0, &present, &geo) ==
                      SIDEREON_STATUS_OK,
                  "sbas preferred geo");
            sidereon_sbas_store_free(store);
        }
        sidereon_sbas_block_free(block);
    }
}

static void test_ssr(void) {
    static const char *const hex =
        "d3003c4245438a3040000827968003270026dffea30000f7fff6ffff0000530000000000003e87fff8effc94002c7ffff57fffc80003004128000000000000625cf0";
    uint8_t frame[256];
    size_t frame_len = hex_to_bytes(hex, frame, sizeof(frame));
    check(frame_len > 0, "ssr hex decode");
    SidereonRtcmMessages *messages = NULL;
    check(sidereon_rtcm_decode_messages(frame, frame_len, &messages) == SIDEREON_STATUS_OK &&
              messages != NULL,
          "ssr rtcm decode");
    if (messages) {
        size_t count = 0;
        uint32_t kind = 0;
        uint16_t number = 0;
        SidereonRtcmSsrInfo info;
        size_t written = 0;
        size_t required = 0;
        check(sidereon_rtcm_messages_count(messages, &count) == SIDEREON_STATUS_OK && count == 1,
              "ssr rtcm count");
        check(sidereon_rtcm_message_kind(messages, 0, &kind, &number) == SIDEREON_STATUS_OK &&
                  kind == SIDEREON_RTCM_MESSAGE_KIND_SSR && number == 1060,
              "ssr rtcm kind");
        check(sidereon_rtcm_message_ssr_info(messages, 0, &info) == SIDEREON_STATUS_OK &&
                  info.kind == SIDEREON_RTCM_SSR_KIND_COMBINED_ORBIT_CLOCK &&
                  info.orbit_count > 0 && info.clock_count > 0,
              "ssr info");
        SidereonRtcmSsrOrbitRecord orbits[4];
        SidereonRtcmSsrClockRecord clocks[4];
        check(sidereon_rtcm_message_ssr_orbits(messages, 0, orbits, 4, &written, &required) ==
                  SIDEREON_STATUS_OK &&
                  written > 0,
              "ssr orbit rows");
        check(sidereon_rtcm_message_ssr_clocks(messages, 0, clocks, 4, &written, &required) ==
                  SIDEREON_STATUS_OK &&
                  written > 0,
              "ssr clock rows");
        SidereonSsrCorrectionStore *store = NULL;
        SidereonGnssWeekTow epoch = {SIDEREON_TIME_SCALE_GPST, 2425, 344970.0};
        check(sidereon_ssr_store_from_rtcm(frame, frame_len, &epoch, &store) ==
                  SIDEREON_STATUS_OK &&
                  store != NULL,
              "ssr store from rtcm");
        if (store) {
            bool present = false;
            SidereonSsrOrbitCorrection orbit;
            SidereonSsrClockCorrection clock;
            check(sidereon_ssr_store_orbit(store, "G30", &present, &orbit) ==
                      SIDEREON_STATUS_OK &&
                      present && isfinite(orbit.radial_m),
                  "ssr store orbit");
            check(sidereon_ssr_store_clock(store, "G30", &present, &clock) ==
                      SIDEREON_STATUS_OK &&
                      present && isfinite(clock.c0_m),
                  "ssr store clock");
            sidereon_ssr_store_free(store);
        }
        SidereonSsrCorrectionStore *empty = NULL;
        check(sidereon_ssr_store_new(SIDEREON_SSR_REFERENCE_POINT_CENTER_OF_MASS, &empty) ==
                  SIDEREON_STATUS_OK &&
                  empty != NULL,
              "ssr empty store new");
        sidereon_ssr_store_free(empty);
        sidereon_rtcm_messages_free(messages);
    }
}

int main(int argc, char **argv) {
    if (argc != 7) {
        fprintf(stderr,
                "usage: %s <sp3> <spk> <dted-root> <dted-tile> <dcb> <bia-gz>\n",
                argv[0]);
        return 2;
    }

    SidereonSp3 *sp3 = load_sp3(argv[1]);
    SidereonSpk *spk = load_spk(argv[2]);

    test_labels();
    test_anomaly_and_equinoctial();
    test_angles_and_relative();
    test_observe_and_almanac(spk);
    test_drag_decay();
    if (sp3) {
        test_ephemeris_sample(sp3);
    }
    test_terrain(argv[3], argv[4]);
    test_biases(argv[5], argv[6]);
    test_sbas();
    test_ssr();

    sidereon_spk_free(spk);
    sidereon_sp3_free(sp3);

    if (failures != 0) {
        fprintf(stderr, "phaseb_smoke: %d failure(s)\n", failures);
        return 1;
    }
    printf("phaseb_smoke: OK\n");
    return 0;
}
