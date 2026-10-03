---
id: FR-269
title: "Settle a witness disagreement as a typed DisagreementCause::Witness"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-010
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-031
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-072
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-351
    type: depends_on
---
# FR-269: Settle a witness disagreement as a typed DisagreementCause::Witness

## Description

FR-072's `DisagreementCause` SHALL gain one variant, `Witness`, for a
replay whose verdict agrees with the proved one but whose separating
witness does not: the record differs from the re-derived one, is present on
one side only, or fails the separation check (ADR-031 SW-12, SW-13). Like
`Verdicts` and `NoValue`, it settles `inconclusive` and is never repaired.

## Inputs

- The payload's record and the re-derived record, each
  `Option<SeparatingWitnessRecord>`.
- For a separation-check failure, the failing step.

## Outputs

`DisagreementCause::Witness` with:

- `proved` and `replayed`: the two verdicts, which are equal;
- `given`: the payload's record, or its absence;
- `derived`: the re-derived record, or its absence;
- `failure`: `Mismatch` when the records differ or one is absent, or
  `Separation { step, reason }` naming the separation-check step that
  failed, `Quantifier`, `Domain`, `Element` or `Body` (FR-268 steps 1 to 4),
  and why: `Unmet` when the step's requirement does not hold,
  `UndefinedEvaluation { where, cause }` when the step's evaluation ends
  `undefined`, or `Refused(record)` when it is refused. An exhausted meter is
  not a `Witness` failure; it settles `NoValue` (FR-268).

## Behavior

- `DisagreementCause::proved` and `replayed` SHALL return the `Witness`
  variant's verdicts, matched with no `_` arm.
- A `Witness` cause SHALL settle `inconclusive`. A `Witness`-arm result with
  this cause SHALL carry no QSpec FR-351 record of its own, so no consumer reads a
  disputed record as evidence.
- The cause SHALL serialize and read back with `given` and `derived` each
  present in full or absent (QSpec FR-351 has no partial record), and a
  reader SHALL refuse a `Witness` cause whose `failure` is not one of the
  values above, and a record whose deciding element is a record or tuple
  whose slots do not fit its declaration's shape (QSpec FR-351-AC-5).
- Two results that differ only in their `Witness` cause's `given` or
  `derived` record SHALL compare unequal.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-269-AC-1 | FR-268-AC-2's index-2 replay settles `inconclusive` with `Witness { proved: violation, replayed: violation, given: <index-2 record>, derived: <index-1 record>, failure: Mismatch }`, and its `Witness`-arm result carries no QSpec FR-351 record; the no-record replay holds `given` absent and `derived` the index-1 record. | Test (TC-744) |
| FR-269-AC-2 | FR-268-AC-3's failing records map to `failure` `Separation { Quantifier, Unmet }`, `Separation { Element, Unmet }` (three times) and `Separation { Body, Unmet }`; its undefined-domain, undefined-body and refused-domain variants map to `Separation { Domain, UndefinedEvaluation }`, `Separation { Body, UndefinedEvaluation }` and `Separation { Domain, Refused }`, each `UndefinedEvaluation` naming the expression and its undefined cause. | Test (TC-744) |
| FR-269-AC-3 | A replay result holding FR-269-AC-1's cause round-trips through serialize and read to an equal result; the same document with `failure` set to an undefined value refuses, and one with `given` missing its `index` refuses; two results that differ only in `derived` compare unequal. | Test (TC-744) |
| FR-269-AC-4 | A deciding element whose value or type nests 100,000 levels round-trips through the cause codec on a 512 KiB stack: the writer is `quire-canonical`'s, the reader keeps the document on the heap (`quire_canonical::read`) and decodes each deciding element from it on an explicit stack, and the element reads back and encodes to the same document. No depth refuses (ADR-030 D-1). | Test (TC-744) |

## Status

The cause's own codec round-trips here. The round-trip of a whole replay
result holding the cause lands with the `native-run-result/2` wire
([FR-267](FR-267-write-run-results-as-native-run-result-2.md)).

## Dependencies

- [ADR-031](../decisions/ADR-031-state-forall-separating-witness.md) SW-12
  and SW-13.
- [FR-072](FR-072-implement-typed-replay-result.md) (the result and its
  causes), [FR-268](FR-268-check-a-state-clause-witness-on-replay.md) (the
  replay that settles it).
- QSpec FR-351 (record identity and strict reading).

## References

- Owning ticket: Linear QSL-389. Implementation: Linear QSL-45.
