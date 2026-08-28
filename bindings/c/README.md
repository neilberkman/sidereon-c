# sidereon (C)

A C-ABI binding over the Sidereon GNSS positioning engine. It is a thin interface
in the C idiom: opaque handles, integer status codes, and caller-allocated output
buffers. It adds no modeling of its own, so a solve returns exactly the numbers
the `sidereon-core` engine produces.

## Build

Build the shared library and generate the header. The crate produces both a
`cdylib` and a `staticlib` named `sidereon`.

    # from bindings/c:
    cargo build --release
    # -> <workspace target>/release/libsidereon.{dylib,so}  and  libsidereon.a
    # This crate is the workspace member, so the library lands in the workspace
    # root's target/ directory. `cargo metadata --format-version 1` reports the
    # exact target_directory; tests/run_smoke.sh resolves it automatically.

    cargo install --locked cbindgen --version 0.29.4   # if not already installed
    cbindgen --config cbindgen.toml --crate sidereon-c --output include/sidereon.h

A committed `include/sidereon.h` is already present; regenerate it only after
changing the C surface.

## Example

Parse an SP3 byte buffer, run a single-point positioning solve, and read the
position into a caller buffer. Every fallible call returns `SIDEREON_STATUS_OK` on
success; on any other value, `sidereon_last_error_message` gives the reason.

```c
#include <stdio.h>
#include "sidereon.h"

/* sp3_bytes / sp3_len: the contents of an SP3 file you have read into memory. */
SidereonSp3 *sp3 = NULL;
if (sidereon_sp3_load(sp3_bytes, sp3_len, &sp3) != SIDEREON_STATUS_OK) {
    char msg[256];
    sidereon_last_error_message(msg, sizeof(msg));
    fprintf(stderr, "load failed: %s\n", msg);
    return 1;
}

SidereonObservation obs[] = {
    { "G01", 21000123.4 },
    { "G08", 22517889.1 },
    /* ...more satellites... */
};

SidereonSppInputs in = {
    .observations = obs,
    .observation_count = sizeof(obs) / sizeof(obs[0]),
    .t_rx_j2000_s = /* receiver time, seconds past J2000 */ 0.0,
    .t_rx_second_of_day_s = /* second of day */ 0.0,
    .day_of_year = /* 1-based, fractional allowed */ 1.0,
    .initial_guess = { 0.0, 0.0, 0.0, 0.0 },  /* [x_m, y_m, z_m, clock_state] */
    .ionosphere = false,
    .troposphere = false,
    .with_geodetic = true,
};

SidereonSppSolution *sol = NULL;
if (sidereon_solve_spp(sp3, &in, &sol) != SIDEREON_STATUS_OK) {
    sidereon_sp3_free(sp3);
    return 1;
}

double xyz[3];
sidereon_spp_solution_position(sol, xyz, 3);
printf("position = [%.6f, %.6f, %.6f] m\n", xyz[0], xyz[1], xyz[2]);
double rx_clock_s = 0.0;
sidereon_spp_solution_rx_clock_s(sol, &rx_clock_s);
printf("rx_clock_s = %.9e\n", rx_clock_s);

sidereon_spp_solution_free(sol);
sidereon_sp3_free(sp3);
```

Compile and link against the header and shared library (`$LIBDIR` is the
workspace `target/release` directory reported by `cargo metadata`):

    cc -std=c11 -I include my_program.c \
        -L "$LIBDIR" -lsidereon \
        -Wl,-rpath,"$LIBDIR" -lm -o my_program

Reader functions copy into memory the caller owns: `sidereon_sp3_epoch_count`
and `sidereon_spp_solution_rx_clock_s` write scalars, `sidereon_spp_solution_position`
writes >= 3 doubles, `_residuals` supports `(NULL, 0)` size queries with
`out_required` and copies only when the buffer is large enough, and `_dop` writes
a `SidereonDop` of geometry scalars. Free every handle with its `_free`
function. See `include/sidereon.h` for the full surface and per-function safety
notes.

## Public parity routes

The generated header exposes the fixed-value and policy helpers directly:
`sidereon_covariance6_*` covers construction, validation, unit conversion, PSD
interpolation, and ECI/RTN transforms; `sidereon_second_of_day`,
`sidereon_day_of_year`, and `sidereon_data_day_of_year` cover the calendar
conventions; `sidereon_rinex_band_*`, `sidereon_rinex_observation_*`, and
`sidereon_default_iono_free_pair` apply the signal policy; and
`sidereon_lnav_tow`, `sidereon_lnav_subframe_id`,
`sidereon_lnav_parity`, and `sidereon_lnav_parity_valid` expose the LNAV bit
helpers.

RINEX NAV and broadcast routes include full records, representable GLONASS state
vectors and frequency channels, ionosphere and leap-second header values,
lenient skipped block diagnostics, raw record lists, and NAV encoding. The
standalone GLONASS parser retains representable records and separately exposes
the raw satellite tokens of skipped extended slots (such as `R28`) through its
handle's skipped-record count/item accessors; a successful parse therefore does
not imply that every input GLONASS record was representable. RINEX clock routes
parse strict or lossy input and expose satellite series; SBAS EMS and RTKLIB
text-log routes expose timestamped blocks and their payload bytes. RINEX RTK builders
provide single- and dual-frequency arcs with epoch metadata and observation,
position, wavelength, offset, and sort-key query/fill accessors. DTED tile-list
routes convert caller-supplied tiles to deterministic memory-mappable bytes or
write the same bytes to a path.

