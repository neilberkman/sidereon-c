#include "sidereon.h"
#include "w3_data_distribution_pins.h"

#include <stdbool.h>

#include <stdint.h>
#include <stdlib.h>
#include <string.h>

static void fill_digest(char output[SP3_ARTIFACT_SHA256_C_BYTES], char digit) {
    memset(output, digit, SP3_ARTIFACT_SHA256_C_BYTES - 1);
    output[SP3_ARTIFACT_SHA256_C_BYTES - 1] = '\0';
}

static void fill_pair_digest(
    char output[SP3_ARTIFACT_SHA256_C_BYTES], char first, char second) {
    for (size_t i = 0; i < SP3_ARTIFACT_SHA256_C_BYTES - 1; i += 2) {
        output[i] = first;
        output[i + 1] = second;
    }
    output[SP3_ARTIFACT_SHA256_C_BYTES - 1] = '\0';
}

static void artifact_from_identity(
    struct SidereonSp3ArtifactIdentity *artifact,
    const struct SidereonProductIdentity *identity,
    char digest_digit) {
    memset(artifact, 0, sizeof(*artifact));
    artifact->requested_identity = *identity;
    artifact->resolved_identity = *identity;
    artifact->resolved_identity.has_format_version = 1;
    strcpy(artifact->resolved_identity.format_version, "SP3-d");
    artifact->distribution_source = SIDEREON_DISTRIBUTION_SOURCE_DIRECT;
    strcpy(artifact->official_filename, identity->official_filename);
    fill_digest(artifact->product_sha256, digest_digit);
    artifact->product_byte_length = 12345;
    fill_digest(artifact->archive_sha256, (char)(digest_digit + 1));
    artifact->archive_byte_length = 6789;
    artifact->compression = SIDEREON_ARCHIVE_COMPRESSION_GZIP;
}

static int stable_id_equals(
    const struct SidereonSp3MergeInputIdentity *identity,
    const char *expected) {
    uint8_t value[128];
    size_t written = 0;
    size_t required = 0;
    return sidereon_sp3_merge_input_identity_stable_id(
               identity, value, sizeof(value), &written, &required) ==
               SIDEREON_STATUS_OK &&
        written == strlen(expected) && required == written &&
        memcmp(value, expected, written) == 0;
}

#define ROW_COUNT(rows) (sizeof(rows) / sizeof((rows)[0]))

/* Every catalog query and its answer come from tests/valgen (bin
 * w3_data_distribution), which asks sidereon-core the same questions. */
