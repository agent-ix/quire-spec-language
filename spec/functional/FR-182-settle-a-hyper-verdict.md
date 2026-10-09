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
member (ADR-023 HV-1 to HV-8). `InconclusiveCause` gains `MatchUndetermined`
and `VacuousMatch`. An item whose claim evaluates undefined on a tuple
settles `refuted` with cause `UndefinedEvaluation{where, cause}` naming the
tuple (ADR-018 UE-1, UE-2). HP-5 items settle by the possible family's map
(FR-181). Settlement is `qsl-replay`'s (ADR-029 CB-2): an EN-1 `proved`
settles only after the product-closure certificate checker (FR-163)
accepts its certificate, and a proof whose method has no certificate
checker settles `proved` labelled `Uncertified` (ADR-023 HV-1, HX-7).

## Use case

A verification operator reads the result of a noninterference check. A
proof says it covered every matched pair exhaustively, or under copy-swap.
A refutation counts only after its tuple of traces replays. A clause whose
match no pair of runs satisfies reads `inconclusive`, `VacuousMatch`, never
`proved`, and a run that hit `max_witness_set` names that limit.

## Inputs

- A `ModelCheckOutcome` from FR-176, FR-177, FR-178 or FR-179, or an SMT
  outcome for HP-2 mapped by CG (ADR-023 HC-6, HC-7) into the same values.
- For `Holds` from EN-1: the `ProductClosureCheck` of its certificate
  (FR-163).
- For `Violated`: the FR-072 replay result of its `HyperCounterexample`
  (FR-183), or the `ReplayRefusal` that stopped it.
- For an item negotiation did not route: its negotiation disposition.

## Outputs

```rust
// InconclusiveCause gains:
//   MatchUndetermined,   // μ refused or incomplete at a reachable joint step
//   VacuousMatch,        // non-empty μ_U that no fair universal tuple satisfies
```

and the FR-331 terminal record with its QSpec FR-360 label,
FR-243 basis, O-16 category and method.

## Behavior

- The map SHALL be FR-127's, exhaustive with no `_` arm, with these rows
  for hyper items:

| Verdict | Input | QSpec FR-360 label | QSpec FR-243 basis | `TerminalValue` | O-16 category |
| --- | --- | --- | --- | --- | --- |
| V-1 | `Holds{Exhaustive}` or `Holds{Reduced{…}}` (copy-swap named, FR-174), proof basis `exhaustive` or `reduced` (QSpec FR-399), whose certificate check is `Accepted` | `proved` | `closed-scope` | `Proved{basis: Exhaustive, certification: Certified}` or `Proved{basis: Reduced{…}, certification: Certified}` | success |
| V-3 | SMT `k`-inductive for HP-2 with a safety body | `proved` | `decisive-witness` | `Proved{basis: Inductive{depth: k}, certification: Uncertified}` | success |
| V-4 | `Violated` whose replay settles `reproduced-with-evaluated-witness` | `refuted` | `decisive-counterexample` | `Refuted` | violation |
| V-4 | `Violated` with an `Undefined` counterexample (QSpec FR-400) whose replay reproduces the undefined value at `trace_position` (FR-183) | `refuted`, cause `UndefinedEvaluation{where, cause}` | `decisive-counterexample` | `Refuted` | violation |
| V-5 | `BoundReached{depth}` | `inconclusive` | `unsettled` | `Inconclusive(BoundReached{depth})` | inconclusive |
| V-6 | `Undecided(MatchUndetermined)`, `Undecided(VacuousMatch)`, `UndecidedSuccessor`, `NoInitialState`, `ReductionNotPreserving` (wire cause `reduction-not-preserving`); `Holds` whose certificate check is `Rejected` (`CertificateRejected{rule, at}`, ADR-018 PC-2); SMT `InductionNotClosed{depth}`; `Violated` whose replay settles `inconclusive` (`ReplayParity`, wire cause `replay-parity`) or refuses (`ReplayRefused`) | `inconclusive` | `unsettled` | `Inconclusive(cause)` | inconclusive |
| V-7 | `Stopped(ResourceExhausted, limit)`, including `MaxWitnessSet` and `MaxRelationTuples` (FR-184) | `failed`, execution `resource-incomplete` | `unavailable` | `Incomplete(LimitReached{limit, value, setting})` (ADR-018 V-7, FR-127) | incomplete |
| V-7 | `Stopped(Cancelled, None)` | `failed`, execution `resource-incomplete` | `unavailable` | `Incomplete(Cancelled{source})`, written `cancelled{source}` (FR-127) | incomplete |
| V-8 | HP-4; a `behaviours` clause under a profile other than infinite-trace | `unsupported` | `unavailable` | `Unsupported(unsupported-requested-capability)` | unsupported |

- A `Violated` outcome SHALL settle `refuted` only through its replay.
- An item whose body or `μ` evaluates undefined on a tuple SHALL settle
  `refuted`, through its replay, with cause `UndefinedEvaluation{where,
  cause}`, `where` naming the tuple (ADR-023 HV-8). The item SHALL settle on
  the first refuting evidence in canonical order, a false tuple or an
  undefined one.
