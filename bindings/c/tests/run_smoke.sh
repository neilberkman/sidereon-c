#!/usr/bin/env bash
# Build the sidereon cdylib + C header, compile the C smoke programs against them,
# and run them on committed and core-owned fixtures. Exits non-zero if any step
# or reference-agreement assertion fails.
#
# SIDEREON_CORE_FIXTURES may point to crates/sidereon-core/tests/fixtures from a
# local core checkout. When unset, this script resolves ../sidereon relative to
# the sidereon-c repository root at run time.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
binding_root="$(cd "${here}/.." && pwd)"
repo_root="$(cd "${binding_root}/../.." && pwd)"

cd "${binding_root}"
mkdir -p "${binding_root}/.tmp"
export TMPDIR="${SIDEREON_C_TMPDIR:-${binding_root}/.tmp}"

fixtures="${here}/fixtures"
provenance_fixture="${fixtures}/provenance/sp3-merge-input-v1.json"
provenance_fixture_sha256="6c8b8305ff09a884fb34f1e3f2c14c32735f1758a55e82019710874582d951de"
actual_provenance_fixture_sha256="$(shasum -a 256 "${provenance_fixture}" | awk '{print $1}')"
if [[ "${actual_provenance_fixture_sha256}" != "${provenance_fixture_sha256}" ]]; then
    echo "merged-SP3 provenance fixture digest mismatch" >&2
    exit 1
fi
sp3_path="${fixtures}/sp3/GRG0MGXFIN_20201760000_01D_15M_ORB.SP3"
sp3_surface_path="${fixtures}/sp3/IGS0OPSFIN_20261200945_02H30M_15M_ORB.SP3"
ppp_sp3_path="${fixtures}/sp3/GBM0MGXRAP_20201770000_01D_05M_ORB_120epoch.sp3"
spk_path="${fixtures}/spk/horizons_eros_type21.bsp"
antex_path="${fixtures}/antex/igs20_wettzell_trim.atx"
ionex_path="${fixtures}/ionex/synthetic_2map_7x7.20i"
esbc_crx_path="${fixtures}/obs/ESBC00DNK_R_20201770000_01D_30S_MO_trim.crx"
esbc_rnx_path="${fixtures}/obs/ESBC00DNK_R_20201770000_01D_30S_MO_trim.rnx"
algo_crx_path="${fixtures}/obs/algo0010_2015001_v1_trim.crx"
algo_rnx_path="${fixtures}/obs/algo0010_2015001_v1_trim.rnx"
# Broadcast/fallback fixtures: the ESBC mixed broadcast navigation, the COD MGEX
# final precise SP3 (covers the 2020 DOY177 epoch), and the prior-day (DOY176)
# SP3 the staleness layer degrades to. The 2026 "wrong epoch" SP3 reuses
# sp3_surface_path above.
nav_path="${fixtures}/nav/ESBC00DNK_R_20201770000_01D_MN.rnx"
precise_sp3_path="${fixtures}/sp3/COD0MGXFIN_20201770000_01D_05M_ORB.SP3"
prior_sp3_path="${fixtures}/sp3/GAP_G01_20201760000_15M.sp3"
# CCSDS navigation data messages: the canonical OEM/OPM KVN and XML fixtures,
# exercised by the reader/writer round-trip program below.
oem_kvn_path="${fixtures}/oem/gps.kvn"
oem_xml_path="${fixtures}/oem/gps.xml"
opm_kvn_path="${fixtures}/opm/osprey.kvn"
opm_xml_path="${fixtures}/opm/osprey.xml"
# Capability-parity additions: CDM (CCSDS conjunction data message) and RINEX
# clock fixtures from the canonical engine checkout.
cdm_kvn_path="${fixtures}/cdm/ccsds_example2.kvn"
cdm_xml_path="${fixtures}/cdm/ccsds_example2.xml"
rinex_clk_path="${fixtures}/clk/synthetic_rinex_clock.clk"
geodtest_one_path="${fixtures}/geodesic/geodtest_one.dat"
egm2008_crop_path="${fixtures}/geoid/egm2008_25_norcal_crop.bin"
tdm_annex_path="${fixtures}/tdm/annex_e_18.kvn"
# Universal-parity additions: a single-object OMM (KVN/XML/JSON serializers) from
# the canonical engine checkout.
omm_kvn_path="${fixtures}/omm/24876.kvn"
if [[ -n "${SIDEREON_CORE_FIXTURES:-}" ]]; then
    local_core_fixtures="$(cd "${SIDEREON_CORE_FIXTURES}" && pwd)"
