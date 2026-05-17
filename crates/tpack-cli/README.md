# tpack-cli

Command-line tooling for inspecting and canonicalizing TPACK payloads.

The standalone CLI decodes `FullSchema` and `FullSchemaWithId` as self-contained messages, and `SchemaRef` inputs require a registry through the library API.

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
