/*
 * Round 2 capability-parity smoke program. Exercises the harder-to-marshal
 * bindings added in round 2: high-accuracy frame transforms + TimeScales,
 * nutation/precession, broadcast orbit/clock from Keplerian elements, RINEX NAV
 * serialize, angles-only IOD, GNSS signal correlation/acquisition, the quality
 * remainder (sigmas/weight_vector/raim_for_solution/validate_receiver_solution),
 * cycle-slip detection, ionosphere-free phase combination, encounter-plane
 * covariance projection, the TCA family, and the PPP static correction
 * precompute. Built with -Wall -Wextra -Werror by run_smoke.sh.
 *
 * argv[1] = GRG SP3 (the spp_fixture reference product, reused for the SPP solve
 *           and the PPP-corrections build), argv[2] = RINEX mixed NAV file.
 */
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sidereon.h"
#include "spp_fixture.h"

static double bits_to_f64(uint64_t bits) {
    double value;
    memcpy(&value, &bits, sizeof(value));
    return value;
}

static uint64_t f64_to_bits(double value) {
    uint64_t bits;
    memcpy(&bits, &value, sizeof(bits));
    return bits;
}

static int fail(const char *what, int code) {
    fprintf(stderr, "round2_smoke: %s failed\n", what);
    return code;
}

static int finite3(const double v[3]) {
    return isfinite(v[0]) && isfinite(v[1]) && isfinite(v[2]);
}

static int finite9(const double m[9]) {
    for (int i = 0; i < 9; i++) {
        if (!isfinite(m[i])) {
            return 0;
        }
    }
    return 1;
}

/* ----------------------------- frame transforms ------------------------- */

