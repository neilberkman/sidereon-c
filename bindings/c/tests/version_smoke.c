/* The committed header's SIDEREON_VERSION_* macros must agree with the runtime
 * accessors, which read the crate version. The macros live in cbindgen.toml and
 * do not bump themselves, so this runs on every CI host to catch a release that
 * moves Cargo.toml without moving the header. */
#include "sidereon.h"

#include <stdint.h>
#include <stdio.h>
#include <string.h>

int main(void) {
    uint32_t major = 99, minor = 99, patch = 99;
    sidereon_version(&major, &minor, &patch);
    if (major != SIDEREON_VERSION_MAJOR || minor != SIDEREON_VERSION_MINOR ||
        patch != SIDEREON_VERSION_PATCH) {
        fprintf(stderr,
                "version_smoke: header macros %u.%u.%u but sidereon_version() reports "
                "%u.%u.%u (bump cbindgen.toml after_includes and regenerate the header)\n",
                (unsigned)SIDEREON_VERSION_MAJOR, (unsigned)SIDEREON_VERSION_MINOR,
                (unsigned)SIDEREON_VERSION_PATCH, (unsigned)major, (unsigned)minor,
                (unsigned)patch);
        return 1;
    }
    if (strcmp(sidereon_version_string(), SIDEREON_VERSION_STRING) != 0) {
        fprintf(stderr, "version_smoke: SIDEREON_VERSION_STRING is \"%s\" but "
                        "sidereon_version_string() returns \"%s\"\n",
                SIDEREON_VERSION_STRING, sidereon_version_string());
        return 1;
    }
    sidereon_version(NULL, NULL, NULL); /* all-NULL is a no-op, not a crash. */
    return 0;
}
