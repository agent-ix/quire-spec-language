---
id: FR-269
title: "Settle replay disagreements as typed DisagreementCause variants"
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
  - target: ix://agent-ix/quire-spec-language/FR-450
    type: depends_on
---
# FR-269: Settle replay disagreements as typed DisagreementCause variants

## Description

FR-072's shared `DisagreementCause` SHALL carry typed replay
disagreements. This requirement defines its `Witness` variant; FR-450 adds
the `HybridTrace` variant and extends this same cause codec. Each
disagreement settles `inconclusive` and is never repaired.

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

- `DisagreementCause::proved` and `replayed` SHALL return the
  `Witness` variant's verdicts, matched with no `_` arm.
- A `Witness` cause SHALL settle `inconclusive`. A `Witness`-arm result
  with this cause SHALL carry no QSpec FR-351 record of its own, so no
  consumer reads a disputed record as evidence.
- The cause SHALL serialize and read back with `given` and `derived` each
  present in full or absent (QSpec FR-351 has no partial record), and a
  reader SHALL refuse a `Witness` cause whose `failure` is not one of the
  values above, and a record whose deciding element is a record or tuple
  whose slots do not fit its declaration's shape (QSpec FR-351-AC-5).
- Two results that differ only in their `Witness` cause's `given` or
  `derived` record SHALL compare unequal.

### Hybrid trace disagreement

FR-450 adds `DisagreementCause::HybridTrace(HybridTraceDisagreement)`
directly to this shared cause type and codec. Its wire shape is
`{"cause":"hybrid-trace","disagreement":{"type":<arm>,"counterexample":{"type":"hybrid-counterexample","kind":...,"trace":...}}}`.
The `disagreement.type` is exactly `not-reproduced` or `not-exact`.
`disagreement.counterexample` is the complete QSpec FR-193
`HybridCounterexample`, including both `kind` and `trace`; the trace uses
the FR-193 named wire form. It is never projected to a verdict-only or
witness cause. The enclosing terminal cause remains `replay-parity`.

The shared cause codec SHALL extend its existing methods directly:
`DisagreementCause::to_json(hybrid_model: Option<&CheckedHybridModel>)`
and `DisagreementCause::from_json(text, package, hybrid_model: Option<&CheckedHybridModel>)`.
The caller supplies the checked model bound to the routed hybrid claim. A
`HybridTrace` cause requires that model; absence is refused. For every
other cause, the existing package context continues to resolve package
declarations and the optional model is unused. No adapter or parallel
hybrid-only cause codec is introduced.

When writing a hybrid trace, the codec maps each typed mode index to that
mode's name in the supplied checked model and each enclosure position to the
name of the corresponding checked variable. A transition remains its
zero-based position in the previous mode's transition list. Reading resolves
those names in the same checked model and reconstructs the typed indices.
Unknown or out-of-range mode, variable, or transition references are refused.
A round trip with the same checked model returns an equal typed cause.
`DisagreementCause::proved` and `replayed` return `Some(verdict)` for
`Verdicts`, `NoValue` and `Witness`, and `None` for `HybridTrace`, which
has no pair of verdicts. This preserves the typed cause without inventing a
verdict for an exact-replay failure.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-269-AC-1 | FR-268-AC-2's index-2 replay settles `inconclusive` with `Witness { proved: violation, replayed: violation, given: <index-2 record>, derived: <index-1 record>, failure: Mismatch }`, and its `Witness`-arm result carries no QSpec FR-351 record; the no-record replay holds `given` absent and `derived` the index-1 record. | Test (TC-744) |
| FR-269-AC-2 | FR-268-AC-3's failing records map to `failure` `Separation { Quantifier, Unmet }`, `Separation { Element, Unmet }` (three times) and `Separation { Body, Unmet }`; its undefined-domain, undefined-body and refused-domain variants map to `Separation { Domain, UndefinedEvaluation }`, `Separation { Body, UndefinedEvaluation }` and `Separation { Domain, Refused }`, each `UndefinedEvaluation` naming the expression and its undefined cause. | Test (TC-744) |
| FR-269-AC-3 | A replay result holding FR-269-AC-1's cause round-trips through serialize and read to an equal result; the same document with `failure` set to an undefined value refuses, and one with `given` missing its `index` refuses; two results that differ only in `derived` compare unequal. | Test (TC-744) |
| FR-269-AC-4 | A deciding element whose value or type nests 100,000 levels round-trips through the cause codec on a 512 KiB stack: the writer is `quire-canonical`'s, the reader keeps the document on the heap (`quire_canonical::read`) and decodes each deciding element from it on an explicit stack, and the element reads back and encodes to the same document. No depth refuses (ADR-030 D-1). | Test (TC-744) |
| FR-269-AC-5 | A provider formula counterexample whose exact replay returns `NotReproduced` round-trips as `DisagreementCause::HybridTrace(NotReproduced{counterexample})` through the shared `DisagreementCause::to_json(Some(checked_model))` and `from_json(text, package, Some(checked_model))` methods. The cause wire has `cause: hybrid-trace`, a `disagreement.type` discriminator, and the full `disagreement.counterexample` in FR-193 wire form, including `type: hybrid-counterexample`; mode and variable names map through the supplied checked model. The `proved` and `replayed` accessors return `None` for this cause. The `not-exact` discriminator round-trips to its distinct arm. The same hybrid cause without checked-model context is refused; the existing `Verdicts`, `NoValue` and `Witness` cause round trips remain unchanged. | Test (TC-1221) |

## Dependencies

- [ADR-031](../decisions/ADR-031-state-forall-separating-witness.md) SW-12
  and SW-13.
- [FR-072](FR-072-implement-typed-replay-result.md) (the result and its
  causes), [FR-268](FR-268-check-a-state-clause-witness-on-replay.md) (the
  replay that settles it).
- QSpec FR-351 (record identity and strict reading).

The cause's own codec round-trips here. The round-trip of a whole replay
result holding the cause lands with the `native-run-result/2` wire
([FR-267](FR-267-write-run-results-as-native-run-result-2.md)).

## References

- Owning ticket: Linear QSL-389. Implementation: Linear QSL-45.
