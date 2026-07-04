# sidereon (C)

GNSS and astrodynamics for C. Propagate satellites, predict passes, solve
precise positions (SPP / RTK / PPP), and work with coordinate frames, time, and
the orbital-data formats the field actually exchanges.

This repository is the C ABI interface to sidereon. The engine is a Rust core,
compiled here into one self-contained library plus one generated header. There
is no runtime to install and nothing to link but `libsidereon` and `libm`. The
binding adds no modeling of its own: a solve returns exactly the numbers the
engine computes, in the C idiom you already use (opaque handles, integer status
codes, caller-owned output buffers). The numbers are reference-validated: the
SGP4 propagator is a port of David Vallado's reference implementation, bit-exact
to it, and the positioning stack is checked against IGS products.

## Build

`cargo build --release` builds the library. The C header is generated from the
Rust source with cbindgen and is committed alongside it, so you do not need
cbindgen to consume the binding.

```sh
cargo build --release
# -> target/release/libsidereon.a      (static archive)
#    target/release/libsidereon.dylib  (or .so on Linux)
# header: bindings/c/include/sidereon.h (already committed)
```

This crate is a workspace member, so the library lands in the workspace root's
`target/release`. If your layout differs, `cargo metadata --format-version 1`
reports the exact `target_directory`.

Regenerate the header only after changing the C surface:

```sh
cargo install cbindgen   # once
cbindgen --config bindings/c/cbindgen.toml --crate sidereon-c \
    --output bindings/c/include/sidereon.h
```

Compile and link a C program against the header and the library:

```sh
cc -std=c11 -I bindings/c/include my_program.c \
    -L target/release -lsidereon -Wl,-rpath,target/release -lm \
    -o my_program
```

Link `target/release/libsidereon.a` directly instead of `-lsidereon` if you want
the solver baked into your binary with no shared object to ship.

## Example: a single-point positioning solve

A complete program with no external data files. It loads a trimmed SP3 precise
orbit product from an in-memory buffer, feeds in six GPS L1 pseudoranges, and
prints the receiver position in ECEF metres. Every fallible call returns
`SIDEREON_STATUS_OK`; on anything else, `sidereon_last_error_message` gives the
reason. Each handle gets a matching `_free`.

