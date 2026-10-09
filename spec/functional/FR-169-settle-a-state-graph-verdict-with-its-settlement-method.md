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
replays. The verdict does not depend on the method. A witness settles
`proved` only from a run that ruled out a reachable undefined evaluation;
witnesses from any other run settle `inconclusive`,
`WellDefinednessUnchecked` (ADR-022 GV-1, RU-5). A `proved` from an
unreduced exploration settles only after the core checker accepts its
state-graph certificate, and carries ADR-018 PC-1's `Certified` label; a
`proved` from a reduced exploration has no core checker and carries
`Uncertified` (ADR-022 GC-1 to GC-4). `qsl-replay` owns
`ProofBasis::Witness`, its category and this verdict map, beside the other
verdicts and settlement (ADR-029 CB-2).

## Use case

A verification operator reads a `proved` `possible` claim and sees that a
sampled walk with seed 7 and trace index 3 settled it, and that the walk
replayed. A second run with sampling off reads `proved` too, settled by
exploration. A refuted `always possible` claim names its trap, and a
depth-limited search that found neither a witness nor a trap reads
`inconclusive` with its depth. A run that a state budget stopped after
sampling found every witness reads `inconclusive`: witness found,
well-definedness unchecked.

## Inputs

- A `StateGraphOutcome` (FR-168), or an SMT `possible` path an engine
  returned through CG (ADR-022 GE-3), mapped into the same values.
- For `Witnessed`, `WitnessedUnchecked`, `Trapped` and `PathPair`: the FR-072 replay result of its
  evidence (FR-170), or the `ReplayRefusal` that stopped it.
- The run's seed and limits (FR-166).
- For `Witnessed` and `Holds` from an unreduced run: the
  `StateGraphCertificate` the engine returns with it (FR-168), and FR-098's
  replay request for the subject, which the checker recompiles.

## Outputs

```rust
pub enum ProofBasis {
    // FR-127's members, plus:
    Witness { sources: Vec<WitnessSource> },   // one per initial state, in order
}

pub enum WitnessSource {
    Sampled(SampleProvenance),     // FR-101: seed, trace index, sampler
    Explored,
    Unrolled { depth: u64 },       // only in WellDefinednessUnchecked
}

// InconclusiveCause gains:
//   WellDefinednessUnchecked { sources: Vec<WitnessSource> }
// TerminalValue::Proved carries FR-127's `certification` beside its basis.

pub struct StateGraphCertificate {
    pub closure: Vec<StateKey>,          // explored model states, sorted
    pub ranks: Vec<(u64, u32)>,          // `possible`, `always possible`: (state, d)
    pub counts: Vec<(u64, u8, u64)>,     // `unique path`: (state, count, order)
}

pub enum StateGraphCheck {
    Accepted,
    Rejected(CertificateRejection),          // FR-338's: rule and locus
    Stopped(ReachedLimit),                   // FR-126: the limit and its value
}

pub fn check_state_graph(request: &CertificateRequest<'_>, claim: &StateGraphItem,
    certificate: &StateGraphCertificate) -> StateGraphCheck;
```