static int exercise_frames(void) {
    SidereonTimeScales ts;
    if (sidereon_timescales_from_utc(2020, 6, 25, 12, 0, 0.0, &ts) != SIDEREON_STATUS_OK) {
        return fail("sidereon_timescales_from_utc", 1);
    }
    if (!isfinite(ts.jd_tt) || ts.jd_whole <= 0.0) {
        return fail("timescales fields finite", 1);
    }

    double gi[9];
    double ig[9];
    if (sidereon_frame_gcrs_to_itrs_matrix(&ts, gi) != SIDEREON_STATUS_OK ||
        sidereon_frame_itrs_to_gcrs_matrix(&ts, ig) != SIDEREON_STATUS_OK) {
        return fail("frame matrices", 1);
    }
    if (!finite9(gi) || !finite9(ig)) {
        return fail("frame matrices finite", 1);
    }
    /* itrs_to_gcrs is the transpose of gcrs_to_itrs. */
    for (int r = 0; r < 3; r++) {
        for (int c = 0; c < 3; c++) {
            if (fabs(gi[r * 3 + c] - ig[c * 3 + r]) > 1e-12) {
                return fail("frame matrices transpose relation", 1);
            }
        }
    }

    double pole[9];
    if (sidereon_frame_polar_motion_matrix(0.0, 0.0, pole) != SIDEREON_STATUS_OK) {
        return fail("polar_motion_matrix", 1);
    }
    const double identity[9] = {1, 0, 0, 0, 1, 0, 0, 0, 1};
    for (int i = 0; i < 9; i++) {
        if (fabs(pole[i] - identity[i]) > 1e-15) {
            return fail("zero polar motion is identity", 1);
        }
    }

    double gmst = 0.0;
    double gast = 0.0;
    if (sidereon_frame_gmst_radians(&ts, &gmst) != SIDEREON_STATUS_OK ||
        sidereon_frame_gast_radians(&ts, &gast) != SIDEREON_STATUS_OK) {
        return fail("sidereal time", 1);
    }
    if (!isfinite(gmst) || !isfinite(gast)) {
        return fail("sidereal time finite", 1);
    }

    /* GCRS -> ITRS -> GCRS round trip. */
    const double gcrs[3] = {7000.0, 1500.0, -2200.0};
    double itrs[3];
    double back[3];
    if (sidereon_frame_gcrs_to_itrs(gcrs, &ts, false, itrs) != SIDEREON_STATUS_OK ||
        sidereon_frame_itrs_to_gcrs(itrs, &ts, back) != SIDEREON_STATUS_OK) {
        return fail("gcrs/itrs round trip", 1);
    }
    for (int i = 0; i < 3; i++) {
        if (fabs(back[i] - gcrs[i]) > 1e-6) {
            return fail("gcrs/itrs round trip closes", 1);
        }
    }

    /* geodetic -> ITRS -> geodetic round trip. */
    double itrs_km[3];
    double geo[3];
    if (sidereon_frame_geodetic_to_itrs(37.0, -122.0, 0.1, itrs_km) != SIDEREON_STATUS_OK ||
        sidereon_frame_itrs_to_geodetic(itrs_km, geo) != SIDEREON_STATUS_OK) {
        return fail("geodetic/itrs round trip", 1);
    }
    if (fabs(geo[0] - 37.0) > 1e-6 || fabs(geo[1] - (-122.0)) > 1e-6 || fabs(geo[2] - 0.1) > 1e-6) {
        return fail("geodetic/itrs round trip closes", 1);
    }

    /* TEME -> GCRS produces finite output. */
    const double teme_pos[3] = {-4000.0, 5000.0, 3000.0};
    const double teme_vel[3] = {-3.0, -2.0, 6.0};
    double gpos[3];
    double gvel[3];
    if (sidereon_frame_teme_to_gcrs(teme_pos, teme_vel, &ts, true, gpos, gvel) != SIDEREON_STATUS_OK) {
        return fail("teme_to_gcrs", 1);
    }
    if (!finite3(gpos) || !finite3(gvel)) {
        return fail("teme_to_gcrs finite", 1);
    }

    /* Independent Skyfield 1.49 oracle, captured as IEEE-754 bit patterns. */
    SidereonTimeScales skyfield_ts;
    if (sidereon_timescales_from_utc(2018, 7, 4, 0, 0, 0.0, &skyfield_ts) != SIDEREON_STATUS_OK) {
        return fail("skyfield reference epoch", 1);
    }
    const uint64_t teme_pos_bits[3] = {
        UINT64_C(0x40ace86c23dffb6b), UINT64_C(0x409f7fa61c81cb47), UINT64_C(0x40b4bd8359159cde)};
    const uint64_t teme_vel_bits[3] = {
        UINT64_C(0xc00b2ffb7cf9ad7d), UINT64_C(0x401b7a8751f7fc4a), UINT64_C(0xbfceb36925f07cb4)};
    const uint64_t gcrs_pos_bits[3] = {
        UINT64_C(0x40ad0bd9193713e1), UINT64_C(0x409f41a3b2073733), UINT64_C(0x40b4b6ffad1289d1)};
    const uint64_t gcrs_vel_bits[3] = {
        UINT64_C(0xc00af690723d6cb1), UINT64_C(0x401b88e06212f969), UINT64_C(0xbfcde8575471eaf0)};
    const uint64_t itrs_pos_bits[3] = {
        UINT64_C(0xc092d5d32b319db8), UINT64_C(0x40af8b3b3a722474), UINT64_C(0x40b4bd8359159cdb)};
    double skyfield_teme_pos[3];
    double skyfield_teme_vel[3];
    for (int i = 0; i < 3; i++) {
        skyfield_teme_pos[i] = bits_to_f64(teme_pos_bits[i]);
        skyfield_teme_vel[i] = bits_to_f64(teme_vel_bits[i]);
    }
    if (sidereon_frame_teme_to_gcrs(skyfield_teme_pos, skyfield_teme_vel, &skyfield_ts, true,
                                    gpos, gvel) != SIDEREON_STATUS_OK) {
        return fail("skyfield teme_to_gcrs", 1);
    }
    for (int i = 0; i < 3; i++) {
        if (f64_to_bits(gpos[i]) != gcrs_pos_bits[i] || f64_to_bits(gvel[i]) != gcrs_vel_bits[i]) {
            return fail("skyfield teme_to_gcrs zero ULP", 1);
        }
    }
    if (sidereon_frame_gcrs_to_itrs(gpos, &skyfield_ts, true, itrs) != SIDEREON_STATUS_OK) {
        return fail("skyfield gcrs_to_itrs", 1);
    }
    for (int i = 0; i < 3; i++) {
        if (f64_to_bits(itrs[i]) != itrs_pos_bits[i]) {
            return fail("skyfield gcrs_to_itrs zero ULP", 1);
        }
    }

    /* mat3_vec3_mul with identity returns the vector unchanged. */
    const double vec[3] = {1.5, -2.5, 3.5};
    double prod[3];
    if (sidereon_frame_mat3_vec3_mul(identity, vec, prod) != SIDEREON_STATUS_OK) {
        return fail("mat3_vec3_mul", 1);
    }
    for (int i = 0; i < 3; i++) {
        if (fabs(prod[i] - vec[i]) > 1e-15) {
            return fail("mat3_vec3_mul identity", 1);
        }
    }

    /* topocentric az/el/range finite. */
    double topo[3];
    if (sidereon_frame_gcrs_to_topocentric(gcrs, 37.0, -122.0, 0.0, &ts, false, topo) !=
        SIDEREON_STATUS_OK) {
        return fail("gcrs_to_topocentric", 1);
    }
    if (!finite3(topo)) {
        return fail("gcrs_to_topocentric finite", 1);
    }
    return 0;
}

static int exercise_nutation_precession(void) {
    double dpsi = 0.0;
    double deps = 0.0;
    if (sidereon_nutation_iau2000a_radians(2459000.0, &dpsi, &deps) != SIDEREON_STATUS_OK ||
        !isfinite(dpsi) || !isfinite(deps)) {
        return fail("nutation_iau2000a_radians", 1);
    }
    double mean_ob = 0.0;
    if (sidereon_nutation_mean_obliquity_radians(2459000.0, &mean_ob) != SIDEREON_STATUS_OK ||
        !(mean_ob > 0.4 && mean_ob < 0.42)) {
        return fail("mean_obliquity_radians", 1);
    }
    double fa[5];
    for (int i = 0; i < 5; i++) {
        fa[i] = 0.0;
    }
    if (sidereon_nutation_fundamental_arguments(0.2, fa) != SIDEREON_STATUS_OK) {
        return fail("fundamental_arguments", 1);
    }
    double eqe = 0.0;
    if (sidereon_nutation_equation_of_equinoxes_terms(2459000.0, &eqe) != SIDEREON_STATUS_OK ||
        !isfinite(eqe)) {
        return fail("equation_of_equinoxes_terms", 1);
    }
    double nmat[9];
    if (sidereon_nutation_matrix(mean_ob, mean_ob + deps, dpsi, nmat) != SIDEREON_STATUS_OK ||
        !finite9(nmat)) {
        return fail("nutation_matrix", 1);
    }
    double pmat[9];
    if (sidereon_precession_matrix(2459000.0, pmat) != SIDEREON_STATUS_OK || !finite9(pmat)) {
        return fail("precession_matrix", 1);
    }
    double bias[9];
    if (sidereon_precession_icrs_to_j2000_matrix(bias) != SIDEREON_STATUS_OK || !finite9(bias)) {
        return fail("icrs_to_j2000_matrix", 1);
    }
    return 0;
}

