---
id: FR-142
title: "Check a refinement's safety half on the explicit-state product"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-018
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-020
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-141
    type: depends_on
---
# FR-142: Check a refinement's safety half on the explicit-state product

## Description

QSL's layer-5 `model_check` module SHALL decide the safety half of a
refinement (ADR-020 RS-2 to RS-6, CO-1) on the explicit-state product
(ADR-020 RE-1, ADR-018 EN-1) at stage S6c over edge E10. The product's state
is the concrete state, its history values and, when the refinement has
hidden fields, the AX-2 set of consistent abstract states. Its first phase
runs `check_initial` on each initial state and `check_step` (FR-141) on
each product edge it explores, in FR-101's canonical breadth-first order,
and stops at the first failure. The liveness half is FR-143.

## Use case

A specification author requests a refinement. The checker explores every
reachable concrete state with its mapped abstract state, and either proves
that every concrete step is an abstract step or a stutter, or returns the
shortest prefix that ends at the first step the abstract model forbids,
with the reason.

## Inputs

```rust
pub struct RefinementCheckRequest<'a> {
    pub refinement: &'a CheckedRefinement,       // FR-135
    pub concrete: ModelSubject<'a>,              // FR-125, or a protocol subject (ADR-027 PS-1)
    pub abstract_package: &'a CheckedPackage,    // declaring package or dependency (ADR-020 RM-1)
    pub abstract_initial: Vec<DocumentRef>,      // the abstract subject's FR-106 snapshots
    pub limits: ModelCheckLimits,                // FR-126
}

pub fn check_refinement(
    request: RefinementCheckRequest<'_>,
    poll: impl FnMut() -> bool,
) -> Result<RefinementOutcome, ModelCheckRefusal>;
```

## Outputs

- `RefinementOutcome{safety: ModelCheckOutcome, liveness:
  Option<LivenessHalf>}` (FR-143 fills `liveness`; FR-144 settles both).
  `ModelCheckOutcome` is FR-126's, with `Undecided` carrying the cause
  `MappingUndetermined` (ADR-020 RE-4) as well as FR-126's causes.
- A `Violated` safety outcome carries a `TemporalCounterexample` over the
  concrete subject (FR-126) with its `refinement` member set to the
  `RefinementFailure` (ADR-020 RC-1) and `kind: Formula`. It holds concrete
  steps only.

## Behavior

### Subjects and pre-check

- The checker SHALL build the abstract subject from `abstract_package`, its
  initial snapshots and the derived universes: each abstract population's
  universe SHALL be the concrete universe of its source population (ADR-020
  RM-2).
- Before any expansion it SHALL classify every root of both subjects, and
  the parameter domain of every abstract parameter a step row leaves `_`, by
  FR-126's pre-check; an unbounded root SHALL return
  `ModelCheckRefusal::RequiresBound` and explore nothing.

### Product

- The product SHALL be an FR-101 `TransitionSystem` whose state is
  (concrete state, history values, candidates) and whose key is (the
  concrete subject's state key, the history values, the candidates' sorted
  abstract state keys). Two product states with equal keys SHALL be one
  state.
- Its initial states SHALL be, for each concrete initial state, the
  position `check_initial` gives. A `Fails` from `check_initial` SHALL end
  the phase with `Violated`, failure `InitialNotAbstract{initial}` and an
  empty prefix.
- Its successors SHALL be, for each concrete successor FR-120 (or
  `ProtocolSystem`, ADR-027 TS-1) gives, the `post` position of
  `check_step`, and, under infinite-trace, one terminal stutter edge from
  each product state whose concrete state is terminal (FR-125), decided by
  `check_step` as a `stutter` step.
- The checker SHALL explore the product with FR-101's canonical
  breadth-first engine and retain every explored edge for FR-143.

### Verdicts

- The first edge, in canonical breadth-first order, whose `check_step`
  returns `Fails` SHALL end the phase with `Violated`; the counterexample's
  prefix SHALL be the canonical path to that edge's pre-state followed by
  the edge, and its failure the one `check_step` returned, with `position`
  the index of the failing step's post-state.
- `Undetermined(MappingUndetermined)` or `Undetermined(UndecidedSuccessor)`
  from `check_initial` or `check_step` SHALL return `Undecided` with that
  cause.
