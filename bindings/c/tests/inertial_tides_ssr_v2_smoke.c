#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sidereon.h"
#include "w6_ssr_size_policy_pins.h"

static int failures;

static void check(bool condition, const char *label) {
    if (!condition) {
        fprintf(stderr, "FAIL: %s\n", label);
        failures++;
    }
}

static void check_status(SidereonStatus got, SidereonStatus expected, const char *label) {
    check(got == expected, label);
}

static uint64_t f64_bits(double value) {
    uint64_t bits;
    memcpy(&bits, &value, sizeof(bits));
    return bits;
}

static void test_inertial_abi(void) {
    SidereonInertialConstants constants;
    check_status(sidereon_inertial_constants(&constants), SIDEREON_STATUS_OK,
                 "inertial constants declaration and output type");
    check(constants.normal_gravity_equator_mps2 > 9.7 &&
              constants.normal_gravity_equator_mps2 < 9.9,
          "inertial constants fields");

    SidereonInertialNavState initial = {0};
    initial.position_ecef_m[0] = 6378137.0;
    initial.attitude_body_to_ecef[0] = 1.0;
    initial.attitude_body_to_ecef[4] = 1.0;
    initial.attitude_body_to_ecef[8] = 1.0;
    SidereonInertialMechanizer *mechanizer = NULL;
    check_status(sidereon_inertial_mechanizer_new(&initial, &mechanizer), SIDEREON_STATUS_OK,
                 "inertial mechanizer construction");
    SidereonInertialNavState state = {0};
    check_status(sidereon_inertial_mechanizer_state(mechanizer, &state), SIDEREON_STATUS_OK,
                 "inertial state output declaration");
    check(state.position_ecef_m[0] == initial.position_ecef_m[0],
          "inertial state copied to caller");
    sidereon_inertial_mechanizer_free(mechanizer);

    mechanizer = (SidereonInertialMechanizer *)(uintptr_t)1;
    check_status(sidereon_inertial_mechanizer_new(NULL, &mechanizer),
                 SIDEREON_STATUS_NULL_POINTER, "inertial null input status");
    check(mechanizer == NULL, "inertial null input clears handle output");

    initial.position_ecef_m[0] = NAN;
    check_status(sidereon_inertial_mechanizer_new(&initial, &mechanizer),
                 SIDEREON_STATUS_INVALID_ARGUMENT, "inertial typed invalid-input status");
    SidereonInertialError error = {0};
    check_status(sidereon_inertial_last_error(&error), SIDEREON_STATUS_OK,
                 "inertial typed error accessor");
    check(error.kind == SIDEREON_INERTIAL_ERROR_KIND_INVALID_INPUT && error.has_field,
          "inertial invalid-input discriminant and field flag");
    check(mechanizer == NULL, "inertial invalid input clears handle output");
}

static void test_spp_model_options_abi(const char *nav_path) {
    SidereonSppModelOptions models = {0};
    check_status(sidereon_spp_model_options_init(&models), SIDEREON_STATUS_OK,
                 "SPP model options defaults");
    check(models.qzss_clock == SIDEREON_QZSS_CLOCK_GPS &&
              models.troposphere_model == SIDEREON_TROPOSPHERE_MODEL_RTKLIB,
          "SPP model defaults match the engine");
    models.qzss_clock = SIDEREON_QZSS_CLOCK_SEPARATE;
    models.troposphere_model = SIDEREON_TROPOSPHERE_MODEL_SAASTAMOINEN_NIELL;
    SidereonStaticPositionOptionsV2 options = {0};
    check_status(sidereon_static_position_options_v2_init(&options), SIDEREON_STATUS_OK,
                 "static model options defaults");
    options.models = models;
    check(options.models.qzss_clock == SIDEREON_QZSS_CLOCK_SEPARATE &&
              options.models.troposphere_model ==
                  SIDEREON_TROPOSPHERE_MODEL_SAASTAMOINEN_NIELL,
          "checked model selector declarations and static V2 options");

    SidereonBroadcastEphemeris *broadcast = NULL;
    check_status(sidereon_broadcast_ephemeris_load_nav(nav_path, &broadcast), SIDEREON_STATUS_OK,
                 "model-selector broadcast source load");
    SidereonSppInputsV2 inputs = {0};
    check_status(sidereon_spp_inputs_v2_init(&inputs), SIDEREON_STATUS_OK,
                 "model-selector V2 input defaults");
    SidereonExactEpoch *epoch = NULL;
    check_status(sidereon_exact_epoch_from_civil(2024, 1, 1, 0, 0, 0.0, &epoch),
                 SIDEREON_STATUS_OK, "model-selector exact receive epoch");
    SidereonSppSolution *solution = (SidereonSppSolution *)(uintptr_t)1;
    models.qzss_clock = 99;
    check_status(sidereon_solve_broadcast_with_models_at_exact_epoch(
                     broadcast, &inputs, &models, epoch, &solution),
                 SIDEREON_STATUS_INVALID_ARGUMENT, "exact solve refuses unknown model selector");
    check(solution == NULL, "invalid model selector clears solution output");
    sidereon_exact_epoch_free(epoch);
    sidereon_broadcast_ephemeris_free(broadcast);
}

