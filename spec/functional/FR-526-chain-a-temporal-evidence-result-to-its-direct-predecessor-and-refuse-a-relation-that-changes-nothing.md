---
id: FR-526
title: "Chain a temporal evidence result to its direct predecessor and refuse a relation that names another obligation or changes nothing"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-033
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-052
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-053
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-516
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-522
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-523
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-524
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-499
    type: references
---
# FR-526: Chain a temporal evidence result to its direct predecessor and refuse a relation that names another obligation or changes nothing

## Description

When a temporal evidence request corrects an earlier result, the spine evaluator
retains the exact direct predecessor and gives the result the next lineage number.
It refuses a predecessor that belongs to another obligation, that belongs to
another trigger, or that the request leaves unchanged. The predecessor stays
unchanged.

This is the spine successor of FR-052-AC-12 and of the lineage, correction and
semantic-trigger-identity rows of the legacy `ProtocolLedger`. FR-516 gives the
relation its three forms and the first refusals; this requirement states the
whole rule.

## Use case

A monitor decided an obligation on the evidence it had, then receives a late
observation. It evaluates again, naming the first result as the one it
corrects. The evaluator accepts the correction because the evidence changed and
the obligation is the same. A resend of the same evidence, or a correction
naming another obligation's result, is refused, and the first result is not
touched.

## Semantic authority and boundary

FR-052 owns the correction relation and FR-053 the trigger identity. The legacy
evaluator took the complete predecessor result by value; FR-516 names it by its
`result_identity`. Each refusal below needs the predecessor's obligation, trigger
identity and input, so `prev` is the predecessor `EvidenceResult` value. This
requirement adds `input_identity` to `EvidenceResult` (FR-522's identity without
the relation) so that an unchanged request is decidable. Both are inferences from
the legacy value-taking API.

## Inputs

`relation` of `EvidenceRequest` (FR-516): `Original`, `Superseding(prev)` or
`Invalidating(prev)`, with `prev` an `EvidenceResult`.

## Outputs

`lineage: u64`: 0 for `Original`, `prev.lineage + 1` otherwise.
`relation` echoed with `prev.result_identity`. Each refusal below is
`Err(CallFailure::Admission)` with an empty charge log and no result; a lineage
stop is `Incomplete` (FR-523).

## Behavior

- The evaluator SHALL derive truth, settlement and support from the request's own
  inputs for `Original`, `Superseding` and `Invalidating` alike. The relation
  records how the result stands to the predecessor and does not change what is
  derived.
- `Superseding` records that the result replaces the predecessor and
  `Invalidating` records that it retracts the predecessor, and the evaluator SHALL
  retain the kind in the result.
- The evaluator SHALL refuse a predecessor whose authenticated binding names
  another package or another declaration than the call's clause.
- The evaluator SHALL refuse a predecessor whose trigger identity differs from the
  request's trigger identity.
- The evaluator SHALL refuse a predecessor whose `correspondence` differs from the
  request's.
- The evaluator SHALL refuse a request whose `input_identity` equals the
  predecessor's.
- The evaluator SHALL refuse a predecessor that is the result the request would
  produce (equal `input_identity`), so a self-edge is the unchanged case.
- The evaluator SHALL compute `lineage` with checked `u64` arithmetic, and when
  the sum does not fit or exceeds the `lineage` ceiling (FR-523) it SHALL return
  `Incomplete` naming `lineage`.
- The evaluator SHALL leave the predecessor value, and every earlier result,
  unchanged.
- Equal requests with equal relations SHALL give equal results.
- FR-516's reading of "an identity mismatch" is these three comparisons (package
  and declaration, trigger identity, correspondence): they are the identity of an
  obligation instance (FR-053).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-526-AC-1 | An `Original` request returns `lineage` 0 and relation `Original`. | Test (TC-1321) |
| FR-526-AC-2 | A `Superseding(r1)` request with changed inputs over the same package, clause, trigger and correspondence returns `lineage` 1, relation `Superseding` naming `r1.result_identity`, and `r1` byte-equal to its value before the call. | Test (TC-1321) |
| FR-526-AC-3 | An `Invalidating(r1)` request under the conditions of AC-2 returns relation `Invalidating` naming `r1.result_identity` and the same `lineage`, and its truth, settlement and support equal those of a `Superseding(r1)` request with the same inputs. | Test (TC-1321) |
| FR-526-AC-4 | A request whose `input_identity` equals `r1`'s, under `Superseding(r1)` and under `Invalidating(r1)`, returns `Err(CallFailure::Admission)` with an empty charge log. | Test (TC-1321) |
| FR-526-AC-5 | A predecessor of another clause of the same package, and a predecessor of the same clause in another package, each return `Err(CallFailure::Admission)` with an empty charge log. | Test (TC-1321) |
| FR-526-AC-6 | A predecessor whose trigger identity differs from the request's in one byte returns `Err(CallFailure::Admission)`, and so does one whose `correspondence` differs. | Test (TC-1321) |
| FR-526-AC-7 | A predecessor at `lineage` equal to the `lineage` ceiling returns `Incomplete` naming `lineage` with no truth, a predecessor one below the ceiling completes, and a predecessor at `u64::MAX` returns `Incomplete` naming `lineage` and does not wrap. | Test (TC-1321) |
| FR-526-AC-8 | A chain of three corrections `r1`, `r2`, `r3` returns `lineage` 1, 2, 3, each naming its direct predecessor, with `r1` and `r2` unchanged after `r3`. | Test (TC-1321) |
| FR-526-AC-9 | The same request and relation evaluated twice gives equal results, charge log included. | Test (TC-1321) |
| FR-526-AC-10 | An empty trigger identity is refused at construction of `SemanticTriggerIdentity`, and a non-empty identity round-trips byte for byte through a result and its predecessor chain. | Test (TC-1321) |

## Dependencies

- FR-052-AC-12 and the `ProtocolLedger` rows (the legacy behavior this requirement
  succeeds), FR-053 (identity), FR-516 (the relation and the entry), FR-522
  (`input_identity`, the authenticated binding), FR-523 (`lineage`), FR-524.
- Feeds QSL-1545 (FR-516-AC-6).

## References

- Linear QSL-1503; deletion-table row B31.
