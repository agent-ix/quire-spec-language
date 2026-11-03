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
    type: references
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
retains the exact direct predecessor and gives the result the next lineage
number. It refuses a predecessor that is not the latest result of the same
obligation instance, a request that leaves the evaluation input unchanged, and a
request whose correspondence differs from the predecessor's. The predecessor
stays unchanged.

This is the spine successor of FR-052-AC-12 and of the lineage, correction and
semantic-trigger-identity rows of the legacy `ProtocolLedger`. FR-516 gives the
relation its three forms and the first refusals; this requirement states the
whole rule.

## Use case

A monitor decided an obligation on the evidence it had, then receives a late
observation. It evaluates again, naming the first result as the one it
corrects. The evaluator accepts the correction because the evidence changed and
the first result is the latest of that obligation. A resend of the same evidence,
a correction naming a result that a later correction already superseded, or a
correction naming another obligation's result, is refused, and the first result is
not touched.

## Semantic authority and boundary

FR-052 owns the correction relation and FR-053 the trigger identity.

The legacy `ProtocolLedger` enforced "the latest result of this obligation" from
history it kept. A stateless call cannot know the latest result, so the spine
keeps that history in the caller-held `EvidenceLedger` of FR-524, in the way
FR-502 keeps progress in the `ProgressLedger`. The ledger's `results` table holds,
for each obligation instance (the package identity, the clause and the trigger
identity), one flat entry for the latest result: its `result_identity`, its
`input_identity` (FR-522's identity without the relation), its correspondence
identity and its lineage. Entries hold no recursive chain and no result bytes.
Only the evaluator writes an entry.

## Inputs

`relation` and `ledger` of `EvidenceRequest` (FR-516). `relation` is `Original`,
`Superseding(prev)` or `Invalidating(prev)`, with `prev` the `result_identity` of
the predecessor.

## Outputs

`lineage: u64`: 0 for `Original`, the predecessor entry's lineage plus 1 otherwise.
`relation` echoed with `prev`. `input_identity`. The updated `ledger`.

Each refusal below is `Err(CallFailure::Admission)` with an empty charge log, no
result and the `cause` named in the criterion. A lineage stop is `Incomplete`
(FR-523).

## Behavior

- The evaluator SHALL derive truth, settlement and support from the request's own
  inputs for `Original`, `Superseding` and `Invalidating` alike.
- `Superseding` records that the result replaces the predecessor, and the
  evaluator SHALL retain that kind in the result.
- `Invalidating` records that the result retracts the predecessor, and the
  evaluator SHALL retain that kind in the result.
- When a request carries `Superseding` or `Invalidating` without a `ledger`, the
  evaluator SHALL refuse it with cause `no-ledger`.
- The evaluator SHALL look up the ledger's entry for the request's package, clause
  and trigger identity.
- When the ledger holds no entry for that key, the evaluator SHALL refuse the
  relation with cause `predecessor-unknown`.
- When `prev` is not the `result_identity` of that entry, the evaluator SHALL refuse
  the relation with cause `predecessor-not-latest`.
- When the request's `input_identity` equals the entry's, the evaluator SHALL
  refuse the relation with cause `unchanged-input`.
- When the request's correspondence differs from the entry's, the evaluator SHALL
  refuse the relation with cause `correspondence-changed`.
- The evaluator SHALL compute `lineage` with checked `u64` arithmetic.
- When the sum does not fit or exceeds the `lineage` ceiling (FR-523), the
  evaluator SHALL return `Incomplete` naming `lineage`.
- When a call succeeds, the evaluator SHALL record the result's entry as the
  latest for its key, including for `Original`.
- The evaluator SHALL leave every earlier result and the replaced entry's value as
  the caller holds them.
- An `Original` request SHALL start a new chain at lineage 0 even when the ledger
  holds an entry for its key.
- Equal requests with equal relations and equal ledgers SHALL give equal results
  and equal ledgers.
- The `EvidenceLedger` SHALL have no public constructor of entries.
- A trigger identity that differs from the predecessor's has no entry under the
  request's key and is refused with `predecessor-unknown`, so a correction cannot
  change the trigger identity (FR-053).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-526-AC-1 | An `Original` request returns `lineage` 0 and relation `Original`, and the returned ledger holds its entry for the request's key. | Test (TC-1321) |
| FR-526-AC-2 | A `Superseding(r1)` request with changed inputs, the ledger from `r1`'s call, and the same package, clause, trigger and correspondence returns `lineage` 1 and relation `Superseding` naming `r1.result_identity`. | Test (TC-1321) |
| FR-526-AC-3 | After the AC-2 call, `r1` is byte-equal to its value before the call and the returned ledger holds the new entry as the latest. | Test (TC-1321) |
| FR-526-AC-4 | An `Invalidating(r1)` request under the conditions of AC-2 returns relation `Invalidating` and the same `lineage`. | Test (TC-1321) |
| FR-526-AC-5 | The truth, settlement and support of the AC-4 result equal those of a `Superseding(r1)` request with the same inputs. | Test (TC-1321) |
| FR-526-AC-6 | A `Superseding(r1)` request and an `Invalidating(r1)` request with `r1`'s `input_identity` each return `Err(CallFailure::Admission)` with cause `unchanged-input` and an empty charge log. | Test (TC-1321) |
| FR-526-AC-7 | A `Superseding` request without a `ledger` returns `Err(CallFailure::Admission)` with cause `no-ledger`. | Test (TC-1321) |
| FR-526-AC-8 | With a ledger holding `r1` and `r2` (`r2` superseding `r1`), a `Superseding(r1)` request returns `Err(CallFailure::Admission)` with cause `predecessor-not-latest`, and a `Superseding(r2)` request is accepted. | Test (TC-1321) |
| FR-526-AC-9 | A request for another clause of the same package naming `r1`, and a request for the same clause in another package naming `r1`, each return `Err(CallFailure::Admission)` with cause `predecessor-unknown`. | Test (TC-1321) |
| FR-526-AC-10 | A request whose trigger identity differs from `r1`'s in one byte, naming `r1`, returns `Err(CallFailure::Admission)` with cause `predecessor-unknown`. | Test (TC-1321) |
| FR-526-AC-11 | A request whose `correspondence` differs from `r1`'s, naming `r1`, returns `Err(CallFailure::Admission)` with cause `correspondence-changed`. | Test (TC-1321) |
| FR-526-AC-12 | A predecessor at a `lineage` equal to the `lineage` ceiling returns `Incomplete` naming `lineage` with no truth, and a predecessor one below the ceiling completes. | Test (TC-1321) |
| FR-526-AC-13 | A predecessor entry at lineage `u64::MAX` returns `Incomplete` naming `lineage` and does not wrap. | Test (TC-1321) |
| FR-526-AC-14 | A chain of three corrections `r1`, `r2`, `r3` returns `lineage` 1, 2, 3, each naming its direct predecessor, with `r1` and `r2` unchanged after `r3`. | Test (TC-1321) |
| FR-526-AC-15 | An `Original` request over a ledger that holds an entry for its key returns `lineage` 0 and replaces that entry. | Test (TC-1321) |
| FR-526-AC-16 | The same request, relation and ledger evaluated twice give equal results and equal ledgers, charge log included. | Test (TC-1321) |
| FR-526-AC-17 | A compile-fail test that constructs an `EvidenceLedger` entry outside the evaluator fails to compile. | Test (TC-1321) |
| FR-526-AC-18 | An empty trigger identity is refused at construction of `SemanticTriggerIdentity`. | Test (TC-1321) |
| FR-526-AC-19 | A non-empty trigger identity is echoed byte for byte by every result of a three-correction chain. | Test (TC-1321) |

## Dependencies

- FR-052-AC-12 and the `ProtocolLedger` rows (the legacy behavior this requirement
  succeeds), FR-053 (identity), FR-522 (`input_identity`, the authenticated
  binding), FR-523 (`lineage`), FR-524 (the `EvidenceLedger` value).
- FR-502 is the pattern of the caller-held ledger.
- FR-516 carries the relation and the entry and is a reference, not a dependency,
  of this requirement.

## References

- Linear QSL-1503; deletion-table row B31.
