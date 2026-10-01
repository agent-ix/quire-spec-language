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

QSL SHALL settle every temporal item over a model subject that EN-1 decides
as exactly one FR-331 terminal record whose value states the verdict's
strength (ADR-018 V-1, V-4 to V-7). `qsl-replay` owns the terminal record,
its category map and the `ProofBasis` and `InconclusiveCause` additions
(ADR-013 O-24, as amended by ADR-018), whose category map covers every
value ADR-018 V-1 to V-8 names. `model_check` owns the one map from
`ModelCheckOutcome` (FR-126) and the replay result of its counterexample
(FR-128) to `TerminalValue`.

## Use case

A verification operator reads a run's results. A `proved` item says how it
was proved: exhaustive exploration, a bounded-complete unrolling, or
induction. A depth-limited search that found nothing says `inconclusive`
with its depth, never `proved`. A run that hit a limit says which limit, its
value and how to raise it. A counterexample counts as `refuted` only after
it replays.

## Semantic authority and boundary

QSpec owns the verdict table onto QSpec FR-341 (infinite-trace) and FR-243,
the method and depth members of the FR-331 terminal record, and the new
inconclusive causes on the wire (ADR-018 QS-6; References). This
requirement specifies QSL's types and its map from EN-1's outcomes. The SMT
backend's outcomes (V-2, V-3, `InductionNotClosed`) reach `TerminalValue`
through CG's map (ADR-018 DS-2), and a V-8 settlement through negotiation;
QSL supplies their types and categories.

## Inputs

- A `ModelCheckOutcome` (FR-126).
- For `Violated`: the FR-072 result of replaying its counterexample
  (FR-128), or the `ReplayRefusal` that stopped it.

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

### The map from EN-1's outcomes

- `model_check` SHALL map each input to exactly one row of this table:

| Verdict | Input | QSpec FR-341 label | QSpec FR-243 basis | `TerminalValue` | O-16 category |
| --- | --- | --- | --- | --- | --- |
| V-1 | `Holds{Exhaustive}` | `proved` | `closed-scope` | `Proved{basis: Exhaustive}` | success |
| V-4 | `Violated` whose replay settles `reproduced-with-evaluated-witness` | `refuted` | `decisive-counterexample` | `Refuted` | violation |
| V-4 | `Violated` with `kind: UndefinedEvaluation{where, cause}` whose replay reproduces the undefined value at `where` (FR-128) | `refuted`, cause `UndefinedEvaluation{where, cause}` | `decisive-counterexample` | `Refuted` | violation |
| V-5 | `BoundReached{depth}` | `inconclusive` | `unsettled` | `Inconclusive(BoundReached{depth})` | inconclusive |
| V-6 | `Undecided(UndecidedSuccessor)` or `Undecided(NoInitialState)` | `inconclusive` | `unsettled` | `Inconclusive(cause)` | inconclusive |
| V-6 | `Violated` whose replay settles `inconclusive` (`Verdicts` or `NoValue`) | `inconclusive` | `unsettled` | `Inconclusive(ReplayParity)` | inconclusive |
| V-6 | `Violated` whose replay refuses with any refusal other than an internal fault | `inconclusive` | `unsettled` | `Inconclusive(ReplayRefused)` | inconclusive |
| — | `Violated` whose replay refuses with `InternalFault` | `failed` | `unavailable` | `Failed` | failed (ADR-013 O-16 internal failure) |
| V-7 | `Stopped{cause, limit}` | `failed`, execution `resource-incomplete` | `unavailable` | `Incomplete(cause)` | incomplete |

- When an outcome is `Violated`, `model_check` SHALL settle it `refuted`
  only through its replay.
- When a claim evaluates undefined on an admitted behaviour, `model_check`
  SHALL settle the item `refuted` with cause `UndefinedEvaluation{where,
  cause}`, carried as its counterexample's `kind` (ADR-018 UE-1, UE-2); it
  SHALL add no label, basis, `TerminalValue` variant or category for it.
- `model_check` SHALL write in a `Stopped` record the limit, its value and
  the request member that raises it (FR-126 `ReachedLimit`),
  `max_automaton_states` included.
- `model_check` SHALL give a run that completes every depth up to `k`
  execution `completed` and truth `pending` (V-5), and a run stopped before
  completing `k` V-7 (ADR-014 B-5 as amended by ADR-018).
