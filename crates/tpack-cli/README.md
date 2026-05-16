# tpack-cli

Command-line tooling for inspecting and verifying TPACK payloads.

## Commands

- `inspect`: decode payloads into a readable structural view
- `verify`: validate canonical form and schema consistency
- `canonicalize`: rewrite a payload into canonical form

## Usage

```bash
cargo install tpack-cli
tpack-cli verify -i payload.bin
```

