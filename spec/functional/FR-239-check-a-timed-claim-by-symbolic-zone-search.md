---
id: FR-239
title: "Check a timed claim by symbolic zone search (EN-6)"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-026
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-231
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-232
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-234
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-238
    type: depends_on
---
# FR-239: Check a timed claim by symbolic zone search (EN-6)

## Description

QSL's native zone engine EN-6, module `zone_check`, SHALL decide a timed
item over a timed subject by a passed/waiting search of symbolic states
`(s, q, Z)` (discrete state, claim-automaton state, zone), with
coverage by the aLU simulation, under caller-set budgets (ADR-026 EZ-1,
EZ-3 to EZ-5). It sits beside ADR-018's `model_check`, outside the
qualified core, runs at stage S6c over E10, and registers a provider
manifest advertising (`temporal-satisfaction`, `bounded`) and
(`temporal-satisfaction`, `unbounded`) for timed subjects. It decides TT-1
to TT-4, the deadlock-freedom item and the time-lock-freedom item, and
returns a counterexample (FR-241) or a certificate (FR-244).

## Use case

A verification operator requests a timed safety claim over a model with
real-valued clocks. The engine explores every reachable symbolic state
under a finite abstraction, proves the claim with a certificate, refutes
it with a concrete counterexample, or names the budget it reached. Two
runs with the same subject, item and budgets give the same outcome.

## Inputs

```rust
pub struct ZoneCheckRequest<'a> {
    pub subject: ModelSubject<'a>,          // timed subject, FR-231
    pub item: ZoneCheckItem<'a>,            // Clause | DeadlockFreedom | TimeLockFreedom
    pub limits: ZoneCheckLimits,
}

pub struct ZoneCheckLimits {
    pub max_symbolic_states: u64,   // default 1_048_576 (2^20)
    pub max_transitions: u64,       // default 16_777_216 (2^24)
    pub max_automaton_states: u64,  // default 1_048_576 (2^20)
    pub max_zone_bytes: u64,        // default 1_073_741_824 (2^30)
}

pub fn check_timed_model(
    request: ZoneCheckRequest<'_>,
    poll: impl FnMut() -> bool,
) -> Result<ZoneCheckOutcome, ModelCheckRefusal>;
```

Time and cancellation reach the engine through `poll`, which the caller
owns.

## Outputs

`ZoneCheckOutcome` is one of `Holds(ZoneCertificate)`,
`HoldsDigitized` (FR-243), `Violated(TemporalCounterexample)`,
`Undecided(InconclusiveCause)` or `Stopped(IncompleteCause,
ZoneCheckLimit)`, each with the run's statistics: symbolic states, retained
edges, automaton states and stored zone bytes.

## Behavior

### Symbolic states and successors

- The initial symbolic state SHALL be the zero valuation, let time elapse
  unless `s0` is urgent, intersected with `s0`'s time invariants, paired
  with the claim automaton's initial state.
- The successor by transition identity `t` SHALL intersect `Z` with one
  convex part of `t`'s guard, apply the resets, intersect with the target's
  time invariants, let time elapse unless the target is urgent, and
  intersect with the time invariants again. A guard with several convex
  parts SHALL give one successor per part. An empty zone SHALL give no
  successor.
- Transition identities and their discrete successors SHALL come from
  FR-120's successor relation, in FR-101 canonical transition order.

### Abstraction

- For each clock and discrete state, the engine SHALL compute lower and
  upper bounds `L` and `U` from the constants compared with that clock in
  the model and the claim automaton, by static analysis.
- The engine SHALL store zones unextrapolated and SHALL treat a new symbolic
  state as covered when a stored state with equal `s` and `q` has a zone
  whose aLU abstraction includes the new zone.

### Search

- The engine SHALL explore breadth-first in canonical transition order. A
  covered symbolic state SHALL not be explored, and its covering edge SHALL
  be retained. Retained edges form the symbolic graph that FR-240 and
  FR-244 read.
- A `TimedInvariant` or `BoundedWindow` item, or a deadlock-freedom item,
  SHALL be decided by this search alone; a violation SHALL end the search
  and be concretized by FR-241.
