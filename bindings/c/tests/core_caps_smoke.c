/*
 * Smoke coverage for core-backed C capabilities added in this round:
 * DGNSS position solve, ANTEX encode, RINEX OBS header/epoch/value helpers,
 * SP3 clock-reference offset/align, and source-backed reduced-orbit fit/drift.
 *
 * argv: <sp3> <antex> <rinex_obs>
 */
#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sidereon.h"
/* sidereon-core's results for the inputs below, and the synthetic DGNSS
 * observations, from tests/valgen (w6_core_caps). */
#include "w6_core_caps_pins.h"

static int failures = 0;

static void check(int ok, const char *what) {
    if (!ok) {
        char msg[512];
        size_t n = sidereon_last_error_message((char *)msg, sizeof(msg));
        if (n == 0) {
            msg[0] = '\0';
        }
        /* The last error is sticky and may come from an earlier call, so it is
           reported as context rather than as the cause of this failure. */
        if (msg[0] != '\0') {
            fprintf(stderr, "FAIL: %s (last ABI error, may predate this check: %s)\n",
                    what, msg);
        } else {
            fprintf(stderr, "FAIL: %s\n", what);
        }
        failures++;
    }
}

static bool same_f64(double value, uint64_t expected) {
    uint64_t bits = 0;
    memcpy(&bits, &value, sizeof(bits));
    return bits == expected;
}

static double bits_to_f64(uint64_t bits) {
    double value;
    memcpy(&value, &bits, sizeof(value));
    return value;
}

static bool token_is(const char *token, size_t capacity, const char *expected) {
    return strncmp(token, expected, capacity) == 0;
}

static void test_null_copy_count_preserves_sibling_reset(void) {
    static const uint8_t empty_catalog_json[] = "[]";
    SidereonOmmCatalog *catalog = NULL;
    const enum SidereonStatus build = sidereon_omm_catalog_build_lenient(
        SIDEREON_GNSS_SYSTEM_GPS, empty_catalog_json, sizeof(empty_catalog_json) - 1,
        &catalog);
    check(build == SIDEREON_STATUS_OK && catalog != NULL,
          "empty OMM catalog setup for null count control");
    if (catalog == NULL) return;

    size_t required = 99;
    const enum SidereonStatus status = sidereon_omm_catalog_skipped_object_name(
        catalog, 0, NULL, 0, NULL, &required);
    check(status == SIDEREON_STATUS_NULL_POINTER && required == 0,
          "null written count still resets its valid required-count sibling");

    const enum SidereonStatus alias_status = sidereon_omm_catalog_record_count(
        catalog, (size_t *)(void *)catalog);
    check(alias_status == SIDEREON_STATUS_INVALID_ARGUMENT,
          "catalog count output overlapping the live catalog is refused");
    size_t record_count = 99;
    const enum SidereonStatus count_status =
        sidereon_omm_catalog_record_count(catalog, &record_count);
    check(count_status == SIDEREON_STATUS_OK && record_count == 0,
          "catalog remains usable after overlapping count output refusal");
    sidereon_omm_catalog_free(catalog);
}

static bool payload_decimal_text(const char *payload, const char *field,
                                 char *out, size_t out_size) {
    char key[96];
    const int key_len = snprintf(key, sizeof key, "\"%s\":", field);
    if (key_len < 0 || (size_t)key_len >= sizeof key || out_size == 0) return false;
    const char *field_value = strstr(payload, key);
    if (field_value == NULL) return false;
    const char *decimal = strstr(field_value, "\"decimal\":\"");
    if (decimal == NULL) return false;
    decimal += strlen("\"decimal\":\"");
    const char *end = strchr(decimal, '"');
    if (end == NULL || (size_t)(end - decimal) >= out_size) return false;
    memcpy(out, decimal, (size_t)(end - decimal));
    out[end - decimal] = '\0';
    return true;
}

static bool payload_decimal(const char *payload, const char *field, double *out) {
    char text[96];
    if (out == NULL || !payload_decimal_text(payload, field, text, sizeof text)) return false;
    char *end = NULL;
    const double value = strtod(text, &end);
    if (end == text || *end != '\0' || !isfinite(value)) return false;
    *out = value;
    return true;
}

static uint8_t *read_file(const char *path, size_t *out_len) {
    FILE *f = fopen(path, "rb");
    if (!f) {
        fprintf(stderr, "FAIL: cannot open %s\n", path);
        failures++;
        return NULL;
    }
    if (fseek(f, 0, SEEK_END) != 0) {
        fclose(f);
        return NULL;
    }
    long size = ftell(f);
    if (size < 0) {
        fclose(f);
        return NULL;
    }
    rewind(f);
    uint8_t *buf = (uint8_t *)malloc((size_t)size + 1);
    if (!buf) {
        fclose(f);
        return NULL;
    }
    size_t got = fread(buf, 1, (size_t)size, f);
    fclose(f);
    *out_len = got;
    buf[got] = 0;
    return buf;
}

static SidereonSp3 *load_sp3(const char *path) {
    size_t len = 0;
    uint8_t *bytes = read_file(path, &len);
    if (!bytes) {
        return NULL;
    }
    SidereonSp3 *sp3 = NULL;
    check(sidereon_sp3_load(bytes, len, &sp3) == SIDEREON_STATUS_OK && sp3 != NULL,
          "core caps sp3 load");
    free(bytes);
    return sp3;
}

static double first_sp3_epoch(const SidereonSp3 *sp3) {
    size_t written = 0, required = 0;
    check(sidereon_sp3_epochs_j2000_seconds(sp3, NULL, 0, &written, &required) ==
              SIDEREON_STATUS_OK &&
              required > 0,
          "sp3 epochs query");
    if (required == 0) {
        return 0.0;
    }
    double *epochs = (double *)calloc(required, sizeof(double));
    if (!epochs) {
        check(0, "sp3 epochs allocation");
        return 0.0;
    }
    check(sidereon_sp3_epochs_j2000_seconds(sp3, epochs, required, &written, &required) ==
              SIDEREON_STATUS_OK &&
              written > 0,
          "sp3 epochs copy");
    double first = epochs[0];
    free(epochs);
    return first;
}