else
    local_core_fixtures="$(cd "${repo_root}/../sidereon/crates/sidereon-core/tests/fixtures" && pwd)"
fi
observe_spk_path="${local_core_fixtures}/almanac/almanac_de421.spk"
dted_root_path="${local_core_fixtures}/dted/tiles"
dted_tile_path="${dted_root_path}/n36_w107_1arc_v3.dt2"
dcb_path="${local_core_fixtures}/bias/P1C1_RINEX.DCB"
bias_gz_path="${local_core_fixtures}/bias/COD0OPSFIN_20261330000_01D_01D_OSB.BIA.gz"
rtk_sp3_path="${local_core_fixtures}/sp3/GBM0MGXRAP_20201770000_01D_05M_ORB_120epoch.sp3"
wtzr_obs_path="${local_core_fixtures}/obs/WTZR00DEU_R_20201770000_01D_30S_MO_120epoch.rnx"
wtzz_obs_path="${local_core_fixtures}/obs/WTZZ00DEU_R_20201770000_01D_30S_MO_120epoch.rnx"

echo "== building cdylib =="
cargo build --release

# The compiled library lands in the workspace target directory, which is the
# workspace root's target/ (this crate is a workspace member), not necessarily
# under this directory. Resolve it from cargo metadata.
target_dir="$(cargo metadata --format-version 1 --quiet \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
lib_dir="${target_dir}/release"

echo "== regenerating header =="
cbindgen --quiet --config cbindgen.toml --crate sidereon-c --output include/sidereon.h

echo "== checking header docs =="
grep -Fq "sidereon_sp3_load, sidereon_sp3_load_exact, or sidereon_sp3_merge" include/sidereon.h
grep -Fq "sidereon_solve_spp_v2 and release with sidereon_spp_solution_free" include/sidereon.h
grep -Fq "sidereon_solve_spp or sidereon_solve_spp_v2 and must be freed exactly once" include/sidereon.h
grep -Fq "with sidereon_spk_load and release with sidereon_spk_free" include/sidereon.h
grep -Fq "sidereon_constellation_build and release with sidereon_constellation_free" include/sidereon.h

echo "== compiling smoke program =="
out="${target_dir}/smoke"
cc -std=c11 -Wall -Wextra -Werror \
    -I"${binding_root}/include" \
    -I"${here}" \
    "${here}/smoke.c" \
    -L"${lib_dir}" \
    -lsidereon \
    -Wl,-rpath,"${lib_dir}" \
    -lm \
    -o "${out}"

echo "== running smoke program =="
"${out}" "${sp3_path}" "${sp3_surface_path}" "${ppp_sp3_path}" "${spk_path}" "${antex_path}" \
    "${ionex_path}" "${esbc_crx_path}" "${esbc_rnx_path}" "${algo_crx_path}" "${algo_rnx_path}" \
    "${nav_path}" "${precise_sp3_path}" "${prior_sp3_path}"

echo "== compiling data_distribution_smoke program =="
data_distribution_out="${target_dir}/data_distribution_smoke"
cc -std=c11 -Wall -Wextra -Werror \
    -I"${binding_root}/include" \
    "${here}/data_distribution_smoke.c" \
    -L"${lib_dir}" \
    -lsidereon \
    -Wl,-rpath,"${lib_dir}" \
    -lm \
    -o "${data_distribution_out}"

echo "== running data_distribution_smoke program =="
"${data_distribution_out}"

