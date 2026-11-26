#include "sidereon.h"
#include "orbit_contracts_pins.h"

#include <assert.h>
#include <float.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

_Static_assert(SIDEREON_ANALYTIC_THEORY_BROUWER_LYDDANE == 1, "SIDEREON_ANALYTIC_THEORY_BROUWER_LYDDANE");
_Static_assert(SIDEREON_ANALYTIC_THEORY_ECKSTEIN_HECHLER == 0, "SIDEREON_ANALYTIC_THEORY_ECKSTEIN_HECHLER");
_Static_assert(SIDEREON_ATTITUDE_MODE_MIDNIGHT_TURN == 2, "SIDEREON_ATTITUDE_MODE_MIDNIGHT_TURN");
_Static_assert(SIDEREON_ATTITUDE_MODE_MODIFIED_YAW_STEERING == 5, "SIDEREON_ATTITUDE_MODE_MODIFIED_YAW_STEERING");
_Static_assert(SIDEREON_ATTITUDE_MODE_NOMINAL == 0, "SIDEREON_ATTITUDE_MODE_NOMINAL");
_Static_assert(SIDEREON_ATTITUDE_MODE_NOON_TURN == 1, "SIDEREON_ATTITUDE_MODE_NOON_TURN");
_Static_assert(SIDEREON_ATTITUDE_MODE_ORBIT_NORMAL == 4, "SIDEREON_ATTITUDE_MODE_ORBIT_NORMAL");
_Static_assert(SIDEREON_ATTITUDE_MODE_SHADOW == 3, "SIDEREON_ATTITUDE_MODE_SHADOW");
_Static_assert(SIDEREON_BEIDOU_MANUFACTURER_CAST == 0, "SIDEREON_BEIDOU_MANUFACTURER_CAST");
_Static_assert(SIDEREON_BEIDOU_MANUFACTURER_SECM == 1, "SIDEREON_BEIDOU_MANUFACTURER_SECM");
_Static_assert(SIDEREON_BEIDOU_ORBIT_CLASS_GEO == 0, "SIDEREON_BEIDOU_ORBIT_CLASS_GEO");
_Static_assert(SIDEREON_BEIDOU_ORBIT_CLASS_IGSO == 1, "SIDEREON_BEIDOU_ORBIT_CLASS_IGSO");
_Static_assert(SIDEREON_BEIDOU_ORBIT_CLASS_MEO == 2, "SIDEREON_BEIDOU_ORBIT_CLASS_MEO");
_Static_assert(SIDEREON_CLOCK_DATUM_EPOCH_SYSTEM_MEDIAN == 0, "SIDEREON_CLOCK_DATUM_EPOCH_SYSTEM_MEDIAN");
_Static_assert(SIDEREON_CLOCK_DATUM_UNADJUSTED == 1, "SIDEREON_CLOCK_DATUM_UNADJUSTED");
_Static_assert(SIDEREON_COSINE_TURN_START_ANGLE_COLINEARITY == 0, "SIDEREON_COSINE_TURN_START_ANGLE_COLINEARITY");
_Static_assert(SIDEREON_COSINE_TURN_START_ANGLE_ORBIT_ANGLE == 1, "SIDEREON_COSINE_TURN_START_ANGLE_ORBIT_ANGLE");
_Static_assert(SIDEREON_DE_BODY_EARTH == 4, "SIDEREON_DE_BODY_EARTH");
_Static_assert(SIDEREON_DE_BODY_EARTH_MOON == 3, "SIDEREON_DE_BODY_EARTH_MOON");
_Static_assert(SIDEREON_DE_BODY_JUPITER == 7, "SIDEREON_DE_BODY_JUPITER");
_Static_assert(SIDEREON_DE_BODY_MARS == 6, "SIDEREON_DE_BODY_MARS");
_Static_assert(SIDEREON_DE_BODY_MERCURY == 1, "SIDEREON_DE_BODY_MERCURY");
_Static_assert(SIDEREON_DE_BODY_MOON == 5, "SIDEREON_DE_BODY_MOON");
_Static_assert(SIDEREON_DE_BODY_NEPTUNE == 10, "SIDEREON_DE_BODY_NEPTUNE");
_Static_assert(SIDEREON_DE_BODY_PLUTO == 11, "SIDEREON_DE_BODY_PLUTO");
_Static_assert(SIDEREON_DE_BODY_SATURN == 8, "SIDEREON_DE_BODY_SATURN");
_Static_assert(SIDEREON_DE_BODY_SOLAR_SYSTEM == 12, "SIDEREON_DE_BODY_SOLAR_SYSTEM");
_Static_assert(SIDEREON_DE_BODY_SUN == 0, "SIDEREON_DE_BODY_SUN");
_Static_assert(SIDEREON_DE_BODY_URANUS == 9, "SIDEREON_DE_BODY_URANUS");
_Static_assert(SIDEREON_DE_BODY_VENUS == 2, "SIDEREON_DE_BODY_VENUS");
_Static_assert(SIDEREON_EMPIRICAL_HARMONIC_CONSTANT == 0, "SIDEREON_EMPIRICAL_HARMONIC_CONSTANT");
_Static_assert(SIDEREON_EMPIRICAL_HARMONIC_COSINE == 1, "SIDEREON_EMPIRICAL_HARMONIC_COSINE");
_Static_assert(SIDEREON_EMPIRICAL_HARMONIC_SINE == 2, "SIDEREON_EMPIRICAL_HARMONIC_SINE");
_Static_assert(SIDEREON_FRAME_AGREEMENT_KIND_ASSERTED_EQUIVALENT == 1, "SIDEREON_FRAME_AGREEMENT_KIND_ASSERTED_EQUIVALENT");
_Static_assert(SIDEREON_FRAME_AGREEMENT_KIND_BROADCAST_DATUMS == 3, "SIDEREON_FRAME_AGREEMENT_KIND_BROADCAST_DATUMS");
_Static_assert(SIDEREON_FRAME_AGREEMENT_KIND_SAME_LABEL == 0, "SIDEREON_FRAME_AGREEMENT_KIND_SAME_LABEL");
_Static_assert(SIDEREON_GAP_POLICY_COUNT == 0, "SIDEREON_GAP_POLICY_COUNT");
_Static_assert(SIDEREON_GAP_POLICY_REFUSE == 1, "SIDEREON_GAP_POLICY_REFUSE");
_Static_assert(SIDEREON_GNSS_BLOCK_BEIDOU2_GEO == 8, "SIDEREON_GNSS_BLOCK_BEIDOU2_GEO");
_Static_assert(SIDEREON_GNSS_BLOCK_BEIDOU2_IGSO == 9, "SIDEREON_GNSS_BLOCK_BEIDOU2_IGSO");
_Static_assert(SIDEREON_GNSS_BLOCK_BEIDOU2_MEO == 10, "SIDEREON_GNSS_BLOCK_BEIDOU2_MEO");
_Static_assert(SIDEREON_GNSS_BLOCK_BEIDOU3_GEO == 11, "SIDEREON_GNSS_BLOCK_BEIDOU3_GEO");
_Static_assert(SIDEREON_GNSS_BLOCK_BEIDOU3_IGSO == 12, "SIDEREON_GNSS_BLOCK_BEIDOU3_IGSO");
_Static_assert(SIDEREON_GNSS_BLOCK_GALILEO_FOC == 7, "SIDEREON_GNSS_BLOCK_GALILEO_FOC");
_Static_assert(SIDEREON_GNSS_BLOCK_GALILEO_IOV == 6, "SIDEREON_GNSS_BLOCK_GALILEO_IOV");
_Static_assert(SIDEREON_GNSS_BLOCK_GLONASS_M == 5, "SIDEREON_GNSS_BLOCK_GLONASS_M");
_Static_assert(SIDEREON_GNSS_BLOCK_GPS_IIA == 0, "SIDEREON_GNSS_BLOCK_GPS_IIA");
_Static_assert(SIDEREON_GNSS_BLOCK_GPS_IIF == 3, "SIDEREON_GNSS_BLOCK_GPS_IIF");
_Static_assert(SIDEREON_GNSS_BLOCK_GPS_III == 4, "SIDEREON_GNSS_BLOCK_GPS_III");
_Static_assert(SIDEREON_GNSS_BLOCK_GPS_IIR == 1, "SIDEREON_GNSS_BLOCK_GPS_IIR");
_Static_assert(SIDEREON_GNSS_BLOCK_GPS_IIRM == 2, "SIDEREON_GNSS_BLOCK_GPS_IIRM");
_Static_assert(SIDEREON_ICGEM_ERRORS_CALIBRATED == 1, "SIDEREON_ICGEM_ERRORS_CALIBRATED");
_Static_assert(SIDEREON_ICGEM_ERRORS_CALIBRATED_AND_FORMAL == 3, "SIDEREON_ICGEM_ERRORS_CALIBRATED_AND_FORMAL");
_Static_assert(SIDEREON_ICGEM_ERRORS_FORMAL == 2, "SIDEREON_ICGEM_ERRORS_FORMAL");
_Static_assert(SIDEREON_ICGEM_ERRORS_NO == 0, "SIDEREON_ICGEM_ERRORS_NO");
_Static_assert(SIDEREON_ICGEM_FORMAT_ICGEM1 == 0, "SIDEREON_ICGEM_FORMAT_ICGEM1");
_Static_assert(SIDEREON_ICGEM_FORMAT_ICGEM2 == 1, "SIDEREON_ICGEM_FORMAT_ICGEM2");
_Static_assert(SIDEREON_ICGEM_NORMALIZATION_FULLY_NORMALIZED == 0, "SIDEREON_ICGEM_NORMALIZATION_FULLY_NORMALIZED");
_Static_assert(SIDEREON_ICGEM_NORMALIZATION_UNNORMALIZED == 1, "SIDEREON_ICGEM_NORMALIZATION_UNNORMALIZED");
_Static_assert(SIDEREON_ICGEM_TERM_KIND_CONSTANT == 0, "SIDEREON_ICGEM_TERM_KIND_CONSTANT");
_Static_assert(SIDEREON_ICGEM_TERM_KIND_TREND == 1, "SIDEREON_ICGEM_TERM_KIND_TREND");
_Static_assert(SIDEREON_ORBIT_FRAME_BROADCAST_DATUMS == 1, "SIDEREON_ORBIT_FRAME_BROADCAST_DATUMS");
_Static_assert(SIDEREON_ORBIT_REFERENCE_POINT_ANTENNA_PHASE_CENTER == 1, "SIDEREON_ORBIT_REFERENCE_POINT_ANTENNA_PHASE_CENTER");
_Static_assert(SIDEREON_ORBIT_REFERENCE_POINT_CENTER_OF_MASS == 0, "SIDEREON_ORBIT_REFERENCE_POINT_CENTER_OF_MASS");
_Static_assert(SIDEREON_ORBIT_REGIME_GEO == 2, "SIDEREON_ORBIT_REGIME_GEO");
_Static_assert(SIDEREON_ORBIT_REGIME_IGSO == 1, "SIDEREON_ORBIT_REGIME_IGSO");
_Static_assert(SIDEREON_ORBIT_REGIME_MEO == 0, "SIDEREON_ORBIT_REGIME_MEO");
_Static_assert(SIDEREON_RTN_AXIS_ALONG_TRACK == 1, "SIDEREON_RTN_AXIS_ALONG_TRACK");
_Static_assert(SIDEREON_RTN_AXIS_CROSS_TRACK == 2, "SIDEREON_RTN_AXIS_CROSS_TRACK");
_Static_assert(SIDEREON_RTN_AXIS_RADIAL == 0, "SIDEREON_RTN_AXIS_RADIAL");
_Static_assert(SIDEREON_SCORED_PRODUCT_REFERENCE == 1, "SIDEREON_SCORED_PRODUCT_REFERENCE");
_Static_assert(SIDEREON_SCORED_PRODUCT_SOURCE == 0, "SIDEREON_SCORED_PRODUCT_SOURCE");
_Static_assert(SIDEREON_SHADOW_FUNCTION_APPARENT_DISC == 0, "SIDEREON_SHADOW_FUNCTION_APPARENT_DISC");
_Static_assert(SIDEREON_SHADOW_FUNCTION_CONICAL == 1, "SIDEREON_SHADOW_FUNCTION_CONICAL");
_Static_assert(SIDEREON_TRIAD_VELOCITY_EARTH_FIXED == 1, "SIDEREON_TRIAD_VELOCITY_EARTH_FIXED");
_Static_assert(SIDEREON_TRIAD_VELOCITY_INERTIAL == 0, "SIDEREON_TRIAD_VELOCITY_INERTIAL");

