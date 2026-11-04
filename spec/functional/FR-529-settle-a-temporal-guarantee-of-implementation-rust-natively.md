---
id: FR-529
title: "Settle a temporal guarantee of implementation Rust from checked correspondence and progress premises"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-031
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-020
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-304
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-306
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-307
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-488
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-353
    type: depends_on
---
# FR-529: Settle a temporal guarantee of implementation Rust from checked correspondence and progress premises

## Description

QSL SHALL admit, check and settle QSpec FR-488's implementation-guarantee item
from the checked package's abstraction relation (FR-304, FR-306, FR-307) and
the premise results the request carries, and SHALL return a native conclusion
only through the settlement FR-488 defines.

## Inputs

An implementation-guarantee item: the temporal clause and its fairness set,
the model subject, the abstraction relation revision from the checked package,
and the identities of the model-claim, initialization, transition-simulation,
frame, no-divergence, fairness-correspondence and environment premises.

## Outputs

A result record (QSpec FR-488 "Conclusion record") for `proved`, or the
inconclusive, unsupported, incomplete or refused result FR-488 names, each
source-bound to the checked package.

## Behavior

QSL reuses the existing records and builds only what is missing:

| Premise | Existing owner | Native addition |
| --- | --- | --- |
| Abstraction binding for each model element | FR-304, FR-306, FR-307 (QSL-36) | none: an unbound element refuses as FR-353-AC-3 states |
| Per-step contract on a named Rust function | the FR-350 backend, IR-32 | none: its result is read, not re-derived |
| Model-claim premise, stutter closure | QSpec FR-380 transfer and the existing model-check settlement | none |
| Initialization, transition simulation, frame | the FR-350 backend over the bound relation | the native obligation naming each as its own item with the relation revision |
| No-divergence, fairness correspondence | none | the native obligation, admission of its measure or bound and the settlement of its result |
| Environment premises | none | the typed premise record and its trust basis |

- Compile and read-back of an implementation-guarantee item shall keep the
  authored source correspondence of every premise it names.
- Admission shall refuse a premise whose identity names no settled result, or
  whose subject or relation revision differs from the item's, as
  `invalid_runtime_input`/`invalid-value` naming the member.
- Settlement shall follow QSpec FR-488's table, so that a violating step, a
  valid per-step contract alone, an infinite stutter behaviour or a fairness
  premise the environment does not supply cannot yield `proved`.
- A premise result that only references another result shall be read; the
  reference alone is never evidence of validity.
- Settlement shall keep a missing backend, a missing premise, a missing binding
  and exhausted work as distinct outcomes with their own proof extent and trust
  basis, and shall settle independent supported sibling items unchanged.
- Settlement shall never promote a finite trace to an unbounded liveness
  conclusion.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-529-AC-1 | A valid supported chain yields a `proved` result naming the relation revision and implementation sources, the clause and fairness set, each premise result, the assumptions, the proof extent and the trust basis, bound to the checked package's source map. | Test (TC-1329) |
| FR-529-AC-2 | A Rust step that violates the abstraction relation, and a chain with only valid per-step contracts and no progress premise, each leave a liveness item not `proved` with QSpec FR-488's cause. | Test (TC-1329) |
| FR-529-AC-3 | An infinite internal-step behaviour, a stutter step with no decreasing measure and a fairness premise stronger than the environment's each settle `inconclusive`/`transfer-not-preserving`. | Test (TC-1329) |
| FR-529-AC-4 | A missing backend, premise or binding and an exhausted bound settle with four distinct results and extents; sibling items with resolved premises settle unchanged. | Test (TC-1329) |
| FR-529-AC-5 | Compile, read-back and admission keep source correspondence, refuse a forged or mismatched premise identity, and a result that only references another is not accepted as evidence; no finite trace settles a liveness item `proved`. | Test (TC-1329) |

## Dependencies

- QSpec FR-488: the requirement this settles; QSpec FR-353, FR-350, FR-354 and
  FR-380.
- [FR-304](FR-304-check-an-authored-abstraction-relation.md),
  [FR-306](FR-306-carry-the-abstraction-relation-in-the-checked-package.md),
  [FR-307](FR-307-export-the-bindings-each-item-references.md): the bindings.
- Real Verus execution stays dependent on IR-32. This requirement adds no
  parallel evidence store and no new universal prover.

## References

- Linear QSL-1541; QSL-36.
