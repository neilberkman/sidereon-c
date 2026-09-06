/* Compiled ABI coverage for SP3 interpolation policy and gap threshold factor. */
#include "sidereon.h"

#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static uint8_t *read_file(const char *path, size_t *out_len) {
    FILE *file = fopen(path, "rb");
    if (file == NULL || fseek(file, 0, SEEK_END) != 0) {
        if (file != NULL) {
            fclose(file);
        }
        return NULL;
    }
    long size = ftell(file);
    if (size < 0 || fseek(file, 0, SEEK_SET) != 0) {
        fclose(file);
        return NULL;
    }
    uint8_t *bytes = (uint8_t *)malloc((size_t)size + 1);
    if (bytes == NULL) {
        fclose(file);
        return NULL;
    }
    size_t read = fread(bytes, 1, (size_t)size, file);
    fclose(file);
    if (read != (size_t)size) {
        free(bytes);
        return NULL;
    }
    bytes[read] = 0;
    *out_len = read;
    return bytes;
}

static int fail(const char *context) {
    char message[512] = {0};
    sidereon_last_error_message(message, sizeof(message));
    fprintf(stderr, "FAIL: %s: %s\n", context, message);
    return 1;
}

int main(int argc, char **argv) {
    if (argc < 2) {
        fprintf(stderr, "Usage: %s <path-to-gapped-sp3>\n", argv[0]);
        return 1;
    }

    size_t data_len = 0;
    uint8_t *data = read_file(argv[1], &data_len);
    if (data == NULL) {
        return fail("read gapped fixture");
    }

    const double hole_midpoint_j2000_s = 646260300.0;
    struct SidereonSp3 *sp3 = NULL;

    /* 1. Invalid gap threshold factor (1.0) fails with INVALID_ARGUMENT */
    if (sidereon_sp3_load_with_gap_threshold_factor(
            data, data_len, 1.0, &sp3) != SIDEREON_STATUS_INVALID_ARGUMENT) {
        free(data);
        return fail("load with invalid factor (1.0) must fail");
    }

    /* 2. Default factor (0.0): loads, factor is 1.5, refuses hole midpoint for G01 */
    if (sidereon_sp3_load_with_gap_threshold_factor(
            data, data_len, 0.0, &sp3) != SIDEREON_STATUS_OK || sp3 == NULL) {
        free(data);
        return fail("load with default factor");
    }
    double factor = 0.0;
    if (sidereon_sp3_gap_threshold_factor(sp3, &factor) != SIDEREON_STATUS_OK ||
        fabs(factor - 1.5) > 1e-9) {
        sidereon_sp3_free(sp3);
        free(data);
        return fail("default factor must be 1.5");
    }

    double query_epochs[1] = {hole_midpoint_j2000_s};
    double pos[3] = {0.0};
    double clk = 0.0;
    size_t written = 0;
    if (sidereon_sp3_interpolate(
            sp3, "G01", query_epochs, 1, pos, 3, &clk, 1, &written) != SIDEREON_STATUS_SOLVE) {
        sidereon_sp3_free(sp3);
        free(data);
        return fail("default factor must refuse hole midpoint");
    }

    /* 3. Check continuity with default vs wide factor */
    size_t default_defects = 0;
    size_t wide_defects = 0;
    size_t checked = 0;
    size_t skipped = 0;
    if (sidereon_sp3_check_continuity_with_gap_threshold_factor(
            sp3, -1, 1.0, 1.0, &default_defects, &checked, &skipped) !=
        SIDEREON_STATUS_INVALID_ARGUMENT) {
        sidereon_sp3_free(sp3);
        free(data);
        return fail("check continuity with factor 1.0 must fail");
    }
    if (sidereon_sp3_check_continuity_with_gap_threshold_factor(
            sp3, -1, 1.0, 0.0, &default_defects, &checked, &skipped) != SIDEREON_STATUS_OK) {
        sidereon_sp3_free(sp3);
        free(data);
        return fail("check continuity with default factor");
    }
    if (sidereon_sp3_check_continuity_with_gap_threshold_factor(
            sp3, -1, 1.0, 13.0, &wide_defects, &checked, &skipped) != SIDEREON_STATUS_OK) {
        sidereon_sp3_free(sp3);
        free(data);
        return fail("check continuity with factor 13.0");
    }
    if (wide_defects >= default_defects) {
        sidereon_sp3_free(sp3);
        free(data);
        return fail("factor 13.0 must have fewer hold-out defects than default");
    }

    /* 4. Continuity verdict JSON with factor */
    size_t json_written = 0;
    size_t json_required = 0;
    if (sidereon_sp3_continuity_verdict_json_with_gap_threshold_factor(
            sp3, -1, 1.0, 1.0,
            hole_midpoint_j2000_s - 100.0, hole_midpoint_j2000_s + 100.0,
            NULL, 0, &json_written, &json_required) != SIDEREON_STATUS_INVALID_ARGUMENT) {
        sidereon_sp3_free(sp3);
        free(data);
        return fail("verdict json with factor 1.0 must fail");
    }
    if (sidereon_sp3_continuity_verdict_json_with_gap_threshold_factor(
            sp3, -1, 1.0, 13.0,
            hole_midpoint_j2000_s - 100.0, hole_midpoint_j2000_s + 100.0,
            NULL, 0, &json_written, &json_required) != SIDEREON_STATUS_OK ||
        json_required == 0) {
        sidereon_sp3_free(sp3);
        free(data);
        return fail("verdict json with factor 13.0");
    }

    /* Extract canonical samples for subsequent sample/interpolant tests */
    size_t sample_count = 0;
    if (sidereon_sp3_precise_ephemeris_samples(
            sp3, NULL, 0, &written, &sample_count) != SIDEREON_STATUS_OK ||
        sample_count == 0) {
        sidereon_sp3_free(sp3);
        free(data);
        return fail("get sample count");
    }
    struct SidereonPreciseEphemerisSample *samples = (struct SidereonPreciseEphemerisSample *)malloc(
        sample_count * sizeof(struct SidereonPreciseEphemerisSample));
    if (samples == NULL) {
        sidereon_sp3_free(sp3);
        free(data);
        return fail("alloc samples buffer");
    }
    if (sidereon_sp3_precise_ephemeris_samples(
            sp3, samples, sample_count, &written, &sample_count) != SIDEREON_STATUS_OK ||
        written != sample_count) {
        free(samples);
        sidereon_sp3_free(sp3);
        free(data);
        return fail("extract precise samples");
    }
    sidereon_sp3_free(sp3);
    sp3 = NULL;

    /* 5. Factor 13.0: loads, factor is 13.0, serves hole midpoint for G01 */
    if (sidereon_sp3_load_with_gap_threshold_factor(
            data, data_len, 13.0, &sp3) != SIDEREON_STATUS_OK || sp3 == NULL) {
        free(samples);
        free(data);
        return fail("load with factor 13.0");
    }
    if (sidereon_sp3_gap_threshold_factor(sp3, &factor) != SIDEREON_STATUS_OK ||
        fabs(factor - 13.0) > 1e-9) {
        sidereon_sp3_free(sp3);
        free(samples);
        free(data);
        return fail("wide factor must be 13.0");
    }
    if (sidereon_sp3_interpolate(
            sp3, "G01", query_epochs, 1, pos, 3, &clk, 1, &written) != SIDEREON_STATUS_OK ||
        written != 1 || !isfinite(pos[0])) {
        sidereon_sp3_free(sp3);
        free(samples);
        free(data);
        return fail("factor 13.0 must serve hole midpoint");
    }

    /* 6. Precise interpolant store artifact serialization and header inspection */
    enum SidereonPreciseInterpolantArtifactErrorKind art_err =
        SIDEREON_PRECISE_INTERPOLANT_ARTIFACT_ERROR_KIND_NONE;
    size_t art_len = 0;
    if (sidereon_sp3_precise_interpolant_artifact_bytes(
            sp3, &art_err, NULL, 0, &written, &art_len) != SIDEREON_STATUS_OK ||
        art_len == 0) {
        sidereon_sp3_free(sp3);
        free(samples);
        free(data);
        return fail("get artifact bytes length");
    }
    uint8_t *art_bytes = (uint8_t *)malloc(art_len);
    if (art_bytes == NULL) {
        sidereon_sp3_free(sp3);
        free(samples);
        free(data);
        return fail("alloc artifact buffer");
    }
    if (sidereon_sp3_precise_interpolant_artifact_bytes(
            sp3, &art_err, art_bytes, art_len, &written, &art_len) != SIDEREON_STATUS_OK ||
        written != art_len) {
        free(art_bytes);
        sidereon_sp3_free(sp3);
        free(samples);
        free(data);
        return fail("write artifact bytes");
    }
    sidereon_sp3_free(sp3);
    sp3 = NULL;

    struct SidereonPreciseInterpolantArtifact *artifact = NULL;
    if (sidereon_precise_interpolant_artifact_open_owned(
            art_bytes, art_len, &art_err, &artifact) != SIDEREON_STATUS_OK ||
        artifact == NULL) {
        free(art_bytes);
        free(samples);
        free(data);
        return fail("open artifact");
    }
    double art_factor = 0.0;
    if (sidereon_precise_interpolant_artifact_gap_threshold_factor(
            artifact, &art_factor) != SIDEREON_STATUS_OK ||
        fabs(art_factor - 13.0) > 1e-9) {
        sidereon_precise_interpolant_artifact_free(artifact);
        free(art_bytes);
        free(samples);
        free(data);
        return fail("artifact header factor must be 13.0");
    }
    sidereon_precise_interpolant_artifact_free(artifact);
    free(art_bytes);

    /* 7. PreciseEphemerisSamples with factor */
    struct SidereonPreciseEphemerisSamples *samples_handle = NULL;
    if (sidereon_precise_ephemeris_samples_from_samples_with_gap_threshold_factor(
            samples, sample_count, 1.0, &samples_handle) != SIDEREON_STATUS_INVALID_ARGUMENT) {
        free(samples);
        free(data);
        return fail("samples with factor 1.0 must fail");
    }
    if (sidereon_precise_ephemeris_samples_from_samples_with_gap_threshold_factor(
            samples, sample_count, 0.0, &samples_handle) != SIDEREON_STATUS_OK ||
        samples_handle == NULL) {
        free(samples);
        free(data);
        return fail("samples with default factor");
    }
    double samples_factor = 0.0;
    if (sidereon_precise_ephemeris_samples_gap_threshold_factor(
            samples_handle, &samples_factor) != SIDEREON_STATUS_OK ||
        fabs(samples_factor - 1.5) > 1e-9) {
        sidereon_precise_ephemeris_samples_free(samples_handle);
        free(samples);
        free(data);
        return fail("default samples factor must be 1.5");
    }
    const char *sats[1] = {"G01"};
    bool has_clk = false;
    enum SidereonObservableStateElementStatus elem_status;
    enum SidereonStatus res_status;
    if (sidereon_precise_ephemeris_samples_observable_states_at_shared_j2000_s(
            samples_handle, sats, 1, hole_midpoint_j2000_s,
            pos, &clk, &has_clk, &elem_status, &res_status) != SIDEREON_STATUS_OK ||
        elem_status != SIDEREON_OBSERVABLE_STATE_ELEMENT_STATUS_GAP) {
        sidereon_precise_ephemeris_samples_free(samples_handle);
        free(samples);
        free(data);
        return fail("default samples query at hole midpoint must be Gap");
    }
    sidereon_precise_ephemeris_samples_free(samples_handle);

    if (sidereon_precise_ephemeris_samples_from_samples_with_gap_threshold_factor(
            samples, sample_count, 13.0, &samples_handle) != SIDEREON_STATUS_OK ||
        samples_handle == NULL) {
        free(samples);
        free(data);
        return fail("samples with factor 13.0");
    }
    if (sidereon_precise_ephemeris_samples_gap_threshold_factor(
            samples_handle, &samples_factor) != SIDEREON_STATUS_OK ||
        fabs(samples_factor - 13.0) > 1e-9) {
        sidereon_precise_ephemeris_samples_free(samples_handle);
        free(samples);
        free(data);
        return fail("wide samples factor must be 13.0");
    }
    if (sidereon_precise_ephemeris_samples_observable_states_at_shared_j2000_s(
            samples_handle, sats, 1, hole_midpoint_j2000_s,
            pos, &clk, &has_clk, &elem_status, &res_status) != SIDEREON_STATUS_OK ||
        elem_status != SIDEREON_OBSERVABLE_STATE_ELEMENT_STATUS_VALID || !isfinite(pos[0])) {
        sidereon_precise_ephemeris_samples_free(samples_handle);
        free(samples);
        free(data);
        return fail("wide samples query at hole midpoint must be Valid");
    }
    sidereon_precise_ephemeris_samples_free(samples_handle);

    /* 8. PreciseEphemerisInterpolant with factor */
    struct SidereonPreciseEphemerisInterpolant *interp_handle = NULL;
    if (sidereon_precise_ephemeris_interpolant_from_samples_with_gap_threshold_factor(
            samples, sample_count, 1.0, &interp_handle) != SIDEREON_STATUS_INVALID_ARGUMENT) {
        free(samples);
        free(data);
        return fail("interpolant with factor 1.0 must fail");
    }
    if (sidereon_precise_ephemeris_interpolant_from_samples_with_gap_threshold_factor(
            samples, sample_count, 0.0, &interp_handle) != SIDEREON_STATUS_OK ||
        interp_handle == NULL) {
        free(samples);
        free(data);
        return fail("interpolant with default factor");
    }
    double interp_factor = 0.0;
    if (sidereon_precise_ephemeris_interpolant_gap_threshold_factor(
            interp_handle, &interp_factor) != SIDEREON_STATUS_OK ||
        fabs(interp_factor - 1.5) > 1e-9) {
        sidereon_precise_ephemeris_interpolant_free(interp_handle);
        free(samples);
        free(data);
        return fail("default interpolant factor must be 1.5");
    }
    if (sidereon_precise_ephemeris_interpolant_observable_states_at_shared_j2000_s(
            interp_handle, sats, 1, hole_midpoint_j2000_s,
            pos, &clk, &has_clk, &elem_status, &res_status) != SIDEREON_STATUS_OK ||
        elem_status != SIDEREON_OBSERVABLE_STATE_ELEMENT_STATUS_GAP) {
        sidereon_precise_ephemeris_interpolant_free(interp_handle);
        free(samples);
        free(data);
        return fail("default interpolant query at hole midpoint must be Gap");
    }
    sidereon_precise_ephemeris_interpolant_free(interp_handle);

    if (sidereon_precise_ephemeris_interpolant_from_samples_with_gap_threshold_factor(
            samples, sample_count, 13.0, &interp_handle) != SIDEREON_STATUS_OK ||
        interp_handle == NULL) {
        free(samples);
        free(data);
        return fail("interpolant with factor 13.0");
    }
    if (sidereon_precise_ephemeris_interpolant_gap_threshold_factor(
            interp_handle, &interp_factor) != SIDEREON_STATUS_OK ||
        fabs(interp_factor - 13.0) > 1e-9) {
        sidereon_precise_ephemeris_interpolant_free(interp_handle);
        free(samples);
        free(data);
        return fail("wide interpolant factor must be 13.0");
    }
    if (sidereon_precise_ephemeris_interpolant_observable_states_at_shared_j2000_s(
            interp_handle, sats, 1, hole_midpoint_j2000_s,
            pos, &clk, &has_clk, &elem_status, &res_status) != SIDEREON_STATUS_OK ||
        elem_status != SIDEREON_OBSERVABLE_STATE_ELEMENT_STATUS_VALID || !isfinite(pos[0])) {
        sidereon_precise_ephemeris_interpolant_free(interp_handle);
        free(samples);
        free(data);
        return fail("wide interpolant query at hole midpoint must be Valid");
    }
    sidereon_precise_ephemeris_interpolant_free(interp_handle);

    free(samples);
    free(data);
    printf("sp3_interpolation_smoke: all tests passed\n");
    return 0;
}
