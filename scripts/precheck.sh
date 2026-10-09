#!/usr/bin/env bash
# Run the CI checks locally before pushing (cloud/Linux sessions).
# Same toolchain (pinned in .github/workflows/ci.yml) and crate list as CI, plus a
# viewer `cargo check` that CI only runs on main. Output is capped to the failing
# tail so a red step costs a few hundred tokens, not a full log.
#
#   scripts/precheck.sh            # fmt, clippy, tests, viewers, python
#   scripts/precheck.sh --fast     # fmt + clippy only (iterate), then run the full set once
set -uo pipefail
cd "$(dirname "$0")/.."

ci=.github/workflows/ci.yml
toolchain=$(grep -m1 -oE 'toolchain: "[0-9.]+"' "$ci" | grep -oE '[0-9.]+')
crates=$(awk '/^  CRATES: >-/{f=1;next} f&&/^    /{printf "%s ",$0;next} f{exit}' "$ci")
fast=0; [[ "${1:-}" == "--fast" ]] && fast=1

if ! rustup run "$toolchain" cargo --version >/dev/null 2>&1; then
  echo "installing Rust $toolchain (once per container)"
  rustup toolchain install "$toolchain" --profile minimal -c clippy -c rustfmt >/dev/null 2>&1 \
    || { echo "FAIL: could not install Rust $toolchain"; exit 1; }
fi
cargo_() { cargo "+$toolchain" "$@"; }

failed=()
step() { # name, command...
  local name=$1; shift
  local log; log=$(mktemp)
  if "$@" >"$log" 2>&1; then echo "ok   $name"; else
    echo "FAIL $name"; tail -n 40 "$log"; failed+=("$name"); fi
  rm -f "$log"
}

# shellcheck disable=SC2086
step fmt cargo_ fmt --all --check
step clippy cargo_ clippy -q --locked $crates --all-targets -- -D warnings
if (( ! fast )); then
  step test cargo_ test -q --locked $crates
  if ! pkg-config --exists libudev alsa fontconfig 2>/dev/null && [[ $(id -u) == 0 ]]; then
    (apt-get update -q && apt-get install -y -q libudev-dev libasound2-dev \
      libwayland-dev libxkbcommon-dev libfontconfig1-dev pkg-config) >/dev/null 2>&1
  fi
  if pkg-config --exists libudev alsa fontconfig 2>/dev/null; then
    step viewers cargo_ check -q --locked -p lbp-viewer -p lbp-bevy-viewer --all-targets
  else
    echo "skip viewers (no libudev/alsa/fontconfig dev packages)"
  fi
  step python python3 -m unittest discover -s tests -q
fi

if (( ${#failed[@]} )); then echo "precheck: FAILED (${failed[*]})"; exit 1; fi
echo "precheck: all passed (Rust $toolchain)"
