#include "sidereon.h"

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

static int sample_for_date_equals(
    const char *center,
    uint32_t family,
    int32_t year,
    uint8_t month,
    uint8_t day,
    const char *expected) {
    uint8_t sample[16];
    size_t written = 99;
    size_t required = 99;
    size_t expected_len = strlen(expected);
    return sidereon_data_default_sample_for_date(
               center, family, year, month, day, sample, sizeof(sample), &written,
               &required) == SIDEREON_STATUS_OK &&
        written == expected_len && required == expected_len &&
        memcmp(sample, expected, expected_len) == 0;
}

static int supported_samples_equal(
    const char *center,
    uint32_t family,
    int32_t year,
    uint8_t month,
    uint8_t day,
    const char *issue,
    const char *const *expected,
    size_t expected_count) {
    size_t written = 99;
    size_t required = 99;
    if (sidereon_data_supported_samples(
            center, family, year, month, day, issue, NULL, 0, &written,
            &required) != SIDEREON_STATUS_OK ||
        written != 0 || required != expected_count) {
        return 0;
    }
    if (expected_count == 0) {
        return 1;
    }

    struct SidereonProductSample *samples =
        calloc(expected_count, sizeof(struct SidereonProductSample));
    if (samples == NULL ||
        sidereon_data_supported_samples(
            center, family, year, month, day, issue, samples, expected_count,
            &written, &required) != SIDEREON_STATUS_OK ||
        written != expected_count || required != expected_count) {
        free(samples);
        return 0;
    }
    for (size_t index = 0; index < expected_count; ++index) {
        if (strcmp(samples[index].token, expected[index]) != 0) {
            free(samples);
            return 0;
        }
    }
    free(samples);
    return 1;
}

static int content_start_equals(
    const char *center,
    int32_t year,
    uint8_t month,
    uint8_t day,
    const char *issue,
    enum SidereonSp3ContentStartConvention expected_convention,
    int64_t expected_offset_s) {
    enum SidereonSp3ContentStartConvention convention =
        SIDEREON_SP3_CONTENT_START_CONVENTION_FILENAME_EPOCH;
    int64_t offset_s = 1;
    return sidereon_data_sp3_content_start_convention(
               center, year, month, day, issue, &convention, &offset_s) ==
               SIDEREON_STATUS_OK &&
        convention == expected_convention && offset_s == expected_offset_s;
}

