---
id: FR-126
title: "Check a temporal clause over every behaviour of a model (explicit-state)"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-015
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-123
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-124
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: depends_on
---
# FR-126: Check a temporal clause over every behaviour of a model (explicit-state)

## Description

QSL's layer-5 `model_check` module SHALL decide a temporal item over a model
subject by exploring the product of the subject's state graph with the
item's property automaton (ADR-018 EN-1). It runs at stage S6c over edge E10
(ADR-011 §1, as amended by ADR-018). The product is a FR-101
`TransitionSystem` explored by FR-101's canonical breadth-first engine,
which retains every product edge it explores. A safety form needs the first
phase only. A liveness form adds a second phase that decomposes the
retained graph into strongly connected components (SCCs) and looks for a
fair accepting cycle. The engine returns a `ModelCheckOutcome` that FR-127
settles.

## Use case

A verification operator requests a liveness claim with weak fairness over a
small model. The engine examines every reachable product state and either
proves the claim over that subject, or returns the canonical lasso that
violates it under the stated fairness, or says which limit it reached. Two
runs with the same subject, clause and limits give the same outcome and
the same counterexample.

## Inputs

```rust
pub struct ModelCheckRequest<'a> {
    pub subject: ModelSubject<'a>,          // FR-125
    pub item: ModelCheckItem<'a>,
    pub limits: ModelCheckLimits,
}

pub enum ModelCheckItem<'a> {
    Clause(&'a CheckedTemporalClause),      // FR-123, with its PropertyForm
    DeadlockFreedom,                        // FR-124
}

pub struct ModelCheckLimits {
    pub limits: Limits,                     // FR-101: max_states, max_depth, max_transitions
    pub max_automaton_states: u64,          // default 1_048_576 (2^20)
}

pub fn check_model(
    request: ModelCheckRequest<'_>,
    poll: impl FnMut() -> bool,
) -> Result<ModelCheckOutcome, ModelCheckRefusal>;
```

`ModelCheckRefusal` holds FR-106's `AdmissionFailure` from building the
subject's `ModelSystem`, or FR-101's `NotSimulated::RequiresBound`.

## Outputs

`ModelCheckOutcome` is one of:

- `Holds { basis: ProofBasis::Exhaustive }`: every reachable product state
  was examined and no violation or fair accepting cycle exists (V-1);
- `Violated(TemporalCounterexample)`: the canonical counterexample (§
  "Counterexamples"), to be replayed before it counts (FR-128) (V-4);
- `BoundReached { depth }`: `max_depth` reached with no counterexample (V-5);
- `Undecided(InconclusiveCause)`: `UndecidedSuccessor` or `NoInitialState`
  (V-6);
- `Stopped(IncompleteCause, ModelCheckLimit)`: `max_states`,
  `max_transitions`, `max_automaton_states`, a clause meter or cancellation
  stopped the run (V-7).

Each variant carries the run's statistics: product states, product edges,
automaton states and the depth reached.

## Behavior

### Pre-check

