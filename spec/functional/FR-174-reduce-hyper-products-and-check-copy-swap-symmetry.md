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

QSL SHALL settle a reduction a request selects for a hyper item by
ADR-023 §5's row for the item's form (ADR-023 PH-1 to PH-6): a row that reads
"no" settles it V-6 `ReductionNotPreserving`, and a row that reads "yes"
settles it V-8 `unsupported-requested-capability`, since EN-1 builds no such
reduction for a hyper product. The S3 check SHALL detect, on every HP-2 and HP-6 clause, the universal variables
that can be swapped without changing the clause (ADR-023 CS-1). EN-1 SHALL
store one product state per orbit of the resulting copy group for an HP-2
clause whose swapped variables carry no fairness (CS-2, CS-3). No
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
- For EN-1: a canonicaliser over product states, and the run's group: the
  product of the symmetric groups of the copy group's classes whose variables
  carry no fairness set.

## Behavior

### Preservation rows

- When the request selects a reduction, EN-1 SHALL settle the item V-6
  `ReductionNotPreserving` before any expansion where ADR-023 §5's table
  reads "no" for the item's form and subjects, naming the reduction and the
  form.
- Partial-order reduction SHALL be refused for every hyper form.
- A state constraint SHALL be refused for HP-3 (ADR-023 PH-6).
- Key symmetry SHALL be refused for a clause over two subjects, and for an
  HP-2 clause with an `each` fairness constraint (ADR-023 PH-2, PH-3).
- Where the table reads "yes", the reduction preserves the verdict, but EN-1
  builds the quotient of a hyper product by the copy group alone: it does not
  construct the diagonal action of a key symmetry (ADR-023 PH-2) and does not
  apply a state constraint to a hyper product. A selection of either SHALL
  settle the item V-8 `unsupported-requested-capability` before any
  expansion, naming the reduction.
- A selected reduction SHALL NOT be ignored.
- When several reductions are selected, the first whose row reads "no", in
  the order key symmetry, partial order, state constraint, SHALL decide;
  otherwise the first selected reduction SHALL be named.
- An HP-6 clause is read by the same rows (ADR-023 PH-4): partial-order
  reduction settles `ReductionNotPreserving` naming `PartialOrder` and the
  form `ProjectionAligned`, and key symmetry over one subject or a state
  constraint, which read "yes", settle V-8.

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

### Which classes EN-1 swaps

- S3's copy group is a function of the checked clause alone. EN-1 SHALL
  reduce by a class only when its variables carry no fairness set. A
  fairness constraint is read on the slot of its variable in the stored
  state, and the quotient moves a concrete copy between slots wherever an
  edge swaps them, so a slot's enabled and taken steps are not one
  behaviour's and the quotient would decide fair cycles wrongly. A class
  whose variables carry any fairness set, whatever the granularity, SHALL be
  stored whole and named in no reduction.
- EN-1 SHALL reduce only if every exchange of the copy group fixes the
  initial state of the body's automaton and maps each of its acceptance
  sets, one for each unbounded `eventually` or `until` the negated body
  holds, to itself. A swap that exchanges two acceptance sets, as the swap of
  `a` and `b` does for the body `always holds(v.l @ a = 0) and always
  holds(v.l @ b = 0)`, is not an automorphism of the acceptance condition. If
  the exchange of any class fails, EN-1 SHALL store the product whole and
  name no reduction, for the classes that pass as well, while S3 still
  records the copy group.
