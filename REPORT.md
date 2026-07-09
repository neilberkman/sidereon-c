# sidereon-c parity 0.25 wave report

## #35/#36 RINEX NAV and OBS load conveniences

Status: closed.

Implemented:
- `sidereon_broadcast_ephemeris_load_nav`
- `sidereon_rinex_obs_load`

Coverage:
- `bindings/c/tests/smoke.c`: `exercise_rinex_spp_surface` loads the committed ESBC mixed NAV fixture and trimmed ESBC OBS fixture through the new path loaders before using the handles in real assembly and solve calls.
- Gate: `bash bindings/c/tests/run_smoke.sh`

## #4 RINEX OBS to SPP inputs and solve convenience

Status: closed for the broadcast-NAV source contract exposed by C in this pass.

Implemented:
- `SidereonRinexSppOptions`
- `SidereonRinexSppEpoch`
- `SidereonRinexSppInputs` handle plus count, epoch, epoch-input, and free accessors
- `SidereonRinexSppSolutions` handle plus count, epoch, ok, solution, error, and free accessors
- `sidereon_rinex_spp_options_init`
- `sidereon_spp_inputs_from_rinex_obs`
- `sidereon_solve_spp_from_rinex_obs`

Coverage:
- `bindings/c/tests/smoke.c`: `exercise_rinex_spp_surface` assembles real RINEX OBS epochs against the real ESBC NAV product, verifies epoch/input accessors, runs the serial RINEX SPP solve convenience, and checks a solved epoch has finite clock, at least four used satellites, and a plausible ECEF receiver radius.
- Gate: `bash bindings/c/tests/run_smoke.sh`

## #32 SP3 merge/frame-reconciliation accessor completion

Status: intentionally-not in this commit.

Reason: lower priority than the RINEX rows and already a broad existing C family. Left untouched to keep this commit gated around the RINEX load and RINEX-SPP closure.

## #33 precise samples/interpolant wrappers

Status: intentionally-not in this commit.

Reason: lower priority and already partially covered by existing C precise sample/interpolant handles and smoke programs. No untested expansion was added.

## #57 geometry-quality direct label helpers

Status: intentionally-not in this commit.

Reason: lower priority; C already carries typed quality/tier values inside result structs. Direct label helpers remain for a later focused pass.

## #62 staleness high-level selectors

Status: intentionally-not in this commit.

Reason: lower priority; existing staleness selectors are covered by the smoke suite, but no new high-level selector was added in this gated subset.

## #90 fusion and #92 signal-analysis

Status: intentionally-not in this commit.

Reason: broad typed-model widening. No gated subset was attempted here after closing the higher-priority RINEX rows.

## #93 terrain/geoid store drift

Status: intentionally-not in this commit.

Reason: lower priority; existing terrain/geoid smoke coverage remains unchanged.

## Gates

All passed with exit code 0:
- `cargo test`
- `cargo clippy --all-targets -- -D warnings`
- `cargo fmt --check`
- `bash bindings/c/tests/run_smoke.sh`

Header regenerated:
- `cbindgen --quiet --config cbindgen.toml --crate sidereon-c --output include/sidereon.h`
