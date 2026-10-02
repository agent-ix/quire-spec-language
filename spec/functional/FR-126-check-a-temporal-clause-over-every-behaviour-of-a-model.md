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

The `model_check` module of the `qsl-analyze` crate, layer A above the
qualified core (ADR-018 LA-1), SHALL decide a temporal item over a model
subject by exploring the product of the subject's state graph with the
item's property automaton (ADR-018 EN-1). It runs at stage S6c over edge E10
(ADR-011 §1, as amended by ADR-018). The product is a FR-101
`TransitionSystem` explored by FR-101's canonical breadth-first engine,
which retains every product edge it explores. A safety form needs the first
phase only. A liveness form adds a second phase that decomposes the
retained graph into strongly connected components (SCCs) and looks for a
fair accepting cycle. The engine returns a `ModelCheckOutcome`
(`qsl-replay`, ADR-018 LA-3) that FR-127 settles; a proof carries the
certificate FR-127's core checker verifies (ADR-018 PC-3, PC-4).

## Use case

A verification operator requests a liveness claim with weak fairness over a
small model. The engine examines every reachable product state and either
proves the claim over that subject, or returns the canonical lasso that
violates it under the stated fairness, or says which limit it reached, with
the limit's value and the request member that raises it. Two runs with the
same subject, clause and limits give the same outcome and the same
counterexample.

## Inputs

```rust
pub struct ModelCheckRequest<'a> {
    pub subject: ModelSubject<'a>,          // FR-125
    pub item: ModelCheckItem<'a>,
    pub max_depth: Option<usize>,           // the search horizon k
    pub limits: ModelCheckLimits,
}

pub enum ModelCheckItem<'a> {
    Clause(&'a CheckedTemporalClause),      // FR-123, with its PropertyForm
    DeadlockFreedom,                        // FR-124
}

pub struct ModelCheckLimits {
    pub limits: Limits,                     // FR-101: max_states, max_transitions
    pub max_automaton_states: u64,
}

pub fn check_model(
    request: ModelCheckRequest<'_>,
    poll: impl FnMut() -> bool,
) -> Result<ModelCheckOutcome, ModelCheckRefusal>;
```

`max_states`, `max_transitions` and `max_automaton_states` are
caller-raisable ADR-014 B-5 budgets of QSL's own provider (ADR-018 §1,
IV-6). `ModelCheckLimits::default()` publishes `max_states` 10,000,000,
`max_transitions` 100,000,000 and `max_automaton_states` 1,048,576 (2^20),
with `Limits` at FR-101's defaults. `max_depth` is the search horizon `k`
of a bounded search, a request member beside the limits and never one of
them: a run that completes it settles V-5, not V-7, and the result states
the `k` it used. `None` sets no horizon. The subject's
FR-120 evaluation meter budget and `ExpansionLimits` travel in the
`ModelSubject` (FR-125).

`ModelCheckRefusal` holds FR-106's `AdmissionFailure` from building the
subject's `ModelSystem`, or FR-101's `NotSimulated::RequiresBound`.

## Outputs

`ModelCheckOutcome` is one of:

- `Holds { basis: ProofBasis::Exhaustive, certificate: ProofCertificate }`:
  every reachable product state was examined and no violation or fair
  accepting cycle exists (V-1). `ProofCertificate` is
  `Closure(ClosureCertificate)` for a safety item and
  `Component(ComponentCertificate)` for a TP-4 item (ADR-018 PC-3, PC-4);
- `Violated(TemporalCounterexample)`: the canonical counterexample (§
  "Counterexamples"), to be replayed before it counts (FR-128) (V-4),
  including one whose `kind` is `UndefinedEvaluation{where, cause}`;
- `BoundReached { depth }`: the run completed its search horizon
  `max_depth`, `depth` being that horizon, with no counterexample (V-5);
- `Undecided(InconclusiveCause)`: `UndecidedSuccessor` or `NoInitialState`
  (V-6);
- `Stopped { cause: IncompleteCause, limit: Option<ReachedLimit> }`: a run
  limit or cancellation stopped the run before it completed its method
  (V-7). `ReachedLimit{limit: ModelCheckLimit, value: u64}` names the limit
  and its value. `ModelCheckLimit` is `MaxStates`, `MaxTransitions` or
  `MaxAutomatonStates` (members of `ModelCheckLimits`), or
  `EvaluationMeter` or `MaxCandidates` (members of the subject's FR-120
  limits), so it names the request member that raises it. Cancellation
  has `limit: None`.