static int catalog_checks(void) {
    for (size_t i = 0; i < ROW_COUNT(W3DD_SOLUTION_CLASS_ROWS); ++i) {
        const W3DdSolutionClassRow *row = &W3DD_SOLUTION_CLASS_ROWS[i];
        enum SidereonSolutionClass solution = SIDEREON_SOLUTION_CLASS_RAPID;
        enum SidereonStatus status =
            sidereon_data_product_solution_class(row->center, row->family, &solution);
        if ((int)status != row->status ||
            (status == SIDEREON_STATUS_OK && (uint32_t)solution != row->solution_class)) {
            return 70;
        }
    }

    for (size_t i = 0; i < ROW_COUNT(W3DD_DEFAULT_SAMPLE_ROWS); ++i) {
        const W3DdDefaultSampleRow *row = &W3DD_DEFAULT_SAMPLE_ROWS[i];
        uint8_t sample[16];
        size_t written = 99;
        size_t required = 99;
        enum SidereonStatus status = sidereon_data_default_sample_for_date(
            row->center, row->family, row->year, row->month, row->day, sample, sizeof(sample),
            &written, &required);
        size_t expected_len = strlen(row->sample);
        if ((int)status != row->status ||
            (status == SIDEREON_STATUS_OK &&
             (written != expected_len || required != expected_len ||
              memcmp(sample, row->sample, expected_len) != 0))) {
            return 71;
        }
    }

    for (size_t i = 0; i < ROW_COUNT(W3DD_SUPPORTED_SAMPLES_ROWS); ++i) {
        const W3DdSupportedSamplesRow *row = &W3DD_SUPPORTED_SAMPLES_ROWS[i];
        size_t written = 99;
        size_t required = 99;
        enum SidereonStatus status = sidereon_data_supported_samples(
            row->center, row->family, row->year, row->month, row->day, row->issue, NULL, 0,
            &written, &required);
        if ((int)status != row->status || written != 0 || required != row->count) {
            return 90;
        }
        if (status != SIDEREON_STATUS_OK || row->count == 0) {
            continue;
        }
        struct SidereonProductSample samples[4];
        if (sidereon_data_supported_samples(
                row->center, row->family, row->year, row->month, row->day, row->issue,
                samples, row->count, &written, &required) != SIDEREON_STATUS_OK ||
            written != row->count || required != row->count) {
            return 90;
        }
        for (size_t index = 0; index < row->count; ++index) {
            if (strcmp(samples[index].token, row->samples[index]) != 0) {
                return 90;
            }
        }
    }

    /* A buffer one sample short is refused and reports the count
     * (binding two-call contract). */
    for (size_t i = 0; i < ROW_COUNT(W3DD_SUPPORTED_SAMPLES_ROWS); ++i) {
        const W3DdSupportedSamplesRow *row = &W3DD_SUPPORTED_SAMPLES_ROWS[i];
        if (row->status != SIDEREON_STATUS_OK || row->count < 2) {
            continue;
        }
        struct SidereonProductSample too_small[1];
        size_t samples_written = 99;
        size_t samples_required = 99;
        if (sidereon_data_supported_samples(
                row->center, row->family, row->year, row->month, row->day, row->issue,
                too_small, 1, &samples_written, &samples_required) !=
                SIDEREON_STATUS_INVALID_ARGUMENT ||
            samples_written != 0 || samples_required != row->count) {
            return 91;
        }
    }

    for (size_t i = 0; i < ROW_COUNT(W3DD_CONTENT_START_ROWS); ++i) {
        const W3DdContentStartRow *row = &W3DD_CONTENT_START_ROWS[i];
        enum SidereonSp3ContentStartConvention convention =
            SIDEREON_SP3_CONTENT_START_CONVENTION_FILENAME_EPOCH_MINUS_ONE_DAY;
        int64_t offset_s = 1;
        enum SidereonStatus status = sidereon_data_sp3_content_start_convention(
            row->center, row->year, row->month, row->day, row->issue, &convention, &offset_s);
        if ((int)status != row->status || (uint32_t)convention != row->convention ||
            offset_s != row->offset_s) {
            return 87;
        }
    }

    for (size_t i = 0; i < ROW_COUNT(W3DD_IDENTITY_ROWS); ++i) {
        const W3DdIdentityRow *row = &W3DD_IDENTITY_ROWS[i];
        struct SidereonProductIdentity identity;
        enum SidereonStatus status = sidereon_data_product_identity(
            row->center, row->family, row->year, row->month, row->day, row->sample, row->issue,
            &identity);
        if ((int)status != row->status) {
            return 72;
        }
        if (status == SIDEREON_STATUS_OK &&
            (strcmp(identity.official_filename, row->official_filename) != 0 ||
             strcmp(identity.sample, row->sample_out) != 0 ||
             identity.solution_class != row->solution_class)) {
            return 73;
        }
    }

    for (size_t i = 0; i < ROW_COUNT(W3DD_LOCATION_ROWS); ++i) {
        const W3DdLocationRow *row = &W3DD_LOCATION_ROWS[i];
        struct SidereonDistributionLocation location;
        enum SidereonStatus status = sidereon_data_distribution_location(
            row->center, row->family, row->year, row->month, row->day, row->sample, row->issue,
            row->source, &location);
        if ((int)status != row->status) {
            return 85;
        }
        if (status == SIDEREON_STATUS_OK &&
            ((bool)location.has_original_url != row->has_original_url ||
             strcmp(location.original_url, row->original_url) != 0 ||
             strcmp(location.archive_filename, row->archive_filename) != 0 ||
             (uint32_t)location.compression != row->compression)) {
            return 86;
        }
    }

    return 0;
}

