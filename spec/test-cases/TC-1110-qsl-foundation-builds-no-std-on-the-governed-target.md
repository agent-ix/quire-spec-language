---
id: TC-1110
title: "qsl-foundation builds no_std + alloc on the governed target"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-015
    type: verifies
---
# TC-1110: qsl-foundation builds no_std + alloc on the governed target

## Description

Verify the closure-tier row for `qsl-foundation` and the `build-no-std`
gate. Scope: NFR-015-AC-1, NFR-015-AC-2, NFR-015-AC-3 and NFR-015-AC-8.

## Test Procedure

1. Read `qsl-foundation/Cargo.toml` and its crate root.
2. Run `cargo build --locked -p qsl-foundation --no-default-features
   --target thumbv7em-none-eabi` with the toolchain `rust-toolchain.toml`
   pins.
3. Run `cargo tree --locked -p qsl-foundation --no-default-features -e
   normal,features --target thumbv7em-none-eabi`.
4. Run `cargo test --locked -p qsl-foundation` and `cargo test --locked -p
   qsl-foundation --no-default-features` on the host.
5. Read the Makefile's `build-no-std` recipe and `CI_CHECKS` line, then run
   `make build-no-std`.
6. In a scratch copy, add a shipped `use std::collections::HashMap;` to a
   `qsl-foundation` module outside `#[cfg(feature = "std")]` and run
   `make build-no-std`.

## Expected Results

1. `[features]` has `default = ["std"]`; `serde`, `sha2` and `thiserror` are
   declared with `default-features = false` and `std` enables their `std`
   features; the crate root has `#![cfg_attr(not(feature = "std"), no_std)]`.
2. The build succeeds.
3. Normal dependencies are exactly `quire-exact`, `serde`, `sha2`,
   `thiserror` and the `ix-trace-rs` proc macro, and no `std` feature of
   `serde`, `sha2` or `thiserror` is listed.
4. Both runs pass.
5. The recipe runs step 2's command with `-p` in its own `no-std` target
   directory; `CI_CHECKS` lists `build-no-std`; the run passes.
6. The run fails at the `qsl-foundation` build.
