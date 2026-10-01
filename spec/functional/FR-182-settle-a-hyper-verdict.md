---
id: FR-182
title: "Settle a hyper or step-relation verdict and its causes"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-021
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-023
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-176
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-177
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-178
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-179
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-183
    type: depends_on
---
# FR-182: Settle a hyper or step-relation verdict and its causes

## Description

QSL SHALL settle every HP-1, HP-2, HP-3 and HP-6 item, and every HP-4 item,
as exactly one FR-331 terminal record through FR-127's map, with ADR-018's
verdict kinds V-1 to V-8 and no new label, basis, category or `ProofBasis`
member (ADR-023 HV-1 to HV-6). `InconclusiveCause` gains `MatchUndetermined`
and `VacuousMatch`. HP-5 items settle by the possible family's map (FR-181).

## Use case

A verification operator reads the result of a noninterference check. A
proof says it covered every matched pair exhaustively, or under copy-swap.
A refutation counts only after its tuple of traces replays. A clause whose
match no pair of runs satisfies reads `inconclusive`, `VacuousMatch`, never
`proved`, and a run that hit `max_witness_set` names that limit.

## Inputs

- A `ModelCheckOutcome` from FR-176, FR-177, FR-178 or FR-179, or an SMT
  outcome for HP-2 mapped by CG (ADR-023 HC-6, HC-7) into the same values.
- For `Violated`: the FR-072 replay result of its `HyperCounterexample`
  (FR-183), or the `ReplayRefusal` that stopped it.
- For an item negotiation did not route: its negotiation disposition.

## Outputs

```rust
// InconclusiveCause gains:
//   MatchUndetermined,   // μ undefined, refused or incomplete at a reachable joint step
//   VacuousMatch,        // non-empty μ_U that no fair universal tuple satisfies
```

and the FR-331 terminal record with its FR-341 (infinite-trace) label,
FR-243 basis, O-16 category and method.

## Behavior

- The map SHALL be FR-127's, exhaustive with no `_` arm, with these rows
  for hyper items:

| Verdict | Input | QSpec FR-341 label | QSpec FR-243 basis | `TerminalValue` | O-16 category |
| --- | --- | --- | --- | --- | --- |
| V-1 | `Holds{Exhaustive}` or `Holds{Reduced{…}}` (copy-swap named, FR-174) | `proved` | `closed-scope` | `Proved{basis: Exhaustive}` or `Proved{basis: Reduced{…}}` | success |
| V-3 | SMT `k`-inductive for HP-2 with a safety body | `proved` | `decisive-witness` | `Proved{basis: Inductive{depth: k}}` | success |
| V-4 | `Violated` whose replay settles `reproduced-with-evaluated-witness` | `refuted` | `decisive-counterexample` | `Refuted` | violation |
| V-5 | `BoundReached{depth}` | `inconclusive` | `unsettled` | `Inconclusive(BoundReached{depth})` | inconclusive |
| V-6 | `Undecided(MatchUndetermined)`, `Undecided(VacuousMatch)`, `UndecidedSuccessor`, `NoInitialState`, `ReductionNotPreserving`; SMT `InductionNotClosed{depth}`; `Violated` whose replay settles `inconclusive` (`ReplayParity`) or refuses (`ReplayRefused`) | `inconclusive` | `unsettled` | `Inconclusive(cause)` | inconclusive |
| V-7 | `Stopped(cause, limit)`, including `MaxWitnessSet` and `MaxRelationTuples` (FR-184) | `failed`, execution `resource-incomplete` | `unavailable` | `Incomplete(cause)` | incomplete |
| V-8 | HP-4; a `behaviours` clause under a profile other than infinite-trace | `unsupported` | `unavailable` | `Unsupported(unsupported-requested-capability)` | unsupported |

- A `Violated` outcome SHALL settle `refuted` only through its replay.
- An HP-3 refutation SHALL carry basis `decisive-counterexample`: its
  evidence is the universal tuple, and replay recomputes the existential
  half over it (ADR-023 HX-4).
- A V-5 record SHALL state what the bound covers: for HP-2 and HP-3, no
  counterexample whose universal lasso has at most `k` joint steps; for
  HP-6, no counterexample of at most `k` moves; for HP-1, no counterexample
  whose executions all leave states at depth below `k`.
- A V-7 record SHALL name its limit, its value and the request member that
  raises it.
- The record SHALL carry the method (`explicit-state`, `smt-unrolling` or
  `k-induction`).
- A verdict SHALL hold for exactly its subjects; the obligation identity
  binds them as FR-172 states.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-182-AC-1 | Each input row maps to its `TerminalValue`, FR-341 label, FR-243 basis and O-16 category exactly as the table states, `MatchUndetermined` and `VacuousMatch` each to inconclusive. | Test (TC-607) |
| FR-182-AC-2 | ADR-023 §8.1's leaky refutation settles `refuted`, `decisive-counterexample`, only after FR-183 replay reproduces it; the secure proof settles `proved`, `closed-scope`, `Exhaustive`, and under copy-swap `Reduced` naming `CopySwap`. §8.2's leaky `Opaque` refutation settles `refuted`, `decisive-counterexample`. | Test (TC-607) |
| FR-182-AC-3 | FR-176-AC-2's vacuous match settles `inconclusive`, `VacuousMatch`; FR-177-AC-4's run settles `failed`, `resource-incomplete`, naming `max_witness_set` and its value; FR-179-AC-3's tuple-limited run names `max_relation_tuples`; FR-173-AC-2's HP-4 clauses settle `unsupported`, `unsupported-requested-capability`. | Test (TC-607) |
| FR-182-AC-4 | FR-179-AC-3's `max_depth` run settles `inconclusive`, `BoundReached{depth: 1}`, execution `completed`, truth `pending`, with a record stating the HP-1 reading of the bound. | Test (TC-607) |

## Dependencies

- ADR-023 §6 HV-1 to HV-7, §7 HX-4; ADR-018 §1 (V-1 to V-8); ADR-013 O-16
  and O-24 as amended by ADR-018.
- [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md)
  (the map), [FR-176](FR-176-check-a-universal-hyperproperty-by-self-composition.md),
  [FR-177](FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md),
  [FR-178](FR-178-check-a-projection-aligned-hyperproperty.md),
  [FR-179](FR-179-check-a-step-relation-over-reachable-transitions.md),
  [FR-183](FR-183-replay-a-hyper-counterexample.md).
- QSpec owns the verdict table onto FR-341 and FR-243 and the new
  inconclusive causes on the wire (ADR-023 QS-5).

## References

- ADR-023. QSpec half: Linear STD-136 (ADR-023 QS-5).
