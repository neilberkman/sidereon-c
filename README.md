# sidereon

GNSS and astrodynamics for C: propagate satellites, predict passes, solve
precise positions (SPP / RTK / PPP), and work with coordinate frames and time —
checked against the references the field trusts (Vallado, Skyfield, IGS, IERS).

Under the hood it's a Rust engine compiled to a single self-contained static
library plus one generated header. There's no runtime to install and nothing to
link but `libsidereon` and `libm`: a solve returns exactly the numbers the
engine computes, in the C idiom you already use — opaque handles, integer status
codes, and caller-owned output buffers.

## Build

`cargo build --release` builds the library; the C header is generated from the
Rust source with cbindgen and is committed alongside it.

```
cargo build --release
# -> target/release/libsidereon.a      (static archive)
#    target/release/libsidereon.dylib  (or .so on Linux)
# header: bindings/c/include/sidereon.h
```

The header is already in the tree. Regenerate it only after changing the C
surface:

```
cargo install cbindgen   # once
cbindgen --config bindings/c/cbindgen.toml --crate sidereon-c \
    --output bindings/c/include/sidereon.h
```

Compile a C program against the header and the library:

```
cc -std=c11 -I bindings/c/include my_program.c \
    -L target/release -lsidereon -Wl,-rpath,target/release -lm \
    -o my_program
```

Link `target/release/libsidereon.a` instead of `-lsidereon` if you want the
solver baked into your binary with no shared object to ship.

## Quickstart: when does the ISS fly over you?

No data files, no setup — give it a two-line element set and a ground station,
and ask when the satellite is above the horizon. Every fallible call returns
`SIDEREON_STATUS_OK`; on anything else, `sidereon_last_error_message` gives the
reason. C owns nothing the engine allocates until it frees it, so each handle
gets a matching `_free`.

```c
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <time.h>
#include "sidereon.h"

int main(void) {
    /* Real orbital elements (grab fresh ones from CelesTrak any time). */
    const char *line1 =
        "1 25544U 98067A   26178.50947090  .00006280  00000+0  12016-3 0  9996";
    const char *line2 =
        "2 25544  51.6322 248.9966 0004278 238.4942 121.5629 15.49454046573359";

    SidereonTle *iss = NULL;
    if (sidereon_tle_load(line1, line2, SIDEREON_TLE_OPS_MODE_IMPROVED, &iss)
        != SIDEREON_STATUS_OK) {
        char err[256];
        sidereon_last_error_message(err, sizeof err);
        fprintf(stderr, "tle load failed: %s\n", err);
        return 1;
    }

    /* A ground station: latitude/longitude in degrees, altitude in metres. */
    SidereonGroundStation berkeley = {
        .latitude_deg  = 37.87,
        .longitude_deg = -122.27,
        .altitude_m    = 52.0,
    };

    /* Epochs are UTC unix microseconds. Look one day ahead. */
    int64_t now_us = (int64_t)time(NULL) * 1000000;
    int64_t end_us = now_us + (int64_t)24 * 3600 * 1000000;

    /* Start from engine defaults, then raise the elevation mask to 10 degrees. */
    SidereonPassFinderOptions opts;
    sidereon_pass_finder_options_init(&opts);
    opts.elevation_mask_deg = 10.0;

    SidereonPassList *passes = NULL;
    if (sidereon_tle_find_passes(iss, &berkeley, now_us, end_us, &opts, &passes)
        != SIDEREON_STATUS_OK) {
        char err[256];
        sidereon_last_error_message(err, sizeof err);
        fprintf(stderr, "find_passes failed: %s\n", err);
        sidereon_tle_free(iss);
        return 1;
    }

    /* Ask how many passes there are, then copy them into your own buffer. */
    size_t count = 0;
    sidereon_pass_list_count(passes, &count);

    SidereonSatellitePass *rows = count ? malloc(count * sizeof *rows) : NULL;
    size_t written = 0, required = 0;
    sidereon_pass_list_values(passes, rows, count, &written, &required);

    printf("%zu ISS pass(es) over Berkeley in the next 24 h:\n", written);
    for (size_t i = 0; i < written; i++) {
        time_t aos = (time_t)(rows[i].aos_unix_us / 1000000);  /* unix micros */
        struct tm tm;
        gmtime_r(&aos, &tm);
        printf("  %02d:%02d UTC  %4.1f min  peak %2.0f deg\n",
               tm.tm_hour, tm.tm_min,
               rows[i].duration_s / 60.0, rows[i].max_elevation_deg);
    }

    free(rows);
    sidereon_pass_list_free(passes);
    sidereon_tle_free(iss);
    return 0;
}
```

