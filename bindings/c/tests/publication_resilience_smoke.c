/* Publication-lag resilience surface (core 0.36.0) through the C ABI.
 *
 * The listing rows below are real records from AIUB's whole-tree CSV as
 * recorded live on 2026-08-04, when the one-day predicted ionosphere line's
 * newest map was day 216 while the two-day line already published day 217 -
 * the archive state the cross-line candidate walk exists for.
 */
#include "sidereon.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static const char AIUB_LISTING[] =
    "CODE/IONO/P1/2026/COD0OPSPRD_20262150000_01D_01H_GIM.INX.gz;192455;"
    "2026-08-03T07:09:51Z;41895730d158d884f98a1e0a88cf267e\n"
    "CODE/IONO/P1/2026/COD0OPSPRD_20262160000_01D_01H_GIM.INX.gz;187029;"
    "2026-08-04T06:51:14Z;ca54cbde63323584e040641202b4aa79\n"
    "CODE/IONO/P2/2026/COD0OPSPRD_20262160000_01D_01H_GIM.INX.gz;189076;"
    "2026-08-03T07:09:52Z;c46b8e4b33be2fac60eb72c061cffe1a\n"
    "CODE/IONO/P2/2026/COD0OPSPRD_20262170000_01D_01H_GIM.INX.gz;185825;"
    "2026-08-04T06:51:15Z;ca33b1eccb3959d36c9c631b6b18ffaa\n";

static int fail(const char *what) {
    char detail[512] = {0};
    sidereon_last_error_message(detail, sizeof(detail));
    fprintf(stderr, "FAIL %s: %s\n", what, detail);
    return 1;
}

int main(void) {
    uint8_t out[4096];
    size_t written = 0;
    size_t required = 0;

    /* Cross-line candidates for map date 2026-08-05 (day 217): both lines,
     * same map date, P1 first. */
    if (sidereon_data_predicted_ionex_line_candidates_json(
            2026, 8, 5, NULL, out, sizeof(out), &written, &required) !=
        SIDEREON_STATUS_OK) {
        return fail("predicted_ionex_line_candidates_json");
    }
    if (written == 0 || written != required || written >= sizeof(out)) {
        return fail("candidate JSON byte contract");
    }
    out[written] = '\0';
    if (strstr((const char *)out, "\"center\":\"cod_prd1\"") == NULL ||
        strstr((const char *)out, "\"center\":\"cod_prd2\"") == NULL ||
        strstr((const char *)out, "\"date\":\"2026-08-05\"") == NULL ||
        strstr((const char *)out,
               "/IONO/P2/2026/COD0OPSPRD_20262170000_01D_01H_GIM.INX.gz") == NULL) {
        return fail("candidate JSON content");
    }
    if (strstr((const char *)out, "\"date\":\"2026-08-04\"") != NULL ||
        strstr((const char *)out, "\"date\":\"2026-08-06\"") != NULL) {
        return fail("the walk must never substitute a neighboring map date");
    }

    /* Newest published issue per line from the recorded listing: P1 tops out
     * at day 216 while P2 already has day 217. */
    if (sidereon_data_newest_published_product_json(
            "cod_prd1", SIDEREON_PRODUCT_FAMILY_IONEX, AIUB_LISTING, out,
            sizeof(out), &written, &required) != SIDEREON_STATUS_OK) {
        return fail("newest_published_product_json cod_prd1");
    }
    out[written] = '\0';
    if (strstr((const char *)out, "\"date\":\"2026-08-04\"") == NULL ||
        strstr((const char *)out, "\"observed_at\":\"2026-08-04T06:51:14Z\"") == NULL) {
        return fail("cod_prd1 newest content");
    }

    if (sidereon_data_newest_published_product_json(
            "cod_prd2", SIDEREON_PRODUCT_FAMILY_IONEX, AIUB_LISTING, out,
            sizeof(out), &written, &required) != SIDEREON_STATUS_OK) {
        return fail("newest_published_product_json cod_prd2");
    }
    out[written] = '\0';
    if (strstr((const char *)out, "\"date\":\"2026-08-05\"") == NULL) {
        return fail("cod_prd2 newest content");
    }

    /* Closed dialect detection: an error page is an error status, never an
     * empty parse. */
    if (sidereon_data_newest_published_product_json(
            "cod_prd1", SIDEREON_PRODUCT_FAMILY_IONEX,
            "<html><h1>503 Service Unavailable</h1></html>", out, sizeof(out),
            &written, &required) == SIDEREON_STATUS_OK) {
        fprintf(stderr, "FAIL unrecognized listing must not parse\n");
        return 1;
    }

    /* Bounded listing URLs, newest directory first. */
    if (sidereon_data_publication_listing_urls_json(
            "gfz_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2026, 8, 4, out,
            sizeof(out), &written, &required) != SIDEREON_STATUS_OK) {
        return fail("publication_listing_urls_json");
    }
    out[written] = '\0';
    if (strcmp((const char *)out,
               "[\"https://isdc-data.gfz.de/gnss/products/ultra/w2430/\","
               "\"https://isdc-data.gfz.de/gnss/products/ultra/w2429/\"]") != 0) {
        return fail("listing URLs content");
    }

    /* The Wuhan NRT line is cataloged: solution class round-trips through the
     * new enum value. */
    enum SidereonSolutionClass solution = SIDEREON_SOLUTION_CLASS_FINAL;
    if (sidereon_data_product_solution_class(
            "wum_nrt", SIDEREON_PRODUCT_FAMILY_SP3, &solution) !=
            SIDEREON_STATUS_OK ||
        solution != SIDEREON_SOLUTION_CLASS_NEAR_REAL_TIME) {
        return fail("wum_nrt solution class");
    }

    printf("publication resilience smoke: OK\n");
    return 0;
}
