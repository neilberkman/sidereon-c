/*
 * Focused C-ABI exercise for the newly exposed serializers and CCSDS readers:
 *   1. CCSDS OEM reader + writer  (sidereon_oem_parse_kvn/xml,
 *      sidereon_oem_to_kvn/xml, sidereon_oem_segment_count, sidereon_oem_free)
 *   2. CCSDS OPM reader + writer  (sidereon_opm_parse_kvn/xml,
 *      sidereon_opm_to_kvn/xml, sidereon_opm_free)
 *   3. IONEX serializer  (sidereon_ionex_to_ionex_text)
 *   4. RINEX 3 observation serializer  (sidereon_rinex_obs_to_rinex_text)
 *
 * Compiled with -std=c11 -Wall -Wextra -Werror by run_smoke.sh. argv carries the
 * fixture paths:
 *   argv[1] OEM KVN   argv[2] OEM XML   argv[3] OPM KVN   argv[4] OPM XML
 *   argv[5] IONEX     argv[6] RINEX 3 observation text
 * Exits 0 on success.
 */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sidereon.h"

static int fail(const char *context) {
    size_t needed = sidereon_last_error_message(NULL, 0);
    char *msg = (char *)malloc(needed + 1);
    if (msg != NULL) {
        sidereon_last_error_message(msg, needed + 1);
        fprintf(stderr, "FAIL: %s: %s\n", context, msg);
        free(msg);
    } else {
        fprintf(stderr, "FAIL: %s\n", context);
    }
    return 1;
}