/* --------------------------- broadcast orbit/clock ---------------------- */

static int exercise_broadcast_keplerian(void) {
    /* GPS broadcast constants (IS-GPS-200). */
    SidereonConstellationConstants consts = {
        .gm_m3_s2 = 3.986005e14,
        .omega_e_rad_s = 7.2921151467e-5,
        .dtr_f = -4.442807633e-10,
    };
    SidereonKeplerianElements el;
    memset(&el, 0, sizeof(el));
    el.sqrt_a = 5153.65;     /* ~ a = 2.656e7 m */
    el.e = 0.005;
    el.m0 = 0.3;
    el.delta_n = 4.5e-9;
    el.omega0 = -1.0;
    el.i0 = 0.96;
    el.omega = 0.5;
    el.omega_dot = -8.0e-9;
    el.idot = 1.0e-10;
    el.toe_sow = 432000.0;

    SidereonOrbitState orbit;
    if (sidereon_broadcast_satellite_position_ecef(&el, &consts, 432000.0, false, &orbit) !=
        SIDEREON_STATUS_OK) {
        return fail("broadcast_satellite_position_ecef", 1);
    }
    double r = sqrt(orbit.x_m * orbit.x_m + orbit.y_m * orbit.y_m + orbit.z_m * orbit.z_m);
    if (!(r > 1.5e7 && r < 3.0e7)) {
        return fail("broadcast orbit radius plausible", 1);
    }

    SidereonClockPolynomial clock = {
        .af0 = 1.0e-4, .af1 = 1.0e-11, .af2 = 0.0, .toc_sow = 432000.0};
    SidereonClockOffset offset;
    if (sidereon_broadcast_satellite_clock_offset_s(&clock, &consts, &el, orbit.sin_e, 432000.0,
                                                    3.0e-9, &offset) != SIDEREON_STATUS_OK) {
        return fail("broadcast_satellite_clock_offset_s", 1);
    }
    if (!isfinite(offset.dt_clock_total_s)) {
        return fail("clock offset finite", 1);
    }

    SidereonSatelliteState state;
    if (sidereon_broadcast_satellite_state(&el, &clock, &consts, 432000.0, 3.0e-9, false, &state) !=
        SIDEREON_STATUS_OK) {
        return fail("broadcast_satellite_state", 1);
    }
    if (fabs(state.orbit.x_m - orbit.x_m) > 1e-3 ||
        fabs(state.clock.dt_clock_total_s - offset.dt_clock_total_s) > 1e-15) {
        return fail("satellite_state matches components", 1);
    }
    return 0;
}

/* ----------------------------- RINEX NAV encode ------------------------- */

static char *read_file(const char *path, size_t *out_len) {
    FILE *f = fopen(path, "rb");
    if (!f) {
        return NULL;
    }
    fseek(f, 0, SEEK_END);
    long n = ftell(f);
    fseek(f, 0, SEEK_SET);
    if (n < 0) {
        fclose(f);
        return NULL;
    }
    char *buf = malloc((size_t)n);
    if (!buf) {
        fclose(f);
        return NULL;
    }
    if (fread(buf, 1, (size_t)n, f) != (size_t)n) {
        free(buf);
        fclose(f);
        return NULL;
    }
    fclose(f);
    *out_len = (size_t)n;
    return buf;
}

