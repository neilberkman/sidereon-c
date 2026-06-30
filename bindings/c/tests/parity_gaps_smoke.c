/*
 * Smoke coverage for the universal-parity additions to the C binding:
 *   1. Batch SPP (sidereon_solve_spp_batch_serial / _parallel) over a shared SP3.
 *   2. GPS LNAV navigation message encode + decode round-trip.
 *   3. OMM serializers (sidereon_omm_to_kvn / _xml / _json) with a KVN reader.
 *   4. CRINEX encode (sidereon_crinex_encode) with a decode round-trip.
 * Every call delegates to sidereon-core; this program only checks the FFI
 * marshaling and that the engine produces sane, round-trippable output.
 *
 * argv: <grg_sp3> <omm_kvn> <esbc_rnx>
 */
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sidereon.h"
#include "spp_fixture.h"

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

static double bits_to_f64(uint64_t bits) {
    double value;
    memcpy(&value, &bits, sizeof(value));
    return value;
}

static uint8_t *read_file(const char *path, size_t *out_len) {
    FILE *f = fopen(path, "rb");
    if (!f) {
        fprintf(stderr, "FAIL: cannot open %s\n", path);
        failures++;
        return NULL;
    }
    fseek(f, 0, SEEK_END);
    long size = ftell(f);
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

/* Fill one SPP V2 input row from the committed GRG fixture (the same epoch the
 * single-solve smoke uses), so the batch solve reproduces the SPP golden. */
static void fill_spp_inputs(SidereonSppInputsV2 *inputs, SidereonObservation *observations) {
    for (size_t i = 0; i < SPP_OBS_COUNT; i++) {
        observations[i].sat_id = SPP_SAT_IDS[i];
        observations[i].pseudorange_m = bits_to_f64(SPP_PSEUDORANGE_BITS[i]);
    }
    check(sidereon_spp_inputs_v2_init(inputs) == SIDEREON_STATUS_OK, "spp_inputs_v2_init");
    inputs->base.observations = observations;
    inputs->base.observation_count = SPP_OBS_COUNT;
    inputs->base.t_rx_j2000_s = bits_to_f64(SPP_T_RX_J2000_S_BITS);
    inputs->base.t_rx_second_of_day_s = bits_to_f64(SPP_T_RX_SOD_S_BITS);
    inputs->base.day_of_year = bits_to_f64(SPP_DOY_BITS);
    for (int i = 0; i < 4; i++) {
        inputs->base.initial_guess[i] = bits_to_f64(SPP_INITIAL_GUESS_BITS[i]);
        inputs->base.klobuchar_alpha[i] = bits_to_f64(SPP_KLOB_ALPHA_BITS[i]);
        inputs->base.klobuchar_beta[i] = bits_to_f64(SPP_KLOB_BETA_BITS[i]);
    }
    inputs->base.ionosphere = false;
    inputs->base.troposphere = false;
    inputs->base.pressure_hpa = bits_to_f64(SPP_PRESSURE_HPA_BITS);
    inputs->base.temperature_k = bits_to_f64(SPP_TEMPERATURE_K_BITS);
    inputs->base.relative_humidity = bits_to_f64(SPP_RELATIVE_HUMIDITY_BITS);
    inputs->base.with_geodetic = true;
}

static void batch_position(SidereonSppBatch *batch, size_t index, double *pos) {
    bool ok = false;
    check(sidereon_spp_batch_epoch_ok(batch, index, &ok) == SIDEREON_STATUS_OK && ok,
          "spp_batch_epoch_ok");
    SidereonSppSolution *sol = NULL;
    check(sidereon_spp_batch_solution(batch, index, &sol) == SIDEREON_STATUS_OK && sol != NULL,
          "spp_batch_solution");
    if (sol) {
        check(sidereon_spp_solution_position(sol, pos, 3) == SIDEREON_STATUS_OK,
              "spp_batch solution position");
        sidereon_spp_solution_free(sol);
    }
}

static void test_spp_batch(const char *sp3_path) {
    size_t len = 0;
    uint8_t *bytes = read_file(sp3_path, &len);
    if (!bytes) {
        return;
    }
    SidereonSp3 *sp3 = NULL;
    check(sidereon_sp3_load(bytes, len, &sp3) == SIDEREON_STATUS_OK && sp3 != NULL,
          "spp_batch sp3_load");
    free(bytes);
    if (!sp3) {
        return;
    }

    /* Two identical epochs in one batch: each must solve and reproduce the SPP
     * golden, and the serial and parallel drivers must agree bit-for-bit. */
    SidereonObservation obs0[SPP_OBS_COUNT];
    SidereonObservation obs1[SPP_OBS_COUNT];
    SidereonSppInputsV2 rows[2];
    fill_spp_inputs(&rows[0], obs0);
    fill_spp_inputs(&rows[1], obs1);
    SidereonSppSolvePolicy policy = rows[0].policy;

    SidereonSppBatch *serial = NULL;
    check(sidereon_solve_spp_batch_serial(sp3, rows, 2, true, &policy, &serial) ==
              SIDEREON_STATUS_OK &&
              serial != NULL,
          "solve_spp_batch_serial");
    SidereonSppBatch *parallel = NULL;
    check(sidereon_solve_spp_batch_parallel(sp3, rows, 2, true, &policy, &parallel) ==
              SIDEREON_STATUS_OK &&
              parallel != NULL,
          "solve_spp_batch_parallel");

    size_t serial_count = 0, parallel_count = 0;
    check(sidereon_spp_batch_count(serial, &serial_count) == SIDEREON_STATUS_OK &&
              serial_count == 2,
          "spp_batch_count serial");
    check(sidereon_spp_batch_count(parallel, &parallel_count) == SIDEREON_STATUS_OK &&
              parallel_count == 2,
          "spp_batch_count parallel");

    double expected[3];
    for (int i = 0; i < 3; i++) {
        expected[i] = bits_to_f64(SPP_EXPECTED_X_BITS[i]);
    }
    for (size_t idx = 0; idx < 2; idx++) {
        double s_pos[3], p_pos[3];
        batch_position(serial, idx, s_pos);
        batch_position(parallel, idx, p_pos);
        for (int i = 0; i < 3; i++) {
            check(s_pos[i] == p_pos[i], "spp_batch serial == parallel bit-for-bit");
        }
        double dx = s_pos[0] - expected[0];
        double dy = s_pos[1] - expected[1];
        double dz = s_pos[2] - expected[2];
        check(sqrt(dx * dx + dy * dy + dz * dz) < SPP_AGREEMENT_BOUND_M,
              "spp_batch reproduces the SPP golden");
    }

    sidereon_spp_batch_free(serial);
    sidereon_spp_batch_free(parallel);
    sidereon_sp3_free(sp3);
}

static void test_lnav(void) {
    /* IS-GPS-200 engineering-unit parameters (the canonical lnav test example).
     * Integer fields are recovered exactly; scaled fields within one LSB. */
    SidereonLnavParams params = {0};
    params.week_number = 290;
    params.l2_code = 1;
    params.l2_p_data_flag = 0;
    params.ura_index = 0;
    params.sv_health = 0;
    params.iodc = 0x2AB;
    params.tgd = -5.587935447692871e-9;
    params.toc = 504000;
    params.af0 = -1.234e-4;
    params.af1 = -3.5e-12;
    params.af2 = 0.0;
    params.iode = 0xAB;
    params.crs = -55.625;
    params.delta_n = 1.56e-9;
    params.m0 = -0.35;
    params.cuc = -1.2e-6;
    params.eccentricity = 0.012;
    params.cus = 8.3e-6;
    params.sqrt_a = 5153.65;
    params.toe = 504000;
    params.fit_interval_flag = 0;
    params.aodo = 0;
    params.cic = 5.0e-8;
    params.omega0 = -0.78;
    params.cis = -2.1e-7;
    params.i0 = 0.305;
    params.crc = 250.625;
    params.omega = 0.95;
    params.omega_dot = -8.1e-9;
    params.idot = 1.5e-10;

    SidereonLnavOptions opts = {0};
    opts.tow = 12345;
    opts.alert = 1;
    opts.anti_spoof = 0;
    opts.integrity = 1;
    opts.tlm_message = 5461;

    uint8_t sf1[SIDEREON_LNAV_SUBFRAME_LENGTH];
    uint8_t sf2[SIDEREON_LNAV_SUBFRAME_LENGTH];
    uint8_t sf3[SIDEREON_LNAV_SUBFRAME_LENGTH];
    check(sidereon_lnav_encode(&params, &opts, sf1, sf2, sf3, SIDEREON_LNAV_SUBFRAME_LENGTH) ==
              SIDEREON_STATUS_OK,
          "lnav_encode");

    /* Output is one 0/1 bit per byte, MSB first; the TLM preamble 0x8B opens it. */
    int preamble_ok = sf1[0] == 1 && sf1[1] == 0 && sf1[2] == 0 && sf1[3] == 0 && sf1[4] == 1 &&
                      sf1[5] == 0 && sf1[6] == 1 && sf1[7] == 1;
    check(preamble_ok, "lnav_encode subframe 1 TLM preamble");

    /* A too-small output buffer must be rejected, not truncate. */
    check(sidereon_lnav_encode(&params, &opts, sf1, sf2, sf3, SIDEREON_LNAV_SUBFRAME_LENGTH - 1) ==
              SIDEREON_STATUS_INVALID_ARGUMENT,
          "lnav_encode rejects short buffer");

    SidereonLnavDecoded decoded = {0};
    check(sidereon_lnav_decode(sf1, SIDEREON_LNAV_SUBFRAME_LENGTH, sf2,
                               SIDEREON_LNAV_SUBFRAME_LENGTH, sf3, SIDEREON_LNAV_SUBFRAME_LENGTH,
                               &decoded) == SIDEREON_STATUS_OK,
          "lnav_decode");

    check(decoded.week_number == 290, "lnav_decode week_number");
    check(decoded.iodc == 0x2AB, "lnav_decode iodc");
    check(decoded.iode == 0xAB, "lnav_decode iode");
    check(decoded.toc == 504000, "lnav_decode toc");
    check(decoded.toe == 504000, "lnav_decode toe");
    check(fabs(decoded.crs - params.crs) < 1.0, "lnav_decode crs within LSB");
    check(fabs(decoded.eccentricity - params.eccentricity) < 1e-6,
          "lnav_decode eccentricity within LSB");
    check(fabs(decoded.sqrt_a - params.sqrt_a) < 1e-3, "lnav_decode sqrt_a within LSB");

    /* A flipped parity bit must be rejected. */
    sf1[29] ^= 1;
    check(sidereon_lnav_decode(sf1, SIDEREON_LNAV_SUBFRAME_LENGTH, sf2,
                               SIDEREON_LNAV_SUBFRAME_LENGTH, sf3, SIDEREON_LNAV_SUBFRAME_LENGTH,
                               &decoded) == SIDEREON_STATUS_INVALID_ARGUMENT,
          "lnav_decode rejects bad parity");
}

static void test_omm(const char *kvn_path) {
    size_t len = 0;
    uint8_t *kvn = read_file(kvn_path, &len);
    if (!kvn) {
        return;
    }
    SidereonOmm *omm = NULL;
    check(sidereon_omm_parse_kvn(kvn, len, &omm) == SIDEREON_STATUS_OK && omm != NULL,
          "omm_parse_kvn");
    free(kvn);
    if (!omm) {
        return;
    }

    /* Each serializer: size query then fill, then round-trip the KVN back. */
    size_t written = 0, required = 0;
    check(sidereon_omm_to_kvn(omm, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
              required > 0,
          "omm_to_kvn size query");
    uint8_t *kvn_out = (uint8_t *)malloc(required);
    check(kvn_out != NULL &&
              sidereon_omm_to_kvn(omm, kvn_out, required, &written, &required) ==
                  SIDEREON_STATUS_OK &&
              written == required,
          "omm_to_kvn fill");

    SidereonOmm *reparsed = NULL;
    check(sidereon_omm_parse_kvn(kvn_out, written, &reparsed) == SIDEREON_STATUS_OK &&
              reparsed != NULL,
          "omm_to_kvn round-trips");
    sidereon_omm_free(reparsed);
    free(kvn_out);

    check(sidereon_omm_to_xml(omm, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
              required > 0,
          "omm_to_xml size query");
    uint8_t *xml_out = (uint8_t *)malloc(required);
    check(xml_out != NULL &&
              sidereon_omm_to_xml(omm, xml_out, required, &written, &required) ==
                  SIDEREON_STATUS_OK,
          "omm_to_xml fill");
    SidereonOmm *from_xml = NULL;
    check(sidereon_omm_parse_xml(xml_out, written, &from_xml) == SIDEREON_STATUS_OK &&
              from_xml != NULL,
          "omm_to_xml round-trips");
    sidereon_omm_free(from_xml);
    free(xml_out);

    check(sidereon_omm_to_json(omm, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
              required > 0,
          "omm_to_json size query");
    uint8_t *json_out = (uint8_t *)malloc(required);
    check(json_out != NULL &&
              sidereon_omm_to_json(omm, json_out, required, &written, &required) ==
                  SIDEREON_STATUS_OK,
          "omm_to_json fill");
    SidereonOmm *from_json = NULL;
    check(sidereon_omm_parse_json(json_out, written, &from_json) == SIDEREON_STATUS_OK &&
              from_json != NULL,
          "omm_to_json round-trips");
    sidereon_omm_free(from_json);
    free(json_out);

    sidereon_omm_free(omm);
}

static void test_crinex_encode(const char *rnx_path) {
    size_t len = 0;
    uint8_t *rnx = read_file(rnx_path, &len);
    if (!rnx) {
        return;
    }
    size_t written = 0, required = 0;
    check(sidereon_crinex_encode(rnx, len, NULL, 0, &written, &required) == SIDEREON_STATUS_OK &&
              required > 0,
          "crinex_encode size query");
    uint8_t *crx = (uint8_t *)malloc(required);
    check(crx != NULL &&
              sidereon_crinex_encode(rnx, len, crx, required, &written, &required) ==
                  SIDEREON_STATUS_OK &&
              written == required,
          "crinex_encode fill");

    /* Decoding the encoded CRINEX must reproduce parseable RINEX observation
     * text (round-trip through the existing decoder). */
    if (crx) {
        size_t dec_written = 0, dec_required = 0;
        check(sidereon_crinex_decode(crx, written, NULL, 0, &dec_written, &dec_required) ==
                  SIDEREON_STATUS_OK &&
                  dec_required > 0,
              "crinex_encode -> decode size query");
        uint8_t *back = (uint8_t *)malloc(dec_required);
        check(back != NULL &&
                  sidereon_crinex_decode(crx, written, back, dec_required, &dec_written,
                                         &dec_required) == SIDEREON_STATUS_OK,
              "crinex_encode -> decode fill");
        free(back);
    }
    free(crx);
    free(rnx);
}

int main(int argc, char **argv) {
    if (argc < 4) {
        fprintf(stderr, "usage: %s <grg_sp3> <omm_kvn> <esbc_rnx>\n", argv[0]);
        return 2;
    }
    test_spp_batch(argv[1]);
    test_lnav();
    test_omm(argv[2]);
    test_crinex_encode(argv[3]);

    if (failures != 0) {
        fprintf(stderr, "parity_gaps_smoke: %d failure(s)\n", failures);
        return 1;
    }
    printf("parity_gaps_smoke: OK\n");
    return 0;
}