static uint8_t *read_file(const char *path, size_t *len) {
    FILE *f = fopen(path, "rb");
    if (f == NULL) {
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
    uint8_t *buf = (uint8_t *)malloc((size_t)size);
    if (buf == NULL) {
        fclose(f);
        return NULL;
    }
    size_t got = fread(buf, 1, (size_t)size, f);
    fclose(f);
    if (got != (size_t)size) {
        free(buf);
        return NULL;
    }
    *len = got;
    return buf;
}

/* Signature shared by every variable-length string-output entry point. */
typedef SidereonStatus (*serialize_fn)(const void *handle, uint8_t *out, size_t len,
                                       size_t *out_written, size_t *out_required);

/*
 * Run the two-call query-then-fill dance against one of the new serializers:
 * call once with out=NULL to learn the required length, allocate, then fill.
 * Verifies the documented contract (size query writes zero, exact fill reports
 * matching written/required). Returns a malloc'd buffer (caller frees) and its
 * length through out_len, or NULL on failure.
 */
static uint8_t *serialize(serialize_fn fn, const void *handle, const char *context,
                          size_t *out_len) {
    size_t written = 123;
    size_t required = 123;
    if (fn(handle, NULL, 0, &written, &required) != SIDEREON_STATUS_OK || written != 0) {
        (void)fail(context);
        return NULL;
    }
    uint8_t *buf = (uint8_t *)malloc(required);
    if (buf == NULL) {
        fprintf(stderr, "FAIL: %s: allocation\n", context);
        return NULL;
    }
    written = 123;
    size_t required2 = 123;
    if (fn(handle, buf, required, &written, &required2) != SIDEREON_STATUS_OK ||
        written != required || required2 != required) {
        free(buf);
        (void)fail(context);
        return NULL;
    }
    *out_len = written;
    return buf;
}

/* Thin typed wrappers so the generic serialize() can drive each entry point. */
static SidereonStatus oem_to_kvn(const void *h, uint8_t *o, size_t n, size_t *w, size_t *r) {
    return sidereon_oem_to_kvn((const SidereonOem *)h, o, n, w, r);
}
static SidereonStatus oem_to_xml(const void *h, uint8_t *o, size_t n, size_t *w, size_t *r) {
    return sidereon_oem_to_xml((const SidereonOem *)h, o, n, w, r);
}
static SidereonStatus opm_to_kvn(const void *h, uint8_t *o, size_t n, size_t *w, size_t *r) {
    return sidereon_opm_to_kvn((const SidereonOpm *)h, o, n, w, r);
}
static SidereonStatus opm_to_xml(const void *h, uint8_t *o, size_t n, size_t *w, size_t *r) {
    return sidereon_opm_to_xml((const SidereonOpm *)h, o, n, w, r);
}
static SidereonStatus ionex_to_text(const void *h, uint8_t *o, size_t n, size_t *w, size_t *r) {
    return sidereon_ionex_to_ionex_text((const SidereonIonex *)h, o, n, w, r);
}
static SidereonStatus rinex_to_text(const void *h, uint8_t *o, size_t n, size_t *w, size_t *r) {
    return sidereon_rinex_obs_to_rinex_text((const SidereonRinexObs *)h, o, n, w, r);
}

static int exercise_oem(const char *kvn_path, const char *xml_path) {
    int rc = 1;
    uint8_t *kvn = NULL;
    uint8_t *xml = NULL;
    SidereonOem *from_kvn = NULL;
    SidereonOem *from_xml = NULL;
    SidereonOem *reparsed = NULL;
    uint8_t *out = NULL;

    size_t kvn_len = 0;
    size_t xml_len = 0;
    kvn = read_file(kvn_path, &kvn_len);
    xml = read_file(xml_path, &xml_len);
    if (kvn == NULL || xml == NULL) {
        fprintf(stderr, "FAIL: could not read OEM fixtures\n");
        goto cleanup;
    }

    if (sidereon_oem_parse_kvn(kvn, kvn_len, &from_kvn) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_oem_parse_kvn");
        goto cleanup;
    }
    if (sidereon_oem_parse_xml(xml, xml_len, &from_xml) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_oem_parse_xml");
        goto cleanup;
    }

    size_t segs_kvn = 0;
    size_t segs_xml = 0;
    if (sidereon_oem_segment_count(from_kvn, &segs_kvn) != SIDEREON_STATUS_OK ||
        sidereon_oem_segment_count(from_xml, &segs_xml) != SIDEREON_STATUS_OK ||
        segs_kvn == 0 || segs_kvn != segs_xml) {
        rc = fail("sidereon_oem_segment_count");
        goto cleanup;
    }

    /* KVN round-trip: KVN -> handle -> KVN -> handle, segment count preserved. */
    size_t out_len = 0;
    out = serialize(oem_to_kvn, from_kvn, "sidereon_oem_to_kvn", &out_len);
    if (out == NULL) {
        goto cleanup;
    }
    if (sidereon_oem_parse_kvn(out, out_len, &reparsed) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_oem_to_kvn roundtrip parse");
        goto cleanup;
    }
    size_t segs_rt = 0;
    if (sidereon_oem_segment_count(reparsed, &segs_rt) != SIDEREON_STATUS_OK ||
        segs_rt != segs_kvn) {
        rc = fail("sidereon_oem_to_kvn roundtrip segment count");
        goto cleanup;
    }
    sidereon_oem_free(reparsed);
    reparsed = NULL;
    free(out);

    /* Cross-encoding: KVN handle -> XML -> handle, segment count preserved. */
    out = serialize(oem_to_xml, from_kvn, "sidereon_oem_to_xml", &out_len);
    if (out == NULL) {
        goto cleanup;
    }
    if (sidereon_oem_parse_xml(out, out_len, &reparsed) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_oem_to_xml roundtrip parse");
        goto cleanup;
    }
    segs_rt = 0;
    if (sidereon_oem_segment_count(reparsed, &segs_rt) != SIDEREON_STATUS_OK ||
        segs_rt != segs_kvn) {
        rc = fail("sidereon_oem_to_xml roundtrip segment count");
        goto cleanup;
    }

    rc = 0;
cleanup:
    sidereon_oem_free(reparsed);
    sidereon_oem_free(from_kvn);
    sidereon_oem_free(from_xml);
    free(out);
    free(kvn);
    free(xml);
    return rc;
}

static int exercise_opm(const char *kvn_path, const char *xml_path) {
    int rc = 1;
    uint8_t *kvn = NULL;
    uint8_t *xml = NULL;
    SidereonOpm *from_kvn = NULL;
    SidereonOpm *from_xml = NULL;
    SidereonOpm *reparsed = NULL;
    uint8_t *out = NULL;

    size_t kvn_len = 0;
    size_t xml_len = 0;
    kvn = read_file(kvn_path, &kvn_len);
    xml = read_file(xml_path, &xml_len);
    if (kvn == NULL || xml == NULL) {
        fprintf(stderr, "FAIL: could not read OPM fixtures\n");
        goto cleanup;
    }

    if (sidereon_opm_parse_kvn(kvn, kvn_len, &from_kvn) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_opm_parse_kvn");
        goto cleanup;
    }
    if (sidereon_opm_parse_xml(xml, xml_len, &from_xml) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_opm_parse_xml");
        goto cleanup;
    }

    /* KVN round-trip. */
    size_t out_len = 0;
    out = serialize(opm_to_kvn, from_kvn, "sidereon_opm_to_kvn", &out_len);
    if (out == NULL) {
        goto cleanup;
    }
    if (sidereon_opm_parse_kvn(out, out_len, &reparsed) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_opm_to_kvn roundtrip parse");
        goto cleanup;
    }
    sidereon_opm_free(reparsed);
    reparsed = NULL;
    free(out);

    /* Cross-encoding: KVN handle -> XML -> handle. */
    out = serialize(opm_to_xml, from_kvn, "sidereon_opm_to_xml", &out_len);
    if (out == NULL) {
        goto cleanup;
    }
    if (sidereon_opm_parse_xml(out, out_len, &reparsed) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_opm_to_xml roundtrip parse");
        goto cleanup;
    }

    rc = 0;
cleanup:
    sidereon_opm_free(reparsed);
    sidereon_opm_free(from_kvn);
    sidereon_opm_free(from_xml);
    free(out);
    free(kvn);
    free(xml);
    return rc;
}

static int exercise_ionex(const char *path) {
    int rc = 1;
    uint8_t *data = NULL;
    SidereonIonex *ionex = NULL;
    SidereonIonex *reparsed = NULL;
    uint8_t *out = NULL;

    size_t len = 0;
    data = read_file(path, &len);
    if (data == NULL) {
        fprintf(stderr, "FAIL: could not read IONEX fixture\n");
        goto cleanup;
    }
    if (sidereon_ionex_parse(data, len, &ionex) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_ionex_parse");
        goto cleanup;
    }
    size_t epochs = 0;
    if (sidereon_ionex_epoch_count(ionex, &epochs) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_ionex_epoch_count");
        goto cleanup;
    }

    size_t out_len = 0;
    out = serialize(ionex_to_text, ionex, "sidereon_ionex_to_ionex_text", &out_len);
    if (out == NULL) {
        goto cleanup;
    }
    if (sidereon_ionex_parse(out, out_len, &reparsed) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_ionex_to_ionex_text roundtrip parse");
        goto cleanup;
    }
    size_t epochs_rt = 0;
    if (sidereon_ionex_epoch_count(reparsed, &epochs_rt) != SIDEREON_STATUS_OK ||
        epochs_rt != epochs) {
        rc = fail("sidereon_ionex_to_ionex_text roundtrip epoch count");
        goto cleanup;
    }

    rc = 0;
cleanup:
    sidereon_ionex_free(reparsed);
    sidereon_ionex_free(ionex);
    free(out);
    free(data);
    return rc;
}

static int exercise_rinex(const char *path) {
    int rc = 1;
    uint8_t *data = NULL;
    SidereonRinexObs *obs = NULL;
    SidereonRinexObs *reparsed = NULL;
    uint8_t *out = NULL;

    size_t len = 0;
    data = read_file(path, &len);
    if (data == NULL) {
        fprintf(stderr, "FAIL: could not read RINEX fixture\n");
        goto cleanup;
    }
    if (sidereon_rinex_obs_parse(data, len, &obs) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_rinex_obs_parse");
        goto cleanup;
    }
    double version = 0.0;
    if (sidereon_rinex_obs_version(obs, &version) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_rinex_obs_version");
        goto cleanup;
    }

    size_t out_len = 0;
    out = serialize(rinex_to_text, obs, "sidereon_rinex_obs_to_rinex_text", &out_len);
    if (out == NULL) {
        goto cleanup;
    }
    if (sidereon_rinex_obs_parse(out, out_len, &reparsed) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_rinex_obs_to_rinex_text roundtrip parse");
        goto cleanup;
    }
    double version_rt = 0.0;
    if (sidereon_rinex_obs_version(reparsed, &version_rt) != SIDEREON_STATUS_OK ||
        version_rt != version) {
        rc = fail("sidereon_rinex_obs_to_rinex_text roundtrip version");
        goto cleanup;
    }

    rc = 0;
cleanup:
    sidereon_rinex_obs_free(reparsed);
    sidereon_rinex_obs_free(obs);
    free(out);
    free(data);
    return rc;
}

int main(int argc, char **argv) {
    if (argc != 7) {
        fprintf(stderr,
                "usage: %s OEM_KVN OEM_XML OPM_KVN OPM_XML IONEX RINEX_OBS\n",
                argv[0]);
        return 2;
    }
    if (exercise_oem(argv[1], argv[2]) != 0) {
        return 1;
    }
    if (exercise_opm(argv[3], argv[4]) != 0) {
        return 1;
    }
    if (exercise_ionex(argv[5]) != 0) {
        return 1;
    }
    if (exercise_rinex(argv[6]) != 0) {
        return 1;
    }
    printf("ccsds_serialize: OK\n");
    return 0;
}
