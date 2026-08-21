#include "sidereon.h"

#include <stdint.h>
#include <stdio.h>

static int check(int condition, const char *message) {
    if (!condition) {
        fprintf(stderr, "exact_cache_single_flight_surface_smoke: %s\n", message);
        return 1;
    }
    return 0;
}

int main(void) {
    struct SidereonExactCacheSingleFlightOptions options;
    enum SidereonStatus status =
        sidereon_exact_cache_single_flight_options_init(&options);
    if (check(status == SIDEREON_STATUS_OK,
              "single-flight options initialization must succeed") ||
        check(options.struct_size == sizeof(options),
              "single-flight options must carry their ABI size") ||
        check(options.abi_version ==
                  SIDEREON_EXACT_CACHE_SINGLE_FLIGHT_OPTIONS_ABI_VERSION,
              "single-flight options must carry their ABI version") ||
        check(options.poll_interval_ms == 50 &&
                  options.heartbeat_interval_ms == 5000 &&
                  options.liveness_timeout_ms == 30000 &&
                  options.wait_timeout_ms == 1800000,
              "single-flight options must expose engine defaults")) {
        return 1;
    }

    enum SidereonExactCacheOpenResult result =
        SIDEREON_EXACT_CACHE_OPEN_RESULT_OWNER;
    struct SidereonExactCacheEntry *entry =
        (struct SidereonExactCacheEntry *)(uintptr_t)1;
    struct SidereonExactCacheOwner *owner =
        (struct SidereonExactCacheOwner *)(uintptr_t)1;
    status = sidereon_exact_cache_open_single_flight(
        NULL,
        NULL,
        SIDEREON_DISTRIBUTION_SOURCE_IN_MEMORY,
        &options,
        &result,
        &entry,
        &owner);
    if (check(status == SIDEREON_STATUS_NULL_POINTER,
              "single-flight open must reject a null path") ||
        check(result == SIDEREON_EXACT_CACHE_OPEN_RESULT_HIT,
              "single-flight open must initialize its discriminant") ||
        check(entry == NULL && owner == NULL,
              "single-flight open must clear both handle outputs")) {
        return 1;
    }

    struct SidereonExactCacheEntry *published =
        (struct SidereonExactCacheEntry *)(uintptr_t)1;
    status = sidereon_exact_cache_owner_publish(
        NULL, NULL, 0, NULL, 0, NULL, 0, &published);
    if (check(status == SIDEREON_STATUS_NULL_POINTER && published == NULL,
              "owner publish must reject null and clear its output") ||
        check(sidereon_exact_cache_owner_heartbeat(NULL) ==
                  SIDEREON_STATUS_NULL_POINTER,
              "owner heartbeat must reject a null handle") ||
        check(sidereon_exact_cache_single_flight_options_init(NULL) ==
                  SIDEREON_STATUS_NULL_POINTER,
              "single-flight options init must reject null")) {
        return 1;
    }

    sidereon_exact_cache_owner_free(NULL);
    puts("exact_cache_single_flight_surface_smoke: OK");
    return 0;
}
