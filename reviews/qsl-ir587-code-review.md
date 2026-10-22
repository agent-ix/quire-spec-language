---
id: SR-12336
title: "Code review of PR #1230 (IR-587 S3: qsl-foundation no_std + alloc behind a default std feature)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@43bdfabe9adfd46bb52b2ca8a031ba7a176f3ed3; Makefile, qsl-foundation/Cargo.toml, qsl-foundation/src/**"
review_set: subset
---

## Summary

Ticket: IR-587. Code review with the rust-review lane over
`git diff origin/main...HEAD` (21 files, +118/-78), checked against
NFR-015, TC-1110 and TC-1111.

Checked:

- Semantics with `std` on (the default every dependent takes). The diff only
  renames paths. `std::fmt`, `cmp`, `str`, `hash`, `marker`, `num`, `iter`
  and `slice` move to `core::`, and `collections::{BTreeMap, BTreeSet,
  btree_map}` and `sync::Arc` move to `alloc::`. Those are the same items
  std re-exports. `core::error::Error` is the trait `std::error::Error`
  re-exports, so `Diagnostic`, `InvalidDigest`, `UnknownDigestDomain` and
  every `thiserror::Error` derive keep the same `Display` and `source`. No
  map type changed: the crate had no `HashMap`/`HashSet` before and has none
  now, so iteration order is unchanged.
- Feature wiring. `default = ["std"]` and `std = ["serde/std", "sha2/std",
  "thiserror/std"]`. Each of the three dependencies has default features
  off, and serde keeps `derive` and adds `alloc`. With defaults on, the
  resolved dependency features are the same as on main. With
  `--no-default-features --target thumbv7em-none-eabi`, the depth-1 tree is
  exactly `ix-trace-rs` (proc-macro), `quire-exact`, `serde`, `sha2` and
  `thiserror`, and the feature tree names no `"std"` feature.
- Crate root: `#![cfg_attr(not(feature = "std"), no_std)]` and
  `extern crate alloc;`. No `std::` path is left in `qsl-foundation/src`.
- `make build-no-std` targets the governed target. It runs `cargo build`
  and `cargo clippy -D warnings` with `-p qsl-foundation
  --no-default-features --target thumbv7em-none-eabi` in
  `$CARGO_TARGET_DIR/no-std`. `rust-toolchain.toml` already lists the
  target. It passes at the reviewed head. As a negative probe (TC-1110 step
  6), adding `use std::collections::HashMap;` to `backend.rs` made it fail
  with E0433 at the `qsl-foundation` build, as expected. The probe was
  reverted.
- `build-no-std` is in `CI_CHECKS`, so `make ci` runs it. See FND-001.
- `cargo test --locked -p qsl-foundation` passed with and without default
  features (51 + 4 tests each). `cargo clippy -p qsl-foundation
  --all-targets -D warnings` was clean in both configurations, and
  `cargo fmt -p qsl-foundation --check` was clean.
- Import ordering after the script: rustfmt accepts it. Placement and
  grouping are untidy in places (FND-002).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `build-no-std` was added to `CI_CHECKS` without updating the xtask tests that pin the aggregate. Two tests now fail, so `make ci` is red: `conformance::tests::ordinary_ci_cargo_lane_inherits_canonical_checkout` sees 9 cargo calls instead of 7, and `ci_feature_lane_tests::aggregate_reaches_default_then_all_features_without_changing_recipe_order` sees the two extra `no-std` build and clippy calls in its recorded command list. Both pass with main's Makefile. Fix: add `build-no-std` to each test's `-o` skip list, or to the expected commands | Makefile:305; xtask/src/conformance/tests.rs:342; xtask/src/ci_feature_lane_tests.rs:503-524 |
| FND-002 | low | The script left imports badly grouped. `diagnostic.rs` has its `alloc` items in two places: a group at line 6 and a separate `use alloc::collections::BTreeMap` at line 15. Line 1339 still spells out `alloc::collections::BTreeMap` even though line 15 imports it, and `stage.rs:177-178` does the same. `budget.rs:8-9` has two `use alloc` lines with `string::String, string::ToString` not nested. `bound.rs:612` puts the test module's `alloc` import directly above a doc comment, 66 lines away from that module's other imports (678-679). Merge each into one nested `alloc::{...}` group and use the imported names | qsl-foundation/src/diagnostic.rs:6,15,1339; qsl-foundation/src/diagnostic/stage.rs:177-178; qsl-foundation/src/diagnostic/budget.rs:8-9; qsl-foundation/src/bound.rs:612 |

## Verdict

Changes requested: FND-001 is high because it turns `make ci` red.

The conversion itself is correct. With `std` on, it changes no behaviour:
same traits, same collections, same dependency features. The no_std build
really checks the governed target, and it fails on a shipped `std` path,
which was measured with a negative probe. No compatibility layer was added,
nothing was vendored, and nothing is re-exported. Once FND-001 is fixed,
FND-002 is cosmetic.
