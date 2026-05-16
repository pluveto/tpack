# tpack-macros

Procedural macros for native schema binding.

This crate generates `TpackSerialize` and `TpackDeserialize` implementations from Rust item definitions.

## Purpose

- avoid runtime reflection tables
- avoid string-based field lookup in the hot path
- preserve stable field identity through explicit `field_id` annotations

## Example

```rust
#[derive(TpackSerialize, TpackDeserialize)]
pub struct LogEntry {
    #[tpack(field_id = 1)]
    pub timestamp: u64,
    #[tpack(field_id = 2)]
    pub message: String,
}
```