- **Budgets.** When a count would exceed `max_symbolic_states`,
  `max_transitions`, `max_automaton_states` or `max_zone_bytes`, the engine
  SHALL stop and return `Stopped(ResourceExhausted, limit)` naming the limit
  and its value. When `poll` returns `true`, it SHALL return
  `Stopped(Cancelled, …)`.
- If an atom of the claim evaluates `Undefined` at a point of a symbolic
  state the search creates, then the engine SHALL end the search with
  `Violated` at the first such symbolic state in canonical breadth-first
  order, concretized by FR-241 to a point where it is undefined, with
  `kind: UndefinedEvaluation{where, cause}` (ADR-026 TV-1, ADR-018 UE-1). An
  undefined position SHALL rank like any other violation.
- An expansion that stops on an undecided contract conjunction SHALL return
  `Undecided(UndecidedSuccessor)`; a subject with no initial state,
  `Undecided(NoInitialState)`.
- An unbounded population root SHALL refuse `RequiresBound` before any
  state is explored, as FR-126 states.
- A run that completes with no violation SHALL return `Holds` with the
  certificate FR-244 builds from the retained graph.

### Determinism

- The outcome, counterexample and certificate SHALL be functions of the
  subject, the item and the limits.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-239-AC-1 | Over `Rpc` with `T = 4 ms`, `NoLateReply` (under `model-steps`, read over the timed subject) returns `Holds` with a certificate; with `T = 3 ms` it returns `Violated` at `trace_position` 3. The deadlock-freedom item returns `Holds` under both values. | Test (TC-694) |
| FR-239-AC-2 | ADR-026 §9's retry model (heartbeat while `x < 1`, retry when `x > 1 and y < 1`) returns `Violated` for `always holds(not retried)`, though the same model read over integer clocks has no reachable retry. | Test (TC-694) |
| FR-239-AC-3 | A model whose clock `x` is compared only with 5 and grows without bound in a loop reaches a finite number of stored symbolic states, and the same model with the constant `10^12` reaches the same number. | Test (TC-694) |
| FR-239-AC-4 | `Settles` over `Rpc` with `max_symbolic_states` 2 returns `Stopped(ResourceExhausted, MaxSymbolicStates)` naming the value 2; with a poll that returns `true`, `Stopped(Cancelled, …)`; a subject with an unbounded population root refuses `RequiresBound` with no state explored. Two runs of each request give equal outcomes and byte-equal certificates and counterexamples. | Test (TC-694) |
| FR-239-AC-5 | Over a `Ticker` model (`time dense`; object `t` with `n: Int[0, 2]` and clock `x`; operation `step` with precondition `self.n < 2`, guard `x >= 1`, reset `x` and postcondition `self.n = pre(self.n) + 1`; time invariant `x <= 1`; initial `n = 0`) and `always holds(2 / (2 - t.n) >= 1)` under `model-time`, the search returns `Violated` with the timed prefix `step` after delay 1, `step` after delay 1, and `kind: UndefinedEvaluation{where: 2, cause: division-by-zero}`, the position at time stamp 2. | Test (TC-710) |

## Dependencies

- ADR-026 §8 EZ-1, EZ-3 to EZ-5; ADR-018 EN-1, S6c and E10; ADR-014 B-5.
- [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md),
  [FR-101](FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md),
  [FR-075](FR-075-compute-candidates-from-registered-backends.md) (the
  provider manifest registers through it),
  [FR-231](FR-231-read-a-timed-subject-s-behaviours-as-timed-traces.md),
  [FR-232](FR-232-derive-the-time-lock-freedom-item-and-read-deadlocks-over-time.md),
  [FR-234](FR-234-check-timed-intervals-and-classify-timed-property-forms.md),
  [FR-238](FR-238-represent-zones-as-difference-bound-matrices.md).

## References

- Engine placement relative to the qualified core: ADR-029 CB-2 and RU-2
  (draft).
- G. Behrmann, P. Bouyer, K. G. Larsen and R. Pelánek, 2006; F. Herbreteau,
  B. Srivathsan and I. Walukiewicz, 2012 (ADR-026 References).