static void detail(SidereonEngineErrorFamily family, const char *kind, const char *fields) {
    SidereonEngineErrorInfo info;
    assert(sidereon_last_engine_error_info(&info) == SIDEREON_STATUS_OK);
    assert(info.family == family);
    uint8_t actual[1024];
    assert(info.payload_len < sizeof(actual));
    size_t written, required;
    assert(sidereon_last_engine_error_payload(actual, info.payload_len, &written, &required) ==
           SIDEREON_STATUS_OK);
    assert(written == info.payload_len && required == written);
    actual[written] = 0;
    char expected[768];
    int count = snprintf(expected, sizeof(expected),
                         "\"error\":{\"kind\":\"%s\",\"fields\":%s}", kind, fields);
    assert(count > 0 && (size_t)count < sizeof(expected));
    if (!strstr((const char *)actual, expected)) {
        fprintf(stderr, "actual %s expected %s\n", (char *)actual, expected);
        abort();
    }
}

static void legacy_message(const char *expected) {
    char text[512];
    assert(sidereon_last_error_message(text, sizeof(text)) == strlen(expected));
    assert(strcmp(text, expected) == 0);
}

#define F64(decimal, bits) "{\"decimal\":\"" decimal "\",\"bits_hex\":\"" bits "\"}"
#define ICGEM_HEADER "product_type gravity_field\nmodelname T\nearth_gravity_constant 3.986004415e14\n" \
                     "radius 6378136.3\nmax_degree 2\nerrors no\n"
#define ICGEM_HEAD ICGEM_HEADER "end_of_head\n"
#define ICGEM_HEAD2 ICGEM_HEADER "format icgem2.0\nend_of_head\n"
#define ICGEM_21 "gfc 2 1 0.0 0.0\n"
#define ICGEM_22 "gfc 2 2 2.4e-6 -1.4e-6\n"
#define ICGEM_ROWS "gfc 2 0 -4.8e-4 0.0\n" ICGEM_21 ICGEM_22
#define ICGEM_TIMED "gfct 2 0 -4.8e-4 0.0 20000101.0000 20300101.0000\n"
#define ICGEM_TIMED_ROWS ICGEM_TIMED ICGEM_21 ICGEM_22

static SidereonStatus parse_field(const char *text, SidereonIcgemGravityField **field) {
    return sidereon_icgem_gravity_field_parse((const uint8_t *)text, strlen(text), field);
}

static void icgem_parse_errors(void) {
    static const struct { const char *source, *kind, *fields; } cases[] = {
        {ICGEM_HEAD "gfc 2 0 -4.8e-4\n" ICGEM_21 ICGEM_22,
         "missing_field", "{\"line\":8,\"keyword\":\"gfc\",\"field\":\"S\"}"},
        {ICGEM_HEAD "gfc 2 0 bad 0.0\n" ICGEM_21 ICGEM_22,
         "invalid_field", "{\"line\":8,\"keyword\":\"gfc\",\"field\":\"C\",\"value\":\"bad\"}"},
        {"end_of_head\n", "missing_header_keyword", "{\"keyword\":\"product_type\"}"},
        {ICGEM_HEADER "modelname second\nend_of_head\n",
         "repeated_header_keyword", "{\"line\":7,\"first_line\":2,\"keyword\":\"modelname\"}"},
        {ICGEM_HEADER, "missing_end_of_head", "{}"},
        {ICGEM_HEAD2 "dot 2 0 1e-12 0.0\n" ICGEM_21 ICGEM_22,
         "keyword_not_in_format", "{\"line\":9,\"keyword\":\"dot\"}"},
        {ICGEM_HEAD "bad 2 0 -4.8e-4 0.0\n" ICGEM_21 ICGEM_22,
         "unknown_record_keyword", "{\"line\":8,\"keyword\":\"bad\"}"},
        {ICGEM_HEAD "2 0 -4.8e-4 0.0\n" ICGEM_21 ICGEM_22,
         "missing_record_keyword", "{\"line\":8}"},
        {ICGEM_HEAD "gfc 3 0 -4.8e-4 0.0\n" ICGEM_21 ICGEM_22,
         "degree_above_maximum", "{\"line\":8,\"degree\":3,\"max_degree\":2}"},
        {ICGEM_HEAD "gfc 2 3 -4.8e-4 0.0\n" ICGEM_21 ICGEM_22,
         "order_above_degree", "{\"line\":8,\"degree\":2,\"order\":3}"},
        {ICGEM_HEAD ICGEM_ROWS ICGEM_21,
         "repeated_coefficient", "{\"line\":11,\"first_line\":9,\"degree\":2,\"order\":1}"},
        {ICGEM_HEAD2 ICGEM_TIMED_ROWS ICGEM_TIMED,
         "overlapping_term", "{\"line\":12,\"first_line\":9,\"keyword\":\"gfct\",\"degree\":2,\"order\":0}"},
        {ICGEM_HEAD2 ICGEM_TIMED_ROWS "trnd 2 0 1e-12 0.0 19990101.0000 20300101.0000\n",
         "uncovered_term", "{\"line\":12,\"keyword\":\"trnd\",\"degree\":2,\"order\":0}"},
        {ICGEM_HEAD "gfc 0 0 1.0 0.0\n" ICGEM_21 ICGEM_22,
         "missing_coefficient", "{\"degree\":2,\"order\":0}"},
        {ICGEM_HEAD ICGEM_21 ICGEM_22,
         "too_few_records", "{\"max_degree\":2,\"required\":3,\"lines\":2}"},
        {ICGEM_HEADER "norm unnormalized\nend_of_head\n"
         "gfc 2 2 1.7e308 0.0\ngfc 2 0 0.0 0.0\n" ICGEM_21,
         "normalization_out_of_range", "{\"line\":9,\"degree\":2,\"order\":2}"},
    };
    for (size_t i = 0; i < sizeof(cases) / sizeof(cases[0]); ++i) {
        SidereonIcgemGravityField *field = NULL;
        if (parse_field(cases[i].source, &field) != SIDEREON_STATUS_INVALID_ARGUMENT) {
            fprintf(stderr, "expected ICGEM %s\n", cases[i].kind);
            abort();
        }
        assert(field == NULL);
        detail(SIDEREON_ENGINE_ERROR_FAMILY_ICGEM, cases[i].kind, cases[i].fields);
    }
}