Each variant carries the run's statistics: product states, product edges,
automaton states and the depth reached.

## Behavior

### Pre-check

- Before any expansion, the engine SHALL classify every root of the subject
  (FR-120 `domains()` under the subject's universes). If a root is
  unbounded, then the engine SHALL return
  `ModelCheckRefusal::RequiresBound` with FR-101's `RequiresBound` and
  explore no state.

### Property automaton

- For a `BoundedMltl` (TP-2) clause the engine SHALL build a deterministic
  finite monitor over positions, for activation `on origin` and for
  activation `on each` (ADR-018 TP-2).
- For a `ReachableInvariant` (TP-1) or `Safety` (TP-3) clause and for a
  `DeadlockFreedom` item, the engine SHALL build a deterministic bad-prefix
  monitor: the automaton of the formula itself, trimmed to the states from
  which some infinite run is accepting, then determinized by subset
  construction, whose one rejecting state, the empty subset, is reached
  exactly when the prefix read is a bad prefix (ADR-018 SM-6).
- For a `Liveness` (TP-4) clause the engine SHALL build a generalized Büchi
  automaton for the negation of the formula.
- The engine SHALL track past subformulas as automaton state, so the
  product needs no unrolling.
- The engine SHALL translate each interval operator under infinite-trace by
  ADR-018 IV-3's counter construction: for each occurrence it keeps the
  occurrence's open obligations, merged into one that subsumes them. For
  lower bound 0 over an operand with no interval operator the occurrence
  keeps one counter with `b + 1` values; a past interval operator with
  lower bound 0 keeps one counter of positions since its operand last held,
  saturating at `b + 1`; any other occurrence keeps a set of offsets in
  `[0, b]`.
- The engine SHALL materialize automaton states as the product exploration
  reaches them, counting distinct automaton states with checked arithmetic.
  If the count would exceed `max_automaton_states`, then the engine SHALL
  stop and return `Stopped{cause: ResourceExhausted, limit:
  Some({MaxAutomatonStates, value})}`.
- For a `DeadlockFreedom` item the engine SHALL use the monitor of `always
  holds(not deadlocked)`, where `deadlocked` is FR-124's classification.

### First phase

- The product SHALL be a `TransitionSystem` whose state is (model state,
  automaton state) and whose key is (FR-101 state key, automaton state
  index). Its initial states SHALL be each subject initial state paired with
  each automaton state in the automaton's set of successors of its initial
  state on position 0; a deterministic monitor has one, a generalized Büchi
  automaton may have several (ADR-018 §6's `(0, 0, q0)` and `(0, 0, q1)`).
- The engine SHALL explore the product with FR-101's canonical
  breadth-first engine and retain every explored product edge.
- For each position the claim reads (FR-125), the engine SHALL read the
  position's letter when it creates the product state for that position,
  before it steps the automaton on it.
- If an atom of the letter evaluates `Undefined`, then the engine SHALL end
  the first phase with a violation whose counterexample has `kind:
  UndefinedEvaluation{where, cause}` (ADR-018 UE-1, UE-2), for every
  property form and for a `DeadlockFreedom` item whose `terminal when`
  predicate evaluates `Undefined` at a terminal state.
- When the exploration reaches the monitor's rejecting state, the engine
  SHALL end the first phase with a violation.
- The engine SHALL end the first phase at the first product state in
  FR-101 canonical breadth-first order whose letter is undefined or whose
  monitor state rejects. An undefined position SHALL rank like any other
  violation: the shortest prefix first, then canonical transition order
  (ADR-018 UE-4).
- When a model state's FR-120 expansion gives no successor and FR-124
  classifies it as deadlocked, the engine SHALL end a `DeadlockFreedom`
  item's first phase with a violation (ADR-018 DL-7).
- Under a bounded profile, at a terminal model state the engine SHALL close
  a TP-2 monitor by the profile's closed-boundary rule: it reads the
  remaining positions of the horizon with every atomic predicate false and
  every constant unchanged (QSpec FR-091), and the item is violated when
  that closure reaches the rejecting state.
- Under infinite-trace, a terminal model state SHALL contribute one terminal
  stutter edge from each of its product states (FR-125).
- If an expansion records `ContractUndetermined` with an `Undefined`
  evaluation (an undefined precondition guard or postcondition
  conjunction, FR-120), then the engine SHALL end the first phase with a
  violation whose counterexample has `kind: UndefinedEvaluation{where,
  cause}`, `where` at the expanded state's position (ADR-018 UE-6).
- If an expansion records `ContractUndetermined` with only `Refused`
  evaluations, then the engine SHALL return
  `Undecided(UndecidedSuccessor)`.
- When the subject has no initial state, the engine SHALL return
  `Undecided(NoInitialState)`.
- If an expansion stops because a clause evaluation exhausted the
  subject's evaluation meter, or because it reached `max_candidates`, then
  the engine SHALL return `Stopped{cause: ResourceExhausted, limit:
  Some({EvaluationMeter | MaxCandidates, value})}`.
- When the exploration reaches `max_states` or `max_transitions`, the
  engine SHALL return `Stopped{cause: ResourceExhausted, limit:
  Some({MaxStates | MaxTransitions, value})}`. When the poll returns
  `true`, the engine SHALL return `Stopped{cause: Cancelled, limit: None}`.
- With `max_depth = Some(k)`, the engine SHALL expand every product state at
  depth below `k` and retain its edges. When it reaches depth `k` with no
  first-phase violation, a safety item SHALL return `BoundReached{depth:
  k}`, and a TP-4 item SHALL run the second phase over the retained graph
  and return `Violated` when a candidate passes, `BoundReached{depth: k}`
  otherwise.
- When a safety item's (TP-1, TP-2, TP-3, deadlock-freedom) first phase
  completes with an empty frontier and no violation, the engine SHALL
  return `Holds{basis: Exhaustive}` with a `ClosureCertificate` holding
  every explored product state: its model state's state-key digest and
  its automaton state's canonical key (ADR-018 PC-3).

### Second phase (TP-4)

- The engine SHALL run the second phase only when the first phase ended
  with no violation.
- The engine SHALL decompose the retained product graph into SCCs, in
  discovery order and never in hash-map iteration order.
- The engine SHALL treat an SCC as a candidate when it is non-trivial (it
  has an edge) and holds a state of every acceptance set of the
  generalized Büchi automaton.
- **Fairness filter.** The engine SHALL pass a candidate when, for every
  constraint of the clause's fairness set, it holds an edge whose
  transition identity belongs to the constraint, or a state where the
  constraint is not enabled (ADR-018 FA-4). A transition identity is
  enabled at a state when FR-120's successor relation gives it a successor
  there; a `Whole` constraint is enabled when any of its identities is; the
  stutter edge belongs to no constraint and enables none.
- The filter SHALL be one function over an SCC and the fairness set, with
  one rule per `FairnessKind` (ADR-018 FA-5).
- When a candidate passes, the engine SHALL return `Violated` with the
  canonical lasso. When the first phase completed with an empty frontier
  and no candidate passes, the engine SHALL return `Holds{basis:
  Exhaustive}`.

### Counterexamples

- The canonical stem SHALL be the first path in FR-101 canonical
  breadth-first order to the first passing SCC.
- The canonical loop SHALL be ADR-018 CX-5's greedy walk over the passing
  SCC `C` from its entry state `e`, the stem's last state. The obligations
  are one state of each acceptance set and each fairness obligation of
  `C`; a weak constraint's obligation is discharged by an edge of `C` that
  takes it or by a state of `C` where it is disabled. Starting at `e`, with
  the obligations `e` discharges removed, the walk repeatedly appends the
  path, inside `C`, to the nearest state or edge that discharges an open
  obligation (nearest by edge count, ties broken by canonical transition
  order, then by obligation order) and removes every obligation that path
  discharges. With none open, it appends the shortest path inside `C` back
  to `e`, ties broken by canonical transition order, or the shortest cycle
  through `e` when the loop would otherwise be empty.
- A safety counterexample SHALL be the canonical path to the first violating
  product state. An undefined-evaluation counterexample SHALL be the
  canonical path to the first product state whose letter is undefined, for
  every property form; its `where` SHALL be that state's position and its
  `cause` the `UndefinedRecord` FR-125 returns there (ADR-018 UE-3). A deadlock counterexample SHALL be the canonical path to
  the first deadlocked state (ADR-018 DL-4).
- The counterexample SHALL be a `TemporalCounterexample` over the model
  subject: the index of its initial state in `subject.initial`; its steps in
  the `CounterexampleSteps::Model` arm (FR-128), each step as its FR-120
  transition identity and its post-state's
  `quire.simulation.state-key/v1` digest, which selects the step's
  successor among the post-states of its transition identity; the loop
  entry, when there is a loop; the terminal stutter marker, when the loop
  is the stutter step; the `over` binding; the fairness set; and `kind`:
  `Formula`, `Deadlock` for a deadlock-freedom violation, or
  `UndefinedEvaluation{where, cause}`.
- Its length SHALL be its number of transitions, stem and loop together.

### Certificates

- For a TP-4 item that holds, the engine SHALL return a
  `ComponentCertificate`: the `ClosureCertificate` of every explored product
  state, and the second phase's SCCs in topological order, each with the
  first witness that holds for it in the order `Trivial`,
  `MissingAcceptance{set}` (the lowest such set), `UnfairWeak{constraint}`
  (the first such constraint of the fairness set) (ADR-018 PC-4).
- The certificate SHALL be a function of the subject, the item and the
  limits.

### Determinism

- The outcome and the counterexample SHALL be functions of the subject, the
  item and the limits.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-126-AC-1 | ADR-018 §6: over the example unit with universe `{a, b}` and both versions 0, `always eventually holds(c.versionNumber = 2)` under a weak `each` constraint on `attemptUpdate` returns `Holds{Exhaustive}` with 15 product states; under a weak constraint with no granularity it returns `Violated` with an empty stem and the loop `(0,0) -upd(a)-> (1,0) -upd(a)-> (2,0) -upd(a)-> (0,0)` bound to `c = b`, length 3. Over the shipped unit (`VersionUnchanged`), under the weak `each` constraint it returns `Violated` with loop `(0,0) -upd(a)-> (0,0) -upd(b)-> (0,0)`. | Test (TC-521) |
| FR-126-AC-2 | `eventually[0,5] holds(c.versionNumber = 2)` under event-position false-extension, `on origin` (TP-2), over the example subject returns `Violated` with a five-step finite prefix for `c = b` on which `vb` is never 2. `always holds(c.versionNumber <= 1000)` (TP-1) returns `Holds{Exhaustive}`. Over the `Counter` subject (FR-124-AC-1, no `terminal` member), `eventually[0,3] holds(c.value = 3)` under event-position false-extension `on each` returns `Holds{Exhaustive}`, and `eventually[0,1] holds(c.value = 3)` `on each` returns `Violated` with the prefix `0 -inc-> 1`. | Test (TC-521) |
| FR-126-AC-3 | Over the `Counter` subject with no `terminal` member, the `DeadlockFreedom` item returns `Violated` with `kind: Deadlock` and the prefix `0 -inc-> 1 -inc-> 2 -inc-> 3`; with a `When` member covering value 3 it returns `Holds{Exhaustive}`. Under infinite-trace, `always eventually holds(c.value = 0)` returns `Violated` whose loop is the terminal stutter step at value 3, with the stutter marker set. | Test (TC-521) |
| FR-126-AC-4 | Over the `Health` subject (object `s`, fields `healthy: Bool` and `failures: Int[0, 1]`; operation `fail` with precondition `self.failures = 0` setting `healthy` false and `failures` 1; operation `recover` with precondition `not self.healthy` setting `healthy` true; `terminal any`; initial `healthy` true and `failures` 0), the recovery-stability formula `always (holds(not s.healthy) implies eventually always[0,2] holds(s.healthy))` returns `Holds{Exhaustive}`. In the `Restless` variant, where `fail` has no precondition, it returns `Violated` with a lasso on which `healthy` never holds at three consecutive positions. | Test (TC-521) |
| FR-126-AC-5 | Limits over the example subject and the TP-4 claim of AC-1 with no granularity: `max_depth` 2 returns `BoundReached{depth: 2}`, since the length-3 loop needs an edge out of depth 2; `max_depth` 3 returns `Violated` with the AC-1 loop, found by the second phase over the retained graph although depth 3 left a frontier; `max_states` 2 returns `Stopped{ResourceExhausted, {MaxStates, 2}}`; `max_transitions` 3 returns `Stopped{ResourceExhausted, {MaxTransitions, 3}}`; a `true` poll returns `Stopped{Cancelled, None}`. Over the `Counter` subject with an evaluation meter budget of zero, the `inc` precondition's evaluation stops the run with `Stopped{ResourceExhausted, {EvaluationMeter, 0}}`. | Test (TC-521) |
| FR-126-AC-6 | `always (holds(not s.healthy) implies eventually[0,100] holds(s.healthy))` over the `Restless` subject, where `fail` can repeat forever, with `max_automaton_states` 50 returns `Stopped{ResourceExhausted, {MaxAutomatonStates, 50}}` with an automaton-state count of 50 and no counterexample; with the default limit it returns `Violated`, a finite prefix with at least 101 consecutive unhealthy positions. A subject whose contract conjunction is refused, with no clause false and none undefined, returns `Undecided(UndecidedSuccessor)`; over FR-120-AC-9's `test/tallies` subject from `t1`, whose `pre Low` evaluates undefined, `always holds(true)` returns `Violated` with the empty prefix at `t1` and `kind: UndefinedEvaluation` with cause `SumOutOfDomain` at position 0; and one with an unbounded population root returns `RequiresBound` before exploring. | Test (TC-521) |
| FR-126-AC-7 | Running AC-1's two requests twice each gives equal outcomes and byte-equal counterexamples. | Test (TC-521) |
| FR-126-AC-8 | `ModelCheckLimits::default()` is `max_states` 10,000,000, `max_transitions` 100,000,000, `max_automaton_states` 1,048,576, with no depth member; a request with `max_depth: None` sets no horizon. AC-1's requests with the default limits return the AC-1 outcomes. A run stopped by `max_states` 2 with the other members at their defaults returns `Stopped{ResourceExhausted, {MaxStates, 2}}`, naming the `ModelCheckLimits` member that raises it. | Test (TC-536) |
| FR-126-AC-9 | Over the `Counter` subject (no `terminal` member), `always holds(6 / (2 - c.value) >= 0)` under infinite-trace returns `Violated` with the prefix `0 -inc-> 1 -inc-> 2` and `kind: UndefinedEvaluation{where: position 2, cause: division-by-zero}`. The TP-4 claim `always eventually holds(6 / (2 - c.value) = 6)` returns the same counterexample, not the terminal stutter lasso. `always (holds(c.value <= 0) and holds(6 / (2 - c.value) >= 0))` returns `kind: Formula` with the prefix `0 -inc-> 1`, which violates before position 2. `eventually[0,1] holds(6 / (2 - c.value) = 6)` under event-position false-extension, `on origin`, returns `Holds{Exhaustive}`. With a `terminal when 6 / (3 - c.value) = 0` member, the `DeadlockFreedom` item returns `Violated` with the prefix to value 3 and `kind: UndefinedEvaluation{where: position 3, cause: division-by-zero}`. | Test (TC-537) |

## Dependencies

- ADR-018 §1 (with UE-1 to UE-4), §2 SM-6, §3 EN-1 (its explicit-state limits and determinism),
  §4 FA-1 to FA-6, §5 CX-1, CX-2 and CX-5, §10 DL-4 and DL-7, §11 IV-3, IV-5
  and IV-6; ADR-011 §1 and §6.1 (S6c, E10, layer A `qsl-analyze` `model_check`) as amended
  by ADR-018; ADR-014 §1 B-5 as amended; ADR-016 §2 (the requires-bound
  pre-check).
- [FR-101](FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md)
  (engine, canonical order, state key, limits),
  [FR-120](FR-120-simulate-a-checked-package-s-state-family.md)
  (`ModelSystem`, evaluation meter, `ExpansionLimits`),
  [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md),
  [FR-124](FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md),
  [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md),
  [FR-075](FR-075-compute-candidates-from-registered-backends.md) (the EN-1
  provider manifest advertising (`temporal-satisfaction`, `bounded`) and
  (`temporal-satisfaction`, `unbounded`) registers through it).
- FR-127 settles the outcome; FR-128 replays the counterexample.