echo "== compiling sp3_exact_smoke program =="
sp3_exact_out="${target_dir}/sp3_exact_smoke"
python3 "${here}/gen_sp3_terminal_record_fixture.py" \
    --output "${target_dir}/sp3_terminal_record_fixture.h"
cc -std=c11 -Wall -Wextra -Werror \
    -I"${binding_root}/include" \
    -I"${target_dir}" \
    "${here}/sp3_exact_smoke.c" \
    -L"${lib_dir}" \
    -lsidereon \
    -Wl,-rpath,"${lib_dir}" \
    -lm \
    -o "${sp3_exact_out}"

echo "== running sp3_exact_smoke program =="
"${sp3_exact_out}" "${sp3_path}" "${sp3_surface_path}"

# Focused exercise for the parity-gap closes (lenient OMM, standalone LAMBDA, and
# the SP3 merge agreement metric). Built and run with the same warnings-as-errors.
echo "== compiling newgaps program =="
newgaps_out="${target_dir}/newgaps"
cc -std=c11 -Wall -Wextra -Werror \
    -I"${binding_root}/include" \
    -I"${here}" \
    "${here}/newgaps.c" \
    -L"${lib_dir}" \
    -lsidereon \
    -Wl,-rpath,"${lib_dir}" \
    -lm \
    -o "${newgaps_out}"

echo "== running newgaps program =="
"${newgaps_out}" "${sp3_surface_path}"

# Round-trip exercise for the CCSDS OEM/OPM readers+writers and the RINEX/IONEX
# serializers. Built and run with the same warnings-as-errors.
echo "== compiling ccsds_serialize program =="
ccsds_out="${target_dir}/ccsds_serialize"
cc -std=c11 -Wall -Wextra -Werror \
    -I"${binding_root}/include" \
    -I"${here}" \
    "${here}/ccsds_serialize.c" \
    -L"${lib_dir}" \
    -lsidereon \
    -Wl,-rpath,"${lib_dir}" \
    -lm \
    -o "${ccsds_out}"

echo "== running ccsds_serialize program =="
"${ccsds_out}" "${oem_kvn_path}" "${oem_xml_path}" "${opm_kvn_path}" "${opm_xml_path}" \
    "${ionex_path}" "${esbc_rnx_path}"

# Satellite-constellation API: build a fleet from parsed TLE handles and exercise
# propagate / visible / look-angle arcs / ground tracks / passes. Uses only the
# committed TLE in prop_fixture.h, so it needs no runtime fixture paths.
echo "== compiling constellation_smoke program =="
constellation_out="${target_dir}/constellation_smoke"
cc -std=c11 -Wall -Wextra -Werror \
    -I"${binding_root}/include" \
    -I"${here}" \
    "${here}/constellation_smoke.c" \
    -L"${lib_dir}" \
    -lsidereon \
    -Wl,-rpath,"${lib_dir}" \
    -lm \
    -o "${constellation_out}"

echo "== running constellation_smoke program =="
"${constellation_out}"

# SPP robustness + integrity surface (Elixir-parity): FDE exclusion, robust
# Huber/IRLS solve, coarse-search cold start, and broadcast fallback. Uses the
# GRG SP3 (robust + coarse), the ESBC broadcast NAV (FDE + fallback), and the
# 2026 wrong-epoch SP3 (fallback). Built with the same warnings-as-errors.
echo "== compiling robustness_smoke program =="
robustness_out="${target_dir}/robustness_smoke"
cc -std=c11 -Wall -Wextra -Werror \
    -I"${binding_root}/include" \
    -I"${here}" \
    "${here}/robustness_smoke.c" \
    -L"${lib_dir}" \
    -lsidereon \
    -Wl,-rpath,"${lib_dir}" \
    -lm \
    -o "${robustness_out}"

echo "== running robustness_smoke program =="
"${robustness_out}" "${sp3_path}" "${nav_path}" "${sp3_surface_path}"