static void test_antex_encode(const char *antex_path) {
    size_t len = 0;
    uint8_t *bytes = read_file(antex_path, &len);
    if (!bytes) {
        return;
    }
    union {
        size_t alignment;
        uint8_t bytes[sizeof(void *)];
    } aliased_slots;
    memset(aliased_slots.bytes, 0xA7, sizeof aliased_slots.bytes);
    uint8_t slots_before[sizeof aliased_slots.bytes];
    memcpy(slots_before, aliased_slots.bytes, sizeof slots_before);
    check(sidereon_antex_parse_result(
              bytes, len, (SidereonAntex **)(void *)aliased_slots.bytes,
              (SidereonAntexResult **)(void *)aliased_slots.bytes) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(slots_before, aliased_slots.bytes, sizeof slots_before) == 0,
          "antex parse result refuses overlapping output slots before clearing");
    SidereonAntex *result_antex = NULL;
    SidereonAntexResult *parse_result = NULL;
    check(sidereon_antex_parse_result(bytes, len, &result_antex, &parse_result) ==
              SIDEREON_STATUS_OK && result_antex != NULL && parse_result != NULL,
          "antex parse result accepts distinct output slots");
    sidereon_antex_free(result_antex);
    sidereon_antex_result_free(parse_result);
    SidereonAntex *antex = NULL;
    check(sidereon_antex_parse(bytes, len, &antex) == SIDEREON_STATUS_OK && antex != NULL,
          "antex parse");
    free(bytes);
    if (!antex) {
        return;
    }

    size_t before_count = 0;
    check(sidereon_antex_antenna_count(antex, &before_count) == SIDEREON_STATUS_OK &&
              before_count == W6_CC_ANTEX_ANTENNAS,
          "antex count before encode");

    size_t written = 0, required = 0;
    check(sidereon_antex_encode(antex, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
              required == W6_CC_ANTEX_ENCODED_LEN,
          "antex encode query");
    uint8_t *encoded = (uint8_t *)malloc(required + 1);
    if (!encoded) {
        check(0, "antex encode allocation");
        sidereon_antex_free(antex);
        return;
    }
    check(sidereon_antex_encode(antex, encoded, required, &written, &required) ==
              SIDEREON_STATUS_OK &&
              written == required,
          "antex encode copy");
    encoded[written] = 0;
    uint64_t hash = UINT64_C(0xcbf29ce484222325);
    for (size_t i = 0; i < written; i++) {
        hash ^= encoded[i];
        hash *= UINT64_C(0x00000100000001b3);
    }
    check(hash == W6_CC_ANTEX_ENCODED_FNV1A, "antex encoded text");

    SidereonAntex *roundtrip = NULL;
    check(sidereon_antex_parse(encoded, written, &roundtrip) == SIDEREON_STATUS_OK &&
              roundtrip != NULL,
          "antex encoded parse");
    if (roundtrip) {
        size_t after_count = 0;
        check(sidereon_antex_antenna_count(roundtrip, &after_count) == SIDEREON_STATUS_OK &&
                  after_count == before_count &&
                  after_count == W6_CC_ANTEX_REREAD_ANTENNAS,
              "antex encoded count");
        sidereon_antex_free(roundtrip);
    }

    free(encoded);
    sidereon_antex_free(antex);
}

static void test_rinex_obs_helpers(const char *rinex_path) {
    size_t len = 0;
    uint8_t *bytes = read_file(rinex_path, &len);
    if (!bytes) {
        return;
    }
    SidereonRinexObs *obs = NULL;
    check(sidereon_rinex_obs_parse(bytes, len, &obs) == SIDEREON_STATUS_OK && obs != NULL,
          "rinex obs parse");
    free(bytes);
    if (!obs) {
        return;
    }

    union {
        double alignment;
        uint8_t bytes[sizeof(SidereonSsrCorrectedStateResult) + 64];
    } aliased_values;
    memset(aliased_values.bytes, 0xB4, sizeof aliased_values.bytes);
    uint8_t output_before[sizeof aliased_values.bytes];
    memcpy(output_before, aliased_values.bytes, sizeof output_before);
    check(sidereon_rinex_obs_observation(
              obs, 0, W6_CC_OBS_VALUE0_SAT, W6_CC_OBS_CODE0,
              (double *)(void *)aliased_values.bytes,
              (bool *)(void *)aliased_values.bytes,
              (int32_t *)(void *)aliased_values.bytes,
              (int32_t *)(void *)(aliased_values.bytes + 32)) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(output_before, aliased_values.bytes, sizeof output_before) == 0,
          "rinex observation refuses overlapping output fields before writes");
    double observed_value = 0.0;
    bool observed_present = false;
    int32_t lli = -1, ssi = -1;
    check(sidereon_rinex_obs_observation(
              obs, 0, W6_CC_OBS_VALUE0_SAT, W6_CC_OBS_CODE0,
              &observed_value, &observed_present, &lli, &ssi) == SIDEREON_STATUS_OK &&
              observed_present,
          "rinex observation accepts distinct output fields");

    SidereonRinexObsHeader header;
    check(sidereon_rinex_obs_header(obs, &header) == SIDEREON_STATUS_OK &&
              same_f64(header.version, W6_CC_OBS_VERSION_BITS) &&
              header.obs_code_count == W6_CC_OBS_CODE_COUNT,
          "rinex obs header");

    size_t written = 0, required = 0;
    check(sidereon_rinex_obs_codes(obs, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
              required == header.obs_code_count && required > 0,
          "rinex obs codes query");
    SidereonRinexObsCode *codes = (SidereonRinexObsCode *)calloc(required, sizeof(*codes));
    if (codes) {
        check(sidereon_rinex_obs_codes(obs, codes, required, &written, &required) ==
                  SIDEREON_STATUS_OK &&
                  written == required && written == W6_CC_OBS_CODE_COUNT &&
                  token_is((const char *)codes[0].code, sizeof(codes[0].code), W6_CC_OBS_CODE0),
              "rinex obs codes copy");
        free(codes);
    } else {
        check(0, "rinex obs codes allocation");
    }

    size_t epoch_count = 0;
    check(sidereon_rinex_obs_epoch_count(obs, &epoch_count) == SIDEREON_STATUS_OK &&
              epoch_count == W6_CC_OBS_EPOCH_COUNT,
          "rinex obs epoch count");
    check(sidereon_rinex_obs_epochs(obs, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
              required == epoch_count,
          "rinex obs epochs query");
    SidereonRinexObsEpoch *epochs =
        (SidereonRinexObsEpoch *)calloc(required, sizeof(*epochs));
    if (epochs) {
        check(sidereon_rinex_obs_epochs(obs, epochs, required, &written, &required) ==
                  SIDEREON_STATUS_OK &&
                  written == required && epochs[0].satellite_count == W6_CC_OBS_EPOCH0_SATS,
              "rinex obs epochs copy");
        free(epochs);
    } else {
        check(0, "rinex obs epochs allocation");
    }

    check(sidereon_rinex_obs_values(obs, 0, NULL, 0, &written, &required) ==
              SIDEREON_STATUS_OK &&
              required == W6_CC_OBS_VALUE_COUNT,
          "rinex obs values query");
    SidereonRinexObsValue *values = (SidereonRinexObsValue *)calloc(required, sizeof(*values));
    if (values) {
        check(sidereon_rinex_obs_values(obs, 0, values, required, &written, &required) ==
                  SIDEREON_STATUS_OK &&
                  written == required &&
                  token_is(values[0].sat_id.bytes, sizeof(values[0].sat_id.bytes),
                           W6_CC_OBS_VALUE0_SAT),
              "rinex obs values copy");
        free(values);
    } else {
        check(0, "rinex obs values allocation");
    }

    check(sidereon_rinex_obs_pseudoranges(obs, 0, NULL, 0, &written, &required) ==
              SIDEREON_STATUS_OK &&
              required == W6_CC_OBS_PR_COUNT,
          "rinex obs pseudoranges query");
    SidereonRinexObsPseudorange *prs =
        (SidereonRinexObsPseudorange *)calloc(required, sizeof(*prs));
    if (prs) {
        check(sidereon_rinex_obs_pseudoranges(obs, 0, prs, required, &written, &required) ==
                  SIDEREON_STATUS_OK &&
                  written == required &&
                  token_is(prs[0].sat_id.bytes, sizeof(prs[0].sat_id.bytes), W6_CC_OBS_PR0_SAT) &&
                  same_f64(prs[0].pseudorange_m, W6_CC_OBS_PR0_BITS),
              "rinex obs pseudoranges copy");
        free(prs);
    } else {
        check(0, "rinex obs pseudoranges allocation");
    }

    check(sidereon_rinex_obs_carrier_phase(obs, 0, NULL, 0, &written, &required) ==
              SIDEREON_STATUS_OK &&
              required == W6_CC_OBS_PHASE_COUNT,
          "rinex obs carrier phase query");
    SidereonRinexObsCarrierPhase *phase =
        (SidereonRinexObsCarrierPhase *)calloc(required, sizeof(*phase));
    if (phase) {
        check(sidereon_rinex_obs_carrier_phase(obs, 0, phase, required, &written, &required) ==
                  SIDEREON_STATUS_OK &&
                  written == required &&
                  token_is((const char *)phase[0].code, sizeof(phase[0].code),
                           W6_CC_OBS_PHASE0_CODE),
              "rinex obs carrier phase copy");
        free(phase);
    } else {
        check(0, "rinex obs carrier phase allocation");
    }

    sidereon_rinex_obs_free(obs);
}

static void test_sp3_clock_reference(SidereonSp3 *sp3) {
    size_t written = 0, required = 0;
    check(sidereon_sp3_clock_reference_offsets(sp3, sp3, 3, NULL, 0, &written, &required) ==
              SIDEREON_STATUS_OK &&
              required == W6_CC_OFFSET_COUNT,
          "sp3 clock reference offsets query");
    SidereonSp3ClockReferenceOffset *offsets =
        (SidereonSp3ClockReferenceOffset *)calloc(required, sizeof(*offsets));
    if (offsets) {
        check(sidereon_sp3_clock_reference_offsets(sp3, sp3, 3, offsets, required, &written,
                                                   &required) == SIDEREON_STATUS_OK &&
                  written == required && same_f64(offsets[0].offset_s, W6_CC_OFFSET0_BITS) &&
                  offsets[0].satellites == W6_CC_OFFSET0_SATELLITES,
              "sp3 clock reference offsets copy");
        free(offsets);
    } else {
        check(0, "sp3 clock reference offsets allocation");
    }

    SidereonSp3 *aligned = NULL;
    check(sidereon_sp3_align_clock_reference(sp3, sp3, 3, &aligned) == SIDEREON_STATUS_OK &&
              aligned != NULL,
          "sp3 align clock reference");
    if (aligned) {
        check(sidereon_sp3_clock_reference_offsets(sp3, aligned, 3, NULL, 0, &written,
                                                   &required) == SIDEREON_STATUS_OK &&
                  required == W6_CC_ALIGNED_OFFSET_COUNT,
              "sp3 aligned offsets query");
        sidereon_sp3_free(aligned);
    }
}

static void test_reduced_orbit_source(SidereonSp3 *sp3) {
    SidereonReducedOrbitSourceFitOptions fit_options;
    memset(&fit_options, 0, sizeof(fit_options));
    fit_options.sampling.t0 = (SidereonCalendarEpoch){2020, 6, 24, 0, 0, 0.0};
    fit_options.sampling.t1 = (SidereonCalendarEpoch){2020, 6, 24, 3, 0, 0.0};
    fit_options.sampling.cadence_s = 900.0;
    fit_options.model = (uint32_t)SIDEREON_REDUCED_ORBIT_MODEL_CIRCULAR_SECULAR;

    SidereonReducedOrbitElements elements;
    SidereonReducedOrbitSourceFitStats stats;
    check(sidereon_reduced_orbit_fit_sp3_source(sp3, "G01", &fit_options, &elements, &stats) ==
              SIDEREON_STATUS_OK &&
              stats.fit.n_samples == W6_CC_FIT_N_SAMPLES &&
              stats.requested_samples == W6_CC_FIT_REQUESTED &&
              same_f64(stats.fit.rms_m, W6_CC_FIT_RMS_BITS) &&
              same_f64(stats.fit.max_m, W6_CC_FIT_MAX_BITS) &&
              same_f64(elements.a_m, W6_CC_FIT_ELEMENT_BITS[0]) &&
              same_f64(elements.e, W6_CC_FIT_ELEMENT_BITS[1]) &&
              same_f64(elements.i_rad, W6_CC_FIT_ELEMENT_BITS[2]) &&
              same_f64(elements.raan_rad, W6_CC_FIT_ELEMENT_BITS[3]) &&
              same_f64(elements.raan_rate_rad_s, W6_CC_FIT_ELEMENT_BITS[4]) &&
              same_f64(elements.raan_rate_j2_rad_s, W6_CC_FIT_ELEMENT_BITS[5]) &&
              same_f64(elements.arg_lat_rad, W6_CC_FIT_ELEMENT_BITS[6]) &&
              same_f64(elements.mean_motion_rad_s, W6_CC_FIT_ELEMENT_BITS[7]) &&
              same_f64(elements.h, W6_CC_FIT_ELEMENT_BITS[8]) &&
              same_f64(elements.k, W6_CC_FIT_ELEMENT_BITS[9]) &&
              same_f64(elements.arg_perigee_rad, W6_CC_FIT_ELEMENT_BITS[10]),
          "reduced orbit sp3 source fit");

    SidereonReducedOrbitSourceDriftOptions drift_options;
    memset(&drift_options, 0, sizeof(drift_options));
    drift_options.sampling.t0 = (SidereonCalendarEpoch){2020, 6, 24, 0, 0, 0.0};
    drift_options.sampling.t1 = (SidereonCalendarEpoch){2020, 6, 24, 4, 0, 0.0};
    drift_options.sampling.cadence_s = 900.0;
    drift_options.threshold_m = 1.0e9;

    SidereonReducedOrbitDriftReport *report = NULL;
    check(sidereon_reduced_orbit_drift_sp3_source(&elements, sp3, "G01", &drift_options,
                                                  &report) == SIDEREON_STATUS_OK &&
              report != NULL,
          "reduced orbit sp3 source drift");
    if (report) {
        SidereonReducedOrbitDriftSummary summary;
        size_t requested = 0;
        check(sidereon_reduced_orbit_drift_report_summary(report, &summary) ==
                  SIDEREON_STATUS_OK &&
                  same_f64(summary.max_m, W6_CC_DRIFT_MAX_BITS) &&
                  same_f64(summary.rms_m, W6_CC_DRIFT_RMS_BITS),
              "reduced orbit source drift summary");
        check(sidereon_reduced_orbit_drift_report_requested_samples(report, &requested) ==
                  SIDEREON_STATUS_OK &&
                  requested == W6_CC_DRIFT_REQUESTED,
              "reduced orbit source drift requested samples");
        sidereon_reduced_orbit_drift_report_free(report);
    }
}

static void test_dgnss_position(SidereonSp3 *sp3) {
    double t_rx = bits_to_f64(W6_CC_DGNSS_T_RX_BITS);
    double base[3] = {1130773.0, -4831253.0, 3994200.0};
    double rover[3] = {1130833.0, -4831203.0, 3994230.0};

    /* Code observations from the product's own predicted ranges and clocks at
     * both receivers, built by tests/valgen (w6_core_caps). */
    check(same_f64(first_sp3_epoch(sp3) + 3600.0, W6_CC_DGNSS_T_RX_BITS),
          "dgnss receive time is the first epoch plus one hour");
    size_t count = W6_CC_DGNSS_COUNT;
    SidereonCodeObservation base_obs[W6_CC_DGNSS_COUNT];
    SidereonCodeObservation rover_obs[W6_CC_DGNSS_COUNT];
    for (size_t i = 0; i < count; i++) {
        base_obs[i].sat_id = W6_CC_DGNSS_SATS[i];
        base_obs[i].pseudorange_m = bits_to_f64(W6_CC_DGNSS_BASE_PR_BITS[i]);
        rover_obs[i].sat_id = W6_CC_DGNSS_SATS[i];
        rover_obs[i].pseudorange_m = bits_to_f64(W6_CC_DGNSS_ROVER_PR_BITS[i]);
    }

    SidereonDgnssCorrections *corrections = NULL;
    check(sidereon_dgnss_pseudorange_corrections(
              sp3, base, base_obs, count, t_rx, &corrections) == SIDEREON_STATUS_OK &&
              corrections != NULL,
          "dgnss correction source setup");
    if (corrections != NULL) {
        union {
            double alignment;
            uint8_t bytes[sizeof(double)];
        } aliased_correction;
        memset(aliased_correction.bytes, 0xC1, sizeof aliased_correction.bytes);
        uint8_t correction_before[sizeof aliased_correction.bytes];
        memcpy(correction_before, aliased_correction.bytes, sizeof correction_before);
        check(sidereon_dgnss_correction(
                  corrections, W6_CC_DGNSS_SATS[0],
                  (double *)(void *)aliased_correction.bytes,
                  (bool *)(void *)aliased_correction.bytes) ==
                  SIDEREON_STATUS_INVALID_ARGUMENT &&
                  memcmp(correction_before, aliased_correction.bytes,
                         sizeof correction_before) == 0,
              "dgnss correction refuses overlapping value and presence outputs");
        double correction = 0.0;
        bool correction_present = false;
        check(sidereon_dgnss_correction(corrections, W6_CC_DGNSS_SATS[0],
                                        &correction, &correction_present) ==
                  SIDEREON_STATUS_OK && correction_present,
              "dgnss correction accepts distinct outputs");
        sidereon_dgnss_corrections_free(corrections);
    }

    SidereonSppInputsV2 inputs;
    check(sidereon_spp_inputs_v2_init(&inputs) == SIDEREON_STATUS_OK, "dgnss inputs init");
    inputs.base.t_rx_j2000_s = t_rx;
    inputs.base.t_rx_second_of_day_s = 3600.0;
    inputs.base.day_of_year = 176.0;
    inputs.base.initial_guess[0] = rover[0] + 20.0;
    inputs.base.initial_guess[1] = rover[1] - 20.0;
    inputs.base.initial_guess[2] = rover[2] + 10.0;
    inputs.base.initial_guess[3] = 0.0;
    inputs.base.with_geodetic = false;

    SidereonDgnssSolution *solution = NULL;
    check(sidereon_dgnss_position_solve(sp3, base, base_obs, count, rover_obs, count, &inputs,
                                        &solution) == SIDEREON_STATUS_OK &&
              solution != NULL,
          "dgnss position solve");
    if (!solution) {
        return;
    }

    double baseline_vec[3] = {0.0, 0.0, 0.0};
    double baseline_m = 0.0;
    check(sidereon_dgnss_solution_baseline(solution, baseline_vec, 3, &baseline_m) ==
                  SIDEREON_STATUS_OK &&
              same_f64(baseline_m, W6_CC_DGNSS_BASELINE_BITS) &&
              same_f64(baseline_vec[0], W6_CC_DGNSS_BASELINE_VECTOR_BITS[0]) &&
              same_f64(baseline_vec[1], W6_CC_DGNSS_BASELINE_VECTOR_BITS[1]) &&
              same_f64(baseline_vec[2], W6_CC_DGNSS_BASELINE_VECTOR_BITS[2]),
          "dgnss solution baseline");

    SidereonSppSolution *spp = NULL;
    check(sidereon_dgnss_solution_solution(solution, &spp) == SIDEREON_STATUS_OK && spp != NULL,
          "dgnss embedded spp solution");
    if (spp) {
        double position[3] = {0.0, 0.0, 0.0};
        check(sidereon_spp_solution_position(spp, position, 3) == SIDEREON_STATUS_OK &&
                  same_f64(position[0], W6_CC_DGNSS_POSITION_BITS[0]) &&
                  same_f64(position[1], W6_CC_DGNSS_POSITION_BITS[1]) &&
                  same_f64(position[2], W6_CC_DGNSS_POSITION_BITS[2]),
              "dgnss embedded spp position");
        sidereon_spp_solution_free(spp);
    }

    size_t written = 0, required = 0;
    check(sidereon_dgnss_solution_dropped_sats(solution, NULL, 0, &written, &required) ==
                  SIDEREON_STATUS_OK &&
              required == W6_CC_DGNSS_DROPPED,
          "dgnss dropped sats");
    sidereon_dgnss_solution_free(solution);

    SidereonDgnssSolution *failed_solution = (SidereonDgnssSolution *)(uintptr_t)1;
    const SidereonStatus failed_status = sidereon_dgnss_position_solve(
        sp3, base, base_obs, count, rover_obs, 0, &inputs, &failed_solution);
    SidereonEngineErrorInfo failed_info;
    memset(&failed_info, 0, sizeof failed_info);
    size_t payload_written = 0;
    size_t payload_required = 0;
    const SidereonStatus info_status = sidereon_last_engine_error_info(&failed_info);
    const SidereonStatus payload_query_status = sidereon_last_engine_error_payload(
        NULL, 0, &payload_written, &payload_required);
    check(failed_status == SIDEREON_STATUS_SOLVE && failed_solution == NULL &&
              info_status == SIDEREON_STATUS_OK &&
              failed_info.family == SIDEREON_ENGINE_ERROR_FAMILY_DGNSS &&
              payload_query_status == SIDEREON_STATUS_OK && payload_written == 0 &&
              payload_required == failed_info.payload_len,
          "dgnss nested SPP refusal status and family");
    uint8_t *payload = (uint8_t *)malloc(payload_required + 1);
    check(payload != NULL && sidereon_last_engine_error_payload(
                                  payload, payload_required, &payload_written,
                                  &payload_required) == SIDEREON_STATUS_OK &&
              payload_written == payload_required,
          "dgnss nested SPP payload retained");
    if (payload != NULL) {
        payload[payload_written] = 0;
        check(strstr((const char *)payload, "\"schema_version\":1") != NULL &&
                  strstr((const char *)payload, "\"family\":\"dgnss\"") != NULL &&
                  strstr((const char *)payload,
                         "\"operation\":\"sidereon_dgnss_position_solve\"") != NULL &&
                  strstr((const char *)payload, "\"kind\":\"spp\"") != NULL &&
                  strstr((const char *)payload, "\"kind\":\"too_few_satellites\"") != NULL &&
                  strstr((const char *)payload, "\"used\":0") != NULL &&
                  strstr((const char *)payload, "\"required\":4") != NULL,
              "dgnss full nested SPP cause");
        free(payload);
    }
    char failed_message[256] = {0};
    (void)sidereon_last_error_message(failed_message, sizeof failed_message);
    check(strcmp(failed_message,
                 "sidereon_dgnss_position_solve: only 0 usable satellites; need at least 4 "
                 "(3 position + 1 clock per GNSS)") == 0,
          "dgnss nested SPP exact legacy message");
}

static void test_remaining_multi_output_preflights(void) {
    union {
        size_t alignment;
        uint8_t bytes[sizeof(SidereonSourceCovariance)];
    } source_outputs;
    memset(source_outputs.bytes, 0xD2, sizeof source_outputs.bytes);
    uint8_t source_before[sizeof source_outputs.bytes];
    memcpy(source_before, source_outputs.bytes, sizeof source_before);
    check(sidereon_source_solution_covariance(
              NULL, (SidereonSourceCovariance *)(void *)source_outputs.bytes,
              (bool *)(void *)source_outputs.bytes) == SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(source_before, source_outputs.bytes, sizeof source_before) == 0,
          "source covariance rejects overlapping outputs before dereferencing handle");
    SidereonSourceCovariance covariance;
    bool covariance_available = false;
    check(sidereon_source_solution_covariance(NULL, &covariance,
                                             &covariance_available) ==
              SIDEREON_STATUS_NULL_POINTER,
          "source covariance reaches handle validation for distinct outputs");

    union {
        double alignment;
        uint8_t bytes[64];
    } ssr_outputs;
    memset(ssr_outputs.bytes, 0xE3, sizeof ssr_outputs.bytes);
    uint8_t ssr_before[sizeof ssr_outputs.bytes];
    memcpy(ssr_before, ssr_outputs.bytes, sizeof ssr_before);
    SidereonSsrCorrectedStateResult policy_sentinel = {0};
    check(sidereon_ssr_transmit_epoch_clock_at_epoch_queries(
              NULL, NULL, "G01", NULL, NULL, 0.0, 0, false, 0, 0,
              (bool *)(void *)ssr_outputs.bytes,
              (double *)(void *)ssr_outputs.bytes,
              (bool *)(void *)(ssr_outputs.bytes + 16),
              &policy_sentinel) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(ssr_before, ssr_outputs.bytes, sizeof ssr_before) == 0,
          "ssr query refuses overlapping output fields before writes");
    bool has_clock = false, degraded = false;
    double clock_s = 0.0;
    SidereonSsrCorrectedStateResult policy_result = {0};
    check(sidereon_ssr_transmit_epoch_clock_at_epoch_queries(
              NULL, NULL, "G01", NULL, NULL, 0.0, 0, false, 0, 0,
              &has_clock, &clock_s, &degraded, &policy_result) ==
              SIDEREON_STATUS_NULL_POINTER,
          "ssr query reaches handle validation for distinct outputs");
}

static double rtk_range_m(const double position[3], const double receiver[3]) {
    const double dx = position[0] - receiver[0];
    const double dy = position[1] - receiver[1];
    const double dz = position[2] - receiver[2];
    return sqrt(dx * dx + dy * dy + dz * dz);
}

static void test_rtk_residual_refusal(void) {
    const double base[3] = {4075580.0, 931854.0, 4801568.0};
    const double truth[3] = {1.2, -0.85, 0.91};
    double rover[3];
    for (size_t axis = 0; axis < 3; axis++) rover[axis] = base[axis] + truth[axis];
    const char *const ids[5] = {"G01", "G02", "G03", "G04", "G05"};
    const double positions[5][3] = {
        {15000000.0, 7000000.0, 21000000.0},
        {-12000000.0, 18000000.0, 19000000.0},
        {20000000.0, -10000000.0, 17000000.0},
        {-19000000.0, -13000000.0, 20000000.0},
        {9000000.0, 22000000.0, 16000000.0},
    };
    const int64_t cycles[5] = {0, 4, -7, 9, -3};
    const double lambda = 299792458.0 / 1575.42e6;
    const double g05_noise[3] = {40.0, -40.0, 40.0};
    SidereonRtkSatMeasurement rows[3][5];
    SidereonRtkEpoch epochs[3];
    memset(rows, 0, sizeof rows);
    memset(epochs, 0, sizeof epochs);
    for (size_t epoch = 0; epoch < 3; epoch++) {
        for (size_t sat = 0; sat < 5; sat++) {
            SidereonRtkSatMeasurement *row = &rows[epoch][sat];
            row->sat_id = ids[sat];
            row->sd_ambiguity_id = ids[sat];
            const double base_range = rtk_range_m(positions[sat], base);
            const double rover_range = rtk_range_m(positions[sat], rover);
            row->base_code_m = base_range;
            row->base_phase_m = base_range;
            row->rover_code_m = rover_range + (sat == 4 ? g05_noise[epoch] : 0.0);
            row->rover_phase_m = rover_range + (double)cycles[sat] * lambda;
            memcpy(row->base_tx_pos, positions[sat], sizeof row->base_tx_pos);
            memcpy(row->rover_tx_pos, positions[sat], sizeof row->rover_tx_pos);
            memcpy(row->pos, positions[sat], sizeof row->pos);
        }
        epochs[epoch].references = &rows[epoch][0];
        epochs[epoch].reference_count = 1;
        epochs[epoch].nonref = &rows[epoch][1];
        epochs[epoch].nonref_count = 4;
        epochs[epoch].dt_s = 0.0;
    }

    const char *const ambiguity_ids[4] = {"G02", "G03", "G04", "G05"};
    SidereonRtkAmbiguitySatellite ambiguity_satellites[4];
    SidereonRtkFloatMapEntry wavelengths[4];
    SidereonRtkFloatMapEntry offsets[4];
    for (size_t i = 0; i < 4; i++) {
        ambiguity_satellites[i].id = ambiguity_ids[i];
        ambiguity_satellites[i].sat_id = ambiguity_ids[i];
        wavelengths[i].id = ambiguity_ids[i];
        wavelengths[i].value = lambda;
        offsets[i].id = ambiguity_ids[i];
        offsets[i].value = 0.0;
    }

    SidereonRtkFixedConfig config;
    memset(&config, 0, sizeof config);
    config.epochs = epochs;
    config.epoch_count = 3;
    memcpy(config.base_ecef_m, base, sizeof config.base_ecef_m);
    config.ambiguity_ids = ambiguity_ids;
    config.ambiguity_id_count = 4;
    config.ambiguity_satellites = ambiguity_satellites;
    config.ambiguity_satellite_count = 4;
    config.wavelengths_m = wavelengths;
    config.wavelength_count = 4;
    config.offsets_m = offsets;
    config.offset_count = 4;
    check(sidereon_rtk_measurement_model_init(&config.model) == SIDEREON_STATUS_OK,
          "RTK residual model init");
    config.model.code_sigma_m = 0.3;
    config.model.phase_sigma_m = 0.003;
    config.model.sagnac = false;
    config.model.stochastic = SIDEREON_RTK_STOCHASTIC_MODEL_SIMPLE;
    config.model.elevation_weighting = false;
    check(sidereon_rtk_float_options_init(&config.float_options) == SIDEREON_STATUS_OK,
          "RTK residual float options init");
    config.float_options.position_tol_m = 1.0e-3;
    config.float_options.ambiguity_tol_m = 1.0e-6;
    config.float_options.max_iterations = 10;
    check(sidereon_rtk_fixed_options_init(&config.fixed_options) == SIDEREON_STATUS_OK,
          "RTK residual fixed options init");
    config.fixed_options.position_tol_m = 1.0e-3;
    config.fixed_options.ambiguity_tol_m = 1.0e-6;
    config.fixed_options.max_iterations = 10;
    config.fixed_options.ratio_threshold = 3.0;
    config.fixed_options.partial_ambiguity_resolution = false;
    config.fixed_options.partial_min_ambiguities = 4;
    check(sidereon_rtk_residual_validation_options_init(&config.residual_options) ==
              SIDEREON_STATUS_OK,
          "RTK residual options init");
    config.residual_options.threshold_sigma_enabled = true;
    config.residual_options.threshold_sigma = 6.0;
    config.residual_options.max_exclusions = 0;
    config.initial_baseline_m[0] = -30.0;
    config.initial_baseline_m[1] = 25.0;
    config.initial_baseline_m[2] = -10.0;

    SidereonRtkFixedSolution *solution = (SidereonRtkFixedSolution *)(uintptr_t)1;
    const SidereonStatus status = sidereon_solve_rtk_fixed(&config, &solution);
    SidereonEngineErrorInfo info;
    memset(&info, 0, sizeof info);
    size_t written = 0, required = 0;
    const SidereonStatus info_status = sidereon_last_engine_error_info(&info);
    const SidereonStatus query_status = sidereon_last_engine_error_payload(NULL, 0, &written, &required);
    check(status == SIDEREON_STATUS_SOLVE && solution == NULL &&
              info_status == SIDEREON_STATUS_OK && info.family == SIDEREON_ENGINE_ERROR_FAMILY_FACADE &&
              query_status == SIDEREON_STATUS_OK && written == 0 && required == info.payload_len,
          "public RTK residual refusal status and family");

    uint8_t *payload = (uint8_t *)malloc(required + 1);
    check(payload != NULL &&
              sidereon_last_engine_error_payload(payload, required, &written, &required) ==
                  SIDEREON_STATUS_OK && written == required,
          "public RTK residual payload retained");
    if (payload != NULL) {
        payload[written] = 0;
        const char *text = (const char *)payload;
        double residual_m = 0.0;
        double sigma_m = 0.0;
        double normalized_residual = 0.0;
        double threshold_sigma = 0.0;
        const char *epoch_field = strstr(text, "\"epoch_index\":");
        const char *epoch_value = epoch_field == NULL
                                      ? NULL
                                      : epoch_field + strlen("\"epoch_index\":");
        char *epoch_end = NULL;
        const unsigned long epoch_index =
            epoch_value == NULL ? 99UL : strtoul(epoch_value, &epoch_end, 10);
        char normalized_text[96];
        const bool numeric_fields =
            payload_decimal(text, "residual_m", &residual_m) &&
            payload_decimal(text, "sigma_m", &sigma_m) &&
            payload_decimal(text, "normalized_residual", &normalized_residual) &&
            payload_decimal(text, "threshold_sigma", &threshold_sigma) &&
            payload_decimal_text(text, "normalized_residual", normalized_text,
                                 sizeof normalized_text);
        const double division_roundoff =
            2.0 * 2.2204460492503131e-16 * fmax(1.0, fabs(residual_m / sigma_m));
        check(strstr(text, "\"schema_version\":1") != NULL &&
                  strstr(text, "\"family\":\"facade\"") != NULL &&
                  strstr(text, "\"operation\":\"sidereon_solve_rtk_fixed\"") != NULL &&
                  strstr(text, "\"kind\":\"rtk_fixed\"") != NULL &&
                  strstr(text, "\"kind\":\"residual_validation_failed\"") != NULL &&
                  epoch_value != NULL && epoch_end != epoch_value && epoch_index < 3UL &&
                  strstr(text, "\"satellite_id\":\"G05\"") != NULL &&
                  strstr(text, "\"reference_satellite_id\":\"G01\"") != NULL &&
                  strstr(text, "\"ambiguity_id\":\"G05\"") != NULL &&
                  strstr(text, "\"component\":\"code\"") != NULL &&
                  numeric_fields && sigma_m > 0.0 && threshold_sigma == 6.0 &&
                  fabs(normalized_residual - residual_m / sigma_m) <= division_roundoff &&
                  fabs(normalized_residual) > threshold_sigma &&
                  strstr(text, "\"exclusions\":[]") != NULL,
              "public RTK residual full typed payload");
        char expected_message[512] = {0};
        (void)snprintf(expected_message, sizeof expected_message,
                       "fixed RTK residual validation failed on G05 code residual "
                       "for satellite G05 against reference G01 with normalized residual %s; "
                       "0 exclusions were already tried",
                       numeric_fields ? normalized_text : "invalid");
        char actual_message[512] = {0};
        (void)sidereon_last_error_message(actual_message, sizeof actual_message);
        check(numeric_fields && strcmp(actual_message, expected_message) == 0,
              "public RTK residual exact legacy message");
        free(payload);
    }
}

static void test_output_alias_rejection(void) {
    double offset = 0.0;
    check(sidereon_timescale_offset_s(SIDEREON_TIME_SCALE_TCG,
                                      SIDEREON_TIME_SCALE_UTC, &offset) ==
              SIDEREON_STATUS_INVALID_ARGUMENT,
          "alias test seeds a typed engine payload");

    size_t aliased_count = 0x1234U;
    check(sidereon_last_engine_error_payload(NULL, 0, &aliased_count,
                                             &aliased_count) ==
              SIDEREON_STATUS_INVALID_ARGUMENT && aliased_count == 0x1234U,
          "reject aliased count outputs before writing");

    size_t written = 0, required = 0;
    check(sidereon_last_engine_error_payload(NULL, 0, &written, &required) ==
              SIDEREON_STATUS_OK && required > sizeof(size_t),
          "distinct count query remains valid");
    uint8_t zero_len_marker = 0x7CU;
    size_t zero_written = 0, zero_required = 0;
    check(sidereon_last_engine_error_payload(&zero_len_marker, 0, &zero_written,
                                             &zero_required) ==
              SIDEREON_STATUS_INVALID_ARGUMENT && zero_written == 0 &&
              zero_required == required && zero_len_marker == 0x7CU,
          "non-null zero-length short buffer is refused without writing");
    union {
        size_t alignment;
        uint8_t bytes[4096];
    } storage;
    memset(storage.bytes, 0xA5, sizeof storage.bytes);
    uint8_t before[sizeof(size_t)];
    memcpy(before, storage.bytes, sizeof before);
    check(required <= sizeof storage.bytes &&
              sidereon_last_engine_error_payload(
                  storage.bytes, required, (size_t *)storage.bytes, &required) ==
                  SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(storage.bytes, before, sizeof before) == 0,
          "reject overlapping payload and count before writing");

    uint8_t *short_payload = (uint8_t *)malloc(required);
    check(short_payload != NULL, "short payload allocation");
    if (short_payload != NULL) {
        memset(short_payload, 0x3D, required);
        size_t short_written = 99, short_required = 0;
        check(sidereon_last_engine_error_payload(
                  short_payload, required - 1, &short_written, &short_required) ==
                  SIDEREON_STATUS_INVALID_ARGUMENT && short_written == 0 &&
                  short_required == required && short_payload[0] == 0x3D,
              "short output reports required count without writing payload");
        free(short_payload);
    }

    uint8_t *payload = (uint8_t *)malloc(required + 1);
    check(payload != NULL, "distinct payload allocation");
    if (payload != NULL) {
        check(sidereon_last_engine_error_payload(payload, required, &written,
                                                 &required) ==
                  SIDEREON_STATUS_OK && written == required,
              "distinct payload buffer remains valid");
        free(payload);
    }
}

static void test_rinex_parse_result_failure_resets_outputs(void) {
    SidereonRinexClock *clock = (SidereonRinexClock *)(uintptr_t)1;
    SidereonRinexClockResult *result = (SidereonRinexClockResult *)(uintptr_t)1;
    const enum SidereonStatus status =
        sidereon_rinex_clock_parse_result(NULL, 1, &clock, &result);
    check(status == SIDEREON_STATUS_NULL_POINTER && clock == NULL && result == NULL,
          "RINEX parse-result input refusal clears both output slots");
}

static void test_inertial_dual_output_alias_rejection(void) {
    SidereonFusionImuSpec spec = {0};
    spec.accel_bias_tau_s = 1.0;
    spec.gyro_bias_tau_s = 1.0;
    SidereonInertialSimulationOptions options = {0};
    options.output = 0;
    options.accel_scale_misalignment[0] = 1.0;
    options.accel_scale_misalignment[4] = 1.0;
    options.accel_scale_misalignment[8] = 1.0;
    options.gyro_scale_misalignment[0] = 1.0;
    options.gyro_scale_misalignment[4] = 1.0;
    options.gyro_scale_misalignment[8] = 1.0;
    SidereonInertialIncrement increment = {0};
    increment.t_j2000_s = 1.0;
    increment.dt_s = 1.0;
    SidereonInertialNavState trajectory[2] = {{0}};
    trajectory[0].position_ecef_m[0] = 6378137.0;
    trajectory[0].attitude_body_to_ecef[0] = 1.0;
    trajectory[0].attitude_body_to_ecef[4] = 1.0;
    trajectory[0].attitude_body_to_ecef[8] = 1.0;
    trajectory[1] = trajectory[0];
    trajectory[1].t_j2000_s = 1.0;

    union {
        size_t alignment;
        uint8_t bytes[2048];
    } storage;
    uint8_t before[sizeof(SidereonFusionImuSample)];
    memset(storage.bytes, 0x5A, sizeof storage.bytes);
    memcpy(before, storage.bytes, sizeof before);
    check(sidereon_inertial_simulate_increments(
              &increment, 1, &spec, &options,
              (SidereonFusionImuSample *)storage.bytes,
              (SidereonInertialSimulatorState *)storage.bytes) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(storage.bytes, before, sizeof before) == 0,
          "inertial increments rejects overlapping outputs before writes");
    SidereonFusionImuSample sample = {0};
    SidereonInertialSimulatorState bias = {0};
    check(sidereon_inertial_simulate_increments(
              &increment, 1, &spec, &options, &sample, &bias) ==
              SIDEREON_STATUS_OK,
          "inertial increments accepts distinct outputs");

    memset(storage.bytes, 0x6B, sizeof storage.bytes);
    memcpy(before, storage.bytes, sizeof before);
    check(sidereon_inertial_simulate_trajectory(
              trajectory, 2, &spec, &options,
              (SidereonFusionImuSample *)storage.bytes,
              (SidereonInertialSimulatorState *)storage.bytes) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(storage.bytes, before, sizeof before) == 0,
          "inertial trajectory rejects overlapping outputs before writes");
    check(sidereon_inertial_simulate_trajectory(
              trajectory, 2, &spec, &options, &sample, &bias) ==
              SIDEREON_STATUS_OK,
          "inertial trajectory accepts distinct outputs");
}

static void test_additional_multi_output_aliases(void) {
    double scalar = 123.5;
    check(sidereon_nutation_iau2000a_radians(2451545.0, &scalar, &scalar) ==
              SIDEREON_STATUS_INVALID_ARGUMENT && scalar == 123.5,
          "nutation rejects aliased scalar outputs before writes");

    size_t count = 0x1234U;
    check(sidereon_coverage_grid_dimensions(NULL, &count, &count) ==
              SIDEREON_STATUS_INVALID_ARGUMENT && count == 0x1234U,
          "coverage dimensions rejects aliased counts before handle access");

    double vector[6];
    for (size_t i = 0; i < 6; ++i) vector[i] = 10.0 + (double)i;
    double vector_before[6];
    memcpy(vector_before, vector, sizeof vector);
    check(sidereon_frame_teme_to_gcrs(NULL, NULL, NULL, false, vector, vector) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(vector, vector_before, sizeof vector) == 0,
          "frame transform rejects overlapping vector outputs before writes");
    check(sidereon_iod_gauss_angles(NULL, NULL, NULL, NULL, NULL, vector, vector) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(vector, vector_before, sizeof vector) == 0,
          "IOD rejects overlapping vector outputs before writes");

    union {
        double align;
        uint8_t bytes[16];
    } clock_outputs;
    memset(clock_outputs.bytes, 0xA4, sizeof clock_outputs.bytes);
    uint8_t clock_before[sizeof clock_outputs.bytes];
    memcpy(clock_before, clock_outputs.bytes, sizeof clock_before);
    check(sidereon_clock_power_law_noise_slopes(
              0,
              (double *)(void *)clock_outputs.bytes,
              (double *)(void *)clock_outputs.bytes,
              (int32_t *)(void *)(clock_outputs.bytes + 8)) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(clock_before, clock_outputs.bytes, sizeof clock_before) == 0,
          "clock slopes rejects overlapping outputs before writes");

    union {
        uint16_t align;
        uint8_t bytes[8];
    } constellation_outputs;
    memset(constellation_outputs.bytes, 0xB5, sizeof constellation_outputs.bytes);
    uint8_t constellation_before[sizeof constellation_outputs.bytes];
    memcpy(constellation_before, constellation_outputs.bytes,
           sizeof constellation_before);
    check(sidereon_constellation_galileo_prn_for_gsat(
              210, (bool *)(void *)constellation_outputs.bytes,
              (uint16_t *)(void *)constellation_outputs.bytes) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(constellation_before, constellation_outputs.bytes,
                     sizeof constellation_before) == 0,
          "constellation conversion rejects overlapping outputs");
    check(sidereon_constellation_glonass_slot_for_number(
              1, (bool *)(void *)constellation_outputs.bytes,
              (uint16_t *)(void *)constellation_outputs.bytes) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(constellation_before, constellation_outputs.bytes,
                     sizeof constellation_before) == 0,
          "GLONASS slot conversion rejects overlapping outputs");
    check(sidereon_constellation_glonass_fdma_channel(
              1, (bool *)(void *)constellation_outputs.bytes,
              (int8_t *)(void *)constellation_outputs.bytes) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(constellation_before, constellation_outputs.bytes,
                     sizeof constellation_before) == 0,
          "GLONASS channel conversion rejects overlapping outputs");

    union {
        SidereonCarrierPair align;
        uint8_t bytes[sizeof(SidereonCarrierPair)];
    } frequency_outputs;
    memset(frequency_outputs.bytes, 0xC6, sizeof frequency_outputs.bytes);
    uint8_t frequency_before[sizeof frequency_outputs.bytes];
    memcpy(frequency_before, frequency_outputs.bytes, sizeof frequency_before);
    check(sidereon_default_iono_free_pair(
              SIDEREON_GNSS_SYSTEM_GPS,
              (SidereonCarrierPair *)(void *)frequency_outputs.bytes,
              (bool *)(void *)frequency_outputs.bytes) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(frequency_before, frequency_outputs.bytes,
                     sizeof frequency_before) == 0,
          "default ionosphere-free pair rejects overlapping outputs");

    union {
        SidereonGeodetic align;
        uint8_t bytes[sizeof(SidereonGeodetic)];
    } geodetic_outputs;
    memset(geodetic_outputs.bytes, 0xD7, sizeof geodetic_outputs.bytes);
    uint8_t geodetic_before[sizeof geodetic_outputs.bytes];
    memcpy(geodetic_before, geodetic_outputs.bytes, sizeof geodetic_before);
    check(sidereon_static_position_solution_geodetic(
              NULL, (SidereonGeodetic *)(void *)geodetic_outputs.bytes,
              (bool *)(void *)geodetic_outputs.bytes) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(geodetic_before, geodetic_outputs.bytes,
                     sizeof geodetic_before) == 0,
          "static geodetic accessor rejects overlapping outputs");

    union {
        SidereonStalenessMetadata align;
        uint8_t bytes[sizeof(SidereonStalenessMetadata)];
    } staleness_outputs;
    memset(staleness_outputs.bytes, 0xE8, sizeof staleness_outputs.bytes);
    uint8_t staleness_before[sizeof staleness_outputs.bytes];
    memcpy(staleness_before, staleness_outputs.bytes, sizeof staleness_before);
    check(sidereon_sourced_solution_staleness(
              NULL, (SidereonStalenessMetadata *)(void *)staleness_outputs.bytes,
              (bool *)(void *)staleness_outputs.bytes) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(staleness_before, staleness_outputs.bytes,
                     sizeof staleness_before) == 0,
          "sourced staleness rejects overlapping outputs before handle access");

    union {
        SidereonBroadcastReasonKind align;
        uint8_t bytes[sizeof(SidereonStalenessMetadata)];
    } reason_outputs;
    memset(reason_outputs.bytes, 0xF9, sizeof reason_outputs.bytes);
    uint8_t reason_before[sizeof reason_outputs.bytes];
    memcpy(reason_before, reason_outputs.bytes, sizeof reason_before);
    SidereonStalenessMetadata attempted = {0};
    uint8_t attempted_before[sizeof attempted];
    memcpy(attempted_before, &attempted, sizeof attempted_before);
    bool has_attempted = true;
    check(sidereon_sourced_solution_broadcast_reason(
              NULL, (SidereonBroadcastReasonKind *)(void *)reason_outputs.bytes,
              (SidereonSelectionStatus *)(void *)reason_outputs.bytes,
              &attempted, &has_attempted) == SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(reason_before, reason_outputs.bytes, sizeof reason_before) == 0 &&
              memcmp(attempted_before, &attempted, sizeof attempted_before) == 0 &&
              has_attempted,
          "broadcast reason rejects overlapping outputs before writes");

    union {
        double align;
        uint8_t bytes[32];
    } time_outputs;
    memset(time_outputs.bytes, 0x1A, sizeof time_outputs.bytes);
    uint8_t time_before[sizeof time_outputs.bytes];
    memcpy(time_before, time_outputs.bytes, sizeof time_before);
    check(sidereon_gnss_week_and_seconds_of_week(
              123.0, (double *)(void *)time_outputs.bytes,
              (double *)(void *)time_outputs.bytes) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(time_before, time_outputs.bytes, sizeof time_before) == 0,
          "GNSS week conversion rejects overlapping outputs");
    int64_t jdn_storage = INT64_C(0x0102030405060708);
    int64_t jdn_before = jdn_storage;
    check(sidereon_gnss_week_epoch_julian_day_number(
              SIDEREON_TIME_SCALE_GPST, (bool *)(void *)&jdn_storage,
              &jdn_storage) == SIDEREON_STATUS_INVALID_ARGUMENT &&
              jdn_storage == jdn_before,
          "GNSS week epoch rejects overlapping outputs");
    uint32_t week_storage = UINT32_C(0x12345678);
    uint32_t week_before = week_storage;
    check(sidereon_gnss_week_from_calendar(
              SIDEREON_TIME_SCALE_GPST, 2024, 1, 1,
              (bool *)(void *)&week_storage, &week_storage) ==
              SIDEREON_STATUS_INVALID_ARGUMENT && week_storage == week_before,
          "GNSS calendar week rejects overlapping outputs");
    union {
        double align;
        uint8_t bytes[16];
    } gps_outputs;
    memset(gps_outputs.bytes, 0x4D, sizeof gps_outputs.bytes);
    uint8_t gps_before[sizeof gps_outputs.bytes];
    memcpy(gps_before, gps_outputs.bytes, sizeof gps_before);
    check(sidereon_civil_to_gps_seconds(
              2024, 1, 1, 0, 0, 0.0, (double *)(void *)gps_outputs.bytes,
              (bool *)(void *)gps_outputs.bytes) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(gps_before, gps_outputs.bytes, sizeof gps_before) == 0,
          "civil-to-GPS conversion rejects overlapping outputs");

    union {
        void *align;
        uint8_t bytes[16];
    } handle_outputs;
    memset(handle_outputs.bytes, 0x2B, sizeof handle_outputs.bytes);
    uint8_t handle_before[sizeof handle_outputs.bytes];
    memcpy(handle_before, handle_outputs.bytes, sizeof handle_before);
    check(sidereon_sp3_merge(NULL, 0, NULL,
                             (SidereonSp3 **)(void *)handle_outputs.bytes,
                             (SidereonSp3MergeReport **)(void *)handle_outputs.bytes) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(handle_before, handle_outputs.bytes, sizeof handle_before) == 0,
          "SP3 merge rejects overlapping handle outputs before clearing");

    union {
        SidereonFusionUpdate align;
        uint8_t bytes[sizeof(SidereonFusionUpdate)];
    } fusion_outputs;
    memset(fusion_outputs.bytes, 0x3C, sizeof fusion_outputs.bytes);
    uint8_t fusion_before[sizeof fusion_outputs.bytes];
    memcpy(fusion_before, fusion_outputs.bytes, sizeof fusion_before);
    check(sidereon_fusion_filter_update_stationary(
              NULL, (SidereonFusionUpdate *)(void *)fusion_outputs.bytes,
              (bool *)(void *)fusion_outputs.bytes) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(fusion_before, fusion_outputs.bytes,
                     sizeof fusion_before) == 0,
          "fusion update rejects overlapping outputs before handle access");

    check(sidereon_fusion_filter_update_stationary_recorded(
              NULL, NULL, (SidereonFusionUpdate *)(void *)fusion_outputs.bytes,
              (bool *)(void *)fusion_outputs.bytes) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(fusion_before, fusion_outputs.bytes,
                     sizeof fusion_before) == 0,
          "recorded fusion update rejects overlapping outputs");
    check(sidereon_fusion_filter_update_non_holonomic(
              NULL, (SidereonFusionUpdate *)(void *)fusion_outputs.bytes,
              (bool *)(void *)fusion_outputs.bytes) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(fusion_before, fusion_outputs.bytes,
                     sizeof fusion_before) == 0,
          "non-holonomic fusion update rejects overlapping outputs");
    check(sidereon_fusion_filter_update_non_holonomic_recorded(
              NULL, NULL,
              (SidereonFusionUpdate *)(void *)fusion_outputs.bytes,
              (bool *)(void *)fusion_outputs.bytes) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(fusion_before, fusion_outputs.bytes,
                     sizeof fusion_before) == 0,
          "recorded non-holonomic fusion update rejects overlapping outputs");

    check(sidereon_sp3_source_transmit_epoch_clock_at_epoch_queries(
              NULL, "G01", NULL, NULL,
              (bool *)(void *)time_outputs.bytes,
              (double *)(void *)time_outputs.bytes,
              (bool *)(void *)(time_outputs.bytes + 16)) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(time_before, time_outputs.bytes, sizeof time_before) == 0,
          "SP3 source clock query rejects overlapping outputs");
    union {
        double align;
        uint8_t bytes[32];
    } relativity_outputs;
    memset(relativity_outputs.bytes, 0x5E, sizeof relativity_outputs.bytes);
    uint8_t relativity_before[sizeof relativity_outputs.bytes];
    memcpy(relativity_before, relativity_outputs.bytes, sizeof relativity_before);
    SidereonSsrCorrectedStateResult relativity_policy = {0};
    check(sidereon_sp3_source_clock_relativity_at_epoch_query(
              NULL, "G01", NULL, NULL,
              (SidereonClockRelativityKind *)(void *)relativity_outputs.bytes,
              (double *)(void *)relativity_outputs.bytes) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(relativity_before, relativity_outputs.bytes,
                     sizeof relativity_before) == 0,
          "SP3 clock relativity query rejects overlapping outputs");

    check(sidereon_ssr_corrected_state(
              NULL, NULL, "G01", 0.0, 0.0, 0, false, 0,
              (bool *)(void *)time_outputs.bytes,
              (double *)(void *)time_outputs.bytes,
              (double *)(void *)(time_outputs.bytes + 16)) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(time_before, time_outputs.bytes, sizeof time_before) == 0,
          "SSR corrected state rejects overlapping outputs");
    SidereonSsrCorrectedStateResult ssr_clock_policy = {0};
    check(sidereon_ssr_transmit_epoch_clock_at_epoch_queries(
              NULL, NULL, "G01", NULL, NULL, 0.0, 0, false, 0, 0,
              (bool *)(void *)time_outputs.bytes,
              (double *)(void *)time_outputs.bytes,
              (bool *)(void *)(time_outputs.bytes + 16),
              &ssr_clock_policy) == SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(time_before, time_outputs.bytes, sizeof time_before) == 0,
          "SSR transmit clock query rejects overlapping outputs");
    check(sidereon_ssr_clock_relativity_at_epoch_query(
              NULL, NULL, "G01", NULL, NULL, 0.0, 0, false, 0, 0,
              (SidereonClockRelativityKind *)(void *)relativity_outputs.bytes,
              (double *)(void *)relativity_outputs.bytes,
              &relativity_policy) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(relativity_before, relativity_outputs.bytes,
                     sizeof relativity_before) == 0,
          "SSR clock relativity query rejects overlapping outputs");

    check(sidereon_broadcast_transmit_epoch_clock_at_epoch_queries(
              NULL, "G01", NULL, NULL,
              (bool *)(void *)time_outputs.bytes,
              (double *)(void *)time_outputs.bytes,
              (bool *)(void *)(time_outputs.bytes + 16)) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(time_before, time_outputs.bytes, sizeof time_before) == 0,
          "broadcast clock query rejects overlapping outputs");
    check(sidereon_broadcast_clock_relativity_at_epoch_query(
              NULL, "G01", NULL, NULL,
              (SidereonClockRelativityKind *)(void *)relativity_outputs.bytes,
              (double *)(void *)relativity_outputs.bytes) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(relativity_before, relativity_outputs.bytes,
                     sizeof relativity_before) == 0,
          "broadcast relativity query rejects overlapping outputs");
    union {
        double align_double;
        size_t align_size;
        uint8_t bytes[sizeof(double) + sizeof(size_t)];
    } eccentric_outputs;
    memset(eccentric_outputs.bytes, 0x7D, sizeof eccentric_outputs.bytes);
    uint8_t eccentric_before[sizeof eccentric_outputs.bytes];
    memcpy(eccentric_before, eccentric_outputs.bytes, sizeof eccentric_before);
    check(sidereon_broadcast_eccentric_anomaly(
              0.1, 0.01, (double *)(void *)eccentric_outputs.bytes,
              (size_t *)(void *)eccentric_outputs.bytes) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(eccentric_before, eccentric_outputs.bytes,
                     sizeof eccentric_before) == 0,
          "eccentric anomaly rejects overlapping outputs");
    check(sidereon_broadcast_ephemeris_leap_seconds(
              NULL, (double *)(void *)time_outputs.bytes,
              (bool *)(void *)time_outputs.bytes) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(time_before, time_outputs.bytes, sizeof time_before) == 0,
          "broadcast leap seconds rejects overlapping outputs");
    check(sidereon_precise_interpolant_transmit_epoch_clock_at_epoch_queries(
              NULL, "G01", NULL, NULL,
              (bool *)(void *)time_outputs.bytes,
              (double *)(void *)time_outputs.bytes,
              (bool *)(void *)(time_outputs.bytes + 16)) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(time_before, time_outputs.bytes, sizeof time_before) == 0,
          "precise clock query rejects overlapping outputs");
    check(sidereon_precise_interpolant_clock_relativity_at_epoch_query(
              NULL, "G01", NULL, NULL,
              (SidereonClockRelativityKind *)(void *)relativity_outputs.bytes,
              (double *)(void *)relativity_outputs.bytes) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(relativity_before, relativity_outputs.bytes,
                     sizeof relativity_before) == 0,
          "precise relativity query rejects overlapping outputs");
    check(sidereon_rinex_obs_observation(
              NULL, 0, "G01", "C1C",
              (double *)(void *)time_outputs.bytes,
              (bool *)(void *)time_outputs.bytes,
              (int32_t *)(void *)(time_outputs.bytes + 16),
              (int32_t *)(void *)(time_outputs.bytes + 24)) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              memcmp(time_before, time_outputs.bytes, sizeof time_before) == 0,
          "RINEX observation rejects overlapping outputs");

    SidereonIonexHeader *ionex_header = NULL;
    check(sidereon_ionex_header_new(&ionex_header) == SIDEREON_STATUS_OK &&
              ionex_header != NULL,
          "create IONEX header for output alias controls");
    if (ionex_header != NULL) {
        union {
            uint32_t align;
            uint8_t bytes[8];
        } ionex_outputs;
        memset(ionex_outputs.bytes, 0x6F, sizeof ionex_outputs.bytes);
        uint8_t ionex_before[sizeof ionex_outputs.bytes];
        memcpy(ionex_before, ionex_outputs.bytes, sizeof ionex_before);
        check(sidereon_ionex_header_get_station_count(
                  ionex_header, (uint32_t *)(void *)ionex_outputs.bytes,
                  (bool *)(void *)ionex_outputs.bytes) ==
                  SIDEREON_STATUS_INVALID_ARGUMENT &&
                  memcmp(ionex_before, ionex_outputs.bytes,
                         sizeof ionex_before) == 0,
              "IONEX station count rejects overlapping outputs");
        check(sidereon_ionex_header_get_satellite_count(
                  ionex_header, (uint32_t *)(void *)ionex_outputs.bytes,
                  (bool *)(void *)ionex_outputs.bytes) ==
                  SIDEREON_STATUS_INVALID_ARGUMENT &&
                  memcmp(ionex_before, ionex_outputs.bytes,
                         sizeof ionex_before) == 0,
              "IONEX satellite count rejects overlapping outputs");
        check(sidereon_ionex_header_get_maps_in_file(
                  ionex_header, (uint32_t *)(void *)ionex_outputs.bytes,
                  (bool *)(void *)ionex_outputs.bytes) ==
                  SIDEREON_STATUS_INVALID_ARGUMENT &&
                  memcmp(ionex_before, ionex_outputs.bytes,
                         sizeof ionex_before) == 0,
              "IONEX map count rejects overlapping outputs");
        check(sidereon_ionex_header_get_mapping_declaration(
                  ionex_header,
                  (SidereonIonexMappingDeclarationKind *)(void *)ionex_outputs.bytes,
                  (SidereonIonexMappingFunctionKind *)(void *)ionex_outputs.bytes) ==
                  SIDEREON_STATUS_INVALID_ARGUMENT &&
                  memcmp(ionex_before, ionex_outputs.bytes,
                         sizeof ionex_before) == 0,
              "IONEX mapping declaration rejects overlapping outputs");
        sidereon_ionex_header_free(ionex_header);
    }
}

static void assert_ut1_frame_refusal(const SidereonTimeScales *scales,
                                    const char *reason,
                                    const char *label) {
    const double position[3] = {7000.0, 0.0, 0.0};
    double output[3] = {1.0, 2.0, 3.0};
    check(sidereon_frame_gcrs_to_itrs(position, scales, false, output) ==
              SIDEREON_STATUS_UT1_OUTSIDE_COVERAGE,
          label);
    check(output[0] == 0.0 && output[1] == 0.0 && output[2] == 0.0,
          "UT1 frame refusal resets output");

    SidereonEngineErrorInfo info;
    memset(&info, 0, sizeof info);
    size_t written = 0;
    size_t required = 0;
    const SidereonStatus info_status = sidereon_last_engine_error_info(&info);
    const SidereonStatus query_status =
        sidereon_last_engine_error_payload(NULL, 0, &written, &required);
    check(info_status == SIDEREON_STATUS_OK &&
              info.family == SIDEREON_ENGINE_ERROR_FAMILY_FRAME_TRANSFORM &&
              query_status == SIDEREON_STATUS_OK && written == 0 &&
              required == info.payload_len,
          "UT1 frame refusal retains typed frame error");
    if (required == 0 || required >= 4096) return;

    uint8_t *payload = (uint8_t *)malloc(required + 1);
    check(payload != NULL, "allocate UT1 frame error payload");
    if (payload == NULL) return;
    const SidereonStatus payload_status = sidereon_last_engine_error_payload(
        payload, required, &written, &required);
    check(payload_status == SIDEREON_STATUS_OK && written == required,
          "copy UT1 frame error payload");
    if (payload_status == SIDEREON_STATUS_OK && written == required) {
        payload[written] = 0;
        check(strstr((const char *)payload,
                     "\"kind\":\"ut1_outside_coverage\"") != NULL &&
                  strstr((const char *)payload, reason) != NULL,
              "UT1 frame error retains expected reason");
    }
    free(payload);
}

static void test_ut1_degradation_public_abi(void) {
    SidereonTimeScales before;
    SidereonTimeScales covered;
    SidereonTimeScales after;
    check(sidereon_timescales_from_utc(1960, 1, 1, 0, 0, 0.0, &before) ==
              SIDEREON_STATUS_OK &&
              before.ut1_degraded == SIDEREON_UT1_DEGRADATION_BEFORE_COVERAGE,
          "UTC conversion reports before-table UT1 degradation");
    check(sidereon_timescales_from_utc(2024, 1, 1, 0, 0, 0.0, &covered) ==
              SIDEREON_STATUS_OK &&
              covered.ut1_degraded == SIDEREON_UT1_DEGRADATION_NONE,
          "UTC conversion reports table-backed UT1 without degradation");
    check(sidereon_timescales_from_utc(2500, 1, 1, 0, 0, 0.0, &after) ==
              SIDEREON_STATUS_OK &&
              after.ut1_degraded == SIDEREON_UT1_DEGRADATION_AFTER_COVERAGE,
          "UTC conversion reports after-table UT1 degradation");

    assert_ut1_frame_refusal(&before,
                             "\"reason\":\"before_coverage\"",
                             "valid before-coverage UT1 flag refuses frame transform");
    assert_ut1_frame_refusal(&after,
                             "\"reason\":\"after_coverage\"",
                             "valid after-coverage UT1 flag refuses frame transform");

    double position[3] = {7000.0, 0.0, 0.0};
    double output[3] = {1.0, 2.0, 3.0};
    SidereonTimeScales invalid = covered;
    invalid.ut1_degraded = 99;
    check(sidereon_frame_gcrs_to_itrs(position, &invalid, false, output) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              output[0] == 0.0 && output[1] == 0.0 && output[2] == 0.0,
          "frame transform rejects unknown UT1 degradation and resets output");
}

static void test_frame_iod_in_place_inputs(void) {
    SidereonTimeScales ts;
    SidereonStatus epoch_status =
        sidereon_timescales_from_utc(2018, 7, 4, 0, 0, 0.0, &ts);
    check(epoch_status == SIDEREON_STATUS_OK,
          "create frame transform epoch for in-place controls");
    if (epoch_status == SIDEREON_STATUS_OK) {
        double expected_position[3] = {0.0, 0.0, 0.0};
        double expected_velocity[3] = {0.0, 0.0, 0.0};
        double position[3] = {-4000.0, 5000.0, 3000.0};
        double velocity[3] = {-3.0, -2.0, 6.0};
        check(sidereon_frame_teme_to_gcrs(position, velocity, &ts, true,
                                          expected_position,
                                          expected_velocity) ==
                  SIDEREON_STATUS_OK,
              "frame transform disjoint reference outputs");
        check(sidereon_frame_teme_to_gcrs(position, velocity, &ts, true,
                                          position, velocity) ==
                  SIDEREON_STATUS_OK &&
                  memcmp(position, expected_position, sizeof position) == 0 &&
                  memcmp(velocity, expected_velocity, sizeof velocity) == 0,
              "frame transform preserves valid in-place inputs");
        double reset_position[3] = {1.0, 2.0, 3.0};
        double reset_velocity[3] = {4.0, 5.0, 6.0};
        check(sidereon_frame_teme_to_gcrs(position, velocity, NULL, true,
                                          reset_position, reset_velocity) ==
                  SIDEREON_STATUS_NULL_POINTER &&
                  reset_position[0] == 0.0 && reset_position[1] == 0.0 &&
                  reset_position[2] == 0.0 && reset_velocity[0] == 0.0 &&
                  reset_velocity[1] == 0.0 && reset_velocity[2] == 0.0,
              "frame transform keeps output reset on invalid time scales");
    }

    const double degrees_to_radians = 3.14159265358979323846 / 180.0;
    double decl[3] = {18.667717 * degrees_to_radians,
                      35.664741 * degrees_to_radians,
                      36.996583 * degrees_to_radians};
    double rtasc[3] = {0.939913 * degrees_to_radians,
                       45.025748 * degrees_to_radians,
                       67.886655 * degrees_to_radians};
    const double jd[3] = {2456159.5, 2456159.5, 2456159.5};
    const double jdf[3] = {0.4864351851851852, 0.49199074074074073,
                            0.4947685185185185};
    const double rseci[9] = {4054.881, 2748.195, 4074.237,
                             3956.224, 2888.232, 4074.364,
                             3905.073, 2956.935, 4074.430};
    double expected_position[3] = {0.0, 0.0, 0.0};
    double expected_velocity[3] = {0.0, 0.0, 0.0};
    check(sidereon_iod_gauss_angles(decl, rtasc, jd, jdf, rseci,
                                    expected_position, expected_velocity) ==
              SIDEREON_STATUS_OK,
          "IOD disjoint reference outputs");
    check(sidereon_iod_gauss_angles(decl, rtasc, jd, jdf, rseci, decl, rtasc) ==
              SIDEREON_STATUS_OK &&
              memcmp(decl, expected_position, sizeof decl) == 0 &&
              memcmp(rtasc, expected_velocity, sizeof rtasc) == 0,
          "IOD preserves valid in-place input arrays");
    double reset_position[3] = {1.0, 2.0, 3.0};
    double reset_velocity[3] = {4.0, 5.0, 6.0};
    check(sidereon_iod_gauss_angles(NULL, rtasc, jd, jdf, rseci,
                                    reset_position, reset_velocity) ==
              SIDEREON_STATUS_NULL_POINTER &&
              reset_position[0] == 0.0 && reset_position[1] == 0.0 &&
              reset_position[2] == 0.0 && reset_velocity[0] == 0.0 &&
              reset_velocity[1] == 0.0 && reset_velocity[2] == 0.0,
          "IOD keeps output reset on invalid input");
    reset_position[0] = reset_position[1] = reset_position[2] = 9.0;
    reset_velocity[0] = reset_velocity[1] = reset_velocity[2] = 9.0;
    check(sidereon_iod_gauss_angles(NULL, NULL, jd, jdf, rseci,
                                    reset_position, reset_velocity) ==
              SIDEREON_STATUS_NULL_POINTER &&
              reset_position[0] == 0.0 && reset_position[1] == 0.0 &&
              reset_position[2] == 0.0 && reset_velocity[0] == 0.0 &&
              reset_velocity[1] == 0.0 && reset_velocity[2] == 0.0,
          "IOD clears outputs and reports first of multiple invalid inputs");
    char first_iod_error[256];
    (void)sidereon_last_error_message(first_iod_error, sizeof first_iod_error);
    check(strstr(first_iod_error, "decl_rad") != NULL &&
              strstr(first_iod_error, "rtasc_rad") == NULL,
          "IOD reports the first invalid input field");
}

static void set_test_bits(uint8_t *bits, size_t offset, size_t width,
                          uint64_t value) {
    for (size_t index = 0; index < width; index++) {
        bits[offset + index] =
            (uint8_t)((value >> (width - index - 1)) & UINT64_C(1));
    }
}

static void test_observable_public_output_overlap(SidereonSp3 *sp3) {
    union {
        double align;
        uint8_t bytes[32];
    } shared;
    memset(shared.bytes, 0xa5, sizeof shared.bytes);
    bool has_clock = true;
    SidereonObservableStateElementStatus element = SIDEREON_OBSERVABLE_STATE_ELEMENT_STATUS_VALID;
    SidereonStatus result = SIDEREON_STATUS_OK;
    check(sidereon_sp3_observable_states_at_j2000_s(
              sp3, NULL, NULL, 1, (double *)(void *)shared.bytes,
              (double *)(void *)shared.bytes, &has_clock, &element, &result) ==
              SIDEREON_STATUS_INVALID_ARGUMENT &&
              shared.bytes[0] == 0xa5 && has_clock &&
              element == SIDEREON_OBSERVABLE_STATE_ELEMENT_STATUS_VALID &&
              result == SIDEREON_STATUS_OK,
          "SP3 observable public output overlap rejects before writes");
}

static bool same_range_prediction(const SidereonRangePrediction *a,
                                  const SidereonRangePrediction *b) {
    return a->geometric_range_m == b->geometric_range_m &&
           a->has_sat_clock_s == b->has_sat_clock_s &&
           a->sat_clock_s == b->sat_clock_s &&
           a->transmit_time_j2000_s == b->transmit_time_j2000_s &&
           a->sat_pos_ecef_m[0] == b->sat_pos_ecef_m[0] &&
           a->sat_pos_ecef_m[1] == b->sat_pos_ecef_m[1] &&
           a->sat_pos_ecef_m[2] == b->sat_pos_ecef_m[2];
}

static void test_range_prediction_in_place_inputs(SidereonSp3 *sp3) {
    const SidereonRangePredictionRequest request = {
        .sat_id = "G01",
        .receiver_ecef_m = {6378137.0, 0.0, 0.0},
        .t_rx_j2000_s = 646274096.0,
    };
    SidereonRangePrediction expected = {0};
    const SidereonStatus expected_status = sidereon_sp3_predict_ranges(
        sp3, &request, 1, NULL, &expected);
    check(expected_status == SIDEREON_STATUS_OK && expected.geometric_range_m > 0.0,
          "SP3 range prediction disjoint request/options control is usable");
    union {
        SidereonRangePrediction result;
        SidereonRangePredictionRequest request;
    } request_result;
    memset(&request_result, 0, sizeof request_result);
    request_result.request = request;
    const SidereonStatus request_alias_status = sidereon_sp3_predict_ranges(
        sp3, &request_result.request, 1, NULL, &request_result.result);
    check(request_alias_status == expected_status &&
              same_range_prediction(&request_result.result, &expected),
          "SP3 range prediction preserves in-place request input");

    const SidereonObservablesOptions options = {
        .carrier_hz = 1575420000.0, .light_time = true, .sagnac = true,
    };
    SidereonRangePrediction expected_with_options = {0};
    const SidereonStatus options_status = sidereon_sp3_predict_ranges(
        sp3, &request, 1, &options, &expected_with_options);
    check(options_status == SIDEREON_STATUS_OK &&
              expected_with_options.geometric_range_m > 0.0,
          "SP3 range prediction disjoint nondefault options control is usable");
    union {
        SidereonRangePrediction result;
        SidereonObservablesOptions options;
    } options_result;
    memset(&options_result, 0, sizeof options_result);
    options_result.options = options;
    const SidereonStatus options_alias_status = sidereon_sp3_predict_ranges(
        sp3, &request, 1, &options_result.options, &options_result.result);
    check(options_alias_status == options_status &&
              same_range_prediction(&options_result.result, &expected_with_options),
          "SP3 range prediction preserves in-place options input");
}

static void test_observable_epoch_in_place_input(SidereonSp3 *sp3) {
    const char *satellites[1] = {"G01"};
    const double epoch = 646274096.0; /* 2020-06-24 12:34:56 UTC from J2000. */
    double expected_position[3] = {0};
    double expected_clock = 0.0;
    bool expected_has_clock = false;
    SidereonObservableStateElementStatus expected_element =
        SIDEREON_OBSERVABLE_STATE_ELEMENT_STATUS_ERROR;
    SidereonStatus expected_result = SIDEREON_STATUS_INVALID_ARGUMENT;
    const SidereonStatus expected_status =
        sidereon_sp3_observable_states_at_j2000_s(
            sp3, satellites, &epoch, 1, expected_position, &expected_clock,
            &expected_has_clock, &expected_element, &expected_result);
    check(expected_status == SIDEREON_STATUS_OK &&
              expected_element == SIDEREON_OBSERVABLE_STATE_ELEMENT_STATUS_VALID,
          "SP3 observable epoch disjoint control is usable");
    union {
        double align;
        uint8_t bytes[32];
    } storage;
    memset(storage.bytes, 0, sizeof storage.bytes);
    memcpy(storage.bytes, &epoch, sizeof epoch);
    double clock = 0.0;
    bool has_clock = false;
    SidereonObservableStateElementStatus element =
        SIDEREON_OBSERVABLE_STATE_ELEMENT_STATUS_ERROR;
    SidereonStatus result = SIDEREON_STATUS_INVALID_ARGUMENT;
    const SidereonStatus status = sidereon_sp3_observable_states_at_j2000_s(
        sp3, satellites, (const double *)(const void *)storage.bytes, 1,
        (double *)(void *)storage.bytes, &clock, &has_clock, &element, &result);
    check(status == expected_status &&
              memcmp(storage.bytes, expected_position, sizeof expected_position) == 0 &&
              clock == expected_clock && has_clock == expected_has_clock &&
              element == expected_element && result == expected_result,
          "SP3 observable states preserve in-place epoch input");
}

static SidereonStatus emission_media_call(
    SidereonSp3 *sp3, const char *const *satellites, const double *epochs,
    const double *receiver, const SidereonEmissionMediaOptions *options,
    double *positions, bool *has_positions, double *clocks, bool *has_clocks,
    double *iono, bool *has_iono, double *tropo, bool *has_tropo,
    SidereonEmissionMediaStatus *statuses, SidereonStatus *result_statuses) {
    return sidereon_sp3_emission_media_batch_at_j2000_s(
        sp3, satellites, epochs, 1, receiver, options, positions, has_positions,
        clocks, has_clocks, iono, has_iono, tropo, has_tropo, statuses,
        result_statuses);
}

static void test_emission_media_in_place_inputs_and_overlap(SidereonSp3 *sp3) {
    const char *satellites[1] = {"G01"};
    const double receiver[3] = {6378137.0, 0.0, 0.0};
    const double epoch = 646274096.0;
    SidereonEmissionMediaOptions options;
    check(sidereon_emission_media_options_init(&options) == SIDEREON_STATUS_OK,
          "emission media options init");
    double expected_positions[3] = {0};
    bool expected_has_positions = false;
    double expected_clock = 0.0;
    bool expected_has_clock = false;
    double expected_iono = 0.0;
    bool expected_has_iono = false;
    double expected_tropo = 0.0;
    bool expected_has_tropo = false;
    SidereonEmissionMediaStatus expected_status = SIDEREON_EMISSION_MEDIA_STATUS_ERROR;
    SidereonStatus expected_result = SIDEREON_STATUS_INVALID_ARGUMENT;
    const SidereonStatus reference_status = emission_media_call(
        sp3, satellites, &epoch, receiver, &options, expected_positions,
        &expected_has_positions, &expected_clock, &expected_has_clock,
        &expected_iono, &expected_has_iono, &expected_tropo, &expected_has_tropo,
        &expected_status, &expected_result);
    check(reference_status == SIDEREON_STATUS_OK && expected_has_positions &&
              expected_status == SIDEREON_EMISSION_MEDIA_STATUS_VALID,
          "emission media disjoint reference is valid");

    union {
        double align;
        uint8_t bytes[32];
    } epoch_storage;
    memcpy(epoch_storage.bytes, &epoch, sizeof epoch);
    bool has_positions = false;
    double clock = 0.0;
    bool has_clock = false;
    double iono = 0.0;
    bool has_iono = false;
    double tropo = 0.0;
    bool has_tropo = false;
    SidereonEmissionMediaStatus status = SIDEREON_EMISSION_MEDIA_STATUS_ERROR;
    SidereonStatus result_status = SIDEREON_STATUS_INVALID_ARGUMENT;
    const SidereonStatus epoch_alias_status = emission_media_call(
        sp3, satellites, (const double *)(const void *)epoch_storage.bytes,
        receiver, &options, (double *)(void *)epoch_storage.bytes,
        &has_positions, &clock, &has_clock,
        &iono, &has_iono, &tropo, &has_tropo, &status, &result_status);
    check(epoch_alias_status == reference_status && has_positions == expected_has_positions &&
              memcmp(epoch_storage.bytes, expected_positions, sizeof expected_positions) == 0 &&
              clock == expected_clock && has_clock == expected_has_clock &&
              iono == expected_iono && has_iono == expected_has_iono &&
              tropo == expected_tropo && has_tropo == expected_has_tropo &&
              status == expected_status && result_status == expected_result,
          "emission media preserves in-place epoch input");

    SidereonEmissionMediaOptions cutoff_options = options;
    cutoff_options.min_elevation_enabled = true;
    cutoff_options.min_elevation_rad = 1.55;
    double cutoff_positions[3] = {0};
    bool cutoff_has_positions = false;
    double cutoff_clock = 0.0;
    bool cutoff_has_clock = false;
    double cutoff_iono = 0.0;
    bool cutoff_has_iono = false;
    double cutoff_tropo = 0.0;
    bool cutoff_has_tropo = false;
    SidereonEmissionMediaStatus cutoff_status = SIDEREON_EMISSION_MEDIA_STATUS_ERROR;
    SidereonStatus cutoff_result = SIDEREON_STATUS_INVALID_ARGUMENT;
    const SidereonStatus cutoff_reference = emission_media_call(
        sp3, satellites, &epoch, receiver, &cutoff_options, cutoff_positions,
        &cutoff_has_positions, &cutoff_clock, &cutoff_has_clock,
        &cutoff_iono, &cutoff_has_iono, &cutoff_tropo, &cutoff_has_tropo,
        &cutoff_status, &cutoff_result);
    check(cutoff_reference == SIDEREON_STATUS_OK &&
              cutoff_status == SIDEREON_EMISSION_MEDIA_STATUS_BELOW_ELEVATION_CUTOFF,
          "emission media nondefault cutoff control is effective");
    union {
        SidereonEmissionMediaOptions options;
        uint8_t bytes[sizeof(SidereonEmissionMediaOptions)];
    } options_storage;
    options_storage.options = cutoff_options;
    bool *aliased_minimum_enabled = &options_storage.options.min_elevation_enabled;
    double option_positions[3] = {0};
    double option_clock = 0.0;
    bool option_has_clock = false;
    double option_iono = 0.0;
    bool option_has_iono = false;
    double option_tropo = 0.0;
    bool option_has_tropo = false;
    SidereonEmissionMediaStatus option_status = SIDEREON_EMISSION_MEDIA_STATUS_ERROR;
    SidereonStatus option_result = SIDEREON_STATUS_INVALID_ARGUMENT;
    const SidereonStatus option_alias_status = emission_media_call(
        sp3, satellites, &epoch, receiver, &options_storage.options,
        option_positions, aliased_minimum_enabled, &option_clock, &option_has_clock,
        &option_iono, &option_has_iono, &option_tropo, &option_has_tropo,
        &option_status, &option_result);
    check(option_alias_status == cutoff_reference &&
              memcmp(option_positions, cutoff_positions, sizeof option_positions) == 0 &&
              *aliased_minimum_enabled == cutoff_has_positions &&
              option_clock == cutoff_clock && option_has_clock == cutoff_has_clock &&
              option_iono == cutoff_iono && option_has_iono == cutoff_has_iono &&
              option_tropo == cutoff_tropo && option_has_tropo == cutoff_has_tropo &&
              option_status == cutoff_status && option_result == cutoff_result,
          "emission media preserves in-place options bool input");

    union {
        double align;
        uint8_t bytes[32];
    } aliased_outputs;
    memset(aliased_outputs.bytes, 0xa5, sizeof aliased_outputs.bytes);
    SidereonEmissionMediaStatus untouched_status = SIDEREON_EMISSION_MEDIA_STATUS_VALID;
    SidereonStatus untouched_result = SIDEREON_STATUS_OK;
    check(emission_media_call(
              sp3, satellites, &epoch, receiver, &options,
              (double *)(void *)aliased_outputs.bytes,
              (bool *)(void *)aliased_outputs.bytes,
              &clock, &has_clock, &iono, &has_iono, &tropo, &has_tropo,
              &untouched_status, &untouched_result) == SIDEREON_STATUS_INVALID_ARGUMENT &&
              aliased_outputs.bytes[0] == 0xa5 && untouched_status == SIDEREON_EMISSION_MEDIA_STATUS_VALID &&
              untouched_result == SIDEREON_STATUS_OK,
          "emission media rejects overlapping public outputs before writes");
}

static void test_sp3_interpolate_input_and_output_aliases(SidereonSp3 *sp3) {
    const double epoch = 646274096.0;
    double expected_position[3] = {0};
    double expected_clock = 0.0;
    size_t expected_written = 0;
    const SidereonStatus reference = sidereon_sp3_interpolate(
        sp3, "G01", &epoch, 1, expected_position, 3, &expected_clock, 1,
        &expected_written);
    check(reference == SIDEREON_STATUS_OK && expected_written == 1,
          "SP3 interpolate disjoint reference is usable");

    union {
        double align;
        uint8_t bytes[32];
    } in_place;
    memcpy(in_place.bytes, &epoch, sizeof epoch);
    double clock = 0.0;
    size_t written = 0;
    const SidereonStatus in_place_status = sidereon_sp3_interpolate(
        sp3, "G01", (const double *)(const void *)in_place.bytes, 1,
        (double *)(void *)in_place.bytes, 3, &clock, 1, &written);
    check(in_place_status == reference && written == expected_written &&
              memcmp(in_place.bytes, expected_position, sizeof expected_position) == 0 &&
              clock == expected_clock,
          "SP3 interpolate preserves in-place epoch input");

    union { double double_align; size_t count_align; uint8_t bytes[32]; } count_epoch_storage;
    memcpy(count_epoch_storage.bytes, &epoch, sizeof epoch);
    double count_epoch_position[3] = {0};
    double count_epoch_clock = 0.0;
    const SidereonStatus count_epoch_status = sidereon_sp3_interpolate(
        sp3, "G01", (const double *)(const void *)count_epoch_storage.bytes, 1,
        count_epoch_position, 3, &count_epoch_clock, 1,
        (size_t *)(void *)count_epoch_storage.bytes);
    check(count_epoch_status == reference &&
              *(size_t *)(void *)count_epoch_storage.bytes == expected_written &&
              memcmp(count_epoch_position, expected_position, sizeof expected_position) == 0 &&
              count_epoch_clock == expected_clock,
          "SP3 interpolate preserves epoch when count aliases input");

    union {
        double align;
        uint8_t bytes[32];
    } overlapping_outputs;
    memset(overlapping_outputs.bytes, 0xa5, sizeof overlapping_outputs.bytes);
    written = 99;
    check(sidereon_sp3_interpolate(
              sp3, "G01", &epoch, 1,
              (double *)(void *)overlapping_outputs.bytes, 3,
              (double *)(void *)overlapping_outputs.bytes, 1, &written) ==
              SIDEREON_STATUS_INVALID_ARGUMENT && written == 0 &&
              overlapping_outputs.bytes[0] == 0xa5,
          "SP3 interpolate refuses output overlap before array writes");

    union { double align; uint8_t bytes[32]; } count_position;
    memset(count_position.bytes, 0xa5, sizeof count_position.bytes);
    size_t *count_at_position = (size_t *)(void *)count_position.bytes;
    *count_at_position = 99;
    check(sidereon_sp3_interpolate(
              sp3, "G01", &epoch, 1, (double *)(void *)count_position.bytes,
              3, &clock, 1, count_at_position) == SIDEREON_STATUS_INVALID_ARGUMENT &&
              *count_at_position == 0 && count_position.bytes[sizeof(size_t)] == 0xa5,
          "SP3 interpolate rejects count/position overlap after count reset only");

    union { double align; uint8_t bytes[32]; } count_clock;
    memset(count_clock.bytes, 0xa5, sizeof count_clock.bytes);
    size_t *count_at_clock = (size_t *)(void *)count_clock.bytes;
    *count_at_clock = 99;
    check(sidereon_sp3_interpolate(
              sp3, "G01", &epoch, 1, expected_position, 3,
              (double *)(void *)count_clock.bytes, 1, count_at_clock) ==
              SIDEREON_STATUS_INVALID_ARGUMENT && *count_at_clock == 0 &&
              count_clock.bytes[sizeof(size_t)] == 0xa5,
          "SP3 interpolate rejects count/clock overlap after count reset only");
}

static SidereonLnavParams lnav_alias_test_params(void) {
    SidereonLnavParams p = {0};
    p.week_number = 290; p.l2_code = 1; p.iodc = 0x2AB;
    p.tgd = -5.587935447692871e-9; p.toc = 504000; p.af0 = -1.234e-4;
    p.af1 = -3.5e-12; p.iode = 0xAB; p.crs = -55.625; p.delta_n = 1.56e-9;
    p.m0 = -0.35; p.cuc = -1.2e-6; p.eccentricity = 0.012; p.cus = 8.3e-6;
    p.sqrt_a = 5153.65; p.toe = 504000; p.cic = 5.0e-8; p.omega0 = -0.78;
    p.cis = -2.1e-7; p.i0 = 0.305; p.crc = 250.625; p.omega = 0.95;
    p.omega_dot = -8.1e-9; p.idot = 1.5e-10;
    return p;
}

static void test_lnav_encode_decode_aliases(void) {
    const SidereonLnavParams p = lnav_alias_test_params();
    const SidereonLnavOptions o = {.tow=12345, .alert=1, .integrity=1, .tlm_message=5461};
    uint8_t sf1[SIDEREON_LNAV_SUBFRAME_LENGTH] = {0};
    uint8_t sf2[SIDEREON_LNAV_SUBFRAME_LENGTH] = {0};
    uint8_t sf3[SIDEREON_LNAV_SUBFRAME_LENGTH] = {0};
    check(sidereon_lnav_encode(&p, &o, sf1, sf2, sf3, sizeof sf1) == SIDEREON_STATUS_OK,
          "LNAV encode disjoint reference is usable");
    union { uint64_t align; uint8_t bytes[SIDEREON_LNAV_SUBFRAME_LENGTH]; } alias;
    memset(alias.bytes, 0xa5, sizeof alias.bytes);
    check(sidereon_lnav_encode(&p, &o, alias.bytes, alias.bytes, sf3, sizeof sf1) ==
              SIDEREON_STATUS_INVALID_ARGUMENT && alias.bytes[0] == 0xa5,
          "LNAV encode refuses overlapping output ranges before writes");
    SidereonLnavDecoded expected = {0};
    check(sidereon_lnav_decode(sf1, sizeof sf1, sf2, sizeof sf2, sf3, sizeof sf3,
                               &expected) == SIDEREON_STATUS_OK,
          "LNAV decode disjoint reference is usable");
    union { SidereonLnavDecoded align; uint8_t bytes[SIDEREON_LNAV_SUBFRAME_LENGTH]; } in_place;
    memcpy(in_place.bytes, sf1, sizeof sf1);
    SidereonLnavDecoded *decoded = (SidereonLnavDecoded *)(void *)in_place.bytes;
    check(sidereon_lnav_decode(in_place.bytes, sizeof sf1, sf2, sizeof sf2, sf3, sizeof sf3,
                               decoded) == SIDEREON_STATUS_OK &&
              decoded->week_number == expected.week_number && decoded->toe == expected.toe &&
              decoded->sqrt_a == expected.sqrt_a && decoded->omega0 == expected.omega0,
          "LNAV decode preserves aliased source subframe");
}

static void test_lnav_input_aliases(void) {
    const SidereonLnavParams p = lnav_alias_test_params();
    const SidereonLnavOptions o = {.tow=12345, .alert=1, .integrity=1, .tlm_message=5461};
    uint8_t expected1[SIDEREON_LNAV_SUBFRAME_LENGTH], expected2[SIDEREON_LNAV_SUBFRAME_LENGTH];
    uint8_t expected3[SIDEREON_LNAV_SUBFRAME_LENGTH];
    check(sidereon_lnav_encode(&p, &o, expected1, expected2, expected3, sizeof expected1) ==
              SIDEREON_STATUS_OK,
          "LNAV disjoint encode for input alias controls");
    union { SidereonLnavParams p; uint64_t align; uint8_t bytes[SIDEREON_LNAV_SUBFRAME_LENGTH]; } pbuf;
    pbuf.p = p;
    uint8_t sf2[SIDEREON_LNAV_SUBFRAME_LENGTH], sf3[SIDEREON_LNAV_SUBFRAME_LENGTH];
    check(sidereon_lnav_encode(&pbuf.p, &o, pbuf.bytes, sf2, sf3, sizeof sf2) ==
              SIDEREON_STATUS_OK && memcmp(pbuf.bytes, expected1, sizeof expected1) == 0 &&
              memcmp(sf2, expected2, sizeof sf2) == 0 && memcmp(sf3, expected3, sizeof sf3) == 0,
          "LNAV encode snapshots params before aliased output");
    union { SidereonLnavOptions o; uint64_t align; uint8_t bytes[SIDEREON_LNAV_SUBFRAME_LENGTH]; } obuf;
    obuf.o = o;
    check(sidereon_lnav_encode(&p, &obuf.o, obuf.bytes, sf2, sf3, sizeof sf2) ==
              SIDEREON_STATUS_OK && memcmp(obuf.bytes, expected1, sizeof expected1) == 0 &&
              memcmp(sf2, expected2, sizeof sf2) == 0 && memcmp(sf3, expected3, sizeof sf3) == 0,
          "LNAV encode snapshots options before aliased output");
}

static void test_ils_output_aliases(void) {
    const double floats[2] = {0.1, 0.9}, covariance[4] = {1.0, 0.0, 0.0, 1.0};
    int64_t fixed[2] = {0};
    SidereonIlsResult result = {0};
    check(sidereon_bounded_ils_search(floats, 2, covariance, 4, 1, 200000, 3.0,
                                     fixed, &result) == SIDEREON_STATUS_OK,
          "ILS disjoint public reference is usable");
    union { SidereonIlsResult align; uint8_t bytes[sizeof(SidereonIlsResult)]; } alias;
    memset(alias.bytes, 0xa5, sizeof alias.bytes);
    check(sidereon_bounded_ils_search(floats, 2, covariance, 4, 1, 200000, 3.0,
                                     (int64_t *)(void *)alias.bytes,
                                     (SidereonIlsResult *)(void *)alias.bytes) ==
              SIDEREON_STATUS_INVALID_ARGUMENT && alias.bytes[0] == 0xa5,
          "ILS rejects overlapping result/vector output before writes");
}

static void test_raw_buffer_in_place_inputs(void) {
    union {
        uint64_t align;
        uint8_t bytes[32];
    } how;
    memset(how.bytes, 0, sizeof how.bytes);
    set_test_bits(how.bytes, 0, 17, 12345);
    set_test_bits(how.bytes, 19, 3, 5);
    uint64_t expected_tow = 0;
    uint64_t expected_subframe = 0;
    check(sidereon_lnav_tow(how.bytes, 30, &expected_tow) == SIDEREON_STATUS_OK &&
              sidereon_lnav_subframe_id(how.bytes, 30,
                                        &expected_subframe) ==
                  SIDEREON_STATUS_OK,
          "LNAV disjoint reference values");
    check(sidereon_lnav_tow(how.bytes, 30, (uint64_t *)(void *)how.bytes) ==
              SIDEREON_STATUS_OK &&
              *(uint64_t *)(void *)how.bytes == expected_tow,
          "LNAV TOW preserves in-place input bytes");

    memset(how.bytes, 0, sizeof how.bytes);
    set_test_bits(how.bytes, 0, 17, 12345);
    set_test_bits(how.bytes, 19, 3, 5);
    check(sidereon_lnav_subframe_id(
              how.bytes, 30, (uint64_t *)(void *)(how.bytes + 16)) ==
              SIDEREON_STATUS_OK &&
              *(uint64_t *)(void *)(how.bytes + 16) == expected_subframe,
          "LNAV subframe ID preserves in-place input bits under output range");

    uint8_t source[24] = {0};
    source[0] = 1;
    source[5] = 1;
    source[23] = 1;
    uint8_t expected_parity[6] = {0};
    check(sidereon_lnav_parity(source, sizeof source, 1, 0, expected_parity,
                               sizeof expected_parity) == SIDEREON_STATUS_OK,
          "LNAV parity disjoint reference output");
    union {
        uint64_t align;
        uint8_t bytes[24];
    } parity_storage;
    memset(parity_storage.bytes, 0, sizeof parity_storage.bytes);
    memcpy(parity_storage.bytes, source, sizeof source);
    check(sidereon_lnav_parity(parity_storage.bytes,
                               sizeof parity_storage.bytes, 1, 0,
                               parity_storage.bytes, sizeof expected_parity) ==
              SIDEREON_STATUS_OK &&
              memcmp(parity_storage.bytes, expected_parity,
                     sizeof expected_parity) == 0,
          "LNAV parity preserves in-place input bytes");

    uint8_t word[30] = {0};
    word[0] = 1;
    uint8_t expected_word_parity[6] = {0};
    check(sidereon_lnav_parity(word, 24, 1, 0, expected_word_parity,
                               sizeof expected_word_parity) == SIDEREON_STATUS_OK,
          "LNAV parity-valid word reference parity");
    memcpy(word + 24, expected_word_parity, sizeof expected_word_parity);
    bool expected_valid = false;
    check(sidereon_lnav_parity_valid(word, sizeof word, 1, 0,
                                     &expected_valid) == SIDEREON_STATUS_OK,
          "LNAV parity-valid disjoint reference output");
    check(sidereon_lnav_parity_valid(word, sizeof word, 1, 0,
                                     (bool *)(void *)word) ==
              SIDEREON_STATUS_OK &&
              *(bool *)(void *)word == expected_valid,
          "LNAV parity-valid preserves in-place input bytes");

    const double position[3] = {6378137.0, 0.0, 0.0};
    const SidereonStationTideEpoch epoch = {
        .year = 2024, .month = 1, .day = 1, .hour = 0, .minute = 0,
        .second = 0.0, .has_polar_motion = false,
        .xp_arcsec = 0.0, .yp_arcsec = 0.0,
    };
    const SidereonStationTideOptions options = {0};
    SidereonStationTideBatchRow expected_row = {0};
    check(sidereon_station_tide_displacement_batch(
              position, &epoch, 1, &options, &expected_row) ==
              SIDEREON_STATUS_OK,
          "station tide batch disjoint reference output");
    union {
        SidereonStationTideBatchRow row_align;
        SidereonStationTideEpoch epoch_align;
        uint8_t bytes[sizeof(SidereonStationTideBatchRow) >
                              sizeof(SidereonStationTideEpoch)
                          ? sizeof(SidereonStationTideBatchRow)
                          : sizeof(SidereonStationTideEpoch)];
    } epoch_row_storage;
    memset(epoch_row_storage.bytes, 0, sizeof epoch_row_storage.bytes);
    memcpy(epoch_row_storage.bytes, &epoch, sizeof epoch);
    check(sidereon_station_tide_displacement_batch(
              position,
              (const SidereonStationTideEpoch *)(const void *)epoch_row_storage.bytes,
              1, &options,
              (SidereonStationTideBatchRow *)(void *)epoch_row_storage.bytes) ==
              SIDEREON_STATUS_OK &&
              ((SidereonStationTideBatchRow *)(void *)epoch_row_storage.bytes)->status ==
                  expected_row.status &&
              ((SidereonStationTideBatchRow *)(void *)epoch_row_storage.bytes)
                      ->displacement.ecef_m[0] == expected_row.displacement.ecef_m[0] &&
              ((SidereonStationTideBatchRow *)(void *)epoch_row_storage.bytes)
                      ->displacement.ecef_m[1] == expected_row.displacement.ecef_m[1] &&
              ((SidereonStationTideBatchRow *)(void *)epoch_row_storage.bytes)
                      ->displacement.ecef_m[2] == expected_row.displacement.ecef_m[2] &&
              ((SidereonStationTideBatchRow *)(void *)epoch_row_storage.bytes)
                      ->displacement.has_solid_earth_tide ==
                  expected_row.displacement.has_solid_earth_tide &&
              memcmp(((SidereonStationTideBatchRow *)(void *)epoch_row_storage.bytes)
                         ->displacement.solid_earth_tide_ecef_m,
                     expected_row.displacement.solid_earth_tide_ecef_m,
                     sizeof expected_row.displacement.solid_earth_tide_ecef_m) == 0 &&
              ((SidereonStationTideBatchRow *)(void *)epoch_row_storage.bytes)
                      ->displacement.has_pole_tide ==
                  expected_row.displacement.has_pole_tide &&
              memcmp(((SidereonStationTideBatchRow *)(void *)epoch_row_storage.bytes)
                         ->displacement.pole_tide_ecef_m,
                     expected_row.displacement.pole_tide_ecef_m,
                     sizeof expected_row.displacement.pole_tide_ecef_m) == 0 &&
              ((SidereonStationTideBatchRow *)(void *)epoch_row_storage.bytes)
                      ->displacement.has_ocean_loading ==
                  expected_row.displacement.has_ocean_loading &&
              memcmp(((SidereonStationTideBatchRow *)(void *)epoch_row_storage.bytes)
                         ->displacement.ocean_loading_ecef_m,
                     expected_row.displacement.ocean_loading_ecef_m,
                     sizeof expected_row.displacement.ocean_loading_ecef_m) == 0 &&
              ((SidereonStationTideBatchRow *)(void *)epoch_row_storage.bytes)
                      ->displacement.degrade_reason ==
                  expected_row.displacement.degrade_reason &&
              ((SidereonStationTideBatchRow *)(void *)epoch_row_storage.bytes)->error.kind ==
                  expected_row.error.kind &&
              ((SidereonStationTideBatchRow *)(void *)epoch_row_storage.bytes)->error.nested_kind ==
                  expected_row.error.nested_kind &&
              ((SidereonStationTideBatchRow *)(void *)epoch_row_storage.bytes)->error.sun_moon_cause ==
                  expected_row.error.sun_moon_cause &&
              ((SidereonStationTideBatchRow *)(void *)epoch_row_storage.bytes)->error.input_kind ==
                  expected_row.error.input_kind &&
              ((SidereonStationTideBatchRow *)(void *)epoch_row_storage.bytes)->error.degrade_reason ==
                  expected_row.error.degrade_reason &&
              ((SidereonStationTideBatchRow *)(void *)epoch_row_storage.bytes)->error.has_field ==
                  expected_row.error.has_field &&
              ((SidereonStationTideBatchRow *)(void *)epoch_row_storage.bytes)->error.has_reason ==
                  expected_row.error.has_reason,
          "station tide batch preserves in-place epoch input");
}

int main(int argc, char **argv) {
    if (argc != 4) {
        fprintf(stderr, "usage: %s <sp3> <antex> <rinex_obs>\n", argv[0]);
        return 2;
    }

    SidereonSp3 *sp3 = load_sp3(argv[1]);
    if (sp3) {
        test_sp3_clock_reference(sp3);
        test_reduced_orbit_source(sp3);
        test_dgnss_position(sp3);
        test_observable_public_output_overlap(sp3);
        test_range_prediction_in_place_inputs(sp3);
        test_observable_epoch_in_place_input(sp3);
        test_emission_media_in_place_inputs_and_overlap(sp3);
        test_sp3_interpolate_input_and_output_aliases(sp3);
        sidereon_sp3_free(sp3);
    }
    test_antex_encode(argv[2]);
    test_rinex_obs_helpers(argv[3]);
    test_output_alias_rejection();
    test_rinex_parse_result_failure_resets_outputs();
    test_inertial_dual_output_alias_rejection();
    test_remaining_multi_output_preflights();
    test_additional_multi_output_aliases();
    test_ut1_degradation_public_abi();
    test_frame_iod_in_place_inputs();
    test_raw_buffer_in_place_inputs();
    test_rtk_residual_refusal();
    test_lnav_encode_decode_aliases();
    test_lnav_input_aliases();
    test_ils_output_aliases();
    test_null_copy_count_preserves_sibling_reset();

    if (failures != 0) {
        fprintf(stderr, "core caps smoke failures: %d\n", failures);
        return 1;
    }
    return 0;
}