static void icgem_queries(void) {
    const char text[] = ICGEM_HEAD2 ICGEM_TIMED_ROWS
        "acos 2 0 1.0e-11 0.0 20000101.0000 20300101.0000 1e-320\n";
    SidereonIcgemGravityField *field = NULL, *other = NULL;
    assert(sidereon_icgem_gravity_field_parse_truncated(
               (const uint8_t *)text, strlen(text), 3, 2, &field) == SIDEREON_STATUS_INVALID_ARGUMENT);
    detail(SIDEREON_ENGINE_ERROR_FAMILY_ICGEM, "truncation_out_of_range",
           "{\"degree\":3,\"order\":2,\"max_degree\":2}");
    assert(sidereon_icgem_gravity_field_parse_truncated(
               (const uint8_t *)text, strlen(text), 2, 1, &field) == SIDEREON_STATUS_OK);
    assert(sidereon_icgem_gravity_field_with_assumed_tide_system(
               field, SIDEREON_GRAVITY_TIDE_SYSTEM_TIDE_FREE, (SidereonIcgemGravityField **)field) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    legacy_message("sidereon_icgem_gravity_field_with_assumed_tide_system: out overlaps field");
    assert(sidereon_icgem_gravity_field_gravity(field, 2, 1, (SidereonSphericalHarmonicGravity **)field) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    legacy_message("sidereon_icgem_gravity_field_gravity: out overlaps field");
    assert(sidereon_icgem_gravity_field_gravity_in_tide_system(
               field, 2, 1, SIDEREON_GRAVITY_TIDE_SYSTEM_TIDE_FREE,
               (SidereonSphericalHarmonicGravity **)field) == SIDEREON_STATUS_INVALID_ARGUMENT);
    legacy_message("sidereon_icgem_gravity_field_gravity_in_tide_system: out overlaps field");
    uint16_t limit;
    assert(sidereon_icgem_gravity_field_stored_max_degree(field, &limit) == SIDEREON_STATUS_OK &&
           limit == 2);
    assert(sidereon_icgem_gravity_field_stored_max_order(field, &limit) == SIDEREON_STATUS_OK &&
           limit == 1);
    bool flag;
    assert(sidereon_icgem_gravity_field_is_time_variable(field, &flag) == SIDEREON_STATUS_OK &&
           flag);
    SidereonIcgemHeader header;
    assert(sidereon_icgem_gravity_field_header(field, &header) == SIDEREON_STATUS_OK);
    assert(header.gravity_constant_m3_s2 == 3.986004415e14 && header.radius_m == 6378136.3 &&
           header.max_degree == 2 && header.errors == SIDEREON_ICGEM_ERRORS_NO &&
           header.normalization == SIDEREON_ICGEM_NORMALIZATION_FULLY_NORMALIZED &&
           !header.has_tide_system && header.tide_system == 0 && header.format == SIDEREON_ICGEM_FORMAT_ICGEM2);
    uint8_t name[1];
    size_t written, required;
    assert(sidereon_icgem_gravity_field_model_name(field, name, sizeof(name), &written, &required) ==
           SIDEREON_STATUS_OK && written == 1 && required == 1 && name[0] == 'T');

    SidereonOptionalHarmonicCoefficient coefficient;
    assert(sidereon_icgem_gravity_field_coefficient_at(field, 2, 0, NAN, &coefficient) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    detail(SIDEREON_ENGINE_ERROR_FAMILY_ICGEM, "non_finite_epoch", "{}");
    legacy_message("sidereon_icgem_gravity_field_coefficient_at: epoch_tt_seconds must be finite");
    SidereonIcgemSigmas no_sigmas;
    assert(sidereon_icgem_gravity_field_static_sigmas(field, 2, 1, &no_sigmas) ==
           SIDEREON_STATUS_OK);
    assert(!no_sigmas.has_calibrated && !no_sigmas.has_formal &&
           no_sigmas.calibrated[0] == 0.0 && no_sigmas.calibrated[1] == 0.0 &&
           no_sigmas.formal[0] == 0.0 && no_sigmas.formal[1] == 0.0);
    assert(sidereon_icgem_gravity_field_coefficient_at(field, 2, 0, -43201.0, &coefficient) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    detail(SIDEREON_ENGINE_ERROR_FAMILY_ICGEM, "epoch_outside_validity",
           "{\"degree\":2,\"order\":0,\"epoch_tt_seconds\":" F64("-43201", "c0e5182000000000") "}");
    assert(sidereon_icgem_gravity_field_coefficient_at(field, 2, 0, -43199.0, &coefficient) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    detail(SIDEREON_ENGINE_ERROR_FAMILY_ICGEM, "non_finite_coefficient",
           "{\"degree\":2,\"order\":0,\"epoch_tt_seconds\":" F64("-43199", "c0e517e000000000") "}");
    assert(sidereon_icgem_gravity_field_coefficient_at(field, 2, 0, -43200.0, &coefficient) ==
           SIDEREON_STATUS_OK);
    assert(coefficient.present && coefficient.coefficient.c == -4.8e-4 + 1.0e-11 &&
           coefficient.coefficient.s == 0.0);
    assert(sidereon_icgem_gravity_field_static_coefficient(field, 2, 1, &coefficient) ==
           SIDEREON_STATUS_OK && coefficient.present && coefficient.coefficient.c == 0.0 &&
           coefficient.coefficient.s == 0.0);
    assert(sidereon_icgem_gravity_field_static_coefficient(field, 2, 2, &coefficient) ==
           SIDEREON_STATUS_OK && !coefficient.present);

    SidereonSphericalHarmonicGravity *gravity = NULL;
    assert(sidereon_icgem_gravity_field_gravity_in_tide_system(
               field, 2, 1, SIDEREON_GRAVITY_TIDE_SYSTEM_TIDE_FREE, &gravity) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    detail(SIDEREON_ENGINE_ERROR_FAMILY_ICGEM, "unknown_tide_system", "{}");
    assert(sidereon_icgem_gravity_field_with_assumed_tide_system(
               field, SIDEREON_GRAVITY_TIDE_SYSTEM_TIDE_FREE, &other) == SIDEREON_STATUS_OK);
    assert(sidereon_icgem_gravity_field_equal(field, field, &flag) == SIDEREON_STATUS_OK && flag);
    assert(sidereon_icgem_gravity_field_equal(field, other, &flag) == SIDEREON_STATUS_OK && !flag);
    assert(sidereon_icgem_gravity_field_header(other, &header) == SIDEREON_STATUS_OK &&
           header.has_tide_system && header.tide_system == SIDEREON_GRAVITY_TIDE_SYSTEM_TIDE_FREE);
    assert(sidereon_icgem_gravity_field_gravity_in_tide_system(
               other, 2, 1, SIDEREON_GRAVITY_TIDE_SYSTEM_TIDE_FREE, &gravity) == SIDEREON_STATUS_OK);
    assert(sidereon_spherical_harmonic_gravity_is_time_variable(gravity, &flag) ==
           SIDEREON_STATUS_OK && flag);
    assert(sidereon_spherical_harmonic_gravity_coefficient_at(gravity, 2, 0, -43200.0,
                                                             &coefficient) == SIDEREON_STATUS_OK);
    assert(coefficient.present && coefficient.coefficient.c == -4.8e-4 + 1.0e-11 &&
           coefficient.coefficient.s == 0.0);
    sidereon_spherical_harmonic_gravity_free(gravity);
    sidereon_icgem_gravity_field_free(field);
    assert(sidereon_icgem_gravity_field_with_assumed_tide_system(
               other, SIDEREON_GRAVITY_TIDE_SYSTEM_ZERO_TIDE, &field) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    detail(SIDEREON_ENGINE_ERROR_FAMILY_ICGEM, "tide_system_conflict",
           "{\"stated\":0,\"assumed\":1}");
    sidereon_icgem_gravity_field_free(other);
}


static const double PI = 0x1.921fb54442d18p+1;

static void gnss_blocks_and_constants(void) {
    const double rate = sidereon_gps_iir_default_yaw_rate_rad_s();
    assert(rate == 0.2 * PI / 180.0);
    assert(sidereon_gps_iia_default_yaw_bias_rad() == 0.5 * PI / 180.0);
    assert(sidereon_gps_iif_default_yaw_rate_rad_s() == 0.11 * PI / 180.0);
    assert(sidereon_gps_iif_default_yaw_bias_rad() == -0.7 * PI / 180.0);
    assert(sidereon_gps_shadow_limit_rad() == (180.0 - 13.25) * PI / 180.0);
    assert(sidereon_glonass_m_default_yaw_rate_rad_s() == 0.25 * PI / 180.0);
    assert(sidereon_glonass_m_shadow_limit_rad() == (180.0 - 14.2) * PI / 180.0);
    assert(sidereon_galileo_iov_beta_x_rad() == 15.0 * PI / 180.0);
    assert(sidereon_galileo_iov_beta_y_rad() == 2.0 * PI / 180.0);
    assert(sidereon_gps_iii_beta_x_rad() == 15.0 * PI / 180.0);
    assert(sidereon_gps_iii_beta_y_rad() == 5.8 * PI / 180.0);
    assert(sidereon_beidou3_secm_beta0_rad() == 3.0 * PI / 180.0);
    assert(sidereon_beidou2_ys_to_on_beta_rad() == 4.1 * PI / 180.0);
    assert(sidereon_beidou2_on_to_ys_beta_rad() == 3.9 * PI / 180.0);
    assert(sidereon_iau_2015_solar_radius_km() == 695700.0);
    assert(sidereon_default_velocity_half_step_s() == 0.5);
    assert(sidereon_min_velocity_half_step_s() == 1e-8);

    SidereonCosineTurnParameters turn = sidereon_galileo_foc_turn();
    assert(turn.beta_limit_rad == 4.1 * PI / 180.0 && !turn.beta_limit_inclusive &&
           turn.start_angle == SIDEREON_COSINE_TURN_START_ANGLE_COLINEARITY &&
           turn.start_angle_rad == 10.0 * PI / 180.0 && turn.period_s == 5656.0);
    const SidereonCosineTurnParameters cast[] = {
        sidereon_beidou3_cast_meo_turn(), sidereon_beidou3_cast_igso_turn(),
    };
    const double periods[] = {3090.0, 5740.0};
    for (size_t i = 0; i < 2; ++i) {
        assert(cast[i].beta_limit_rad == 3.0 * PI / 180.0 && cast[i].beta_limit_inclusive &&
               cast[i].start_angle == SIDEREON_COSINE_TURN_START_ANGLE_ORBIT_ANGLE &&
               cast[i].start_angle_rad == 6.0 * PI / 180.0 && cast[i].period_s == periods[i]);
    }

    double yaw_rate;
    assert(sidereon_gps_iia_default_yaw_rate_rad_s(1, &yaw_rate) == SIDEREON_STATUS_OK &&
           yaw_rate == 0.1211 * PI / 180.0);
    assert(sidereon_gps_iia_default_yaw_rate_rad_s(33, &yaw_rate) == SIDEREON_STATUS_INVALID_ARGUMENT);
    detail(SIDEREON_ENGINE_ERROR_FAMILY_GNSS_ATTITUDE, "no_default_yaw_rate", "{\"prn\":33}");
    legacy_message("sidereon_gps_iia_default_yaw_rate_rad_s: no default GPS IIA yaw rate for PRN Some(33)");

    SidereonGnssBlock block;
    assert(sidereon_gnss_block_from_igs_name((const uint8_t *)"X", 1, &block) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    detail(SIDEREON_ENGINE_ERROR_FAMILY_GNSS_BLOCK, "unknown", "{\"name\":\"X\"}");
    legacy_message("sidereon_gnss_block_from_igs_name: unknown GNSS block name \"X\"");
    assert(sidereon_gnss_block_from_igs_name((const uint8_t *)"BLOCK I", 7, &block) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    detail(SIDEREON_ENGINE_ERROR_FAMILY_GNSS_BLOCK, "unsupported", "{\"name\":\"BLOCK I\"}");
    assert(sidereon_gnss_block_from_constellation_block_type(
               SIDEREON_GNSS_SYSTEM_BEI_DOU, (const uint8_t *)"BDS-3", 5, NULL, &block) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    detail(SIDEREON_ENGINE_ERROR_FAMILY_GNSS_BLOCK, "beidou_orbit_class_required",
           "{\"block_type\":\"BDS-3\"}");
    const uint32_t meo = SIDEREON_BEIDOU_ORBIT_CLASS_MEO;
    assert(sidereon_gnss_block_from_constellation_block_type(
               SIDEREON_GNSS_SYSTEM_BEI_DOU, (const uint8_t *)"BDS-3", 5, &meo, &block) ==
           SIDEREON_STATUS_OK && block == SIDEREON_GNSS_BLOCK_BEIDOU3_MEO_UNKNOWN);
    SidereonGnssSystem system;
    assert(sidereon_gnss_block_system(block, &system) == SIDEREON_STATUS_OK &&
           system == SIDEREON_GNSS_SYSTEM_BEI_DOU);
    const uint32_t manufacturer = SIDEREON_BEIDOU_MANUFACTURER_CAST;
    assert(sidereon_gnss_block_beidou3_meo(&manufacturer, &block) == SIDEREON_STATUS_OK &&
           block == SIDEREON_GNSS_BLOCK_BEIDOU3_MEO_CAST);
    assert(sidereon_gnss_block_beidou3_meo(NULL, &block) == SIDEREON_STATUS_OK &&
           block == SIDEREON_GNSS_BLOCK_BEIDOU3_MEO_UNKNOWN);
    int32_t order;
    assert(sidereon_gnss_block_compare(SIDEREON_GNSS_BLOCK_GPS_IIA, SIDEREON_GNSS_BLOCK_GPS_IIF,
                                      &order) == SIDEREON_STATUS_OK && order == -1);
    assert(sidereon_beidou_manufacturer_compare(SIDEREON_BEIDOU_MANUFACTURER_SECM, manufacturer,
                                                &order) == SIDEREON_STATUS_OK && order == 1);
    assert(sidereon_beidou_orbit_class_compare(meo, meo, &order) == SIDEREON_STATUS_OK && order == 0);
}

static void yaw_geometry_errors(void) {
    SidereonAttitudeSample sample = {
        .position_m = {NAN, 0.0, 0.0},
        .velocity_m_s = {0.0, 1.0, 0.0},
    };
    SidereonNominalYaw yaw;
    assert(sidereon_nominal_yaw(&sample, &yaw) == SIDEREON_STATUS_INVALID_ARGUMENT);
    detail(SIDEREON_ENGINE_ERROR_FAMILY_GNSS_ATTITUDE, "non_finite", "{\"field\":\"position_m\"}");
    sample.position_m[0] = 26000000.0;
    assert(sidereon_nominal_yaw(&sample, &yaw) == SIDEREON_STATUS_INVALID_ARGUMENT);
    detail(SIDEREON_ENGINE_ERROR_FAMILY_GNSS_ATTITUDE, "zero_length", "{\"field\":\"sun_position_m\"}");
    sample.sun_position_m[2] = 150000000000.0;
    assert(sidereon_nominal_yaw(&sample, &yaw) == SIDEREON_STATUS_INVALID_ARGUMENT);
    detail(SIDEREON_ENGINE_ERROR_FAMILY_GNSS_ATTITUDE, "degenerate_geometry", "{\"t_s\":null}");
}

static void eclipse_values_and_errors(void) {
    const double sunlit[3] = {7000.0, 0.0, 0.0};
    const double umbra[3] = {-7000.0, 0.0, 0.0};
    const double sun[3] = {149597870.7, 0.0, 0.0};
    const double moon[3] = {-384400.0, 0.0, 0.0};
    double fraction;
    assert(sidereon_eclipse_moon_shadow_fraction(sunlit, sun, moon, &fraction) ==
           SIDEREON_STATUS_OK && fraction == 0.0);
    assert(sidereon_eclipse_shadow_fraction_with_moon(sunlit, sun, moon, &fraction) ==
           SIDEREON_STATUS_OK && fraction == 0.0);
    assert(sidereon_eclipse_shadow_fraction_with_model_and_moon(
               umbra, sun, moon, SIDEREON_EARTH_SHADOW_MODEL_WGS84_OBLATE, &fraction) ==
           SIDEREON_STATUS_OK && fraction == 1.0);
    SidereonEclipseStatus status;
    assert(sidereon_eclipse_status_with_model_and_moon(
               umbra, sun, moon, SIDEREON_EARTH_SHADOW_MODEL_WGS84_OBLATE, &status) ==
           SIDEREON_STATUS_OK && status == SIDEREON_ECLIPSE_STATUS_UMBRA);
    SidereonShadowGradient gradient;
    assert(sidereon_eclipse_shadow_fraction_with_model_moon_and_gradient(
               umbra, sun, moon, SIDEREON_EARTH_SHADOW_MODEL_WGS84_OBLATE, &gradient) ==
           SIDEREON_STATUS_OK);
    assert(gradient.fraction == 1.0 && gradient.gradient_per_km[0] == 0.0 &&
           gradient.gradient_per_km[1] == 0.0 && gradient.gradient_per_km[2] == 0.0);
    const double antumbra[3] = {-2e6, 0.0, 0.0};
    assert(sidereon_eclipse_shadow_fraction_with_gradient(antumbra, sun, &gradient) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    detail(SIDEREON_ENGINE_ERROR_FAMILY_SHADOW_GRADIENT, "numerical_failure", "{}");
    legacy_message("sidereon_eclipse_shadow_fraction_with_gradient: shadow fraction gradient is not finite");
    const double zero[3] = {0.0, 0.0, 0.0};
    assert(sidereon_eclipse_shadow_fraction_with_gradient(sunlit, zero, &gradient) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    detail(SIDEREON_ENGINE_ERROR_FAMILY_SHADOW_GRADIENT, "eclipse",
           "{\"cause\":{\"kind\":\"invalid_input\",\"fields\":{\"field\":\"sun_pos\",\"reason\":\"zero vector\"}}}");
}

static void exact(double value, uint64_t bits) {
    uint64_t actual;
    memcpy(&actual, &value, sizeof(actual));
    if (actual != bits) {
        fprintf(stderr, "actual %016llx expected %016llx\n",
                (unsigned long long)actual, (unsigned long long)bits);
        abort();
    }
}

static double from_bits(uint64_t bits) {
    double value;
    memcpy(&value, &bits, sizeof(value));
    return value;
}

static void vector(const double *values, const uint64_t *pins, size_t count) {
    for (size_t i = 0; i < count; ++i) {
        exact(values[i], pins[i]);
    }
}

static void circular_elements(SidereonCircularElements elements, const uint64_t *pins) {
    const double values[] = {
        elements.a_km, elements.ex, elements.ey, elements.i_rad,
        elements.raan_rad, elements.mean_latitude_argument_rad,
    };
    vector(values, pins, 6);
}

static void keplerian_elements(SidereonAnalyticKeplerianElements elements, const uint64_t *pins) {
    const double values[] = {
        elements.a_km, elements.e, elements.i_rad, elements.raan_rad,
        elements.arg_perigee_rad, elements.mean_anomaly_rad,
    };
    vector(values, pins, 6);
}

static void zonal_gravity(SidereonZonalHarmonics gravity) {
    assert(gravity.mu_km3_s2 == 398600.4418 && gravity.re_km == 6378.137 &&
           gravity.c20 == -1.08262668e-3 && gravity.c30 == 2.5326564853322355e-6 &&
           gravity.c40 == 1.619621591367e-6 && gravity.c50 == 2.272960828686982e-7 &&
           gravity.c60 == -5.406812391070848e-7);
}

static void analytic_values(void) {
    SidereonZonalHarmonics gravity = sidereon_zonal_harmonics_default();
    zonal_gravity(gravity);
    SidereonMeanConversionOptions eh_options = sidereon_mean_conversion_options_eckstein_hechler_default();
    SidereonMeanConversionOptions bl_options = sidereon_mean_conversion_options_brouwer_lyddane_default();
    assert(eh_options.tolerance == 1e-11 && eh_options.max_iterations == 100);
    assert(bl_options.tolerance == 1e-13 && bl_options.max_iterations == 200);

    const SidereonAnalyticCartesian state = {
        .position_km = {7000.0, 20.0, 30.0},
        .velocity_km_s = {0.0, 7.5, 1.0},
    };
    const SidereonCartesianState initial = {
        .epoch_s = 646315200.25,
        .position_km = {7000.0, 20.0, 30.0},
        .velocity_km_s = {0.0, 7.5, 1.0},
    };
    SidereonCircularElements circular;
    assert(sidereon_circular_elements_from_cartesian(state, 398600.4418, &circular) ==
           SIDEREON_STATUS_OK);
    circular_elements(circular, CONTRACT_CIRCULAR);
    SidereonEcksteinHechlerMeanElements eh_mean;
    assert(sidereon_eckstein_hechler_mean_elements(state, gravity, eh_options, &eh_mean) ==
           SIDEREON_STATUS_OK);
    circular_elements(eh_mean.elements, CONTRACT_EH_MEAN);
    exact((double)eh_mean.iterations, CONTRACT_EH_MEAN[6]);
    assert(sidereon_eckstein_hechler_osculating_elements(eh_mean.elements, gravity, &circular) ==
           SIDEREON_STATUS_OK);
    circular_elements(circular, CONTRACT_EH_OSCULATING);
    SidereonBrouwerLyddaneMeanElements bl_mean;
    assert(sidereon_brouwer_lyddane_mean_elements(state, gravity, bl_options, &bl_mean) ==
           SIDEREON_STATUS_OK);
    keplerian_elements(bl_mean.elements, CONTRACT_BL_MEAN);
    exact((double)bl_mean.iterations, CONTRACT_BL_MEAN[6]);
    SidereonAnalyticKeplerianElements keplerian;
    assert(sidereon_brouwer_lyddane_osculating_elements(bl_mean.elements, gravity, &keplerian) ==
           SIDEREON_STATUS_OK);
    keplerian_elements(keplerian, CONTRACT_BL_OSCULATING);

    SidereonEcksteinHechlerPropagator *eh = NULL;
    SidereonBrouwerLyddanePropagator *bl = NULL;
    assert(sidereon_eckstein_hechler_propagator_new(initial, gravity, eh_options, &eh) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_brouwer_lyddane_propagator_new(initial, gravity, 0.0, bl_options, &bl) ==
           SIDEREON_STATUS_OK);
    double epoch;
    assert(sidereon_eckstein_hechler_propagator_epoch_tdb_seconds(eh, &epoch) ==
           SIDEREON_STATUS_OK && epoch == 646315200.25);
    assert(sidereon_brouwer_lyddane_propagator_epoch_tdb_seconds(bl, &epoch) ==
           SIDEREON_STATUS_OK && epoch == 646315200.25);
    double m2;
    assert(sidereon_brouwer_lyddane_propagator_m2_rad_s2(bl, &m2) ==
           SIDEREON_STATUS_OK && m2 == 0.0);
    SidereonZonalHarmonics result_gravity;
    assert(sidereon_eckstein_hechler_propagator_gravity(eh, &result_gravity) == SIDEREON_STATUS_OK);
    zonal_gravity(result_gravity);
    assert(sidereon_brouwer_lyddane_propagator_gravity(bl, &result_gravity) == SIDEREON_STATUS_OK);
    zonal_gravity(result_gravity);
    assert(sidereon_eckstein_hechler_propagator_mean_elements(eh, &circular) == SIDEREON_STATUS_OK);
    circular_elements(circular, CONTRACT_EH_MEAN);
    assert(sidereon_brouwer_lyddane_propagator_mean_elements(bl, &keplerian) == SIDEREON_STATUS_OK);
    keplerian_elements(keplerian, CONTRACT_BL_MEAN);

    const double epochs[2] = {646315320.25, 646315140.25};
    assert(sidereon_eckstein_hechler_propagator_osculating_elements(eh, epochs[0], &circular) ==
           SIDEREON_STATUS_OK);
    circular_elements(circular, CONTRACT_EH_AT_EPOCH);
    assert(sidereon_brouwer_lyddane_propagator_osculating_elements(bl, epochs[0], &keplerian) ==
           SIDEREON_STATUS_OK);
    keplerian_elements(keplerian, CONTRACT_BL_AT_EPOCH);
    size_t written, required;
    SidereonCartesianState eh_states[2], bl_states[2];
    assert(sidereon_eckstein_hechler_propagator_ephemeris(eh, epochs, 2, eh_states, 2,
                                                         &written, &required) == SIDEREON_STATUS_OK);
    assert(written == 2 && required == 2);
    assert(sidereon_brouwer_lyddane_propagator_ephemeris(bl, epochs, 2, bl_states, 2,
                                                        &written, &required) == SIDEREON_STATUS_OK);
    assert(written == 2 && required == 2);
    for (size_t i = 0; i < 2; ++i) {
        exact(eh_states[i].epoch_s, CONTRACT_EH_EPHEMERIS[7*i]);
        vector(eh_states[i].position_km, CONTRACT_EH_EPHEMERIS + 7*i + 1, 3);
        vector(eh_states[i].velocity_km_s, CONTRACT_EH_EPHEMERIS + 7*i + 4, 3);
        exact(bl_states[i].epoch_s, CONTRACT_BL_EPHEMERIS[7*i]);
        vector(bl_states[i].position_km, CONTRACT_BL_EPHEMERIS + 7*i + 1, 3);
        vector(bl_states[i].velocity_km_s, CONTRACT_BL_EPHEMERIS + 7*i + 4, 3);
    }
    sidereon_eckstein_hechler_propagator_free(eh);
    sidereon_brouwer_lyddane_propagator_free(bl);
}

static void analytic_errors(void) {
    const SidereonEngineErrorFamily family = SIDEREON_ENGINE_ERROR_FAMILY_ANALYTIC_PROPAGATION;
    SidereonZonalHarmonics gravity = sidereon_zonal_harmonics_earth();
    const SidereonCircularElements valid = {7200.0, 0.001, 0.002, 0.5, 0.25, 0.125};
    SidereonCircularElements elements = valid;
    SidereonEcksteinHechlerPropagator *eh = NULL;
    assert(sidereon_eckstein_hechler_propagator_from_mean_elements(NAN, valid, gravity, &eh) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    detail(family, "non_finite", "{\"field\":\"epoch_tdb_seconds\"}");
    legacy_message("sidereon_eckstein_hechler_propagator_from_mean_elements: non-finite input epoch_tdb_seconds");
    gravity.mu_km3_s2 = 0.0;
    assert(sidereon_eckstein_hechler_propagator_from_mean_elements(0.0, valid, gravity, &eh) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    detail(family, "invalid_parameter", "{\"field\":\"mu_km3_s2\",\"reason\":\"must be positive\"}");
    gravity.mu_km3_s2 = 398600.4418;
    elements.a_km = -1.0;
    assert(sidereon_eckstein_hechler_propagator_from_mean_elements(0.0, elements, gravity, &eh) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    detail(family, "not_elliptic", "{\"field\":\"mean elements\"}");
    elements.a_km = 6000.0;
    assert(sidereon_eckstein_hechler_propagator_from_mean_elements(0.0, elements, gravity, &eh) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    detail(family, "inside_reference_sphere",
           "{\"semi_major_axis_km\":" F64("6000", "40b7700000000000")
           ",\"reference_radius_km\":" F64("6378.137", "40b8ea23126e978d") "}");
    elements = valid;
    elements.i_rad = 0.0;
    assert(sidereon_eckstein_hechler_propagator_from_mean_elements(0.0, elements, gravity, &eh) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    detail(family, "nearly_equatorial", "{\"inclination_rad\":" F64("0", "0000000000000000") "}");
    elements.i_rad = 1.107;
    assert(sidereon_eckstein_hechler_propagator_from_mean_elements(0.0, elements, gravity, &eh) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    detail(family, "near_critical_inclination",
           "{\"inclination_rad\":" F64("1.107", "3ff1b645a1cac083") "}");
    elements = valid;
    elements.ex = 0.2;
    elements.ey = 0.0;
    assert(sidereon_eckstein_hechler_propagator_from_mean_elements(0.0, elements, gravity, &eh) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    detail(family, "eccentricity_too_large",
           "{\"eccentricity\":" F64("0.2", "3fc999999999999a")
           ",\"limit\":" F64("0.1", "3fb999999999999a") "}");

    SidereonAnalyticCartesian retrograde = {
        .position_km = {7000.0, 0.0, 0.0}, .velocity_km_s = {0.0, -7.5, 0.0},
    };
    SidereonBrouwerLyddaneMeanElements retrograde_mean;
    assert(sidereon_brouwer_lyddane_mean_elements(
               retrograde, gravity, sidereon_mean_conversion_options_brouwer_lyddane_default(),
               &retrograde_mean) == SIDEREON_STATUS_INVALID_ARGUMENT);
    detail(family, "retrograde_equatorial",
           "{\"inclination_rad\":" F64("3.141592653589793", "400921fb54442d18") "}");

    const SidereonAnalyticCartesian initial = {
        .position_km = {7000.0, 20.0, 30.0}, .velocity_km_s = {0.0, 7.5, 1.0},
    };
    SidereonEcksteinHechlerMeanElements mean;
    assert(sidereon_eckstein_hechler_mean_elements(initial, gravity,
               (SidereonMeanConversionOptions){1e-11, 1}, &mean) == SIDEREON_STATUS_INVALID_ARGUMENT);
    detail(family, "mean_elements_not_converged", "{\"theory\":\"Eckstein-Hechler\",\"iterations\":1}");

    assert(sidereon_eckstein_hechler_propagator_from_mean_elements(-DBL_MAX, valid, gravity, &eh) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_eckstein_hechler_propagator_osculating_elements(eh, DBL_MAX, &elements) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    detail(family, "non_finite_result", "{\"theory\":\"Eckstein-Hechler\",\"field\":\"a_km\"}");
    const SidereonAnalyticKeplerianElements extreme = {
        .a_km = 7200.0, .e = 0.99999999999272404, .i_rad = 0.9,
        .mean_anomaly_rad = 0.00063684089789871291,
    };
    SidereonAnalyticCartesian converted;
    assert(sidereon_analytic_keplerian_elements_to_cartesian(extreme, 398600.4418, &converted) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    detail(family, "kepler_not_converged", "{\"field\":\"eccentric anomaly\"}");
    sidereon_eckstein_hechler_propagator_free(eh);
}

static void de_errors(void) {
    SidereonDeEphemeris *de = NULL;
    assert(sidereon_de_ephemeris_load("", &de) == SIDEREON_STATUS_SOLVE);
    detail(SIDEREON_ENGINE_ERROR_FAMILY_DE, "io",
           "{\"message\":\"No such file or directory (os error 2)\",\"raw_os_error\":2}");
    const uint8_t bytes[] = {0};
    assert(sidereon_de_ephemeris_from_bytes(bytes, sizeof(bytes), &de) == SIDEREON_STATUS_SOLVE);
    detail(SIDEREON_ENGINE_ERROR_FAMILY_DE, "invalid_file",
           "{\"reason\":\"incomplete first header record\"}");
    legacy_message("sidereon_de_ephemeris_from_bytes: Invalid DE ephemeris: incomplete first header record");
    assert(de == NULL);
}

static uint8_t *read_bytes(const char *path, size_t *length) {
    FILE *file = fopen(path, "rb");
    assert(file && fseek(file, 0, SEEK_END) == 0);
    long size = ftell(file);
    assert(size > 0 && fseek(file, 0, SEEK_SET) == 0);
    uint8_t *bytes = malloc((size_t)size);
    assert(bytes && fread(bytes, 1, (size_t)size, file) == (size_t)size);
    assert(fclose(file) == 0);
    *length = (size_t)size;
    return bytes;
}

static void de_values(const char *path) {
    size_t length;
    uint8_t *bytes = read_bytes(path, &length);
    SidereonDeEphemeris *de = NULL;
    assert(sidereon_de_ephemeris_from_bytes(bytes, length, &de) == SIDEREON_STATUS_OK);
    size_t record_size, count;
    assert(sidereon_de_ephemeris_record_size(de, &record_size) == SIDEREON_STATUS_OK &&
           record_size == CONTRACT_DE_RECORD_SIZE);
    assert(sidereon_de_ephemeris_constant_count(de, &count) == SIDEREON_STATUS_OK &&
           count == CONTRACT_DE_CONSTANT_COUNT);
    double value;
    assert(sidereon_de_ephemeris_astronomical_unit_m(de, &value) == SIDEREON_STATUS_OK);
    exact(value, CONTRACT_DE_METADATA[0]);
    assert(sidereon_de_ephemeris_earth_moon_mass_ratio(de, &value) == SIDEREON_STATUS_OK);
    exact(value, CONTRACT_DE_METADATA[1]);
    SidereonDeGm gm;
    assert(sidereon_de_ephemeris_gm_m3_s2(de, SIDEREON_DE_BODY_EARTH, &gm) ==
           SIDEREON_STATUS_OK && gm.present);
    exact(gm.value_m3_s2, CONTRACT_DE_METADATA[2]);
    uint8_t name[32];
    size_t written, required;
    assert(sidereon_de_ephemeris_constant(de, 0, name, sizeof(name), &written, &required, &value) ==
           SIDEREON_STATUS_OK);
    assert(written == sizeof(CONTRACT_DE_FIRST_CONSTANT) - 1 && required == written &&
           memcmp(name, CONTRACT_DE_FIRST_CONSTANT, written) == 0);
    exact(value, CONTRACT_DE_METADATA[3]);

    const double epoch = from_bits(CONTRACT_DE_STATES[0]);
    SidereonDeSunMoon states;
    assert(sidereon_de_ephemeris_sun_moon_at_tdb_seconds(de, epoch, &states) == SIDEREON_STATUS_OK);
    vector(states.sun.position_m, CONTRACT_DE_STATES + 1, 3);
    vector(states.sun.velocity_m_s, CONTRACT_DE_STATES + 4, 3);
    vector(states.moon.position_m, CONTRACT_DE_STATES + 7, 3);
    vector(states.moon.velocity_m_s, CONTRACT_DE_STATES + 10, 3);
    SidereonSunMoon positions;
    assert(sidereon_de_ephemeris_sun_moon_positions_at_tdb_seconds(de, epoch, &positions) ==
           SIDEREON_STATUS_OK);
    vector(positions.sun_m, CONTRACT_DE_STATES + 1, 3);
    vector(positions.moon_m, CONTRACT_DE_STATES + 7, 3);

    assert(sidereon_de_ephemeris_sun_moon_at_tdb_seconds(de, NAN, &states) == SIDEREON_STATUS_SOLVE);
    detail(SIDEREON_ENGINE_ERROR_FAMILY_DE, "non_finite_epoch", "{}");
    assert(sidereon_de_ephemeris_sun_moon_at_tdb_seconds(de, -1e9, &states) == SIDEREON_STATUS_SOLVE);
    detail(SIDEREON_ENGINE_ERROR_FAMILY_DE, "outside_coverage",
           "{\"epoch_tdb_seconds\":" F64("-1000000000", "c1cdcd6500000000") "}");
    sidereon_de_ephemeris_free(de);

    assert(sidereon_de_ephemeris_from_bytes(bytes, length, &de) == SIDEREON_STATUS_OK);
    SidereonStatePropagationConfig config;
    assert(sidereon_state_propagation_config_init(&config) == SIDEREON_STATUS_OK);
    config.force_model = SIDEREON_PROPAGATION_FORCE_MODEL_COMPOSITE;
    config.epoch_s = 646315800.25;
    config.position_km[0] = 7000.0;
    config.position_km[1] = 20.0;
    config.position_km[2] = 30.0;
    config.velocity_km_s[1] = 7.5;
    config.velocity_km_s[2] = 1.0;
    const SidereonPropagationContext context = {.de_ephemeris = de};
    const SidereonEarthRadiationPressureOptions earth = {.cr = 1.25, .area_to_mass_m2_kg = 0.02};
    const SidereonStatePropagationEnvironment environment = {
        .context = &context, .earth_radiation_pressure = &earth,
    };
    const double target = 646315860.25;
    SidereonEphemeris *arc = NULL;
    assert(sidereon_propagate_state_with_force_model(&config, &target, 1, &environment, NULL, 0, &arc) ==
           SIDEREON_STATUS_OK);
    SidereonCartesianState state;
    assert(sidereon_ephemeris_states(arc, &state, 1, &written, &required) == SIDEREON_STATUS_OK);
    assert(written == 1 && required == 1);
    exact(state.epoch_s, CONTRACT_DE_PROPAGATION[0]);
    vector(state.position_km, CONTRACT_DE_PROPAGATION + 1, 3);
    vector(state.velocity_km_s, CONTRACT_DE_PROPAGATION + 4, 3);
    sidereon_ephemeris_free(arc);
    sidereon_de_ephemeris_free(de);

    // First Sun coefficient in the first data record of the little-endian DE440 file.
    const size_t offset = 2 * record_size + (753 - 1) * sizeof(double);
    const uint8_t nan_le[8] = {0, 0, 0, 0, 0, 0, 0xf8, 0x7f};
    assert(offset + sizeof(nan_le) <= length);
    memcpy(bytes + offset, nan_le, sizeof(nan_le));
    assert(sidereon_de_ephemeris_from_bytes(bytes, length, &de) == SIDEREON_STATUS_OK);
    assert(sidereon_de_ephemeris_sun_moon_at_tdb_seconds(de, epoch, &states) == SIDEREON_STATUS_SOLVE);
    detail(SIDEREON_ENGINE_ERROR_FAMILY_DE, "non_finite_state", "{}");
    sidereon_de_ephemeris_free(de);
    free(bytes);
}

typedef SidereonStatus (*IndexedName)(size_t, uint8_t *, size_t, size_t *, size_t *);

static void indexed_names(IndexedName function, const char *const *expected, size_t count) {
    for (size_t i = 0; i < count; ++i) {
        uint8_t text[32];
        size_t written, required;
        assert(function(i, text, sizeof(text), &written, &required) == SIDEREON_STATUS_OK);
        assert(written == strlen(expected[i]) && required == written &&
               memcmp(text, expected[i], written) == 0);
    }
}

static void ecom_names(void) {
    const char *const ecom1[] = {"D0", "Dc", "Ds", "Y0", "Yc", "Ys", "B0", "Bc", "Bs"};
    const char *const ecom1_parameters[] = {
        "ecom1.d0", "ecom1.dc", "ecom1.ds", "ecom1.y0", "ecom1.yc", "ecom1.ys",
        "ecom1.b0", "ecom1.bc", "ecom1.bs",
    };
    const char *const d2b1[] = {"B0", "B1c", "B1s", "D0", "D2c", "D2s", "Y0"};
    const char *const d2b1_parameters[] = {
        "ecom2.b0", "ecom2.b1c", "ecom2.b1s", "ecom2.d0", "ecom2.d2c", "ecom2.d2s", "ecom2.y0",
    };
    const char *const d4b1[] = {"B0", "B1c", "B1s", "D0", "D2c", "D4c", "D2s", "D4s", "Y0"};
    const char *const d4b1_parameters[] = {
        "ecom2.b0", "ecom2.b1c", "ecom2.b1s", "ecom2.d0", "ecom2.d2c", "ecom2.d4c",
        "ecom2.d2s", "ecom2.d4s", "ecom2.y0",
    };
    indexed_names(sidereon_ecom1_coefficients_names, ecom1, 9);
    indexed_names(sidereon_ecom1_coefficients_parameter_names, ecom1_parameters, 9);
    indexed_names(sidereon_ecom2_d2b1_coefficients_names, d2b1, 7);
    indexed_names(sidereon_ecom2_d2b1_coefficients_parameter_names, d2b1_parameters, 7);
    indexed_names(sidereon_ecom2_d4b1_coefficients_names, d4b1, 9);
    indexed_names(sidereon_ecom2_d4b1_coefficients_parameter_names, d4b1_parameters, 9);
    for (size_t i = 0; i < 9; ++i) {
        uint8_t text[32];
        size_t written, required;
        assert(sidereon_ecom2_solar_radiation_pressure_parameter_name(
                   2, 1, i, text, sizeof(text), &written, &required) == SIDEREON_STATUS_OK);
        assert(written == strlen(d4b1_parameters[i]) && required == written &&
               memcmp(text, d4b1_parameters[i], written) == 0);
    }
}

typedef struct {
    SidereonStatus (*function)(const SidereonForceModel *, SidereonShadowFunction *);
    SidereonStatus (*earth)(const SidereonForceModel *, SidereonEarthShadowModel *);
    SidereonStatus (*moon)(const SidereonForceModel *, bool *);
    SidereonStatus (*with_function)(const SidereonForceModel *, uint32_t, SidereonForceModel **);
    SidereonStatus (*with_earth)(const SidereonForceModel *, uint32_t, SidereonForceModel **);
    SidereonStatus (*with_moon)(const SidereonForceModel *, bool, SidereonForceModel **);
} ShadowAccess;

#define SHADOW_ACCESS(prefix) {prefix##_shadow_function, prefix##_earth_shadow_model, \
    prefix##_moon_eclipse_enabled, prefix##_with_shadow_function, \
    prefix##_with_earth_shadow_model, prefix##_with_moon_eclipse}

static SidereonForceModel *shadow_settings(const SidereonForceModel *model, ShadowAccess api) {
    SidereonShadowFunction function;
    SidereonEarthShadowModel earth;
    bool moon;
    assert(api.function(model, &function) == SIDEREON_STATUS_OK &&
           function == SIDEREON_SHADOW_FUNCTION_APPARENT_DISC);
    assert(api.earth(model, &earth) == SIDEREON_STATUS_OK &&
           earth == SIDEREON_EARTH_SHADOW_MODEL_WGS84_OBLATE);
    assert(api.moon(model, &moon) == SIDEREON_STATUS_OK && moon);

    SidereonForceModel *changed = NULL;
    assert(api.with_function(model, SIDEREON_SHADOW_FUNCTION_CONICAL, &changed) ==
           SIDEREON_STATUS_OK);
    assert(api.function(changed, &function) == SIDEREON_STATUS_OK &&
           function == SIDEREON_SHADOW_FUNCTION_CONICAL);
    sidereon_force_model_free(changed);
    assert(api.with_earth(model, SIDEREON_EARTH_SHADOW_MODEL_SPHERICAL, &changed) ==
           SIDEREON_STATUS_OK);
    assert(api.earth(changed, &earth) == SIDEREON_STATUS_OK &&
           earth == SIDEREON_EARTH_SHADOW_MODEL_SPHERICAL);
    sidereon_force_model_free(changed);
    assert(api.with_moon(model, false, &changed) == SIDEREON_STATUS_OK);
    assert(api.moon(changed, &moon) == SIDEREON_STATUS_OK && !moon);
    return changed;
}

static void ecom1_coefficients(const SidereonEcom1Coefficients *c, const double *v) {
    const double actual[] = {c->d0, c->dc, c->ds, c->y0, c->yc, c->ys, c->b0, c->bc, c->bs};
    assert(memcmp(actual, v, sizeof(actual)) == 0);
}

static void force_models(void) {
    const double coefficients[] = {1e-8, 2e-8, 3e-8, 4e-8, 5e-8, 6e-8, 7e-8, 8e-8, 9e-8};
    SidereonEcom1Coefficients ecom1_coefficients_value;
    SidereonEcom2D2b1Coefficients d2b1;
    SidereonEcom2D4b1Coefficients d4b1;
    assert(sidereon_ecom1_coefficients_from_array(coefficients, &ecom1_coefficients_value) ==
           SIDEREON_STATUS_OK);
    ecom1_coefficients(&ecom1_coefficients_value, coefficients);
    assert(sidereon_ecom2_d2b1_coefficients_from_array(coefficients, &d2b1) == SIDEREON_STATUS_OK);
    assert(sidereon_ecom2_d4b1_coefficients_from_array(coefficients, &d4b1) == SIDEREON_STATUS_OK);
    const double d4b1_values[] = {
        d4b1.b0, d4b1.b1c, d4b1.b1s, d4b1.d0, d4b1.d2c, d4b1.d4c, d4b1.d2s, d4b1.d4s, d4b1.y0,
    };
    assert(memcmp(d4b1_values, coefficients, sizeof(coefficients)) == 0);
    const double zeros[9] = {0};
    double zero_values[9];
    const SidereonEcom1Coefficients zero_ecom1 = {0};
    const SidereonEcom2D2b1Coefficients zero_d2b1 = {0};
    const SidereonEcom2D4b1Coefficients zero_d4b1 = {0};
    assert(sidereon_ecom1_coefficients_to_array(zero_ecom1, zero_values) ==
           SIDEREON_STATUS_OK && memcmp(zero_values, zeros, sizeof(zeros)) == 0);
    assert(sidereon_ecom2_d2b1_coefficients_to_array(zero_d2b1, zero_values) ==
           SIDEREON_STATUS_OK && memcmp(zero_values, zeros, 7 * sizeof(double)) == 0);
    assert(sidereon_ecom2_d4b1_coefficients_to_array(zero_d4b1, zero_values) ==
           SIDEREON_STATUS_OK && memcmp(zero_values, zeros, sizeof(zeros)) == 0);

    double copied[9];
    assert(sidereon_ecom1_coefficients_to_array(ecom1_coefficients_value, copied) ==
           SIDEREON_STATUS_OK && memcmp(copied, coefficients, sizeof(copied)) == 0);
    assert(sidereon_ecom2_d2b1_coefficients_to_array(d2b1, copied) == SIDEREON_STATUS_OK &&
           memcmp(copied, coefficients, 7 * sizeof(double)) == 0);
    assert(sidereon_ecom2_d4b1_coefficients_to_array(d4b1, copied) == SIDEREON_STATUS_OK &&
           memcmp(copied, coefficients, sizeof(copied)) == 0);

    SidereonRadiationShadowOptions shadow = sidereon_radiation_shadow_options_default();
    SidereonForceModel *ecom1 = NULL, *ecom2 = NULL, *general = NULL, *solar = NULL, *earth = NULL;
    assert(sidereon_ecom1_solar_radiation_pressure_new(ecom1_coefficients_value, shadow, &ecom1) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_ecom2_solar_radiation_pressure_d4b1(d4b1, shadow, &ecom2) == SIDEREON_STATUS_OK);
    assert(sidereon_ecom2_solar_radiation_pressure_new(2, 1, coefficients, 9, shadow, &general) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_ecom1_solar_radiation_pressure_coefficients(ecom1, &ecom1_coefficients_value) ==
           SIDEREON_STATUS_OK);
    ecom1_coefficients(&ecom1_coefficients_value, coefficients);
    size_t written, required;
    assert(sidereon_ecom2_solar_radiation_pressure_coefficients(ecom2, copied, 9, &written,
                                                              &required) == SIDEREON_STATUS_OK);
    assert(written == 9 && required == 9 && memcmp(copied, coefficients, sizeof(copied)) == 0);
    bool equal;
    assert(sidereon_ecom2_solar_radiation_pressure_equal(ecom2, general, &equal) ==
           SIDEREON_STATUS_OK && equal);
    sidereon_force_model_free(general);
    assert(sidereon_solar_radiation_pressure_new(1.25, 0.02, shadow, &solar) == SIDEREON_STATUS_OK);
    assert(sidereon_earth_radiation_pressure_new(1.25, 0.02, true, &earth) == SIDEREON_STATUS_OK);

    SidereonForceModel *ecom1_sun = shadow_settings(ecom1,
        (ShadowAccess)SHADOW_ACCESS(sidereon_ecom1_solar_radiation_pressure));
    SidereonForceModel *ecom2_sun = shadow_settings(ecom2,
        (ShadowAccess)SHADOW_ACCESS(sidereon_ecom2_solar_radiation_pressure));
    SidereonForceModel *solar_sun = shadow_settings(solar,
        (ShadowAccess)SHADOW_ACCESS(sidereon_solar_radiation_pressure));
    bool moon_enabled;
    assert(sidereon_earth_radiation_pressure_moon_eclipse_enabled(earth, &moon_enabled) ==
           SIDEREON_STATUS_OK && moon_enabled);
    SidereonForceModel *earth_sun = NULL;
    assert(sidereon_earth_radiation_pressure_with_moon_eclipse(earth, false, &earth_sun) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_earth_radiation_pressure_moon_eclipse_enabled(earth_sun, &moon_enabled) ==
           SIDEREON_STATUS_OK && !moon_enabled);
    assert(sidereon_ecom1_solar_radiation_pressure_equal(ecom1, ecom1, &equal) ==
           SIDEREON_STATUS_OK && equal);
    assert(sidereon_ecom1_solar_radiation_pressure_equal(ecom1, ecom1_sun, &equal) ==
           SIDEREON_STATUS_OK && !equal);
    double value;
    assert(sidereon_solar_radiation_pressure_cr(solar, &value) == SIDEREON_STATUS_OK &&
           value == 1.25);
    assert(sidereon_solar_radiation_pressure_area_to_mass_m2_kg(solar, &value) ==
           SIDEREON_STATUS_OK && value == 0.02);
    assert(sidereon_solar_radiation_pressure_au_km(solar, &value) == SIDEREON_STATUS_OK &&
           value == 149597870.7);
    uint8_t name[6];
    assert(sidereon_solar_radiation_pressure_cr_parameter(name, sizeof(name), &written, &required) ==
           SIDEREON_STATUS_OK);
    assert(written == 6 && required == 6 && memcmp(name, "srp.cr", 6) == 0);

    const SidereonVector3 position = {{7000.0, 20.0, 30.0}};
    const SidereonVector3 velocity = {{0.0, 7.5, 1.0}};
    const SidereonVector3 sun = {{150000000.0, 20000000.0, -10000000.0}};
    const SidereonVector3 moon = {{-100000.0, 200000.0, 300000.0}};
    const SidereonCartesianState state = {646315200.25, {7000.0, 20.0, 30.0}, {0.0, 7.5, 1.0}};
    SidereonVector3 acceleration;
    assert(sidereon_solar_radiation_pressure_acceleration_with_sun_moon(
               solar, position, sun, moon, &acceleration) == SIDEREON_STATUS_OK);
    vector(acceleration.xyz, CONTRACT_SOLAR, 3);
    assert(sidereon_earth_radiation_pressure_acceleration_with_sun_moon(
               earth_sun, state, sun, moon, &acceleration) == SIDEREON_STATUS_OK);
    vector(acceleration.xyz, CONTRACT_EARTH, 3);

    SidereonEcomGeometry geometry;
    assert(sidereon_ecom_geometry_with_shadow(position, velocity, sun, &moon,
               SIDEREON_EARTH_SHADOW_MODEL_SPHERICAL, &geometry) == SIDEREON_STATUS_OK);
    vector(geometry.dyb.d, CONTRACT_ECOM_GEOMETRY, 3);
    vector(geometry.dyb.y, CONTRACT_ECOM_GEOMETRY + 3, 3);
    vector(geometry.dyb.b, CONTRACT_ECOM_GEOMETRY + 6, 3);
    assert(geometry.delta_u_rad.present && geometry.argument_of_latitude_rad.present);
    exact(geometry.delta_u_rad.value, CONTRACT_ECOM_GEOMETRY[9]);
    exact(geometry.argument_of_latitude_rad.value, CONTRACT_ECOM_GEOMETRY[10]);
    exact(geometry.lighting_ratio, CONTRACT_ECOM_GEOMETRY[11]);

#define CHECK_ECOM(prefix, model, model_sun, pin) do { \
    SidereonAccelerationPartials partials; \
    SidereonVector3 parameters[9]; \
    assert(prefix##_acceleration_with_sun_moon(model, position, velocity, sun, moon, \
                                                &acceleration) == SIDEREON_STATUS_OK); \
    vector(acceleration.xyz, pin##_SUN_MOON, 3); \
    assert(prefix##_acceleration_with_sun(model_sun, position, velocity, sun, &acceleration) == \
           SIDEREON_STATUS_OK); \
    vector(acceleration.xyz, pin##_SUN, 3); \
    assert(prefix##_acceleration_partials_with_sun_moon(model, position, velocity, sun, moon, \
                                                         &partials) == SIDEREON_STATUS_OK); \
    vector(partials.wrt_position, pin##_STATE_SUN_MOON, 9); \
    vector(partials.wrt_velocity, pin##_STATE_SUN_MOON + 9, 9); \
    assert(prefix##_acceleration_partials_with_sun(model_sun, position, velocity, sun, \
                                                    &partials) == SIDEREON_STATUS_OK); \
    vector(partials.wrt_position, pin##_STATE_SUN, 9); \
    vector(partials.wrt_velocity, pin##_STATE_SUN + 9, 9); \
    assert(prefix##_parameter_partials_with_sun_moon(model, position, velocity, sun, moon, \
               parameters, 9, &written, &required) == SIDEREON_STATUS_OK); \
    assert(written == 9 && required == 9); \
    for (size_t i = 0; i < 9; ++i) { \
        vector(parameters[i].xyz, pin##_PARAMETER_SUN_MOON + 3*i, 3); \
    } \
    assert(prefix##_parameter_partials_with_sun(model_sun, position, velocity, sun, \
               parameters, 9, &written, &required) == SIDEREON_STATUS_OK); \
    assert(written == 9 && required == 9); \
    for (size_t i = 0; i < 9; ++i) { \
        vector(parameters[i].xyz, pin##_PARAMETER_SUN + 3*i, 3); \
    } \
} while (0)

    CHECK_ECOM(sidereon_ecom1_solar_radiation_pressure, ecom1, ecom1_sun, CONTRACT_ECOM1);
    CHECK_ECOM(sidereon_ecom2_solar_radiation_pressure, ecom2, ecom2_sun, CONTRACT_ECOM2);
#undef CHECK_ECOM
    SidereonForceModel *models[] = {ecom1, ecom2, solar, earth, ecom1_sun, ecom2_sun, solar_sun,
                                    earth_sun};
    for (size_t i = 0; i < sizeof(models) / sizeof(models[0]); ++i) {
        sidereon_force_model_free(models[i]);
    }
}

static void frames_and_bodies(void) {
    SidereonTimeScales time;
    assert(sidereon_timescales_from_utc(2020, 6, 25, 12, 0, 0.0, &time) == SIDEREON_STATUS_OK);
    double matrix[9];
    assert(sidereon_frame_gcrs_to_mean_of_date_matrix(&time, matrix) == SIDEREON_STATUS_OK);
    vector(matrix, CONTRACT_GCRS_TO_MOD, 9);
    assert(sidereon_frame_mean_of_date_to_gcrs_matrix(&time, matrix) == SIDEREON_STATUS_OK);
    vector(matrix, CONTRACT_MOD_TO_GCRS, 9);
    SidereonSunMoon bodies;
    assert(sidereon_sun_moon_gcrs(0.2, &bodies) == SIDEREON_STATUS_OK);
    vector(bodies.sun_m, CONTRACT_BODIES, 3);
    vector(bodies.moon_m, CONTRACT_BODIES + 3, 3);
    assert(sidereon_sun_moon_gcrs_at(INT64_C(1593086400000000), &bodies) == SIDEREON_STATUS_OK);
    vector(bodies.sun_m, CONTRACT_BODIES_AT, 3);
    vector(bodies.moon_m, CONTRACT_BODIES_AT + 3, 3);
}

static void iers_values(void) {
    SidereonIers2010EopSample samples[2] = {{
        .mjd_utc = 59025, .xp_arcsec = 0.1, .yp_arcsec = 0.2, .ut1_minus_utc_s = -0.2,
        .lod_s = 0.001, .dx_arcsec = 0.0001, .dy_arcsec = -0.0001, .has_pole_rates = true,
        .pole_rates_rad_s = {8e-14, -3e-14},
    }};
    samples[1] = samples[0];
    samples[1].mjd_utc = 59026;
    SidereonIers2010EarthOrientationProvider *strict = NULL, *copy = NULL, *permissive = NULL;
    assert(sidereon_iers2010_earth_orientation_provider_new(samples, 2, &strict) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_iers2010_earth_orientation_provider_with_validity(strict, false, &copy) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_iers2010_earth_orientation_provider_with_validity(strict, true, &permissive) ==
           SIDEREON_STATUS_OK);
    bool equal;
    assert(sidereon_iers2010_earth_orientation_provider_equal(strict, copy, &equal) ==
           SIDEREON_STATUS_OK && equal);
    assert(sidereon_iers2010_earth_orientation_provider_equal(strict, permissive, &equal) ==
           SIDEREON_STATUS_OK && !equal);
    SidereonEarthOrientation orientation;
    assert(sidereon_iers2010_earth_orientation_provider_orientation_at_tdb_seconds(
               permissive, 646315800.25, &orientation) == SIDEREON_STATUS_OK);
    vector(orientation.polar_motion_rad, CONTRACT_IERS_ANGLES, 2);
    vector(orientation.earth_rotation_vector_itrf_rad_s, CONTRACT_IERS_ANGLES + 2, 3);
    assert(orientation.has_iers2010_quaternion &&
           orientation.time_scales.ut1_degraded == SIDEREON_UT1_DEGRADATION_NONE);
    vector(orientation.iers2010_quaternion, CONTRACT_IERS_ANGLES + 5, 4);
    for (size_t i = 0; i < 3; ++i) {
        vector(orientation.gcrf_to_itrf_matrix[i], CONTRACT_IERS_MATRICES + 3*i, 3);
        vector(orientation.itrf_to_gcrf_matrix[i], CONTRACT_IERS_MATRICES + 9 + 3*i, 3);
        vector(orientation.gcrf_to_itrf_rotation_rate_matrix[i],
               CONTRACT_IERS_MATRICES + 18 + 3*i, 3);
        vector(orientation.itrf_to_gcrf_rotation_rate_matrix[i],
               CONTRACT_IERS_MATRICES + 27 + 3*i, 3);
    }
    sidereon_iers2010_earth_orientation_provider_free(strict);
    sidereon_iers2010_earth_orientation_provider_free(copy);
    sidereon_iers2010_earth_orientation_provider_free(permissive);
}

static void check_state(SidereonCartesianState actual, SidereonCartesianState expected) {
    assert(actual.epoch_s == expected.epoch_s);
    assert(memcmp(actual.position_km, expected.position_km, sizeof(actual.position_km)) == 0);
    assert(memcmp(actual.velocity_km_s, expected.velocity_km_s, sizeof(actual.velocity_km_s)) == 0);
}

static void propagation_values(const char *weather_path) {
    const SidereonCartesianState initial = {646315800.25, {6400.0, 0.0, 0.0}, {0.0, 7.9, 0.0}};
    const SidereonPropagationContext context = {0};
    const SidereonStatePropagationEnvironment environment = {.context = &context};
    SidereonStatePropagationConfig config;
    assert(sidereon_state_propagation_config_init(&config) == SIDEREON_STATUS_OK);
    config.epoch_s = initial.epoch_s;
    memcpy(config.position_km, initial.position_km, sizeof(config.position_km));
    memcpy(config.velocity_km_s, initial.velocity_km_s, sizeof(config.velocity_km_s));

    size_t written, required;
    SidereonEphemeris *ephemeris = NULL;
    SidereonCartesianState propagated;
    assert(sidereon_propagate_state_with_force_model(&config, &config.epoch_s, 1, &environment, NULL, 0,
                                                 &ephemeris) == SIDEREON_STATUS_OK);
    assert(sidereon_ephemeris_states(ephemeris, &propagated, 1, &written, &required) ==
           SIDEREON_STATUS_OK && written == 1 && required == 1);
    check_state(propagated, initial);
    sidereon_ephemeris_free(ephemeris);

    assert(sidereon_propagate_state(&config, &config.epoch_s, 1, &ephemeris) ==
           SIDEREON_STATUS_OK);
    assert(sidereon_ephemeris_states(ephemeris, &propagated, 1, &written, &required) ==
           SIDEREON_STATUS_OK && written == 1 && required == 1);
    check_state(propagated, initial);
    sidereon_ephemeris_free(ephemeris);

    SidereonCovarianceMatrix6 covariance = {0};
    for (size_t i = 0; i < 6; ++i) {
        covariance.values[i][i] = (double)(i + 1);
    }
    SidereonStateWithCovariance state_covariance;
    assert(sidereon_propagate_state_with_covariance_with_context(
               &config, &covariance, 0.0, &environment, &state_covariance) == SIDEREON_STATUS_OK);
    check_state(state_covariance.state, initial);
    assert(memcmp(state_covariance.covariance.values, covariance.values, sizeof(covariance.values)) == 0);

    SidereonCovarianceEphemeris *arc = NULL;
    const SidereonCovariancePropagationOptions covariance_options = {
        .input_frame = SIDEREON_COVARIANCE_FRAME_INERTIAL,
        .output_frame = SIDEREON_COVARIANCE_FRAME_INERTIAL,
    };
    assert(sidereon_propagate_covariance_with_context(
               &config, &covariance, &config.epoch_s, 1, covariance_options, &environment,
               &arc) == SIDEREON_STATUS_OK);
    SidereonCovarianceNode node;
    assert(sidereon_covariance_ephemeris_nodes(arc, &node, 1, &written, &required) ==
           SIDEREON_STATUS_OK && written == 1 && required == 1);
    assert(node.frame == SIDEREON_COVARIANCE_FRAME_INERTIAL);
    check_state(node.state, initial);
    assert(memcmp(node.covariance.values, covariance.values, sizeof(covariance.values)) == 0);
    sidereon_covariance_ephemeris_free(arc);

    SidereonDecayConfig decay;
    assert(sidereon_decay_config_init(&decay) == SIDEREON_STATUS_OK);
    SidereonDecayEstimate estimate;
    assert(sidereon_estimate_decay_with_context(&initial, &decay, &context, &estimate) ==
           SIDEREON_STATUS_OK && estimate.time_to_decay_s == 0.0);
    check_state(estimate.reentry_state, initial);
    exact(estimate.reentry_altitude_km, CONTRACT_DECAY_ALTITUDE[0]);

    if (weather_path) {
        size_t length;
        uint8_t *bytes = read_bytes(weather_path, &length);
        SidereonSpaceWeatherTable *weather = NULL;
        assert(sidereon_space_weather_table_parse_csv(bytes, length, &weather) ==
               SIDEREON_STATUS_OK);
        free(bytes);
        assert(sidereon_estimate_decay_with_source_and_context(
                   &initial, &decay, weather, &context, &estimate) == SIDEREON_STATUS_OK);
        assert(estimate.time_to_decay_s == 0.0);
        check_state(estimate.reentry_state, initial);
        exact(estimate.reentry_altitude_km, CONTRACT_DECAY_ALTITUDE[0]);
        sidereon_space_weather_table_free(weather);
    }
}

static void check_propagation_arc(SidereonEphemeris *arc, const uint64_t *pins) {
    SidereonCartesianState states[2];
    size_t written, required;
    assert(sidereon_ephemeris_states(arc, states, 2, &written, &required) ==
           SIDEREON_STATUS_OK && written == 2 && required == 2);
    for (size_t i = 0; i < 2; ++i, pins += 7) {
        exact(states[i].epoch_s, pins[0]);
        vector(states[i].position_km, pins + 1, 3);
        vector(states[i].velocity_km_s, pins + 4, 3);
    }
    sidereon_ephemeris_free(arc);
}

static void propagation_with_forces(const char *weather_path) {
    SidereonStatePropagationConfig config;
    assert(sidereon_state_propagation_config_init(&config) == SIDEREON_STATUS_OK);
    config.epoch_s = 120700800.0;
    config.force_model = SIDEREON_PROPAGATION_FORCE_MODEL_TWO_BODY_J2;
    const double position[] = {6778.0, 20.0, 30.0};
    const double velocity[] = {0.0, 7.5, 1.0};
    memcpy(config.position_km, position, sizeof(position));
    memcpy(config.velocity_km_s, velocity, sizeof(velocity));
    config.has_drag = true;
    const double epochs[] = {config.epoch_s + 60.0, config.epoch_s + 120.0};

    const SidereonEmpiricalRtnTerm terms[] = {
        {SIDEREON_RTN_AXIS_ALONG_TRACK, SIDEREON_EMPIRICAL_HARMONIC_CONSTANT, 2e-6},
        {SIDEREON_RTN_AXIS_CROSS_TRACK, SIDEREON_EMPIRICAL_HARMONIC_CONSTANT, -1e-6},
    };
    SidereonForceModel *along, *cross;
    assert(sidereon_empirical_rtn_acceleration_new(terms, 1, &along) == SIDEREON_STATUS_OK);
    assert(sidereon_empirical_rtn_acceleration_new(terms + 1, 1, &cross) == SIDEREON_STATUS_OK);
    const SidereonForceModel *forces[] = {along, cross};
    assert(sidereon_composite_force_model_new(forces, 2, (SidereonForceModel **)cross) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    legacy_message("sidereon_composite_force_model_new: out overlaps force");
    assert(sidereon_propagate_state_with_force_model(
               &config, epochs, 2, NULL, forces, 2, (SidereonEphemeris **)along) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    legacy_message("sidereon_propagate_state_with_force_model: out overlaps force");

    SidereonEphemeris *arc = NULL;
    assert(sidereon_propagate_state_with_force_model(&config, epochs, 2, NULL, forces, 2, &arc) ==
           SIDEREON_STATUS_OK);
    check_propagation_arc(arc, CONTRACT_PROPAGATION_FIXED_WEATHER);

    SidereonCartesianState final_state;
    SidereonDenseOutput *dense;
    assert(sidereon_propagate_state_with_dense_output(&config, epochs[1], NULL, forces, 2,
                                                      &final_state, &dense) == SIDEREON_STATUS_OK);
    assert(dense != NULL);
    exact(final_state.epoch_s, CONTRACT_PROPAGATION_DENSE_FORCES[0]);
    vector(final_state.position_km, CONTRACT_PROPAGATION_DENSE_FORCES + 1, 3);
    vector(final_state.velocity_km_s, CONTRACT_PROPAGATION_DENSE_FORCES + 4, 3);
    sidereon_dense_output_free(dense);

    const SidereonForceModel *invalid_forces[] = {NULL};
    assert(sidereon_propagate_state_with_force_model(
               &config, epochs, 2, NULL, invalid_forces, 1, &arc) == SIDEREON_STATUS_NULL_POINTER);
    legacy_message("sidereon_propagate_state_with_force_model: null additional_force");
    assert(arc == NULL);

    if (weather_path) {
        size_t length;
        uint8_t *bytes = read_bytes(weather_path, &length);
        SidereonSpaceWeatherTable *table = NULL;
        assert(sidereon_space_weather_table_parse_csv(bytes, length, &table) ==
               SIDEREON_STATUS_OK);
        free(bytes);
        assert(sidereon_sourced_drag_force_new(0.01, 100.0, table, (SidereonForceModel **)table) ==
               SIDEREON_STATUS_INVALID_ARGUMENT);
        legacy_message("sidereon_sourced_drag_force_new: out overlaps table");
        const SidereonStatePropagationEnvironment environment = {.space_weather = table};
        assert(sidereon_propagate_state_with_force_model(
                   &config, epochs, 2, &environment, forces, 2, &arc) == SIDEREON_STATUS_OK);
        check_propagation_arc(arc, CONTRACT_PROPAGATION_TABLE_WEATHER);

        config.has_drag = false;
        assert(sidereon_propagate_state_with_force_model(
                   &config, epochs, 2, &environment, forces, 2, &arc) ==
               SIDEREON_STATUS_INVALID_ARGUMENT);
        assert(arc == NULL);
        detail(SIDEREON_ENGINE_ERROR_FAMILY_PROPAGATION, "invalid_input",
               "{\"message\":\"space weather source without drag\","
               "\"reason\":\"space weather source without drag\"}");
        sidereon_space_weather_table_free(table);
    }

    config.force_model = SIDEREON_PROPAGATION_FORCE_MODEL_COMPOSITE;
    config.force_components.has_zonal = true;
    config.force_components.has_spherical_harmonic = true;
    assert(sidereon_propagate_state_with_force_model(&config, epochs, 2, NULL, NULL, 0, &arc) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    detail(SIDEREON_ENGINE_ERROR_FAMILY_PROPAGATION, "invalid_input",
           "{\"message\":\"zonal and spherical harmonic gravity cannot both be selected\","
           "\"reason\":\"zonal and spherical harmonic gravity cannot both be selected\"}");
    sidereon_force_model_free(along);
    sidereon_force_model_free(cross);
}

static void j2_force(void) {
    SidereonForceModel *j2;
    assert(sidereon_j2_gravity_new(398600.4418, 6378.137, 1.08262668e-3, &j2) ==
           SIDEREON_STATUS_OK);
    SidereonCartesianState state = {NAN, {6778.0, 20.0, 30.0}, {NAN, NAN, NAN}};
    SidereonVector3 acceleration;
    assert(sidereon_force_model_acceleration(j2, state, NULL, &acceleration) == SIDEREON_STATUS_OK);
    vector(acceleration.xyz, CONTRACT_J2, 3);
    SidereonAccelerationPartials partials;
    assert(sidereon_force_model_acceleration_partials(j2, state, NULL, 0, NULL, &partials) ==
           SIDEREON_STATUS_OK);
    vector(partials.wrt_position, CONTRACT_J2_PARTIALS, 9);
    for (size_t i = 0; i < 9; ++i) {
        exact(partials.wrt_velocity[i], 0);
    }
    state.position_km[0] = 1e80;
    state.position_km[1] = -2e80;
    state.position_km[2] = 3e80;
    assert(sidereon_force_model_acceleration(j2, state, NULL, &acceleration) == SIDEREON_STATUS_OK);
    vector(acceleration.xyz, CONTRACT_J2_EXTREME, 3);
    state.position_km[1] = NAN;
    assert(sidereon_force_model_acceleration(j2, state, NULL, &acceleration) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    detail(SIDEREON_ENGINE_ERROR_FAMILY_PROPAGATION, "invalid_input",
           "{\"message\":\"position must be finite\",\"reason\":\"position must be finite\"}");
    sidereon_force_model_free(j2);
}

static void gravity_values(const char *path) {
    size_t length;
    uint8_t *bytes = read_bytes(path, &length);
    SidereonIcgemGravityField *field = NULL;
    assert(sidereon_icgem_gravity_field_parse(bytes, length, &field) == SIDEREON_STATUS_OK);
    free(bytes);
    SidereonIcgemSigmas sigmas;
    assert(sidereon_icgem_gravity_field_static_sigmas(field, 3, 1, &sigmas) == SIDEREON_STATUS_OK);
    assert(sigmas.has_calibrated && sigmas.has_formal &&
           sigmas.calibrated[0] == 0.13965165e-9 && sigmas.calibrated[1] == 0.13645882e-9 &&
           sigmas.formal[0] == 0.13965165e-9 && sigmas.formal[1] == 0.13645882e-9);
    SidereonIcgemTimeVariableTerm terms[27];
    size_t written, required;
    assert(sidereon_icgem_gravity_field_time_variable_terms(field, terms, 27, &written, &required) ==
           SIDEREON_STATUS_OK && written == 27 && required == 27);
    assert(terms[0].line == 16 && terms[0].kind == SIDEREON_ICGEM_TERM_KIND_CONSTANT &&
           terms[0].period_years == 0.0 && terms[0].degree == 1 && terms[0].order == 0 &&
           terms[0].c == 7e-11 && terms[0].s == 0.0 &&
           terms[0].sigmas.has_calibrated && terms[0].sigmas.has_formal &&
           terms[0].sigmas.calibrated[0] == 1e-13 && terms[0].sigmas.calibrated[1] == 0.0 &&
           terms[0].sigmas.formal[0] == 2e-13 && terms[0].sigmas.formal[1] == 0.0 &&
           terms[0].reference_tt_seconds == -43200.0 && terms[0].valid_from_tt_seconds == -43200.0 &&
           terms[0].valid_until_tt_seconds == 946728000.0);
    assert(terms[1].kind == SIDEREON_ICGEM_TERM_KIND_COSINE && terms[1].period_years == 1.0 &&
           terms[1].c == 7.8e-11);
    bool sine_found = false;
    for (size_t i = 0; i < written; ++i) {
        if (terms[i].line == 26) {
            assert(terms[i].kind == SIDEREON_ICGEM_TERM_KIND_SINE &&
                   terms[i].period_years == 1.0 && terms[i].c == 5.92e-11);
            sine_found = true;
        }
    }
    assert(sine_found);
    SidereonSphericalHarmonicGravity *gravity = NULL;
    assert(sidereon_icgem_gravity_field_gravity(field, 2, 2, &gravity) == SIDEREON_STATUS_OK);
    assert(sidereon_spherical_harmonic_gravity_as_force_model(gravity, (SidereonForceModel **)gravity) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    legacy_message("sidereon_spherical_harmonic_gravity_as_force_model: out overlaps gravity");
    SidereonVector3 position = {{7000.0, 20.0, 30.0}}, acceleration;
    assert(sidereon_spherical_harmonic_gravity_body_fixed_acceleration_at_km_s2(
               gravity, position, 86400.0, &acceleration) == SIDEREON_STATUS_OK);
    vector(acceleration.xyz, CONTRACT_ICGEM_ACCELERATION, 3);
    sidereon_spherical_harmonic_gravity_free(gravity);
    sidereon_icgem_gravity_field_free(field);

    const char zero_field[] = ICGEM_HEADER "tide_system tide_free\nend_of_head\n" "gfc 2 0 0 0\n" ICGEM_21 "gfc 2 2 0 0\n";
    assert(parse_field(zero_field, &field) == SIDEREON_STATUS_OK);
    assert(sidereon_icgem_gravity_field_gravity(field, 2, 2, &gravity) == SIDEREON_STATUS_OK);
    double potential;
    assert(sidereon_spherical_harmonic_gravity_body_fixed_potential_km2_s2(
               gravity, position, &potential) == SIDEREON_STATUS_OK && potential == 0.0);
    sidereon_spherical_harmonic_gravity_free(gravity);
    sidereon_icgem_gravity_field_free(field);
}

static void shared_engine_errors(void) {
    SidereonSunMoon bodies;
    assert(sidereon_sun_moon_gcrs(NAN, &bodies) == SIDEREON_STATUS_INVALID_ARGUMENT);
    detail(SIDEREON_ENGINE_ERROR_FAMILY_SUN_MOON, "invalid_input",
           "{\"field\":\"t\",\"reason\":\"not finite\"}");
    assert(sidereon_sun_moon_gcrs_at(INT64_MAX, &bodies) == SIDEREON_STATUS_INVALID_ARGUMENT);
    detail(SIDEREON_ENGINE_ERROR_FAMILY_SUN_MOON, "frame_transform",
           "{\"cause\":{\"kind\":\"invalid_input\",\"fields\":"
           "{\"field\":\"datetime\",\"reason\":\"invalid UTC instant\"}}}");

    const SidereonVector3 position = {{7000.0, 0.0, 0.0}};
    const SidereonVector3 velocity = {{0.0, 7.5, 0.0}};
    const SidereonVector3 sun = {{150000000.0, 0.0, 0.0}};
    const SidereonVector3 zero = {{0.0, 0.0, 0.0}};
    double fraction;
    assert(sidereon_eclipse_shadow_fraction_with_moon(position.xyz, sun.xyz, zero.xyz, &fraction) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    detail(SIDEREON_ENGINE_ERROR_FAMILY_ECLIPSE, "invalid_input",
           "{\"field\":\"moon_pos\",\"reason\":\"zero vector\"}");
    SidereonEcomGeometry geometry;
    assert(sidereon_ecom_geometry(position, velocity, sun, &geometry) == SIDEREON_STATUS_SOLVE);
    detail(SIDEREON_ENGINE_ERROR_FAMILY_PROPAGATION, "numerical_failure",
           "{\"message\":\"ECOM DYB frame is undefined or overflows\","
           "\"reason\":\"ECOM DYB frame is undefined or overflows\"}");
    SidereonForceModel *srp = NULL;
    assert(sidereon_solar_radiation_pressure_new(1.25, 0.02,
               sidereon_radiation_shadow_options_default(), &srp) == SIDEREON_STATUS_OK);
    SidereonVector3 result;
    assert(sidereon_solar_radiation_pressure_acceleration_with_sun_moon(
               srp, position, sun, zero, &result) == SIDEREON_STATUS_SOLVE);
    detail(SIDEREON_ENGINE_ERROR_FAMILY_PROPAGATION, "force_model_failure",
           "{\"message\":\"eclipse: invalid eclipse input moon_pos: zero vector\","
           "\"reason\":\"eclipse: invalid eclipse input moon_pos: zero vector\"}");
    sidereon_force_model_free(srp);
}

static void decay_errors(void) {
    const SidereonEngineErrorFamily family = SIDEREON_ENGINE_ERROR_FAMILY_DECAY;
    SidereonDecayConfig config;
    assert(sidereon_decay_config_init(&config) == SIDEREON_STATUS_OK);
    SidereonCartesianState initial = {646315800.25, {7000.0, 0.0, 0.0}, {0.0, 7.5, 0.0}};
    SidereonDecayEstimate result;
    config.scan_step_s = 0.0;
    assert(sidereon_estimate_decay_with_context(&initial, &config, NULL, &result) ==
           SIDEREON_STATUS_INVALID_ARGUMENT);
    detail(family, "invalid_config", "{\"field\":\"scan_step_s\",\"reason\":\"scan_step_s\"}");
    config.scan_step_s = 1.0;
    config.max_duration_s = 1.0;
    assert(sidereon_estimate_decay_with_context(&initial, &config, NULL, &result) ==
           SIDEREON_STATUS_SOLVE);
    detail(family, "no_decay_within_horizon",
           "{\"horizon_s\":" F64("1", "3ff0000000000000") "}");
    config.max_duration_s = 5.0;
    config.max_scan_samples = 1;
    assert(sidereon_estimate_decay_with_context(&initial, &config, NULL, &result) ==
           SIDEREON_STATUS_SOLVE);
    detail(family, "scan_budget_exhausted",
           "{\"scanned_s\":" F64("1", "3ff0000000000000") ",\"samples\":1}");
    initial.position_km[0] = NAN;
    assert(sidereon_estimate_decay_with_context(&initial, &config, NULL, &result) ==
           SIDEREON_STATUS_SOLVE);
    detail(family, "propagation", "{\"cause\":{\"kind\":\"invalid_input\",\"fields\":"
           "{\"message\":\"position_km not finite\",\"reason\":\"position_km not finite\"}}}");
}

int main(int argc, char **argv) {
    icgem_parse_errors();
    icgem_queries();
    gnss_blocks_and_constants();
    yaw_geometry_errors();
    eclipse_values_and_errors();
    analytic_values();
    analytic_errors();
    de_errors();
    ecom_names();
    force_models();
    frames_and_bodies();
    shared_engine_errors();
    decay_errors();
    iers_values();
    propagation_values(argc > 3 ? argv[3] : NULL);
    propagation_with_forces(argc > 3 ? argv[3] : NULL);
    j2_force();
    if (argc > 1) {
        de_values(argv[1]);
    }
    if (argc > 2) {
        gravity_values(argv[2]);
    }
    return 0;
}
