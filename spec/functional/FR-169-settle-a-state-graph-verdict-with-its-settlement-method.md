---
id: FR-169
title: "Settle a state-graph verdict with its settlement method"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-020
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-022
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-069
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-072
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-168
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-170
    type: depends_on
---
# FR-169: Settle a state-graph verdict with its settlement method

## Description

QSL SHALL settle every state-graph item instance as exactly one FR-331
terminal record, through the same map FR-127 owns, extended by two verdict
kinds (ADR-022 GV-1 to GV-6): V-9 `Witnessed` and V-10 refuted by trap. The
record SHALL state how the item was settled (ADR-022 §4 "Settlement
method"): a sampled witness with its seed and trace index, an explored or
unrolled witness, exhaustive exploration, or exploration with evidence that
replays. The verdict does not depend on the method. `qsl-replay` owns
`ProofBasis::Witness` and its category; `model_check` owns the map.

## Use case

A verification operator reads a `proved` `possible` claim and sees that a
sampled walk with seed 7 and trace index 3 settled it, and that the walk
replayed. A second run with sampling off reads `proved` too, settled by
exploration. A refuted `always possible` claim names its trap, and a
depth-limited search that found neither a witness nor a trap reads
`inconclusive` with its depth.

## Inputs

- A `StateGraphOutcome` (FR-168), or an SMT `possible` path an engine
  returned through CG (ADR-022 GE-3), mapped into the same values.
- For `Witnessed`, `Trapped` and `PathPair`: the FR-072 replay result of its
  evidence (FR-170), or the `ReplayRefusal` that stopped it.
- The run's seed and limits (FR-166).

## Outputs

```rust
pub enum ProofBasis {
    // FR-127's members, plus:
    Witness { sources: Vec<WitnessSource> },   // one per initial state, in order
}

pub enum WitnessSource {
    Sampled(SampleProvenance),     // FR-101: seed, trace index, sampler
    Explored,
    Unrolled { depth: u64 },
}
```

and the FR-331 terminal record carrying the value, its QSpec FR-341 label,
its QSpec FR-243 basis, its O-16 category and its settlement method.

## Behavior

- The map SHALL be exhaustive, with no `_` arm, and give each input exactly
  one row:

| Verdict | Input | QSpec FR-341 label | QSpec FR-243 basis | `TerminalValue` | O-16 category |
| --- | --- | --- | --- | --- | --- |
| V-9 | `Witnessed` whose replay settles `reproduced-with-evaluated-witness` | `proved` | `decisive-witness` | `Proved{basis: Witness{sources}}` | success |
| V-10 | `Trapped` whose replay settles `reproduced-with-evaluated-witness` | `refuted` | `closed-scope` | `Refuted` | violation |
| V-4 | `PathPair` whose replay settles `reproduced-with-evaluated-witness` | `refuted` | `decisive-counterexample` | `Refuted` | violation |
| V-4 | `Undefined` whose replay reproduces the undefined value at `where` (FR-170) | `refuted`, cause `UndefinedEvaluation{where, cause}` | `decisive-counterexample` | `Refuted` | violation |
| V-1 | `Holds{basis}` | `proved` | `closed-scope` | `Proved{basis: Exhaustive}` or `Proved{basis: Reduced{…}}` | success |
| V-5 | `NoDecision` with `end` `Completed`, open causes only `MaxDepth`, and no partial-order reduction | `inconclusive` | `unsettled` | `Inconclusive(BoundReached{depth: max_depth})` | inconclusive |
| V-6 | `NoDecision` with `end` `Completed` and an `UndecidedSuccessor` open cause (`UndecidedSuccessor`), else a `ConstraintBoundary` one (`ConstraintReached`); `NoInitialState`; `ReductionNotPreserving` (FR-167); evidence whose replay settles `inconclusive` (`ReplayParity`) or refuses (`ReplayRefused`) | `inconclusive` | `unsettled` | `Inconclusive(cause)` | inconclusive |
| V-7 | `NoDecision` with `end` `Stopped(cause, limit)`; `NoDecision` with `end` `Completed` and open causes only `MaxDepth` under partial-order reduction (ADR-021 RV-5); a `Stopped` replay of a trap (FR-170); `Stopped(ResourceExhausted, WitnessSamples)` (FR-166) | `failed`, execution `resource-incomplete` | `unavailable` | `Incomplete(cause)` | incomplete |
| V-8 | No candidate discharges the form | `unsupported` | `unavailable` | `Unsupported(cause)` | unsupported |

- `TerminalValue::category` SHALL map `Proved{basis: Witness{…}}` to
  success.
- No item SHALL settle `proved` from a witness, or `refuted` from a trap,
  path pair or undefined evaluation, before its evidence replays (FR-170).
- An `Undefined` item SHALL settle `refuted` with cause
  `UndefinedEvaluation{where, cause}`, adding no label, basis or category
  (ADR-022 GV-7).
- **Settlement method.** The record SHALL carry the method
  `explicit-state` (or the SMT method for an unrolled witness) and:
  - for V-9, each initial state's `WitnessSource`; a `Sampled` source SHALL
    name its seed and trace index;
  - for V-1, `Exhaustive`, or `Reduced` with its reductions;
  - for V-10 and V-4, that exploration found the evidence it carries;
  - for V-5 to V-7, the limits the run used.
- The record SHALL carry the run's seed whenever phase 0 ran, whether the
  request named the seed or the default applied.
- A V-5 record SHALL have execution `completed` and truth `pending`, as
  FR-127 states.
- A V-7 record SHALL name its limit, its value and the request member that
  raises it.
- A verdict SHALL hold for exactly its subject; the obligation identity
  binds the subject, the claim and the instance (ADR-013 O-09).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-169-AC-1 | Each input row maps to its `TerminalValue`, FR-341 label, FR-243 basis and O-16 category exactly as the table states; `Proved{basis: Witness{sources}}` maps to success. | Test (TC-594) |
| FR-169-AC-2 | ADR-022 §7.1's `ReachesTwo` under default limits settles `proved`, `decisive-witness`, `Proved{basis: Witness{[Sampled(…)]}}`, success, and its record names the seed and trace index of each walk; with `witness_samples` 0 it settles the same label and basis with sources `[Explored]` and a record naming exploration. `ReachesThree` settles `refuted`, `closed-scope`, after its trap replays. | Test (TC-594) |
| FR-169-AC-3 | §7.2's `CanStillWin` settles `refuted`, `closed-scope` (V-10); its `from` variant `proved`, `closed-scope`, `Proved{Exhaustive}` (V-1). §7.3's `InOneWay` settles `refuted`, `decisive-counterexample` (V-4). | Test (TC-594) |
| FR-169-AC-4 | FR-168-AC-4's trap settles `refuted` (V-10) and its `NoDecision` settles `inconclusive`, `BoundReached{depth: 3}`, execution `completed`, truth `pending`. A run stopped by `max_states` settles `failed`, `resource-incomplete`, naming `max_states` and its value; an undecided node with no decisive evidence settles `inconclusive`, `UndecidedSuccessor`; a subject with no initial state settles `inconclusive`, `NoInitialState`. | Test (TC-594) |
| FR-169-AC-5 | A `Witnessed` outcome whose replay settles `inconclusive` settles `inconclusive`, `ReplayParity`, never `proved`; a `Trapped` outcome whose replay a limit stops settles `failed`, `resource-incomplete`. | Test (TC-594) |
| FR-169-AC-6 | FR-168-AC-5's `Undefined` outcome settles `refuted`, `decisive-counterexample`, category violation, after FR-170 replay reproduces it, with cause `UndefinedEvaluation` naming the node `(2, 0)` and `division-by-zero`. | Test (TC-612) |

## Dependencies

- ADR-022 §4 GV-1 to GV-6 and "Settlement method"; §10 RU-4; ADR-018 §1
  (V-1 to V-8); ADR-013 O-16 and O-24 as amended by ADR-018.
- [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md)
  (the map this extends), [FR-069](FR-069-implement-typed-proof-result-envelope.md),
  [FR-072](FR-072-implement-typed-replay-result.md),
  [FR-168](FR-168-decide-state-graph-claims-over-the-explored-graph.md),
  [FR-170](FR-170-replay-state-graph-evidence.md).
- QSpec owns the verdict table onto FR-341 and FR-243, the widening of
  FR-341's scope to state-graph results, and the settlement method and seed
  in the terminal record (ADR-022 QS-4, QS-7).

## References

- ADR-022. QSpec half: Linear STD-135 (ADR-022 QS-4, QS-7).
