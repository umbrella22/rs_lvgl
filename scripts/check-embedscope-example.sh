#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

manifest=examples/embedscope/Cargo.toml
artifact_dir="$PWD/target/embedscope-example"
host=$(rustc -vV | sed -n 's/^host: //p')
[[ -n "$host" ]]
# Force the host even when the invoking shell has an embedded Cargo target.
unset "CARGO_TARGET_$(printf '%s' "$host" | tr '[:lower:]-' '[:upper:]_')_RUNNER"
if [[ ! -f examples/embedscope/Cargo.lock ]]; then
    cargo generate-lockfile --manifest-path "$manifest"
fi
mkdir -p "$artifact_dir"

graph() {
    cargo tree --color never --manifest-path "$manifest" --locked --target "$host" \
        --edges normal,build --prefix none --format '{p}' "$@"
}
graph --features embedscope >"$artifact_dir/enabled.graph"
graph --no-default-features >"$artifact_dir/disabled.graph"
providers=$(awk '$1 == "rs_lvgl_sys" { sub(/ \(\*\)$/, ""); print }' "$artifact_dir/enabled.graph" | sort -u)
# The default format includes the source for non-registry packages. Require
# one registry provider in the 0.8 series, including after a patch is released.
if [[ ! "$providers" =~ ^rs_lvgl_sys\ v0\.8\.[0-9]+$ ]]; then
    printf 'Expected one registry rs_lvgl_sys 0.8 provider, found: %s\n' "$providers" >&2
    exit 1
fi
grep -Fxq 'escope-lvgl v0.6.0' "$artifact_dir/enabled.graph"
if grep -q '^escope-' "$artifact_dir/disabled.graph"; then
    printf '%s\n' 'Feature-off normal/build graph contains EmbedScope' >&2
    exit 1
fi

cargo test --manifest-path "$manifest" --target-dir "$artifact_dir/rust" \
    --target "$host" --locked --features embedscope -- --test-threads=1
cargo build --manifest-path "$manifest" --target-dir "$artifact_dir/rust" \
    --target "$host" --locked --no-default-features --release
disabled="$artifact_dir/rust/$host/release/rs-lvgl-embedscope-example"
"$disabled" --run-for-ms 100 >"$artifact_dir/disabled.log"
grep -Fxq 'EmbedScope disabled; native memory display running' "$artifact_dir/disabled.log"
if grep -q '^LISTEN ' "$artifact_dir/disabled.log"; then
    printf '%s\n' 'The feature-off application reported a listener' >&2
    exit 1
fi
if "$disabled" --listen 127.0.0.1:0 --run-for-ms 100 >"$artifact_dir/disabled-listen.log" 2>&1; then
    printf '%s\n' 'The feature-off executable accepted --listen' >&2
    exit 1
fi
grep -Fq 'unknown option: --listen' "$artifact_dir/disabled-listen.log"