# Capability-parity surface (Elixir-parity): SP3-backed geometry (visible /
# visibility series / passes), SP3 + broadcast observables, broadcast velocity,
# reduced-orbit fit/eval/drift, and NRLMSISE-00 atmosphere density. Uses the GRG
# SP3 (geometry + observables + reduced orbit) and the ESBC broadcast NAV
# (broadcast observables + velocity). Built with the same warnings-as-errors.
echo "== compiling parity_smoke program =="
parity_out="${target_dir}/parity_smoke"
cc -std=c11 -Wall -Wextra -Werror \
    -I"${binding_root}/include" \
    -I"${here}" \
    "${here}/parity_smoke.c" \
    -L"${lib_dir}" \
    -lsidereon \
    -Wl,-rpath,"${lib_dir}" \
    -lm \
    -o "${parity_out}"

echo "== running parity_smoke program =="
"${parity_out}" "${sp3_path}" "${nav_path}"

# Capability-parity additions surface: RF link budget, frequencies/combinations,
# carrier-phase combinations + Hatch smoothing, GNSS signal scalars, weighting +
# RAIM, troposphere, Sun/Moon angles + eclipse + ephemeris, IOD, Lambert,
# conjunction, civil-time conversions, CDM, RINEX clock, broadcast orbit/clock
# evaluation, DGNSS corrections, and broadcast-vs-precise comparison. Built with
# the same warnings-as-errors.
echo "== compiling extras_smoke program =="
extras_out="${target_dir}/extras_smoke"
cc -std=c11 -Wall -Wextra -Werror \
    -I"${binding_root}/include" \
    -I"${here}" \
    "${here}/extras_smoke.c" \
    -L"${lib_dir}" \
    -lsidereon \
    -Wl,-rpath,"${lib_dir}" \
    -lm \
    -o "${extras_out}"

echo "== running extras_smoke program =="
"${extras_out}" "${sp3_path}" "${cdm_kvn_path}" "${cdm_xml_path}" "${rinex_clk_path}" \
    "${nav_path}" "${precise_sp3_path}"

# Round 2 capability-parity additions surface: high-accuracy frame transforms +
# TimeScales, nutation/precession, broadcast orbit/clock from Keplerian elements,
# RINEX NAV serialize, angles-only IOD, GNSS signal correlation/acquisition, the
# quality remainder (sigmas/weight_vector/raim_for_solution/validate), cycle-slip
# detection, ionosphere-free phase, encounter-plane covariance, the TCA family,
# and the PPP static-correction precompute. Uses the GRG SP3 (SPP solve + PPP
# build) and the ESBC broadcast NAV (RINEX NAV serialize). Built with the same
# warnings-as-errors.
echo "== compiling round2_smoke program =="
round2_out="${target_dir}/round2_smoke"
cc -std=c11 -Wall -Wextra -Werror \
    -I"${binding_root}/include" \
    -I"${here}" \
    "${here}/round2_smoke.c" \
    -L"${lib_dir}" \
    -lsidereon \
    -Wl,-rpath,"${lib_dir}" \
    -lm \
    -o "${round2_out}"

echo "== running round2_smoke program =="
"${round2_out}" "${sp3_path}" "${nav_path}"

# Capability-parity round: Galileo NeQuick-G ionosphere, rv<->COE element
# conversions, observation geometry, geoid undulation + heights, civil-instant
# construction, moving-baseline RTK, and RTCM 3 decode/encode (typed message
# accessors + framing). Fully self-contained (the RTCM stream is embedded), so it
# needs no runtime fixture paths. Built with the same warnings-as-errors.
echo "== compiling capround_smoke program =="
capround_out="${target_dir}/capround_smoke"
cc -std=c11 -Wall -Wextra -Werror \
    -I"${binding_root}/include" \
    -I"${here}" \
    "${here}/capround_smoke.c" \
    -L"${lib_dir}" \
    -lsidereon \
    -Wl,-rpath,"${lib_dir}" \
    -lm \
    -o "${capround_out}"

echo "== running capround_smoke program =="
"${capround_out}"