Handles returned by these routes are owned by the caller. Release
`SidereonBroadcastEphemeris` with `sidereon_broadcast_ephemeris_free`,
`SidereonRinexNavParse` with `sidereon_nav_parse_free`,
`SidereonRinexNavRecords` with `sidereon_rinex_nav_records_free`,
`SidereonRinexGlonassRecords` with `sidereon_rinex_glonass_records_free`,
`SidereonRinexClock` with `sidereon_rinex_clock_free`, `SidereonClockSeries`
with `sidereon_rinex_clock_series_free`, `SidereonSbasLogBlocks` with
`sidereon_sbas_log_blocks_free`, `SidereonRtkRinexArc` with
`sidereon_rtk_rinex_arc_free`, and `SidereonRtkRinexDualFrequencyArc` with
`sidereon_rtk_rinex_dual_frequency_arc_free`. Passing NULL to a free function
is a no-op. The handles and borrowed pointers must not be used after their
matching handle is freed.

Variable-length query/fill accessors use the shared caller-buffer convention:
call with a NULL buffer and length 0 to receive the required element count in
`out_required`, allocate caller-owned storage, and call again. A short buffer
returns `SIDEREON_STATUS_INVALID_ARGUMENT`, writes zero elements, and reports
the required count. The generated header documents the exact contract for each
route, including byte-oriented accessors.

For exact acquired products, construct an owned request with
`sidereon_sp3_exact_request_new` or
`sidereon_sp3_exact_request_from_identity`, then call
`sidereon_sp3_load_exact`. Success reports whether the regular epoch grid uses
the half-open or inclusive boundary representation. Header/identity, cadence,
span, structure, and grid mismatches fail without returning an SP3 handle. The
legacy `sidereon_sp3_load` remains the permissive general parser; use
`sidereon_sp3_declared_epoch_count` and
`sidereon_sp3_declared_start_j2000_seconds` to inspect its line-1 evidence.

The data catalog also exposes `sidereon_data_product_solution_class`,
`sidereon_data_default_sample_for_date`, `sidereon_data_supported_samples`, and
`sidereon_data_sp3_content_start_convention`. The content-start query returns a
typed convention plus the signed seconds added to the filename epoch, validates
ultra-rapid issues strictly, and is the same catalog fact inherited by exact
requests built from identities. The supported-samples query uses the standard
`(NULL, 0)` size query followed by a caller-allocated array of
`SidereonProductSample` records; its exact count and null-terminated tokens
report all cataloged cadences for the selected date and issue. Product
constructors enforce that same set. Historical IGS final CDDIS locations
report Unix `.Z` compression through the appended
`SIDEREON_ARCHIVE_COMPRESSION_UNIX_COMPRESS` value; prior enum values are
unchanged. Historical direct-BKG layout is not modeled and is rejected rather
than guessed. Catalog derivation fails before the evidenced ESA-final
SP3/clock, GFZ-rapid SP3/clock, IGS-ultra, CODE-ultra, ESA-ultra, and GFZ-ultra
starts. It preserves the historical GFZ-ultra cadence change and ESA-ultra
issue-level transition, and rejects unmodeled pre-week-2238 CDDIS long-name
SP3/IONEX locations while retaining the separately modeled legacy IGS final
`.sp3.Z` family. ESA `ESA0MGNFIN` final SP3 remains direct-only rather than
being substituted with another CDDIS product.

## Integrity

The direct post-solve integrity APIs are available without ephemeris handles or
solver coupling:

- Call `sidereon_raim` with satellite tokens, post-fit residuals, optional
  per-satellite inverse-variance weights, false-alarm probability, and optional
  GNSS clock-system count. The output is `SidereonRaimResult` with
  `fault_detected`, `test_statistic`, `threshold`, `worst_sat`, `dof`,
  `reduced_chi_square`, `rms_m`, and the normalized residual count. Use
  `sidereon_raim_normalized_residuals` to copy the per-satellite normalized
  residual rows with the standard `(NULL, 0)` size query contract. RAIM weights
  must come from per-satellite residual variances; unit weights on metre-scale
  residuals make `fault_detected` saturate near 100%.

  ```c
  SidereonFdeRaimWeight weights[6];
  for (size_t i = 0; i < 6; i++) {
      double sin_el = fmax(sin(elevation_rad[i]), 0.2);
      double variance_m2 = (0.8 / sin_el) * (0.8 / sin_el);
      weights[i] = (SidereonFdeRaimWeight){sat_ids[i], 1.0 / variance_m2};
  }
  ```
- Call `sidereon_araim` with `SidereonAraimGeometry`, `SidereonAraimIsm`, and
  `SidereonAraimIntegrityAllocation`; read `hpl_m`, `vpl_m`,
  `sigma_acc_h_m`, and `sigma_acc_v_m` through
  `sidereon_araim_result_summary`, then release the result with
  `sidereon_araim_result_free`.

## Smoke test

`tests/run_smoke.sh` builds the library, regenerates the header with cbindgen
0.29.4, compiles `tests/smoke.c`, and runs it on a committed crate-side SP3
fixture, asserting the binding reproduces the engine reference position
bit-exact:

    ./tests/run_smoke.sh

CI runs `tests/run_ci_smoke.sh` on Linux and macOS. It compares regenerated and
committed headers byte-for-byte, then compiles, links, and executes the focused
programs using only repository fixtures. In addition to the existing gates, the
script runs `fixed_policy_smoke` with no arguments,
`rinex_nav_clock_smoke` with `fixtures/nav/ESBC00DNK_R_20201770000_01D_MN.rnx`
and `fixtures/clk/synthetic_rinex_clock.clk`, and
`rinex_rtk_dted_smoke` with the committed SP3, WTZR/WTZZ observation fixtures,
and `fixtures/dted/tiles`.
