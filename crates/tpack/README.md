# tpack

The `std` facade for TPACK.

This crate re-exports the core API and derive macros, and it hosts convenience features that depend on the standard library.

## Features

- `derive` default feature for native derive support
- `serde_support` for schema-aware serde integration
- `std` for registry and convenience APIs built on top of the core crate

## Native Path

For low-latency use cases, prefer the native traits and a schema registry that can resolve `SchemaRef` payloads without extra work.

When decoding `FullSchemaWithId` with a registry hit, the default path now reparses the embedded schema bytes and requires them to match the cached schema before reusing the cached AST. If a deployment intentionally trusts the registry entry and wants the older skip-only behavior, set `DecodeOptions::validate_embedded_schema_on_cache_hit` to `false`.

## Serde Path

The serde bridge is available when the `serde_support` feature is enabled. It is intended for compatibility and convenience, not the fastest decode path.

`from_slice` and `from_slice_with_registry` inherit the byte-level decoder limits. `from_value` applies `Limits::default()`, and `from_value_with_limits` is available when an in-memory `TpackValue` needs an explicit depth budget or other non-default limits.
