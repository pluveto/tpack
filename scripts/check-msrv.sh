#!/bin/sh

set -eu

if ! command -v cargo >/dev/null 2>&1; then
  echo "msrv check: cargo is required" >&2
  exit 1
fi

if ! cargo +1.85.0 --version >/dev/null 2>&1; then
  echo "msrv check: Rust toolchain 1.85.0 is required" >&2
  exit 1
fi

cargo +1.85.0 test --locked -p tpack-macros
cargo +1.85.0 test --locked -p tpack --tests reference
