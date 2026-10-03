#!/usr/bin/env bash
# Regenerate every golden the C smoke tests compare against, from the
# sidereon-core revision tests/CORE_REVISION names. CI runs this and then
# `git diff --exit-code`, so a committed golden that the generators no longer
# reproduce fails the build.
#
# Order matters: verified core fixture emitters run first, then the Rust
# generators, then the PPP expected block, then the Python transcribers that
# read their output.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
binding_root="$(cd "${here}/.." && pwd)"
cd "${binding_root}"

scratch="$(mktemp -d "${TMPDIR:-/tmp}/sidereon-c-generators.XXXXXX")"
trap 'rm -rf "${scratch}"' EXIT

python3 tests/test_validate_generator_sources.py
python3 tests/validate_generator_sources.py > "${scratch}/source-report.json"
rev="$(tr -d '[:space:]' < tests/CORE_REVISION)"
source_mode="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["mode"])' "${scratch}/source-report.json")"
echo "== verified sidereon-core fixture emitters (${source_mode} ${rev}) =="
python3 tests/prepare_core_fixtures.py "${scratch}/source-report.json" "${scratch}" > "${scratch}/fixture-paths.json"
export SIDEREON_CORE_FIXTURES="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["core_fixtures"])' "${scratch}/fixture-paths.json")"
export SIDEREON_EMITTED_FIXTURES="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["emitted_fixtures"])' "${scratch}/fixture-paths.json")"

echo "== Rust generators =="
run() { cargo run --locked --release --quiet --manifest-path "tests/$1/Cargo.toml"; }
run sppgen > tests/fixtures/spp_expected.json
run fbgen > tests/broadcast_fixture.h
run velgen > tests/velocity_fixture.h
run rinexgen > tests/rinex_fixture.h
run pingen > tests/engine_pins_fixture.h
run rtkgen > tests/rtk_fixture.h
# Every valgen binary writes tests/<name>_pins.h.
for bin in tests/valgen/src/bin/*.rs; do
    [ -e "${bin}" ] || continue
    name="$(basename "${bin}" .rs)"
    cargo run --locked --release --quiet --manifest-path tests/valgen/Cargo.toml --bin "${name}" \
        > "tests/${name}_pins.h"
done

echo "== PPP expected block =="
cargo test --locked --quiet --lib ppp_fixture_parity::write_ppp_esbc_expected -- --ignored --exact

echo "== Python transcribers =="
for script in gen_fixture_header gen_dop_fixture_header gen_antex_fixture_header gen_constellation_fixture_header \
    gen_iono_fixture_header gen_prop_fixture_header gen_ppp_fixture_header; do
    python3 "tests/${script}.py" > /dev/null
done
cat "${scratch}/generation-context.json"
if [[ -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
    printf '\n### C generator source provenance\n\n```json\n' >> "${GITHUB_STEP_SUMMARY}"
    cat "${scratch}/generation-context.json" >> "${GITHUB_STEP_SUMMARY}"
    printf '\n```\n' >> "${GITHUB_STEP_SUMMARY}"
fi
if [[ -n "${RUNNER_TEMP:-}" ]]; then
    cp "${scratch}/generation-context.json" "${RUNNER_TEMP}/sidereon-c-generator-provenance.json"
fi
echo "generators done"
