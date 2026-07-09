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

## Run 2

## #32 SP3 merge/frame-reconciliation accessor completion

Status: closed.

Implemented / verified:
- Existing C merge inputs include asserted frame-label sets and Helmert reconciliation controls.
- Existing report accessors cover reconciliation count, typed reconciliation row, source label, target label, asserted labels, provenance, flag counts/rows/source indices, per-epoch agreement, and agreement summary.

Coverage:
- `bindings/c/tests/smoke.c`: exercises asserted SP3 frame-label reconciliation through `sidereon_sp3_merge`, then reads the typed reconciliation row plus source/asserted label byte accessors.
- `bindings/c/tests/newgaps.c`: verifies SP3 merge agreement metrics.
- `bindings/c/tests/core_caps_smoke.c`: verifies clock-reference offset and alignment helpers.

## #33 precise samples/interpolant wrappers

Status: closed.

Implemented / verified:
- Existing sample-backed precise ephemeris handle and cached interpolant handle follow the C handle/free pattern.
- Existing wrappers cover extraction from SP3, construction from canonical samples, construction of interpolants from SP3/samples/sample-backed handles, batch observable-state access, shared-epoch access, regular-grid sampling, and range prediction.

Coverage:
- `bindings/c/tests/precise_samples_smoke.c`: extracts real SP3 samples, rebuilds a sample-backed source, and checks batch range prediction through the rebuilt source.
- `bindings/c/tests/cap013_smoke.c`: covers cached interpolants from SP3, raw samples, and sample-backed handles, with observable-state accessors.

## #57 geometry-quality direct label helpers

Status: closed.

Implemented:
- `sidereon_observability_tier_label`

Coverage:
- `bindings/c/tests/cap013_smoke.c`: obtains a real `SidereonGeometryQuality` from a source-localization solve and verifies its tier label resolves to `nominal`.

## #62 staleness high-level selectors

Status: closed.

Implemented / verified:
- Existing C high-level selectors mirror the Python/wasm/Rust contract: `sidereon_select_sp3`, `sidereon_select_sp3_over_range`, `sidereon_select_ionex`, `sidereon_select_ionex_over_range`, policy helpers, sourced-solution metadata accessors, and `sidereon_solve_with_fallback`.

Coverage:
- `bindings/c/tests/smoke.c`: exercises SP3 exact, nearest-prior, cap, empty, no-prior, IONEX exact, IONEX diurnal-shift, invalid-policy, and precise/broadcast fallback branches.
- `bindings/c/tests/robustness_smoke.c`: covers additional fallback branches against committed NAV/SP3 fixtures.

## #90 fusion/inertial widening

Status: closed.

Implemented / verified:
- Existing C fusion surface uses opaque handles plus accessors and `_free` functions for the filter, RTS history builder, recorded RTS history, and smoothed trajectory.
- Existing typed structs cover IMU specs, navigation state, filter config, loose and tight measurements, update summaries, time-sync status, filter state, RTS epochs, and smoothed epochs.
- Existing wrappers cover propagation, loose/tight/stationary/non-holonomic updates, recorded updates, time-sync updates, state encode/restore, RTS smoothing, velocity-match outage repair, and state/covariance accessors.

Coverage:
- `bindings/c/tests/domain018_smoke.c`: covers fusion config defaults, IMU propagation, loose updates, field-mode options, stationary and non-holonomic updates, time-sync status, state codec round-trip, recorded RTS history, smoothing accessors, and velocity matching.

## #92 signal-analysis widening

Status: closed.

Implemented:
- `sidereon_signal_spectral_separation_coefficient_hz`
- `sidereon_signal_spectral_separation_coefficient_db_hz`
- `sidereon_signal_white_noise_spectral_separation_hz`

Implemented / verified:
- Existing signal-analysis structs and wrappers cover modulation descriptors, PSD, fraction power, RMS bandwidth, combined spectral separation, effective C/N0 degradation, DLL jitter/lower bound, and multipath envelope.

Coverage:
- `bindings/c/tests/domain018_smoke.c`: verifies PSD, bandwidth, combined and scalar spectral-separation helpers, white-noise scalar separation, C/N0 degradation, DLL metrics, and multipath envelope against real closed-form values.

## #93 terrain/geoid store drift

Status: closed.

Implemented / verified:
- Existing C surface covers DTED terrain handles, batch height queries, memory-mappable terrain-store build/read/write/from-bytes/from-vec/from-path access, checksums, tile index, typed terrain errors, EGM96 15-minute geoid loading, embedded EGM96 helpers, EGM96/geoid batch helpers, EGM2008 raster/window loading, and grid height conversions.

Coverage:
- `bindings/c/tests/phaseb_smoke.c`: checks DTED terrain load and scalar height.
- `bindings/c/tests/core012_smoke.c`: checks DTED batch terrain, terrain-store build/read/write/round-trip, tile index, checksums, orthometric and ellipsoidal queries, and typed missing-DAC errors.
- `bindings/c/tests/capround_smoke.c`, `bindings/c/tests/caps_extra_smoke.c`, and `bindings/c/tests/round2_parity_smoke.c`: cover geoid undulation, height conversions, grid construction, and EGM96 batches.
- `bindings/c/tests/wave2_smoke.c`: covers EGM2008 raster-window loading and grid undulation.

## Run 2 Gates

All passed with exit code 0:
- `cargo test`
- `cargo clippy --all-targets -- -D warnings`
- `cargo fmt --check`
- `bash bindings/c/tests/run_smoke.sh`

Header regenerated:
- `cbindgen --quiet --config bindings/c/cbindgen.toml --crate sidereon-c --output bindings/c/include/sidereon.h`
