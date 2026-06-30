/*
 * Smoke coverage for core-backed C capabilities added in this round:
 * DGNSS position solve, ANTEX encode, RINEX OBS header/epoch/value helpers,
 * SP3 clock-reference offset/align, and source-backed reduced-orbit fit/drift.
 *
 * argv: <sp3> <antex> <rinex_obs>
 */
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sidereon.h"

static int failures = 0;

static void check(int ok, const char *what) {
    if (!ok) {
        char msg[512];
        size_t n = sidereon_last_error_message((char *)msg, sizeof(msg));
        if (n == 0) {
            msg[0] = '\0';
        }
        fprintf(stderr, "FAIL: %s (last_error: %s)\n", what, msg);
        failures++;
    }
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
    SidereonAntex *antex = NULL;
    check(sidereon_antex_parse(bytes, len, &antex) == SIDEREON_STATUS_OK && antex != NULL,
          "antex parse");
    free(bytes);
    if (!antex) {
        return;
    }

    size_t before_count = 0;
    check(sidereon_antex_antenna_count(antex, &before_count) == SIDEREON_STATUS_OK &&
              before_count > 0,
          "antex count before encode");

    size_t written = 0, required = 0;
    check(sidereon_antex_encode(antex, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
              required > 0,
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

    SidereonAntex *roundtrip = NULL;
    check(sidereon_antex_parse(encoded, written, &roundtrip) == SIDEREON_STATUS_OK &&
              roundtrip != NULL,
          "antex encoded parse");
    if (roundtrip) {
        size_t after_count = 0;
        check(sidereon_antex_antenna_count(roundtrip, &after_count) == SIDEREON_STATUS_OK &&
                  after_count == before_count,
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

    SidereonRinexObsHeader header;
    check(sidereon_rinex_obs_header(obs, &header) == SIDEREON_STATUS_OK &&
              header.version >= 3.0 && header.obs_code_count > 0,
          "rinex obs header");

    size_t written = 0, required = 0;
    check(sidereon_rinex_obs_codes(obs, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
              required == header.obs_code_count && required > 0,
          "rinex obs codes query");
    SidereonRinexObsCode *codes = (SidereonRinexObsCode *)calloc(required, sizeof(*codes));
    if (codes) {
        check(sidereon_rinex_obs_codes(obs, codes, required, &written, &required) ==
                  SIDEREON_STATUS_OK &&
                  written == required && codes[0].code[0] != 0,
              "rinex obs codes copy");
        free(codes);
    } else {
        check(0, "rinex obs codes allocation");
    }

    size_t epoch_count = 0;
    check(sidereon_rinex_obs_epoch_count(obs, &epoch_count) == SIDEREON_STATUS_OK &&
              epoch_count > 0,
          "rinex obs epoch count");
    check(sidereon_rinex_obs_epochs(obs, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
              required == epoch_count,
          "rinex obs epochs query");
    SidereonRinexObsEpoch *epochs =
        (SidereonRinexObsEpoch *)calloc(required, sizeof(*epochs));
    if (epochs) {
        check(sidereon_rinex_obs_epochs(obs, epochs, required, &written, &required) ==
                  SIDEREON_STATUS_OK &&
                  written == required && epochs[0].satellite_count > 0,
              "rinex obs epochs copy");
        free(epochs);
    } else {
        check(0, "rinex obs epochs allocation");
    }

    check(sidereon_rinex_obs_values(obs, 0, NULL, 0, &written, &required) ==
              SIDEREON_STATUS_OK &&
              required > 0,
          "rinex obs values query");
    SidereonRinexObsValue *values = (SidereonRinexObsValue *)calloc(required, sizeof(*values));
    if (values) {
        check(sidereon_rinex_obs_values(obs, 0, values, required, &written, &required) ==
                  SIDEREON_STATUS_OK &&
                  written == required && values[0].sat_id.bytes[0] != 0,
              "rinex obs values copy");
        free(values);
    } else {
        check(0, "rinex obs values allocation");
    }

    check(sidereon_rinex_obs_pseudoranges(obs, 0, NULL, 0, &written, &required) ==
              SIDEREON_STATUS_OK &&
              required > 0,
          "rinex obs pseudoranges query");
    SidereonRinexObsPseudorange *prs =
        (SidereonRinexObsPseudorange *)calloc(required, sizeof(*prs));
    if (prs) {
        check(sidereon_rinex_obs_pseudoranges(obs, 0, prs, required, &written, &required) ==
                  SIDEREON_STATUS_OK &&
                  written == required && isfinite(prs[0].pseudorange_m),
              "rinex obs pseudoranges copy");
        free(prs);
    } else {
        check(0, "rinex obs pseudoranges allocation");
    }

    check(sidereon_rinex_obs_carrier_phase(obs, 0, NULL, 0, &written, &required) ==
              SIDEREON_STATUS_OK &&
              required > 0,
          "rinex obs carrier phase query");
    SidereonRinexObsCarrierPhase *phase =
        (SidereonRinexObsCarrierPhase *)calloc(required, sizeof(*phase));
    if (phase) {
        check(sidereon_rinex_obs_carrier_phase(obs, 0, phase, required, &written, &required) ==
                  SIDEREON_STATUS_OK &&
                  written == required && phase[0].code[0] != 0,
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
              required > 0,
          "sp3 clock reference offsets query");
    SidereonSp3ClockReferenceOffset *offsets =
        (SidereonSp3ClockReferenceOffset *)calloc(required, sizeof(*offsets));
    if (offsets) {
        check(sidereon_sp3_clock_reference_offsets(sp3, sp3, 3, offsets, required, &written,
                                                   &required) == SIDEREON_STATUS_OK &&
                  written == required && fabs(offsets[0].offset_s) < 1.0e-18 &&
                  offsets[0].satellites >= 3,
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
                  required > 0,
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
              stats.fit.n_samples >= 4 && stats.requested_samples >= stats.fit.n_samples &&
              isfinite(elements.a_m) && elements.a_m > 0.0,
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
                  isfinite(summary.max_m) && isfinite(summary.rms_m),
              "reduced orbit source drift summary");
        check(sidereon_reduced_orbit_drift_report_requested_samples(report, &requested) ==
                  SIDEREON_STATUS_OK &&
                  requested >= 4,
              "reduced orbit source drift requested samples");
        sidereon_reduced_orbit_drift_report_free(report);
    }
}

static void test_dgnss_position(SidereonSp3 *sp3) {
    static const double C_M_S = 299792458.0;
    double t_rx = first_sp3_epoch(sp3) + 3600.0;
    double base[3] = {1130773.0, -4831253.0, 3994200.0};
    double rover[3] = {1130833.0, -4831203.0, 3994230.0};

    size_t sat_written = 0, sat_required = 0;
    check(sidereon_sp3_satellites(sp3, NULL, 0, &sat_written, &sat_required) ==
                  SIDEREON_STATUS_OK &&
              sat_required > 0,
          "dgnss satellite query");
    if (sat_required == 0) {
        return;
    }
    SidereonSatelliteToken *sats =
        (SidereonSatelliteToken *)calloc(sat_required, sizeof(*sats));
    SidereonCodeObservation *base_obs =
        (SidereonCodeObservation *)calloc(sat_required, sizeof(*base_obs));
    SidereonCodeObservation *rover_obs =
        (SidereonCodeObservation *)calloc(sat_required, sizeof(*rover_obs));
    if (!sats || !base_obs || !rover_obs) {
        check(0, "dgnss allocation");
        free(sats);
        free(base_obs);
        free(rover_obs);
        return;
    }
    check(sidereon_sp3_satellites(sp3, sats, sat_required, &sat_written, &sat_required) ==
                  SIDEREON_STATUS_OK &&
              sat_written == sat_required,
          "dgnss satellite copy");

    size_t count = 0;

    SidereonObservablesOptions options;
    check(sidereon_observables_options_init(&options) == SIDEREON_STATUS_OK,
          "observables options init");
    for (size_t i = 0; i < sat_written; i++) {
        const char *sat_id = (const char *)sats[i].bytes;
        SidereonPredictedObservables b;
        SidereonPredictedObservables r;
        if (sidereon_sp3_observables(sp3, sat_id, base, t_rx, &options, &b) !=
                SIDEREON_STATUS_OK ||
            sidereon_sp3_observables(sp3, sat_id, rover, t_rx, &options, &r) !=
                SIDEREON_STATUS_OK ||
            !b.has_sat_clock_s || !r.has_sat_clock_s) {
            continue;
        }
        base_obs[count].sat_id = sat_id;
        base_obs[count].pseudorange_m = b.geometric_range_m - C_M_S * b.sat_clock_s;
        rover_obs[count].sat_id = sat_id;
        rover_obs[count].pseudorange_m = r.geometric_range_m - C_M_S * r.sat_clock_s;
        count++;
        if (count == 18) {
            break;
        }
    }
    check(count >= 4, "dgnss synthetic observation count");
    if (count < 4) {
        free(sats);
        free(base_obs);
        free(rover_obs);
        return;
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
        free(sats);
        free(base_obs);
        free(rover_obs);
        return;
    }

    double baseline_vec[3] = {0.0, 0.0, 0.0};
    double baseline_m = 0.0;
    check(sidereon_dgnss_solution_baseline(solution, baseline_vec, 3, &baseline_m) ==
                  SIDEREON_STATUS_OK &&
              isfinite(baseline_m) && baseline_m > 0.0,
          "dgnss solution baseline");

    SidereonSppSolution *spp = NULL;
    check(sidereon_dgnss_solution_solution(solution, &spp) == SIDEREON_STATUS_OK && spp != NULL,
          "dgnss embedded spp solution");
    if (spp) {
        double position[3] = {0.0, 0.0, 0.0};
        check(sidereon_spp_solution_position(spp, position, 3) == SIDEREON_STATUS_OK &&
                  isfinite(position[0]) && isfinite(position[1]) && isfinite(position[2]),
              "dgnss embedded spp position");
        sidereon_spp_solution_free(spp);
    }

    size_t written = 0, required = 0;
    check(sidereon_dgnss_solution_dropped_sats(solution, NULL, 0, &written, &required) ==
                  SIDEREON_STATUS_OK &&
              required == 0,
          "dgnss dropped sats");
    sidereon_dgnss_solution_free(solution);
    free(sats);
    free(base_obs);
    free(rover_obs);
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
        sidereon_sp3_free(sp3);
    }
    test_antex_encode(argv[2]);
    test_rinex_obs_helpers(argv[3]);

    if (failures != 0) {
        fprintf(stderr, "core caps smoke failures: %d\n", failures);
        return 1;
    }
    return 0;
}
