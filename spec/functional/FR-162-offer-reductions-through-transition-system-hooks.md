---
id: FR-162
title: "Offer reductions through TransitionSystem hooks and expand the reduced product"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-019
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-021
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-152
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-155
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-156
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-158
    type: depends_on
---
# FR-162: Offer reductions through TransitionSystem hooks and expand the reduced product

## Description

FR-101's `TransitionSystem` carries three optional hooks that a system
implements to offer a reduction: `canonical` for symmetry, `footprint` for
partial-order reduction and `holds_constraint` for a state constraint
(ADR-021 EI-4), and `offered_reductions` says which of them it implements.
`ModelSystem` implements all three. The model checker's product system
delegates each hook to its model component, so the property automaton is
never reduced. `model_check`, in `qsl-analyze`, SHALL expand the reduced product
in FR-101's breadth-first order, canonicalising each initial state and
successor, choosing ample sets, cutting at the constraint, and retaining
each edge with its permutation (EI-2, EI-3). FR-101's `explore` and `sample`
and FR-120's `explore_model` never call the hooks (RD-2).

## Use case

A verification operator's reduced run and unreduced run differ only in which
states are stored. Simulation of the same subject still reports a finding at
every reachable state (US-019).

## Inputs

- A `TransitionSystem` and the admitted reductions (FR-159).

## Outputs

The hooks `offered_reductions`, `canonical`, `footprint` and
`holds_constraint` are members of FR-101's `TransitionSystem`, which
`qsl-eval` owns; this FR states their behaviour and does not restate the
trait. `Footprint` is FR-155's, `OfferedReductions` FR-159's and
`ConstraintValue` FR-158's. `RetainedEdge` is owned by `model_check` in
`qsl-analyze` (ADR-018 LA-1).

```rust
pub struct RetainedEdge {
    pub source: ProductStateId,
    pub transition: TransitionIdentity,
    pub target: ProductStateId,       // canonical
    pub permutation: Permutation,     // identity without symmetry
}
```

## Behavior

- **Offering.** `offered_reductions` SHALL name exactly the hooks the system
  implements, and the pre-check settles a request selecting a reduction it
  does not offer V-8 (FR-159). When a hook the system offers returns `None`
  in a run, the engine SHALL settle an internal fault,
  `TerminalValue::Failed`. `ModelSystem` SHALL offer all three and return `Some` from each, through FR-152, FR-155
  and FR-158.
- **Product delegation.** The product system SHALL answer
  `offered_reductions` with its model component's answer, and `canonical` by
  canonicalising its model component and keeping the automaton state,
  `footprint` with its model component's footprint, and `holds_constraint`
  with its model component's answer. The terminal stutter edge SHALL have an
  empty footprint and belong to no ample-set candidate.
- **Initial states.** `model_check` SHALL canonicalise each initial product
  state and coalesce by canonical key, in subject order. A constraint false
  at an initial state SHALL make it a boundary state.
- **Expansion.** `model_check` SHALL expand each product state in
  breadth-first order by computing the full enabled set and each enabled
  identity's successors, choosing `A(s)` (FR-156) or taking `E(s)`,
  canonicalising each successor through `A(s)` and keeping its permutation,
  applying C3 against the discovered set, marking each successor where the
  constraint is false as a boundary state, and retaining each edge as a
  `RetainedEdge`. `model_check` SHALL leave boundary states unexpanded
  (FR-158).
- **Combination.** With symmetry and partial-order reduction both admitted,
  `model_check` SHALL choose ample sets at the canonical state and judge C3
  on canonical product states.
- **Second phase.** FR-126's SCC phase and fairness filter, and FR-153's
  annotated quotient, SHALL run on the retained edges.
- **Limits.** `max_states` SHALL count stored product states, boundary
  states included; the frontier SHALL hold canonical state keys' digests
  (ADR-014 TR-7). FR-126's limit map holds; `max_depth` is the search horizon, not a limit,
  and FR-160 settles a partial-order run that completes to it.
- **Simulation unchanged.** `explore`, `sample`, `explore_model` and
  `sample_model` SHALL NOT call `canonical`, `footprint` or
  `holds_constraint`.
- **Determinism.** The retained product SHALL be a function of the subject,
  the item, the reductions and the limits.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-162-AC-1 | `ModelSystem` offers all three reductions through `offered_reductions` and returns `Some` from all three hooks for ADR-021 §7.1's annotated subject. A test `TransitionSystem` whose `offered_reductions` is `NONE` settles a request selecting symmetry, one selecting partial-order reduction, and one selecting a constraint each V-8 in the pre-check. FR-120's `explore_model` over §7.1's annotated subject stores 27 states and records findings for each, as it does without the annotation. | Test (TC-587) |
| FR-162-AC-2 | The product system's `canonical` keeps the automaton state and canonicalises only the model state; its `footprint` for a model transition equals `ModelSystem::footprint`; the terminal stutter edge of FR-126-AC-3's subject has an empty footprint. Under symmetry every retained edge's target is canonical and `permutation` maps the concrete successor to it; without symmetry every `permutation` is the identity. | Test (TC-587) |
| FR-162-AC-3 | Combination: ADR-021 §7.2's subject with `counters` annotated `symmetric`, `[[a, b, c]]` and partial-order reduction, instance `x = a` (group `{id, (b c)}`): the run stores 7 model states, `(0,0,0)`, `(0,0,1)`, `(0,1,1)`, `(0,1,2)`, `(0,2,2)`, `(1,2,2)` and `(2,2,2)`, and returns `Holds`, settling `Proved{Reduced{[Symmetry{…}, PartialOrder{BreadthFirstRevisit}]}, Uncertified}`. | Test (TC-587) |
| FR-162-AC-4 | `max_states` 5 over ADR-021 §7.3's constrained run stops `Stopped(ResourceExhausted, MaxStates)` with boundary states counted toward the 5. Two runs of AC-3 retain equal edge lists. | Test (TC-587) |

## Dependencies

- ADR-021 RD-2, EI-2 to EI-5, §2 "Combining reductions"; ADR-014 TR-7.
- [FR-101](FR-101-explore-finite-models-with-canonical-order-and-the-qspec-sampler.md)
  (the trait and engine), [FR-120](FR-120-simulate-a-checked-package-s-state-family.md)
  (`ModelSystem`, `explore_model`), [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (the product and its phases), [FR-152](FR-152-canonicalise-a-model-state-by-sorting-its-symmetry-classes.md),
  [FR-155](FR-155-derive-and-enforce-read-and-write-footprints.md),
  [FR-156](FR-156-expand-ample-sets-with-the-breadth-first-proviso.md),
  [FR-158](FR-158-cut-the-search-with-a-state-constraint.md).
- QSpec owns reduced exhaustive exploration as normative semantics (ADR-021
  QS-2).

## References

- QSpec half: QSpec FR-383 (Linear STD-134; ADR-021 §9 QS-2). Owning ticket: Linear
  QSL-368.