- EN-1 SHALL apply copy-swap to an HP-2 clause only. An HP-6 clause SHALL be
  stored whole as its projected product and name no `CopySwap`, though S3
  records its copy group as for an HP-2 clause: EN-1 builds no exchange of
  the bad-prefix monitor's states for the projected product.

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
- The lasso a refutation reports SHALL be the canonical lasso of the quotient
  (FR-126's canonical stem and loop over the stored states) concretised as
  above, not the canonical lasso of the unreduced product. The two are both
  real and both replay, but may differ in their stem, loop and
  `trace_position`; the one reported is a function of the request and is
  the same on every run.
- A proof under copy-swap SHALL settle `Proved{basis: Reduced{reductions},
  certification: Certified}` with `CopySwap{classes}` among its reductions,
  once FR-163's checker accepts its certificate; a refutation SHALL name no
  reduction (ADR-023 CS-5).
- The copy group adds no obligation identity member: it is a function of
  the checked clause.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-174-AC-1 | ADR-023 §1's `NonInterference` has copy group `[[a, b]]`. The same clause with `v.l @ a <= v.l @ b` as its body, with `fair { weak V::Vault::step }` on `a` only, or with `SavesPower`'s two aliases has an empty copy group. A body written `v.l @ b = v.l @ a` and one written `v.l @ a = v.l @ b` are both swappable. | Test (TC-599) |
| FR-174-AC-2 | With copy-swap, §8.1's leaky product stores 7 states and the secure product 6, against 10 and 8 unreduced; the leaky verdict is `refuted` and the secure verdict `proved` in both runs, the secure proof with basis `Reduced` naming `CopySwap{[[a, b]]}`. | Test (TC-599) |
| FR-174-AC-3 | The leaky refutation under copy-swap is a `HyperCounterexample` with traces in quantifier order and no canonical state, and it replays by FR-183. Running the request twice gives byte-equal counterexamples. | Test (TC-599) |
| FR-174-AC-4 | With partial-order reduction selected, `NonInterference` settles `ReductionNotPreserving` before any expansion; with a state constraint selected, `Opaque` settles `ReductionNotPreserving`; with key symmetry selected over `SavesPower`'s two subjects, it settles `ReductionNotPreserving`. | Test (TC-599) |
| FR-174-AC-5 | A clause with the body `always holds(v.l @ a = v.l @ b)`, the `match` conjunct `a.step.op = b.step.op` and `fair weak V::Vault::reset` on both `a` and `b`, over a vault whose `step` flips `l` and whose `reset` is enabled at `l = 1` and keeps it, from `a = (0, 0)` and `b = (0, 1)`, has the copy group `[[a, b]]` and is stored whole: 4 product states, the unreduced count, no reduction named, and `Violated` with a counterexample that replays fair. With the fairness on `a` and `b` and a second class `c`, `d` without any, the outcome names `CopySwap{[[c, d]]}` alone. | Test (TC-599) |
| FR-174-AC-6 | The clause with the body `always holds(v.l @ a = 0) and always holds(v.l @ b = 0)` over the secure vault has the copy group `[[a, b]]` and is stored whole with no reduction named, since its swap exchanges two acceptance sets. The body `eventually holds(v.l @ a = 1) and eventually holds(v.l @ b = 1)` over a vault whose `step` toggles `l`, from the initial states `(0, 0)` and `(0, 1)`, has a negation with no acceptance set, which its swap fixes, and is stored as 5 states against the unreduced 8. A clause with one class whose swap passes and one whose exchange fails is stored whole with no reduction named. | Test (TC-599) |
| FR-174-AC-7 | With key symmetry selected over one subject, `NonInterference` (whole fairness) and `Opaque` each settle V-8 `Unsupported` naming the symmetry; with a state constraint selected, `NonInterference` settles V-8 naming the constraint; each before any expansion. With key symmetry selected over one subject and `fair each` constraints on `NonInterference`, it settles `ReductionNotPreserving`. With no selection, `NonInterference` over the secure vault holds with the reduction `CopySwap`. With key symmetry and a state constraint both selected on `NonInterference`, the item settles V-8 naming the symmetry; with partial-order reduction and key symmetry both selected on `Opaque`, it settles `ReductionNotPreserving` naming partial-order reduction. | Test (TC-599) |
| FR-174-AC-8 | The `align skip` clause of FR-178-AC-1 has the copy group `[[a, b]]`, is stored as the 18 states of its projected product with no reduction named; with partial-order reduction selected it settles `ReductionNotPreserving{PartialOrder, ProjectionAligned}`, and with key symmetry selected over its one subject it settles V-8 `Unsupported`. | Test (TC-599) |

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