static int catalog_checks(void) {
    enum SidereonSolutionClass solution = SIDEREON_SOLUTION_CLASS_RAPID;
    if (sidereon_data_product_solution_class(
            "igs", SIDEREON_PRODUCT_FAMILY_SP3, &solution) != SIDEREON_STATUS_OK ||
        solution != SIDEREON_SOLUTION_CLASS_FINAL ||
        sidereon_data_product_solution_class(
            "igs", SIDEREON_PRODUCT_FAMILY_RINEX_NAVIGATION, &solution) !=
            SIDEREON_STATUS_OK ||
        solution != SIDEREON_SOLUTION_CLASS_BROADCAST ||
        sidereon_data_product_solution_class(
            "igs", SIDEREON_PRODUCT_FAMILY_RINEX_CLOCK, &solution) !=
            SIDEREON_STATUS_INVALID_ARGUMENT) {
        return 70;
    }

    if (!sample_for_date_equals(
            "gfz", SIDEREON_PRODUCT_FAMILY_SP3, 2021, 5, 17, "15M") ||
        !sample_for_date_equals(
            "gfz", SIDEREON_PRODUCT_FAMILY_SP3, 2021, 5, 18, "05M") ||
        !sample_for_date_equals(
            "gfz", SIDEREON_PRODUCT_FAMILY_SP3, 2026, 7, 19, "05M")) {
        return 71;
    }

    if (!sample_for_date_equals(
            "esa_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2024, 9, 3, "15M") ||
        !sample_for_date_equals(
            "esa_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2025, 2, 2, "15M") ||
        !sample_for_date_equals(
            "esa_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2025, 2, 3, "05M") ||
        !sample_for_date_equals(
            "gfz_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2021, 5, 15, "15M") ||
        !sample_for_date_equals(
            "gfz_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2021, 5, 16, "05M")) {
        return 81;
    }

    static const char *sample_05m[] = {"05M"};
    static const char *sample_15m[] = {"15M"};
    static const char *sample_15m_05m[] = {"15M", "05M"};
    static const char *sample_30s[] = {"30S"};
    static const char *sample_01h[] = {"01H"};
    static const char *sample_01d[] = {"01D"};
    if (!supported_samples_equal(
            "esa", SIDEREON_PRODUCT_FAMILY_SP3, 2026, 6, 15, NULL,
            sample_05m, 1) ||
        !supported_samples_equal(
            "gfz", SIDEREON_PRODUCT_FAMILY_SP3, 2021, 5, 17, NULL,
            sample_15m, 1) ||
        !supported_samples_equal(
            "gfz", SIDEREON_PRODUCT_FAMILY_SP3, 2021, 5, 18, NULL,
            sample_05m, 1) ||
        !supported_samples_equal(
            "esa_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2025, 2, 2, "0600",
            sample_15m, 1) ||
        !supported_samples_equal(
            "esa_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2025, 2, 2, "1200",
            sample_05m, 1) ||
        !supported_samples_equal(
            "gfz_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2021, 5, 15, "0000",
            sample_15m_05m, 2) ||
        !supported_samples_equal(
            "gfz_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2021, 5, 15, "2100",
            sample_15m, 1) ||
        !supported_samples_equal(
            "cod", SIDEREON_PRODUCT_FAMILY_RINEX_CLOCK, 2026, 6, 15, NULL,
            sample_30s, 1) ||
        !supported_samples_equal(
            "cod", SIDEREON_PRODUCT_FAMILY_IONEX, 2026, 6, 15, NULL,
            sample_01h, 1) ||
        !supported_samples_equal(
            "igs", SIDEREON_PRODUCT_FAMILY_RINEX_NAVIGATION, 2026, 6, 15,
            NULL, sample_01d, 1)) {
        return 90;
    }

    struct SidereonProductSample too_small[1];
    size_t samples_written = 99;
    size_t samples_required = 99;
    if (sidereon_data_supported_samples(
            "gfz_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2021, 5, 15, "0000",
            too_small, 1, &samples_written, &samples_required) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        samples_written != 0 || samples_required != 2 ||
        sidereon_data_supported_samples(
            "gfz_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2021, 5, 15, "0130",
            NULL, 0, &samples_written, &samples_required) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        samples_written != 0 || samples_required != 0) {
        return 91;
    }

    static const char *gfz_issues[] = {
        "0000", "0300", "0600", "0900", "1200", "1500", "1800", "2100",
    };
    static const enum SidereonSp3ContentStartConvention day_7[] = {
        SIDEREON_SP3_CONTENT_START_CONVENTION_FILENAME_EPOCH,
        SIDEREON_SP3_CONTENT_START_CONVENTION_FILENAME_EPOCH_MINUS_ONE_DAY,
        SIDEREON_SP3_CONTENT_START_CONVENTION_FILENAME_EPOCH_MINUS_ONE_DAY,
        SIDEREON_SP3_CONTENT_START_CONVENTION_FILENAME_EPOCH_MINUS_ONE_DAY,
        SIDEREON_SP3_CONTENT_START_CONVENTION_FILENAME_EPOCH_MINUS_ONE_DAY,
        SIDEREON_SP3_CONTENT_START_CONVENTION_FILENAME_EPOCH_MINUS_ONE_DAY,
        SIDEREON_SP3_CONTENT_START_CONVENTION_FILENAME_EPOCH_MINUS_ONE_DAY,
        SIDEREON_SP3_CONTENT_START_CONVENTION_FILENAME_EPOCH_MINUS_ONE_DAY,
    };
    static const enum SidereonSp3ContentStartConvention day_8[] = {
        SIDEREON_SP3_CONTENT_START_CONVENTION_FILENAME_EPOCH,
        SIDEREON_SP3_CONTENT_START_CONVENTION_FILENAME_EPOCH_MINUS_ONE_DAY,
        SIDEREON_SP3_CONTENT_START_CONVENTION_FILENAME_EPOCH_MINUS_ONE_DAY,
        SIDEREON_SP3_CONTENT_START_CONVENTION_FILENAME_EPOCH,
        SIDEREON_SP3_CONTENT_START_CONVENTION_FILENAME_EPOCH,
        SIDEREON_SP3_CONTENT_START_CONVENTION_FILENAME_EPOCH,
        SIDEREON_SP3_CONTENT_START_CONVENTION_FILENAME_EPOCH,
        SIDEREON_SP3_CONTENT_START_CONVENTION_FILENAME_EPOCH,
    };
    if (!content_start_equals(
            "gfz_ult", 2022, 9, 6, "2100",
            SIDEREON_SP3_CONTENT_START_CONVENTION_FILENAME_EPOCH_MINUS_ONE_DAY,
            -86400) ||
        !content_start_equals(
            "gfz_ult", 2022, 9, 9, "0000",
            SIDEREON_SP3_CONTENT_START_CONVENTION_FILENAME_EPOCH, 0) ||
        !content_start_equals(
            "igs", 2022, 9, 7, NULL,
            SIDEREON_SP3_CONTENT_START_CONVENTION_FILENAME_EPOCH, 0)) {
        return 87;
    }
    for (size_t index = 0; index < 8; ++index) {
        if (!content_start_equals(
                "gfz_ult", 2022, 9, 7, gfz_issues[index], day_7[index],
                day_7[index] ==
                        SIDEREON_SP3_CONTENT_START_CONVENTION_FILENAME_EPOCH
                    ? 0
                    : -86400) ||
            !content_start_equals(
                "gfz_ult", 2022, 9, 8, gfz_issues[index], day_8[index],
                day_8[index] ==
                        SIDEREON_SP3_CONTENT_START_CONVENTION_FILENAME_EPOCH
                    ? 0
                    : -86400)) {
            return 88;
        }
    }
    enum SidereonSp3ContentStartConvention invalid_convention =
        SIDEREON_SP3_CONTENT_START_CONVENTION_FILENAME_EPOCH_MINUS_ONE_DAY;
    int64_t invalid_offset = -1;
    if (sidereon_data_sp3_content_start_convention(
            "gfz_ult", 2022, 9, 7, "0130", &invalid_convention,
            &invalid_offset) != SIDEREON_STATUS_INVALID_ARGUMENT ||
        invalid_convention !=
            SIDEREON_SP3_CONTENT_START_CONVENTION_FILENAME_EPOCH ||
        invalid_offset != 0 ||
        sidereon_data_sp3_content_start_convention(
            "gfz", 2022, 9, 7, "0000", &invalid_convention,
            &invalid_offset) != SIDEREON_STATUS_INVALID_ARGUMENT ||
        sidereon_data_sp3_content_start_convention(
            "gfz_ult", 2022, 9, 7, NULL, &invalid_convention,
            &invalid_offset) != SIDEREON_STATUS_INVALID_ARGUMENT) {
        return 89;
    }

    struct SidereonProductIdentity legacy;
    struct SidereonDistributionLocation location;
    if (sidereon_data_product_identity(
            "igs", SIDEREON_PRODUCT_FAMILY_SP3, 2022, 11, 26, NULL, NULL,
            &legacy) != SIDEREON_STATUS_OK ||
        strcmp(legacy.official_filename, "igs22376.sp3") != 0 ||
        legacy.solution_class != SIDEREON_SOLUTION_CLASS_FINAL ||
        sidereon_data_distribution_location(
            "igs", SIDEREON_PRODUCT_FAMILY_SP3, 2022, 11, 26, NULL, NULL,
            SIDEREON_DISTRIBUTION_SOURCE_NASA_CDDIS, &location) !=
            SIDEREON_STATUS_OK ||
        location.compression != SIDEREON_ARCHIVE_COMPRESSION_UNIX_COMPRESS ||
        strcmp(location.archive_filename, "igs22376.sp3.Z") != 0 ||
        strcmp(
            location.original_url,
            "https://cddis.nasa.gov/archive/gnss/products/2237/igs22376.sp3.Z") !=
            0 ||
        sidereon_data_distribution_location(
            "igs", SIDEREON_PRODUCT_FAMILY_SP3, 2022, 11, 26, NULL, NULL,
            SIDEREON_DISTRIBUTION_SOURCE_DIRECT, &location) !=
            SIDEREON_STATUS_INVALID_ARGUMENT) {
        return 72;
    }

    struct SidereonProductIdentity current;
    if (sidereon_data_product_identity(
            "igs", SIDEREON_PRODUCT_FAMILY_SP3, 2022, 11, 27, NULL, NULL,
            &current) != SIDEREON_STATUS_OK ||
        strcmp(
            current.official_filename,
            "IGS0OPSFIN_20223310000_01D_15M_ORB.SP3") != 0 ||
        sidereon_data_distribution_location(
            "igs", SIDEREON_PRODUCT_FAMILY_SP3, 2022, 11, 27, NULL, NULL,
            SIDEREON_DISTRIBUTION_SOURCE_DIRECT, &location) != SIDEREON_STATUS_OK ||
        location.compression != SIDEREON_ARCHIVE_COMPRESSION_GZIP ||
        strcmp(
            location.original_url,
            "https://igs.bkg.bund.de/root_ftp/IGS/products/2238/"
            "IGS0OPSFIN_20223310000_01D_15M_ORB.SP3.gz") != 0 ||
        sidereon_data_distribution_location(
            "igs", SIDEREON_PRODUCT_FAMILY_SP3, 2022, 11, 27, NULL, NULL,
            SIDEREON_DISTRIBUTION_SOURCE_NASA_CDDIS, &location) !=
            SIDEREON_STATUS_OK ||
        location.compression != SIDEREON_ARCHIVE_COMPRESSION_GZIP ||
        strcmp(
            location.original_url,
            "https://cddis.nasa.gov/archive/gnss/products/2238/"
            "IGS0OPSFIN_20223310000_01D_15M_ORB.SP3.gz") != 0) {
        return 73;
    }

    if (sidereon_data_product_identity(
            "igs", SIDEREON_PRODUCT_FAMILY_SP3, 1994, 1, 2, NULL, NULL,
            &legacy) != SIDEREON_STATUS_OK ||
        strcmp(legacy.official_filename, "igs07300.sp3") != 0 ||
        sidereon_data_distribution_location(
            "igs", SIDEREON_PRODUCT_FAMILY_SP3, 1994, 1, 2, NULL, NULL,
            SIDEREON_DISTRIBUTION_SOURCE_NASA_CDDIS, &location) !=
            SIDEREON_STATUS_OK ||
        strcmp(
            location.original_url,
            "https://cddis.nasa.gov/archive/gnss/products/0730/igs07300.sp3.Z") !=
            0 ||
        sidereon_data_product_identity(
            "igs", SIDEREON_PRODUCT_FAMILY_SP3, 1994, 1, 1, NULL, NULL,
            &legacy) != SIDEREON_STATUS_INVALID_ARGUMENT) {
        return 74;
    }

    if (sidereon_data_product_identity(
            "esa", SIDEREON_PRODUCT_FAMILY_SP3, 2014, 1, 4, NULL, NULL,
            &legacy) != SIDEREON_STATUS_INVALID_ARGUMENT ||
        sidereon_data_product_identity(
            "esa", SIDEREON_PRODUCT_FAMILY_SP3, 2014, 1, 5, NULL, NULL,
            &legacy) != SIDEREON_STATUS_OK ||
        strcmp(
            legacy.official_filename,
            "ESA0MGNFIN_20140050000_01D_05M_ORB.SP3") != 0 ||
        sidereon_data_product_identity(
            "gfz", SIDEREON_PRODUCT_FAMILY_SP3, 2020, 5, 12, NULL, NULL,
            &legacy) != SIDEREON_STATUS_INVALID_ARGUMENT ||
        sidereon_data_product_identity(
            "gfz", SIDEREON_PRODUCT_FAMILY_SP3, 2020, 5, 13, NULL, NULL,
            &legacy) != SIDEREON_STATUS_OK ||
        strcmp(legacy.sample, "15M") != 0 ||
        sidereon_data_product_identity(
            "esa", SIDEREON_PRODUCT_FAMILY_RINEX_CLOCK, 2014, 1, 4, NULL,
            NULL, &legacy) != SIDEREON_STATUS_INVALID_ARGUMENT ||
        sidereon_data_product_identity(
            "esa", SIDEREON_PRODUCT_FAMILY_RINEX_CLOCK, 2014, 1, 5, NULL,
            NULL, &legacy) != SIDEREON_STATUS_OK ||
        sidereon_data_product_identity(
            "gfz", SIDEREON_PRODUCT_FAMILY_RINEX_CLOCK, 2020, 5, 12, NULL,
            NULL, &legacy) != SIDEREON_STATUS_INVALID_ARGUMENT ||
        sidereon_data_product_identity(
            "gfz", SIDEREON_PRODUCT_FAMILY_RINEX_CLOCK, 2020, 5, 13, NULL,
            NULL, &legacy) != SIDEREON_STATUS_OK) {
        return 82;
    }

    if (sidereon_data_product_identity(
            "igs_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2022, 11, 26, NULL,
            "0600", &legacy) != SIDEREON_STATUS_INVALID_ARGUMENT ||
        sidereon_data_product_identity(
            "igs_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2022, 11, 27, NULL,
            "0600", &legacy) != SIDEREON_STATUS_OK ||
        strcmp(legacy.sample, "15M") != 0 ||
        sidereon_data_product_identity(
            "cod_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2022, 11, 26, NULL,
            "0000", &legacy) != SIDEREON_STATUS_INVALID_ARGUMENT ||
        sidereon_data_product_identity(
            "cod_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2022, 11, 27, NULL,
            "0000", &legacy) != SIDEREON_STATUS_OK ||
        strcmp(legacy.sample, "05M") != 0 ||
        sidereon_data_product_identity(
            "esa_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2022, 10, 3, NULL,
            "0600", &legacy) != SIDEREON_STATUS_INVALID_ARGUMENT ||
        sidereon_data_product_identity(
            "esa_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2022, 10, 4, NULL,
            "0600", &legacy) != SIDEREON_STATUS_OK ||
        strcmp(legacy.sample, "15M") != 0 ||
        sidereon_data_product_identity(
            "gfz_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2020, 10, 5, NULL,
            "0600", &legacy) != SIDEREON_STATUS_INVALID_ARGUMENT ||
        sidereon_data_product_identity(
            "gfz_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2020, 10, 6, NULL,
            "0600", &legacy) != SIDEREON_STATUS_OK ||
        strcmp(legacy.sample, "15M") != 0) {
        return 83;
    }

    if (sidereon_data_product_identity(
            "esa_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2024, 9, 3, NULL,
            "0600", &legacy) != SIDEREON_STATUS_OK ||
        strcmp(legacy.sample, "15M") != 0 ||
        sidereon_data_product_identity(
            "esa_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2025, 2, 2, NULL,
            "0600", &legacy) != SIDEREON_STATUS_OK ||
        strcmp(legacy.sample, "15M") != 0 ||
        sidereon_data_product_identity(
            "esa_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2025, 2, 2, NULL,
            "1200", &legacy) != SIDEREON_STATUS_OK ||
        strcmp(legacy.sample, "05M") != 0 ||
        sidereon_data_product_identity(
            "gfz_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2021, 5, 15, NULL,
            "0600", &legacy) != SIDEREON_STATUS_OK ||
        strcmp(legacy.sample, "15M") != 0 ||
        sidereon_data_product_identity(
            "gfz_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2021, 5, 16, NULL,
            "0600", &legacy) != SIDEREON_STATUS_OK ||
        strcmp(legacy.sample, "05M") != 0) {
        return 84;
    }

    if (sidereon_data_product_identity(
            "esa", SIDEREON_PRODUCT_FAMILY_SP3, 2026, 6, 15, "15M", NULL,
            &legacy) != SIDEREON_STATUS_INVALID_ARGUMENT ||
        sidereon_data_product_identity(
            "gfz", SIDEREON_PRODUCT_FAMILY_SP3, 2021, 5, 17, "05M", NULL,
            &legacy) != SIDEREON_STATUS_INVALID_ARGUMENT ||
        sidereon_data_product_identity(
            "esa_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2025, 2, 2, "05M",
            "0600", &legacy) != SIDEREON_STATUS_INVALID_ARGUMENT ||
        sidereon_data_product_identity(
            "gfz_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2021, 5, 15, "05M",
            "2100", &legacy) != SIDEREON_STATUS_INVALID_ARGUMENT ||
        sidereon_data_product_identity(
            "gfz_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2021, 5, 15, "05M",
            "0000", &legacy) != SIDEREON_STATUS_OK) {
        return 92;
    }

    if (sidereon_data_distribution_location(
            "esa", SIDEREON_PRODUCT_FAMILY_SP3, 2020, 6, 24, NULL, NULL,
            SIDEREON_DISTRIBUTION_SOURCE_DIRECT, &location) !=
            SIDEREON_STATUS_OK ||
        sidereon_data_distribution_location(
            "esa", SIDEREON_PRODUCT_FAMILY_SP3, 2020, 6, 24, NULL, NULL,
            SIDEREON_DISTRIBUTION_SOURCE_NASA_CDDIS, &location) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        sidereon_data_distribution_location(
            "gfz", SIDEREON_PRODUCT_FAMILY_SP3, 2020, 6, 24, NULL, NULL,
            SIDEREON_DISTRIBUTION_SOURCE_DIRECT, &location) !=
            SIDEREON_STATUS_OK ||
        sidereon_data_distribution_location(
            "gfz", SIDEREON_PRODUCT_FAMILY_SP3, 2020, 6, 24, NULL, NULL,
            SIDEREON_DISTRIBUTION_SOURCE_NASA_CDDIS, &location) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        sidereon_data_distribution_location(
            "cod_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2022, 11, 26, NULL,
            "0000", SIDEREON_DISTRIBUTION_SOURCE_DIRECT, &location) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        sidereon_data_distribution_location(
            "esa_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2022, 10, 4, NULL,
            "0600", SIDEREON_DISTRIBUTION_SOURCE_DIRECT, &location) !=
            SIDEREON_STATUS_OK ||
        sidereon_data_distribution_location(
            "esa_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2022, 10, 4, NULL,
            "0600", SIDEREON_DISTRIBUTION_SOURCE_NASA_CDDIS, &location) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        sidereon_data_distribution_location(
            "gfz_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2020, 10, 6, NULL,
            "0600", SIDEREON_DISTRIBUTION_SOURCE_DIRECT, &location) !=
            SIDEREON_STATUS_OK ||
        sidereon_data_distribution_location(
            "gfz_ult", SIDEREON_PRODUCT_FAMILY_SP3, 2020, 10, 6, NULL,
            "0600", SIDEREON_DISTRIBUTION_SOURCE_NASA_CDDIS, &location) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        sidereon_data_distribution_location(
            "igs", SIDEREON_PRODUCT_FAMILY_SP3, 2020, 6, 24, NULL, NULL,
            SIDEREON_DISTRIBUTION_SOURCE_NASA_CDDIS, &location) !=
            SIDEREON_STATUS_OK ||
        location.compression != SIDEREON_ARCHIVE_COMPRESSION_UNIX_COMPRESS ||
        sidereon_data_distribution_location(
            "esa", SIDEREON_PRODUCT_FAMILY_SP3, 2024, 6, 24, NULL, NULL,
            SIDEREON_DISTRIBUTION_SOURCE_NASA_CDDIS, &location) !=
            SIDEREON_STATUS_INVALID_ARGUMENT) {
        return 85;
    }

    struct SidereonProductIdentity nav;
    if (sidereon_data_product_identity(
            "igs", SIDEREON_PRODUCT_FAMILY_RINEX_NAVIGATION, 2020, 6, 25, NULL,
            NULL, &nav) != SIDEREON_STATUS_OK ||
        nav.solution_class != SIDEREON_SOLUTION_CLASS_BROADCAST ||
        strcmp(nav.official_filename, "BRDC00WRD_R_20201770000_01D_MN.rnx") != 0 ||
        sidereon_data_distribution_location(
            "igs", SIDEREON_PRODUCT_FAMILY_RINEX_NAVIGATION, 2020, 6, 25, NULL,
            NULL, SIDEREON_DISTRIBUTION_SOURCE_DIRECT, &location) !=
            SIDEREON_STATUS_OK ||
        strcmp(
            location.original_url,
            "https://igs.bkg.bund.de/root_ftp/IGS/BRDC/2020/177/"
            "BRDC00WRD_R_20201770000_01D_MN.rnx.gz") != 0) {
        return 75;
    }

    const uint32_t code_families[] = {
        SIDEREON_PRODUCT_FAMILY_SP3,
        SIDEREON_PRODUCT_FAMILY_RINEX_CLOCK,
        SIDEREON_PRODUCT_FAMILY_IONEX,
    };
    const char *code_urls[] = {
        "https://www.aiub.unibe.ch/download/CODE_MGEX/CODE/2026/"
        "COD0MGXFIN_20261200000_01D_05M_ORB.SP3.gz",
        "https://www.aiub.unibe.ch/download/CODE_MGEX/CODE/2026/"
        "COD0MGXFIN_20261200000_01D_30S_CLK.CLK.gz",
        "https://www.aiub.unibe.ch/download/CODE/2026/"
        "COD0OPSFIN_20261200000_01D_01H_GIM.INX.gz",
    };
    for (size_t index = 0; index < 3; ++index) {
        if (sidereon_data_distribution_location(
                "cod", code_families[index], 2026, 4, 30, NULL, NULL,
                SIDEREON_DISTRIBUTION_SOURCE_DIRECT, &location) !=
                SIDEREON_STATUS_OK ||
            strcmp(location.original_url, code_urls[index]) != 0) {
            return 76;
        }
        if (sidereon_data_product_identity(
                "cod", code_families[index], 2022, 11, 26, NULL, NULL,
                &legacy) != SIDEREON_STATUS_INVALID_ARGUMENT) {
            return 77;
        }
    }
    if (sidereon_data_distribution_location(
            "cod_rap", SIDEREON_PRODUCT_FAMILY_IONEX, 2026, 4, 30, NULL, NULL,
            SIDEREON_DISTRIBUTION_SOURCE_DIRECT, &location) != SIDEREON_STATUS_OK ||
        strcmp(
            location.original_url,
            "https://www.aiub.unibe.ch/download/CODE/"
            "COD0OPSRAP_20261200000_01D_01H_GIM.INX.gz") != 0 ||
        sidereon_data_product_identity(
            "cod_rap", SIDEREON_PRODUCT_FAMILY_SP3, 2026, 4, 30, NULL, NULL,
            &legacy) != SIDEREON_STATUS_INVALID_ARGUMENT) {
        return 78;
    }

    if (sidereon_data_distribution_location(
            "esa", SIDEREON_PRODUCT_FAMILY_IONEX, 2022, 11, 26, NULL, NULL,
            SIDEREON_DISTRIBUTION_SOURCE_NASA_CDDIS, &location) !=
            SIDEREON_STATUS_INVALID_ARGUMENT ||
        sidereon_data_distribution_location(
            "esa", SIDEREON_PRODUCT_FAMILY_IONEX, 2024, 6, 24, NULL, NULL,
            SIDEREON_DISTRIBUTION_SOURCE_NASA_CDDIS, &location) !=
            SIDEREON_STATUS_OK ||
        strcmp(
            location.original_url,
            "https://cddis.nasa.gov/archive/gnss/products/ionex/2024/176/"
            "ESA0OPSFIN_20241760000_01D_02H_GIM.INX.gz") != 0) {
        return 86;
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
        legacy_canonical.compression !=
            SIDEREON_ARCHIVE_COMPRESSION_UNIX_COMPRESS) {
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
        schema != 1) {
        return 12;
    }
    size_t written = 99;
    size_t required = 0;
    if (sidereon_sp3_merge_input_identity_stable_id(
            identity, NULL, 0, &written, &required) != SIDEREON_STATUS_OK ||
        written != 0 || required == 0 || required > 128) {
        return 13;
    }
    uint8_t stable_id[128];
    if (sidereon_sp3_merge_input_identity_stable_id(
            identity, stable_id, sizeof(stable_id), &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != required ||
        !stable_id_equals(
            identity,
            "sidereon-sp3-merge-input-v1:bfba88f693a65c2068208ce66e9282d4e447812ff4cffc2e94972da8fb1a8ed9")) {
        return 14;
    }
    size_t canonical_count = 0;
    uint8_t precedence_present = 2;
    size_t precedence_count = 99;
    struct SidereonSp3ArtifactIdentity canonical;
    if (sidereon_sp3_merge_input_identity_contributor_count(
            identity, &canonical_count) != SIDEREON_STATUS_OK ||
        canonical_count != 2 ||
        sidereon_sp3_merge_input_identity_contributor(identity, 0, &canonical) !=
            SIDEREON_STATUS_OK ||
        sidereon_sp3_merge_input_identity_precedence_contributor_count(
            identity, &precedence_present, &precedence_count) != SIDEREON_STATUS_OK ||
        precedence_present != 0 || precedence_count != 0 ||
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
        reversed_written != written ||
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
        precedence_present != 1 || precedence_count != 2 ||
        sidereon_sp3_merge_input_identity_precedence_contributor(
            precedence_identity, 0, &canonical) != SIDEREON_STATUS_OK ||
        canonical.product_sha256[0] != '1' ||
        !stable_id_equals(
            precedence_identity,
            "sidereon-sp3-merge-input-v1:a6098cc21485781411418ca235555ed7cace5275981e8f597d5e41ae83f6893b")) {
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
            "sidereon-sp3-merge-input-v1:0f91ca5d17ec2f912b080d4c83dd6fabbdabb5ac0f615c9e278f9011d1ca3df7")) {
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
            "sidereon-sp3-merge-input-v1:4c102b45c1a845f7ef84dbcda74af867bbc8f48278a2ed78dd422121a5d734eb")) {
        return 19;
    }
    sidereon_sp3_merge_input_identity_free(policy_identity);

    options.combine = SIDEREON_SP3_MERGE_COMBINE_MEAN;
    struct SidereonSp3MergeInputIdentity *single_identity = NULL;
    if (sidereon_sp3_merge_input_identity(
            artifacts, 1, &options, &single_identity) != SIDEREON_STATUS_OK ||
        !stable_id_equals(
            single_identity,
            "sidereon-sp3-merge-input-v1:61b7a723717a9e03db1701d769e965e18ce81c87ed2caffae33e9e0c41e75c94")) {
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
            "sidereon-sp3-merge-input-v1:bfba88f693a65c2068208ce66e9282d4e447812ff4cffc2e94972da8fb1a8ed9")) {
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
            "sidereon-sp3-merge-input-v1:bfba88f693a65c2068208ce66e9282d4e447812ff4cffc2e94972da8fb1a8ed9")) {
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
        changed_written != written || memcmp(stable_id, changed_id, written) == 0) {
        return 21;
    }
    sidereon_sp3_merge_input_identity_free(changed_identity);

    changed[0] = artifacts[0];
    changed[1] = artifacts[1];
    strcpy(changed[1].resolved_identity.format_version, "SP3-c");
    changed_identity = NULL;
    if (sidereon_sp3_merge_input_identity(
            changed, 2, &options, &changed_identity) != SIDEREON_STATUS_OK ||
        stable_id_equals(
            changed_identity,
            "sidereon-sp3-merge-input-v1:bfba88f693a65c2068208ce66e9282d4e447812ff4cffc2e94972da8fb1a8ed9")) {
        return 50;
    }
    sidereon_sp3_merge_input_identity_free(changed_identity);

    struct SidereonSp3MergeOptions changed_policy_options = options;
    changed_policy_options.clock_tolerance_s = 3.5e-9;
    changed_identity = NULL;
    if (sidereon_sp3_merge_input_identity(
            artifacts, 2, &changed_policy_options, &changed_identity) !=
            SIDEREON_STATUS_OK ||
        stable_id_equals(
            changed_identity,
            "sidereon-sp3-merge-input-v1:bfba88f693a65c2068208ce66e9282d4e447812ff4cffc2e94972da8fb1a8ed9")) {
        return 51;
    }
    sidereon_sp3_merge_input_identity_free(changed_identity);

    changed[0] = artifacts[0];
    changed[1] = artifacts[1];
    strcpy(changed[1].product_sha256, "not-a-digest");
    if (sidereon_sp3_merge_input_identity(
            changed, 2, &options, &changed_identity) == SIDEREON_STATUS_OK) {
        return 22;
    }
    changed[0] = artifacts[0];
    changed[1] = artifacts[1];
    changed[1].archive_sha256[0] = '\0';
    changed_identity = NULL;
    if (sidereon_sp3_merge_input_identity(
            changed, 2, &options, &changed_identity) == SIDEREON_STATUS_OK) {
        return 29;
    }
    if (sidereon_sp3_merge_input_identity(
            NULL, 0, &options, &changed_identity) == SIDEREON_STATUS_OK) {
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
                          invalid_options.target_epoch_interval_s = 0.5, 45);
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
            &invalid_key_required) != SIDEREON_STATUS_INVALID_ARGUMENT ||
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
    if (sidereon_data_validate_exact_product_set(expected, 2, complete, 2) !=
        SIDEREON_STATUS_OK) {
        return 3;
    }
    if (sidereon_data_validate_exact_product_set(expected, 2, complete, 1) ==
        SIDEREON_STATUS_OK) {
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