```c
#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include "sidereon.h"

/* Precise orbits (SP3-c), trimmed to the satellites and epoch below. */
static const char SP3[] =
    "#cP2020  6 24  9 45  0.00000000      19 TRACK IGb14 FIT GRGS\n"
    "## 2111 259200.00000000   900.00000000 59024 0.0000000000000\n"
    "+    6   G08G10G16G18G20G21  0  0  0  0  0  0  0  0  0  0  0\n"
    "+          0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0\n"
    "+          0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0\n"
    "+          0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0\n"
    "+          0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0\n"
    "++         4  4  4  4  4  4  0  0  0  0  0  0  0  0  0  0  0\n"
    "++         0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0\n"
    "++         0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0\n"
    "++         0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0\n"
    "++         0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0\n"
    "%c M  cc GPS ccc cccc cccc cccc cccc ccccc ccccc ccccc ccccc\n"
    "%c cc cc ccc ccc cccc cccc cccc cccc ccccc ccccc ccccc ccccc\n"
    "%f  0.0000000  0.000000000  0.00000000000  0.000000000000000\n"
    "%f  0.0000000  0.000000000  0.00000000000  0.000000000000000\n"
    "%i    0    0    0    0      0      0      0      0         0\n"
    "%i    0    0    0    0      0      0      0      0         0\n"
    "/* CNES/CLS/GRGS - TOULOUSE,FRANCE - Contact : igs-ac@cls.fr\n"
    "/* PCV:IGS14_2108 OL/AL:FES2012  NONE     NN ORB:CoN CLK:CoN\n"
    "/* CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC\n"
    "/* CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC\n"
    "*  2020  6 24  9 45  0.00000000\n"
    "PG08   4643.904912 -24162.558516  -9650.049610    -38.635183\n"
    "PG10  17886.671789   3039.006353 -19391.393793   -380.478330\n"
    "PG16   3673.608258 -18933.969815  17839.814705   -174.352223\n"
    "PG18  23700.127530   6507.753313  10093.688366    228.808891\n"
    "PG20  21015.602926  12381.822675 -10856.524893    527.448122\n"
    "PG21  26299.269767  -2460.995007    716.844563     15.508264\n"
    "*  2020  6 24 10  0  0.00000000\n"
    "PG08   5254.320976 -24937.345008  -6980.386897    -38.636691\n"
    "PG10  19010.269319   4866.851618 -17921.084337   -380.488198\n"
    "PG16   4860.112779 -17100.303698  19364.987730   -174.356395\n"
    "PG18  22418.061092   6785.447511  12538.206018    228.818132\n"
    "PG20  21540.808052  13373.630254  -8364.466576    527.447968\n"
    "PG21  26177.802655  -2146.941378   3560.974533     15.512752\n"
    "*  2020  6 24 10 15  0.00000000\n"
    "PG08   5710.564170 -25453.105280  -4188.721104    -38.638443\n"
    "PG10  20115.189466   6485.476927 -16142.894217   -380.498119\n"
    "PG16   6226.321564 -15204.115225  20546.504507   -174.360500\n"
    "PG18  20889.343380   7160.242478  14767.111628    228.827365\n"
    "PG20  21908.611100  14137.388789  -5730.669927    527.447926\n"
    "PG21  25760.514626  -1769.431822   6343.514786     15.516981\n"
    "*  2020  6 24 10 30  0.00000000\n"
    "PG08   6042.284075 -25684.525952  -1323.814227    -38.640412\n"
    "PG10  21159.408012   7879.460815 -14087.964262   -380.507982\n"
    "PG16   7759.098529 -13294.312785  21364.232984   -174.364502\n"
    "PG18  19148.071571   7657.452624  16742.023312    228.836711\n"
    "PG20  22085.133601  14686.270178  -2999.737823    527.447449\n"
    "PG21  25067.475736  -1296.107146   9017.301047     15.521531\n"
    "*  2020  6 24 10 45  0.00000000\n"
    "PG08   6283.596097 -25614.361768   1564.242656    -38.641756\n"
    "PG10  22100.268611   9041.937175 -11792.020438   -380.517890\n"
    "PG16   9436.967444 -11417.672716  21804.746560   -174.368582\n"
    "PG18  17234.002467   8295.952772  18428.910848    228.845896\n"
    "PG20  22041.002117  15040.256156   -217.934016    527.447614\n"
    "PG21  24126.033756   -698.817607  11537.888708     15.525474\n"
    "*  2020  6 24 11  0  0.00000000\n"
    "PG08   6471.529324 -25234.155183   4424.947554    -38.642177\n"
    "PG10  22896.131923   9974.623387  -9294.735744   -380.527755\n"
    "PG16  11230.846890  -9617.105625  21861.506015   -174.372582\n"
    "PG18  15191.136796   9087.282030  19798.684014    228.855179\n"
    "PG20  21752.562074  15225.214120   2567.564373    527.447459\n"
    "PG21  22969.651026     45.186775  13864.197409     15.529722\n"
    "*  2020  6 24 11 15  0.00000000\n"
    "PG08   6644.360327 -24544.616731   7208.313715    -38.643219\n"
    "PG10  23507.988002  10687.523653  -6639.037631   -380.537732\n"
    "PG16  13105.095965  -7930.108701  21534.895433   -174.377072\n"
    "PG18  13066.149231  10035.038521  20827.696867    228.864441\n"
    "PG20  21202.891348  15271.756854   5309.432624    527.447520\n"
    "PG21  21636.563260    952.305394  15959.053118     15.534059\n"
    "*  2020  6 24 11 30  0.00000000\n"
    "PG08   6839.902243 -23555.650675   9865.783600    -38.644720\n"
    "PG10  23900.971736  11198.322998  -3870.373323   -380.547626\n"
    "PG16  15018.822256  -6387.464812  20832.117593   -174.381097\n"
    "PG18  10906.724082  11134.589308  21498.159124    228.873724\n"
    "PG20  20382.574992  15213.927788   7960.972253    527.447536\n"
    "PG21  20168.317061   2031.899012  17789.631146     15.538108\n"
    "*  2020  6 24 11 45  0.00000000\n"
    "PG08   7093.820244 -22286.025531  12351.111011    -38.645281\n"
    "PG10  24045.725638  11531.498375  -1035.945281   -380.557547\n"
    "PG16  16927.393599  -5012.236439  19766.958053   -174.385107\n"
    "PG18   8759.860313  12373.107746  21798.447374    228.883076\n"
    "PG20  19290.210666  15087.759974  10476.882192    527.447233\n"
    "PG21  18608.242309   3285.876753  19327.806335     15.542220\n"
    "*  2020  6 24 12  0  0.00000000\n"
    "PG08   7438.042916 -20762.704119  14621.192800    -38.647002\n"
    "PG10  23919.560682  11717.183156   1816.071269   -380.567404\n"
    "PG16  18784.088401  -3819.088286  18359.430195   -174.389524\n"
    "PG18   6670.210753  13729.937789  21723.310519    228.892428\n"
    "PG20  17932.623680  14929.762015  12814.016152    527.447354\n"
    "PG21  16999.913180   4708.560403  20550.418405     15.546299\n"
    "*  2020  6 24 12 15  0.00000000\n"
    "PG08   7899.334503 -19019.862023  16636.833344    -38.647763\n"
    "PG10  23507.373783  11789.829543   4637.317275   -380.577286\n"
    "PG16  20541.815806  -2813.957336  16635.315598   -174.393761\n"
    "PG18   4678.519090  15177.272097  21273.965621    228.901739\n"
    "PG20  16324.780780  14775.387488  14932.114622    527.446948\n"
    "PG21  15385.647811   6286.819595  21439.461249     15.550671\n"
    "*  2020  6 24 12 30  0.00000000\n"
    "PG08   8498.085463 -17097.635989  18363.427105    -38.649562\n"
    "PG10  22802.289648  11786.720063   7380.034276   -380.587160\n"
    "PG16  22154.836702  -1994.074300  14625.615492   -174.397842\n"
    "PG18   2820.214554  16681.118608  20458.082166    228.911022\n"
    "PG20  14489.402694  14657.545765  16794.499670    527.447051\n"
    "PG21  13805.092069   8000.463976  21982.204956     15.554459\n"
    "*  2020  6 24 12 45  0.00000000\n"
    "PG08   9247.369015 -15040.654467  19771.547156    -38.650484\n"
    "PG10  21806.004803  11746.384064   9997.833858   -380.596890\n"
    "PG16  23580.419142  -1348.326156  12365.929519   -174.402246\n"
    "PG18   1124.218120  18202.518914  19289.654667    228.920337\n"
    "PG20  12456.286563  14605.211454  18368.720568    527.447006\n"
    "PG21  12293.927824   9822.872282  22171.258698     15.558791\n"
    "*  2020  6 24 13  0  0.00000000\n"
    "PG08  10152.299669 -12896.410588  20837.430270    -38.651742\n"
    "PG10  20528.822194  11706.977946  12446.442268   -380.606741\n"
    "PG16  24780.366877   -857.936737   9895.777935   -174.406704\n"
    "PG18   -387.994051  19698.971935  17788.765433    228.929616\n"
    "PG20  10261.360393  14642.186916  19627.138333    527.447150\n"
    "PG21  10882.740798  11721.833118  22004.581485     15.563408\n"
    "*  2020  6 24 13 15  0.00000000\n"
    "PG08  11209.716830 -10713.543221  21543.351971    -38.652897\n"
    "PG10  18989.375629  11704.688739  14684.410767   -380.616599\n"
    "PG16  25722.366833   -497.431201   7257.882649   -174.411115\n"
    "PG18  -1703.034135  21126.008215  15981.241263    228.938925\n"
    "PG20   7945.502154  14786.067388  20547.437547    527.446927\n"
    "PG21   9596.077666  13660.568119  21485.446350     15.567441\n"
    "*  2020  6 24 13 30  0.00000000\n"
    "PG08  12408.203090  -8540.094231  21877.887705    -38.653740\n"
    "PG10  17214.054103  11772.219589  16673.782838   -380.626446\n"
    "PG16  26381.110092   -235.841125   4497.421430   -174.415665\n"
    "PG18  -2815.928687  22438.854161  13898.209721    228.948171\n"
    "PG20   5553.165699  15047.451338  21113.054383    527.446839\n"
    "PG21   8451.716682  15598.904800  20622.361836     15.572148\n"
    "*  2020  6 24 13 45  0.00000000\n"
    "PG08  13728.433306  -6421.809657  21836.058871    -38.653666\n"
    "PG10  15236.146433  11937.412720  18380.709769   -380.636302\n"
    "PG16  26739.150669    -38.100050   1661.268176   -174.419882\n"
    "PG18  -3730.115104  23594.121838  11575.562442    228.957546\n"
    "PG20   3130.863904  15429.429881  21313.510668    527.447015\n"
    "PG21   7460.170730  17494.563735  19428.952908     15.577088\n"
    "*  2020  6 24 14  0  0.00000000\n"
    "PG08  15143.837640  -4400.549449  21419.364822    -38.654444\n"
    "PG10  13094.736367  12222.060725  19776.006518   -380.646060\n"
    "PG16  26787.476793    133.425531  -1202.769329   -174.424084\n"
    "PG18  -4457.109220  24551.458779   9053.334699    228.966797\n"
    "PG20    725.565911  15927.378707  21144.644989    527.446993\n"
    "PG21   6624.436243  19304.522499  17923.801627     15.580392\n"
    "*  2020  6 24 14 15  0.00000000\n"
    "PG08  16621.549904  -2512.864913  20635.703989    -38.655549\n"
    "PG10  10833.387063  12640.950613  20835.640171   -380.655943\n"
    "PG16  26525.779980    317.327470  -4046.704331   -174.428345\n"
    "PG18  -5015.857646  25275.093632   6375.012002    228.975990\n"
    "PG20  -1616.930248  16529.064324  20608.733434    527.446900\n"
    "PG21   5939.995958  20986.417129  16130.246267     15.584692\n"
    "EOF\n";

int main(void) {
    char err[256];

    SidereonSp3 *sp3 = NULL;
    if (sidereon_sp3_load((const uint8_t *)SP3, strlen(SP3), &sp3)
        != SIDEREON_STATUS_OK) {
        sidereon_last_error_message(err, sizeof err);
        fprintf(stderr, "sp3 load: %s\n", err);
        return 1;
    }

    /* GPS L1 pseudoranges (m) for the satellites in view at the epoch. */
    SidereonObservation obs[] = {
        { "G08", 23825519.8 }, { "G10", 22717690.1 }, { "G16", 20478653.4 },
        { "G18", 21768335.2 }, { "G20", 21248327.7 }, { "G21", 20808709.8 },
    };
    SidereonSppInputs inputs = {
        .observations         = obs,
        .observation_count    = 6,
        .t_rx_j2000_s         = 646272000.0,
        .t_rx_second_of_day_s = 43200.0,
        .day_of_year          = 176.5,
        .initial_guess        = { 4.5e6, 0.5e6, 4.5e6, 0.0 },
        .ionosphere           = false,   /* geometry-only L1 solve */
        .troposphere          = false,
        .with_geodetic        = true,
    };

    SidereonSppSolution *sol = NULL;
    if (sidereon_solve_spp(sp3, &inputs, &sol) != SIDEREON_STATUS_OK) {
        sidereon_last_error_message(err, sizeof err);
        fprintf(stderr, "solve spp: %s\n", err);
        sidereon_sp3_free(sp3);
        return 1;
    }

    double xyz[3] = { 0 };
    if (sidereon_spp_solution_position(sol, xyz, 3) != SIDEREON_STATUS_OK) {
        sidereon_last_error_message(err, sizeof err);
        fprintf(stderr, "spp position: %s\n", err);
        return 1;
    }
    printf("ECEF m  %.3f %.3f %.3f\n", xyz[0], xyz[1], xyz[2]);
    /* ~4484128 550582 4487561 */

    sidereon_spp_solution_free(sol);
    sidereon_sp3_free(sp3);
    return 0;
}
```

