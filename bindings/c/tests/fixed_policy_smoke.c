/*
 * Deterministic C ABI smoke for the fixed-value and policy parity routes.
 * All declarations come from the generated public sidereon.h header.
 */
#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "sidereon.h"

static int failures;

static void check_condition(int condition, const char *what) {
    if (!condition) {
        char error[256] = {0};
        sidereon_last_error_message(error, sizeof(error));
        /* The last error is sticky and may come from an earlier call, so it is
           reported as context rather than as the cause of this failure. */
        if (error[0] != '\0') {
            fprintf(stderr, "FAIL: %s (last ABI error, may predate this check: %s)\n",
                    what, error);
        } else {
            fprintf(stderr, "FAIL: %s\n", what);
        }
        failures++;
    }
}

static void check_status(SidereonStatus actual,
                         SidereonStatus expected,
                         const char *what) {
    check_condition(actual == expected, what);
}

static uint64_t f64_bits(double value) {
    uint64_t bits;
    memcpy(&bits, &value, sizeof(bits));
    return bits;
}

static int f64_same(double left, double right) {
    return f64_bits(left) == f64_bits(right);
}

static int matrix_same(const SidereonCovarianceMatrix6 *left,
                       const SidereonCovarianceMatrix6 *right) {
    for (size_t row = 0; row < 6; row++) {
        for (size_t column = 0; column < 6; column++) {
            if (!f64_same(left->values[row][column], right->values[row][column])) {
                return 0;
            }
        }
    }
    return 1;
}

static int matrix_close(const SidereonCovarianceMatrix6 *left,
                        const SidereonCovarianceMatrix6 *right,
                        double tolerance) {
    for (size_t row = 0; row < 6; row++) {
        for (size_t column = 0; column < 6; column++) {
            if (fabs(left->values[row][column] - right->values[row][column]) > tolerance) {
                return 0;
            }
        }
    }
    return 1;
}

static void set_bits(uint8_t *bits, size_t offset, size_t width, uint64_t value) {
    for (size_t index = 0; index < width; index++) {
        bits[offset + index] = (uint8_t)((value >> (width - index - 1)) & 1U);
    }
}

