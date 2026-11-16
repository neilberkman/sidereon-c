#include "sidereon.h"
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CHECK(condition) do { if (!(condition)) { fprintf(stderr, "failure at line %d: %s\n", __LINE__, #condition); return 1; } } while (0)
#define CHECK_BUFFER(call) do { \
    size_t written = 77, required = 0; uint8_t tiny[1]; \
    CHECK((call(NULL, 0, &written, &required)) == SIDEREON_STATUS_OK); \
    CHECK(written == 0 && required > 1); size_t expected = required; \
    CHECK((call(tiny, sizeof tiny, &written, &required)) == SIDEREON_STATUS_INVALID_ARGUMENT); \
    CHECK(written == 0 && required == expected); \
} while (0)
#define SOURCE(out, len, written, required) sidereon_celestrak_gp_source_entry(out, len, written, required)
#define NAV_SOURCE(out, len, written, required) sidereon_navcen_source_entry(out, len, written, required)
#define NAV_URL(out, len, written, required) sidereon_navcen_constellation_url(out, len, written, required)
#define GROUP(out, len, written, required) sidereon_celestrak_group(SIDEREON_GNSS_SYSTEM_GPS, out, len, written, required)
#define CONST_URL(out, len, written, required) sidereon_celestrak_constellation_url(SIDEREON_GNSS_SYSTEM_GPS, SIDEREON_CELESTRAK_GP_FORMAT_JSON, out, len, written, required)
#define GP_URL(out, len, written, required) sidereon_celestrak_gp_url(SIDEREON_CELESTRAK_GP_QUERY_KIND_GROUP, (const uint8_t *)"active", 6, SIDEREON_CELESTRAK_GP_FORMAT_JSON, out, len, written, required)
#define CAT_URL(out, len, written, required) sidereon_celestrak_gp_catalog_number_url(25544, SIDEREON_CELESTRAK_GP_FORMAT_JSON, out, len, written, required)
#define SPECIAL_URL(out, len, written, required) sidereon_celestrak_gp_special_url(SIDEREON_CELESTRAK_GP_SPECIAL_GPZ_PLUS, SIDEREON_CELESTRAK_GP_FORMAT_JSON, out, len, written, required)
#define SPECIAL(out, len, written, required) sidereon_celestrak_gp_special_as_str(SIDEREON_CELESTRAK_GP_SPECIAL_GPZ_PLUS, out, len, written, required)
#define FORMAT(out, len, written, required) sidereon_celestrak_gp_format_as_str(SIDEREON_CELESTRAK_GP_FORMAT_JSON_PRETTY, out, len, written, required)
#define HOSTS(out, len, written, required) sidereon_data_allowed_hosts(out, len, written, required)
#define RAW(out, len, written, required) sidereon_celestrak_gp_response_raw(response, false, 0, out, len, written, required)
#define RECORD(out, len, written, required) sidereon_celestrak_gp_response_record_json(response, false, 0, out, len, written, required)
#define KVN(out, len, written, required) sidereon_omm_to_kvn_lenient(omm, out, len, written, required)
#define XML(out, len, written, required) sidereon_omm_to_xml_lenient(omm, out, len, written, required)

static uint8_t *read_file(const char *path, size_t *size) {
    FILE *file = fopen(path, "rb");
    if (!file || fseek(file, 0, SEEK_END) != 0) return NULL;
    long len = ftell(file);
    if (len < 0 || fseek(file, 0, SEEK_SET) != 0) { fclose(file); return NULL; }
    uint8_t *data = malloc((size_t)len + 1);
    if (!data) { fclose(file); return NULL; }
    *size = fread(data, 1, (size_t)len, file);
    fclose(file);
    if (*size != (size_t)len) { free(data); return NULL; }
    return data;
}

static int format_records(const char *xml_path, const char *kvn_path, const char *json_path, const char *pretty_path) {
    const char tle[] = "ISS\n1 25544U 98067A   26095.55331950  .00008543  00000+0  16420-3 0  9998\n2 25544  51.6328 299.5432 0006351 274.8255  85.2008 15.48786980560492\n";
    const char three[] = "0 ISS\n1 25544U 98067A   26095.55331950  .00008543  00000+0  16420-3 0  9998\n2 25544  51.6328 299.5432 0006351 274.8255  85.2008 15.48786980560492\n";
    const char two[] = "1 25544U 98067A   26095.55331950  .00008543  00000+0  16420-3 0  9998\n2 25544  51.6328 299.5432 0006351 274.8255  85.2008 15.48786980560492\n";
    const char csv[] = "OBJECT_NAME,EPOCH,MEAN_MOTION,ECCENTRICITY,INCLINATION,RA_OF_ASC_NODE,ARG_OF_PERICENTER,MEAN_ANOMALY,BSTAR\nISS,2026-01-01T00:00:00,1,0,0,0,0,0,0";
    size_t lengths[8] = {strlen(tle), strlen(three), strlen(two), 0, 0, 0, 0, strlen(csv)};
    uint8_t *files[4] = {read_file(xml_path, &lengths[3]), read_file(kvn_path, &lengths[4]), read_file(json_path, &lengths[5]), read_file(pretty_path, &lengths[6])};
    CHECK(files[0] && files[1] && files[2] && files[3]);
    const uint8_t *inputs[8] = {(const uint8_t *)tle, (const uint8_t *)three, (const uint8_t *)two, files[0], files[1], files[2], files[3], (const uint8_t *)csv};
    for (uint32_t format = 0; format < 8; ++format) {
        SidereonCelestrakGpResponse *response = NULL;
        CHECK(sidereon_parse_celestrak_gp_response(inputs[format], lengths[format], format, &response) == SIDEREON_STATUS_OK);
        size_t accepted = 0, rejected = 0, written = 0, required = 0;
        CHECK(sidereon_celestrak_gp_response_counts(response, &accepted, &rejected) == SIDEREON_STATUS_OK && accepted == 1 && rejected == 0);
        uint8_t *raw = malloc(lengths[format]); CHECK(raw != NULL);
        CHECK(sidereon_celestrak_gp_response_raw(response, false, 0, raw, lengths[format], &written, &required) == SIDEREON_STATUS_OK);
        CHECK(written == lengths[format] && memcmp(raw, inputs[format], written) == 0);
        free(raw); sidereon_celestrak_gp_response_free(response);
    }
    for (size_t i = 0; i < 4; ++i) free(files[i]);
    return 0;
}

static int spans(void) {
    const char *inputs[] = {"<ndm>\n<omm></omm>\n</ndm>", "EPOCH\nbad\n"};
    for (size_t i = 0; i < 2; ++i) {
        SidereonOmmArray *array = NULL;
        SidereonStatus status = i == 0
            ? sidereon_omm_parse_xml_all((const uint8_t *)inputs[i], strlen(inputs[i]), &array)
            : sidereon_omm_parse_csv_array((const uint8_t *)inputs[i], strlen(inputs[i]), &array);
        CHECK(status == SIDEREON_STATUS_OK);
        size_t count = 0;
        SidereonOmmRecordSpan span;
        CHECK(sidereon_omm_array_span_count(array, &count) == SIDEREON_STATUS_OK && count == 1);
        CHECK(sidereon_omm_array_span(array, 0, &span) == SIDEREON_STATUS_OK);
        CHECK(span.index == 0 && span.byte_start == 6 && span.byte_end == (i == 0 ? 17 : 9) && span.line == 2);
        sidereon_omm_array_free(array);
    }
    return 0;
}

static int strict_writers(void) {
    const char json[] = "{\"OBJECT_NAME\":\"TEST\",\"OBJECT_ID\":\"2026-001A\",\"CENTER_NAME\":\"EARTH\",\"REF_FRAME\":\"TEME\",\"TIME_SYSTEM\":\"UTC\",\"MEAN_ELEMENT_THEORY\":\"OTHER\",\"EPOCH\":\"2026-01-01T00:00:00\",\"MEAN_MOTION\":1,\"ECCENTRICITY\":0.1,\"INCLINATION\":2,\"RA_OF_ASC_NODE\":3,\"ARG_OF_PERICENTER\":4,\"MEAN_ANOMALY\":5}";
    const char kvn[] = "CCSDS_OMM_VERS = 3.0\nCREATION_DATE = 2026-01-01T00:00:00\nORIGINATOR = TEST\nOBJECT_NAME = TEST\nOBJECT_ID = 2026-001A\nCENTER_NAME = EARTH\nREF_FRAME = TEME\nTIME_SYSTEM = UTC\nMEAN_ELEMENT_THEORY = OTHER\nEPOCH = 2026-01-01T00:00:00.000000\nMEAN_MOTION = 1\nECCENTRICITY = 0.1\nINCLINATION = 2\nRA_OF_ASC_NODE = 3\nARG_OF_PERICENTER = 4\nMEAN_ANOMALY = 5\n";
    const char xml[] = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<ndm>\n<omm id=\"CCSDS_OMM_VERS\" version=\"3.0\">\n<header><CREATION_DATE>2026-01-01T00:00:00</CREATION_DATE><ORIGINATOR>TEST</ORIGINATOR></header>\n<body><segment>\n<metadata><OBJECT_NAME>TEST</OBJECT_NAME><OBJECT_ID>2026-001A</OBJECT_ID><CENTER_NAME>EARTH</CENTER_NAME><REF_FRAME>TEME</REF_FRAME><TIME_SYSTEM>UTC</TIME_SYSTEM><MEAN_ELEMENT_THEORY>OTHER</MEAN_ELEMENT_THEORY></metadata>\n<data>\n<meanElements><EPOCH>2026-01-01T00:00:00.000000</EPOCH><MEAN_MOTION>1</MEAN_MOTION><ECCENTRICITY>0.1</ECCENTRICITY><INCLINATION>2</INCLINATION><RA_OF_ASC_NODE>3</RA_OF_ASC_NODE><ARG_OF_PERICENTER>4</ARG_OF_PERICENTER><MEAN_ANOMALY>5</MEAN_ANOMALY></meanElements>\n</data>\n</segment></body>\n</omm>\n</ndm>\n";
    SidereonOmm *source = NULL, *headed = NULL;
    CHECK(sidereon_omm_parse_json((const uint8_t *)json, strlen(json), &source) == SIDEREON_STATUS_OK);
    CHECK(sidereon_omm_with_ccsds_header(source, (const uint8_t *)"3.0", 3, (const uint8_t *)"2026-01-01T00:00:00", 19, (const uint8_t *)"TEST", 4, &headed) == SIDEREON_STATUS_OK);
    uint8_t out[2048]; size_t written = 0, required = 0;
    CHECK(sidereon_omm_to_kvn(headed, out, sizeof out, &written, &required) == SIDEREON_STATUS_OK);
    CHECK(written == strlen(kvn) && memcmp(out, kvn, written) == 0);
    CHECK(sidereon_omm_to_xml(headed, out, sizeof out, &written, &required) == SIDEREON_STATUS_OK);
    CHECK(written == strlen(xml) && memcmp(out, xml, written) == 0);
    sidereon_omm_free(headed); sidereon_omm_free(source);
    return 0;
}

int main(int argc, char **argv) {
    CHECK(argc == 5);
    CHECK(format_records(argv[1], argv[2], argv[3], argv[4]) == 0);
    CHECK(strict_writers() == 0);
    CHECK(spans() == 0);
    const char body[] = "ISS (ZARYA)\n1 25544U 98067A   26095.55331950  .00008543  00000+0  16420-3 0  9998\n2 25544  51.6328 299.5432 0006351 274.8255  85.2008 15.48786980560492\n";
    SidereonCelestrakGpResponse *response = NULL;
    CHECK(sidereon_parse_celestrak_gp_response((const uint8_t *)body, strlen(body), SIDEREON_CELESTRAK_GP_FORMAT_TLE, &response) == SIDEREON_STATUS_OK);
    CHECK(response != NULL);
    size_t count = 0, rejected = 0;
    CHECK(sidereon_celestrak_gp_response_counts(response, &count, &rejected) == SIDEREON_STATUS_OK && count == 1 && rejected == 0);
    CHECK_BUFFER(SOURCE); CHECK_BUFFER(NAV_SOURCE); CHECK_BUFFER(NAV_URL); CHECK_BUFFER(GROUP);
    CHECK_BUFFER(CONST_URL); CHECK_BUFFER(GP_URL); CHECK_BUFFER(CAT_URL); CHECK_BUFFER(SPECIAL_URL);
    CHECK_BUFFER(SPECIAL); CHECK_BUFFER(FORMAT); CHECK_BUFFER(HOSTS); CHECK_BUFFER(RAW); CHECK_BUFFER(RECORD);
    SidereonTle *tle = NULL;
    CHECK(sidereon_celestrak_gp_record_tle(response, 0, SIDEREON_TLE_OPS_MODE_IMPROVED, SIDEREON_TLE_POLICY_STRICT, &tle) == SIDEREON_STATUS_OK);
    SidereonOmmElementSet record_elements;
    CHECK(sidereon_celestrak_gp_record_to_element_set(response, 0, &record_elements) == SIDEREON_STATUS_OK);
    CHECK(record_elements.epoch_whole == 2461135.5 && record_elements.epoch_fraction == 0.5533195);
    CHECK(record_elements.bstar == 0.0001642 && record_elements.mean_motion_dot_present && record_elements.mean_motion_dot == 0.00008543);
    CHECK(record_elements.mean_motion_double_dot_present && record_elements.mean_motion_double_dot == 0.0);
    CHECK(record_elements.eccentricity == 0.0006351 && record_elements.argument_of_perigee_deg == 274.8255);
    CHECK(record_elements.inclination_deg == 51.6328 && record_elements.mean_anomaly_deg == 85.2008);
    CHECK(record_elements.mean_motion_rev_per_day == 15.48786980 && record_elements.right_ascension_deg == 299.5432);
    CHECK(record_elements.catalog_number_present && record_elements.catalog_number == 25544);
    CHECK(!record_elements.omm_epoch_days_present && record_elements.omm_epoch_days == 0.0);
    SidereonOmm *omm = NULL, *from_tle = NULL;
    CHECK(sidereon_celestrak_gp_record_to_omm(response, 0, &omm) == SIDEREON_STATUS_OK);
    CHECK(sidereon_omm_from_tle(tle, (const uint8_t *)"ISS (ZARYA)", 11, &from_tle) == SIDEREON_STATUS_OK);
    uint8_t snapshot[4096]; size_t snap_written = 0, snap_required = 0;
    CHECK(sidereon_omm_snapshot_json(from_tle, snapshot, sizeof snapshot - 1, &snap_written, &snap_required) == SIDEREON_STATUS_OK);
    snapshot[snap_written] = 0;
    const char *fields[] = {
        "\"ccsds_omm_vers\":null", "\"classification\":null", "\"creation_date\":null", "\"originator\":null", "\"message_id\":null",
        "\"object_name\":\"ISS (ZARYA)\"", "\"object_id\":\"1998-067A\"", "\"center_name\":\"EARTH\"", "\"ref_frame\":\"TEME\"", "\"ref_frame_epoch\":null",
        "\"time_system\":\"UTC\"", "\"mean_element_theory\":\"SGP4\"", "\"mean_motion\":15.4878698", "\"semi_major_axis_km\":null",
        "\"eccentricity\":0.0006351", "\"inclination_deg\":51.6328", "\"ra_of_asc_node_deg\":299.5432", "\"arg_of_pericenter_deg\":274.8255", "\"mean_anomaly_deg\":85.2008",
        "\"gm_km3_s2\":null", "\"spacecraft\":null", "\"ephemeris_type\":0", "\"classification_type\":\"U\"", "\"norad_cat_id\":25544", "\"element_set_no\":999", "\"rev_at_epoch\":56049",
        "\"bstar\":0.0001642", "\"bterm_m2_kg\":null", "\"mean_motion_dot\":0.00008543", "\"mean_motion_ddot\":0.0", "\"agom_m2_kg\":null", "\"covariance\":null", "\"user_defined\":[]", "\"exact_sgp4_epoch\":null", "\"quantize_tle_derived_fields\":true",
        "\"year\":2026", "\"month\":4", "\"day\":5", "\"hour\":13", "\"minute\":16", "\"second\":46", "\"microsecond\":804800", "\"femtosecond\":0",
        "\"header\":[]", "\"metadata\":[]", "\"mean_elements\":[]", "\"tle_parameters\":[]", "\"user_defined\":[]"
    };
    for (size_t i = 0; i < sizeof fields / sizeof fields[0]; ++i) CHECK(strstr((const char *)snapshot, fields[i]) != NULL);
    CHECK(sidereon_omm_snapshot_json(omm, snapshot, sizeof snapshot - 1, &snap_written, &snap_required) == SIDEREON_STATUS_OK);
    snapshot[snap_written] = 0;
    for (size_t i = 0; i < sizeof fields / sizeof fields[0]; ++i) CHECK(strstr((const char *)snapshot, fields[i]) != NULL);
    sidereon_omm_free(from_tle);
    CHECK_BUFFER(KVN); CHECK_BUFFER(XML);
    uint8_t raw[sizeof body]; size_t written = 0, required = 0;
    CHECK(sidereon_celestrak_gp_response_raw(response, false, 0, raw, sizeof raw, &written, &required) == SIDEREON_STATUS_OK);
    CHECK(written == strlen(body) && memcmp(raw, body, written) == 0);
    CHECK(sidereon_celestrak_gp_response_raw(response, false, 0, (uint8_t *)response, 1, &written, &required) == SIDEREON_STATUS_INVALID_ARGUMENT);
    CHECK(sidereon_celestrak_gp_response_raw(response, false, 0, raw, sizeof raw, &written, &written) == SIDEREON_STATUS_INVALID_ARGUMENT);
    SidereonOmm *invalid = (SidereonOmm *)(uintptr_t)1;
    const uint8_t invalid_utf8[] = {0xff};
    CHECK(sidereon_omm_from_tle(tle, invalid_utf8, sizeof invalid_utf8, &invalid) == SIDEREON_STATUS_INVALID_TOKEN && invalid == NULL);
    invalid = (SidereonOmm *)(uintptr_t)1;
    CHECK(sidereon_omm_with_ccsds_header(omm, invalid_utf8, 1, (const uint8_t *)"2026-01-01T00:00:00", 19, (const uint8_t *)"TEST", 4, &invalid) == SIDEREON_STATUS_INVALID_TOKEN && invalid == NULL);
    invalid = (SidereonOmm *)(uintptr_t)1;
    CHECK(sidereon_omm_with_ccsds_header(omm, (const uint8_t *)"3.0", 3, invalid_utf8, 1, (const uint8_t *)"TEST", 4, &invalid) == SIDEREON_STATUS_INVALID_TOKEN && invalid == NULL);
    invalid = (SidereonOmm *)(uintptr_t)1;
    CHECK(sidereon_omm_with_ccsds_header(omm, (const uint8_t *)"3.0", 3, (const uint8_t *)"2026-01-01T00:00:00", 19, invalid_utf8, 1, &invalid) == SIDEREON_STATUS_INVALID_TOKEN && invalid == NULL);
    SidereonSgp4FitConfig config;
    SidereonSgp4TleFit *fit = (SidereonSgp4TleFit *)(uintptr_t)1;
    CHECK(sidereon_sgp4_fit_config_init(&config) == SIDEREON_STATUS_OK);
    CHECK(sidereon_sgp4_fit_tle_with_omm_header(NULL, 0, &config, invalid_utf8, 1, (const uint8_t *)"TEST", 4, &fit) == SIDEREON_STATUS_INVALID_TOKEN && fit == NULL);
    fit = (SidereonSgp4TleFit *)(uintptr_t)1;
    CHECK(sidereon_sgp4_fit_tle_with_omm_header(NULL, 0, &config, (const uint8_t *)"2026-01-01T00:00:00", 19, invalid_utf8, 1, &fit) == SIDEREON_STATUS_INVALID_TOKEN && fit == NULL);
    sidereon_tle_free(tle);
    CHECK(sidereon_celestrak_gp_record_tle(response, 0, UINT32_MAX, 0, &tle) == SIDEREON_STATUS_INVALID_ARGUMENT);
    CHECK(tle == NULL);
    tle = (SidereonTle *)(uintptr_t)1;
    CHECK(sidereon_celestrak_gp_record_tle(response, 0, SIDEREON_TLE_OPS_MODE_IMPROVED, UINT32_MAX, &tle) == SIDEREON_STATUS_INVALID_ARGUMENT && tle == NULL);
    sidereon_omm_free(omm);
    omm = NULL;
    CHECK(sidereon_celestrak_gp_url(0, invalid_utf8, 1, 5, raw, sizeof raw, &written, &required) == SIDEREON_STATUS_INVALID_TOKEN);
    SidereonCelestrakGpResponse *invalid_response = (SidereonCelestrakGpResponse *)(uintptr_t)1;
    CHECK(sidereon_parse_celestrak_gp_response((const uint8_t *)body, strlen(body), UINT32_MAX, &invalid_response) == SIDEREON_STATUS_INVALID_ARGUMENT && invalid_response == NULL);
    invalid_response = (SidereonCelestrakGpResponse *)(uintptr_t)1;
    CHECK(sidereon_parse_celestrak_gp_response(invalid_utf8, 1, SIDEREON_CELESTRAK_GP_FORMAT_JSON, &invalid_response) == SIDEREON_STATUS_INVALID_ARGUMENT && invalid_response == NULL);
    CHECK(sidereon_celestrak_gp_url(UINT32_MAX, (const uint8_t *)"active", 6, 5, raw, sizeof raw, &written, &required) == SIDEREON_STATUS_INVALID_ARGUMENT);
    CHECK(sidereon_celestrak_gp_url(0, (const uint8_t *)"active", 6, UINT32_MAX, raw, sizeof raw, &written, &required) == SIDEREON_STATUS_INVALID_ARGUMENT);
    CHECK(sidereon_celestrak_constellation_url(UINT32_MAX, 5, raw, sizeof raw, &written, &required) == SIDEREON_STATUS_INVALID_ARGUMENT);
    CHECK(sidereon_celestrak_constellation_url(0, UINT32_MAX, raw, sizeof raw, &written, &required) == SIDEREON_STATUS_INVALID_ARGUMENT);
    CHECK(sidereon_celestrak_group(UINT32_MAX, raw, sizeof raw, &written, &required) == SIDEREON_STATUS_INVALID_ARGUMENT);
    const char hosts[] = "[\"www.aiub.unibe.ch\",\"download.aiub.unibe.ch\",\"zhw-b.s3.cloud.switch.ch\",\"navigation-office.esa.int\",\"isdc-data.gfz.de\",\"igs.bkg.bund.de\",\"igs.gnsswhu.cn\",\"s3.amazonaws.com\",\"celestrak.org\",\"cddis.nasa.gov\",\"urs.earthdata.nasa.gov\",\"www.navcen.uscg.gov\"]";
    uint8_t host_buffer[512];
    CHECK(sidereon_data_allowed_hosts(host_buffer, sizeof host_buffer, &written, &required) == SIDEREON_STATUS_OK);
    CHECK(written == strlen(hosts) && memcmp(host_buffer, hosts, written) == 0);
    CHECK(sidereon_celestrak_gp_catalog_number_url(25544, UINT32_MAX, raw, sizeof raw, &written, &required) == SIDEREON_STATUS_INVALID_ARGUMENT);
    CHECK(sidereon_celestrak_gp_catalog_number_url(0, 5, raw, sizeof raw, &written, &required) == SIDEREON_STATUS_INVALID_ARGUMENT);
    CHECK(sidereon_celestrak_gp_catalog_number_url(1000000000, 5, raw, sizeof raw, &written, &required) == SIDEREON_STATUS_INVALID_ARGUMENT);
    CHECK(sidereon_celestrak_gp_catalog_number_url(UINT64_MAX, 5, raw, sizeof raw, &written, &required) == SIDEREON_STATUS_INVALID_ARGUMENT);
    CHECK(sidereon_celestrak_gp_special_url(UINT32_MAX, 5, raw, sizeof raw, &written, &required) == SIDEREON_STATUS_INVALID_ARGUMENT);
    CHECK(sidereon_celestrak_gp_special_url(0, UINT32_MAX, raw, sizeof raw, &written, &required) == SIDEREON_STATUS_INVALID_ARGUMENT);
    CHECK(sidereon_celestrak_gp_format_as_str(UINT32_MAX, raw, sizeof raw, &written, &required) == SIDEREON_STATUS_INVALID_ARGUMENT);
    CHECK(sidereon_celestrak_gp_special_as_str(UINT32_MAX, raw, sizeof raw, &written, &required) == SIDEREON_STATUS_INVALID_ARGUMENT);
    bool *overlap = (bool *)response;
    CHECK(sidereon_celestrak_gp_response_equal(response, response, overlap) == SIDEREON_STATUS_INVALID_ARGUMENT);
    CHECK(sidereon_celestrak_gp_record_equal(response, response, false, 0, 0, overlap) == SIDEREON_STATUS_INVALID_ARGUMENT);
    CHECK(sidereon_celestrak_gp_payload_equal(response, response, 0, 0, overlap) == SIDEREON_STATUS_INVALID_ARGUMENT);
    CHECK(sidereon_celestrak_tle_record_equal(response, response, 0, 0, overlap) == SIDEREON_STATUS_INVALID_ARGUMENT);
    CHECK(sidereon_celestrak_gp_rejection_reason_equal(response, response, 0, 0, overlap) == SIDEREON_STATUS_INVALID_ARGUMENT);
    CHECK(sidereon_celestrak_gp_structural_error_equal(response, response, 0, 0, overlap) == SIDEREON_STATUS_INVALID_ARGUMENT);
    CHECK(sidereon_celestrak_gp_record_to_omm(response, 0, (SidereonOmm **)response) == SIDEREON_STATUS_INVALID_ARGUMENT);
    CHECK(sidereon_celestrak_gp_record_tle(response, 0, 0, 0, (SidereonTle **)response) == SIDEREON_STATUS_INVALID_ARGUMENT);
    CHECK(sidereon_celestrak_gp_record_to_element_set(response, 0, (SidereonOmmElementSet *)response) == SIDEREON_STATUS_INVALID_ARGUMENT);
    CHECK(sidereon_celestrak_gp_response_record_json(response, false, 0, (uint8_t *)response, 1, &written, &required) == SIDEREON_STATUS_INVALID_ARGUMENT);
    bool equal = false;
    SidereonOmmElementSet elements;
    CHECK(sidereon_parse_celestrak_gp_response((const uint8_t *)body, strlen(body), 0, NULL) == SIDEREON_STATUS_NULL_POINTER);
    CHECK(sidereon_celestrak_gp_response_counts(NULL, &count, &rejected) == SIDEREON_STATUS_NULL_POINTER);
    CHECK(sidereon_celestrak_gp_response_equal(NULL, response, &equal) == SIDEREON_STATUS_NULL_POINTER);
    CHECK(sidereon_celestrak_gp_record_equal(NULL, response, false, 0, 0, &equal) == SIDEREON_STATUS_NULL_POINTER);
    CHECK(sidereon_celestrak_gp_payload_equal(NULL, response, 0, 0, &equal) == SIDEREON_STATUS_NULL_POINTER);
    CHECK(sidereon_celestrak_tle_record_equal(NULL, response, 0, 0, &equal) == SIDEREON_STATUS_NULL_POINTER);
    CHECK(sidereon_celestrak_gp_rejection_reason_equal(NULL, response, 0, 0, &equal) == SIDEREON_STATUS_NULL_POINTER);
    CHECK(sidereon_celestrak_gp_structural_error_equal(NULL, response, 0, 0, &equal) == SIDEREON_STATUS_NULL_POINTER);
    CHECK(sidereon_celestrak_gp_record_tle(NULL, 0, 0, 0, &tle) == SIDEREON_STATUS_NULL_POINTER);
    CHECK(sidereon_celestrak_gp_record_to_omm(NULL, 0, &omm) == SIDEREON_STATUS_NULL_POINTER);
    CHECK(sidereon_celestrak_gp_record_to_element_set(NULL, 0, &elements) == SIDEREON_STATUS_NULL_POINTER);
    CHECK(sidereon_celestrak_gp_response_raw(NULL, false, 0, raw, sizeof raw, &written, &required) == SIDEREON_STATUS_NULL_POINTER);
    CHECK(sidereon_celestrak_gp_response_record_json(NULL, false, 0, raw, sizeof raw, &written, &required) == SIDEREON_STATUS_NULL_POINTER);
    CHECK(sidereon_celestrak_gp_source_entry(NULL, 0, NULL, &required) == SIDEREON_STATUS_NULL_POINTER);
    CHECK(sidereon_navcen_source_entry(NULL, 0, NULL, &required) == SIDEREON_STATUS_NULL_POINTER);
    CHECK(sidereon_navcen_constellation_url(NULL, 0, NULL, &required) == SIDEREON_STATUS_NULL_POINTER);
    CHECK(sidereon_celestrak_group(0, NULL, 0, NULL, &required) == SIDEREON_STATUS_NULL_POINTER);
    CHECK(sidereon_celestrak_constellation_url(0, 0, NULL, 0, NULL, &required) == SIDEREON_STATUS_NULL_POINTER);
    CHECK(sidereon_celestrak_gp_url(0, (const uint8_t *)"active", 6, 0, NULL, 0, NULL, &required) == SIDEREON_STATUS_NULL_POINTER);
    CHECK(sidereon_celestrak_gp_catalog_number_url(25544, 0, NULL, 0, NULL, &required) == SIDEREON_STATUS_NULL_POINTER);
    CHECK(sidereon_celestrak_gp_special_url(0, 0, NULL, 0, NULL, &required) == SIDEREON_STATUS_NULL_POINTER);
    CHECK(sidereon_celestrak_gp_special_as_str(0, NULL, 0, NULL, &required) == SIDEREON_STATUS_NULL_POINTER);
    CHECK(sidereon_celestrak_gp_format_as_str(0, NULL, 0, NULL, &required) == SIDEREON_STATUS_NULL_POINTER);
    CHECK(sidereon_data_allowed_hosts(NULL, 0, NULL, &required) == SIDEREON_STATUS_NULL_POINTER);
    CHECK(sidereon_omm_from_tle(NULL, NULL, 0, &omm) == SIDEREON_STATUS_NULL_POINTER);
    CHECK(sidereon_omm_with_ccsds_header(NULL, NULL, 0, NULL, 0, NULL, 0, &omm) == SIDEREON_STATUS_NULL_POINTER);
    CHECK(sidereon_omm_to_kvn_lenient(NULL, raw, sizeof raw, &written, &required) == SIDEREON_STATUS_NULL_POINTER);
    CHECK(sidereon_omm_to_xml_lenient(NULL, raw, sizeof raw, &written, &required) == SIDEREON_STATUS_NULL_POINTER);
    CHECK(sidereon_omm_equal(NULL, NULL, &equal) == SIDEREON_STATUS_NULL_POINTER);
    CHECK(sidereon_omm_array_equal(NULL, NULL, &equal) == SIDEREON_STATUS_NULL_POINTER);
    CHECK(sidereon_omm_array_span_count(NULL, &count) == SIDEREON_STATUS_NULL_POINTER);
    SidereonOmmRecordSpan span;
    CHECK(sidereon_omm_array_span(NULL, 0, &span) == SIDEREON_STATUS_NULL_POINTER);
    CHECK(sidereon_sgp4_fit_tle_with_omm_header(NULL, 0, NULL, NULL, 0, NULL, 0, NULL) == SIDEREON_STATUS_NULL_POINTER);
    sidereon_celestrak_gp_response_free(response);
    sidereon_celestrak_gp_response_free(NULL);
    return 0;
}
