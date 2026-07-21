#!/usr/bin/env bash
# Deterministic CI gate for the generated public header and the 0.33 C ABI.
# The exhaustive run_smoke.sh remains available locally; this focused gate uses
# only fixtures committed to this repository so it can run on every CI host.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
binding_root="$(cd "${here}/.." && pwd)"

cd "${binding_root}"
cargo build --release

target_dir="$(cargo metadata --format-version 1 --quiet \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
lib_dir="${target_dir}/release"
generated_header="$(mktemp "${TMPDIR:-/tmp}/sidereon-header.XXXXXX")"
trap 'rm -f "${generated_header}"' EXIT

cbindgen --quiet --config cbindgen.toml --crate sidereon-c --output "${generated_header}"
if ! cmp -s include/sidereon.h "${generated_header}"; then
    echo "committed sidereon.h differs from cbindgen output" >&2
    diff -u include/sidereon.h "${generated_header}" >&2 || true
    exit 1
fi

cc -std=c11 -Wall -Wextra -Werror \
    -I"${binding_root}/include" \
    "${here}/data_distribution_smoke.c" \
    -L"${lib_dir}" \
    -lsidereon \
    -Wl,-rpath,"${lib_dir}" \
    -lm \
    -o "${target_dir}/data_distribution_smoke_ci"
"${target_dir}/data_distribution_smoke_ci"

cc -std=c11 -Wall -Wextra -Werror \
    -I"${binding_root}/include" \
    "${here}/sp3_exact_smoke.c" \
    -L"${lib_dir}" \
    -lsidereon \
    -Wl,-rpath,"${lib_dir}" \
    -lm \
    -o "${target_dir}/sp3_exact_smoke_ci"
"${target_dir}/sp3_exact_smoke_ci" \
    "${here}/fixtures/sp3/GRG0MGXFIN_20201760000_01D_15M_ORB.SP3" \
    "${here}/fixtures/sp3/IGS0OPSFIN_20261200945_02H30M_15M_ORB.SP3"
