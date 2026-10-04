#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sidereon.h"

static int failures;

static void check(int condition, const char *label) {
    if (!condition) {
        fprintf(stderr, "FAIL: %s\n", label);
        failures++;
    }
}

static uint8_t *read_file(const char *path, size_t *length) {
    FILE *file = fopen(path, "rb");
    if (file == NULL || fseek(file, 0, SEEK_END) != 0) {
        if (file != NULL) fclose(file);
        return NULL;
    }
    long size = ftell(file);
    if (size < 0 || fseek(file, 0, SEEK_SET) != 0) {
        fclose(file);
        return NULL;
    }
    uint8_t *bytes = (uint8_t *)malloc((size_t)size + 1);
    if (bytes == NULL) {
        fclose(file);
        return NULL;
    }
    size_t got = fread(bytes, 1, (size_t)size, file);
    fclose(file);
    if (got != (size_t)size) {
        free(bytes);
        return NULL;
    }
    bytes[got] = 0;
    *length = got;
    return bytes;
}

static uint8_t *copy_native_bytes(
    SidereonStatus (*copy)(const SidereonOmmArray *, uint8_t *, size_t, size_t *, size_t *),
    const SidereonOmmArray *array,
    size_t *length) {
    size_t written = 0, required = 0;
    check(copy(array, NULL, 0, &written, &required) == SIDEREON_STATUS_OK && written == 0 && required != 0,
          "query OMM writer size");
    uint8_t *result = (uint8_t *)malloc(required + 1);
    check(result != NULL, "allocate OMM writer output");
    if (result == NULL) return NULL;
    check(copy(array, result, required, &written, &required) == SIDEREON_STATUS_OK && written == required,
          "copy OMM writer output");
    result[written] = 0;
    *length = written;
    return result;
}