- Before any expansion, the engine SHALL classify every root of the subject
  (FR-120 `domains()` under the subject's universes). An unbounded root
  SHALL return `ModelCheckRefusal::RequiresBound` with FR-101's
  `RequiresBound`, and no state is explored.

### Property automaton

- For a `BoundedMltl` (TP-2) clause the engine SHALL build a deterministic
  finite monitor over positions. For an infinite-trace clause it SHALL build
  a generalized Büchi automaton for the negation of the formula. Past
  subformulas SHALL be tracked as monitor state.
- Each interval operator under infinite-trace SHALL be expanded into nested
  next-position steps before the automaton is built (ADR-018 IV-3):
  `eventually[a,b] p` to `X^a (p or X p or … or X^(b-a) p)`, `always[a,b]
  p` to the conjunction, `p until[a,b] q` with QSpec FR-091's lower-bound
  convention, `release[a,b]` as its dual.
- The engine SHALL materialize automaton states as the product exploration
  reaches them, counting distinct automaton states with checked arithmetic.
  When the count would exceed `max_automaton_states`, the engine SHALL stop
  and return `Stopped(ResourceExhausted, MaxAutomatonStates)`.
- A `DeadlockFreedom` item SHALL use the monitor of `always holds(not
  deadlocked)`, where `deadlocked` is FR-124's predicate.

### First phase

- The product SHALL be a `TransitionSystem` whose state is (model state,
  automaton state) and whose key is (FR-101 state key, automaton state
  index). Its initial states SHALL be each subject initial state paired with
  the automaton's successor from its initial state on position 0.
- The engine SHALL explore the product with FR-101's canonical
  breadth-first engine and retain every explored product edge.
- Reaching a rejecting monitor state SHALL end the first phase with a
  violation. A model state whose FR-120 expansion gives no successor and
  that FR-124 classifies as deadlocked SHALL end a `DeadlockFreedom` item's
  first phase with a violation.
- Under infinite-trace, a terminal model state SHALL contribute one terminal
  stutter edge from each of its product states (FR-125).
- An expansion that stops on an undecided contract conjunction (FR-120
  `ContractUndetermined`) SHALL return `Undecided(UndecidedSuccessor)`.
- A subject with no initial state SHALL return `Undecided(NoInitialState)`.
- Reaching `max_depth` with no violation SHALL return `BoundReached{depth:
  max_depth}`. Reaching `max_states`, `max_transitions`, a clause meter or a
  `true` poll SHALL return `Stopped` with that limit.
- A safety item (TP-1, TP-2, TP-3, deadlock-freedom) whose first phase
  completes with no violation SHALL return `Holds{basis: Exhaustive}`.

### Second phase (TP-4)

- The engine SHALL decompose the retained product graph into SCCs, in
  discovery order and never in hash-map iteration order.
- An SCC SHALL be a candidate when it is non-trivial (it has an edge) and,
  for each acceptance set of the generalized Büchi automaton, holds a state
  of that set.
- **Fairness filter.** A candidate SHALL pass when, for every constraint of
  the clause's fairness set, it holds an edge whose transition identity
  belongs to the constraint, or a state where the constraint is not
  enabled. A transition identity is enabled at a state when FR-120's
  successor relation gives it a successor there; a `Whole` constraint is
  enabled when any of its identities is; the stutter edge belongs to no
  constraint and enables none.
- The filter SHALL be one function over an SCC and the fairness set, with
  one rule per `FairnessKind`.
- A passing candidate SHALL return `Violated` with the canonical lasso. No
  passing candidate SHALL return `Holds{basis: Exhaustive}`.

### Counterexamples

- The canonical stem SHALL be the first path in FR-101 canonical
  breadth-first order to the first passing SCC. The canonical loop SHALL be
  the shortest cycle inside that SCC through a state of each acceptance set
  and an edge or disabled state for each fairness constraint, with ties
  broken by canonical transition order.
- A safety counterexample SHALL be the canonical path to the first violating
  product state.
- The counterexample SHALL be a `TemporalCounterexample` over the model
  subject: the index of its initial state in `subject.initial`; each step as
  its FR-120 transition identity and its post-state's
  `quire.simulation.state-key/v1` digest; the loop entry, when there is a
  loop; the terminal stutter marker, when the loop is the stutter step; the
  `over` binding; the fairness set; and `kind`: `Formula`, or `Deadlock` for
  a deadlock-freedom violation.
- Its length SHALL be its number of transitions, stem and loop together.

### Determinism

- The outcome and the counterexample SHALL be functions of the subject, the
  item and the limits.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-126-AC-1 | ADR-018 §6: over the example unit with universe `{a, b}` and both versions 0, `always eventually holds(c.versionNumber = 2)` under `fair weak each attemptUpdate` returns `Holds{Exhaustive}` with 15 product states; under `fair weak attemptUpdate` it returns `Violated` with an empty stem and the loop `(0,0) -upd(a)-> (1,0) -upd(a)-> (2,0) -upd(a)-> (0,0)` bound to `c = b`, length 3. Over the shipped unit (`VersionUnchanged`), under `fair weak each attemptUpdate` it returns `Violated` with loop `(0,0) -upd(a)-> (0,0) -upd(b)-> (0,0)`. | Test (TC-521) |
| FR-126-AC-2 | `eventually[0,5] holds(c.versionNumber = 2)` under event-position false-extension (TP-2) over the example subject returns `Violated` with a five-step finite prefix for `c = b` on which `vb` is never 2. `always holds(c.versionNumber <= 1000)` (TP-1) returns `Holds{Exhaustive}`. | Test (TC-521) |
| FR-126-AC-3 | Over the `Counter` subject with no `terminal` member, the `DeadlockFreedom` item returns `Violated` with `kind: Deadlock` and the prefix `0 -inc-> 1 -inc-> 2 -inc-> 3`; with `terminal when` covering value 3 it returns `Holds{Exhaustive}`. Under infinite-trace, `always eventually holds(c.value = 0)` returns `Violated` whose loop is the terminal stutter step at value 3, with the stutter marker set. | Test (TC-521) |
| FR-126-AC-4 | Over the `Health` subject (object `s`, fields `healthy: Bool` and `failures: Int[0, 1]`; operation `fail` with precondition `self.failures = 0` setting `healthy` false and `failures` 1; operation `recover` with precondition `not self.healthy` setting `healthy` true; `terminal any`; initial `healthy` true and `failures` 0), the recovery-stability formula `always (holds(not s.healthy) implies eventually always[0,2] holds(s.healthy))` returns `Holds{Exhaustive}`. In the `Restless` variant, where `fail` has no precondition, it returns `Violated` with a lasso on which `healthy` never holds at three consecutive positions. | Test (TC-521) |
| FR-126-AC-5 | Limits: the example subject with `max_depth` 1 returns `BoundReached{depth: 1}` for the TP-4 claim; with `max_states` 2, `Stopped(ResourceExhausted, MaxStates)`; a `true` poll, `Stopped(Cancelled, …)`. `always (holds(not s.healthy) implies eventually[0,100] holds(s.healthy))` over the `Restless` subject, where `fail` can repeat forever, with `max_automaton_states` 50 returns `Stopped(ResourceExhausted, MaxAutomatonStates)` with an automaton-state count of 50 and no counterexample; with the default limit it returns `Violated`, a prefix of at least 101 consecutive unhealthy positions. A subject with an undecided contract conjunction returns `Undecided(UndecidedSuccessor)`, and one with an unbounded population root returns `RequiresBound` before exploring. | Test (TC-521) |
| FR-126-AC-6 | Running AC-1's two requests twice each gives equal outcomes and byte-equal counterexamples. | Test (TC-521) |

## Dependencies

- ADR-018 §1, §3 EN-1, §4 FA-1 to FA-6, §5 CX-1 and CX-2, §10 DL-7, §11
  IV-3, IV-5 and IV-6; ADR-011 §1 and §6.1 (S6c, E10, layer 5
  `model_check`) as amended by ADR-018; ADR-016 §2 (the requires-bound
  pre-check).
- [FR-101](FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md)
  (engine, canonical order, state key, limits),
  [FR-120](FR-120-simulate-a-checked-package-s-state-family.md)
  (`ModelSystem`), [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md),
  [FR-124](FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md),
  [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md),
  [FR-075](FR-075-compute-candidates-from-registered-backends.md) (the EN-1
  provider manifest advertising (`temporal-satisfaction`, `bounded`) and
  (`temporal-satisfaction`, `unbounded`) registers through it).
- FR-127 settles the outcome; FR-128 replays the counterexample.
