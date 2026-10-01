---
id: FR-127
title: "Settle a model-check verdict as an FR-331 terminal record"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-015
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-069
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-072
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: depends_on
---
# FR-127: Settle a model-check verdict as an FR-331 terminal record

## Description

QSL SHALL settle every temporal item over a model subject as exactly one
FR-331 terminal record whose value states the verdict's strength (ADR-018
V-1 to V-8). `qsl-replay` owns the terminal record, its category map and the
additions here (ADR-013 O-24, as amended by ADR-018). `model_check` owns the
one map from `ModelCheckOutcome` (FR-126) and the replay result of its
counterexample (FR-128) to `TerminalValue`.

## Use case

A verification operator reads a run's results. A `proved` item says how it
was proved: exhaustive exploration, a bounded-complete unrolling, or
induction. A depth-limited search that found nothing says `inconclusive`
with its depth, never `proved`. A run that hit a limit says which. A
counterexample counts as `refuted` only after it replays.

## Inputs

- A `ModelCheckOutcome` (FR-126), or an outcome of the SMT temporal backend
  mapped by CG (ADR-018 DS-2) into the same values.
- For `Violated`: the FR-072 result of replaying its counterexample
  (FR-128), or the `ReplayRefusal` that stopped it.
- For an item negotiation did not route to a model-check engine: its
  negotiation disposition.

## Outputs

`TerminalValue` (`qsl_replay::proof_result`) with these additions:

```rust
pub enum ProofBasis {
    Checks { success_checks: u32 },   // Kani; zero is the vacuous proof
    Exhaustive,
    BoundedComplete { depth: u64 },
    Inductive { depth: u64 },
}

// TerminalValue::Proved { basis: ProofBasis }
// InconclusiveCause gains:
//   BoundReached { depth: u64 }, InductionNotClosed { depth: u64 },
//   UndecidedSuccessor, NoInitialState
```

and the FR-331 terminal record carrying the value, its QSpec FR-341
(infinite-trace) label, its QSpec FR-243 basis and its O-16 category.

## Behavior

- The map SHALL be exhaustive, with no `_` arm, and give each input exactly
  one row:

| Verdict | Input | QSpec FR-341 label | QSpec FR-243 basis | `TerminalValue` | O-16 category |
| --- | --- | --- | --- | --- | --- |
| V-1 | `Holds{Exhaustive}` | `proved` | `closed-scope` | `Proved{basis: Exhaustive}` | success |
| V-2 | SMT bounded-complete to `k` at or past a TP-2 `on origin` horizon | `proved` | `closed-scope` | `Proved{basis: BoundedComplete{depth: k}}` | success |
| V-3 | SMT `k`-inductive | `proved` | `decisive-witness` | `Proved{basis: Inductive{depth: k}}` | success |
| V-4 | `Violated` whose replay settles `reproduced-with-evaluated-witness` | `refuted` | `decisive-counterexample` | `Refuted` | violation |
| V-5 | `BoundReached{depth}` | `inconclusive` | `unsettled` | `Inconclusive(BoundReached{depth})` | inconclusive |
| V-6 | `Undecided(cause)`; SMT `InductionNotClosed{depth}`; `Violated` whose replay settles `inconclusive` (`ReplayParity`) or refuses (`ReplayRefused`) | `inconclusive` | `unsettled` | `Inconclusive(cause)` | inconclusive |
| V-7 | `Stopped(cause, limit)`, including `MaxAutomatonStates` | `failed`, execution `resource-incomplete` | `unavailable` | `Incomplete(cause)` | incomplete |
| V-8 | No candidate settles the form, or a profile the model subject does not admit | `unsupported` | `unavailable` | `Unsupported(cause)` | unsupported |

- `TerminalValue::category` SHALL map `Proved{basis: Checks{success_checks:
  0}}` to inconclusive with `KaniVacuousProof`, and every other `Proved`
  basis to success.
- A `Violated` outcome SHALL settle `refuted` only through its replay. A
  replay that faults SHALL settle `failed` (ADR-013 O-16 internal failure).
- A `Stopped` record SHALL name its limit, `max_automaton_states`
  included.
- A run that completes every depth up to `k` SHALL have execution
  `completed` and truth `pending` (V-5); a run that stops before completing
  `k` SHALL settle V-7 (ADR-014 B-5 as amended by ADR-018).
- The record SHALL carry the method (`explicit-state`, `smt-unrolling` or
  `k-induction`) and, for V-2, V-3, V-5 and `InductionNotClosed`, the depth.
- A `DeadlockFreedom` item SHALL settle through the same map; its V-4 record
  carries the counterexample whose `kind` is `Deadlock`.
- A verdict SHALL hold for exactly its subject; the record's obligation
  identity binds the subject (ADR-013 O-09).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-127-AC-1 | Each of the eight input rows maps to its `TerminalValue`, FR-341 label, FR-243 basis and O-16 category exactly as the table states, with V-2 at depth 5, V-3 at depth 2 and V-5 at depth 1 carrying their depths and method. `Proved{basis: Checks{success_checks: 0}}` maps to inconclusive `KaniVacuousProof` and `Checks{1}` to success. | Test (TC-522) |
| FR-127-AC-2 | FR-126-AC-1's `fair weak each` outcome settles `proved`, `closed-scope`, `Proved{Exhaustive}`, success. Its `fair weak` counterexample settles `refuted` only after FR-128 replay reproduces it; the same counterexample with one post-state digest altered settles `inconclusive`, `ReplayRefused`, and one whose formula evaluates `true` on replay settles `inconclusive`, `ReplayParity`. | Test (TC-522) |
| FR-127-AC-3 | FR-126-AC-5's `max_automaton_states` run settles `failed`, `resource-incomplete`, `unavailable`, `Incomplete(ResourceExhausted)`, category incomplete, naming `max_automaton_states`; its `max_depth` run settles `inconclusive`, `BoundReached{depth: 1}`, execution `completed`, truth `pending`. | Test (TC-522) |
| FR-127-AC-4 | FR-126-AC-3's deadlock-freedom violation settles `refuted` after replay, with a record whose counterexample `kind` is `Deadlock`; a claim under the fixed-sample profile over a model subject settles `unsupported`, `unsupported-requested-capability`. | Test (TC-522) |

## Dependencies

- ADR-018 §1 (V-1 to V-8, `ProofBasis`, the new causes, length, depth);
  ADR-013 O-16 and O-24 as amended by ADR-018; ADR-014 B-5 as amended.
- [FR-069](FR-069-implement-typed-proof-result-envelope.md) (terminal record
  and category map), [FR-072](FR-072-implement-typed-replay-result.md)
  (replay result), [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md),
  [FR-128](FR-128-replay-a-model-counterexample.md).
- QSpec owns the verdict table onto FR-341 (infinite-trace) and FR-243, the
  method and depth members of the FR-331 terminal record, and the new
  inconclusive causes on the wire (ADR-018 QS-6).