# Newly merged core features: full NeQuick-G slant integration, the standalone
# range RAIM/FDE design, the sequential RTK baseline arc driver, the SPP-seeded
# PPP auto-initialization drivers, and RTCM 3 from-scratch message construction
# (construct -> encode -> decode round-trips). Uses the PPP SP3 fixture for the
# auto-init drivers; the rest is self-contained. Built with the same
# warnings-as-errors.
echo "== compiling merged_smoke program =="
merged_out="${target_dir}/merged_smoke"
cc -std=c11 -Wall -Wextra -Werror \
    -I"${binding_root}/include" \
    -I"${here}" \
    "${here}/merged_smoke.c" \
    -L"${lib_dir}" \
    -lsidereon \
    -Wl,-rpath,"${lib_dir}" \
    -lm \
    -o "${merged_out}"

echo "== running merged_smoke program =="
"${merged_out}" "${ppp_sp3_path}"

# Universal-parity additions: batch SPP (serial + parallel), GPS LNAV encode/decode,
# OMM serializers (KVN/XML/JSON), and CRINEX encode. Uses the GRG SP3 (batch SPP),
# a single-object OMM, and the ESBC RINEX observation file (CRINEX encode). Built
# with the same warnings-as-errors.
echo "== compiling parity_gaps_smoke program =="
parity_gaps_out="${target_dir}/parity_gaps_smoke"
cc -std=c11 -Wall -Wextra -Werror \
    -I"${binding_root}/include" \
    -I"${here}" \
    "${here}/parity_gaps_smoke.c" \
    -L"${lib_dir}" \
    -lsidereon \
    -Wl,-rpath,"${lib_dir}" \
    -lm \
    -o "${parity_gaps_out}"

echo "== running parity_gaps_smoke program =="
"${parity_gaps_out}" "${sp3_path}" "${omm_kvn_path}" "${esbc_rnx_path}"

# Core-backed C additions: DGNSS position solve, ANTEX encode, fuller RINEX OBS
# helpers, SP3 clock-reference offset/align, and source-backed reduced-orbit
# fit/drift.
echo "== compiling core_caps_smoke program =="
core_caps_out="${target_dir}/core_caps_smoke"
cc -std=c11 -Wall -Wextra -Werror \
    -I"${binding_root}/include" \
    -I"${here}" \
    "${here}/core_caps_smoke.c" \
    -L"${lib_dir}" \
    -lsidereon \
    -Wl,-rpath,"${lib_dir}" \
    -lm \
    -o "${core_caps_out}"

echo "== running core_caps_smoke program =="
"${core_caps_out}" "${sp3_path}" "${antex_path}" "${esbc_rnx_path}"

# Full public-API coverage additions: force accelerations, Doppler, covariance,
# time metadata and GNSS week helpers, coverage grids, constellation diff and
# strict validation helpers, and source-backed piecewise reduced-orbit fit/drift.
echo "== compiling full_coverage_smoke program =="
full_coverage_out="${target_dir}/full_coverage_smoke"
cc -std=c11 -Wall -Wextra -Werror \
    -I"${binding_root}/include" \
    -I"${here}" \
    "${here}/full_coverage_smoke.c" \
    -L"${lib_dir}" \
    -lsidereon \
    -Wl,-rpath,"${lib_dir}" \
    -lm \
    -o "${full_coverage_out}"

echo "== running full_coverage_smoke program =="
"${full_coverage_out}" "${sp3_path}"

# Newer merged-core capabilities: generic data-driven trust-region least squares
# (solve + leave-one-out), Jacobian-derived covariance / Hessian trace / error
# ellipse, DOP with an explicit ENU convention, residual-distribution statistics,
# batch observable prediction, leap-second accessors, the embedded EGM96 geoid,
# and ground-observer Sun/Moon geometry. The trust-region and statistics
# bit-exact-vs-scipy checks are linux-x86_64-pinned inside the program; the rest
# is closed-form and runs everywhere. Uses the GRG SP3 (SP3 batch observables)
# and the ESBC broadcast NAV (broadcast batch observables).
echo "== compiling caps_extra_smoke program =="
caps_extra_out="${target_dir}/caps_extra_smoke"
cc -std=c11 -Wall -Wextra -Werror \
    -I"${binding_root}/include" \
    -I"${here}" \
    "${here}/caps_extra_smoke.c" \
    -L"${lib_dir}" \
    -lsidereon \
    -Wl,-rpath,"${lib_dir}" \
    -lm \
    -o "${caps_extra_out}"

