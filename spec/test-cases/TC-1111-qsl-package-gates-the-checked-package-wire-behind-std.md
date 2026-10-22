---
id: TC-1111
title: "qsl-package keeps the checked-IR path no_std and the v2 wire behind std"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-015
    type: verifies
---
# TC-1111: qsl-package keeps the checked-IR path no_std and the v2 wire behind std

## Description

Verify the crate-tier row for `qsl-package`: the in-memory checked closure
builds with `std` disabled, the `quire.checked-package/v2` emitter and I2
reader are behind `std`, and default-feature output is unchanged. Scope:
NFR-015-AC-4 to NFR-015-AC-7.

## Test Procedure

1. Read `qsl-package/Cargo.toml` and its crate root.
2. Run `cargo check --locked -p qsl-package --no-default-features` on the
   host.
3. Run `cargo tree --locked -p qsl-package --no-default-features -e normal
   --depth 1`.
4. From a test target built with `--no-default-features`, name
   `CheckedPackage` and call `graph`, `shared_graph`, `dependencies`,
   `dependency_selections` and `abstraction`; in a compile-fail case, name
   `qsl_package::emit_checked`, `qsl_package::read_import_view` and
   `CheckedPackage::emitted_package_id`.
5. With default features and `QSPEC_DIR` set, run `cargo test --locked -p
   qsl-package --lib checked_v2` and the FR-091 round-trip test.
6. Search the shipped source of each crate in NFR-015's per-crate table, with
   `#[cfg(feature = "std")]` items excluded, for `serde_json`, `HashMap` and
   `HashSet`; run `make arch-lint-canonical-encoder`.

## Expected Results

1. `default = ["std"]`; `std` enables `dep:quire-contract-model` and
   `qsl-foundation/std`; `quire-contract-model` is `optional = true`;
   `qsl-foundation` is declared with `default-features = false`; the crate
   root has `#![cfg_attr(not(feature = "std"), no_std)]`.
2. The check succeeds.
3. Neither `quire-contract-model` nor `serde_json` is listed.
4. The accessor calls compile; each named emitter, reader and
   `emitted_package_id` item fails to resolve.
5. Both pass: the emitted bytes and `package_id` match the FR-322 vectors.
6. No match; the canonical-encoder check passes.
