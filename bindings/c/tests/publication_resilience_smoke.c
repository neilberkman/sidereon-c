/* Publication-lag resilience surface (core 0.36.0) through the C ABI.
 *
 * The listing rows below are real records from AIUB's whole-tree CSV as
 * recorded live on 2026-09-23, when the one-day predicted ionosphere line's
 * newest map under CODE/IONO/PRD/ was day 265 while the two-day line already
 * published day 266 - the archive state the cross-line candidate walk exists
 * for.
 */
#include "sidereon.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static const char AIUB_LISTING[] =
    "CODE/IONO/PRD/COD0OPSP0D_20262640000_01D_01H_GIM.INX.gz;194613;"
    "2026-09-21T10:23:03Z;bf99b9bd1323d4425415edcaef3bd29a\n"
    "CODE/IONO/PRD/COD0OPSP0D_20262650000_01D_01H_GIM.INX.gz;195173;"
    "2026-09-22T10:00:02Z;617af90a2d509d1bda329b3cd098d46d\n"
    "CODE/IONO/PRD/COD0OPSP1D_20262650000_01D_01H_GIM.INX.gz;194282;"
    "2026-09-21T10:23:04Z;f4650077ba859b418744d48c117c7785\n"
    "CODE/IONO/PRD/COD0OPSP1D_20262660000_01D_01H_GIM.INX.gz;194061;"
    "2026-09-22T10:00:02Z;89e17e5d8fcbe79f89e6dd9fafbbd624\n";

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

    /* Cross-line candidates for map date 2026-09-23 (day 266): both lines,
     * same map date, one-day first. */
    if (sidereon_data_predicted_ionex_line_candidates_json(
            2026, 9, 23, NULL, out, sizeof(out), &written, &required) !=
        SIDEREON_STATUS_OK) {
        return fail("predicted_ionex_line_candidates_json");
    }
    if (written == 0 || written != required || written >= sizeof(out)) {
        return fail("candidate JSON byte contract");
    }
    out[written] = '\0';
    if (strstr((const char *)out, "\"center\":\"cod_prd1\"") == NULL ||
        strstr((const char *)out, "\"center\":\"cod_prd2\"") == NULL ||
        strstr((const char *)out, "\"date\":\"2026-09-23\"") == NULL ||
        strstr((const char *)out,
               "/IONO/PRD/COD0OPSP0D_20262660000_01D_01H_GIM.INX.gz") == NULL ||
        strstr((const char *)out,
               "/IONO/PRD/COD0OPSP1D_20262660000_01D_01H_GIM.INX.gz") == NULL) {
        return fail("candidate JSON content");
    }
    if (strstr((const char *)out, "\"date\":\"2026-09-22\"") != NULL ||
        strstr((const char *)out, "\"date\":\"2026-09-24\"") != NULL) {
        return fail("the walk must never substitute a neighboring map date");
    }

    /* Newest published issue per line from the recorded listing: the one-day
     * line tops out at day 265 while the two-day line already has day 266. */
    if (sidereon_data_newest_published_product_json(
            "cod_prd1", SIDEREON_PRODUCT_FAMILY_IONEX, AIUB_LISTING, out,
            sizeof(out), &written, &required) != SIDEREON_STATUS_OK) {
        return fail("newest_published_product_json cod_prd1");
    }
    out[written] = '\0';
    if (strstr((const char *)out, "\"date\":\"2026-09-22\"") == NULL ||
        strstr((const char *)out, "\"observed_at\":\"2026-09-22T10:00:02Z\"") == NULL) {
        return fail("cod_prd1 newest content");
    }

    if (sidereon_data_newest_published_product_json(
            "cod_prd2", SIDEREON_PRODUCT_FAMILY_IONEX, AIUB_LISTING, out,
            sizeof(out), &written, &required) != SIDEREON_STATUS_OK) {
        return fail("newest_published_product_json cod_prd2");
    }
    out[written] = '\0';
    if (strstr((const char *)out, "\"date\":\"2026-09-23\"") == NULL ||
        strstr((const char *)out, "COD0OPSP1D_20262660000_01D_01H_GIM.INX") == NULL) {
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
