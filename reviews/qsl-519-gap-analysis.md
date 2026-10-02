---
id: SR-1206
title: "Gap analysis of quire-spec-language PR #593: FR-265, FR-266, FR-268, FR-269 acceptance criteria to tests"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language PR #593 diff against origin/main; ACs FR-265-AC-1..6, FR-266-AC-1..3, FR-268-AC-1..4, FR-269-AC-1..3, with FR-098 and FR-122 regression tests touched by the diff; tests in qsl-replay/src/spine/clause/tests/witness.rs, state_clause_replay.rs, execute/tests.rs, result.rs"
review_set: subset
---
# Gap analysis of quire-spec-language PR #593

## Summary

Ticket: QSL-519. Each AC from FR-265-AC-1 to FR-269-AC-3 has a `#[trace]`
test in `qsl-replay/src/spine/clause/tests/witness.rs`, under TC-740, 741,
743 and 744. The function-replay and frame-replay records are now absent
under ADR-031 SW-7. The FR-098 and FR-122 regression tests assert that
absence. All 16 TC-74x tests pass at this head.

Test-oracle strength:
- FR-265-AC-1 counts `collection.visit` charges, two against a duplicate
  element after the stop. This is a real oracle that no element after the
  stop is evaluated.
- The records are asserted whole, including the occurrence key resolved
  from the checked graph.
- FR-269-AC-3 round-trips the cause and mutates the document for each
  refusal: an unknown failure tag, a missing index and a `null` index.
- The bound test builds a record past `MAX_ENCODED_BYTES`.

These oracles are strong.

The team-leader rulings are not raised:
1. Built-in-claim collections replace the `ConfigVersion` population.
2. The `UndefinedEvaluation` mapping is tested over a synthetic evaluation.
3. Only the cause round-trips here; the result-level round trip is E19b's.

## Verdict

Coverage holds for the paths the fixture reaches, but one value-path branch
is untested (FND-001), and FR-269-AC-2's last mapping is asserted one layer
short (FND-002).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test exercises a domain that is not a built literal. Every `CLAUSES` domain is a collection literal passed through the identity functions `ints`/`refs`. So the `ValuePathSubject::Object` member path (`note_member`), the pre-observation selection inside it, and the no-provenance fallback (`provenance_of`) are all unexercised. That is FR-265's "value path SHALL be the QSpec FR-207 runtime value path" for a stored collection. Ruling 1 replaces the population, which a state clause cannot name; a model member of collection type can be named, and a fixture model with such a member would cover it. | qsl-eval/src/value/expression/evaluate.rs:2102 |
| FND-002 | low | FR-269-AC-2 says FR-268-AC-3's failures map to `DisagreementCause::Witness { failure: Separation { step, reason } }`. The test asserts only the intermediate `SeparationOutcome`. The `compare_witness` arm that turns `SeparationOutcome::Failed` into `WitnessFailure::Separation`, with `given` and `derived` attached, has no test. | qsl-replay/src/execute/state_clause.rs:411 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | The record-field path this round adds has no test. That covers `note_field`, `Provenance::field`, `ValuePathStep::Field` and the codec's `field` step. No fixture reads a collection through a field of a record stored on an object, and no codec test carries a `field` step. | qsl-eval/src/value/expression/evaluate.rs:2136 |

## Dispositions

Round 1, reviewed at `0a2609e8f64f4e7d05b634a4567b3b21120b216e` (diff `4fb4e69b..0a2609e8`).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 0a2609e8f64f4e7d05b634a4567b3b21120b216e |
| FND-002 | fixed | 0a2609e8f64f4e7d05b634a4567b3b21120b216e |

Round 2, reviewed at `7226baa9eccc241574fec9b7a5706b8c45f1952d` (rebased onto main f5a2cc1d; round-2 changes `0fc3c91fa..7226baa9e`).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-003 | fixed | 2881f26f234f71461944a429e82e5549fac0e706 |
