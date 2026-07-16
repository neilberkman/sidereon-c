#include "sidereon.h"

int main(void) {
    struct SidereonProductIdentity identity;
    enum SidereonStatus status = sidereon_data_product_identity(
        "cod",
        SIDEREON_PRODUCT_FAMILY_SP3,
        2026,
        7,
        12,
        NULL,
        NULL,
        &identity);
    if (status != SIDEREON_STATUS_OK) {
        return 1;
    }

    struct SidereonProductIdentity next_identity;
    status = sidereon_data_product_identity(
        "cod",
        SIDEREON_PRODUCT_FAMILY_SP3,
        2026,
        7,
        13,
        NULL,
        NULL,
        &next_identity);
    if (status != SIDEREON_STATUS_OK) {
        return 2;
    }
    const struct SidereonProductIdentity expected[] = {identity, next_identity};
    const struct SidereonProductIdentity complete[] = {next_identity, identity};
    if (sidereon_data_validate_exact_product_set(expected, 2, complete, 2) !=
        SIDEREON_STATUS_OK) {
        return 3;
    }
    if (sidereon_data_validate_exact_product_set(expected, 2, complete, 1) ==
        SIDEREON_STATUS_OK) {
        return 4;
    }

    struct SidereonDistributionLocation location;
    status = sidereon_data_distribution_location(
        "cod",
        SIDEREON_PRODUCT_FAMILY_SP3,
        2026,
        7,
        12,
        NULL,
        NULL,
        SIDEREON_DISTRIBUTION_SOURCE_NASA_CDDIS,
        &location);
    return status == SIDEREON_STATUS_OK ? 0 : 5;
}