The other solvers follow the same shape: a typed config in, an opaque solution
out, reader functions that copy scalars and positions into memory you own. Every
variable-length result uses one contract: call with a NULL buffer and length 0
to learn the required count, then call again with storage you own. Anything that
takes time takes UTC unix microseconds. The full surface (every struct, status
code, and per-function ownership note) lives in `bindings/c/include/sidereon.h`.

## Capabilities

- **Orbit propagation.** SGP4 from TLE/OMM, numerical state propagation with a
  composable force model (zonal harmonics through J6, Sun/Moon third-body,
  solar radiation pressure, relativistic correction, atmospheric drag) and
  orbital decay estimation, Kepler two-body propagation, batch and
  constellation propagation, pass prediction, look angles, ground tracks,
  coverage grids, and batch least-squares orbit fitting against precise
  ephemerides with a per-satellite residual ledger.
- **GNSS positioning.** Single-point positioning (SPP), RTK float and fixed
  (static, kinematic, and moving baseline), PPP float and fixed, DGNSS,
  velocity solving, RAIM / FDE fault detection and exclusion, robust solving,
  and DOP.
- **Integrity and error bounds.** Multi-constellation ARAIM protection levels,
  SBAS protection levels (DO-229), per-observation reliability (minimal
  detectable bias, internal and external), observability classification of
  every solution (rank, redundancy, conditioning), and covariance-derived
  error metrics (CEP, R95, SEP, error ellipse) that report wide or flagged
  bounds for weak geometry rather than fabricated confidence.
