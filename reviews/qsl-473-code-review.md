---
id: SR-1370
title: "Code review of quire-spec-language PR #658: delete revision-mismatch and the handoff checksum inventory (QSL-473)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@384d6898ec47da42782f8d7fa0a6b2158c95c053; PR #658 diff against origin/main: qsl-replay/src/execute*.rs, qsl-replay/src/spine/**, qsl-foundation/src/diagnostic*.rs, qsl-cst/src/diagnostic.rs, qsl-package/src/emit/extent_agreement.rs, src/protocol_artifact/handoff.rs, src/protocol_artifact/handoff/writer.rs, tests/it/*.rs, artifacts/compiled-protocol-v{1,2}/SHA256SUMS, docs/compiled-protocol-v2.md, README.md, examples/protocol-handoff/README.md, Cargo.toml"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-116
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-357
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
---
# Code review of quire-spec-language PR #658

## Summary

Ticket: QSL-473 (slice A2, layer L2). Code review with the rust-review lane folded in.

The PR renames the three replay identity refusals (`FrameIdentity`, `ClauseIdentity`, `ScalarIdentity`) from `stale_dependency/revision-mismatch` to `stale_dependency/content-mismatch`, deletes every "catalog revision 1-draft.8" comment, deletes `SHA256SUMS` (both committed files, the writer's `write_checksum_inventory`/`collect_handoff_files`, `PUBLISHED_CHECKSUMS_FILE`, `PUBLISHED_V1_CHECKSUMS_FILE`, and the tests that verified it), and deletes `tests/it/lowering_registry_isolation.rs` (FR-079).

Checked and clean:
- `git grep revision-mismatch` and `git grep 'catalog revision'` find nothing in `*.rs` at the head (hits are only in spec/review prose; FR-087-AC-11 says the library has no `revision-mismatch`, which is true).
- No compat layer: no alias constant, no fallback reader for `SHA256SUMS`, no dual cause.
- No consumer of `SHA256SUMS` or the checksum constants remains in src/, tests/, examples/ or qsl-*/ (only the stale arch-lint exemption, FND-001).
- The sampler version check is gone on main already: `sample_request` checks the sampler's identity only (qsl-eval/src/simulation/sample.rs:280).
- `lowering_registry_isolation.rs` held only FR-079 tests. The target-list behaviour its second test checked is still covered by tests/it/integer_lowering.rs:482.
- The handoff tests that lost the checksum cross-check now check that each selected/manifest path is a published file, which is the behaviour that matters.
- The docs/compiled-protocol-v2.md edit changes the bytes `writer.rs` embeds (`CONTRACT_V2`). The committed v2 snapshot's contract digest (sha256:7e052cd4...) is now that of the old doc text. `PUBLISHED_HANDOFF`'s own doc says the snapshot is not an admissible handoff and that a consumer must write a fresh one, and no test or reader compares the snapshot with a fresh write, so this breaks nothing. Not a finding.
- The `FR-058-preserve-retry-and-partial-recovery.md` rule key in src/linking/composed/definition_source.rs:273 is not a reference to the deleted QSL FR-058 (`FR-058-detect-current-head-cross-repository-incompatibility.md`, deleted in #586). It is a QSpec protocol requirement that still exists (quire-specification `spec/functional/protocol/FR-058-preserve-retry-and-partial-recovery.md`), listed with its FR-049..FR-061 siblings. Keeping it is correct; deleting it would remove a live rule. Not a finding.

Examined:
- qsl-replay/src/execute.rs, execute/{frame,operator_parity,state_clause,tests}.rs, spine/call.rs, spine/clause/tests/{frame_replay,state_clause_replay}.rs (examined)
- qsl-foundation/src/diagnostic.rs, diagnostic/stage.rs, qsl-cst/src/diagnostic.rs (examined)
- src/protocol_artifact/handoff.rs, handoff/writer.rs (examined)
- tests/it/compiled_protocol_v2.rs, handoff_writer.rs, main.rs, deleted lowering_registry_isolation.rs (examined)
- qsl-package/src/emit/extent_agreement.rs (examined)
- tools/arch-lint/canonical_encoder.rs (examined, not in the diff)
- src/linking/composed/definition_source.rs, qsl-eval/src/simulation/sample.rs (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The arch-lint canonical-encoder exemption for `protocol_artifact::handoff::writer` still lists `write_checksum_inventory`, which this PR deletes, and its reason still names `SHA256SUMS` members. The lint fails any listed function with no hash site as stale, so `make arch-lint-canonical-encoder` (part of `make ci`) fails. Remove the function from the list and the `SHA256SUMS` clause from the reason. | tools/arch-lint/canonical_encoder.rs:228-235 |
| FND-002 | medium | The module doc now says IR's `requires-bound` agrees with QSL on the quantity record and that "the agreement test over it asserts both". No such test exists: `Measure` is not in `records()`, and the only test over it, `tc_440_a_quantity_record_is_emitted_and_reaches_ir`, accepts either `Lowered` or `RequiresBound` and never checks QSL's classification. Replace the sentence with what is true (it is emitted and IR lowers it; no agreement test covers it), or add the test. This edit is also outside the ticket's delete list. | qsl-package/src/emit/extent_agreement.rs:21-25 |
| FND-003 | low | `writer::Error::HandoffPath` lost its only constructors with `collect_handoff_files`/`write_checksum_inventory`. It is a public variant that can no longer be produced. Delete it. | src/protocol_artifact/handoff/writer.rs:393-395 |

## Verdict

Gates run at 384d6898e through locked-build.sh: `cargo test --locked --test it -- compiled_protocol_v2::` passes (16 passed, 0 failed), so the docs/compiled-protocol-v2.md edit breaks no embedded-doc test. `cargo run --locked -p arch-lint -- canonical-encoder --qsl .` FAILS with "stale exemption, `write_checksum_inventory` has no hash site: src protocol_artifact::handoff::writer" (FND-001, reproduced). The rename and the deletions are correct and complete in code, with no compat layer. FND-001 turns `make ci` red and must be fixed before merge; FND-002 is a false coverage claim; FND-003 is dead API left by the deletion. Mergeable on the code side once all three are fixed.

## Dispositions

Round 1, reviewed at 7b326716f68cfb2b7e0086c94704b9382e6175ca (rebased onto main dac0d90f).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 7b326716f: `write_checksum_inventory` and the `SHA256SUMS` clause removed from the exemption; `arch-lint canonical-encoder` re-run at the head |
| FND-002 | rejected | The finding was wrong. `tc_440_quantity_extent_agrees_with_ir_requires_bound` already existed at 384d6898e (extent_agreement.rs:560). It calls `disagreement("Measure", Some(DomainKind::Quantity), ..)`, which accepts only QSL `Unbounded` with a Quantity domain plus IR `RequiresBound`. The reviewer had read only `tc_440_a_quantity_record_is_emitted_and_reaches_ir`. The doc sentence was true; 7b326716f now names the test, which is fine. |
| FND-003 | fixed | 7b326716f: `Error::HandoffPath` deleted; no reference remains |
