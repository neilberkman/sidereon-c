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

    cargo install cbindgen   # if not already installed
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

## Integrity

The direct post-solve integrity APIs are available without ephemeris handles or
solver coupling:

- Call `sidereon_raim` with satellite tokens, post-fit residuals, optional
  per-satellite inverse-variance weights, false-alarm probability, and optional
  GNSS clock-system count. The output is `SidereonRaimResult` with
  `fault_detected`, `test_statistic`, `threshold`, `worst_sat`, `dof`,
  `reduced_chi_square`, `rms_m`, and the normalized residual count. Use
  `sidereon_raim_normalized_residuals` to copy the per-satellite normalized
  residual rows with the standard `(NULL, 0)` size query contract.
- Call `sidereon_araim` with `SidereonAraimGeometry`, `SidereonAraimIsm`, and
  `SidereonAraimIntegrityAllocation`; read `hpl_m`, `vpl_m`,
  `sigma_acc_h_m`, and `sigma_acc_v_m` through
  `sidereon_araim_result_summary`, then release the result with
  `sidereon_araim_result_free`.

## Smoke test

`tests/run_smoke.sh` builds the library, regenerates the header, compiles
`tests/smoke.c`, and runs it on a committed crate-side SP3 fixture, asserting the
binding reproduces the engine reference position bit-exact:

    ./tests/run_smoke.sh
