/*
 * Smoke coverage for the capability-parity additions to the C binding: RF link
 * budget, GNSS frequencies/combinations, carrier-phase combinations + Hatch
 * smoothing, GNSS signal scalars, measurement weighting + RAIM, troposphere,
 * standalone tides, Sun/Moon angles + eclipse, Sun/Moon ephemeris, IOD,
 * Lambert, conjunction, civil-time conversions, CDM parse/serialize, RINEX
 * clock parse, broadcast orbit/clock evaluation, DGNSS differential
 * corrections, and broadcast-vs-precise comparison. Every call delegates to
 * sidereon-core; each engine value is compared exactly with sidereon-core's
 * own result for the same inputs (tests/w5_extras_pins.h).
 *
 * argv: <grg_sp3> <cdm_kvn> <cdm_xml> <rinex_clk> <nav> <precise_sp3>
 */
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sidereon.h"
#include "w5_extras_pins.h"

static int failures = 0;

static void check(int ok, const char *what) {
    if (!ok) {
        char msg[512];
        size_t n = sidereon_last_error_message((char *)msg, sizeof(msg));
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

static int same_bits(double got, uint64_t want) {
    uint64_t bits;
    memcpy(&bits, &got, sizeof(bits));
    return bits == want;
}

static int same_bits_n(const double *got, const uint64_t *want, size_t n) {
    for (size_t i = 0; i < n; i++) {
        if (!same_bits(got[i], want[i])) {
            return 0;
        }
    }
    return 1;
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
    *out_len = got;
    buf[got] = 0;
    return buf;
}

static void test_rf(void) {
    double fspl = 0.0, eirp = 0.0, cn0 = 0.0, margin = 0.0, lambda = 0.0, gain = 0.0;
    check(sidereon_rf_fspl(36000.0, 1575.42, &fspl) == SIDEREON_STATUS_OK &&
              same_bits(fspl, W5_EXTRAS_RF_FSPL_BITS),
          "rf_fspl");
    check(sidereon_rf_eirp(40.0, 30.0, &eirp) == SIDEREON_STATUS_OK &&
              same_bits(eirp, W5_EXTRAS_RF_EIRP_BITS),
          "rf_eirp");
    check(sidereon_rf_cn0(eirp, fspl, 5.0, 2.0, &cn0) == SIDEREON_STATUS_OK &&
              same_bits(cn0, W5_EXTRAS_RF_CN0_BITS),
          "rf_cn0");
    SidereonLinkBudget budget = {eirp, fspl, 5.0, 2.0, 35.0};
    check(sidereon_rf_link_margin(&budget, &margin) == SIDEREON_STATUS_OK &&
              same_bits(margin, W5_EXTRAS_RF_LINK_MARGIN_BITS),
          "rf_link_margin");
    check(sidereon_rf_wavelength(1.57542e9, &lambda) == SIDEREON_STATUS_OK &&
              same_bits(lambda, W5_EXTRAS_RF_WAVELENGTH_BITS),
          "rf_wavelength");
    check(sidereon_rf_dish_gain(2.4, 1.57542e9, 0.6, &gain) == SIDEREON_STATUS_OK &&
              same_bits(gain, W5_EXTRAS_RF_DISH_GAIN_BITS),
          "rf_dish_gain");
}

static void test_frequencies_combinations(void) {
    double f1 = 0.0, f2 = 0.0, lambda = 0.0, glo = 0.0, def = 0.0;
    check(sidereon_frequency_hz(SIDEREON_GNSS_SYSTEM_GPS, SIDEREON_CARRIER_BAND_L1, &f1) ==
              SIDEREON_STATUS_OK &&
              same_bits(f1, W5_EXTRAS_GPS_L1_HZ_BITS),
          "frequency_hz GPS L1");
    check(sidereon_frequency_hz(SIDEREON_GNSS_SYSTEM_GPS, SIDEREON_CARRIER_BAND_L2, &f2) ==
                  SIDEREON_STATUS_OK &&
              same_bits(f2, W5_EXTRAS_GPS_L2_HZ_BITS),
          "frequency_hz GPS L2");
    check(sidereon_wavelength_m(SIDEREON_GNSS_SYSTEM_GPS, SIDEREON_CARRIER_BAND_L1, &lambda) ==
                  SIDEREON_STATUS_OK &&
              same_bits(lambda, W5_EXTRAS_GPS_L1_WAVELENGTH_M_BITS),
          "wavelength_m");
    check(sidereon_glonass_g1_frequency_hz(0, &glo) == SIDEREON_STATUS_OK &&
              same_bits(glo, W5_EXTRAS_GLONASS_G1_CHANNEL_0_HZ_BITS),
          "glonass_g1_frequency_hz");
    check(sidereon_default_spp_frequency_hz(SIDEREON_GNSS_SYSTEM_GPS, &def) ==
                  SIDEREON_STATUS_OK &&
              same_bits(def, W5_EXTRAS_GPS_DEFAULT_SPP_HZ_BITS),
          "default_spp_frequency_hz");

    double gamma = 0.0, namp = 0.0, iono = 0.0, iono_phase = 0.0;
    check(sidereon_combination_gamma(f1, f2, &gamma) == SIDEREON_STATUS_OK &&
              same_bits(gamma, W5_EXTRAS_GAMMA_BITS),
          "combination_gamma");
    check(sidereon_combination_noise_amplification(f1, f2, &namp) == SIDEREON_STATUS_OK &&
              same_bits(namp, W5_EXTRAS_NOISE_AMPLIFICATION_BITS),
          "combination_noise_amplification");
    check(sidereon_combination_ionosphere_free(2.0e7, 2.0e7, f1, f2, &iono) ==
                  SIDEREON_STATUS_OK &&
              same_bits(iono, W5_EXTRAS_IONOSPHERE_FREE_BITS),
          "combination_ionosphere_free");
    check(sidereon_combination_ionosphere_free_phase_m(2.0e7, 2.0e7, f1, f2, &iono_phase) ==
                  SIDEREON_STATUS_OK &&
              same_bits(iono_phase, W5_EXTRAS_IONOSPHERE_FREE_PHASE_M_BITS),
          "combination_ionosphere_free_phase_m");
}

static void test_carrier_phase_scalars(void) {
    double pm = 0.0, gf = 0.0, wl = 0.0, nl = 0.0, mw = 0.0, wlc = 0.0, cmc = 0.0;
    check(sidereon_carrier_phase_meters(1.0e8, 1.57542e9, &pm) == SIDEREON_STATUS_OK &&
              same_bits(pm, W5_EXTRAS_PHASE_METERS_BITS),
          "carrier_phase_meters");
    check(sidereon_carrier_geometry_free(2.0e7, 2.0e7, &gf) == SIDEREON_STATUS_OK &&
              same_bits(gf, W5_EXTRAS_GEOMETRY_FREE_BITS),
          "carrier_geometry_free");
    check(sidereon_carrier_wide_lane_wavelength(1.57542e9, 1.22760e9, &wl) ==
              SIDEREON_STATUS_OK &&
              same_bits(wl, W5_EXTRAS_WIDE_LANE_WAVELENGTH_BITS),
          "carrier_wide_lane_wavelength");
    check(sidereon_carrier_narrow_lane_code(2.0e7, 2.0e7, 1.57542e9, 1.22760e9, &nl) ==
                  SIDEREON_STATUS_OK &&
              same_bits(nl, W5_EXTRAS_NARROW_LANE_CODE_BITS),
          "carrier_narrow_lane_code");
    check(sidereon_carrier_melbourne_wubbena(1.0e8, 8.0e7, 2.0e7, 2.0e7, 1.57542e9, 1.22760e9,
                                             &mw) == SIDEREON_STATUS_OK &&
              same_bits(mw, W5_EXTRAS_MELBOURNE_WUBBENA_BITS),
          "carrier_melbourne_wubbena");
    check(sidereon_carrier_wide_lane_cycles(1.0e8, 8.0e7, 2.0e7, 2.0e7, 1.57542e9, 1.22760e9,
                                            &wlc) == SIDEREON_STATUS_OK &&
              same_bits(wlc, W5_EXTRAS_WIDE_LANE_CYCLES_BITS),
          "carrier_wide_lane_cycles");
    check(sidereon_carrier_code_minus_carrier(2.0e7, 1.0e8, 1.57542e9, &cmc) ==
                  SIDEREON_STATUS_OK &&
              same_bits(cmc, W5_EXTRAS_CODE_MINUS_CARRIER_BITS),
          "carrier_code_minus_carrier");
}

static void test_signal_quality(void) {
    int8_t chip = 0;
    double loss = 0.0, loss_db = 0.0, snr = 0.0;
    check(sidereon_signal_ca_chip(1, 0, &chip) == SIDEREON_STATUS_OK &&
              chip == W5_EXTRAS_CA_CHIP_PRN1_INDEX0,
          "signal_ca_chip");
    check(sidereon_signal_coherent_loss(100.0, 0.001, &loss) == SIDEREON_STATUS_OK &&
              same_bits(loss, W5_EXTRAS_COHERENT_LOSS_BITS),
          "signal_coherent_loss");
    check(sidereon_signal_coherent_loss_db(100.0, 0.001, &loss_db) == SIDEREON_STATUS_OK &&
              same_bits(loss_db, W5_EXTRAS_COHERENT_LOSS_DB_BITS),
          "signal_coherent_loss_db");
    check(sidereon_signal_snr_post_db(45.0, 0.02, &snr) == SIDEREON_STATUS_OK &&
              same_bits(snr, W5_EXTRAS_SNR_POST_DB_BITS),
          "signal_snr_post_db");

    SidereonPseudorangeVarianceOptions opts;
    check(sidereon_pseudorange_variance_options_init(&opts) == SIDEREON_STATUS_OK,
          "pseudorange_variance_options_init");
    double var = 0.0, chi2 = 0.0;
    check(sidereon_pseudorange_variance(30.0, &opts, &var) == SIDEREON_STATUS_OK &&
              same_bits(var, W5_EXTRAS_PSEUDORANGE_VARIANCE_BITS),
          "pseudorange_variance");
    check(sidereon_chi2_inv(0.999, 1, &chi2) == SIDEREON_STATUS_OK &&
              same_bits(chi2, W5_EXTRAS_CHI2_INV_BITS),
          "chi2_inv");
}

static void test_raim(void) {
    const char *sats[5] = {"G01", "G02", "G03", "G04", "G05"};
    double residuals[5] = {0.4, -0.3, 0.5, -0.2, 0.35};
    SidereonRaimResult result;
    check(sidereon_raim(sats, residuals, NULL, 5, 1.0e-3, SIDEREON_RAIM_WEIGHTS_MODE_UNIT, NULL, 0,
                        false, 0, &result) == SIDEREON_STATUS_OK,
          "raim");
    check(result.fault_detected == W5_EXTRAS_RAIM_FAULT_DETECTED &&
              same_bits(result.test_statistic, W5_EXTRAS_RAIM_TEST_STATISTIC_BITS),
          "raim test statistic");
    check(result.has_threshold == W5_EXTRAS_RAIM_HAS_THRESHOLD &&
              same_bits(result.threshold, W5_EXTRAS_RAIM_THRESHOLD_BITS),
          "raim threshold");
    check(result.has_reduced_chi_square == W5_EXTRAS_RAIM_HAS_REDUCED_CHI_SQUARE &&
              same_bits(result.reduced_chi_square, W5_EXTRAS_RAIM_REDUCED_CHI_SQUARE_BITS),
          "raim reduced chi-square");
    check(same_bits(result.rms_m, W5_EXTRAS_RAIM_RMS_M_BITS), "raim rms");
    check(result.dof == W5_EXTRAS_RAIM_DOF && result.testable == W5_EXTRAS_RAIM_TESTABLE,
          "raim dof");
    check(result.normalized_residual_count == W5_EXTRAS_RAIM_NORMALIZED_RESIDUAL_COUNT,
          "raim normalized residual count");
    check(result.has_worst_sat == W5_EXTRAS_RAIM_HAS_WORST_SAT &&
              strncmp(result.worst_sat, W5_EXTRAS_RAIM_WORST_SAT, sizeof(result.worst_sat)) == 0,
          "raim worst satellite");

    size_t written = 0;
    size_t required = 0;
    check(sidereon_raim_normalized_residuals(sats, residuals, NULL, 5, 1.0e-3,
                                             SIDEREON_RAIM_WEIGHTS_MODE_UNIT, NULL, 0, false, 0,
                                             NULL, 0, &written, &required) ==
              SIDEREON_STATUS_OK &&
              written == 0 && required == W5_EXTRAS_RAIM_NORMALIZED_RESIDUAL_COUNT,
          "raim normalized residual query");
    SidereonRaimNormalizedResidual rows[5];
    check(sidereon_raim_normalized_residuals(sats, residuals, NULL, 5, 1.0e-3,
                                             SIDEREON_RAIM_WEIGHTS_MODE_UNIT, NULL, 0, false, 0,
                                             rows, 5, &written, &required) ==
              SIDEREON_STATUS_OK &&
              written == required && required == W5_EXTRAS_RAIM_NORMALIZED_RESIDUAL_COUNT,
          "raim normalized residual copy");
    check(strncmp(rows[0].sat_id.bytes, W5_EXTRAS_RAIM_FIRST_NORMALIZED_SAT,
                  sizeof(rows[0].sat_id.bytes)) == 0,
          "raim normalized residual sat id");
    check(same_bits(rows[0].normalized_residual, W5_EXTRAS_RAIM_FIRST_NORMALIZED_RESIDUAL_BITS),
          "raim normalized residual value");

    /* The engine default weights read each residual's variance. */
    double variances[5];
    memcpy(variances, W5_EXTRAS_RAIM_VARIANCES_BITS, sizeof(variances));
    SidereonRaimResult solution;
    check(sidereon_raim(sats, residuals, variances, 5, 1.0e-3,
                        SIDEREON_RAIM_WEIGHTS_MODE_SOLUTION, NULL, 0, false, 0,
                        &solution) == SIDEREON_STATUS_OK &&
              solution.fault_detected == W5_EXTRAS_RAIM_SOLUTION_FAULT_DETECTED &&
              same_bits(solution.test_statistic, W5_EXTRAS_RAIM_SOLUTION_TEST_STATISTIC_BITS),
          "raim over the residual variances");
    check(sidereon_raim_normalized_residuals(sats, residuals, variances, 5, 1.0e-3,
                                             SIDEREON_RAIM_WEIGHTS_MODE_SOLUTION, NULL, 0, false,
                                             0, rows, 5, &written, &required) ==
                  SIDEREON_STATUS_OK &&
              same_bits(rows[0].normalized_residual,
                        W5_EXTRAS_RAIM_SOLUTION_FIRST_NORMALIZED_RESIDUAL_BITS),
          "raim residual over its standard deviation");
    check(W5_EXTRAS_RAIM_SOLUTION_WITHOUT_VARIANCES_REFUSED &&
              sidereon_raim(sats, residuals, NULL, 5, 1.0e-3,
                            SIDEREON_RAIM_WEIGHTS_MODE_SOLUTION, NULL, 0, false, 0,
                            &solution) == SIDEREON_STATUS_INVALID_ARGUMENT,
          "raim solution weights without variances refused");
    check(sidereon_last_quality_error_kind() == SIDEREON_QUALITY_ERROR_KIND_MISSING_VARIANCES,
          "raim missing variances kind retained");
    check(sidereon_raim(sats, residuals, NULL, 5, 1.0e-3,
                        SIDEREON_RAIM_WEIGHTS_MODE_UNIT, NULL, 0, false, 0,
                        &solution) == SIDEREON_STATUS_OK &&
              sidereon_last_quality_error_kind() == SIDEREON_QUALITY_ERROR_KIND_NONE,
          "successful RAIM clears quality kind");
    check(sidereon_raim(sats, residuals, NULL, 5, 1.0e-3,
                        SIDEREON_RAIM_WEIGHTS_MODE_SOLUTION, NULL, 0, false, 0,
                        &solution) == SIDEREON_STATUS_INVALID_ARGUMENT &&
              sidereon_last_quality_error_kind() == SIDEREON_QUALITY_ERROR_KIND_MISSING_VARIANCES,
          "raim missing variances reseeded");
    check(sidereon_raim(sats, residuals, NULL, 5, 1.0e-3,
                        SIDEREON_RAIM_WEIGHTS_MODE_SOLUTION, NULL, 0, false, 0,
                        NULL) == SIDEREON_STATUS_NULL_POINTER &&
              sidereon_last_quality_error_kind() == SIDEREON_QUALITY_ERROR_KIND_NONE,
          "early RAIM argument failure clears quality kind");
}

static void test_tropo(void) {
    SidereonGeodetic receiver = {0.7, 0.1, 100.0};
    // The standard-atmosphere met comes from the same core source
    // (sidereon_core::spp::SurfaceMet::default()) as the other bindings.
    SidereonMet met = {0.0, 0.0, 0.0};
    check(sidereon_met_init(&met) == SIDEREON_STATUS_OK &&
              same_bits(met.pressure_hpa, W5_EXTRAS_MET_PRESSURE_HPA_BITS) &&
              same_bits(met.temperature_k, W5_EXTRAS_MET_TEMPERATURE_K_BITS) &&
              same_bits(met.relative_humidity, W5_EXTRAS_MET_RELATIVE_HUMIDITY_BITS),
          "met_init equals core SurfaceMet::default triad");
    SidereonZenithDelay zd = {0.0, 0.0};
    check(sidereon_tropo_zenith_delay(receiver, &met, &zd) == SIDEREON_STATUS_OK &&
              same_bits(zd.dry_m, W5_EXTRAS_ZENITH_DRY_M_BITS) &&
              same_bits(zd.wet_m, W5_EXTRAS_ZENITH_WET_M_BITS),
          "tropo_zenith_delay");
    SidereonMappingFactors mf = {0.0, 0.0};
    check(sidereon_tropo_mapping_factors(0.5, receiver, SIDEREON_TIME_SCALE_TT, 2451545.0, 0.0,
                                         &mf) == SIDEREON_STATUS_OK &&
              same_bits(mf.dry, W5_EXTRAS_MAPPING_DRY_BITS) &&
              same_bits(mf.wet, W5_EXTRAS_MAPPING_WET_BITS),
          "tropo_mapping_factors");
    double slant = 0.0;
    check(sidereon_tropo_slant_delay(0.5, receiver, &met, SIDEREON_TIME_SCALE_TT, 2451545.0, 0.0,
                                     &slant) == SIDEREON_STATUS_OK &&
              same_bits(slant, W5_EXTRAS_SLANT_DELAY_BITS),
          "tropo_slant_delay");
}

static void test_tides(void) {
    const double station[3] = {4517590.0, 837270.0, 4527420.0};
    const double sun_ecef[3] = {1.4e11, 0.4e11, 0.2e11};
    const double moon_ecef[3] = {3.0e8, 1.5e8, 1.0e8};
    double solid[3] = {0.0, 0.0, 0.0};
    check(sidereon_solid_earth_tide(station, 2020, 6, 24, 12.0, sun_ecef, moon_ecef, solid) ==
                  SIDEREON_STATUS_OK &&
              same_bits_n(solid, W5_EXTRAS_SOLID_EARTH_TIDE_M_BITS, 3),
          "solid_earth_tide");
    double solid_with_constants[3] = {0.0, 0.0, 0.0};
    /* sidereon-core's solid_earth_tide is the Conventions constant set. */
    check(sidereon_solid_earth_tide_with_constants(
              station, 2020, 6, 24, 12.0, sun_ecef, moon_ecef,
              SIDEREON_STATION_TIDE_CONSTANTS_CONVENTIONS, solid_with_constants) ==
                  SIDEREON_STATUS_OK &&
              same_bits_n(solid_with_constants, W5_EXTRAS_SOLID_EARTH_TIDE_M_BITS, 3),
          "solid_earth_tide_with_constants");

    SidereonOceanLoadingBlq blq;
    memset(&blq, 0, sizeof(blq));
    double ocean[3] = {1.0, 1.0, 1.0};
    check(sidereon_ocean_tide_loading(station, 2020, 6, 24, 12.0, &blq, ocean) ==
                  SIDEREON_STATUS_OK &&
              same_bits_n(ocean, W5_EXTRAS_OCEAN_TIDE_ZERO_BLQ_M_BITS, 3),
          "ocean_tide_loading zero BLQ");

    double pole[3] = {0.0, 0.0, 0.0};
    check(sidereon_solid_earth_pole_tide(station, 2020, 6, 24, 12.0, 0.1, 0.3, pole) ==
                  SIDEREON_STATUS_OK &&
              same_bits_n(pole, W5_EXTRAS_POLE_TIDE_M_BITS, 3),
          "solid_earth_pole_tide");
}

static void test_gravity_tide_system_selector(void) {
    SidereonStatePropagationConfig config;
    SidereonEphemeris *ephemeris = NULL;
    double time_s = 0.0;
    check(sidereon_state_propagation_config_init(&config) == SIDEREON_STATUS_OK,
          "state propagation config init for tide selector");
    check(sidereon_propagate_state_with_tide_system(
              &config, SIDEREON_GRAVITY_TIDE_SYSTEM_TIDE_FREE, &time_s, 1, &ephemeris) ==
              SIDEREON_STATUS_OK &&
              ephemeris != NULL,
          "checked tide-free selector");
    sidereon_ephemeris_free(ephemeris);
    ephemeris = NULL;
    check(sidereon_propagate_state_with_tide_system(&config, UINT32_MAX, &time_s, 1, &ephemeris) ==
                  SIDEREON_STATUS_INVALID_ARGUMENT &&
              ephemeris == NULL,
          "invalid gravity tide selector refused");
}

static void test_angles_eclipse_bodies(void) {
    double sat[3] = {7000.0, 0.0, 0.0};
    double sun[3] = {1.5e8, 0.0, 0.0};
    double moon[3] = {3.8e5, 0.0, 0.0};
    double observer[3] = {6371.0, 0.0, 0.0};
    double ang = 0.0;
    check(sidereon_sun_angle_deg(sat, sun, &ang) == SIDEREON_STATUS_OK &&
              same_bits(ang, W5_EXTRAS_SUN_ANGLE_DEG_BITS),
          "sun_angle_deg");
    check(sidereon_moon_angle_deg(sat, moon, &ang) == SIDEREON_STATUS_OK &&
              same_bits(ang, W5_EXTRAS_MOON_ANGLE_DEG_BITS),
          "moon_angle_deg");
    check(sidereon_sun_elevation_deg(sat, sun, &ang) == SIDEREON_STATUS_OK &&
              same_bits(ang, W5_EXTRAS_SUN_ELEVATION_DEG_BITS),
          "sun_elevation_deg");
    check(sidereon_phase_angle_deg(sat, sun, observer, &ang) == SIDEREON_STATUS_OK &&
              same_bits(ang, W5_EXTRAS_PHASE_ANGLE_DEG_BITS),
          "phase_angle_deg");
    check(sidereon_earth_angular_radius_deg(sat, &ang) == SIDEREON_STATUS_OK &&
              same_bits(ang, W5_EXTRAS_EARTH_ANGULAR_RADIUS_DEG_BITS),
          "earth_angular_radius_deg");

    double frac = 0.0;
    SidereonEclipseStatus status = SIDEREON_ECLIPSE_STATUS_UMBRA;
    check(sidereon_eclipse_shadow_fraction(sat, sun, &frac) == SIDEREON_STATUS_OK &&
              same_bits(frac, W5_EXTRAS_ECLIPSE_SHADOW_FRACTION_BITS),
          "eclipse_shadow_fraction");
    check(sidereon_eclipse_status(sat, sun, &status) == SIDEREON_STATUS_OK &&
              status == W5_EXTRAS_ECLIPSE_STATUS,
          "eclipse_status");

    double sun_m[3] = {0.0, 0.0, 0.0};
    double moon_m[3] = {0.0, 0.0, 0.0};
    check(sidereon_sun_moon_eci(0.21, sun_m, moon_m) == SIDEREON_STATUS_OK &&
              same_bits_n(sun_m, W5_EXTRAS_SUN_ECI_021_M_BITS, 3) &&
              same_bits_n(moon_m, W5_EXTRAS_MOON_ECI_021_M_BITS, 3),
          "sun_moon_eci");

    int64_t epochs[2] = {INT64_C(946728000000000), INT64_C(1593002096000000)};
    double sun_eci[6] = {0.0};
    double moon_eci[6] = {0.0};
    check(sidereon_sun_moon_eci_batch(epochs, 2, sun_eci, 6, moon_eci, 6) ==
                  SIDEREON_STATUS_OK &&
              same_bits_n(sun_eci, W5_EXTRAS_SUN_ECI_BATCH_M_BITS, 6) &&
              same_bits_n(moon_eci, W5_EXTRAS_MOON_ECI_BATCH_M_BITS, 6),
          "sun_moon_eci_batch");

    double sun_ecef[6] = {0.0};
    double moon_ecef[6] = {0.0};
    check(sidereon_sun_moon_ecef_batch(epochs, 2, sun_ecef, 6, moon_ecef, 6) ==
                  SIDEREON_STATUS_OK &&
              same_bits_n(sun_ecef, W5_EXTRAS_SUN_ECEF_BATCH_M_BITS, 6) &&
              same_bits_n(moon_ecef, W5_EXTRAS_MOON_ECEF_BATCH_M_BITS, 6),
          "sun_moon_ecef_batch");

    double sun_ecef_one[3] = {0.0, 0.0, 0.0};
    double moon_ecef_one[3] = {0.0, 0.0, 0.0};
    check(sidereon_sun_moon_ecef(epochs[0], sun_ecef_one, moon_ecef_one) == SIDEREON_STATUS_OK &&
              memcmp(sun_ecef_one, sun_ecef, sizeof(sun_ecef_one)) == 0 &&
              memcmp(moon_ecef_one, moon_ecef, sizeof(moon_ecef_one)) == 0,
          "sun_moon_ecef");

    check(sidereon_sun_moon_eci_batch(NULL, 0, NULL, 0, NULL, 0) ==
              SIDEREON_STATUS_INVALID_ARGUMENT,
          "sun_moon_eci_batch empty rejected");
}

static void test_iod_lambert_conjunction(void) {
    double r1[3] = {0.0, 0.0, 6378.1363};
    double r2[3] = {0.0, -4464.696, -5102.509};
    double r3[3] = {0.0, 5740.323, 3189.068};
    double v2[3], t12 = 0.0, t23 = 0.0, copa = 0.0;
    check(sidereon_iod_gibbs(r1, r2, r3, v2, &t12, &t23, &copa) == SIDEREON_STATUS_OK,
          "iod_gibbs");
    check(same_bits_n(v2, W5_EXTRAS_GIBBS_V2_BITS, 3), "iod_gibbs velocity");

    double hr1[3] = {3419.85564, 6019.82602, 2784.60022};
    double hr2[3] = {2935.91195, 6326.18324, 2660.59584};
    double hr3[3] = {2434.95202, 6597.38674, 2521.52311};
    const double jd1 = 0.0;
    const double jd2 = (60.0 + 16.48) / 86400.0;
    const double jd3 = (120.0 + 33.04) / 86400.0;
    check(sidereon_iod_hgibbs(hr1, hr2, hr3, jd1, jd2, jd3, v2, &t12, &t23, &copa) ==
              SIDEREON_STATUS_OK,
          "iod_hgibbs");
    check(same_bits_n(v2, W5_EXTRAS_HGIBBS_V2_BITS, 3), "iod_hgibbs velocity");

    const double earth_radius_km = 6378.1363;
    double lr1[3] = {2.5 * earth_radius_km, 0.0, 0.0};
    double lr2[3] = {1.9151111 * earth_radius_km, 1.6069690 * earth_radius_km, 0.0};
    double v1[3] = {0.0, 4.999792554221911, 0.0};
    double out_v1[3], out_v2[3];
    check(sidereon_lambert_battin(lr1, lr2, v1, 0, 1, 1, 92854.234, out_v1, out_v2) ==
              SIDEREON_STATUS_OK,
          "lambert_battin");
    check(same_bits_n(out_v1, W5_EXTRAS_LAMBERT_V1_BITS, 3), "lambert_battin departure velocity");
    check(same_bits_n(out_v2, W5_EXTRAS_LAMBERT_V2_BITS, 3), "lambert_battin arrival velocity");

    double cr1[3] = {7000.0, 0.0, 0.0};
    double cv1[3] = {0.0, 7.5, 0.0};
    double cr2[3] = {7000.05, 0.02, 0.0};
    double cv2[3] = {0.0, -7.5, 0.1};
    SidereonEncounterFrame frame;
    check(sidereon_encounter_frame(cr1, cv1, cr2, cv2, &frame) == SIDEREON_STATUS_OK &&
              same_bits(frame.miss_km, W5_EXTRAS_ENCOUNTER_MISS_KM_BITS) &&
              same_bits(frame.relative_speed_km_s, W5_EXTRAS_ENCOUNTER_RELATIVE_SPEED_KM_S_BITS),
          "encounter_frame");

    SidereonConjunctionState o1 = {
        {7000.0, 0.0, 0.0}, {0.0, 7.5, 0.0}, {{0.01, 0.0, 0.0}, {0.0, 0.01, 0.0}, {0.0, 0.0, 0.01}}};
    SidereonConjunctionState o2 = {
        {7000.05, 0.02, 0.0}, {0.0, -7.5, 0.1}, {{0.01, 0.0, 0.0}, {0.0, 0.01, 0.0}, {0.0, 0.0, 0.01}}};
    SidereonCollisionPc pc;
    check(sidereon_collision_probability(&o1, &o2, 0.02, SIDEREON_PC_METHOD_FOSTER_EQUAL_AREA,
                                         &pc) == SIDEREON_STATUS_OK &&
              same_bits(pc.pc, W5_EXTRAS_COLLISION_PC_BITS) &&
              same_bits(pc.miss_km, W5_EXTRAS_COLLISION_MISS_KM_BITS),
          "collision_probability");
}

static void test_civil_time(void) {
    double sec = 0.0, sec2 = 0.0;
    check(sidereon_civil_to_j2000_seconds(2020, 6, 25, 12, 0, 0.0, &sec) == SIDEREON_STATUS_OK &&
              same_bits(sec, W5_EXTRAS_CIVIL_J2000_S_BITS),
          "civil_to_j2000_seconds");
    check(sidereon_split_jd_to_j2000_seconds(2451545.0, 0.0, &sec2) == SIDEREON_STATUS_OK &&
              same_bits(sec2, W5_EXTRAS_SPLIT_JD_J2000_S_BITS),
          "split_jd_to_j2000_seconds");
    int64_t y = 0, mo = 0, d = 0, h = 0, mi = 0, s = 0;
    check(sidereon_j2000_seconds_to_civil((int64_t)llround(sec), &y, &mo, &d, &h, &mi, &s) ==
              SIDEREON_STATUS_OK &&
              y == W5_EXTRAS_CIVIL_ROUND_TRIP[0] && mo == W5_EXTRAS_CIVIL_ROUND_TRIP[1] &&
              d == W5_EXTRAS_CIVIL_ROUND_TRIP[2] && h == W5_EXTRAS_CIVIL_ROUND_TRIP[3] &&
              mi == W5_EXTRAS_CIVIL_ROUND_TRIP[4] && s == W5_EXTRAS_CIVIL_ROUND_TRIP[5],
          "j2000_seconds_to_civil round-trip");
}

static void test_carrier_smoothing(void) {
    SidereonCycleSlipOptions opts;
    check(sidereon_cycle_slip_options_init(&opts) == SIDEREON_STATUS_OK,
          "cycle_slip_options_init");
    SidereonArcEpoch arc[4];
    for (size_t i = 0; i < 4; i++) {
        double k = (double)i;
        arc[i].phi1_cycles = 1.0e8 + k * 1000.0;
        arc[i].phi2_cycles = 0.78e8 + k * 780.0;
        arc[i].p1_m = 2.0e7 + k * 190.0;
        arc[i].p2_m = 2.0e7 + k * 244.0;
        arc[i].has_lli1 = false;
        arc[i].lli1 = 0;
        arc[i].has_lli2 = false;
        arc[i].lli2 = 0;
        arc[i].f1_hz = 1.57542e9;
        arc[i].f2_hz = 1.22760e9;
        arc[i].gap_time_s = (i == 0) ? NAN : 30.0;
    }
    SidereonSmoothCodeResult sc[4];
    size_t written = 0, required = 0;
    check(sidereon_smooth_code(arc, 4, &opts, 100, sc, 4, &written, &required) ==
              SIDEREON_STATUS_OK &&
              written == 4 && required == W5_EXTRAS_SMOOTH_P_M_BITS_COUNT,
          "smooth_code");
    for (size_t i = 0; i < 4; i++) {
        check(same_bits(sc[i].p_smooth_m, W5_EXTRAS_SMOOTH_P_M_BITS[i]) &&
                  sc[i].window == W5_EXTRAS_SMOOTH_WINDOW[i] &&
                  sc[i].reset == W5_EXTRAS_SMOOTH_RESET[i],
              "smooth_code row");
    }
    SidereonIonoFreeSmoothResult sif[4];
    check(sidereon_smooth_iono_free_code(arc, 4, &opts, 100, sif, 4, &written, &required) ==
              SIDEREON_STATUS_OK &&
              written == 4 && required == W5_EXTRAS_IONO_FREE_SMOOTH_P_M_BITS_COUNT,
          "smooth_iono_free_code");
    for (size_t i = 0; i < 4; i++) {
        check(same_bits(sif[i].p_smooth_m, W5_EXTRAS_IONO_FREE_SMOOTH_P_M_BITS[i]) &&
                  same_bits(sif[i].p_if_m, W5_EXTRAS_IONO_FREE_P_IF_M_BITS[i]) &&
                  same_bits(sif[i].l_if_m, W5_EXTRAS_IONO_FREE_L_IF_M_BITS[i]),
              "smooth_iono_free_code row");
    }
}

static void test_cdm(const char *kvn_path, const char *xml_path) {
    size_t len = 0;
    uint8_t *kvn = read_file(kvn_path, &len);
    if (!kvn) {
        return;
    }
    SidereonCdm *cdm = NULL;
    check(sidereon_cdm_parse_kvn(kvn, len, &cdm) == SIDEREON_STATUS_OK && cdm != NULL,
          "cdm_parse_kvn");
    if (cdm) {
        double miss = 0.0, speed = 0.0, prob = 0.0, hbr = 0.0;
        check(sidereon_cdm_numbers(cdm, &miss, &speed, &prob, &hbr) == SIDEREON_STATUS_OK &&
                  same_bits(miss, W5_EXTRAS_CDM_MISS_DISTANCE_M_BITS) &&
                  same_bits(speed, W5_EXTRAS_CDM_RELATIVE_SPEED_M_S_BITS) &&
                  same_bits(prob, W5_EXTRAS_CDM_COLLISION_PROBABILITY_BITS) &&
                  same_bits(hbr, W5_EXTRAS_CDM_HARD_BODY_RADIUS_M_BITS),
              "cdm_numbers");
        double pos[3], vel[3], cov[6];
        check(sidereon_cdm_object_state(cdm, 1, pos, vel, cov) == SIDEREON_STATUS_OK &&
                  same_bits_n(pos, W5_EXTRAS_CDM_OBJECT1_POSITION_BITS, 3) &&
                  same_bits_n(vel, W5_EXTRAS_CDM_OBJECT1_VELOCITY_BITS, 3) &&
                  same_bits_n(cov, W5_EXTRAS_CDM_OBJECT1_COVARIANCE_RTN_BITS, 6),
              "cdm_object_state");
        size_t written = 0, required = 0;
        check(sidereon_cdm_to_kvn(cdm, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
                  required == W5_EXTRAS_CDM_KVN_TEXT_LEN,
              "cdm_to_kvn size query");
        uint8_t *buf = (uint8_t *)malloc(required);
        check(buf != NULL && sidereon_cdm_to_kvn(cdm, buf, required, &written, &required) ==
                                 SIDEREON_STATUS_OK,
              "cdm_to_kvn fill");
        free(buf);
        uint8_t field[128];
        check(sidereon_cdm_string_field(cdm, SIDEREON_CDM_STRING_FIELD_TCA, field, sizeof(field),
                                        &written, &required) == SIDEREON_STATUS_OK &&
                  written == W5_EXTRAS_CDM_TCA_LEN &&
                  memcmp(field, W5_EXTRAS_CDM_TCA, W5_EXTRAS_CDM_TCA_LEN) == 0,
              "cdm_string_field tca");

        /* Full per-object metadata block: a representative field from each kind. */
        check(sidereon_cdm_object_string_field(cdm, 1,
                                               SIDEREON_CDM_OBJECT_STRING_FIELD_REF_FRAME, field,
                                               sizeof(field), &written, &required) ==
                      SIDEREON_STATUS_OK &&
                  written == W5_EXTRAS_CDM_OBJECT1_REF_FRAME_LEN &&
                  memcmp(field, W5_EXTRAS_CDM_OBJECT1_REF_FRAME, written) == 0,
              "cdm_object_string_field ref_frame");
        check(sidereon_cdm_object_string_field(cdm, 2,
                                               SIDEREON_CDM_OBJECT_STRING_FIELD_COVARIANCE_METHOD,
                                               field, sizeof(field), &written, &required) ==
                      SIDEREON_STATUS_OK &&
                  written == W5_EXTRAS_CDM_OBJECT2_COVARIANCE_METHOD_LEN &&
                  memcmp(field, W5_EXTRAS_CDM_OBJECT2_COVARIANCE_METHOD, written) == 0,
              "cdm_object_string_field covariance_method");
        check(sidereon_cdm_object_string_field(cdm, 1,
                                               SIDEREON_CDM_OBJECT_STRING_FIELD_MANEUVERABLE, field,
                                               sizeof(field), &written, &required) ==
                      SIDEREON_STATUS_OK &&
                  written == W5_EXTRAS_CDM_OBJECT1_MANEUVERABLE_LEN &&
                  memcmp(field, W5_EXTRAS_CDM_OBJECT1_MANEUVERABLE, written) == 0,
              "cdm_object_string_field maneuverable");
        check(sidereon_cdm_object_string_field(cdm, 3,
                                               SIDEREON_CDM_OBJECT_STRING_FIELD_OBJECT_NAME, field,
                                               sizeof(field), &written, &required) ==
                  SIDEREON_STATUS_INVALID_ARGUMENT,
              "cdm_object_string_field rejects bad object_index");

        /* Velocity-covariance block: present-flag reported either way, and the
         * 15 elements are copied when the producer carried them. */
        double vcov[15];
        bool vcov_present = false;
        check(sidereon_cdm_object_velocity_covariance(cdm, 1, vcov, &vcov_present) ==
                      SIDEREON_STATUS_OK &&
                  vcov_present == W5_EXTRAS_CDM_OBJECT1_HAS_VELOCITY_COVARIANCE,
              "cdm_object_velocity_covariance object 1");
        check(sidereon_cdm_object_velocity_covariance(cdm, 2, vcov, &vcov_present) ==
                      SIDEREON_STATUS_OK &&
                  vcov_present == W5_EXTRAS_CDM_OBJECT2_HAS_VELOCITY_COVARIANCE,
              "cdm_object_velocity_covariance object 2");

        sidereon_cdm_free(cdm);
    }
    free(kvn);

    uint8_t *xml = read_file(xml_path, &len);
    if (!xml) {
        return;
    }
    SidereonCdm *cdm_xml = NULL;
    check(W5_EXTRAS_CDM_XML_OK &&
              sidereon_cdm_parse_xml(xml, len, &cdm_xml) == SIDEREON_STATUS_OK && cdm_xml != NULL,
          "cdm_parse_xml");
    sidereon_cdm_free(cdm_xml);
    free(xml);
}

static void test_rinex_clock(const char *clk_path) {
    size_t len = 0;
    uint8_t *clk = read_file(clk_path, &len);
    if (!clk) {
        return;
    }
    SidereonRinexClock *clock = NULL;
    check(sidereon_rinex_clock_parse(clk, len, &clock) == SIDEREON_STATUS_OK && clock != NULL,
          "rinex_clock_parse");
    if (clock) {
        size_t count = 0;
        check(sidereon_rinex_clock_satellite_count(clock, &count) == SIDEREON_STATUS_OK &&
                  count == W5_EXTRAS_RINEX_CLOCK_SATELLITE_COUNT,
              "rinex_clock_satellite_count");
        size_t written = 0, required = 0;
        check(sidereon_rinex_clock_to_text(clock, NULL, 0, &written, &required) ==
                  SIDEREON_STATUS_OK &&
                  required == W5_EXTRAS_RINEX_CLOCK_TEXT_LEN,
              "rinex_clock_to_text size query");
        sidereon_rinex_clock_free(clock);
    }
    free(clk);

    double gps_seconds = 0.0;
    bool available = false;
    check(sidereon_civil_to_gps_seconds(2020, 6, 25, 0, 0, 0.0, &gps_seconds, &available) ==
              SIDEREON_STATUS_OK &&
              available == W5_EXTRAS_CIVIL_TO_GPS_AVAILABLE &&
              same_bits(gps_seconds, W5_EXTRAS_CIVIL_TO_GPS_SECONDS_BITS),
          "civil_to_gps_seconds");
}

static void test_broadcast_eval(void) {
    double ea = 0.0;
    size_t iters = 0;
    check(sidereon_broadcast_eccentric_anomaly(0.5, 0.01, &ea, &iters) == SIDEREON_STATUS_OK &&
              same_bits(ea, W5_EXTRAS_ECCENTRIC_ANOMALY_RAD_BITS) &&
              iters == W5_EXTRAS_ECCENTRIC_ANOMALY_ITERATIONS,
          "broadcast_eccentric_anomaly");
}

static double *load_sp3_epochs(SidereonSp3 *sp3, size_t *out_count) {
    size_t written = 0, required = 0;
    if (sidereon_sp3_epochs_j2000_seconds(sp3, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        required == 0) {
        return NULL;
    }
    double *epochs = (double *)malloc(required * sizeof(double));
    if (!epochs) {
        return NULL;
    }
    if (sidereon_sp3_epochs_j2000_seconds(sp3, epochs, required, &written, &required) !=
        SIDEREON_STATUS_OK) {
        free(epochs);
        return NULL;
    }
    *out_count = required;
    return epochs;
}

static void test_sp3_coupled(const char *sp3_path) {
    size_t len = 0;
    uint8_t *bytes = read_file(sp3_path, &len);
    if (!bytes) {
        return;
    }
    SidereonSp3 *sp3 = NULL;
    check(sidereon_sp3_load(bytes, len, &sp3) == SIDEREON_STATUS_OK && sp3 != NULL,
          "sp3_load (extras)");
    free(bytes);
    if (!sp3) {
        return;
    }
    size_t epoch_count = 0;
    double *epochs = load_sp3_epochs(sp3, &epoch_count);
    if (!epochs || epoch_count == 0) {
        sidereon_sp3_free(sp3);
        free(epochs);
        return;
    }
    double mid = epochs[epoch_count / 2];

    double pos[3] = {0.0, 0.0, 0.0};
    double clock = 0.0;
    bool has_clock = false;
    int ok = sidereon_sp3_observable_state(sp3, "G01", mid, pos, &clock, &has_clock) ==
             SIDEREON_STATUS_OK;
    check(ok && same_bits_n(pos, W5_EXTRAS_G01_MID_POSITION_M_BITS, 3) &&
              has_clock == W5_EXTRAS_G01_MID_HAS_CLOCK &&
              same_bits(clock, W5_EXTRAS_G01_MID_CLOCK_S_BITS),
          "sp3_observable_state");

    /* DGNSS: build one base observation as the geometric range to a ground
     * point, so the correction path has consistent inputs. */
    double base[3] = {1130773.0, -4831253.0, 3994200.0};
    double dx = pos[0] - base[0], dy = pos[1] - base[1], dz = pos[2] - base[2];
    double range = sqrt(dx * dx + dy * dy + dz * dz);
    SidereonCodeObservation base_obs = {"G01", range};
    SidereonDgnssCorrections *corr = NULL;
    check(sidereon_dgnss_pseudorange_corrections(sp3, base, &base_obs, 1, mid, &corr) ==
              SIDEREON_STATUS_OK &&
              corr != NULL,
          "dgnss_pseudorange_corrections");
    if (corr) {
        size_t corr_count = 0;
        check(sidereon_dgnss_corrections_count(corr, &corr_count) == SIDEREON_STATUS_OK &&
                  corr_count == W5_EXTRAS_DGNSS_CORRECTION_COUNT,
              "dgnss_corrections_count");
        SidereonDgnssApplied *applied = NULL;
        check(sidereon_dgnss_apply_corrections(&base_obs, 1, corr, &applied) ==
                  SIDEREON_STATUS_OK &&
                  applied != NULL,
              "dgnss_apply_corrections");
        if (applied) {
            size_t cc = 0, dc = 0;
            check(sidereon_dgnss_applied_counts(applied, &cc, &dc) == SIDEREON_STATUS_OK &&
                      cc == W5_EXTRAS_DGNSS_CORRECTED_COUNT && dc == W5_EXTRAS_DGNSS_DROPPED_COUNT,
                  "dgnss_applied_counts");
            sidereon_dgnss_applied_free(applied);
        }
        sidereon_dgnss_corrections_free(corr);
    }

    free(epochs);
    sidereon_sp3_free(sp3);
}

static void test_broadcast_comparison(const char *nav_path, const char *precise_sp3_path) {
    size_t nav_len = 0;
    uint8_t *nav = read_file(nav_path, &nav_len);
    if (!nav) {
        return;
    }
    SidereonBroadcastEphemeris *broadcast = NULL;
    check(sidereon_broadcast_ephemeris_parse_nav(nav, nav_len, &broadcast) ==
              SIDEREON_STATUS_OK &&
              broadcast != NULL,
          "broadcast_ephemeris_parse_nav (extras)");
    free(nav);

    size_t sp3_len = 0;
    uint8_t *sp3_bytes = read_file(precise_sp3_path, &sp3_len);
    SidereonSp3 *precise = NULL;
    if (sp3_bytes) {
        check(sidereon_sp3_load(sp3_bytes, sp3_len, &precise) == SIDEREON_STATUS_OK,
              "sp3_load precise (extras)");
        free(sp3_bytes);
    }
    if (!broadcast || !precise) {
        sidereon_broadcast_ephemeris_free(broadcast);
        sidereon_sp3_free(precise);
        return;
    }

    size_t epoch_count = 0;
    double *epochs = load_sp3_epochs(precise, &epoch_count);
    if (epochs && epoch_count > 0) {
        double t = epochs[epoch_count / 2];
        double jd = t / 86400.0 + 2451545.0;
        double whole = floor(jd);
        double frac = jd - whole;
        double half = 1.0 / 86400.0;
        SidereonCompareEpoch ep = {t, whole, frac, whole, frac + half, whole, frac - half};
        const char *sats[1] = {"G01"};
        SidereonBroadcastComparison *report = NULL;
        enum SidereonStatus st = sidereon_broadcast_comparison_compare(broadcast, precise, sats,
                                                                       1, &ep, 1, 1.0, &report);
        /* The engine's outcome for the same epoch (pinned); the binding reports
         * a refusal as INVALID_ARGUMENT (src/broadcast.rs). */
        check(st == (W5_EXTRAS_COMPARE_OK ? SIDEREON_STATUS_OK : SIDEREON_STATUS_INVALID_ARGUMENT),
              "broadcast_comparison_compare outcome");
        if (st == SIDEREON_STATUS_OK && report) {
            SidereonCompareStats overall;
            check(sidereon_broadcast_comparison_overall(report, &overall) == SIDEREON_STATUS_OK &&
                      overall.count == W5_EXTRAS_COMPARE_OVERALL_COUNT,
                  "broadcast_comparison_overall");
            size_t sat_count = 0;
            check(sidereon_broadcast_comparison_satellite_count(report, &sat_count) ==
                          SIDEREON_STATUS_OK &&
                      sat_count == W5_EXTRAS_COMPARE_SATELLITE_COUNT,
                  "broadcast_comparison_satellite_count");
        }
        sidereon_broadcast_comparison_free(report);

        /* Window-form driver: same anchor epoch, a short two-sample window. It
         * builds the per-epoch grid internally and delegates to compare. */
        double t0 = t;
        double t1 = t + 900.0;
        SidereonCompareWindow window = {t0, t1, whole, frac, 900.0, half};
        const char *wsats[1] = {"G01"};
        SidereonBroadcastComparison *wreport = NULL;
        enum SidereonStatus wst = sidereon_broadcast_comparison_compare_window(
            broadcast, precise, wsats, 1, &window, &wreport);
        check(wst == (W5_EXTRAS_COMPARE_WINDOW_OK ? SIDEREON_STATUS_OK
                                                   : SIDEREON_STATUS_INVALID_ARGUMENT),
              "broadcast_comparison_compare_window outcome");
        if (wst == SIDEREON_STATUS_OK && wreport) {
            SidereonCompareStats overall;
            check(sidereon_broadcast_comparison_overall(wreport, &overall) == SIDEREON_STATUS_OK &&
                      overall.count == W5_EXTRAS_COMPARE_WINDOW_OVERALL_COUNT,
                  "broadcast_comparison_compare_window overall");
        }
        sidereon_broadcast_comparison_free(wreport);
    }
    free(epochs);

    sidereon_broadcast_ephemeris_free(broadcast);
    sidereon_sp3_free(precise);
}

int main(int argc, char **argv) {
    if (argc < 7) {
        fprintf(stderr,
                "usage: %s <grg_sp3> <cdm_kvn> <cdm_xml> <rinex_clk> <nav> <precise_sp3>\n",
                argv[0]);
        return 2;
    }

    test_rf();
    test_frequencies_combinations();
    test_carrier_phase_scalars();
    test_signal_quality();
    test_raim();
    test_tropo();
    test_tides();
    test_gravity_tide_system_selector();
    test_angles_eclipse_bodies();
    test_iod_lambert_conjunction();
    test_civil_time();
    test_carrier_smoothing();
    test_broadcast_eval();
    test_cdm(argv[2], argv[3]);
    test_rinex_clock(argv[4]);
    test_sp3_coupled(argv[1]);
    test_broadcast_comparison(argv[5], argv[6]);

    if (failures != 0) {
        fprintf(stderr, "extras_smoke: %d check(s) failed\n", failures);
        return 1;
    }
    printf("extras_smoke: all checks passed\n");
    return 0;
}
