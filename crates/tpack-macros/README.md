# tpack-macros

Procedural macros for native schema binding.

This crate generates `TpackSerialize` and `TpackDeserialize` implementations from Rust item definitions.

## Purpose

- avoid runtime reflection tables
- avoid string-based field lookup in the hot path
- preserve stable field identity through explicit `field_id` annotations

## Example

```rust
#[tpack(auto)]
#[derive(TpackSerialize, TpackDeserialize)]
pub struct LogEntry {
    pub timestamp: u64,
    pub message: String,
}
```

By default, each field must declare `#[tpack(field_id = N)]`. Add `#[tpack(auto)]` to opt into declaration-order numbering starting at 1.
