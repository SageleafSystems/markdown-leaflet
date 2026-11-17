#!/usr/bin/env bash
# Lint for every desktop target so cfg-gated code (e.g. the in-window menu bar,
# which macOS builds omit) is compiled before it ships. Cross-checks only; no
# linking, so it runs from any host. Run alongside the checks in AGENTS.md.
set -euo pipefail

cd "$(dirname "$0")/.."

targets=(
  aarch64-apple-darwin
  x86_64-unknown-linux-gnu
  x86_64-pc-windows-msvc
)

installed=$(rustup target list --installed)
status=0
for target in "${targets[@]}"; do
  if ! grep -qx "$target" <<<"$installed"; then
    echo "==> $target: not installed (rustup target add $target)" >&2
    status=1
    continue
  fi
  echo "==> $target"
  cargo clippy --locked --all-targets --target "$target" -- -D warnings || status=1
done
exit $status