- **Timing, estimation, and geodesy.** Allan-family clock stability with
  power-law noise identification (IEEE 1139), scalar Kalman and alpha-beta
  trackers with innovation gating and CFAR thresholds, source localization
  (ToA/TDOA), robust station velocity (MIDAS) with trajectory fitting, step
  detection, and network motion fields, and repeating-geometry (sidereal)
  filtering.
- **GNSS corrections and biases.** SBAS message decode and corrected solving,
  SSR orbit / clock / bias corrections from RTCM SSR or Galileo HAS,
  Bias-SINEX DCB and OSB products, and DGNSS pseudorange corrections.
- **GNSS measurements.** Carrier-phase combinations (wide-lane, narrow-lane,
  Melbourne-Wubbena, ionosphere-free), cycle-slip detection, carrier smoothing,
  Doppler, and C/A-code generation, correlation, and acquisition search.
- **Ephemeris and time.** Broadcast ephemeris with fallback selection, SP3
  precise products, JPL SPK (DAF/.bsp) sampling, one sampling contract across
  broadcast, precise, and SSR-corrected sources, solving with
  precise-to-broadcast fallback and staleness reporting, scale-aware time
  conversions, and Earth-orientation handling.
- **Orbital mechanics.** Classical, equinoctial, and modified-equinoctial
  element conversions, anomaly conversions, Lambert transfer solutions, initial
  orbit determination (Gauss, Gibbs, Herrick-Gibbs), and relative motion in
  RSW / RTN / RIC / LVLH frames with Clohessy-Wiltshire propagation.
