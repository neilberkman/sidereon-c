#include "sidereon.h"

#include <stdint.h>
#include <stdio.h>

static int check(int condition, const char *message) {
    if (!condition) {
        fprintf(stderr, "attested_open_surface_smoke: %s\n", message);
        return 1;
    }
    return 0;
}

int main(void) {
    SidereonMmapTerrain *terrain = (SidereonMmapTerrain *)(uintptr_t)1;
    enum SidereonStatus status =
        sidereon_mmap_terrain_from_path_attested(NULL, UINT64_MAX, &terrain);
    if (check(status == SIDEREON_STATUS_NULL_POINTER && terrain == NULL,
              "terrain attested constructor must clear output on a null path")) {
        return 1;
    }

    enum SidereonDigestProvenance provenance = SIDEREON_DIGEST_PROVENANCE_ATTESTED;
    status = sidereon_mmap_terrain_digest_provenance(NULL, &provenance);
    if (check(status == SIDEREON_STATUS_NULL_POINTER &&
                  provenance == SIDEREON_DIGEST_PROVENANCE_VERIFIED,
              "terrain provenance must initialize output before handle validation")) {
        return 1;
    }
    if (check(sidereon_mmap_terrain_verify(NULL) == SIDEREON_STATUS_NULL_POINTER,
              "terrain verify must reject a null handle")) {
        return 1;
    }

    SidereonPreciseInterpolantArtifact *artifact =
        (SidereonPreciseInterpolantArtifact *)(uintptr_t)1;
    enum SidereonPreciseInterpolantArtifactErrorKind artifact_error =
        SIDEREON_PRECISE_INTERPOLANT_ARTIFACT_ERROR_KIND_CORRUPT;
    status = sidereon_precise_interpolant_artifact_from_path_attested(
        NULL, UINT64_MAX, &artifact_error, &artifact);
    if (check(status == SIDEREON_STATUS_NULL_POINTER && artifact == NULL &&
                  artifact_error == SIDEREON_PRECISE_INTERPOLANT_ARTIFACT_ERROR_KIND_NONE,
              "precise attested constructor must initialize outputs on a null path")) {
        return 1;
    }

    provenance = SIDEREON_DIGEST_PROVENANCE_ATTESTED;
    status = sidereon_precise_interpolant_artifact_digest_provenance(NULL, &provenance);
    if (check(status == SIDEREON_STATUS_NULL_POINTER &&
                  provenance == SIDEREON_DIGEST_PROVENANCE_VERIFIED,
              "precise provenance must initialize output before handle validation")) {
        return 1;
    }
    artifact_error = SIDEREON_PRECISE_INTERPOLANT_ARTIFACT_ERROR_KIND_CORRUPT;
    status = sidereon_precise_interpolant_artifact_verify(NULL, &artifact_error);
    if (check(status == SIDEREON_STATUS_NULL_POINTER &&
                  artifact_error == SIDEREON_PRECISE_INTERPOLANT_ARTIFACT_ERROR_KIND_NONE,
              "precise verify must initialize its typed error before handle validation")) {
        return 1;
    }

    puts("attested_open_surface_smoke: OK");
    return 0;
}