static void test_covariance(void) {
    const double diagonal_values[6] = {1, 4, 9, 16, 25, 36};
    SidereonCovarianceMatrix6 covariance = {{{0}}};
    SidereonCovarianceMatrix6 scaled = {{{0}}};
    SidereonCovarianceMatrix6 restored = {{{0}}};
    SidereonCovarianceMatrix6 other = {{{0}}};
    SidereonCovarianceMatrix6 interpolated = {{{0}}};
    SidereonCovariance6Validation validation = {false, false};

    check_status(sidereon_covariance6_from_diagonal(diagonal_values, 6, &covariance),
                 SIDEREON_STATUS_OK,
                 "covariance diagonal construction");
    check_condition(covariance.values[0][0] == 1.0 && covariance.values[5][5] == 36.0 &&
                        covariance.values[0][1] == 0.0,
                    "covariance diagonal layout");
    check_status(sidereon_covariance6_validate(&covariance, &validation),
                 SIDEREON_STATUS_OK,
                 "covariance validation");
    check_condition(validation.symmetric && validation.positive_semidefinite,
                    "covariance validation flags");

    double invalid_diagonal[6] = {1, 2, 3, 4, 5, -1};
    check_status(sidereon_covariance6_from_diagonal(invalid_diagonal, 6, &scaled),
                 SIDEREON_STATUS_INVALID_ARGUMENT,
                 "invalid non-PSD covariance construction");
    SidereonCovarianceMatrix6 invalid = covariance;
    invalid.values[5][5] = -1.0;
    check_status(sidereon_covariance6_validate(&invalid, &validation),
                 SIDEREON_STATUS_INVALID_ARGUMENT,
                 "invalid non-PSD covariance validation");
    check_condition(!validation.symmetric && !validation.positive_semidefinite,
                    "invalid covariance output flags");

    check_status(sidereon_covariance6_km_to_m(&covariance, &scaled),
                 SIDEREON_STATUS_OK,
                 "covariance km to m");
    for (size_t index = 0; index < 6; index++) {
        check_condition(f64_same(scaled.values[index][index], diagonal_values[index] * 1.0e6),
                        "covariance exact km to m scale");
    }
    check_status(sidereon_covariance6_m_to_km(&scaled, &restored),
                 SIDEREON_STATUS_OK,
                 "covariance m to km");
    check_condition(matrix_same(&restored, &covariance), "covariance exact scale round trip");

    const double other_diagonal[6] = {4, 9, 16, 25, 36, 49};
    check_status(sidereon_covariance6_from_diagonal(other_diagonal, 6, &other),
                 SIDEREON_STATUS_OK,
                 "second covariance construction");
    check_status(sidereon_covariance6_interpolate_psd(&covariance, &other, 0.0, &interpolated),
                 SIDEREON_STATUS_OK,
                 "covariance interpolation start");
    check_condition(matrix_same(&interpolated, &covariance), "covariance interpolation start value");
    check_status(sidereon_covariance6_interpolate_psd(&covariance, &other, 1.0, &interpolated),
                 SIDEREON_STATUS_OK,
                 "covariance interpolation end");
    check_condition(matrix_same(&interpolated, &other), "covariance interpolation end value");
    check_status(sidereon_covariance6_interpolate_psd(&covariance, &other, 0.5, &interpolated),
                 SIDEREON_STATUS_OK,
                 "covariance interpolation interior");
    check_condition(fabs(interpolated.values[0][0] - 2.0) < 1.0e-12 &&
                        fabs(interpolated.values[5][5] - 42.0) < 1.0e-12,
                    "covariance interpolation interior value");

    SidereonCartesianState state = {0, {7000, 1000, 2000}, {-1, 7.2, 2}};
    SidereonCovarianceMatrix6 rtn = {{{0}}};
    SidereonCovarianceMatrix6 round_trip = {{{0}}};
    check_status(sidereon_covariance6_eci_to_rtn(&covariance, &state, &rtn),
                 SIDEREON_STATUS_OK,
                 "covariance ECI to RTN");
    check_status(sidereon_covariance6_rtn_to_eci(&rtn, &state, &round_trip),
                 SIDEREON_STATUS_OK,
                 "covariance RTN to ECI");
    check_condition(matrix_close(&round_trip, &covariance, 1.0e-12),
                    "covariance transform round trip");
    SidereonCartesianState invalid_state = {0, {0, 0, 0}, {1, 0, 0}};
    check_status(sidereon_covariance6_eci_to_rtn(&covariance, &invalid_state, &rtn),
                 SIDEREON_STATUS_INVALID_ARGUMENT,
                 "invalid covariance transform state");
}

static void test_calendar(void) {
    double second_of_day = 0.0;
    double day_of_year = 0.0;
    uint16_t product_day = 0;
    check_status(sidereon_second_of_day(1, 2, 3.5, &second_of_day),
                 SIDEREON_STATUS_OK,
                 "second of day");
    check_condition(f64_same(second_of_day, 3723.5), "fractional second of day");
    check_status(sidereon_day_of_year(2024, 1, 1, 0, 0, 0.0, &day_of_year),
                 SIDEREON_STATUS_OK,
                 "calendar January first");
    check_condition(f64_same(day_of_year, 1.0), "calendar January first value");
    check_status(sidereon_day_of_year(2024, 2, 29, 12, 0, 0.25, &day_of_year),
                 SIDEREON_STATUS_OK,
                 "calendar leap date");
    check_condition(fabs(day_of_year - (60.0 + 43200.25 / 86400.0)) < 1.0e-12,
                    "calendar fractional leap date");
    check_status(sidereon_data_day_of_year(2020, 3, 1, &product_day),
                 SIDEREON_STATUS_OK,
                 "product day of year");
    check_condition(product_day == 61, "product day integer value");
    check_status(sidereon_day_of_year(2023, 2, 29, 0, 0, 0.0, &day_of_year),
                 SIDEREON_STATUS_INVALID_ARGUMENT,
                 "invalid non-leap date");
}