- **Geometry and events.** Coordinate-frame transforms, look angles, eclipse,
  conjunction screening with collision probability, and angular measures:
  separation, position angle, phase angle, beta angle, parallactic angle.
- **Observation and almanac.** Astrometric and apparent places (RA/Dec,
  azimuth/elevation with optional refraction, aberration, and light deflection)
  for the Sun, Moon, and any SPK body, sub-solar and sub-observer points, Moon
  rise/set and transit finding, satellite visual magnitude, and almanac events:
  seasons, moon phases, meridian transits, lunar and solar eclipses, and
  planetary events.
- **Atmosphere.** Klobuchar and NeQuick-G ionosphere, IONEX maps, troposphere
  delay models, and NRLMSISE-00 density.
- **Terrain and geoid.** DTED elevation lookup on tiles you supply, EGM96 geoid
  undulation, and orthometric / ellipsoidal height conversion.
- **RF.** Link-budget computation.
- **Formats.** Parsing and serialization for TLE/OMM, CCSDS (OEM/OPM/CDM),
  RINEX (observation, navigation, clock), CRINEX, SP3, IONEX, ANTEX,
  Bias-SINEX, RTCM 3, and SBAS messages.

Every result is exactly what the engine computes; the binding adds no modeling
of its own, and no data acquisition either: every product it consumes (orbit
files, bias products, ionosphere maps, terrain tiles) arrives as a buffer or
file you supply.

## How it's validated

`bindings/c/tests/run_smoke.sh` builds the library, regenerates the header,
compiles a suite of C programs against it, and runs them on committed reference
fixtures, asserting the binding reproduces the engine's reference numbers
bit-exact.

## Other interfaces

sidereon is one validated engine with first-class interfaces in several
languages, all returning the same numbers:

- Engine / core: [github.com/neilberkman/sidereon](https://github.com/neilberkman/sidereon)
- Python: [sidereon-python](https://github.com/neilberkman/sidereon-python)
- Elixir: [sidereon-ex](https://github.com/neilberkman/sidereon-ex)
- WebAssembly: [sidereon-wasm](https://github.com/neilberkman/sidereon-wasm)

See the live demo and docs at [sidereon.dev](https://sidereon.dev).

## License

MIT. See [LICENSE](LICENSE).
