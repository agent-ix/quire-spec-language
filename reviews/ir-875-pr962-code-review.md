---
id: SR-12880
title: IR-875 QSL PR 962 code-review and Rust review
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; Cargo.lock; qsl-package/src/checked.rs; qsl-package/src/emit.rs; qsl-package/src/emit/tests.rs; qsl-package/src/emit/tests/fixtures/owned_projection_wire.json; src/population_fixtures.rs (context: other caller of the changed IR API)"
review_set: subset
---

## Summary

Ticket: IR-875. Non-author diff-scoped review of agent-ix/quire-spec-language#962 at head 3a67aaa842c305f0caacb34dbbd64dc418963ff1 (`git diff origin/main...HEAD`, merge base 7c5659dc9). The code-review and Rust-review lanes are folded into this one artifact.

The PR moves `qsl-package` emission from IR's owned `CheckedPackageIdentityPreimageV2` (which cloned every node body into `identity_projection`) to IR's borrowed `CheckedPackageIdentityPreimageRefV2` over `semantic_graph.nodes`. It also bumps the `quire-contract-model` git pin in `Cargo.lock` from 30c3e4e to fdef3c5 and adds an emitted-bytes golden.

Measured by the reviewer:

- The golden `owned_projection_wire.json` (8034 bytes) and `package_id` f2332c7e... are what the old producer emits. Commit 9958758 (test and fixture only, with IR 30c3e4e and the old owned-preimage emitter) passes `emitted_package_preserves_the_owned_projection_golden` and `package_id_matches_its_golden_vector`. The head passes the same tests, so bytes and identity are unchanged.
- In the fixture, every `identity_projection` entry equals its graph node minus `occurrences`, and all three nodes carry an occurrence. The golden would therefore catch occurrences leaking into the projection.
- `cargo test --locked -p qsl-package` at head: 187 unit and 12 integration tests passed.
- `cargo fmt --check -p qsl-package`: clean.
- `cargo check --locked --workspace --all-targets --all-features` at head fails with E0308 in `src/population_fixtures.rs:460` (FND-001). On main, IR 30c3e4e's `encode_checked_package_v2` takes `&CheckedPackageIdentityPreimageV2`, so main compiles.

## Examined units

- **qsl-package/src/emit.rs `emit_package_inner`** (`examined`): `let preimage = CheckedPackageIdentityPreimageRefV2 { version: IDENTITY_PREIMAGE_V2, edition: &edition, ..., identity_projection: &semantic_graph.nodes };` passed to `EmittedPackage::new` and `encode_checked_package_v2`. Clean: the owned per-node projection clone is gone, and the borrow lifetimes are sound.
- **qsl-package/src/checked.rs `EmittedPackage::new`** (`examined`): the signature now takes `&CheckedPackageIdentityPreimageRefV2<'_>`. Its docs are updated. Clean.
- **qsl-package/src/checked.rs `package_id_matches_its_golden_vector`** (`examined`): rebuilt over the borrowed view, from a semantic node derived from the golden projection with `occurrences: []`. Clean.
- **qsl-package/src/emit/tests.rs `emitted_package_preserves_the_owned_projection_golden`** (`examined`): `#[trace("TC-416", "FR-087-AC-3")]`; it asserts the bytes, the `package_id`, and a Verified read-back. See FND-002.
- **Cargo.lock quire-contract-model 30c3e4e -> fdef3c5** (`examined`): see FND-001.
- **src/population_fixtures.rs** (`context_only`): the second workspace caller of `encode_checked_package_v2`. The diff does not touch it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The Cargo.lock bump to IR fdef3c5 changes `encode_checked_package_v2` to take `&CheckedPackageIdentityPreimageRefV2`. `src/population_fixtures.rs` still passes the owned `CheckedPackageIdentityPreimageV2`, so the root crate fails to compile (E0308) under `--features population-fixtures`. That breaks every `--all-features` gate: Makefile `ci-all-features` and `ci-docs`, and the `.github/workflows/ci.yml` clippy, test and doc steps. The PR's checks ran default features only. The same function also still clones every node body via `nodes.iter().map(Into::into)`, the duplication IR-875 removes. | src/population_fixtures.rs:441-470; Cargo.lock:1600 |
| FND-002 | low | The new golden test is tagged `TC-416` with `FR-087-AC-3`. TC-416's scope lists only FR-093 criteria, and FR-087-AC-3 names TC-253 as its verifying case (the existing `package_id_matches_its_golden_vector` uses TC-253). The test-case label contradicts the spec's TC-to-AC allocation. | qsl-package/src/emit/tests.rs:261 |

## Verdict

FAIL for this method because of FND-001: with the PR as it stands, the workspace does not build under `--all-features`. The emitter change itself is correct. It is minimal and byte-identical to the old producer (verified independently against the pre-change commit), and it removes the per-node owned projection clone. FND-001 is fixed by migrating `population_fixtures::retarget` to the borrowed view, built over `graph.nodes` and used for both the `sha256` and the encode, which also removes that function's body clones. Then re-run `cargo check --workspace --all-targets --all-features`. FND-002 is fixed by retagging the test with `TC-253`.

Gates run: `cargo test -p qsl-package` passed. `cargo fmt --check` was clean. `cargo check --all-features` failed (FND-001). Clippy with `--all-features` was not run separately, because it cannot pass while the crate does not compile. `make ci` was not run: main is red at the known qsl-forms `protocol_control_forms` parser test, and cargo-deny rejects the Makefile `--config` on main too. Neither is attributed to this PR. Main has moved to a0ffcac since the merge base. A merge-tree with it is conflict-free, and its `catalog_selections` change does not affect the no-layer golden package by inspection, but a rebase test run is still advised before merge.

## Dispositions

Disposition pass round 1, re-measured at rebased head 7d66d8d4bc9daf73003fa92ac337b7fcd7dcc7cd (merge base bec89ae). `git range-diff` shows the three reviewed commits unchanged and two new commits: 08d096cb8 (the fix) and 7d66d8d4b (fmt plus the review records). The two copied `reviews/` files are byte-identical to the review-pass artifacts and pass `quire validate`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `retarget` now builds `CheckedPackageIdentityPreimageRefV2` over `&graph.nodes` with no per-node clone, and uses it for both `sha256` and `encode_checked_package_v2`. `cargo check --locked --workspace --all-targets --all-features` passes. The 10 `population_fixtures` tests and 7 `handoff_writer` tests pass with `--all-features`. The 8 population fixture files written at head are byte-identical to main's (bec89ae, IR 30c3e4e). |
| FND-002 | fixed | The test is now tagged `#[trace("TC-253", "FR-087-AC-3")]`. |

No new findings in the round-1 delta. The golden tests still pass at head; the fixture is unchanged at 8034 bytes and the asserted `package_id` is still f2332c7e....
