---
id: FR-159
title: "Select reductions on a request and pre-check them against the preservation table"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-019
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-021
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-123
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-151
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-155
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-158
    type: depends_on
---
# FR-159: Select reductions on a request and pre-check them against the preservation table

## Description

A model-check request selects any combination of symmetry, partial-order
reduction and a state constraint (ADR-021 RD-1). Before the first expansion,
QSL's `model_check` SHALL look up each selected reduction in the normative
preservation table for the claim's form and fairness set (PT-1, PT-2), admit
the symmetry declarations, check that the system offers each selected
reduction, and
resolve the constraint (EI-1). A selection the table does not admit settles
`inconclusive`, `ReductionNotPreserving`; the engine never runs a different
method than the one requested (RD-3, RV-9). Negotiation routes a request
that selects a reduction only to a candidate that advertises it (RV-6). The
symmetry declarations and the partial-order selection enter the obligation
identity; the constraint does not (RV-8).

## Use case

A verification operator selects partial-order reduction for a bounded MLTL
claim. Bounded MLTL counts positions, which the reduction changes, so the
request settles `inconclusive`, `ReductionNotPreserving{PartialOrder, TP-2}`,
at once, and the operator resubmits without it (US-019).

## Inputs

```rust
pub struct Reductions {
    pub symmetry: Vec<SymmetryDeclaration>,   // FR-151
    pub partial_order: bool,
    pub constraint: Option<ClauseName>,       // FR-158
}

// ModelCheckRequest (FR-126) gains: pub reductions: Reductions
```

- The checked clause's property form (FR-123), whether it has an interval
  operator, and its fairness set; or the deadlock-freedom item; or a
  refinement item (ADR-020).

## Outputs

- The admitted reductions for the run, with each one's table row; or
- `ReductionNotPreserving{reduction, form}` (FR-160); or a symmetry refusal
  or `SymmetryBroken` (FR-151); or V-8 `unsupported-requested-capability`;
  or a constraint refusal (FR-158).

```rust
pub struct OfferedReductions {
    pub symmetry: bool,       // implements `canonical`
    pub partial_order: bool,  // implements `footprint`
    pub constraint: bool,     // implements `holds_constraint`
}
impl OfferedReductions {
    pub const NONE: Self = Self { symmetry: false, partial_order: false, constraint: false };
}
```

`OfferedReductions` is owned by `qsl-eval`, beside FR-101's
`TransitionSystem`, whose `offered_reductions` returns it. `ModelSystem`
offers all three; the product system offers what its model component
offers.

## Behavior

- **Preservation table.** `model_check` SHALL hold one function
  `preserves(reduction, form) -> bool`, exhaustive over its inputs with no
  `_` arm, that reads:
  - symmetry: `true` for every form, fairness set and item below;
  - partial-order reduction: `true` for TP-1, for TP-3 and TP-4 with no
    interval operator under any fairness set, weak or strong, `whole` or
    `each`, and for the deadlock-freedom item; `false` for TP-2, for any
    formula with an interval operator, and for both halves of a refinement
    item;
  - state constraint: `true` for every form.
- **Order.** After FR-126's root classification, the pre-check SHALL, in
  this order: (a) look up each selected reduction, symmetry, then
  partial-order, then constraint, and settle the first `false` as
  `ReductionNotPreserving{reduction, form}`; (b) admit the symmetry
  declarations (FR-151); (c) settle V-8
  `unsupported-requested-capability` naming the first selected reduction,
  in the order symmetry, partial-order, constraint, that the system's
  `offered_reductions` does not offer; (d) resolve the
  constraint (FR-158). The first failure SHALL settle the item, and no
  state is expanded.
- **Combination.** The pre-check SHALL admit a combination only when every
  selected reduction's row reads `true`. With symmetry and partial-order
  reduction both selected, the engine SHALL compute ample sets on canonical
  states and judge C3 on canonical product states (FR-162).
- **No substitution.** The engine SHALL NOT run a request without a
  reduction it selected, or with one it did not select.
- **Capability.** EN-1's provider manifest SHALL advertise the reductions it
  applies: `symmetry`, `partial-order` and `state-constraint`. Negotiation
  (FR-075) SHALL route an item that selects reductions only to a candidate
  advertising every one of them, and SHALL settle the item V-8
  `unsupported-requested-capability` when no candidate does. It SHALL NOT
  drop or add a reduction.
- **Identity.** The request writer SHALL put the symmetry declarations
  (population and key classes, classes in request order) and the
  partial-order selection into the item's obligation identity, beside the
  subject (ADR-013 O-09). The constraint SHALL stay out of it (FR-158).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-159-AC-1 | ADR-021 §7.2: `always[0,2] holds(x.v = 0)` under event-position false-extension (TP-2) with partial-order reduction settles `ReductionNotPreserving{PartialOrder, TP-2}` with no state expanded. `always (holds(x.v = 0) implies eventually[0,2] holds(x.v = 1))` under infinite-trace (TP-3 with an interval) with partial-order reduction settles `ReductionNotPreserving{PartialOrder, TP-3}`. The same two claims with symmetry selected over an annotated population are admitted. | Test (TC-582) |
| FR-159-AC-2 | `preserves` returns, for partial-order reduction: `true` for TP-1, for TP-3 with no interval, for TP-4 with no interval under the empty fairness set, under `fair weak whole` and under `fair weak each`, and for the deadlock-freedom item; `false` for TP-2, for TP-4 with an interval under an unbounded operator, and for each refinement half. It returns `true` for symmetry and for the constraint for every one of these. A request selecting symmetry and partial-order reduction for TP-2 settles `ReductionNotPreserving{PartialOrder, TP-2}`, naming partial-order reduction, not symmetry. | Test (TC-582) |
| FR-159-AC-3 | A request selecting partial-order reduction, one selecting symmetry and one selecting a constraint, each over a test `TransitionSystem` whose `offered_reductions` is `NONE`, each settle V-8 `unsupported-requested-capability` in the pre-check, naming the reduction, with no state expanded. A registry whose only model-check candidate advertises `symmetry` alone settles a request selecting `partial-order` V-8, and routes a request selecting `symmetry` to that candidate; negotiation never routes the first to the candidate without partial-order reduction. | Test (TC-583) |
| FR-159-AC-4 | Over ADR-021 §7.1's subject, the request with `[[a, b, c]]` and the request with no reduction for the same claim have different obligation identities; requests that differ only by constraint `Low` versus none have equal identities. After `ReductionNotPreserving`, the outcome records no exploration statistics. | Test (TC-583) |

## Dependencies

- ADR-021 RD-1 to RD-3, PT-1, PT-2, §2 "Combining reductions", EI-1, RV-2,
  RV-6, RV-8, RV-9.
- [FR-075](FR-075-compute-candidates-from-registered-backends.md) (provider
  manifests and candidates), [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md)
  (property forms), [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (the request and pre-check), [FR-151](FR-151-admit-a-request-s-symmetry-declarations.md),
  [FR-155](FR-155-derive-and-enforce-read-and-write-footprints.md),
  [FR-158](FR-158-cut-the-search-with-a-state-constraint.md).
- ADR-020 (refinement items) and ADR-019 (strong fairness) supply the forms
  the table also covers.
- QSpec owns the request members, the obligation-identity membership, the
  normative preservation table and the provider advertisement (ADR-021 QS-1,
  QS-5, QS-9).

## References

- QSpec half: QSpec FR-382 and FR-385 (Linear STD-134; ADR-021 §9 QS-1, QS-5, QS-9). Owning ticket:
  Linear QSL-368.
