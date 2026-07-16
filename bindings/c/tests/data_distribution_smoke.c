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
    return status == SIDEREON_STATUS_OK ? 0 : 2;
}