The checker uses FR-338's `CertificateRejection{rule, at}`. FR-338's
`CertificateRule` gains `PredicateUndefined`, `RankMissing`, `RankBroken`,
`OrderBroken` and `CountBroken` (`InitialMissing` and `SuccessorMissing`
are FR-338's own), and its `CertificateLocus` gains
`ModelState(DigestRecord)`, a model state's
`quire.simulation.state-key/v1` digest, since a state-graph certificate
holds model states, not product states.

and the FR-331 terminal record carrying the value, its QSpec FR-360 label,
its QSpec FR-243 basis, its O-16 category and its settlement method.

## Behavior

- The map SHALL be exhaustive, with no `_` arm, and give each input exactly
  one row:

| Verdict | Input | QSpec FR-360 label | QSpec FR-243 basis | `TerminalValue` | O-16 category |
| --- | --- | --- | --- | --- | --- |
| V-9 | `Witnessed` whose replay settles `reproduced-with-evaluated-witness` and whose certificate the checker accepts | `proved` | `decisive-witness` | `Proved{basis: Witness{sources}, certification: Certified}` | success |
| V-10 | `Trapped` whose replay settles `reproduced-with-evaluated-witness` | `refuted` | `closed-scope` | `Refuted` | violation |
| V-4 | `PathPair` whose replay settles `reproduced-with-evaluated-witness` | `refuted` | `decisive-counterexample` | `Refuted` | violation |
| V-4 | `Undefined` whose replay reproduces the undefined value at `where` (FR-170) | `refuted`, cause `UndefinedEvaluation{where, cause}` | `decisive-counterexample` | `Refuted` | violation |
| V-1 | `Holds{Exhaustive}` whose certificate the checker accepts; `Holds{Reduced{…}}` | `proved` | `closed-scope` | `Proved{basis: Exhaustive, certification: Certified}`, or `Proved{basis: Reduced{…}, certification: Uncertified}` | success |
| V-6 | `Witnessed` or `Holds{Exhaustive}` whose certificate the checker rejects | `inconclusive` | `unsettled` | `Inconclusive(CertificateRejected{rule, at})` (ADR-018 PC-2) | inconclusive |
| V-5 | `NoDecision` with `end` `Completed`, open causes only `MaxDepth`, and no partial-order reduction | `inconclusive` | `unsettled` | `Inconclusive(BoundReached{depth: max_depth})` | inconclusive |
| V-6 | `WitnessedUnchecked` whose witnesses each replay to `reproduced-with-evaluated-witness` | `inconclusive` | `unsettled` | `Inconclusive(WellDefinednessUnchecked{sources})` | inconclusive |
| V-6 | `NoDecision` with `end` `Completed` and an `UndecidedSuccessor` open cause (`UndecidedSuccessor`), else a `ConstraintBoundary` one (`ConstraintReached`); `NoInitialState`; `ReductionNotPreserving` (FR-167); evidence whose replay settles `inconclusive` (`ReplayParity`, wire cause `replay-parity`) or refuses (`ReplayRefused`), a trap replay a limit stopped included (FR-170) | `inconclusive` | `unsettled` | `Inconclusive(cause)` | inconclusive |
| V-7 | `NoDecision` with `end` `Stopped(cause, limit)`; `NoDecision` with `end` `Completed` and open causes only `MaxDepth` under partial-order reduction (ADR-021 RV-5); `Stopped(ResourceExhausted, WitnessSamples)` (FR-166); a certificate check a limit stopped; a trap replay a limit stopped (FR-170) | `failed`, execution `resource-incomplete` | `unavailable` | `Incomplete(LimitReached{limit, value, setting})` (ADR-018 V-7, FR-127) | incomplete |
| V-8 | No candidate discharges the form | `unsupported` | `unavailable` | `Unsupported(cause)` | unsupported |

- `TerminalValue::category` SHALL map `Proved{basis: Witness{…}, certification}` to
  success, with either certification.

### State-graph certificate (ADR-022 GC-1 to GC-4)

- **Production.** With `Witnessed` or `Holds` from an unreduced
  exploration, the engine SHALL return a `StateGraphCertificate`: every
  explored model state's `quire.simulation.state-key/v1` digest, sorted;
  for `possible` and `always possible`, each state's distance `d` to the
  nearest target node, for every state from which a target is reachable;
  for `unique path`, each node of `H`'s path count and its position in a
  reverse topological order of `H`.
- **Check.** `check_state_graph`, a layer-6 entry of `qsl-replay` beside
  FR-338's `check_closure`, SHALL recompile the package, re-admit the
  subject and recompute with its own code, reading nothing from the engine
  but the certificate. It SHALL return `Rejected` with FR-338's
  `CertificateRejection{rule, at}`, `at` the `ModelState` locus of
  the first failing state in certificate order, which settles
  `CertificateRejected{rule, at}`:
  - `InitialMissing`, when an initial state is not in `closure`;
  - `SuccessorMissing`, when a successor that `ModelSystem` computes for a
    `closure` state is not in `closure`, as FR-338's `check_closure` does;
  - `PredicateUndefined`, when a claim predicate evaluates undefined at a
    `closure` state;
  - for `possible`: `RankMissing`, when an initial state has no rank;
    for `always possible`: `RankMissing`, when a state where `from` holds
    has no rank; and for both, `RankBroken`, when a ranked state with
    `d = 0` fails the target or one with `d > 0` has no successor ranked
    `d - 1`;
  - for `unique path`: `OrderBroken`, when an edge of `H` does not go to a
    node earlier in the order, and `CountBroken`, when a node's count is not
    1 for a `to` node, otherwise the sum of its `H` successors' counts
    saturated at 2, or when a node where `from` holds counts other than 1.
- An accepted certificate establishes the claim over every reachable
  state, well-definedness included: the closure holds every reachable
  state, no predicate is undefined there, and the ranks or counts give the
  claim's truth (ADR-022 GM-4, GM-5).
- The checker SHALL count states against `max_states` and successors
  against `max_transitions`, with checked arithmetic, and return
  `Stopped` naming the reached limit and its value, which settles V-7,
  `incomplete`, `LimitReached{limit, value, setting}`.
- A proof under a reduction SHALL carry `certification: Uncertified`, with
  no certificate (ADR-022 GC-4).
- No item SHALL settle `proved` from a witness unless FR-168 returned
  `Witnessed`, which it returns only from a run that completed with no open
  node and no undefined evaluation of the target.
- No item SHALL settle `proved` from a witness, or `refuted` from a trap,
  path pair or undefined evaluation, before its evidence replays (FR-170).
- An `Undefined` item SHALL settle `refuted` with cause
  `UndefinedEvaluation{where, cause}`, adding no label, basis or category
  (ADR-022 GV-7).
- **Settlement method.** The record SHALL carry the method
  `explicit-state` (or the SMT method for an unrolled witness) and:
  - for V-9, each initial state's `WitnessSource`; a `Sampled` source SHALL
    name its seed and trace index;
  - for V-6 `WellDefinednessUnchecked`, each witness's source, the run's
    end and the limits the run used;
  - for V-1, `Exhaustive`, or `Reduced` with its reductions;
  - for V-10 and V-4, that exploration found the evidence it carries;
  - for V-5 to V-7, the limits the run used.
- Every record SHALL state the search horizon `max_depth` the run used as a
  method parameter beside the limits, never as a limit (QSpec FR-392).
- Every record SHALL carry the run's seed and `witness_samples`, whether
  the request named them or the defaults applied, and the count of walks
  `max_walk_steps` stopped (FR-166), as QSpec FR-392 records them.
- `Witness{sources}` SHALL NOT carry an `Unrolled` source; an EN-2 witness
  settles only `WellDefinednessUnchecked` (ADR-022 GE-3).
- A V-5 record SHALL have execution `completed` and truth `pending`, as
  FR-127 states.
- A V-7 record SHALL name its limit, its value and the request member that
  raises it.
- A verdict SHALL hold for exactly its subject; the obligation identity
  binds the subject, the claim and the instance (ADR-013 O-09).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-169-AC-1 | Each input row maps to its `TerminalValue`, FR-360 label, FR-243 basis and O-16 category exactly as the table states; `Proved{basis: Witness{sources}, certification: Certified}` maps to success. | Test (TC-594) |
| FR-169-AC-2 | ADR-022 §7.1's `ReachesTwo` under default limits settles `proved`, `decisive-witness`, `Proved{basis: Witness{[Sampled(…)]}, certification: Certified}`, success, and its record names the seed and trace index of each walk; with `witness_samples` 0 it settles the same label and basis with sources `[Explored]` and a record naming exploration. `ReachesThree` settles `refuted`, `closed-scope`, after its trap replays. | Test (TC-594) |
| FR-169-AC-3 | §7.2's `CanStillWin` settles `refuted`, `closed-scope` (V-10); its `from` variant `proved`, `closed-scope`, `Proved{Exhaustive, Certified}` (V-1). §7.3's `InOneWay` settles `refuted`, `decisive-counterexample` (V-4). | Test (TC-594) |
| FR-169-AC-4 | FR-168-AC-4's trap settles `refuted` (V-10) and its `NoDecision` settles `inconclusive`, `BoundReached{depth: 3}`, execution `completed`, truth `pending`, and its record states horizon 3 as a method parameter, not among the limits. A run stopped by `max_states` settles `incomplete`, `LimitReached`, naming `max_states` and its value; an undecided node with no decisive evidence settles `inconclusive`, `UndecidedSuccessor`; a subject with no initial state settles `inconclusive`, `NoInitialState`. | Test (TC-594) |
| FR-169-AC-5 | A `Witnessed` outcome whose replay settles `inconclusive` settles `inconclusive`, `ReplayParity`, never `proved`; a `Trapped` outcome whose replay a limit stops settles `incomplete`, `LimitReached`, naming the limit. | Test (TC-594) |
| FR-169-AC-6 | FR-168-AC-5's `Undefined` outcome settles `refuted`, `decisive-counterexample`, category violation, after FR-170 replay reproduces it, with cause `UndefinedEvaluation` naming the node `(2, 0)` and `division-by-zero`. | Test (TC-612) |
| FR-169-AC-7 | FR-168-AC-7's `Undefined` outcome, found although a sampled witness existed, settles `refuted`, `decisive-counterexample`, cause `UndefinedEvaluation`; `ReachesTwo` with default limits settles `proved`, `decisive-witness`, with `Sampled` sources, and its record names a completed exploration. | Test (TC-613) |
| FR-169-AC-8 | FR-168-AC-8's `WitnessedUnchecked` outcome settles `inconclusive`, `unsettled`, `Inconclusive(WellDefinednessUnchecked{sources})` with `Sampled` sources, naming `max_states` and its value, never `proved`; its `NoDecision` variant with `witness_samples` 0 settles `incomplete`, `LimitReached`. | Test (TC-614) |
| FR-169-AC-9 | Accepted certificates. `ReachesTwo` for `c = a` returns a certificate whose closure is §7.1's 9 states and whose ranks are `d = 0` at `va = 2`, 1 at `va = 1` and 2 at `va = 0`; `check_state_graph` accepts it and the item settles `Proved{Witness{…}, Certified}`. §7.2's `CanStillWin` `from (x.phase != Lost)` returns ranks `Won` 0, `Mid` 1, `Start` 2 and no rank for `Lost`, accepted, `Proved{Exhaustive, Certified}`. §7.3's sequenced `InOneWay` returns counts 1 at every node of `H` with a reverse topological order, accepted. The `possible` item of FR-167-AC-4 proved under partial-order reduction settles `Proved{Reduced{…}, Uncertified}` with no certificate. | Test (TC-643) |
| FR-169-AC-10 | Rejected certificates, each settling `inconclusive`, `CertificateRejected{rule, at}` and never `proved`: AC-9's `ReachesTwo` certificate with `(2, 2)` removed (`SuccessorMissing` at the state that reaches it); with the rank of `(1, 0)` changed to 2 (`RankBroken` at `(1, 0)`); with `(0, 0)` unranked (`RankMissing`); a hand-built certificate for FR-168-AC-5's item (`PredicateUndefined` at `(2, 0)`); the `InOneWay` certificate over §7.3's unsequenced job with every count 1 (`CountBroken` at the node with two paths); the sequenced certificate with its order reversed (`OrderBroken`). The check with `max_states` 2 stops and settles `failed`, naming `max_states`. | Test (TC-644) |

## Dependencies

- ADR-022 §4 GV-1 to GV-6 and "Settlement method"; §10 RU-4 and RU-5; ADR-018 §1
  (V-1 to V-8); ADR-013 O-16 and O-24 as amended by ADR-018.
- [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md)
  (the map this extends), [FR-069](FR-069-implement-typed-proof-result-envelope.md),
  [FR-072](FR-072-implement-typed-replay-result.md),
  [FR-168](FR-168-decide-state-graph-claims-over-the-explored-graph.md),
  [FR-170](FR-170-replay-state-graph-evidence.md).
- QSpec FR-391 owns the verdict table onto FR-360 and FR-243 and the
  widening of FR-360's scope to state-graph results; QSpec FR-392 owns the
  settlement method and seed in the terminal record (ADR-022 QS-4, QS-7).

## References

- ADR-022. QSpec half: QSpec FR-391 and FR-392 (Linear STD-135; ADR-022
  QS-4, QS-7).
