---
id: FR-158
title: "Cut the model-check search with a state constraint"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-019
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-021
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-124
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
---
# FR-158: Cut the model-check search with a state constraint

## Description

A state constraint is a Boolean state clause declared in the unit, checked
at S3 like an invariant body, and named by a model-check request (ADR-021
SC-1). The model checker SHALL expand a state only where the constraint
holds; a reached state where it does not is a **boundary state**, which is
stored and read by the property automaton but never expanded, never
stutter-extended and never a deadlock (SC-2). A constraint is a method
parameter: it does not change the subject or the obligation identity, and a
run that reaches a boundary state cannot settle `proved` (SC-3, RV-4).

## Use case

A verification operator's model is too large to finish. They cut the search
at counters above 1 and find a real violation in seconds; when the cut hides
the violations, the result says `ConstraintReached` with the number of
boundary states, never `proved` (US-019).

## Inputs

- A checked `constraint` clause of the unit (spelling QSpec's; ADR-021 §7.3
  writes `constraint Low using v on M::Counter at current { self.v <= 1 }`).
- `ModelCheckRequest.reductions.constraint: Option<ClauseName>` (FR-159).

## Outputs

`ModelSystem` implements FR-101's `holds_constraint` hook and always
returns `Some`. Its value:

```rust
pub enum ConstraintValue {
    Holds,                // the state is expanded
    Boundary,             // the constraint is false: a boundary state
    Undefined {           // the item refutes at this state (ADR-021 SC-2)
        object: ObjectReference,
        reason: UndefinedReason,   // FR-100's catalog spelling
    },
}
```

and, per run, the count of boundary states reached.

## Behavior

- **Checking.** S3 SHALL check a `constraint` clause as it checks an
  invariant body (FR-104): a Boolean state clause over a context object
  type, observing `current`, with the same refusals. A constraint that reads
  a population annotated `symmetric` SHALL also be checked by FR-150.
- **Truth at a state.** `holds_constraint` SHALL evaluate the clause with
  FR-107's `evaluate_clause` for each object of the state whose most-specific
  type conforms to the context type, in ascending reference-key order,
  charging every evaluation to the run's meter, as FR-152 and FR-153 charge
  the other reduction work. It SHALL return `Holds` when every evaluation is
  `Completed(true)`; `Undefined{object, reason}` for the first object in key
  order whose evaluation is `Undefined`; and otherwise `Boundary` when some
  evaluation is `Completed(false)`. When `holds_constraint` returns
  `Undefined`, the engine SHALL return
  `Violated` with cause `UndefinedEvaluation` at that state, with a
  counterexample ending there that replays (ADR-021 SC-2); that state is not
  a boundary state. A `Refused` evaluation SHALL stop the expansion with its
  refusal. An `Incomplete` evaluation is the run's meter running out, and the
  engine SHALL stop the run as a reached budget (FR-160). A `CallFailure` SHALL
  stop the expansion with `runtime_invariant`/`established-invariant-broken`.
- **Resolution.** The pre-check SHALL resolve the request's constraint name
  to a checked `constraint` clause of the subject's package; a name that
  resolves to none SHALL refuse `invalid_runtime_input`/`invalid-value`,
  naming it, before any expansion.
- **Boundary states.** The engine SHALL store, key, count toward
  `max_states`, and read with the property automaton and the safety monitor
  every product state whose model state does not satisfy the constraint,
  initial or reached, and SHALL leave each such state unexpanded, with no
  terminal stutter edge and no deadlock classification.
- **Expanded states.** The engine SHALL compute enabledness at an expanded
  state from the model, unaffected by the cut.
- **Outcome.** A safety violation found at any stored state, boundary states
  included, SHALL return `Violated` with its prefix. A run that finds no
  violation and reached at least one boundary state SHALL return
  `ConstraintReached{boundary_states}` (FR-160). When a run reached no
  boundary state, the engine SHALL return the outcome FR-126 states.
- **Identity.** The request writer SHALL keep the constraint out of the
  item's obligation identity, and FR-160's map SHALL record it in the
  terminal record's method.
- **Bounds.** A constraint SHALL NOT replace a bound: the pre-check still
  classifies every root (FR-126), and an unbounded root still returns
  `RequiresBound`.

## Acceptance Criteria

The ADR-021 §7.3 subject: `Counter` with `v: Int[0, 3]`, `bump()` framed
`modifies self.v`, pre `self.v < 3`, post `self.v = pre(self.v) + 1`;
universe `{a, b, c}`, all at 0; constraint `Low` (`self.v <= 1`); instance
`x = a`; no other reduction.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-158-AC-1 | The constrained run stores 20 states and expands 8: the 8 with every counter at 0 or 1, and 12 boundary states with one counter at 2. `always holds(x.v <= 1)` returns `Violated` at the boundary state `(2,0,0)` with prefix `bump(a), bump(a)`, the unconstrained run's prefix. | Test (TC-580) |
| FR-158-AC-2 | `always holds(x.v <= 2)` returns `ConstraintReached{boundary_states: 12}` (the unconstrained run returns `Violated` at `(3,0,0)`). `eventually holds(x.v = 3)` under the empty fairness set returns `ConstraintReached{12}`: no boundary state is stutter-extended, so no lasso is formed (the unconstrained run returns `Holds`). The deadlock-freedom item returns `ConstraintReached{12}`: no boundary state is a deadlock (the unconstrained run returns `Violated` at `(3,3,3)`). | Test (TC-580) |
| FR-158-AC-3 | A constraint that evaluates `Undefined` at a reached state settles the item `Violated` with cause `UndefinedEvaluation` and a replaying counterexample ending at that state, and that state is not a boundary state. A request naming an absent constraint refuses `invalid_runtime_input`/`invalid-value` naming it, with no state expanded. A constraint false at the initial state makes it a boundary state, and the run stores one state. With no universe for `counters`, the constrained run still returns `RequiresBound`. | Test (TC-581) |
| FR-158-AC-4 | The constrained and unconstrained requests for one claim have equal obligation identities and different request identities, no result of one settles the other, and the constrained run's terminal record names `Low` in its method. Repeating AC-1's run gives equal outcomes and equal boundary counts. | Test (TC-581) |

## Dependencies

- ADR-021 SC-1 to SC-3, RV-4, EI-2, EI-3, PT-2 (constraint column).
- [FR-104](FR-104-check-state-clauses.md) (invariant bodies),
  [FR-107](FR-107-evaluate-state-clauses-at-s6a.md) (`evaluate_clause`),
  [FR-124](FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md)
  (deadlocks), [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md)
  (terminal stutter), [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (the engine and its pre-check).
- QSpec owns the constraint clause's surface syntax, boundary states and the
  constraint's place in the method (ADR-021 QS-1, QS-2).

## References

- QSpec half: QSpec FR-382, FR-383 and FR-385 (Linear STD-134; ADR-021 §9 QS-1, QS-2). Owning ticket: Linear
  QSL-368.
