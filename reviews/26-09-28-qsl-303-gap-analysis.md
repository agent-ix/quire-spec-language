---
id: SR-779
title: "QSL-303 gap analysis of PR 515 (M-6d SEAM-3 handoff deletions)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; spec/functional/FR-051-publish-checked-native-handoffs.md; spec/functional/FR-052-publish-native-temporal-evaluation-owner.md; spec/functional/FR-053-preserve-opaque-semantic-trigger-identity.md; spec/test-cases/TC-139-publish-checked-native-handoffs.md; spec/test-cases/TC-140-native-temporal-owner-boundary.md; spec/test-cases/TC-141-preserve-opaque-semantic-trigger-identity.md; spec/native-temporal/tests.md; spec/spec.md; tests/it/contract_model_architecture.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-051
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-052
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-053
    type: reviews
---
## Summary

Ticket: QSL-303. PR: quire-spec-language#515. There is no plan
bundle. The ticket's goal is ADR-011's M-6d row: delete the SEAM-3 handoffs to
IR. The delivered part (checked-predicate, temporal-subject, native-temporal
v1/v2) is fully removed from code. The undelivered part (B8/B9 emission and the
composed ProtocolClause checker) is split to QSL-316, and SR-778 confirms that
split rests on real live callers.

Trace delta, measured by diffing `#[trace]` tags between origin/main and
the PR head. Removed: TC-139 (4 tests; FR-051-AC-1..6), TC-140 (14 tests;
FR-052-AC-1..8) and TC-141 (1 test; FR-053-AC-1..6). No other requirement
lost a traced test. TC-132, TC-142, FR-048-AC-1 and FR-049-AC-9 appear only as
diff context and are still traced in `tests/it/compiled_protocol_v2.rs`.

Underspecified code: none. The diff adds no production code.
My own `make ci` run exited 0, with 6507 `ok` lines and 0 FAILED or
panicked.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The spec still requires the deleted surface. FR-051 SHALLs `quire.checked-predicate/v1` and `quire.checked-temporal-subject/v1`. FR-052 SHALLs `protocol_artifact::native_temporal::{request,result}`. FR-053 depends on FR-052's request/result. TC-140 names the deleted `tests/native_temporal_owner.rs`. The spec/native-temporal/tests.md matrix still reports FR-051/FR-052 and TC-139/TC-140 as Implemented/Passing. None of them has an implementation or a traced test any more. Retire FR-052, FR-053, TC-140 and TC-141, update the matrix and index rows, and retire FR-051's handoff ACs. One part of FR-051 is still live: FR-051-AC-6, the cycle-free production graph. It is tested by `production_graph_is_cycle_free_and_historical_ir_is_test_only` (`tests/it/contract_model_architecture.rs:11`, traced TC-139/FR-051-AC-6). That AC and its TC-139 test must be kept or moved to an owning requirement, not deleted with the rest. | spec/functional/FR-051-*.md; spec/functional/FR-052-*.md; spec/functional/FR-053-*.md; spec/test-cases/TC-139-*.md; spec/test-cases/TC-140-*.md; spec/test-cases/TC-141-*.md; spec/native-temporal/tests.md:16-18,47,91-92,122-123; spec/spec.md; tests/it/contract_model_architecture.rs:11 |

## Verdict

FAIL until FND-001 is fixed. The code deletion is complete and correct, but
the spec still states the deleted handoffs as required and implemented. Per
"spec states target design", the retirement belongs in this PR.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | low | spec/native-temporal/tests.md:20-21 says QSL-303 "deletes the handoff modules and their only test, `tests/it/compiled_protocol_v2.rs`". That file is not deleted; only the FR-051 tests inside it are (it remains for TC-138/TC-132/TC-142 coverage). The same file's line 59-60, FR-051's Status and TC-139 all say it correctly. Reword to "their tests, formerly in `tests/it/compiled_protocol_v2.rs`". | spec/native-temporal/tests.md:20-21 |

## Dispositions

Round 1.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | still-open | The fix retires the AC tables correctly: FR-051-AC-1..5, all FR-052 and FR-053 ACs, and TC-140/TC-141 follow the FR-078-AC-1/2 and TC-172/TC-201 RETIRED-by-ticket convention with ids kept; the matrix and index rows are updated; and the amended FR-051-AC-6 matches what tests/it/contract_model_architecture.rs:11 asserts. But FR-051's requirement body is untouched. Its Description (line 20) still says the compiler SHALL derive `quire.checked-predicate/v1`/`quire.checked-temporal-subject/v1`, and "Contract set and public API" (lines 22-40) still SHALLs `protocol_artifact::checked_predicate::{...}` and `temporal_subject::{...}`. The envelope, predicate, temporal and admission sections (lines 42-104) have no retirement marker either. The only live criterion, AC-6, has no body statement at all. Fix: mark those body sections RETIRED in place, following FR-068:391-406, and add one live sentence for the production-graph fact AC-6 verifies. FR-052 and FR-053 have the same issue in a milder form: the retirement appears only in a trailing Status note while the Description still SHALLs the surface, so each also needs a one-line RETIRED banner under Description. |
| FND-002 | still-open | New this round (see New findings); no fix yet. A one-line reword of spec/native-temporal/tests.md:20-21. |

Round 2.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | FR-051's Description and all five body sections (Contract set and public API, Common closed envelope, Predicate document, Temporal-subject document, Admission/reading/limits) carry a "(RETIRED by QSL-303 M-6d)" heading and a bold retirement note in the FR-068:391-406 style, with the old text kept as history. A new live paragraph states the AC-6 fact, and it matches what tests/it/contract_model_architecture.rs:11-103 asserts: the `quire_contract_ir` alias resolves to `quire-contract-model` as a normal dependency, and no `quire-contract-ir` package is in the normal-dependency closure. FR-052:18 and FR-053:15 each have the one-line RETIRED banner under Description that round 1 asked for. |
| FND-002 | fixed | spec/native-temporal/tests.md:20-21 now reads "deletes the handoff modules and their tests, formerly in `tests/it/compiled_protocol_v2.rs`", which agrees with line 59-60. |
