---
id: FR-174
title: "Reduce hyper products: preservation rows and compiler-checked copy-swap symmetry"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-021
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-023
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-172
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-173
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-176
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-178
    type: depends_on
---
# FR-174: Reduce hyper products: preservation rows and compiler-checked copy-swap symmetry

## Description

QSL SHALL apply a state-space reduction to a hyper item exactly where
ADR-023 §5's row for the item's form reads "yes" (ADR-023 PH-1 to PH-6). The S3
check SHALL detect, on every HP-2 and HP-6 clause, the universal variables
that can be swapped without changing the clause (ADR-023 CS-1). EN-1 SHALL
store one product state per orbit of the resulting copy group (CS-2). No
request selects copy-swap and no author declares it: the compiler applies
it only when the checked clause has the symmetry.

## Use case

A verification operator checks noninterference over two copies of one
model. The body and the match read the same with `a` and `b` swapped, so
the compiler stores each unordered pair of runs once and the run takes half
the states, with the same verdict. A clause that is not symmetric gets no
swap.

## Inputs

- A `CheckedHyperClause` (FR-172) with its `HyperForm` (FR-173).
- Any reduction the request selects (ADR-021, as its requirements state).

## Outputs

- `CopyGroup { classes: Vec<Vec<Name>> }` recorded beside the checked
  clause: the swappable classes of universal variables, empty when none.
- For EN-1: a canonicaliser over product states, and the run's group, the
  product of the copy group and any admitted diagonal key symmetry.

## Behavior

### Preservation rows

- When the request selects a reduction, EN-1 SHALL admit it for a hyper
  item exactly where ADR-023 §5's table reads "yes" for the item's form and
  subjects, and SHALL settle the item V-6 `ReductionNotPreserving` before
  any expansion otherwise.
- An admitted key symmetry SHALL act diagonally: one permutation applied to
  every component over the one subject, with the object parameters'
  stabiliser applied (ADR-023 PH-2).
- Partial-order reduction SHALL be refused for every hyper form.
- A state constraint SHALL be refused for HP-3 (ADR-023 PH-6).

### Copy-swap detection (S3)

- S3 SHALL test every pair of universal variables `a`, `b` of an HP-2 or
  HP-6 clause. The pair SHALL be swappable when both name the same alias,
  their fairness sets are equal under the unmarked `whole` reading, and the
  swap `σ` that renames `a` to `b` and `b` to `a` in every indexed atom and
  step label leaves `μ_U` and the body equal in S3's normal form.
- The normal form SHALL sort the operands of `and`, `or`, `=` and `!=` by
  their checked encoding, flatten nested `and` and `or`, and leave every
  other node in place. S3 SHALL compare the normal form of `σ(φ)` with that
  of `φ` node by node, and the same for `μ_U`.
- Swappable variables SHALL fall into classes by the equivalence
  "swappable", and the copy group SHALL be the product of the symmetric
  groups of the classes.

### Canonicalisation (EN-1)

- EN-1 SHALL build the body's automaton from its normal form, so `σ`
  permutes automaton states by renaming the subformulas each records.
- The canonical form of a product state SHALL be the member of its orbit
  under the run's group with the least key bytes, the component keys in
  quantifier order and then the automaton state index.
- EN-1 SHALL canonicalise every successor and store canonical states only.

### Counterexamples and verdicts

- A counterexample found on the quotient SHALL be concretised before it is
  enveloped: replay of the quotient path tracks the copy permutation that
  maps each stored state to the concrete tuple, and a loop that closes on a
  permuted state is repeated until the concrete tuple closes, at most the
  order of that permutation. The result is an ordinary
  `HyperCounterexample` (FR-183) with its traces in quantifier order.
- A proof under copy-swap SHALL settle `Proved{basis: Reduced{reductions}}`
  with `CopySwap{classes}` among its reductions; a refutation SHALL name no
  reduction (ADR-023 CS-5).
- The copy group adds no obligation identity member: it is a function of
  the checked clause.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-174-AC-1 | ADR-023 §1's `NonInterference` has copy group `[[a, b]]`. The same clause with `v.l @ a <= v.l @ b` as its body, with `fair { weak V::Vault::step }` on `a` only, or with `SavesPower`'s two aliases has an empty copy group. A body written `v.l @ b = v.l @ a` and one written `v.l @ a = v.l @ b` are both swappable. | Test (TC-599) |
| FR-174-AC-2 | With copy-swap, §8.1's leaky product stores 9 states and the secure product 6, against 14 and 8 unreduced; the leaky verdict is `refuted` and the secure verdict `proved` in both runs, the secure proof with basis `Reduced` naming `CopySwap{[[a, b]]}`. | Test (TC-599) |
| FR-174-AC-3 | The leaky refutation under copy-swap is a `HyperCounterexample` with traces in quantifier order and no canonical state, and it replays by FR-183. | Test (TC-599) |
| FR-174-AC-4 | With partial-order reduction selected, `NonInterference` settles `ReductionNotPreserving` before any expansion; with a state constraint selected, `Opaque` settles `ReductionNotPreserving`; with key symmetry selected over `SavesPower`'s two subjects, it settles `ReductionNotPreserving`. | Test (TC-599) |

## Dependencies

- ADR-023 §5 PH-1 to PH-6, §16 CS-1 to CS-5, §12 RU-4.
- [FR-172](FR-172-check-hyper-and-relation-clauses-over-model-subjects.md),
  [FR-173](FR-173-classify-hyper-clauses-into-forms.md),
  [FR-176](FR-176-check-a-universal-hyperproperty-by-self-composition.md)
  and [FR-178](FR-178-check-a-projection-aligned-hyperproperty.md) (the
  products it reduces).
- The reductions themselves, their selection and their identity rule are
  ADR-021's.
- QSpec owns the preservation rows and copy-swap's detection rule,
  admission and `CopySwap` basis member (ADR-023 QS-8).

## References

- ADR-023. QSpec half: QSpec FR-403 (Linear STD-136; ADR-023 QS-8).
