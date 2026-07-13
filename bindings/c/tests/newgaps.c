/*
 * Focused C-ABI exercise for the parity-gap closes:
 *   1. lenient OMM catalog  (sidereon_omm_catalog_*)
 *   2. standalone LAMBDA / bounded ILS  (sidereon_lambda_ils_search,
 *      sidereon_bounded_ils_search)
 *   3. SP3 merge agreement metric  (sidereon_sp3_merge_report_epoch_agreement*,
 *      sidereon_sp3_merge_report_agreement_summary)
 *
 * Compiled with -std=c11 -Wall -Wextra -Werror by run_smoke.sh. argv[1] is an
 * SP3 fixture path (used only by the agreement section). Exits 0 on success.
 */
#include <float.h>
#include <math.h>
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

/* One CelesTrak OMM JSON object with the required mean-element fields. */
#define OMM_OBJ(name, norad)                                                    \
    "{\"OBJECT_NAME\":\"" name "\",\"EPOCH\":\"2020-06-25T00:00:00.000000\","   \
    "\"MEAN_MOTION\":2.0056,\"ECCENTRICITY\":0.0001,\"INCLINATION\":55.0,"      \
    "\"RA_OF_ASC_NODE\":100.0,\"ARG_OF_PERICENTER\":50.0,\"MEAN_ANOMALY\":10.0,"\
    "\"NORAD_CAT_ID\":" norad ",\"BSTAR\":0.0,\"MEAN_MOTION_DOT\":0.0,"         \
    "\"MEAN_MOTION_DDOT\":0.0}"

static const char *const OMM_FEED =
    "[" OMM_OBJ("GPS BIIF-8  (PRN 03)", "40294") ","
    OMM_OBJ("GPS BIII-1  (PRN 04)", "43873") ","
    OMM_OBJ("QZS-2 (QZSS/PRN 194)", "42738") "]";