```
5 ISS pass(es) over Berkeley in the next 24 h:
  08:30 UTC   6.8 min  peak 88 deg
  10:09 UTC   4.3 min  peak 16 deg
  ...
```

A `SidereonTle` also gives you `sidereon_tle_propagate` (TEME state arcs) and
`sidereon_tle_look_angles` (azimuth/elevation/range over a time grid). Anything
that takes time takes UTC unix microseconds, and every variable-length result
follows the same contract: call with a NULL buffer and length 0 to learn the
required count, then call again with storage you own.

## Precise positioning

The positioning engine is the other half of the library: feed it pseudoranges
and a precise-ephemeris product and it returns a least-squares fix.

```c
#include "sidereon.h"

/* sp3_bytes / sp3_len: an SP3 file you have read into memory. */
SidereonSp3 *sp3 = NULL;
sidereon_sp3_load(sp3_bytes, sp3_len, &sp3);

SidereonObservation obs[] = {
    { "G01", 21000123.4 },   /* PRN, pseudorange (m) */
    { "G08", 22517889.1 },
    /* ...more satellites... */
};

SidereonSppInputs in = {
    .observations         = obs,
    .observation_count    = sizeof obs / sizeof obs[0],
    .t_rx_j2000_s         = /* receiver time, seconds past J2000 */ 0.0,
    .t_rx_second_of_day_s = /* second of day */ 0.0,
    .day_of_year          = /* 1-based, fractional allowed */ 1.0,
    .initial_guess        = { 0.0, 0.0, 0.0, 0.0 },  /* x_m, y_m, z_m, clock */
    .ionosphere           = true,
    .troposphere          = true,
    .with_geodetic        = true,
};

SidereonSppSolution *fix = NULL;
sidereon_solve_spp(sp3, &in, &fix);

double xyz[3];
sidereon_spp_solution_position(fix, xyz, 3);          /* ECEF metres */

SidereonGeodetic geo;
bool have_geo = false;
sidereon_spp_solution_geodetic(fix, &geo, &have_geo); /* lat/lon rad, height m */

sidereon_spp_solution_free(fix);
sidereon_sp3_free(sp3);
```

`sidereon_solve_rtk_float` / `_fixed` and `sidereon_solve_ppp_float` / `_fixed`
follow the same shape: a typed config in, an opaque solution out, reader
functions that copy scalars and positions into memory you own.

## What's in the box

C exposes the engine's GNSS-heavy surface:

- **Positioning** — SPP, RTK (float/fixed), PPP (float/fixed), DOP, velocity
- **Orbits** — TLE/SGP4 propagation, passes, look angles, numerical state propagation
- **GNSS data** — SP3, RINEX (obs/nav), CRINEX, ANTEX, IONEX, broadcast ephemeris with fallback selection
- **Ephemeris** — JPL SPK (DAF/.bsp) sampling

Every result is exactly what the engine computes; the binding adds no modeling
of its own. The full surface — every struct, status code, and per-function
ownership note — lives in `bindings/c/include/sidereon.h`.

The astrodynamics convenience layer (standalone frame/time conversions,
Sun/Moon positions, conjunction screening) is not yet wrapped for C; reach for
the Rust or Python interface if you need it today.

## Other languages

sidereon is one validated engine with first-class interfaces in **Rust**,
**Python**, **C**, **Elixir**, and **WebAssembly** — same numbers everywhere.
See the live demo and docs at [sidereon.dev](https://sidereon.dev).

## How it's validated

The SGP4 propagator is a Rust port of David Vallado's reference implementation,
bit-exact to it. The positioning stack is checked against IGS products.
`bindings/c/tests/run_smoke.sh` builds the library, compiles a C program against
the generated header, and asserts the binding reproduces the engine's reference
position bit-exact.