echo "== running caps_extra_smoke program =="
"${caps_extra_out}" "${sp3_path}" "${nav_path}"

# Sample-backed precise-ephemeris source + batch range prediction: extract an
# SP3 product's canonical samples, rebuild an interpolatable source from them,
# and assert the batch range predictor and interpolated states agree with the
# SP3-parsed source (round-trip tolerance), the one-call batch equals per-request
# calls, and the validation-error paths report InvalidArgument. Uses the GRG SP3.
echo "== compiling precise_samples_smoke program =="
precise_samples_out="${target_dir}/precise_samples_smoke"
cc -std=c11 -Wall -Wextra -Werror \
    -I"${binding_root}/include" \
    -I"${here}" \
    "${here}/precise_samples_smoke.c" \
    -L"${lib_dir}" \
    -lsidereon \
    -Wl,-rpath,"${lib_dir}" \
    -lm \
    -o "${precise_samples_out}"

echo "== running precise_samples_smoke program =="
"${precise_samples_out}" "${sp3_path}"

# 0.13 capabilities: batched observable states and cached precise interpolants,
# estimation/detection primitives, and source localization. Uses the GRG SP3 for
# observable-state parity; source-localization inputs are synthetic in the test.
echo "== compiling cap013_smoke program =="
cap013_out="${target_dir}/cap013_smoke"
cc -std=c11 -Wall -Wextra -Werror \
    -I"${binding_root}/include" \
    -I"${here}" \
    "${here}/cap013_smoke.c" \
    -L"${lib_dir}" \
    -lsidereon \
    -Wl,-rpath,"${lib_dir}" \
    -lm \
    -o "${cap013_out}"

echo "== running cap013_smoke program =="
"${cap013_out}" "${sp3_path}"

# Track filter and RTS smoother surface, plus tide force-model switches.
echo "== compiling track_smoke program =="
track_out="${target_dir}/track_smoke"
cc -std=c11 -Wall -Wextra -Werror \
    -I"${binding_root}/include" \
    -I"${here}" \
    "${here}/track_smoke.c" \
    -L"${lib_dir}" \
    -lsidereon \
    -Wl,-rpath,"${lib_dir}" \
    -lm \
    -o "${track_out}"

echo "== running track_smoke program =="
"${track_out}"

# 0.15 capabilities: composite perturbation selection, sidereal filtering,
# geodetic time series, position-error metrics, clock-noise identification, and
# orbit-fit residual ledgers. Self-contained.
echo "== compiling cap015_smoke program =="
cap015_out="${target_dir}/cap015_smoke"
cc -std=c11 -Wall -Wextra -Werror \
    -I"${binding_root}/include" \
    -I"${here}" \
    "${here}/cap015_smoke.c" \
    -L"${lib_dir}" \
    -lsidereon \
    -Wl,-rpath,"${lib_dir}" \
    -lm \
    -o "${cap015_out}"

echo "== running cap015_smoke program =="
"${cap015_out}"

# 0.18 domain exposure: GNSS/INS fusion handle, deterministic scenario
# simulator, and signal-analysis closed forms. Self-contained.
echo "== compiling domain018_smoke program =="
domain018_out="${target_dir}/domain018_smoke"
cc -std=c11 -Wall -Wextra -Werror \
    -I"${binding_root}/include" \
    -I"${here}" \
    "${here}/domain018_smoke.c" \
    -L"${lib_dir}" \
    -lsidereon \
    -Wl,-rpath,"${lib_dir}" \
    -lm \
    -o "${domain018_out}"

echo "== running domain018_smoke program =="
"${domain018_out}"