- `model_check` SHALL write in the record the method `explicit-state`, and
  for V-5 the depth.
- `model_check` SHALL settle a `DeadlockFreedom` item through the same map;
  its V-4 record carries the counterexample whose `kind` is `Deadlock`.
- `model_check` SHALL bind the subject in the record's obligation identity
  (ADR-013 O-09), so a verdict holds for exactly its subject.

### Categories of every proof value

- `TerminalValue::category` SHALL map `Proved{basis: Checks{success_checks:
  0}}` to inconclusive with `KaniVacuousProof`, and every other `Proved`
  basis, `Exhaustive`, `BoundedComplete{depth}` and `Inductive{depth}`, to
  success.
- `TerminalValue::category` SHALL map `Inconclusive` with each of the four
  new causes to inconclusive, and `Unsupported(cause)` to unsupported.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-127-AC-1 | Each row of the map settles its `TerminalValue`, FR-341 label, FR-243 basis and O-16 category exactly as the table states: `Holds{Exhaustive}`; a reproduced `Violated`; `BoundReached{depth: 2}` carrying depth 2 and method `explicit-state`; `Undecided(UndecidedSuccessor)`; `Undecided(NoInitialState)`; `Stopped{ResourceExhausted, {MaxStates, 2}}` naming `max_states`, value 2. | Test (TC-522) |
| FR-127-AC-2 | `TerminalValue::category` maps `Proved{Checks{0}}` to inconclusive `KaniVacuousProof`; `Proved{Checks{1}}`, `Proved{Exhaustive}`, `Proved{BoundedComplete{depth: 5}}` and `Proved{Inductive{depth: 2}}` to success; `Inconclusive` with `BoundReached{1}`, `InductionNotClosed{2}`, `UndecidedSuccessor` and `NoInitialState` to inconclusive. | Test (TC-522) |
| FR-127-AC-3 | FR-126-AC-1's weak `each` outcome settles `proved`, `closed-scope`, `Proved{Exhaustive}`, success. Its counterexample under the constraint with no granularity settles `refuted` only after FR-128 replay reproduces it. The same counterexample with one post-state digest altered settles `inconclusive`, `ReplayRefused`; replayed in an envelope for the weak `each` clause, whose fairness it fails, it settles `inconclusive`, `ReplayRefused`; with its last step removed, so the loop does not close, it settles `inconclusive`, `ReplayRefused`; one whose formula evaluates `true` on replay settles `inconclusive`, `ReplayParity`. A replay that returns `InternalFault` settles `failed`, category failed. | Test (TC-522) |
| FR-127-AC-4 | FR-126-AC-6's `max_automaton_states` run settles `failed`, `resource-incomplete`, `unavailable`, `Incomplete(ResourceExhausted)`, category incomplete, naming `max_automaton_states` with value 50; FR-126-AC-5's `max_depth` 2 run settles `inconclusive`, `BoundReached{depth: 2}`, execution `completed`, truth `pending`; its evaluation-meter run names `EvaluationMeter` with value 0. | Test (TC-522) |
| FR-127-AC-5 | FR-126-AC-3's deadlock-freedom violation settles `refuted` after replay, with a record whose counterexample `kind` is `Deadlock`, and the record's obligation identity differs from the authored claims' over the same subject. | Test (TC-522) |
| FR-127-AC-6 | FR-126-AC-9's undefined-evaluation counterexample settles `refuted`, `decisive-counterexample`, `Refuted`, category violation, after FR-128 replay reproduces it, and the record's counterexample carries `kind: UndefinedEvaluation{where: 2, cause: division-by-zero}`. The same payload with its cause changed to another undefined reason settles `inconclusive`, `ReplayParity`. | Test (TC-538) |

## Dependencies

- ADR-018 §1 (V-1 to V-8, `ProofBasis`, the new causes, length, depth,
  UE-1 and UE-2);
  ADR-013 O-16 and O-24 as amended by ADR-018; ADR-014 B-5 as amended.
- [FR-069](FR-069-implement-typed-proof-result-envelope.md) (terminal record
  and category map), [FR-072](FR-072-implement-typed-replay-result.md)
  (replay result), [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md),
  [FR-128](FR-128-replay-a-model-counterexample.md).

## References

- QSpec half of ADR-018, carrying ADR-018 QS-6: Linear STD-131.