static void test_rinex_policy(void) {
    double frequency = 0.0;
    double wavelength = 0.0;
    check_status(sidereon_rinex_band_frequency_hz(SIDEREON_GNSS_SYSTEM_GPS,
                                                  "1",
                                                  false,
                                                  0,
                                                  &frequency),
                 SIDEREON_STATUS_OK,
                 "GPS RINEX band frequency");
    check_condition(f64_same(frequency, 1575420000.0), "GPS RINEX band frequency value");
    check_status(sidereon_rinex_band_wavelength_m(SIDEREON_GNSS_SYSTEM_GPS,
                                                  "1",
                                                  false,
                                                  0,
                                                  &wavelength),
                 SIDEREON_STATUS_OK,
                 "GPS RINEX band wavelength");
    check_condition(fabs(frequency * wavelength - 299792458.0) < 1.0e-6,
                    "RINEX frequency wavelength policy");
    check_status(sidereon_rinex_band_frequency_hz(SIDEREON_GNSS_SYSTEM_GLONASS,
                                                  "1",
                                                  true,
                                                  -4,
                                                  &frequency),
                 SIDEREON_STATUS_OK,
                 "GLONASS channel frequency");
    check_condition(f64_same(frequency, 1599750000.0), "GLONASS channel frequency value");
    check_status(sidereon_rinex_band_frequency_hz(SIDEREON_GNSS_SYSTEM_GLONASS,
                                                  "1",
                                                  false,
                                                  0,
                                                  &frequency),
                 SIDEREON_STATUS_INVALID_ARGUMENT,
                 "missing GLONASS channel");

    check_status(sidereon_rinex_observation_frequency_hz(SIDEREON_GNSS_SYSTEM_BEI_DOU,
                                                         "C1I",
                                                         3.02,
                                                         false,
                                                         0,
                                                         &frequency),
                 SIDEREON_STATUS_OK,
                 "BeiDou RINEX observation frequency");
    check_condition(f64_same(frequency, 1561098000.0), "BeiDou RINEX version 3.02 policy");
    check_status(sidereon_rinex_observation_frequency_hz(SIDEREON_GNSS_SYSTEM_BEI_DOU,
                                                         "C1I",
                                                         3.03,
                                                         false,
                                                         0,
                                                         &frequency),
                 SIDEREON_STATUS_OK,
                 "BeiDou RINEX version 3.03 frequency");
    check_condition(f64_same(frequency, 1575420000.0), "BeiDou RINEX version 3.03 policy");
    check_status(sidereon_rinex_observation_wavelength_m(SIDEREON_GNSS_SYSTEM_BEI_DOU,
                                                         "C1I",
                                                         3.03,
                                                         false,
                                                         0,
                                                         &wavelength),
                 SIDEREON_STATUS_OK,
                 "BeiDou RINEX observation wavelength");
    check_condition(fabs(frequency * wavelength - 299792458.0) < 1.0e-6,
                    "RINEX observation wavelength policy");

    const uint32_t systems[3] = {SIDEREON_GNSS_SYSTEM_GPS,
                                 SIDEREON_GNSS_SYSTEM_GALILEO,
                                 SIDEREON_GNSS_SYSTEM_BEI_DOU};
    const SidereonCarrierBand first[3] = {SIDEREON_CARRIER_BAND_L1,
                                          SIDEREON_CARRIER_BAND_E1,
                                          SIDEREON_CARRIER_BAND_B1I};
    const SidereonCarrierBand second[3] = {SIDEREON_CARRIER_BAND_L2,
                                           SIDEREON_CARRIER_BAND_E5A,
                                           SIDEREON_CARRIER_BAND_B3I};
    for (size_t index = 0; index < 3; index++) {
        SidereonCarrierPair pair = {SIDEREON_CARRIER_BAND_L1, SIDEREON_CARRIER_BAND_L1};
        bool present = false;
        check_status(sidereon_default_iono_free_pair(systems[index], &pair, &present),
                     SIDEREON_STATUS_OK,
                     "default ionosphere-free pair");
        check_condition(present && pair.band1 == first[index] && pair.band2 == second[index],
                        "default ionosphere-free pair value");
    }
    SidereonCarrierPair no_pair = {SIDEREON_CARRIER_BAND_L1, SIDEREON_CARRIER_BAND_L1};
    bool present = true;
    check_status(sidereon_default_iono_free_pair(SIDEREON_GNSS_SYSTEM_GLONASS,
                                                 &no_pair,
                                                 &present),
                 SIDEREON_STATUS_OK,
                 "GLONASS default pair absence");
    check_condition(!present, "GLONASS has no default ionosphere-free pair");
}