static int merge_input_identity_checks(void) {
    struct SidereonProductIdentity first_identity;
    struct SidereonProductIdentity second_identity;
    if (sidereon_data_product_identity(
            "esa", SIDEREON_PRODUCT_FAMILY_SP3, 2026, 7, 16, NULL, NULL,
            &first_identity) != SIDEREON_STATUS_OK ||
        sidereon_data_product_identity(
            "cod", SIDEREON_PRODUCT_FAMILY_SP3, 2026, 7, 16, NULL, NULL,
            &second_identity) != SIDEREON_STATUS_OK) {
        return 9;
    }
    struct SidereonSp3ArtifactIdentity artifacts[2];
    artifact_from_identity(&artifacts[0], &first_identity, '1');
    artifact_from_identity(&artifacts[1], &second_identity, '2');
    fill_pair_digest(artifacts[0].archive_sha256, '1', '2');
    fill_pair_digest(artifacts[1].archive_sha256, '2', '3');

    struct SidereonSp3MergeOptions options;
    if (sidereon_sp3_merge_options_init(&options) != SIDEREON_STATUS_OK) {
        return 10;
    }

    struct SidereonProductIdentity legacy_identity;
    struct SidereonSp3ArtifactIdentity legacy_artifact;
    struct SidereonSp3ArtifactIdentity legacy_canonical;
    struct SidereonSp3MergeInputIdentity *legacy_merge_identity = NULL;
    if (sidereon_data_product_identity(
            "igs", SIDEREON_PRODUCT_FAMILY_SP3, 2022, 11, 26, NULL, NULL,
            &legacy_identity) != SIDEREON_STATUS_OK) {
        return 79;
    }
    artifact_from_identity(&legacy_artifact, &legacy_identity, 'a');
    legacy_artifact.distribution_source = SIDEREON_DISTRIBUTION_SOURCE_NASA_CDDIS;
    legacy_artifact.compression = SIDEREON_ARCHIVE_COMPRESSION_UNIX_COMPRESS;
    if (sidereon_sp3_merge_input_identity(
            &legacy_artifact, 1, &options, &legacy_merge_identity) !=
            SIDEREON_STATUS_OK ||
        legacy_merge_identity == NULL ||
        sidereon_sp3_merge_input_identity_contributor(
            legacy_merge_identity, 0, &legacy_canonical) != SIDEREON_STATUS_OK ||
        legacy_canonical.compression != W3DD_LEGACY_CANONICAL_COMPRESSION) {
        sidereon_sp3_merge_input_identity_free(legacy_merge_identity);
        return 80;
    }
    sidereon_sp3_merge_input_identity_free(legacy_merge_identity);

    const uint32_t systems[] = {
        SIDEREON_GNSS_SYSTEM_GPS,
        SIDEREON_GNSS_SYSTEM_GALILEO,
    };
    const char *frame_labels_0[] = {"IGS20", "ITRF2020"};
    const char *frame_labels_1[] = {"IGS14", "ITRF2014"};
    const struct SidereonSp3FrameLabelSet frame_sets[] = {
        {frame_labels_0, 2},
        {frame_labels_1, 2},
    };
    options.position_tolerance_m = 0.0;
    options.clock_tolerance_s = 2.5e-9;
    options.min_agree = 2;
    options.clock_min_common = 3;
    options.precedence_scope = SIDEREON_SP3_MERGE_PRECEDENCE_SCOPE_SATELLITE_ARC;
    options.outlier_reject_enabled = 1;
    options.outlier_reject_position_tolerance_m = 1.25;
    options.outlier_reject_clock_tolerance_s = 7.5e-9;
    options.target_epoch_interval_s_enabled = 1;
    options.target_epoch_interval_s = 900.0;
    options.systems = systems;
    options.system_count = 2;
    options.asserted_frame_label_sets = frame_sets;
    options.asserted_frame_label_set_count = 2;
    options.helmert_frame_reconciliation = 1;

    struct SidereonSp3MergeInputIdentity *identity = NULL;
    if (sidereon_sp3_merge_input_identity(artifacts, 2, &options, &identity) !=
            SIDEREON_STATUS_OK ||
        identity == NULL) {
        return 11;
    }
    uint8_t schema = 0;
    if (sidereon_sp3_merge_input_identity_schema_version(identity, &schema) !=
            SIDEREON_STATUS_OK ||
        schema != W3DD_SCHEMA_VERSION) {
        return 12;
    }
    size_t written = 99;
    size_t required = 0;
    if (sidereon_sp3_merge_input_identity_stable_id(
            identity, NULL, 0, &written, &required) != SIDEREON_STATUS_OK ||
        written != 0 || required != strlen(W3DD_STABLE_ID) || required > 128) {
        return 13;
    }
    uint8_t stable_id[128];
    if (sidereon_sp3_merge_input_identity_stable_id(
            identity, stable_id, sizeof(stable_id), &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != required ||
        !stable_id_equals(
            identity,
            W3DD_STABLE_ID)) {
        return 14;
    }
    size_t canonical_count = 0;
    uint8_t precedence_present = 2;
    size_t precedence_count = 99;
    struct SidereonSp3ArtifactIdentity canonical;
    if (sidereon_sp3_merge_input_identity_contributor_count(
            identity, &canonical_count) != SIDEREON_STATUS_OK ||
        canonical_count != W3DD_CONTRIBUTOR_COUNT ||
        sidereon_sp3_merge_input_identity_contributor(identity, 0, &canonical) !=
            SIDEREON_STATUS_OK ||
        sidereon_sp3_merge_input_identity_precedence_contributor_count(
            identity, &precedence_present, &precedence_count) != SIDEREON_STATUS_OK ||
        precedence_present != W3DD_PRECEDENCE_PRESENT ||
        precedence_count != W3DD_PRECEDENCE_COUNT ||
        sidereon_sp3_merge_input_identity_precedence_contributor(
            identity, 0, &canonical) == SIDEREON_STATUS_OK) {
        return 15;
    }

    struct SidereonSp3ArtifactIdentity reversed[2] = {artifacts[1], artifacts[0]};
    uint8_t reversed_id[128];
    size_t reversed_written = 0;
    size_t reversed_required = 0;
    struct SidereonSp3MergeInputIdentity *reversed_identity = NULL;
    if (sidereon_sp3_merge_input_identity(
            reversed, 2, &options, &reversed_identity) != SIDEREON_STATUS_OK ||
        sidereon_sp3_merge_input_identity_stable_id(
            reversed_identity, reversed_id, sizeof(reversed_id), &reversed_written,
            &reversed_required) != SIDEREON_STATUS_OK ||
        reversed_written != strlen(W3DD_REVERSED_STABLE_ID) ||
        memcmp(reversed_id, W3DD_REVERSED_STABLE_ID, reversed_written) != 0 ||
        memcmp(stable_id, reversed_id, written) != 0) {
        return 16;
    }
    sidereon_sp3_merge_input_identity_free(reversed_identity);

    options.combine = SIDEREON_SP3_MERGE_COMBINE_PRECEDENCE;
    uint8_t precedence_id[128];
    size_t precedence_written = 0;
    size_t precedence_required = 0;
    struct SidereonSp3MergeInputIdentity *precedence_identity = NULL;
    if (sidereon_sp3_merge_input_identity(
            artifacts, 2, &options, &precedence_identity) != SIDEREON_STATUS_OK ||
        sidereon_sp3_merge_input_identity_stable_id(
            precedence_identity, precedence_id, sizeof(precedence_id),
            &precedence_written, &precedence_required) != SIDEREON_STATUS_OK ||
        precedence_written != written ||
        sidereon_sp3_merge_input_identity_precedence_contributor_count(
            precedence_identity, &precedence_present, &precedence_count) !=
            SIDEREON_STATUS_OK ||
        precedence_present != W3DD_PRECEDENCE_ID_PRESENT ||
        precedence_count != W3DD_PRECEDENCE_ID_COUNT ||
        sidereon_sp3_merge_input_identity_precedence_contributor(
            precedence_identity, 0, &canonical) != SIDEREON_STATUS_OK ||
        strcmp(canonical.product_sha256, W3DD_PRECEDENCE_ID_FIRST_PRODUCT_SHA256) != 0 ||
        !stable_id_equals(
            precedence_identity,
            W3DD_PRECEDENCE_STABLE_ID)) {
        return 17;
    }
    uint8_t reversed_precedence_id[128];
    size_t reversed_precedence_written = 0;
    size_t reversed_precedence_required = 0;
    struct SidereonSp3MergeInputIdentity *reversed_precedence_identity = NULL;
    if (sidereon_sp3_merge_input_identity(
            reversed, 2, &options, &reversed_precedence_identity) !=
            SIDEREON_STATUS_OK ||
        sidereon_sp3_merge_input_identity_stable_id(
            reversed_precedence_identity, reversed_precedence_id,
            sizeof(reversed_precedence_id), &reversed_precedence_written,
            &reversed_precedence_required) != SIDEREON_STATUS_OK ||
        reversed_precedence_written != precedence_written ||
        memcmp(precedence_id, reversed_precedence_id, precedence_written) == 0 ||
        !stable_id_equals(
            reversed_precedence_identity,
            W3DD_REVERSED_PRECEDENCE_STABLE_ID)) {
        return 18;
    }
    sidereon_sp3_merge_input_identity_free(reversed_precedence_identity);
    sidereon_sp3_merge_input_identity_free(precedence_identity);

    options.combine = SIDEREON_SP3_MERGE_COMBINE_MEDIAN;
    uint8_t policy_id[128];
    size_t policy_written = 0;
    size_t policy_required = 0;
    struct SidereonSp3MergeInputIdentity *policy_identity = NULL;
    if (sidereon_sp3_merge_input_identity(
            artifacts, 2, &options, &policy_identity) != SIDEREON_STATUS_OK ||
        sidereon_sp3_merge_input_identity_stable_id(
            policy_identity, policy_id, sizeof(policy_id), &policy_written,
            &policy_required) != SIDEREON_STATUS_OK ||
        policy_written != written || memcmp(stable_id, policy_id, written) == 0 ||
        !stable_id_equals(
            policy_identity,
            W3DD_MEDIAN_STABLE_ID)) {
        return 19;
    }
    sidereon_sp3_merge_input_identity_free(policy_identity);

    options.combine = SIDEREON_SP3_MERGE_COMBINE_MEAN;
    struct SidereonSp3MergeInputIdentity *single_identity = NULL;
    if (sidereon_sp3_merge_input_identity(
            artifacts, 1, &options, &single_identity) != SIDEREON_STATUS_OK ||
        !stable_id_equals(
            single_identity,
            W3DD_SINGLE_STABLE_ID)) {
        return 24;
    }
    sidereon_sp3_merge_input_identity_free(single_identity);

    struct SidereonSp3MergeOptions negative_zero_options = options;
    negative_zero_options.position_tolerance_m = -0.0;
    struct SidereonSp3MergeInputIdentity *negative_zero_identity = NULL;
    if (sidereon_sp3_merge_input_identity(
            artifacts, 2, &negative_zero_options, &negative_zero_identity) !=
            SIDEREON_STATUS_OK ||
        !stable_id_equals(
            negative_zero_identity,
            W3DD_NEGATIVE_ZERO_STABLE_ID)) {
        return 25;
    }
    sidereon_sp3_merge_input_identity_free(negative_zero_identity);

    struct SidereonSp3MergeOptions positive_zero_options = options;
    positive_zero_options.clock_tolerance_s = 0.0;
    negative_zero_options = positive_zero_options;
    negative_zero_options.clock_tolerance_s = -0.0;
    struct SidereonSp3MergeInputIdentity *positive_zero_identity = NULL;
    negative_zero_identity = NULL;
    if (sidereon_sp3_merge_input_identity(
            artifacts, 2, &positive_zero_options, &positive_zero_identity) !=
            SIDEREON_STATUS_OK ||
        sidereon_sp3_merge_input_identity(
            artifacts, 2, &negative_zero_options, &negative_zero_identity) !=
            SIDEREON_STATUS_OK) {
        return 26;
    }
    uint8_t positive_zero_id[128];
    uint8_t negative_zero_id[128];
    size_t positive_zero_written = 0;
    size_t positive_zero_required = 0;
    size_t negative_zero_written = 0;
    size_t negative_zero_required = 0;
    if (sidereon_sp3_merge_input_identity_stable_id(
            positive_zero_identity, positive_zero_id, sizeof(positive_zero_id),
            &positive_zero_written, &positive_zero_required) != SIDEREON_STATUS_OK ||
        sidereon_sp3_merge_input_identity_stable_id(
            negative_zero_identity, negative_zero_id, sizeof(negative_zero_id),
            &negative_zero_written, &negative_zero_required) != SIDEREON_STATUS_OK ||
        positive_zero_written != strlen(W3DD_POSITIVE_ZERO_CLOCK_STABLE_ID) ||
        memcmp(positive_zero_id, W3DD_POSITIVE_ZERO_CLOCK_STABLE_ID, positive_zero_written) !=
            0 ||
        negative_zero_written != strlen(W3DD_NEGATIVE_ZERO_CLOCK_STABLE_ID) ||
        memcmp(negative_zero_id, W3DD_NEGATIVE_ZERO_CLOCK_STABLE_ID, negative_zero_written) !=
            0 ||
        positive_zero_written != negative_zero_written ||
        memcmp(positive_zero_id, negative_zero_id, positive_zero_written) != 0) {
        return 27;
    }
    sidereon_sp3_merge_input_identity_free(positive_zero_identity);
    sidereon_sp3_merge_input_identity_free(negative_zero_identity);

    const uint32_t reversed_systems[] = {
        SIDEREON_GNSS_SYSTEM_GALILEO,
        SIDEREON_GNSS_SYSTEM_GPS,
    };
    const char *reversed_frame_labels_0[] = {"ITRF2014", "IGS14"};
    const char *reversed_frame_labels_1[] = {"ITRF2020", "IGS20"};
    const struct SidereonSp3FrameLabelSet reversed_frame_sets[] = {
        {reversed_frame_labels_0, 2},
        {reversed_frame_labels_1, 2},
    };
    struct SidereonSp3MergeOptions reordered_options = options;
    reordered_options.systems = reversed_systems;
    reordered_options.asserted_frame_label_sets = reversed_frame_sets;
    struct SidereonSp3MergeInputIdentity *reordered_identity = NULL;
    if (sidereon_sp3_merge_input_identity(
            artifacts, 2, &reordered_options, &reordered_identity) !=
            SIDEREON_STATUS_OK ||
        !stable_id_equals(
            reordered_identity,
            W3DD_REORDERED_STABLE_ID)) {
        return 28;
    }
    sidereon_sp3_merge_input_identity_free(reordered_identity);

    struct SidereonSp3ArtifactIdentity changed[2] = {artifacts[0], artifacts[1]};
    fill_digest(changed[1].product_sha256, '3');
    struct SidereonSp3MergeInputIdentity *changed_identity = NULL;
    if (sidereon_sp3_merge_input_identity(
            changed, 2, &options, &changed_identity) != SIDEREON_STATUS_OK) {
        return 20;
    }
    uint8_t changed_id[128];
    size_t changed_written = 0;
    size_t changed_required = 0;
    if (sidereon_sp3_merge_input_identity_stable_id(
            changed_identity, changed_id, sizeof(changed_id), &changed_written,
            &changed_required) != SIDEREON_STATUS_OK ||
        changed_written != strlen(W3DD_CHANGED_DIGEST_STABLE_ID) ||
        memcmp(changed_id, W3DD_CHANGED_DIGEST_STABLE_ID, changed_written) != 0 ||
        memcmp(stable_id, changed_id, written) == 0) {
        return 21;
    }
    sidereon_sp3_merge_input_identity_free(changed_identity);

    changed[0] = artifacts[0];
    changed[1] = artifacts[1];
    strcpy(changed[1].resolved_identity.format_version, "SP3-c");
    changed_identity = NULL;
    if (sidereon_sp3_merge_input_identity(
            changed, 2, &options, &changed_identity) != SIDEREON_STATUS_OK ||
        !stable_id_equals(
            changed_identity,
            W3DD_CHANGED_VERSION_STABLE_ID)) {
        return 50;
    }
    sidereon_sp3_merge_input_identity_free(changed_identity);

    struct SidereonSp3MergeOptions changed_policy_options = options;
    changed_policy_options.clock_tolerance_s = 3.5e-9;
    changed_identity = NULL;
    if (sidereon_sp3_merge_input_identity(
            artifacts, 2, &changed_policy_options, &changed_identity) !=
            SIDEREON_STATUS_OK ||
        !stable_id_equals(
            changed_identity,
            W3DD_CHANGED_POLICY_STABLE_ID)) {
        return 51;
    }
    sidereon_sp3_merge_input_identity_free(changed_identity);

    changed[0] = artifacts[0];
    changed[1] = artifacts[1];
    strcpy(changed[1].product_sha256, "not-a-digest");
    if ((sidereon_sp3_merge_input_identity(changed, 2, &options, &changed_identity) !=
         SIDEREON_STATUS_OK) != W3DD_BAD_PRODUCT_DIGEST_REFUSED) {
        return 22;
    }
    changed[0] = artifacts[0];
    changed[1] = artifacts[1];
    changed[1].archive_sha256[0] = '\0';
    changed_identity = NULL;
    if ((sidereon_sp3_merge_input_identity(changed, 2, &options, &changed_identity) !=
         SIDEREON_STATUS_OK) != W3DD_EMPTY_ARCHIVE_DIGEST_REFUSED) {
        return 29;
    }
    if ((sidereon_sp3_merge_input_identity(NULL, 0, &options, &changed_identity) !=
         SIDEREON_STATUS_OK) != W3DD_NO_CONTRIBUTORS_REFUSED) {
        return 23;
    }

    /* Every raw nested discriminant and C boolean fails before Rust enum/bool use. */
#define EXPECT_INVALID_MUTATION(statement, code)                                  \
    do {                                                                          \
        struct SidereonSp3ArtifactIdentity invalid[2] = {artifacts[0], artifacts[1]}; \
        statement;                                                                \
        changed_identity = NULL;                                                   \
        if (sidereon_sp3_merge_input_identity(                                     \
                invalid, 2, &options, &changed_identity) == SIDEREON_STATUS_OK) {  \
            sidereon_sp3_merge_input_identity_free(changed_identity);              \
            return code;                                                           \
        }                                                                          \
    } while (0)
    EXPECT_INVALID_MUTATION(invalid[0].requested_identity.family = UINT32_MAX, 30);
    EXPECT_INVALID_MUTATION(invalid[0].requested_identity.publisher = UINT32_MAX, 31);
    EXPECT_INVALID_MUTATION(invalid[0].requested_identity.solution_class = UINT32_MAX, 32);
    EXPECT_INVALID_MUTATION(invalid[0].requested_identity.campaign = UINT32_MAX, 33);
    EXPECT_INVALID_MUTATION(invalid[0].requested_identity.format = UINT32_MAX, 34);
    EXPECT_INVALID_MUTATION(invalid[0].requested_identity.has_issue = 2, 35);
    EXPECT_INVALID_MUTATION(invalid[0].requested_identity.has_format_version = 2, 36);
    EXPECT_INVALID_MUTATION(invalid[0].requested_identity.has_prediction_horizon_days = 2, 37);
    EXPECT_INVALID_MUTATION(invalid[0].distribution_source = UINT32_MAX, 38);
    EXPECT_INVALID_MUTATION(invalid[0].compression = UINT32_MAX, 39);
#undef EXPECT_INVALID_MUTATION

#define EXPECT_INVALID_OPTION(statement, code)                                    \
    do {                                                                          \
        struct SidereonSp3MergeOptions invalid_options = options;                 \
        statement;                                                                \
        changed_identity = NULL;                                                   \
        if (sidereon_sp3_merge_input_identity(                                     \
                artifacts, 2, &invalid_options, &changed_identity) ==             \
            SIDEREON_STATUS_OK) {                                                  \
            sidereon_sp3_merge_input_identity_free(changed_identity);              \
            return code;                                                           \
        }                                                                          \
    } while (0)
    EXPECT_INVALID_OPTION(invalid_options.combine = UINT32_MAX, 40);
    EXPECT_INVALID_OPTION(invalid_options.precedence_scope = UINT32_MAX, 41);
    EXPECT_INVALID_OPTION(invalid_options.outlier_reject_enabled = 2, 42);
    EXPECT_INVALID_OPTION(invalid_options.target_epoch_interval_s_enabled = 2, 43);
    EXPECT_INVALID_OPTION(invalid_options.helmert_frame_reconciliation = 2, 44);
    EXPECT_INVALID_OPTION(invalid_options.target_epoch_interval_s_enabled = 1;
                          invalid_options.target_epoch_interval_s = 1.0e-9, 45);
#undef EXPECT_INVALID_OPTION

    sidereon_sp3_merge_input_identity_free(identity);
    return 0;
}

int main(void) {
    int catalog = catalog_checks();
    if (catalog != 0) {
        return catalog;
    }
    struct SidereonProductIdentity identity;
    struct SidereonProductIdentity invalid_identity;
    if (sidereon_data_product_identity(
            "cod", UINT32_MAX, 2026, 7, 12, NULL, NULL, &invalid_identity) !=
        SIDEREON_STATUS_INVALID_ARGUMENT) {
        return 60;
    }
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

    invalid_identity = identity;
    strcpy(invalid_identity.span, "02D");
    char *span_token = strstr(invalid_identity.official_filename, "_01D_");
    if (span_token == NULL) {
        return 65;
    }
    memcpy(span_token, "_02D_", 5);
    size_t invalid_key_written = 99;
    size_t invalid_key_required = 99;
    if (sidereon_data_product_identity_cache_key(
            &invalid_identity, NULL, 0, &invalid_key_written,
            &invalid_key_required) !=
            (W3DD_INCONSISTENT_KEY_REFUSED ? SIDEREON_STATUS_INVALID_ARGUMENT
                                           : SIDEREON_STATUS_OK) ||
        invalid_key_written != 0 || invalid_key_required != 0) {
        return 65;
    }

    struct SidereonDistributionLocation invalid_location;
    if (sidereon_data_distribution_location(
            "cod", UINT32_MAX, 2026, 7, 12, NULL, NULL,
            SIDEREON_DISTRIBUTION_SOURCE_DIRECT, &invalid_location) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        sidereon_data_distribution_location(
            "cod", SIDEREON_PRODUCT_FAMILY_SP3, 2026, 7, 12, NULL, NULL,
            UINT32_MAX, &invalid_location) != SIDEREON_STATUS_INVALID_ARGUMENT) {
        return 61;
    }

    struct SidereonExactCache *invalid_cache = (struct SidereonExactCache *)(uintptr_t)1;
    if (sidereon_exact_cache_open(
            "/tmp/sidereon-invalid-source", &identity, UINT32_MAX, 1,
            &invalid_cache) != SIDEREON_STATUS_INVALID_ARGUMENT ||
        invalid_cache != NULL) {
        return 62;
    }
    bool invalid_hit = true;
    struct SidereonExactCacheEntry *invalid_entry =
        (struct SidereonExactCacheEntry *)(uintptr_t)1;
    if (sidereon_exact_cache_read_unlocked(
            "/tmp/sidereon-invalid-source", &identity, UINT32_MAX, &invalid_hit,
            &invalid_entry) != SIDEREON_STATUS_INVALID_ARGUMENT ||
        invalid_hit || invalid_entry != NULL) {
        return 63;
    }
    size_t invalid_written = 99;
    size_t invalid_required = 99;
    if (sidereon_exact_cache_entry_copy_bytes(
            NULL, UINT32_MAX, NULL, 0, &invalid_written, &invalid_required) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        invalid_written != 0 || invalid_required != 0 ||
        sidereon_exact_cache_entry_copy_path(
            NULL, UINT32_MAX, NULL, 0, &invalid_written, &invalid_required) !=
            SIDEREON_STATUS_INVALID_ARGUMENT) {
        return 64;
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
    if ((sidereon_data_validate_exact_product_set(expected, 2, complete, 2) ==
         SIDEREON_STATUS_OK) != W3DD_EXACT_SET_COMPLETE_OK) {
        return 3;
    }
    if ((sidereon_data_validate_exact_product_set(expected, 2, complete, 1) ==
         SIDEREON_STATUS_OK) != W3DD_EXACT_SET_MISSING_OK) {
        return 4;
    }
    int provenance = merge_input_identity_checks();
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
