# TPACK (Typed Pack)

[![Crates.io](https://img.shields.io/crates/v/tpack.svg)](https://crates.io/crates/tpack)
[![Documentation](https://docs.rs/tpack/badge.svg)](https://docs.rs/tpack)
[![CI Status](https://github.com/pluveto/tpack/actions/workflows/ci.yml/badge.svg)](https://github.com/pluveto/tpack/actions/workflows/ci.yml)

TPACK is a strictly typed, self-describing binary serialization format. This repository is the Rust reference implementation for v1.

## Workspace Layout

The workspace is split by responsibility:

- `tpack-core`: `#![no_std] + alloc` core wire codec, schema AST, validation, and native traits
- `tpack-macros`: procedural macros for native derive support
- `tpack`: `std` facade, registry integration, and optional `serde` support
- `tpack-cli`: command-line tooling for inspection, verification, and canonicalization
- `tpack-bench`: internal size/throughput harness (`publish = false`; see `docs/benchmarks.md`)

## Wire Protocol v1

TPACK messages are built from a fixed header and an envelope.

```text
[Header: Magic 4B + Version 1B] [Envelope: EnvelopeMode 1B + Payload...]
```

Envelope modes:

- `0x00` `FullSchema`: `SchemaLen + Schema + Data`
- `0x01` `FullSchemaWithId`: `SchemaIdLen + SchemaId + SchemaLen + Schema + Data`
- `0x02` `SchemaRef`: `SchemaIdLen + SchemaId + Data`

`FullSchemaWithId` reuses the cached schema AST when the schema ID is already in the registry. The decoder validates the embedded schema bytes against the cached schema by default.

`SchemaRef` requires an active registry entry.

`SchemaId` remains opaque in the core format.

The only official recommended `SchemaId` profile is `xxh64-v1`: `xxHash64(seed=0)` over the canonical schema descriptor bytes, serialized as a fixed 8-byte big-endian value. This recommendation is for bounded or registry-backed deployments and does not change the core opaque-bytes semantics.

If a deployment uses `xxh64-v1`, it must keep the binding scope explicit and fail closed on ambiguity, collision, stale cache state, or loss of binding context after reset or reconnect. Deployments may use another profile by prior agreement, but that is outside the official recommendation.

## What The Core Guarantees

- Single-pass parsing after the active schema is available
- Borrowed strings and byte slices on the native data path
- Canonical encoding checks, including shortest varints and map ordering
- Explicit failure on malformed or non-canonical inputs
- Shared schema size limits on both decode and encode paths
- No dependency on host-language object layout

## Current Conformance Boundary

This repository is the Rust reference implementation for the envelope layout, schema encoding, validation rules, canonicalization behavior, and the example vectors in the Internet-Draft.

The draft data model defines `Decimal`, `BigInt`, and `BigUInt` as arbitrary-precision types. The Rust value model matches that boundary via `num-bigint`, with explicit decoder limits:

- `Decimal { scale: i64, coefficient: BigInt }`
- `Decimal(P,S)` coefficient is `BigInt` (also capped by schema precision `P`)
- `BigInt` / `BigUInt` use `num_bigint::{BigInt, BigUint}`
- Default limits: `max_bigint_bytes = 1024`, `max_decimal_digits = 10_000`

Scale stays `i64` for practical range. Small values that fit historical
`i64`/`u64` ranges remain wire-compatible with published test vectors.
See `docs/implementation-status.md` for details.

## Verification

```bash
cargo fmt --all --check
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features
```

Public interoperability vectors live under `test-vectors/` and are
consumed directly by `crates/tpack/tests/reference.rs`. Cached-schema
safety behavior is covered by `crates/tpack/tests/cache_validation.rs`.
The current reference-implementation boundary is summarized in
`docs/implementation-status.md`.

Additional repository checks are defined in `deny.toml`, `typos.toml`, and the GitHub Actions workflows.

## Benchmarks

Size tables and methodology live in [`docs/benchmarks.md`](docs/benchmarks.md)
(generated). Steady-state comparisons emphasize `SchemaRef` and amortized
multi-message streams; FullSchema rows are cold/bootstrap.

```bash
cargo run -p tpack-bench --example size_report
cargo bench -p tpack-bench -- --quick   # optional; not in default CI
```

Hot encode paths should use `PreparedSchema` + a reused `Encoder` (see
`tpack-core`); the bench harness does this via `SteadyEncoder`.

## Internet-Draft

The Internet-Draft is maintained in `drafts/draft-zhang-tpack-format-00.md`
using `kramdown-rfc`.  Regenerate the rendered artifacts with:

```bash
make -C drafts
```

`scripts/build-draft.sh` (invoked by that Makefile) prefers a local
`kramdown-rfc2629` / `kramdown-rfc` install.  If neither is on `PATH`,
it falls back to Docker (`ruby:3.3-slim` by default) and caches the gem
in a named volume so subsequent runs skip reinstall.  You still need a
local `xml2rfc` for the XML → text/html step
(`pip install --user xml2rfc` is enough).

Override the image or gem-cache volume if needed:

```bash
KRAMDOWN_DOCKER_IMAGE=ruby:3.3-slim \
KRAMDOWN_GEM_CACHE_VOLUME=tpack-kramdown-gem-cache \
make -C drafts
```

`make -C drafts` writes `drafts/draft-zhang-tpack-format-00.xml`,
`drafts/draft-zhang-tpack-format-00.txt`, and
`drafts/draft-zhang-tpack-format-00.html`.  Run `idnits` against the
generated `.txt` before submission.

## Release Flow

Releases are automated with `release-plz` and GitHub Actions. Release notes are accumulated in `CHANGELOG.md`, and the process is documented in `RELEASING.md`.