static void test_lnav_words(void) {
    uint8_t how[30] = {0};
    uint8_t subframe[300] = {0};
    uint64_t tow = 0;
    uint64_t subframe_id = 0;
    set_bits(how, 0, 17, 12345);
    set_bits(how, 19, 3, 5);
    memcpy(subframe + 30, how, sizeof(how));
    check_status(sidereon_lnav_tow(how, sizeof(how), &tow), SIDEREON_STATUS_OK, "LNAV HOW TOW");
    check_condition(tow == 12345, "LNAV HOW TOW value");
    check_status(sidereon_lnav_subframe_id(how, sizeof(how), &subframe_id),
                 SIDEREON_STATUS_OK,
                 "LNAV HOW subframe ID");
    check_condition(subframe_id == 5, "LNAV HOW subframe ID value");
    check_status(sidereon_lnav_tow(subframe, sizeof(subframe), &tow),
                 SIDEREON_STATUS_OK,
                 "LNAV subframe TOW");
    check_condition(tow == 12345, "LNAV subframe TOW value");
    check_status(sidereon_lnav_subframe_id(subframe, sizeof(subframe), &subframe_id),
                 SIDEREON_STATUS_OK,
                 "LNAV subframe ID");
    check_condition(subframe_id == 5, "LNAV subframe ID value");
    uint8_t malformed[29] = {0};
    check_status(sidereon_lnav_tow(malformed, sizeof(malformed), &tow),
                 SIDEREON_STATUS_INVALID_ARGUMENT,
                 "LNAV malformed TOW length");

    uint8_t source[24] = {0};
    uint8_t parity[6] = {0};
    uint8_t word[30] = {0};
    check_status(sidereon_lnav_parity(source, sizeof(source), 1, 0, parity, sizeof(parity)),
                 SIDEREON_STATUS_OK,
                 "LNAV parity D29/D30 dependency");
    const uint8_t expected_parity[6] = {1, 0, 1, 0, 0, 1};
    check_condition(memcmp(parity, expected_parity, sizeof(parity)) == 0,
                    "LNAV parity value");
    memcpy(word + 24, parity, sizeof(parity));
    bool valid = false;
    check_status(sidereon_lnav_parity_valid(word, sizeof(word), 1, 0, &valid),
                 SIDEREON_STATUS_OK,
                 "LNAV parity validity");
    check_condition(valid, "LNAV parity valid word");
    word[0] = 1;
    check_status(sidereon_lnav_parity_valid(word, sizeof(word), 1, 0, &valid),
                 SIDEREON_STATUS_OK,
                 "LNAV invalid parity word");
    check_condition(!valid, "LNAV parity invalid word");
    check_status(sidereon_lnav_parity(source, sizeof(source), 0, 0, parity, 5),
                 SIDEREON_STATUS_INVALID_ARGUMENT,
                 "LNAV parity output length");
    check_status(sidereon_lnav_parity(malformed, sizeof(malformed), 0, 0, parity, sizeof(parity)),
                 SIDEREON_STATUS_INVALID_ARGUMENT,
                 "LNAV parity input length");
}

int main(void) {
    test_covariance();
    test_calendar();
    test_rinex_policy();
    test_lnav_words();
    if (failures != 0) {
        fprintf(stderr, "fixed_policy_smoke: %d failure(s)\n", failures);
        return 1;
    }
    puts("fixed_policy_smoke: OK");
    return 0;
}
