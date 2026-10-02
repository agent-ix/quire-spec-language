---
id: SR-1202
title: "QSL-496 gap analysis of PR #591 (FR-303, TC-796)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@08430f21bacf87c9708c2ef9eeeaa7d5a60792d8; PR #591 diff against origin/main; spec/functional/FR-303-keep-the-model-correspondence-one-to-one.md; spec/tests.md (TC-796)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-303
    type: reviews
---
## Summary

Ticket: QSL-496. PR: quire-spec-language#591.

Trace:
- FR-303-AC-1: `a_second_declaration_for_a_node_is_refused` (identity.rs:617)
  asserts the exact `NodeRebound` naming n, d1, d2, and that n still resolves to d1.
- FR-303-AC-2: `a_second_node_for_a_declaration_is_refused` (identity.rs:636)
  asserts the exact `DeclarationRebound`, that n2 resolves to none, and that n1
  is kept.
- FR-303-AC-3: `recording_the_same_pair_twice_keeps_one_entry`
  (identity.rs:657) compares by equality against a single-record
  correspondence.
- FR-303-AC-4: `keying_a_second_declaration_onto_a_recorded_node_refuses`
  (lowering/model/tests.rs:813) asserts the fault value, `runtime_invariant`,
  `established-invariant-broken` and category internal failure. It is accepted
  at the lowering level per the team-leader ruling.
- Behavior "compile SHALL stop the check": `model_node`'s `?` propagates the
  fault out of lowering. Every production line in the diff traces to FR-303.

The oracles assert exact values and are not tautologies. A `record` that kept
overwriting fails AC-1 and AC-2.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-303-AC-1 and AC-2 say the record "returns `runtime_invariant`/`established-invariant-broken`". The tests tagged to them assert only the `CorrespondenceConflict` value, because `record` returns a conflict with no catalog code. The code is asserted only for `NodeRebound`, through the AC-4 lowering test, and `DeclarationRebound`'s code is asserted nowhere. Behaviour is correct, since `KeyFault::CorrespondenceConflict(_)` maps both variants the same way. The AC wording and the tagged oracles disagree on where the code comes from. Either say in AC-1/AC-2 that the record names both pairs and the S3 fault carries the code, or add the code assertion for `DeclarationRebound`. | spec/functional/FR-303-keep-the-model-correspondence-one-to-one.md:61-62; qsl-semantics/src/check/identity.rs:617-653 |

## Verdict

Mergeable apart from one low, non-blocking wording or oracle finding.
