# AGENTS.md

This file provides guidance to coding agents working in this repository.

## Commands

```sh
cargo build --release   # build all binaries
cargo check             # typecheck without building
cargo clippy            # lint
cargo fmt               # format
nix build               # build via Nix (requires Cargo.lock)
```

## Architecture

Single Cargo crate (`niri-toolbelt`) with binaries under `src/bin/`. No shared library code — each binary is self-contained.

All binaries communicate with the running niri compositor via `niri-ipc` (Unix socket). The pattern is: connect → send request → match response → send action if needed.

New utilities go in `src/bin/<name>.rs`. No need to register them in `Cargo.toml` — Cargo auto-discovers `src/bin/`.
