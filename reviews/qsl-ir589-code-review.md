---
id: SR-12342
title: "Code review of PR #1233 (IR-589 S5: qsl-package checked-IR path no_std + alloc behind a default std feature)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@30e831e6d2ebf9f0186c8ad28cb40c9da13f0d71; qsl-package/Cargo.toml, qsl-package/src/checked.rs, qsl-package/src/export.rs, qsl-package/src/lib.rs, qsl-package/tests/checked_ir_surface.rs"
review_set: subset
---

## Summary

Ticket: IR-589. This is a code review with the rust-review lane, over
`git diff origin/main...HEAD` (5 files, +253/-15). It was checked against
NFR-015 (the `qsl-package` crate-tier row, the surface table and the
checked-IR path section), TC-1111, and S5 of STD-299.

Checked:

- **The default build keeps its public API.** Every new gate is
  `#[cfg(feature = "std")]`, and `default = ["std"]`. With default features,
  the crate root re-exports the same items as on main:
  `CheckedPackage`, `ResolvedDependency`, `EmittedPackage`, `Import`,
  `LinkRefusal`, the whole `checked_v2` and `emit` lists, and `export`.
  `test-support` now needs `all(std, test-support)`, and with defaults on
  that is the same set as before. Under `std`, `bind_abstraction` still
  resets the remembered emission, and `bind_abstraction_at` still does not,
  as NFR-015 states. The `std::` to `core::`/`alloc::` renames
  (`mem::take`, `iter::once`, `fmt`, `BTreeMap`, `Arc`, `Vec`) name the same
  items. `cargo test --locked -p qsl-package` passes with default features:
  230 lib, 3 integration and 12 doc tests. This needs
  `QSPEC_DIR` at quire-specification main 5ee0071f. The machine's sibling
  checkout is a stale detached HEAD at 3a0a9f22, and with it 3 conformance
  tests fail on fixture drift that has nothing to do with this diff.
  `arch-lint-api-surface-qsl` passes.
- **The std-off build.** `cargo check --locked -p qsl-package
  --no-default-features` passes, and so does `cargo clippy
  --no-default-features --all-targets -D warnings`. `cargo test
  --no-default-features` passes: 3 integration tests and 37 doc tests,
  including the 26 `compile_fail` cases. Clippy with `--all-features
  --all-targets -D warnings` is clean, and `cargo fmt --check` is clean.
- **Dependency wiring.** `quire-contract-model` is `optional` and comes in
  through `dep:` in `std`. `cargo tree --no-default-features -e normal -i
  quire-contract-model` finds no path to it (AC-4, third clause). `serde`
  and `thiserror` have default features off, and `std` turns their `std`
  features on. The `qsl-foundation/std` token is missing. That is the
  known gap while S3 (PR #1230) is open: on main, `qsl-foundation` has no
  `std` feature, so the token would not resolve. After S3 merges, the
  Cargo side is exactly one token. The Makefile side is two lines (the
  `cargo check` and `cargo clippy` row) added to S3's `build-no-std`
  recipe. See SR-12343 FND-002 for a third line it needs.
- **`serde_json` stays out of shipped source.** Outside `#[cfg(test)]`, no
  file in `qsl-package/src` names `serde_json`, `HashMap` or `HashSet`
  (`emit/extent_agreement.rs` is a `#[cfg(test)] mod`).
  `arch-lint-canonical-encoder` and `arch-lint-qualified-core` pass.
- **Stacked `#[cfg(test)]` and `#[cfg(feature = "std")]`.** This is
  required, not ceremony. `has_cfg_test` in `tools/arch-lint/api_surface.rs`
  matches only a top-level `test` path. A probe replaced `checked.rs`'s pair
  with `#[cfg(all(test, feature = "std"))]`, and `arch-lint
  canonical-encoder` then FAILED on `checked.rs`, reading it as a second
  canonical encoder. The probe was reverted. The workaround fails loudly,
  not silently, if someone collapses it, so it is safe, but it exists only
  because the lint is imprecise (FND-005).
- **`allow(dead_code)` on `DependencyPath`.** This is justified. Without
  `std`, only `link_with` builds a selection, so no variant is constructed.
  The spec keeps `ResolvedDependency` and `dependency_selections` in the
  std-off surface, so the type has to stay. The allow is scoped to
  `not(feature = "std")` and carries a reason (FND-004 is a nit).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `serde` is a dead dependency in both configurations. No file under `qsl-package/src` or `qsl-package/tests` names `serde` other than as `serde_json`: no `Serialize`/`Deserialize` derive and no `serde::` path. The PR rewrote this line, and its comment ("The emitter writes its wire envelope as a `Serialize` struct over IR's types") describes code that no longer exists. NFR-015's row lists `serde` among the std-off dependencies only because the crate declares it. Delete the dependency, the `serde/std` entry in `std`, and the row's `serde` entry | qsl-package/Cargo.toml:15, qsl-package/Cargo.toml:48-50 |
| FND-002 | low | With `std` off, `thiserror`, `quire-canonical` and `quire-exact` are used only by `std` items or by tests (`LinkRefusal`, `EMIT_LIMITS`, `EmittedPackage::new`, `checked_v2`, `emit`), but they stay non-optional. NFR-015's feature contract says "A dependency used only by `std` items is `optional = true` and enabled by `std`". The per-crate row lists them as std-off dependencies, so the code follows the row and the spec contradicts itself. Either make them `optional` under `std` and update the row, or reword the contract bullet. A no_std consumer pays to compile three unused crates | qsl-package/Cargo.toml:38, 44, 51; spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md |
| FND-003 | low | `#[allow(dead_code)]` on the empty `mod behind_std {}` suppresses nothing, because the `dead_code` lint does not fire on an empty private module. Delete the attribute and keep the module as the carrier of the compile-fail docs | qsl-package/src/lib.rs:171-172 |
| FND-004 | low | `#[cfg_attr(not(feature = "std"), allow(dead_code))]` on `DependencyPath` would be better as `expect(dead_code)`. Then it fails as soon as a std-off constructor of a selection makes the type live (the S4/IR-588 closure-tier work), and does not linger silently | qsl-package/src/checked.rs:213-215 |
| FND-005 | low | The arch-lint `has_cfg_test` recognises only a bare top-level `test` in `#[cfg(...)]`, so `#[cfg(all(test, feature = "std"))]` is scanned as shipped code. Measured: collapsing `checked.rs`'s stacked pair fails `canonical-encoder`. Every std-only test module in the NFR-015 slices must use the two-attribute form. Teach `has_cfg_test` that `test` inside `all(...)` is test-only. This can be deferred to its own ticket | tools/arch-lint/api_surface.rs:493-507; qsl-package/src/checked.rs:708-709, 752-753, 778-779; qsl-package/src/export.rs:112-113 |

## Verdict

Correct, and the default build is unchanged. There is no API removal, no
behaviour change under `std`, and no compatibility layer, and nothing is
vendored. `serde_json` is out of the shipped checked-IR path, and
`quire-contract-model` is out of the std-off closure. Every finding is
`low`: a dead `serde` dependency (FND-001), std-only dependencies left
non-optional against the contract (FND-002), two attribute nits
(FND-003, FND-004), and a lint-precision follow-up (FND-005). From the code
side this is mergeable. Merge readiness depends on SR-12343, the gap
analysis.