int main(int argc, char **argv) {
    if (argc != 4) {
        fprintf(stderr, "usage: %s <omm-json> <omm-xml> <omm-kvn>\n", argv[0]);
        return 2;
    }
    size_t json_length = 0, xml_length = 0, kvn_length = 0;
    uint8_t *json_file = read_file(argv[1], &json_length);
    uint8_t *xml_file = read_file(argv[2], &xml_length);
    uint8_t *kvn_file = read_file(argv[3], &kvn_length);
    check(json_file != NULL && xml_file != NULL && kvn_file != NULL, "read OMM fixtures");
    if (json_file == NULL || xml_file == NULL || kvn_file == NULL) {
        free(json_file);
        free(xml_file);
        free(kvn_file);
        return 1;
    }

    const char *object_start = strchr((const char *)json_file, '{');
    const char *object_end = strrchr((const char *)json_file, '}');
    check(object_start != NULL && object_end != NULL && object_end >= object_start,
          "find fixture JSON object");
    if (object_start == NULL || object_end == NULL || object_end < object_start) {
        free(json_file);
        free(xml_file);
        free(kvn_file);
        return 1;
    }
    size_t object_length = (size_t)(object_end - object_start) + 1;
    size_t array_length = object_length * 2 + 9;
    char *json_array = (char *)malloc(array_length);
    check(json_array != NULL, "allocate JSON array source");
    if (json_array == NULL) {
        free(json_file);
        free(xml_file);
        free(kvn_file);
        return 1;
    }
    int json_array_written = snprintf(json_array, array_length, "[%.*s,null,%.*s]",
                                      (int)object_length, object_start,
                                      (int)object_length, object_start);
    check(json_array_written > 0 && (size_t)json_array_written < array_length,
          "construct JSON array with skipped record");
    if (json_array_written <= 0 || (size_t)json_array_written >= array_length) {
        free(json_array);
        free(json_file);
        free(xml_file);
        free(kvn_file);
        return 1;
    }

    SidereonOmm *omm = NULL;
    check(sidereon_omm_parse((const uint8_t *)object_start, object_length, &omm) == SIDEREON_STATUS_OK && omm != NULL,
          "generic parser autodetects JSON");
    SidereonOmm *csv_omm = NULL;
    static const uint8_t csv[] =
        "OBJECT_NAME,OBJECT_ID,EPOCH,MEAN_MOTION,ECCENTRICITY,INCLINATION,RA_OF_ASC_NODE,ARG_OF_PERICENTER,MEAN_ANOMALY,EPHEMERIS_TYPE,CLASSIFICATION_TYPE,NORAD_CAT_ID,ELEMENT_SET_NO,REV_AT_EPOCH,BSTAR,MEAN_MOTION_DOT,MEAN_MOTION_DDOT\n"
        "ISS (ZARYA),1998-067A,2026-06-17T04:32:52.099296,15.49273435,0.0004737,51.6332,300.0813,195.1146,164.9702,0,U,25544,999,57175,0.00017172,9.113e-5,0";
    check(sidereon_omm_parse_csv(csv, sizeof(csv) - 1, &csv_omm) == SIDEREON_STATUS_OK && csv_omm != NULL,
          "single GP CSV parser");
    SidereonOmm *csv_auto = NULL;
    check(sidereon_omm_parse(csv, sizeof(csv) - 1, &csv_auto) == SIDEREON_STATUS_OK && csv_auto != NULL,
          "generic parser autodetects CSV");
    SidereonOmm *kvn_auto = NULL;
    check(sidereon_omm_parse(kvn_file, kvn_length, &kvn_auto) == SIDEREON_STATUS_OK && kvn_auto != NULL,
          "generic parser autodetects KVN");
    SidereonOmm *xml_auto = NULL;
    check(sidereon_omm_parse(xml_file, xml_length, &xml_auto) == SIDEREON_STATUS_OK && xml_auto != NULL,
          "generic parser autodetects XML");

    SidereonOmmArray *json_records = NULL;
    check(sidereon_omm_parse_json_array((const uint8_t *)json_array, (size_t)json_array_written,
                                        &json_records) == SIDEREON_STATUS_OK && json_records != NULL,
          "JSON array parser retains successes");
    size_t count = 0, skipped_count = 0;
    check(sidereon_omm_array_count(json_records, &count) == SIDEREON_STATUS_OK && count == 2,
          "JSON array success count");
    check(sidereon_omm_array_skipped_count(json_records, &skipped_count) == SIDEREON_STATUS_OK && skipped_count == 1,
          "JSON array skipped count");
    size_t skipped_index = SIZE_MAX, written = 0, required = 0;
    check(sidereon_omm_array_skipped(json_records, 0, &skipped_index, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
              skipped_index == 1 && written == 0 && required != 0,
          "JSON array skipped source index and payload size");
    uint8_t *skip_payload = (uint8_t *)malloc(required + 1);
    check(skip_payload != NULL, "allocate skipped error payload");
    if (skip_payload != NULL) {
        check(sidereon_omm_array_skipped(json_records, 0, &skipped_index, skip_payload, required,
                                         &written, &required) == SIDEREON_STATUS_OK && written == required,
              "copy typed skipped error payload");
        skip_payload[written] = 0;
        check(strstr((const char *)skip_payload, "\"kind\":\"field\"") != NULL &&
                  strstr((const char *)skip_payload, "\"fields\"") != NULL,
              "skipped error retains full tagged fields");
        free(skip_payload);
    }
    SidereonOmm *record = NULL;
    check(sidereon_omm_array_record(json_records, 1, &record) == SIDEREON_STATUS_OK && record != NULL,
          "array record returns an independently owned OMM");
    if (record != NULL) sidereon_omm_free(record);
    size_t json_output_length = 0, csv_output_length = 0;
    uint8_t *json_output = copy_native_bytes(sidereon_omm_array_to_json, json_records, &json_output_length);
    uint8_t *json_discarded = copy_native_bytes(sidereon_omm_array_to_json_discarding_comments, json_records, &json_output_length);
    uint8_t *csv_output = copy_native_bytes(sidereon_omm_array_to_csv, json_records, &csv_output_length);
    uint8_t *csv_discarded = copy_native_bytes(sidereon_omm_array_to_csv_discarding_comments, json_records, &csv_output_length);
    check(json_output != NULL && json_discarded != NULL && csv_output != NULL && csv_discarded != NULL,
          "strict and comment-discarding array writers");
    free(json_output);
    free(json_discarded);
    free(csv_output);
    free(csv_discarded);
    (void)json_output_length;
    (void)csv_output_length;

    SidereonOmmArray *assembled = NULL;
    check(omm != NULL && sidereon_omm_array_new(&assembled) == SIDEREON_STATUS_OK && assembled != NULL,
          "create caller-owned OMM array");
    if (assembled != NULL) {
        check(sidereon_omm_array_push(assembled, omm) == SIDEREON_STATUS_OK,
              "append a cloned OMM for caller-supplied writer input");
        check(sidereon_omm_array_count(assembled, &count) == SIDEREON_STATUS_OK && count == 1,
              "caller-owned array count");
        sidereon_omm_array_free(assembled);
    }

    const char *xml_start = strstr((const char *)xml_file, "<omm ");
    const char *xml_end = strstr((const char *)xml_file, "</omm>");
    check(xml_start != NULL && xml_end != NULL && xml_end >= xml_start, "find fixture XML message");
    if (xml_start != NULL && xml_end != NULL && xml_end >= xml_start) {
        size_t message_length = (size_t)(xml_end - xml_start) + strlen("</omm>");
        size_t combined_capacity = message_length * 2 + 128;
        char *combined_xml = (char *)malloc(combined_capacity);
        check(combined_xml != NULL, "allocate combined XML source");
        if (combined_xml != NULL) {
            int used = snprintf(combined_xml, combined_capacity, "<ndm>%.*s<omm><header/><body/></omm>%.*s</ndm>",
                                (int)message_length, xml_start, (int)message_length, xml_start);
            SidereonOmmArray *xml_records = NULL;
            check(used > 0 && (size_t)used < combined_capacity &&
                      sidereon_omm_parse_xml_all((const uint8_t *)combined_xml, (size_t)used, &xml_records) == SIDEREON_STATUS_OK,
                  "XML-all parser keeps valid messages around a refusal");
            if (xml_records != NULL) {
                check(sidereon_omm_array_count(xml_records, &count) == SIDEREON_STATUS_OK && count == 2,
                      "XML-all successful message count");
                check(sidereon_omm_array_skipped_count(xml_records, &skipped_count) == SIDEREON_STATUS_OK && skipped_count == 1,
                      "XML-all skipped message count");
                sidereon_omm_array_free(xml_records);
            }
            free(combined_xml);
        }
    }

    static const uint8_t csv_array[] =
        "OBJECT_NAME,OBJECT_ID,EPOCH,MEAN_MOTION,ECCENTRICITY,INCLINATION,RA_OF_ASC_NODE,ARG_OF_PERICENTER,MEAN_ANOMALY,EPHEMERIS_TYPE,CLASSIFICATION_TYPE,NORAD_CAT_ID,ELEMENT_SET_NO,REV_AT_EPOCH,BSTAR,MEAN_MOTION_DOT,MEAN_MOTION_DDOT\n"
        "ISS (ZARYA),1998-067A,2026-06-17T04:32:52.099296,15.49273435,0.0004737,51.6332,300.0813,195.1146,164.9702,0,U,25544,999,57175,0.00017172,9.113e-5,0\n"
        "bad,row\n";
    SidereonOmmArray *csv_records = NULL;
    check(sidereon_omm_parse_csv_array(csv_array, sizeof(csv_array) - 1, &csv_records) == SIDEREON_STATUS_OK && csv_records != NULL,
          "CSV array parser keeps valid rows");
    if (csv_records != NULL) {
        check(sidereon_omm_array_count(csv_records, &count) == SIDEREON_STATUS_OK && count == 1,
              "CSV array success count");
        check(sidereon_omm_array_skipped_count(csv_records, &skipped_count) == SIDEREON_STATUS_OK && skipped_count == 1,
              "CSV array skipped-row count");
        skipped_index = SIZE_MAX;
        written = required = 0;
        check(sidereon_omm_array_skipped(csv_records, 0, &skipped_index, NULL, 0,
                                         &written, &required) == SIDEREON_STATUS_OK && skipped_index == 1,
              "CSV array retains zero-based row index");
        sidereon_omm_array_free(csv_records);
    }

    SidereonOmmEpoch epoch = {0};
    static const uint8_t epoch_text[] = "2016-12-31T23:59:60.123456789123456";
    check(sidereon_omm_parse_epoch(epoch_text, sizeof(epoch_text) - 1, &epoch) == SIDEREON_STATUS_OK &&
              epoch.year == 2016 && epoch.month == 12 && epoch.day == 31 && epoch.second == 60 &&
              epoch.microsecond == 123456 && epoch.femtosecond == 789123456,
          "standalone epoch keeps leap-second and femtosecond fields");

    SidereonOmmElementSet elements = {0};
    check(omm != NULL && sidereon_omm_to_element_set(omm, &elements) == SIDEREON_STATUS_OK,
          "bridge OMM element epoch");
    SidereonSgp4Satellite *satellite = NULL;
    check(omm != NULL && sidereon_sgp4_satellite_from_omm(omm, &satellite) == SIDEREON_STATUS_OK && satellite != NULL,
          "initialize SGP4 satellite directly from OMM");
    if (satellite != NULL) {
        double epoch_whole = -1.0, epoch_fraction = -1.0, epoch_j2000_s = -1.0;
        check(sidereon_sgp4_satellite_epoch_jd(satellite, &epoch_whole, &epoch_fraction) == SIDEREON_STATUS_OK,
              "read exact split initialized SGP4 epoch");
        check(epoch_whole == elements.epoch_whole && epoch_fraction == elements.epoch_fraction,
              "SGP4 epoch getter retains exact split components");
        check(sidereon_sgp4_satellite_epoch_j2000_s(satellite, &epoch_j2000_s) == SIDEREON_STATUS_OK && isfinite(epoch_j2000_s),
              "read exact initialized SGP4 epoch");
        SidereonTemeState at_minutes = {0}, at_jd = {0};
        check(sidereon_sgp4_satellite_propagate_minutes(satellite, 0.0, &at_minutes) == SIDEREON_STATUS_OK,
              "propagate OMM satellite by minutes");
        check(sidereon_sgp4_satellite_propagate_jd(satellite, elements.epoch_whole, elements.epoch_fraction, &at_jd) == SIDEREON_STATUS_OK,
              "propagate OMM satellite by split Julian date");
        check(memcmp(&at_minutes, &at_jd, sizeof(at_minutes)) == 0,
              "minutes and split-JD propagators agree at element epoch");
        sidereon_sgp4_satellite_free(satellite);
    }

    if (omm != NULL) sidereon_omm_free(omm);
    if (csv_omm != NULL) sidereon_omm_free(csv_omm);
    if (csv_auto != NULL) sidereon_omm_free(csv_auto);
    if (kvn_auto != NULL) sidereon_omm_free(kvn_auto);
    if (xml_auto != NULL) sidereon_omm_free(xml_auto);
    if (json_records != NULL) sidereon_omm_array_free(json_records);
    free(json_array);
    free(json_file);
    free(xml_file);
    free(kvn_file);
    if (failures != 0) return 1;
    puts("OMM array and SGP4 C API smoke passed");
    return 0;
}