static void test_rtk_exact_epoch_v2_abi(void) {
    SidereonRtkArcEpochV2 arc_epoch = {0};
    arc_epoch.has_prediction_epoch = true;
    SidereonRtkArcSolution *arc_solution = NULL;
    check_status(sidereon_solve_rtk_arc_v2(&arc_epoch, 1, NULL, &arc_solution),
                 SIDEREON_STATUS_NULL_POINTER,
                 "single-frequency RTK exact-epoch V2 ABI");

    SidereonRtkStaticArcSolution *static_solution = NULL;
    check_status(sidereon_solve_static_rtk_arc_v2(&arc_epoch, 1, NULL, &static_solution),
                 SIDEREON_STATUS_NULL_POINTER,
                 "static RTK exact-epoch V2 ABI");

    SidereonRtkDualFrequencyArcEpochV2 dual_epoch = {0};
    dual_epoch.has_gap_epoch = true;
    dual_epoch.has_prediction_epoch = true;
    SidereonRtkWideLaneArcSolution *wide_lane_solution = NULL;
    check_status(sidereon_fix_wide_lane_rtk_arc_v2(&dual_epoch, 1, NULL, &wide_lane_solution),
                 SIDEREON_STATUS_NULL_POINTER,
                 "wide-lane exact-gap V2 ABI");

    SidereonRtkIonosphereFreeArcSolution *ionosphere_free_solution = NULL;
    check_status(sidereon_prepare_ionosphere_free_rtk_arc_v2(
                     &dual_epoch, 1, NULL, 0, NULL, &ionosphere_free_solution),
                 SIDEREON_STATUS_NULL_POINTER,
                 "ionosphere-free exact-gap V2 ABI");
}

static void test_station_tide_batch(void) {
    const double position[3] = {6378137.0, 0.0, 0.0};
    SidereonStationTideEpoch epochs[2] = {
        {.year = 2024, .month = 1, .day = 1},
        {.year = 2024, .month = 13, .day = 1},
    };
    SidereonStationTideOptions options = {0};
    options.constants = SIDEREON_STATION_TIDE_CONSTANTS_CONVENTIONS;
    options.validity_mode = SIDEREON_STATION_TIDE_VALIDITY_MODE_STRICT;
    SidereonStationTideBatchRow rows[2] = {0};
    check_status(sidereon_station_tide_displacement_batch(position, epochs, 2, &options, rows),
                 SIDEREON_STATUS_OK, "station tide batch status");
    check(rows[0].status == SIDEREON_STATUS_OK &&
              rows[0].error.kind == SIDEREON_STATION_TIDE_ERROR_KIND_NONE,
          "station tide valid row");
    check(rows[1].status == SIDEREON_STATUS_INVALID_ARGUMENT &&
              rows[1].error.kind == SIDEREON_STATION_TIDE_ERROR_KIND_INVALID_INPUT &&
              rows[1].error.has_field,
          "station tide row-local typed error");
    uint8_t field[32];
    size_t written = 0;
    size_t required = 0;
    check_status(sidereon_station_tide_batch_error_text(
                     1, SIDEREON_STATION_TIDE_ERROR_TEXT_FIELD, field, sizeof(field), &written,
                     &required),
                 SIDEREON_STATUS_OK, "station tide typed field text");
    check(written > 0 && required == written, "station tide field text counts");
}

/* An SSR correction above RTKLIB's 10 m limit under both size policies:
 * sidereon-core's size tests raise the G30 radial orbit correction of the real
 * 1060 epoch past the limit, and w6_ssr_size_policy_pins.h holds that frame and
 * sidereon-core's results for this route. */