static int exercise_omm_lenient(void) {
    SidereonOmmCatalog *catalog = NULL;
    int rc = 1;

    if (sidereon_omm_catalog_build_lenient(SIDEREON_GNSS_SYSTEM_GPS,
                                           (const uint8_t *)OMM_FEED,
                                           strlen(OMM_FEED), &catalog) !=
        SIDEREON_STATUS_OK) {
        return fail("sidereon_omm_catalog_build_lenient");
    }

    size_t record_count = 123;
    if (sidereon_omm_catalog_record_count(catalog, &record_count) != SIDEREON_STATUS_OK ||
        record_count != 2) {
        rc = fail("sidereon_omm_catalog_record_count");
        goto cleanup;
    }
    /* Records are sorted by (system, prn): G03 then G04. */
    SidereonConstellationRecord rec0;
    SidereonConstellationRecord rec1;
    if (sidereon_omm_catalog_record(catalog, 0, &rec0) != SIDEREON_STATUS_OK ||
        sidereon_omm_catalog_record(catalog, 1, &rec1) != SIDEREON_STATUS_OK ||
        rec0.system != SIDEREON_GNSS_SYSTEM_GPS || rec0.prn != 3 || rec0.norad_id != 40294 ||
        rec1.system != SIDEREON_GNSS_SYSTEM_GPS || rec1.prn != 4 || rec1.norad_id != 43873) {
        rc = fail("sidereon_omm_catalog_record values");
        goto cleanup;
    }
    if (sidereon_omm_catalog_record(catalog, 2, &rec0) != SIDEREON_STATUS_INVALID_ARGUMENT) {
        rc = fail("sidereon_omm_catalog_record out-of-range index");
        goto cleanup;
    }

    size_t skipped_count = 123;
    if (sidereon_omm_catalog_skipped_count(catalog, &skipped_count) != SIDEREON_STATUS_OK ||
        skipped_count != 1) {
        rc = fail("sidereon_omm_catalog_skipped_count");
        goto cleanup;
    }
    /* The feed parsed cleanly: no malformed JSON elements. */
    size_t malformed_count = 123;
    if (sidereon_omm_catalog_malformed_count(catalog, &malformed_count) != SIDEREON_STATUS_OK ||
        malformed_count != 0) {
        rc = fail("sidereon_omm_catalog_malformed_count clean feed");
        goto cleanup;
    }
    SidereonSkippedOmm skipped;
    if (sidereon_omm_catalog_skipped(catalog, 0, &skipped) != SIDEREON_STATUS_OK ||
        skipped.norad_id != 42738 || !skipped.object_name_present) {
        rc = fail("sidereon_omm_catalog_skipped values");
        goto cleanup;
    }

    const char *expected_name = "QZS-2 (QZSS/PRN 194)";
    size_t written = 123;
    size_t required = 123;
    if (sidereon_omm_catalog_skipped_object_name(catalog, 0, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != 0 || required != strlen(expected_name)) {
        rc = fail("sidereon_omm_catalog_skipped_object_name size query");
        goto cleanup;
    }
    char name[64];
    if (sidereon_omm_catalog_skipped_object_name(catalog, 0, (uint8_t *)name, sizeof(name),
                                                 &written, &required) != SIDEREON_STATUS_OK ||
        written != strlen(expected_name) || required != strlen(expected_name) ||
        memcmp(name, expected_name, written) != 0) {
        rc = fail("sidereon_omm_catalog_skipped_object_name copy");
        goto cleanup;
    }
    if (sidereon_omm_catalog_skipped(catalog, 1, &skipped) != SIDEREON_STATUS_INVALID_ARGUMENT) {
        rc = fail("sidereon_omm_catalog_skipped out-of-range index");
        goto cleanup;
    }

    printf("OMM lenient: %zu GPS records, %zu skipped (%s)\n", record_count, skipped_count,
           expected_name);
    rc = 0;

cleanup:
    sidereon_omm_catalog_free(catalog);
    if (rc != 0) {
        return rc;
    }

    /* A feed with a non-object element (a bare number) parses leniently: the one
     * GPS object resolves, the malformed element is counted, distinguishable from
     * an empty feed. */
    static const char *const bad_feed = "[42," OMM_OBJ("GPS BIIF-8  (PRN 03)", "40294") "]";
    SidereonOmmCatalog *bad_catalog = NULL;
    if (sidereon_omm_catalog_build_lenient(SIDEREON_GNSS_SYSTEM_GPS, (const uint8_t *)bad_feed,
                                           strlen(bad_feed), &bad_catalog) != SIDEREON_STATUS_OK) {
        return fail("sidereon_omm_catalog_build_lenient malformed feed");
    }
    size_t bad_records = 123;
    size_t bad_malformed = 123;
    if (sidereon_omm_catalog_record_count(bad_catalog, &bad_records) != SIDEREON_STATUS_OK ||
        sidereon_omm_catalog_malformed_count(bad_catalog, &bad_malformed) != SIDEREON_STATUS_OK ||
        bad_records != 1 || bad_malformed != 1) {
        rc = fail("sidereon_omm_catalog_malformed_count malformed feed");
    } else {
        rc = 0;
    }
    sidereon_omm_catalog_free(bad_catalog);
    return rc;
}

static int exercise_lambda(void) {
    /* RTKLIB lambda() utest1: a weakly-correlated 6-ambiguity case. */
    const double a[6] = {1585184.171,  -6716599.430, 3915742.905,
                         7627233.455,  9565990.879,  989457273.200};
    const double q[36] = {
        0.227134, 0.112202, 0.112202, 0.112202, 0.112202, 0.103473,
        0.112202, 0.227134, 0.112202, 0.112202, 0.112202, 0.103473,
        0.112202, 0.112202, 0.227134, 0.112202, 0.112202, 0.103473,
        0.112202, 0.112202, 0.112202, 0.227134, 0.112202, 0.103473,
        0.112202, 0.112202, 0.112202, 0.112202, 0.227134, 0.103473,
        0.103473, 0.103473, 0.103473, 0.103473, 0.103473, 0.434339};
    const int64_t expected[6] = {1585184, -6716599, 3915743, 7627234, 9565991, 989457273};

    int64_t fixed[6] = {0};
    SidereonIlsResult result;
    if (sidereon_lambda_ils_search(a, 6, q, 36, 3.0, fixed, &result) != SIDEREON_STATUS_OK) {
        return fail("sidereon_lambda_ils_search");
    }
    if (memcmp(fixed, expected, sizeof(expected)) != 0) {
        return fail("sidereon_lambda_ils_search fixed vector");
    }
    if (fabs(result.best_score - 3.5079844392) > 1e-4 || !result.second_best_present ||
        fabs(result.second_best_score - 3.70845619249) > 1e-4) {
        return fail("sidereon_lambda_ils_search scores");
    }
    /* ratio (~1.06) is well below the 3.0 threshold, so the fix is not accepted. */
    if (result.fixed_status) {
        return fail("sidereon_lambda_ils_search ratio verdict");
    }

    /* Dimension-mismatch covariance is rejected, not a panic. */
    if (sidereon_lambda_ils_search(a, 6, q, 35, 3.0, fixed, &result) !=
        SIDEREON_STATUS_INVALID_ARGUMENT) {
        return fail("sidereon_lambda_ils_search dimension mismatch");
    }

    /* A finite ambiguity outside int64_t's output domain is rejected instead
     * of saturating the integer result and returning non-finite scores. */
    const double outside_integer_domain[1] = {DBL_MAX};
    const double identity_covariance[1] = {1.0};
    int64_t outside_fixed[1] = {0};
    if (sidereon_lambda_ils_search(outside_integer_domain, 1, identity_covariance, 1, 3.0,
                                  outside_fixed, &result) != SIDEREON_STATUS_INVALID_ARGUMENT) {
        return fail("sidereon_lambda_ils_search integer output domain");
    }

    /* bounded_ils_search on a trivial diagonal case rounds componentwise. */
    const double bf[2] = {0.1, 0.9};
    const double bcov[4] = {1.0, 0.0, 0.0, 1.0};
    const int64_t bexpected[2] = {0, 1};
    int64_t bfixed[2] = {0};
    SidereonIlsResult bresult;
    if (sidereon_bounded_ils_search(bf, 2, bcov, 4, 1, 200000, 3.0, bfixed, &bresult) !=
        SIDEREON_STATUS_OK) {
        return fail("sidereon_bounded_ils_search");
    }
    if (memcmp(bfixed, bexpected, sizeof(bexpected)) != 0) {
        return fail("sidereon_bounded_ils_search fixed vector");
    }

    printf("LAMBDA: 6-amb fix matches RTKLIB utest1 (best=%.6f, ratio=%.4f), "
           "bounded 2-amb fix [%lld,%lld]\n",
           result.best_score, result.ratio, (long long)bfixed[0], (long long)bfixed[1]);
    return 0;
}

static int exercise_agreement(const char *sp3_path) {
    size_t len = 0;
    uint8_t *bytes = read_file(sp3_path, &len);
    if (bytes == NULL) {
        fprintf(stderr, "FAIL: could not read SP3 file: %s\n", sp3_path);
        return 2;
    }
    SidereonSp3 *sp3 = NULL;
    SidereonSp3 *merged = NULL;
    SidereonSp3MergeReport *report = NULL;
    int rc = 1;

    if (sidereon_sp3_load(bytes, len, &sp3) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_sp3_load");
        goto cleanup;
    }
    SidereonSp3MergeOptions options;
    if (sidereon_sp3_merge_options_init(&options) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_sp3_merge_options_init");
        goto cleanup;
    }
    options.min_agree = 1;
    options.clock_min_common = 1;
    const SidereonSp3 *sources[2] = {sp3, sp3};
    if (sidereon_sp3_merge(sources, 2, &options, &merged, &report) != SIDEREON_STATUS_OK) {
        rc = fail("sidereon_sp3_merge");
        goto cleanup;
    }

    size_t epoch_count = 0;
    if (sidereon_sp3_merge_report_epoch_agreement_count(report, &epoch_count) !=
            SIDEREON_STATUS_OK ||
        epoch_count == 0) {
        rc = fail("sidereon_sp3_merge_report_epoch_agreement_count");
        goto cleanup;
    }
    SidereonSp3EpochAgreement first;
    if (sidereon_sp3_merge_report_epoch_agreement(report, 0, &first) != SIDEREON_STATUS_OK ||
        !isfinite(first.epoch_j2000_seconds) || first.position_rms_m != 0.0) {
        rc = fail("sidereon_sp3_merge_report_epoch_agreement");
        goto cleanup;
    }
    SidereonSp3AgreementSummary summary;
    if (sidereon_sp3_merge_report_agreement_summary(report, &summary) != SIDEREON_STATUS_OK ||
        !summary.position_rms_present || summary.position_rms_m != 0.0) {
        rc = fail("sidereon_sp3_merge_report_agreement_summary");
        goto cleanup;
    }

    printf("SP3 agreement: %zu epochs, identical-source position RMS %.3f m\n", epoch_count,
           summary.position_rms_m);
    rc = 0;

cleanup:
    sidereon_sp3_merge_report_free(report);
    sidereon_sp3_free(merged);
    sidereon_sp3_free(sp3);
    free(bytes);
    return rc;
}

int main(int argc, char **argv) {
    if (argc < 2) {
        fprintf(stderr, "usage: %s <sp3-path>\n", argv[0]);
        return 2;
    }
    int rc = exercise_omm_lenient();
    if (rc != 0) {
        return rc;
    }
    rc = exercise_lambda();
    if (rc != 0) {
        return rc;
    }
    rc = exercise_agreement(argv[1]);
    if (rc != 0) {
        return rc;
    }
    printf("newgaps: all parity-gap closes OK\n");
    return 0;
}