- An HP-3 refutation SHALL carry basis `decisive-counterexample`: its
  evidence is the universal tuple, and replay recomputes the existential
  half over it (ADR-023 HX-4).
- A V-5 record SHALL state what the bound covers: for HP-2 and HP-3, no
  counterexample whose universal lasso has at most `k` joint steps; for
  HP-6, no counterexample of at most `k` moves; for HP-1, no counterexample
  whose executions all leave states at depth below `k`.
- A V-7 record SHALL name its limit, its value and the request member that
  raises it.
- Every record, whatever its verdict, SHALL state the value of each
  model-check limit the run used, FR-126's and FR-184's, with whether the
  request set it or the published default applied.
- The record SHALL carry the method (`explicit-state`, `smt-unrolling` or
  `k-induction`).
- A `proved` record SHALL carry ADR-018 PC-1's certification label: `Certified` when the
  product-closure certificate checker accepted its certificate, and
  `Uncertified` for a proof whose method has no certificate checker, which
  is V-3 from `k-induction`. An `Uncertified` proof SHALL keep the label
  `proved`.
- A certificate check that a limit stopped SHALL settle V-7 naming the
  limit.
- A verdict SHALL hold for exactly its subjects; the obligation identity
  binds them as FR-172 states.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-182-AC-1 | Each input row maps to its `TerminalValue`, FR-360 label, FR-243 basis and O-16 category exactly as the table states, `MatchUndetermined` and `VacuousMatch` each to inconclusive. | Test (TC-607) |
| FR-182-AC-2 | ADR-023 §8.1's leaky refutation settles `refuted`, `decisive-counterexample`, only after FR-183 replay reproduces it; the secure proof settles `proved`, `closed-scope`, `Exhaustive`, `Certified`, and under copy-swap `Reduced` naming `CopySwap`, `Certified`. §8.2's leaky `Opaque` refutation settles `refuted`, `decisive-counterexample`. | Test (TC-607) |
| FR-182-AC-3 | FR-176-AC-2's vacuous match settles `inconclusive`, `VacuousMatch`; FR-177-AC-4's run settles `incomplete`, `LimitReached`, naming `max_witness_set` and its value; FR-179-AC-3's tuple-limited run names `max_relation_tuples`; FR-173-AC-2's HP-4 clauses settle `unsupported`, `unsupported-requested-capability`. | Test (TC-607) |
| FR-182-AC-4 | FR-179-AC-3's `max_depth` run settles `inconclusive`, `BoundReached{depth: 1}`, execution `completed`, truth `pending`, with a record stating the HP-1 reading of the bound. | Test (TC-607) |
| FR-182-AC-5 | FR-179-AC-4's outcome settles `refuted`, `decisive-counterexample`, category violation, after FR-183 replay reproduces the undefined value at its tuple, with a record carrying `UndefinedEvaluation` naming that tuple, its two executions and the division-by-zero cause; it never settles `proved`. | Test (TC-610) |
| FR-182-AC-6 | FR-179-AC-5's `Violated` outcome settles `refuted`, `decisive-counterexample`, after its `StepTuple` replays, with no `UndefinedEvaluation`: the false tuple is the first refuting evidence, ahead of the undefined tuples. | Test (TC-611) |
| FR-182-AC-7 | §8.1's secure proof settles `proved`, certification `Certified`, after FR-163-AC-1's certificate is accepted; with FR-163-AC-2's member-removed certificate the item settles `inconclusive`, `CertificateRejected{Closure, …}`, never `proved`. An HP-2 `Inductive{depth: 1}` outcome from `k-induction` settles `proved`, `decisive-witness`, certification `Uncertified`. | Test (TC-607) |
| FR-182-AC-8 | FR-177-AC-5's outcome settles `refuted`, `decisive-counterexample`, cause `UndefinedEvaluation` naming `a` and `b`, after its replay; FR-177-AC-6's and FR-178-AC-6's outcomes settle `inconclusive`, `MatchUndetermined`. | Test (TC-607) |

## Dependencies

- ADR-023 §6 HV-1 to HV-8, §7 HX-4 and HX-7; ADR-029 CB-2 and RU-2; ADR-018 §1 (V-1 to V-8); ADR-013 O-16
  and O-24 as amended by ADR-018.
- [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md)
  (the map), [FR-176](FR-176-check-a-universal-hyperproperty-by-self-composition.md),
  [FR-177](FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md),
  [FR-178](FR-178-check-a-projection-aligned-hyperproperty.md),
  [FR-179](FR-179-check-a-step-relation-over-reachable-transitions.md),
  [FR-183](FR-183-replay-a-hyper-counterexample.md).
- QSpec owns the verdict table onto FR-360 and FR-243 and the new
  inconclusive causes on the wire (ADR-023 QS-5).

## References

- ADR-023. QSpec half: QSpec FR-399 (Linear STD-136; ADR-023 QS-5).
