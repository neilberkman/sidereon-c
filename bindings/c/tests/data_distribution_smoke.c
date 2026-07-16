#include "sidereon.h"

#include <stdint.h>
#include <string.h>

static void fill_digest(char output[SP3_ARTIFACT_SHA256_C_BYTES], char digit) {
    memset(output, digit, SP3_ARTIFACT_SHA256_C_BYTES - 1);
    output[SP3_ARTIFACT_SHA256_C_BYTES - 1] = '\0';
}

static void artifact_from_identity(
    struct SidereonSp3ArtifactIdentity *artifact,
    const struct SidereonProductIdentity *identity,
    char digest_digit) {
    memset(artifact, 0, sizeof(*artifact));
    artifact->requested_identity = *identity;
    artifact->resolved_identity = *identity;
    artifact->resolved_identity.has_format_version = true;
    strcpy(artifact->resolved_identity.format_version, "d");
    artifact->distribution_source = SIDEREON_DISTRIBUTION_SOURCE_DIRECT;
    strcpy(artifact->official_filename, identity->official_filename);
    fill_digest(artifact->product_sha256, digest_digit);
    artifact->product_byte_length = 12345;
    fill_digest(artifact->archive_sha256, (char)(digest_digit + 1));
    artifact->archive_byte_length = 6789;
    artifact->compression = SIDEREON_ARCHIVE_COMPRESSION_GZIP;
}

static int merge_input_identity_checks(
    const struct SidereonProductIdentity *first_identity,
    const struct SidereonProductIdentity *second_identity) {
    struct SidereonSp3ArtifactIdentity artifacts[2];
    artifact_from_identity(&artifacts[0], first_identity, '1');
    artifact_from_identity(&artifacts[1], second_identity, '2');

    struct SidereonSp3MergeOptions options;
    if (sidereon_sp3_merge_options_init(&options) != SIDEREON_STATUS_OK) {
        return 10;
    }

    uint8_t schema = 0;
    size_t written = 99;
    size_t required = 0;
    if (sidereon_sp3_merge_input_identity(
            artifacts, 2, &options, &schema, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        schema != 1 || written != 0 || required == 0 || required > 128) {
        return 11;
    }

    uint8_t stable_id[128];
    if (sidereon_sp3_merge_input_identity(
            artifacts, 2, &options, &schema, stable_id, sizeof(stable_id), &written,
            &required) != SIDEREON_STATUS_OK ||
        written != required) {
        return 12;
    }

    struct SidereonSp3ArtifactIdentity reversed[2] = {artifacts[1], artifacts[0]};
    uint8_t reversed_id[128];
    size_t reversed_written = 0;
    size_t reversed_required = 0;
    if (sidereon_sp3_merge_input_identity(
            reversed, 2, &options, &schema, reversed_id, sizeof(reversed_id),
            &reversed_written, &reversed_required) != SIDEREON_STATUS_OK ||
        reversed_written != written ||
        memcmp(stable_id, reversed_id, written) != 0) {
        return 13;
    }

    options.combine = SIDEREON_SP3_MERGE_COMBINE_PRECEDENCE;
    uint8_t precedence_id[128];
    size_t precedence_written = 0;
    size_t precedence_required = 0;
    if (sidereon_sp3_merge_input_identity(
            artifacts, 2, &options, &schema, precedence_id, sizeof(precedence_id),
            &precedence_written, &precedence_required) != SIDEREON_STATUS_OK ||
        precedence_written != written) {
        return 18;
    }
    uint8_t reversed_precedence_id[128];
    size_t reversed_precedence_written = 0;
    size_t reversed_precedence_required = 0;
    if (sidereon_sp3_merge_input_identity(
            reversed, 2, &options, &schema, reversed_precedence_id,
            sizeof(reversed_precedence_id), &reversed_precedence_written,
            &reversed_precedence_required) != SIDEREON_STATUS_OK ||
        reversed_precedence_written != precedence_written ||
        memcmp(precedence_id, reversed_precedence_id, precedence_written) == 0) {
        return 19;
    }

    options.combine = SIDEREON_SP3_MERGE_COMBINE_MEDIAN;
    uint8_t policy_id[128];
    size_t policy_written = 0;
    size_t policy_required = 0;
    if (sidereon_sp3_merge_input_identity(
            artifacts, 2, &options, &schema, policy_id, sizeof(policy_id),
            &policy_written, &policy_required) != SIDEREON_STATUS_OK ||
        policy_written != written || memcmp(stable_id, policy_id, written) == 0) {
        return 14;
    }

    options.combine = SIDEREON_SP3_MERGE_COMBINE_MEAN;
    struct SidereonSp3ArtifactIdentity changed[2] = {artifacts[0], artifacts[1]};
    fill_digest(changed[1].product_sha256, '3');
    uint8_t changed_id[128];
    size_t changed_written = 0;
    size_t changed_required = 0;
    if (sidereon_sp3_merge_input_identity(
            changed, 2, &options, &schema, changed_id, sizeof(changed_id),
            &changed_written, &changed_required) != SIDEREON_STATUS_OK ||
        changed_written != written || memcmp(stable_id, changed_id, written) == 0) {
        return 15;
    }

    changed[1].product_sha256[0] = '\0';
    if (sidereon_sp3_merge_input_identity(
            changed, 2, &options, &schema, changed_id, sizeof(changed_id),
            &changed_written, &changed_required) == SIDEREON_STATUS_OK) {
        return 16;
    }
    if (sidereon_sp3_merge_input_identity(
            NULL, 0, &options, &schema, NULL, 0, &changed_written,
            &changed_required) == SIDEREON_STATUS_OK) {
        return 17;
    }
    return 0;
}

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
    int provenance = merge_input_identity_checks(&identity, &next_identity);
    if (provenance != 0) {
        return provenance;
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