- A concrete subject with no initial state SHALL return
  `Undecided(NoInitialState)`.
- The checker SHALL apply FR-126's limits and poll with FR-126's outcomes:
  `BoundReached{depth}` at `max_depth` with no failure, and `Stopped`
  naming the limit and its value for `max_states`, `max_transitions`,
  `max_candidates` or the evaluation meter, or `Cancelled` for a `true`
  poll. `max_transitions` SHALL also count the abstract successors that
  `check_step` computes.
- A phase that explores every reachable product state with no failure
  SHALL return `Holds{basis: Exhaustive}` for the safety half.

### Determinism

- The outcome and the counterexample SHALL be functions of the request.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-142-AC-1 | ADR-020 §8's `CasRefinesCounter`, universe `counters = {c}`, returns safety `Holds{Exhaustive}` with 28 product states. | Test (TC-547) |
| FR-142-AC-2 | The lost-update model (both commit preconditions `self.busyX and self.tmpX < 2`, no `retry` operations or rows; 36 reachable concrete states) returns `Violated` with prefix `beginA`, `beginB`, `commitA`, `commitB` and failure `AbstractStepRejected{position: 4, transition: inc(c), cause: Postcondition}`. The same model with both commit rows `-> any` returns `Violated` with a six-step prefix and failure `NoAbstractMatch{position: 6}`, whose last step lowers `value` from 2 to 1. | Test (TC-547) |
| FR-142-AC-3 | The compare-and-set model with concrete initial `value` 1 returns `Violated`, `InitialNotAbstract{initial: 0}`, empty prefix. With `commitA` mapped `-> stutter` it returns `Violated` with prefix `beginA`, `commitA` and `StutterChanged{position: 2}`. | Test (TC-547) |
| FR-142-AC-4 | `RegisterHistory` (FR-138) returns `Holds{Exhaustive}` with 4 product states. With the update `on write: self.value` it returns `Violated`, `AbstractStepRejected{position: 1, transition: write(r, 1), cause: Postcondition}`. With a second history field `writes: Int[0, 1] = 0 { on Impl::Register::write: pre(self.writes) + 1; }`, which no row reads, it returns `Undecided(MappingUndetermined)` naming `writes`, at the second write of a behaviour. | Test (TC-547) |
| FR-142-AC-5 | `Coin` (FR-141-AC-4) with `side` hidden returns `Holds{Exhaustive}`; with the row `side = self.face` it returns `Violated` with prefix `toss`, `reveal` to `face = 1` and `AbstractStepRejected{position: 2, transition: show(c), cause: Frame{…}}`. | Test (TC-547) |
| FR-142-AC-6 | `RingIsQueue` with universes `rings = {r}`, `slots = {s0, s1}` returns `Holds{Exhaustive}`; with the broken `take` it returns `Violated` with `AbstractStepRejected{…, transition: deq(r), cause: Postcondition}`; with FR-140-AC-3's `only` it returns `Undecided(MappingUndetermined)` naming the `items` row. | Test (TC-547) |
| FR-142-AC-7 | `CasRefinesCounter` with `max_states` 5 returns `Stopped{ResourceExhausted, {MaxStates, 5}}`; with `max_depth` 2, `BoundReached{depth: 2}`; with a `true` poll, `Stopped{Cancelled, None}`. Running AC-2's two requests twice gives equal outcomes and byte-equal counterexamples. | Test (TC-547) |

## Dependencies

- ADR-020 §2 RS-2 to RS-6, §3 CO-1, §4 AX-2 to AX-4, §5 RE-1 and RE-4, §6
  RC-1; ADR-018 §3 EN-1; ADR-011 §1 S6c and E10 as amended by ADR-018;
  ADR-027 TS-1 and TS-6.
- [FR-101](FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md),
  [FR-120](FR-120-simulate-a-checked-package-s-state-family.md),
  [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md),
  [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (product, outcome, counterexample and limits),
  [FR-141](FR-141-decide-one-concrete-step-against-the-abstract-model.md).
- FR-143 runs the second phase on the retained graph; FR-144 settles the
  outcome; FR-145 replays the counterexample.

## References

- The QSpec half (semantics and the `RefinementFailure` wire): Linear
  STD-133.
