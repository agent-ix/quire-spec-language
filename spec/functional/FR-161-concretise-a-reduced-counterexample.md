---
id: FR-161
title: "Concretise a counterexample found by a reduced search"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-019
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-021
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-152
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-153
    type: depends_on
---
# FR-161: Concretise a counterexample found by a reduced search

## Description

A counterexample found over a symmetry-reduced graph runs through canonical
states. When `model_check` emits it, QSL SHALL first turn it into a concrete
model trace of the subject: it follows the edge permutations to recover
each concrete step, and repeats a lasso's loop until the concrete state
returns to the concrete entry (ADR-021 EI-6). With an `each` fairness
constraint, the loop is built from the SCC's cycle group so that it takes
every `each` identity it must (AQ-7). A path found under partial-order
reduction or a state constraint is already a path of the subject. Every
counterexample is an ordinary FR-126 `TemporalCounterexample` and replays
through FR-128 with no reduction (EI-7).

## Use case

A verification operator receives a refutation from a symmetry-reduced run
and replays it. The trace names real objects and real steps, and the
replayer needs to know nothing about symmetry (US-019).

## Inputs

- The canonical counterexample over the reduced graph (FR-126 chooses it
  there): a stem and, for a lasso, a loop, each a list of retained edges
  (source, transition identity, canonical target, `π`); the subject's
  initial states; and, with an `each` constraint, the passing SCC's
  `CycleGroup` and witnesses (FR-153).

## Outputs

A `TemporalCounterexample` (FR-126) whose steps are concrete transition
identities and concrete post-state digests, with no canonical state,
permutation or reduction in it.

## Behavior

- **Start.** The concrete start SHALL be the first initial state in subject
  order whose canonical form is the stem's first state. With `π0` the
  permutation the canonicaliser returned for it, `σ` SHALL start as
  `π0⁻¹`, so that the concrete state is `σ(canonical state)`.
- **Steps.** Each retained edge (source, `t`, target, `π`) SHALL become the
  concrete step `σ(t)`, with the concrete post-state `σ ∘ π⁻¹ (target)`, and
  `σ` SHALL become `σ ∘ π⁻¹`.
- **Loop closure.** For a lasso, after one pass of the loop the engine SHALL
  compare the concrete state with the concrete entry state, and repeat the
  loop until they are equal. The engine SHALL make at most as many passes as
  the order of the loop's composed permutation, and SHALL count each step
  against `max_transitions`.
- **`each` loops.** With an `each` constraint, the loop SHALL be built as
  ADR-021 AQ-7 states: obligations in canonical order (acceptance sets, then
  `whole` constraints, then each `each` identity); for an `each` identity
  `e`, a witness `x` at node `c` and `h` in `H_C` with `σ0 h u_c(x) = e`,
  `h` written as a word in the generators by breadth-first search over the
  generators in canonical edge order; the closed walk of each generator in
  the word, then the tree path to `c`, then `x` (or a stay at `c` when `x` is
  disabled there); return to the entry by the tree. The whole loop SHALL be
  repeated until the concrete state equals the concrete entry state.
- **Already concrete.** Under partial-order reduction or a state constraint
  alone, the engine SHALL emit the path unchanged.
- **Length and limits.** The counterexample's length SHALL be its number of
  concrete transitions. Building it SHALL count against the run's
  `max_transitions` and meter; reaching either SHALL settle V-7. No length is
  capped otherwise.
- **Replay.** FR-128 SHALL replay a concretised counterexample unchanged,
  including the fairness re-check of every `each` identity (ADR-019 SR-8).
- **Determinism.** The concrete counterexample SHALL be a function of the
  reduced counterexample and the subject.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-161-AC-1 | ADR-021 §7.1 under `fair weak whole attemptUpdate`, instance `c = a`: the reduced loop `(0,0,0) -upd(b)-> (0,0,1)` (`π = (b c)`), `-upd(c)-> (0,0,2)`, `-upd(c)-> (0,0,0)` concretises to `upd(b)`, `upd(b)`, `upd(b)` through `(0,1,0)`, `(0,2,0)`, `(0,0,0)`, closing after one pass, and replays through FR-128 to `reproduced-with-evaluated-witness`. | Test (TC-585) |
| FR-161-AC-2 | A loop that needs two passes: the `Token` unit has object type `Holder` with `tok: Boolean`, population `holders` annotated `symmetric`, universe `{a, b}`, one class `[a, b]`, initial state `a.tok` false and `b.tok` true, and `pass()` (frame `modifies tok`, pre `self.tok`, post `not self.tok and (forall o in holders: o != self implies o.tok)`). The claim `eventually holds(forall x in holders: x.tok)` has no `over` parameter. The quotient has one state with a `pass(b)` self-loop whose `π` is `(a b)`. The concretised lasso has an empty stem and the loop `pass(b)`, `pass(a)`, two passes of the quotient loop, and it replays. | Test (TC-585) |
| FR-161-AC-3 | ADR-021 §7.1's `each` refutation (`eventually always holds(c.versionNumber != 2)` under `fair weak each attemptUpdate`, instance `c = a`) concretises to a loop from `(0,0,0)` back to `(0,0,0)` that takes `upd(a)`, `upd(b)` and `upd(c)` and passes `a.versionNumber = 2`. FR-128 replays it, confirms each `each` identity taken in the loop, and settles `reproduced-with-evaluated-witness`. | Test (TC-586) |
| FR-161-AC-4 | FR-156-AC-3's partial-order counterexample and FR-158-AC-1's constrained counterexample are emitted unchanged and replay. No concretised counterexample in AC-1 to AC-4 holds a canonical state, a permutation or a reduction name. Each one decodes as the same `TemporalCounterexample` type as an unreduced run's. Concretising AC-1 twice gives byte-equal counterexamples. | Test (TC-586) |

## Dependencies

- ADR-021 EI-6, EI-7, AQ-7, TX-5; ADR-018 CX-2, CX-3; ADR-019 SR-8.
- [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (the canonical counterexample and its type), [FR-128](FR-128-replay-a-model-counterexample.md)
  (replay), [FR-152](FR-152-canonicalise-a-model-state-by-sorting-its-symmetry-classes.md)
  (permutations), [FR-153](FR-153-decide-each-fairness-on-the-annotated-quotient.md)
  (cycle groups and witnesses).
- QSpec FR-181-AC-5 and the counterexample contract hold unchanged for a
  concretised trace (ADR-021 QS-7).

## References

- QSpec half: QSpec FR-383 (Linear STD-134; ADR-021 §9 QS-7, QS-10 (d)). Owning ticket:
  Linear QSL-368.
