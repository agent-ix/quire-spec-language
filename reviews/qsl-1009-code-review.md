---
id: SR-12362
title: "code review and Rust review of QSL-1009 shipped identity_projection removal (QSL PR #1300)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@b5a1d14b0c74899b5df276c9f234bf0890f5e6d4; Cargo.lock, qsl-package/src/checked.rs, qsl-package/src/checked_v2.rs, qsl-package/src/checked_v2/tests.rs, qsl-package/src/emit/tests.rs, qsl-package/src/emit/tests/fixtures/owned_projection_wire.json, qsl-package/src/emit/tests/owners.rs, qsl-package/src/emit/tests/receiver_frames.rs, qsl-package/src/emit/tests/temporal_clause.rs, xtask/src/conformance.rs, xtask/tests/fixtures/checks.rs; context: qsl-package/src/emit.rs:1307, src/population_fixtures.rs:456, quire-contract-model @bbc581f5"
review_set: subset
---
# SR-12362: code review and Rust review of QSL-1009 shipped identity_projection removal (QSL PR #1300)

## Summary

Ticket: QSL-1009 (STD-288 follow-up, after IR-975). The rust-review lane is folded into this file.

The PR moves `quire-contract-model` to IR `bbc581f5` (IR-975). At that commit the V2 wire
`identity_preimage` carries only the seven selections. The package-id hash input adds an
`identity_projection` that IR's `Encode for CheckedPackageIdentityPreimageRefV2` derives from
`semantic_graph.nodes` (`ProjectionFields::from_node`, which drops occurrences). QSL's reader
(`checked_v2.rs` `canonical_preimage`) now builds that borrowed view from the admitted selections
and `package.graph().nodes`. The test fixtures stop writing a wire projection.

What was measured:

- **The package-id preimage is byte-identical.** The `checked.rs` golden keeps its hand-written
  RFC 8785 `TEXT` and `DIGEST` (`6dfea2b0…`) unchanged, and it now goes through the borrowed
  view. The emitter golden `owned_projection_wire.json` keeps `package_id` `f2332c7e…`. A Python
  recompute outside Rust gave these results:
  - The main-branch fixture's `identity_preimage`, with its projection, sorted-key JCS hashes to
    `f2332c7e…`.
  - The PR fixture's selections plus `semantic_graph.nodes` minus `occurrences` hash to the same
    value.
  - The derived projection equals the old shipped one element for element.
  - The two fixtures differ only in the removed member.
- **The `IdentityDivergedFromIr` cross-check is real.** As a mutation probe, `canonical_preimage`
  was made to drop the last graph node. Every positive read in `checked_v2::tests` then refused
  `IdentityDivergedFromIr`: QSpec positive fixtures, XD-01, the union vectors, the ceiling and
  frame cases. The probe was reverted.
- **`canonical_preimage` is correct.** Its ceiling is `artifact_bytes`, the value `for_ir` passes
  to IR as `bytes`, so both sides hash under one bound. The hash input still fits under that
  ceiling: it is the selections plus the graph nodes without occurrences, and the wire holds
  both.
- **`emit.rs:1307` and `population_fixtures.rs:456` need no change.** Both already build
  `CheckedPackageIdentityPreimageRefV2 { identity_projection: &graph.nodes, .. }`. At
  `bbc581f5`, `encode_checked_package_v2` writes `encode::SelectionFields` (the seven
  selections) as the wire `identity_preimage`, so neither site emits a projection any more.
- **The lock change is justified.** Only `quire-contract-model` moves (`be1ae59` → `bbc581f5`).
  `quire-verification-contracts` (`e1ede119`) is unchanged from `main`, and it has no dependency
  on `quire-contract-ir`, so holding it back keeps the bump scoped.
- **The xtask entry is correct.** The `CHECKS` entry in `xtask/src/conformance.rs` and its
  process-double row in `xtask/tests/fixtures/checks.rs` (array length 13 → 14) name the new test,
  and the summary string matches its `println!`.
- **The tests fail when they should.** The `owners.rs` assertion that the wire
  `identity_preimage` has no `identity_projection` would have failed against the pre-bump
  encoder. The new FR-322-AC-100 test does three things: it checks the refusal code and locus
  for both QSpec cases, it checks that removing the member restores the base exactly, and it
  checks the published `package_id` with QSL's JSON-side derivation, which is independent of
  IR's encoder.
- **Runs.** These ran at the reviewed head with `QSPEC_DIR` at QSpec `b3f3c803`:
  - `cargo test -p qsl-package --lib`: 237 passed.
  - The conformance subset passed, and the new test printed "2 shipped-projection cases refused
    and admitted once removed".
  - `cargo clippy -p qsl-package -p xtask --all-targets -D warnings` was clean.
  - `cargo test -p xtask`: 202 and 16 passed.
  - `cargo fmt --check` was clean.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The L2 comment says the cross-check compares against "a value IR derived independently". IR now runs the same `Encode for CheckedPackageIdentityPreimageRefV2` over the same admitted selections and nodes. So the check guards QSL's own preimage construction and `PackageId::of_preimage` against `quire_canonical::sha256`, which the mutation probe confirms. It is not an independent derivation. The module doc's example ("if IR's `digest_json` were to switch to a real JCS encoder") is stale: both sides already use `quire-canonical`. Reword both to state what the check catches. | qsl-package/src/checked_v2.rs:693-699, qsl-package/src/checked_v2.rs:15-17 |
| FND-002 | low | The `owners.rs` module doc was edited in place but not reflowed. Line 4 is now 122 characters (`quire.structural-node/v1` preimage carries one. Each key is recomputed from the wire node alone, and every package is), while its neighbours wrap at about 80. | qsl-package/src/emit/tests/owners.rs:4 |

## Verdict

No high or medium findings in the code. Both findings are low: comment and doc wording. The
package-id preimage is byte-identical to the pre-IR-975 input for the pinned vectors (the
`checked.rs` golden and the emitter golden). The cross-check still refuses a diverged
projection. No production emitter site needs a change. Mergeable once the gap-analysis medium
(SR-12363) is addressed; the two lows here can ride in the same fix round.