# Wave-2 local-core additions: geodesics, terrestrial frame catalog, EGM2008,
# spherical-harmonic propagation selection, CCSDS TDM, ECEF SP3 orbit fit, SGP4
# decay latch, typed low-elevation troposphere error, oblate eclipse, and
# reliability noncentrality component marshaling.
echo "== compiling wave2_smoke program =="
wave2_out="${target_dir}/wave2_smoke"
cc -std=c11 -Wall -Wextra -Werror \
    -I"${binding_root}/include" \
    -I"${here}" \
    "${here}/wave2_smoke.c" \
    -L"${lib_dir}" \
    -lsidereon \
    -Wl,-rpath,"${lib_dir}" \
    -lm \
    -o "${wave2_out}"

echo "== running wave2_smoke program =="
"${wave2_out}" "${geodtest_one_path}" "${egm2008_crop_path}" "${tdm_annex_path}" \
    "${prior_sp3_path}"

# 0.12 core capabilities: Allan-family clock stability, terrain batch lookup,
# IONEX sample construction/extraction, SBAS decoded payload accessors, ARAIM,
# and coordinate angular separation / position angle. Uses the IONEX binding
# fixture and local core DTED tiles.
echo "== compiling core012_smoke program =="
core012_out="${target_dir}/core012_smoke"
cc -std=c11 -Wall -Wextra -Werror \
    -I"${binding_root}/include" \
    -I"${here}" \
    "${here}/core012_smoke.c" \
    -L"${lib_dir}" \
    -lsidereon \
    -Wl,-rpath,"${lib_dir}" \
    -lm \
    -o "${core012_out}"

echo "== running core012_smoke program =="
"${core012_out}" "${ionex_path}" "${dted_root_path}"

# Round-2 local-core parity sweep: covariance propagation/transport,
# CNAV/RINEX-4 accessors, SGP4 TLE fitting, RINEX QC/lint/repair, EGM96/geoid
# batches, NMEA sans-IO, space-weather tables, and NTRIP sans-IO.
echo "== compiling round2_parity_smoke program =="
round2_parity_out="${target_dir}/round2_parity_smoke"
cc -std=c11 -Wall -Wextra -Werror \
    -I"${binding_root}/include" \
    -I"${here}" \
    "${here}/round2_parity_smoke.c" \
    -L"${lib_dir}" \
    -lsidereon \
    -Wl,-rpath,"${lib_dir}" \
    -lm \
    -o "${round2_parity_out}"

echo "== running round2_parity_smoke program =="
"${round2_parity_out}" "${local_core_fixtures}"

# Phase B local-core additions: new ASTRO, terrain, drag, sample-grid, bias,
# SBAS, SSR, and shared-label API.
echo "== compiling phaseb_smoke program =="
phaseb_out="${target_dir}/phaseb_smoke"
cc -std=c11 -Wall -Wextra -Werror \
    -I"${binding_root}/include" \
    -I"${here}" \
    "${here}/phaseb_smoke.c" \
    -L"${lib_dir}" \
    -lsidereon \
    -Wl,-rpath,"${lib_dir}" \
    -lm \
    -o "${phaseb_out}"

echo "== running phaseb_smoke program =="
"${phaseb_out}" "${sp3_path}" "${observe_spk_path}" "${dted_root_path}" "${dted_tile_path}" \
    "${dcb_path}" "${bias_gz_path}"

# Real-data raw RINEX to RTK baseline: paired WTZR/WTZZ OBS plus SP3 through the
# C convenience functions.
echo "== compiling rtk_rinex_smoke program =="
rtk_rinex_out="${target_dir}/rtk_rinex_smoke"
cc -std=c11 -Wall -Wextra -Werror \
    -I"${binding_root}/include" \
    -I"${here}" \
    "${here}/rtk_rinex_smoke.c" \
    -L"${lib_dir}" \
    -lsidereon \
    -Wl,-rpath,"${lib_dir}" \
    -lm \
    -o "${rtk_rinex_out}"

echo "== running rtk_rinex_smoke program =="
"${rtk_rinex_out}" "${rtk_sp3_path}" "${wtzr_obs_path}" "${wtzz_obs_path}"