static void test_ssr_size_policy(const char *nav_path) {
    SidereonGnssWeekTow epoch = {
        .system = SIDEREON_TIME_SCALE_GPST,
        .week = W6_SSR_SIZE_WEEK,
    };
    memcpy(&epoch.tow_s, &W6_SSR_SIZE_TOW_S_BITS, sizeof(epoch.tow_s));
    SidereonSsrCorrectionStore *store = NULL;
    check_status(sidereon_ssr_store_from_rtcm(W6_SSR_SIZE_FRAME, W6_SSR_SIZE_FRAME_LEN, &epoch,
                                              &store),
                 SIDEREON_STATUS_OK, "SSR oversized fixture store construction");
    SidereonBroadcastEphemeris *broadcast = NULL;
    check_status(sidereon_broadcast_ephemeris_load_nav(nav_path, &broadcast),
                 SIDEREON_STATUS_OK, "SSR broadcast fixture load");
    double query_s;
    memcpy(&query_s, &W6_SSR_SIZE_QUERY_J2000_S_BITS, sizeof(query_s));
    double staleness_s;
    memcpy(&staleness_s, &W6_SSR_SIZE_STALENESS_S_BITS, sizeof(staleness_s));
    SidereonExactEpochQuery *query = NULL;
    check_status(sidereon_exact_epoch_query_from_binary_j2000_seconds(query_s, &query),
                 SIDEREON_STATUS_OK, "SSR exact query construction");
    if (store == NULL || broadcast == NULL || query == NULL) {
        sidereon_exact_epoch_query_free(query);
        sidereon_broadcast_ephemeris_free(broadcast);
        sidereon_ssr_store_free(store);
        return;
    }

    const uint32_t decline = SIDEREON_SSR_MISSING_CORRECTION_ACTION_DECLINE;
    SidereonSsrCorrectedStateResult strict = {0};
    check_status(sidereon_ssr_corrected_state_at_epoch_queries(
                     broadcast, store, W6_SSR_SIZE_SATELLITE, query, query, staleness_s, decline,
                     false, 0, SIDEREON_SSR_CORRECTION_SIZE_POLICY_STRICT, &strict),
                 SIDEREON_STATUS_OK, "SSR strict exact-query route");
    check(strict.has_state == W6_SSR_SIZE_STRICT_HAS_STATE && strict.has_size_event &&
              strict.strict_refusal && !strict.has_oversized_report,
          "SSR strict refuses the oversized correction");
    check(f64_bits(strict.size.orbit_m) == W6_SSR_SIZE_ORBIT_M_BITS &&
              f64_bits(strict.size.clock_m) == W6_SSR_SIZE_CLOCK_M_BITS &&
              W6_SSR_SIZE_EXCEEDS_LIMIT,
          "SSR strict reports sidereon-core's measured size");

    SidereonSsrCorrectedStateResult lenient = {0};
    check_status(sidereon_ssr_corrected_state_at_epoch_queries(
                     broadcast, store, W6_SSR_SIZE_SATELLITE, query, query, staleness_s, decline,
                     false, 0, SIDEREON_SSR_CORRECTION_SIZE_POLICY_LENIENT, &lenient),
                 SIDEREON_STATUS_OK, "SSR lenient exact-query route");
    check(lenient.has_state && lenient.has_size_event && !lenient.strict_refusal &&
              lenient.has_oversized_report,
          "SSR lenient applies and reports the oversized correction");
    check(f64_bits(lenient.position_ecef_m[0]) == W6_SSR_SIZE_LENIENT_POSITION_ECEF_M_BITS[0] &&
              f64_bits(lenient.position_ecef_m[1]) ==
                  W6_SSR_SIZE_LENIENT_POSITION_ECEF_M_BITS[1] &&
              f64_bits(lenient.position_ecef_m[2]) ==
                  W6_SSR_SIZE_LENIENT_POSITION_ECEF_M_BITS[2] &&
              f64_bits(lenient.clock_s) == W6_SSR_SIZE_LENIENT_CLOCK_S_BITS,
          "SSR lenient state is sidereon-core's");
    check(f64_bits(lenient.size.orbit_m) == W6_SSR_SIZE_REPORT_ORBIT_M_BITS &&
              f64_bits(lenient.size.clock_m) == W6_SSR_SIZE_REPORT_CLOCK_M_BITS &&
              lenient.provider_id == W6_SSR_SIZE_REPORT_PROVIDER_ID &&
              lenient.solution_id == W6_SSR_SIZE_REPORT_SOLUTION_ID &&
              f64_bits(lenient.orbit_ref_epoch_j2000_s) ==
                  W6_SSR_SIZE_REPORT_ORBIT_REF_EPOCH_J2000_S_BITS &&
              f64_bits(lenient.clock_ref_epoch_j2000_s) ==
                  W6_SSR_SIZE_REPORT_CLOCK_REF_EPOCH_J2000_S_BITS &&
              f64_bits(lenient.first_applied_epoch_j2000_s) ==
                  W6_SSR_SIZE_REPORT_T_J2000_S_BITS,
          "SSR lenient report is sidereon-core's");
    check(lenient.size.orbit_m == strict.size.orbit_m &&
              lenient.size.clock_m == strict.size.clock_m,
          "SSR reports the same measured correction size under both policies");

    sidereon_exact_epoch_query_free(query);
    sidereon_broadcast_ephemeris_free(broadcast);
    sidereon_ssr_store_free(store);
}

int main(int argc, char **argv) {
    if (argc != 3) {
        fprintf(stderr, "usage: %s <nav-file> <ssr-nav-file>\n", argv[0]);
        return 2;
    }
    test_inertial_abi();
    test_spp_model_options_abi(argv[1]);
    test_rtk_exact_epoch_v2_abi();
    test_station_tide_batch();
    test_ssr_size_policy(argv[2]);
    if (failures != 0) {
        fprintf(stderr, "inertial_tides_ssr_v2_smoke: %d failure(s)\n", failures);
        return 1;
    }
    puts("inertial_tides_ssr_v2_smoke: OK");
    return 0;
}