static int exercise_rinex_encode_nav(const char *nav_path) {
    size_t len = 0;
    char *bytes = read_file(nav_path, &len);
    if (!bytes) {
        return fail("read nav file", 1);
    }
    SidereonBroadcastEphemeris *eph = NULL;
    SidereonStatus st =
        sidereon_broadcast_ephemeris_parse_nav((const uint8_t *)bytes, len, &eph);
    free(bytes);
    if (st != SIDEREON_STATUS_OK || eph == NULL) {
        return fail("parse_nav for encode", 1);
    }

    size_t written = 0;
    size_t required = 0;
    if (sidereon_rinex_encode_nav(eph, NULL, 0, &written, &required) != SIDEREON_STATUS_OK ||
        required == 0) {
        sidereon_broadcast_ephemeris_free(eph);
        return fail("encode_nav sizing", 1);
    }
    char *out = malloc(required + 1);
    if (!out) {
        sidereon_broadcast_ephemeris_free(eph);
        return fail("encode_nav alloc", 1);
    }
    if (sidereon_rinex_encode_nav(eph, (uint8_t *)out, required, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != required) {
        free(out);
        sidereon_broadcast_ephemeris_free(eph);
        return fail("encode_nav write", 1);
    }
    out[written] = '\0';
    int ok = (strstr(out, "RINEX VERSION / TYPE") != NULL) &&
             (strstr(out, "END OF HEADER") != NULL);
    free(out);
    sidereon_broadcast_ephemeris_free(eph);
    if (!ok) {
        return fail("encode_nav header content", 1);
    }
    return 0;
}

/* ------------------------------ angles-only IOD ------------------------- */

static int exercise_iod_gauss(void) {
    /* The exact orbit recovery needs a real geometry; the smoke confirms the
     * marshalling and delegation run (a converged orbit or a typed validation
     * error, never a panic or null-pointer fault). */
    const double decl[3] = {0.1, 0.12, 0.14};
    const double rtasc[3] = {0.5, 0.55, 0.6};
    const double jd[3] = {2459000.0, 2459000.0, 2459000.0};
    const double jdf[3] = {0.0, 0.0006944, 0.0013888};
    const double rseci[9] = {-5000.0, 0.0, 3500.0, -4990.0, 100.0, 3510.0, -4980.0, 200.0, 3520.0};
    double pos[3];
    double vel[3];
    SidereonStatus st = sidereon_iod_gauss_angles(decl, rtasc, jd, jdf, rseci, pos, vel);
    if (st == SIDEREON_STATUS_PANIC || st == SIDEREON_STATUS_NULL_POINTER) {
        return fail("iod_gauss_angles ran", 1);
    }
    if (st == SIDEREON_STATUS_OK && (!finite3(pos) || !finite3(vel))) {
        return fail("iod_gauss_angles output finite", 1);
    }
    return 0;
}

/* ----------------- iono-free phase + encounter-plane covariance --------- */

static int exercise_combination_and_covariance(void) {
    double iono_free = 0.0;
    if (sidereon_combination_ionosphere_free_phase_cycles(1.0e8, 0.9e8, 1575.42e6, 1227.6e6,
                                                          &iono_free) != SIDEREON_STATUS_OK ||
        !isfinite(iono_free)) {
        return fail("ionosphere_free_phase_cycles", 1);
    }

    /* Paired-band ionosphere-free pseudoranges: G01 in both bands combines,
     * G02 only in band 1 is dropped (MissingBand2). */
    const char *b1_ids[2] = {"G01", "G02"};
    const char *b2_ids[1] = {"G01"};
    SidereonPseudorangeObservation band1[2];
    band1[0].sat_id = b1_ids[0];
    band1[0].pseudorange_m = 2.0e7;
    band1[1].sat_id = b1_ids[1];
    band1[1].pseudorange_m = 2.1e7;
    SidereonPseudorangeObservation band2[1];
    band2[0].sat_id = b2_ids[0];
    band2[0].pseudorange_m = 2.0e7 + 5.0;
    SidereonIonoFreePseudoranges *ifp = NULL;
    if (sidereon_combination_ionosphere_free_pseudoranges(band1, 2, band2, 1, NULL, 0, &ifp) !=
            SIDEREON_STATUS_OK ||
        ifp == NULL) {
        return fail("ionosphere_free_pseudoranges", 1);
    }
    SidereonIonoFreeCombined comb[4];
    SidereonIonoFreeDropped drop[4];
    size_t w = 0;
    size_t req = 0;
    if (sidereon_iono_free_pseudoranges_combined(ifp, comb, 4, &w, &req) != SIDEREON_STATUS_OK ||
        req != 1) {
        sidereon_iono_free_pseudoranges_free(ifp);
        return fail("iono_free combined", 1);
    }
    if (sidereon_iono_free_pseudoranges_dropped(ifp, drop, 4, &w, &req) != SIDEREON_STATUS_OK ||
        req != 1 || drop[0].reason != SIDEREON_PSEUDORANGE_DROP_MISSING_BAND2) {
        sidereon_iono_free_pseudoranges_free(ifp);
        return fail("iono_free dropped", 1);
    }
    sidereon_iono_free_pseudoranges_free(ifp);

    const double r1[3] = {7000.0, 0.0, 0.0};
    const double v1[3] = {0.0, 7.5, 0.0};
    const double r2[3] = {7000.05, 0.0, 0.5};
    const double v2[3] = {0.0, -7.5, 0.1};
    SidereonEncounterFrame frame;
    if (sidereon_encounter_frame(r1, v1, r2, v2, &frame) != SIDEREON_STATUS_OK) {
        return fail("encounter_frame", 1);
    }
    const double cov[9] = {1.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 3.0};
    double plane[4];
    if (sidereon_encounter_plane_covariance(&frame, cov, plane) != SIDEREON_STATUS_OK) {
        return fail("encounter_plane_covariance", 1);
    }
    if (!isfinite(plane[0]) || !isfinite(plane[3]) || plane[0] < 0.0 || plane[3] < 0.0 ||
        fabs(plane[1] - plane[2]) > 1e-9) {
        return fail("encounter_plane_covariance symmetric psd", 1);
    }
    return 0;
}

/* ------------------------------- GNSS signal ---------------------------- */

static int exercise_signal(void) {
    size_t written = 0;
    size_t required = 0;
    if (sidereon_signal_ca_code(1, NULL, 0, &written, &required) != SIDEREON_STATUS_OK ||
        required != 1023) {
        return fail("ca_code sizing", 1);
    }
    int8_t *code = malloc(1023);
    if (!code) {
        return fail("ca_code alloc", 1);
    }
    if (sidereon_signal_ca_code(1, code, 1023, &written, &required) != SIDEREON_STATUS_OK ||
        written != 1023) {
        free(code);
        return fail("ca_code write", 1);
    }
    for (size_t i = 0; i < 1023; i++) {
        if (code[i] != 1 && code[i] != -1) {
            free(code);
            return fail("ca_code bipolar", 1);
        }
    }

    /* autocorrelation peak at lag 0 equals the code length. */
    int32_t *acorr = malloc(1023 * sizeof(int32_t));
    if (!acorr) {
        free(code);
        return fail("autocorr alloc", 1);
    }
    if (sidereon_signal_autocorrelation(code, 1023, acorr, 1023, &written, &required) !=
            SIDEREON_STATUS_OK ||
        acorr[0] != 1023) {
        free(acorr);
        free(code);
        return fail("autocorrelation peak", 1);
    }
    free(acorr);

    int32_t single = 0;
    if (sidereon_signal_correlation_at(code, code, 1023, 0, &single) != SIDEREON_STATUS_OK ||
        single != 1023) {
        free(code);
        return fail("correlation_at lag 0", 1);
    }

    int32_t *xcorr = malloc(1023 * sizeof(int32_t));
    if (!xcorr) {
        free(code);
        return fail("xcorr alloc", 1);
    }
    if (sidereon_signal_cross_correlation(code, code, 1023, xcorr, 1023, &written, &required) !=
            SIDEREON_STATUS_OK ||
        xcorr[0] != 1023) {
        free(xcorr);
        free(code);
        return fail("cross_correlation self", 1);
    }
    free(xcorr);
    free(code);

    /* Build a clean baseband replica (2 samples/chip => 2046 samples). */
    SidereonReplicaOptions ropts = {
        .sample_rate_hz = 2.046e6, .num_samples = 2046, .code_phase_chips = 0.0,
        .code_doppler_hz = 0.0};
    int8_t *rep = malloc(2046);
    if (!rep) {
        return fail("replica alloc", 1);
    }
    if (sidereon_signal_replica(1, &ropts, rep, 2046, &written, &required) != SIDEREON_STATUS_OK ||
        written != 2046) {
        free(rep);
        return fail("replica write", 1);
    }

    SidereonIqSample *iq = malloc(2046 * sizeof(SidereonIqSample));
    if (!iq) {
        free(rep);
        return fail("iq alloc", 1);
    }
    for (size_t i = 0; i < 2046; i++) {
        iq[i].i = (double)rep[i];
        iq[i].q = 0.0;
    }

    double ci = 0.0;
    double cq = 0.0;
    if (sidereon_signal_correlate_against(iq, 2046, rep, 2046, 2.046e6, 0.0, &ci, &cq) !=
        SIDEREON_STATUS_OK) {
        free(iq);
        free(rep);
        return fail("correlate_against", 1);
    }
    /* Aligned replica against itself accumulates full code energy. */
    if (!(ci > 1000.0)) {
        free(iq);
        free(rep);
        return fail("correlate_against energy", 1);
    }

    SidereonCorrelateOptions copts = {
        .sample_rate_hz = 2.046e6, .doppler_hz = 0.0, .code_phase_chips = 0.0,
        .code_doppler_hz = 0.0};
    SidereonCorrelationResult cres;
    if (sidereon_signal_correlate(iq, 2046, 1, &copts, &cres) != SIDEREON_STATUS_OK ||
        !(cres.power > 0.0)) {
        free(iq);
        free(rep);
        return fail("correlate", 1);
    }

    SidereonAcquisitionOptions aopts = {
        .sample_rate_hz = 2.046e6, .doppler_min_hz = -1000.0, .doppler_max_hz = 1000.0,
        .doppler_step_hz = 500.0};
    SidereonAcquisitionResult ares;
    if (sidereon_signal_acquire(iq, 2046, 1, &aopts, &ares, NULL, 0, &written, &required) !=
            SIDEREON_STATUS_OK ||
        required == 0) {
        free(iq);
        free(rep);
        return fail("acquire sizing", 1);
    }
    double *bins = malloc(required * sizeof(double));
    if (!bins) {
        free(iq);
        free(rep);
        return fail("acquire bins alloc", 1);
    }
    if (sidereon_signal_acquire(iq, 2046, 1, &aopts, &ares, bins, required, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != required || !(ares.peak_power > 0.0)) {
        free(bins);
        free(iq);
        free(rep);
        return fail("acquire", 1);
    }
    free(bins);
    free(iq);
    free(rep);
    return 0;
}

/* -------------------------------- quality ------------------------------- */

static int exercise_quality(const SidereonSp3 *sp3) {
    const char *ids[2] = {"G01", "G02"};
    SidereonWeightEntry entries[2];
    entries[0].sat_id = ids[0];
    entries[0].elevation_deg = 30.0;
    entries[0].has_cn0 = false;
    entries[0].cn0_dbhz = 0.0;
    entries[1].sat_id = ids[1];
    entries[1].elevation_deg = 90.0;
    entries[1].has_cn0 = false;
    entries[1].cn0_dbhz = 0.0;

    SidereonPseudorangeVarianceOptions vopts;
    if (sidereon_pseudorange_variance_options_init(&vopts) != SIDEREON_STATUS_OK) {
        return fail("variance options init", 1);
    }
    double sig[2];
    bool sig_present[2];
    if (sidereon_sigmas(entries, 2, &vopts, sig, sig_present) != SIDEREON_STATUS_OK ||
        !sig_present[0] || !sig_present[1] || !(sig[0] > 0.0) || !(sig[1] > 0.0)) {
        return fail("sigmas", 1);
    }
    double wt[2];
    bool wt_present[2];
    if (sidereon_weight_vector(entries, 2, &vopts, wt, wt_present) != SIDEREON_STATUS_OK ||
        !wt_present[0] || !(wt[0] > 0.0)) {
        return fail("weight_vector", 1);
    }
    /* Higher elevation has smaller sigma and larger weight. */
    if (!(sig[1] <= sig[0]) || !(wt[1] >= wt[0])) {
        return fail("weight ordering by elevation", 1);
    }

    /* Solve an SPP fix from the GRG SP3 + reference inputs, then run the
     * solution-driven RAIM and the receiver-solution validation gates. */
    SidereonObservation obs[SPP_OBS_COUNT];
    for (size_t i = 0; i < SPP_OBS_COUNT; i++) {
        obs[i].sat_id = SPP_SAT_IDS[i];
        obs[i].pseudorange_m = bits_to_f64(SPP_PSEUDORANGE_BITS[i]);
    }
    SidereonSppInputsV2 inputs;
    if (sidereon_spp_inputs_v2_init(&inputs) != SIDEREON_STATUS_OK) {
        return fail("spp inputs init", 1);
    }
    inputs.base.observations = obs;
    inputs.base.observation_count = SPP_OBS_COUNT;
    inputs.base.t_rx_j2000_s = bits_to_f64(SPP_T_RX_J2000_S_BITS);
    inputs.base.t_rx_second_of_day_s = bits_to_f64(SPP_T_RX_SOD_S_BITS);
    inputs.base.day_of_year = bits_to_f64(SPP_DOY_BITS);
    for (int i = 0; i < 4; i++) {
        inputs.base.initial_guess[i] = bits_to_f64(SPP_INITIAL_GUESS_BITS[i]);
        inputs.base.klobuchar_alpha[i] = bits_to_f64(SPP_KLOB_ALPHA_BITS[i]);
        inputs.base.klobuchar_beta[i] = bits_to_f64(SPP_KLOB_BETA_BITS[i]);
    }
    inputs.base.ionosphere = false;
    inputs.base.troposphere = false;
    inputs.base.pressure_hpa = bits_to_f64(SPP_PRESSURE_HPA_BITS);
    inputs.base.temperature_k = bits_to_f64(SPP_TEMPERATURE_K_BITS);
    inputs.base.relative_humidity = bits_to_f64(SPP_RELATIVE_HUMIDITY_BITS);
    inputs.base.with_geodetic = true;

    SidereonSppSolution *sol = NULL;
    if (sidereon_solve_spp_v2(sp3, &inputs, &sol) != SIDEREON_STATUS_OK || sol == NULL) {
        return fail("spp solve for quality", 1);
    }

    SidereonRaimResult raim;
    if (sidereon_raim_for_solution(sol, 0.001, true, NULL, 0, false, 0, &raim) !=
        SIDEREON_STATUS_OK) {
        sidereon_spp_solution_free(sol);
        return fail("raim_for_solution", 1);
    }
    if (!isfinite(raim.test_statistic)) {
        sidereon_spp_solution_free(sol);
        return fail("raim test statistic finite", 1);
    }

    SidereonSolutionValidationOptions svo;
    if (sidereon_solution_validation_options_init(&svo) != SIDEREON_STATUS_OK) {
        sidereon_spp_solution_free(sol);
        return fail("validation options init", 1);
    }
    if (sidereon_validate_receiver_solution(sol, &svo) != SIDEREON_STATUS_OK) {
        sidereon_spp_solution_free(sol);
        return fail("validate_receiver_solution", 1);
    }
    sidereon_spp_solution_free(sol);
    return 0;
}

/* ---------------------------- cycle-slip detection ---------------------- */

static int exercise_cycle_slips(void) {
    SidereonCycleSlipOptions opts;
    if (sidereon_cycle_slip_options_init(&opts) != SIDEREON_STATUS_OK) {
        return fail("cycle slip options init", 1);
    }
    SidereonArcEpoch arc[2];
    for (int i = 0; i < 2; i++) {
        arc[i].phi1_cycles = 1.0e8 + (double)i;
        arc[i].phi2_cycles = 0.78e8 + (double)i;
        arc[i].p1_m = 2.0e7 + (double)i;
        arc[i].p2_m = 2.0e7 + (double)i;
        arc[i].has_lli1 = false;
        arc[i].lli1 = 0;
        arc[i].has_lli2 = false;
        arc[i].lli2 = 0;
        arc[i].f1_hz = 1575.42e6;
        arc[i].f2_hz = 1227.6e6;
        arc[i].gap_time_s = (i == 0) ? NAN : 30.0;
    }
    SidereonSlipResult results[2];
    size_t written = 0;
    size_t required = 0;
    if (sidereon_detect_cycle_slips(arc, 2, &opts, results, 2, &written, &required) !=
            SIDEREON_STATUS_OK ||
        written != 2 || required != 2) {
        return fail("detect_cycle_slips", 1);
    }
    return 0;
}

/* -------------------------------- PPP corrections ----------------------- */

static int assert_ppp_empty(const SidereonPppCorrections *corr) {
    SidereonEpochVectorCorrection vbuf[1];
    SidereonSatScalarCorrection sbuf[1];
    SidereonSatVectorCorrection wbuf[1];
    size_t written = 0;
    size_t required = 1;
    if (sidereon_ppp_corrections_tide(corr, vbuf, 1, &written, &required) != SIDEREON_STATUS_OK ||
        required != 0) {
        return 1;
    }
    if (sidereon_ppp_corrections_pole_tide(corr, vbuf, 1, &written, &required) !=
            SIDEREON_STATUS_OK ||
        required != 0) {
        return 1;
    }
    if (sidereon_ppp_corrections_ocean_loading(corr, vbuf, 1, &written, &required) !=
            SIDEREON_STATUS_OK ||
        required != 0) {
        return 1;
    }
    if (sidereon_ppp_corrections_windup(corr, sbuf, 1, &written, &required) != SIDEREON_STATUS_OK ||
        required != 0) {
        return 1;
    }
    if (sidereon_ppp_corrections_sat_pcv(corr, sbuf, 1, &written, &required) !=
            SIDEREON_STATUS_OK ||
        required != 0) {
        return 1;
    }
    if (sidereon_ppp_corrections_sat_pco_ecef(corr, wbuf, 1, &written, &required) !=
            SIDEREON_STATUS_OK ||
        required != 0) {
        return 1;
    }
    return 0;
}

static int exercise_ppp_corrections(const SidereonSp3 *sp3) {
    const double receiver[3] = {3920000.0, 290000.0, 5010000.0};

    /* (a) All switches off: the engine returns empty tables. */
    SidereonPppCorrectionsOptions off;
    memset(&off, 0, sizeof(off));
    SidereonPppCorrections *corr = NULL;
    if (sidereon_ppp_corrections_build(sp3, NULL, 0, receiver, &off, &corr) != SIDEREON_STATUS_OK ||
        corr == NULL) {
        return fail("ppp_corrections_build (all off)", 1);
    }
    if (assert_ppp_empty(corr) != 0) {
        sidereon_ppp_corrections_free(corr);
        return fail("ppp_corrections empty readers", 1);
    }
    sidereon_ppp_corrections_free(corr);

    /* (b) Pole tide + ocean loading + satellite antenna enabled with no epochs:
     * exercises every option's input marshalling and the receiver validation
     * path; output tables stay empty. */
    SidereonSatelliteAntennaOptions ant;
    memset(&ant, 0, sizeof(ant));
    ant.freq1_label = "G01";
    ant.freq1_hz = 1575.42e6;
    ant.freq2_label = "G02";
    ant.freq2_hz = 1227.6e6;
    ant.antennas = NULL;
    ant.antenna_count = 0;

    SidereonPppCorrectionsOptions on;
    memset(&on, 0, sizeof(on));
    on.solid_earth_tide = false;
    on.has_pole_tide = true;
    on.pole_tide.xp_arcsec = 0.1;
    on.pole_tide.yp_arcsec = 0.2;
    on.has_ocean_loading = true; /* zeroed BLQ is finite and accepted */
    on.phase_windup = false;
    on.has_satellite_antenna = true;
    on.satellite_antenna = &ant;

    SidereonPppCorrections *corr2 = NULL;
    if (sidereon_ppp_corrections_build(sp3, NULL, 0, receiver, &on, &corr2) != SIDEREON_STATUS_OK ||
        corr2 == NULL) {
        return fail("ppp_corrections_build (options on)", 1);
    }
    if (assert_ppp_empty(corr2) != 0) {
        sidereon_ppp_corrections_free(corr2);
        return fail("ppp_corrections empty readers (on)", 1);
    }
    sidereon_ppp_corrections_free(corr2);
    return 0;
}

/* ----------------------------------- TCA -------------------------------- */

/* Two well-separated ISS-era TLEs; the smoke confirms the finder/screen/Pc
 * marshalling runs and yields finite candidates over a short window. */
static const char *const TLE_A1 =
    "1 25544U 98067A   20177.50000000  .00001264  00000-0  29621-4 0  9993";
static const char *const TLE_A2 =
    "2 25544  51.6443 142.0099 0001234  90.0000 270.0000 15.49500000228000";
static const char *const TLE_B1 =
    "1 43205U 18015A   20177.50000000  .00000500  00000-0  20000-4 0  9990";
static const char *const TLE_B2 =
    "2 43205  51.6400 145.0000 0002000  80.0000 280.0000 15.50000000220000";

static int exercise_tca(void) {
    SidereonTcaFinderOptions fopts;
    if (sidereon_tca_finder_options_init(&fopts) != SIDEREON_STATUS_OK) {
        return fail("tca_finder_options_init", 1);
    }
    /* Search window: one day starting at 2020 DOY 177.5 (JD 2459023.0). */
    const double ws_whole = 2459023.0;
    const double ws_frac = 0.0;
    const double we_whole = 2459023.0;
    const double we_frac = 1.0;

    size_t written = 0;
    size_t required = 0;
    SidereonStatus st = sidereon_find_tca_candidates_from_tles(
        TLE_A1, TLE_A2, TLE_B1, TLE_B2, ws_whole, ws_frac, we_whole, we_frac, &fopts, NULL, 0,
        &written, &required);
    if (st != SIDEREON_STATUS_OK) {
        return fail("find_tca_candidates_from_tles sizing", 1);
    }
    if (required > 0) {
        SidereonTcaCandidate *cands = malloc(required * sizeof(SidereonTcaCandidate));
        if (!cands) {
            return fail("tca candidate alloc", 1);
        }
        if (sidereon_find_tca_candidates_from_tles(TLE_A1, TLE_A2, TLE_B1, TLE_B2, ws_whole,
                                                   ws_frac, we_whole, we_frac, &fopts, cands,
                                                   required, &written, &required) !=
                SIDEREON_STATUS_OK ||
            written != required || !(cands[0].miss_distance_km >= 0.0)) {
            free(cands);
            return fail("find_tca_candidates_from_tles", 1);
        }

        /* Pc on the first candidate with default covariances. */
        SidereonTcaPcOptions pcopts;
        memset(&pcopts, 0, sizeof(pcopts));
        pcopts.hard_body_radius_km = 0.02;
        pcopts.method = 0; /* Foster equal-area */
        pcopts.use_default_covariance = true;
        SidereonTcaConjunction conj;
        if (sidereon_tca_collision_probability(&cands[0], &pcopts, &conj) != SIDEREON_STATUS_OK ||
            !isfinite(conj.collision_probability.pc)) {
            free(cands);
            return fail("tca_collision_probability", 1);
        }
        free(cands);
    }

    /* Conjunction finder (candidates + Pc in one pass). */
    SidereonTcaPcOptions pcopts;
    memset(&pcopts, 0, sizeof(pcopts));
    pcopts.hard_body_radius_km = 0.02;
    pcopts.method = 0;
    pcopts.use_default_covariance = true;
    written = 0;
    required = 0;
    if (sidereon_find_tca_conjunctions_from_tles(TLE_A1, TLE_A2, TLE_B1, TLE_B2, ws_whole, ws_frac,
                                                 we_whole, we_frac, &fopts, &pcopts, NULL, 0,
                                                 &written, &required) != SIDEREON_STATUS_OK) {
        return fail("find_tca_conjunctions_from_tles sizing", 1);
    }

    /* Catalog screening (serial). */
    SidereonTcaTlePair secondaries[1];
    secondaries[0].line1 = TLE_B1;
    secondaries[0].line2 = TLE_B2;
    written = 0;
    required = 0;
    if (sidereon_screen_tca_candidates_from_tle_catalog(TLE_A1, TLE_A2, secondaries, 1, ws_whole,
                                                        ws_frac, we_whole, we_frac, 1000.0, &fopts,
                                                        NULL, 0, &written, &required) !=
        SIDEREON_STATUS_OK) {
        return fail("screen_tca_candidates_from_tle_catalog", 1);
    }
    if (sidereon_screen_tca_conjunctions_from_tle_catalog(TLE_A1, TLE_A2, secondaries, 1, ws_whole,
                                                          ws_frac, we_whole, we_frac, 1000.0,
                                                          &fopts, &pcopts, NULL, 0, &written,
                                                          &required) != SIDEREON_STATUS_OK) {
        return fail("screen_tca_conjunctions_from_tle_catalog", 1);
    }

    /* Propagated-covariance conjunction finder. */
    SidereonTcaPropagatedCovariancePcOptions popts;
    memset(&popts, 0, sizeof(popts));
    popts.hard_body_radius_km = 0.02;
    popts.method = 0;
    for (int i = 0; i < 6; i++) {
        popts.primary_covariance0[i][i] = 1.0e-2;
        popts.secondary_covariance0[i][i] = 1.0e-2;
    }
    popts.force_model = 1; /* two-body + J2 */
    popts.integrator = 0;  /* DP54 */
    popts.abs_tol = 1.0e-9;
    popts.rel_tol = 1.0e-9;
    popts.initial_step_s = 1.0;
    popts.min_step_s = 1.0e-3;
    popts.max_step_s = 60.0;
    popts.max_steps = 100000;
    popts.mu_km3_s2_enabled = false;
    popts.mu_km3_s2 = 0.0;
    written = 0;
    required = 0;
    SidereonStatus pst = sidereon_find_tca_conjunctions_with_propagated_covariance_from_tles(
        TLE_A1, TLE_A2, TLE_B1, TLE_B2, ws_whole, ws_frac, we_whole, we_frac, &fopts, &popts, NULL,
        0, &written, &required);
    if (pst == SIDEREON_STATUS_PANIC) {
        return fail("find_tca_conjunctions_with_propagated_covariance_from_tles", 1);
    }
    return 0;
}

int main(int argc, char **argv) {
    if (argc < 3) {
        fprintf(stderr, "usage: %s <grg_sp3> <nav>\n", argv[0]);
        return 2;
    }

    size_t sp3_len = 0;
    char *sp3_bytes = read_file(argv[1], &sp3_len);
    if (!sp3_bytes) {
        return fail("read sp3 file", 1);
    }
    SidereonSp3 *sp3 = NULL;
    SidereonStatus sp3_st =
        sidereon_sp3_load((const uint8_t *)sp3_bytes, sp3_len, &sp3);
    free(sp3_bytes);
    if (sp3_st != SIDEREON_STATUS_OK || sp3 == NULL) {
        return fail("sp3_load", 1);
    }

    int rc = 0;
    rc |= exercise_frames();
    rc |= exercise_nutation_precession();
    rc |= exercise_broadcast_keplerian();
    rc |= exercise_rinex_encode_nav(argv[2]);
    rc |= exercise_iod_gauss();
    rc |= exercise_combination_and_covariance();
    rc |= exercise_signal();
    rc |= exercise_quality(sp3);
    rc |= exercise_cycle_slips();
    rc |= exercise_ppp_corrections(sp3);
    rc |= exercise_tca();

    sidereon_sp3_free(sp3);
    if (rc != 0) {
        return 1;
    }
    printf("round2_smoke: OK\n");
    return 0;
}
