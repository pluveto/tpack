# tpack-cli

Command-line tooling for inspecting and canonicalizing TPACK payloads.

The standalone CLI does not expose a schema registry or a switch for
`validate_embedded_schema_on_cache_hit`. In practice it decodes
`FullSchema` and `FullSchemaWithId` as self-contained messages, and
`SchemaRef` inputs fail unless an embedding application supplies a
registry through the library API. If an embedding application chooses the
recommended SHA-256 helper for `SchemaId`, that still only hashes the
schema descriptor bytes; `SchemaId` remains opaque on the wire and cache
binding trust still comes from the embedding deployment. The same
warning applies to the documented compact `xxh64-v1` profile.

Embedding applications that do not want SHA-256 can derive their own
opaque `SchemaId` bytes from `encode_schema(&schema)` and supply those
through the library API instead. The documented compact profile is
`xxh64-v1`, defined as `xxHash64(seed=0)` over the canonical descriptor
bytes with a fixed 8-byte big-endian output. That helper is available as
`tpack_core::recommended_schema_id_xxh64_v1`, while the `tpack` facade
adds `recommended_schema_id_sha256` for the stronger digest profile.
Collision handling and registry policy remain outside the standalone
CLI. The CLI therefore keeps rejecting `SchemaRef` unless an embedding
application defines the binding scope explicitly through the library
API.

## Commands

- `decode`: decode payloads into the raw Rust debug view
- `inspect`: decode payloads into a readable structural view
- `canonicalize`: rewrite a payload into canonical form

## Usage

```bash
cargo install tpack-cli
tpack inspect payload.bin
tpack inspect --format json --section value payload.bin
```

Repository-level interoperability vectors live under the root
`test-vectors/` directory, and the current implementation boundary is
summarized in `docs/implementation-status.md`.
