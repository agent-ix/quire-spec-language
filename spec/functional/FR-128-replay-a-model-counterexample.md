---
id: FR-128
title: "Replay a model counterexample through ModelSystem"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-015
    type: implements
  - target: ix://agent-ix/quire-spec-language/US-010
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-072
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-124
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: depends_on
---
# FR-128: Replay a model counterexample through ModelSystem

## Description

QSL SHALL replay a model counterexample (FR-126) through the layer-6 replay
facade (FR-098) at ADR-011 E9: `qsl_replay::replay_model_trace` takes
FR-098's request with the subject's inputs and a
`WitnessEnvelope<TemporalCounterexample>` whose source is
`ReplaySource::ModelTrace`. It recompiles the package, re-admits the subject,
re-executes each step through FR-120's `ModelSystem` with FR-101 `replay`,
checks the lasso against the fairness set, and evaluates the formula over
the replayed trace by FR-125, or for a deadlock counterexample checks the
final state (ADR-018 CX-3, DL-5). It settles an FR-072 replay result.
Replay needs no model checker and no solver.

## Use case

A verification operator receives a `refuted` liveness claim with a lasso.
They, or an auditor with only the source and the documents, replay it: the
replay rebuilds the model from source, re-runs each step, confirms the loop
closes and is fair, and evaluates the claim false on it. A lasso edited by
hand, or produced by an engine that disagrees with the evaluator, does not
count as a refutation.

## Semantic authority and boundary

QSpec FR-364 owns the counterexample wire and the replay rules for a model
counterexample, and QSpec owns the request's subject members (ADR-018 QS-8,
QS-9, QS-12; References). This requirement specifies QSL's replay entry,
its obligations, its refusal order and its results; it defines no wire
member of its own.

## Inputs

- FR-098's request: package reference, byte provision and limits. The byte
  provision holds the subject's initial-state snapshots by `sha256-jcs`
  digest; the request names the subject's universes.
- A `WitnessEnvelope<TemporalCounterexample>` (FR-070). Its payload is
  QSpec FR-364's counterexample, read by FR-364's member names: `kind`
  (formula, deadlock or undefined evaluation), `initial_state`, `prefix`,
  `loop` (whose steps are transition identities with post-state digests,
  or FR-364's `terminal-stutter` step), `over_binding`, `fairness` and
  `trace_position` (ADR-018 CX-2). The envelope names the clause, or the
  deadlock-freedom item, by its clause node and occurrence key (FR-070).
- `TemporalCounterexample` implements `FamilyPayload` (FR-070-AC-5).

## Outputs

- An FR-072 replay result on the `ModelTrace` arm, holding the evaluated
  value and the `trace_position`, or a typed `ReplayRefusal` with no
  partial result. Where this requirement settles a replay `inconclusive`,
  `ReplayParity`, the FR-072 result carries its `Verdicts` or `NoValue`
  cause and the item settles cause `ReplayParity`, written
  `replay-parity` (FR-127, QSpec FR-364). The arm's value is `ModelTraceValue::{Truth(bool),
  Undefined(UndefinedEvaluation)}`: `Truth` for a `Formula` or `Deadlock`
  counterexample, `Undefined` for an `UndefinedEvaluation` one. FR-072's
  no-value rule reads this arm so: an undefined letter reproduced at
  `where` with the payload's cause is a completed `Undefined` value; an
  evaluation that completes no value (refused, incomplete, or undefined at
  a position of a `Formula` counterexample) is FR-072's no-value case and
  settles `inconclusive`, `NoValue`.

## Behavior

- The executor SHALL recompile and check the package by FR-098's rules, in
  FR-098's order, and refuse by FR-098's stale `package_id` rule on a
  `package_id` mismatch.
- The executor SHALL resolve the clause by its `clause_node` and
  `occurrence_key`, and refuse `stale_dependency`/`content-mismatch`,
  naming both identities, when either differs from the recompile.
- The executor SHALL read the fairness set from the recompiled clause. A
  payload fairness set that differs from it SHALL refuse
  `stale_dependency`/`content-mismatch`, naming both sets.
- The executor SHALL build the subject's `ModelSystem` by FR-120 from the
  byte provision and the request's universes, and refuse with FR-106's
  record when admission fails. An `initial_state` outside the subject's
  initial states SHALL refuse `invalid_runtime_input`/`invalid-value`.
- The executor SHALL re-execute the prefix, then the loop,
  from the indexed initial state with FR-101 `replay`:
  - if a step's transition is not enabled at its pre-state, then the
    executor SHALL refuse `invalid_runtime_input`/`invalid-value`, naming
    the step;
  - the executor SHALL take as a step's post-state the successor of its
    transition identity whose `quire.simulation.state-key/v1` digest equals
    the step's recorded digest, since one transition identity can have
    several post-states (FR-120, FR-101-AC-5);
  - if no successor of a step's transition identity has the recorded
    digest, then the executor SHALL refuse
    `stale_dependency`/`content-mismatch` (FR-101's key mismatch), naming
    the step and the recorded digest;
  - if the loop does not end at its entry state, then the executor SHALL
    refuse `invalid_runtime_input`/`invalid-value`;
  - if the loop is the `terminal-stutter` step and its state is not
    terminal (FR-124), then the executor SHALL refuse
    `invalid_runtime_input`/`invalid-value`.
- For `kind: Formula` the executor SHALL check the lasso against the
  fairness set, reading enabledness from `ModelSystem` at every loop state,
  and refuse an unfair lasso with `invalid_runtime_input`/`invalid-value`
  (ADR-014 A-4). It SHALL then evaluate the formula over the replayed trace
  by FR-125 under the `over` binding.
  - `false` SHALL settle `reproduced-with-evaluated-witness`.
  - `true`, or no value, SHALL settle `inconclusive`, `ReplayParity` or
    `NoValue` (FR-072).
- For `kind: UndefinedEvaluation{where, cause}` the executor SHALL
  evaluate the letters of the replayed positions in order by FR-125, and
  check no fairness (ADR-018 UE-3, UE-5). For the deadlock-freedom item a
  letter at a terminal state holds the state model's `terminal when`
  predicate `P` (FR-124, FR-125), so an undefined `P` there reproduces as
  an undefined letter. At each replayed state the executor SHALL also
  recompute the contract conjunctions of the expansion, and an undefined
  one (an undefined precondition guard included, FR-120
  `ContractUndetermined` with an `Undefined` evaluation) reproduces as an
  undefined evaluation at that state (ADR-018 UE-6).
  - When the first position whose letter, or whose contract conjunction,
    is undefined is `where`, with an
    undefined cause equal to `cause`, the executor SHALL settle
    `reproduced-with-evaluated-witness`, with the `UndefinedEvaluation` as
    the arm's value: the reproduced undefined value is the reproduced
    witness, not FR-072's `NoValue` case.
  - A defined letter at `where`, an undefined letter before `where`, a
    `where` past the replayed positions, or a different cause SHALL settle
    `inconclusive`, `ReplayParity`.
- For `kind: Deadlock` the executor SHALL enumerate the transition
  identities enabled at the last replayed state through `ModelSystem`, and
  evaluate the state model's `terminal when` predicate there when it has
  one. No enabled identity and the predicate `false` or absent SHALL settle
  `reproduced-with-evaluated-witness`. The predicate undefined SHALL settle
  `reproduced-with-evaluated-witness` with value
  `Undefined(UndefinedEvaluation{where, cause})` at that state, so the item
  settles `refuted` with cause `UndefinedEvaluation` (ADR-018 UE-6, DL-5).
  An enabled identity, or the predicate `true`, SHALL settle
  `inconclusive`, `ReplayParity`.
- An internal fault SHALL refuse with an `InternalFault` and settle no
  result.
- Replay SHALL read no path, environment variable, clock or search
  location, and SHALL give the same result for the same request and
  envelope.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-128-AC-1 | FR-126-AC-1's lasso under the weak constraint with no granularity replays to `reproduced-with-evaluated-witness` with `trace_position` 0. FR-126-AC-2's bounded prefix and FR-126-AC-3's stutter lasso each reproduce. Over the `Branch` subject (one object `x`, field `v: Int[0, 2]`, initial 0, and operation `step` with no precondition and postcondition `self.v != pre(self.v)`, so `step` from 0 has the two post-states 1 and 2 under one transition identity), for the clause `always holds(x.v != 2)`, a prefix whose one step records the digest of `v = 2` replays through that successor and reproduces with `trace_position` 1, and one recording a digest that no successor of `step` has refuses `stale_dependency`/`content-mismatch` naming the step and that digest. | Test (TC-523) |
| FR-128-AC-2 | FR-126-AC-3's deadlock counterexample reproduces. The same payload, in an envelope carrying the `package_id` and the deadlock-freedom item's `clause_node` and `occurrence_key` of the `Counter` unit whose `When` member covers value 3, settles `inconclusive`, `ReplayParity`. The original payload truncated to end at value 2 settles `inconclusive`, `ReplayParity`, since `inc` is enabled there. | Test (TC-523) |
| FR-128-AC-3 | Refusals settle no result: AC-1's lasso with its last step removed (the loop does not close); the same lasso in an envelope for the clause with the weak `each` constraint, carrying that clause's identities and fairness set (unfair); AC-1's envelope with its payload fairness set changed to `each` (`stale_dependency`/`content-mismatch` naming both sets); one post-state digest altered (`stale_dependency`/`content-mismatch` naming the step and the recorded digest); a step `upd(a)` replaced by `attemptUpdate` with receiver `z`, outside the universe (not enabled); an `initial_state` of 1 over a one-snapshot subject; a `terminal-stutter` loop on a non-terminal state. | Test (TC-523) |
| FR-128-AC-4 | A lasso of `upd(a)` steps for `c = a` over ADR-018 §6's subject, which visits `a.versionNumber = 2`, settles `inconclusive`, `ReplayParity`. A source edit that changes the `package_id` refuses by FR-098's rule, and a `clause_node` naming another clause refuses `stale_dependency`/`content-mismatch`. Replaying one envelope twice gives equal results. | Test (TC-523) |
| FR-128-AC-5 | FR-126-AC-9's undefined-evaluation counterexample replays to `reproduced-with-evaluated-witness` with `trace_position` 2 and the `UndefinedEvaluation{where: position 2, cause: division-by-zero}` value. The payload with `where` set to 1, or truncated to end at value 1, settles `inconclusive`, `ReplayParity`. | Test (TC-539) |
| FR-128-AC-6 | FR-126-AC-9's deadlock-freedom counterexample under `terminal when 6 / (3 - c.value) = 0`, the prefix to value 3 with `kind: UndefinedEvaluation{where: position 3, cause: division-by-zero}`, replays to `reproduced-with-evaluated-witness` with `trace_position` 3: the letter at the terminal state 3 holds the undefined `P`. The same prefix with `kind: Deadlock` also settles `reproduced-with-evaluated-witness` with value `Undefined(UndefinedEvaluation{where: position 3, cause: division-by-zero})`, and the item settles `refuted`. Over FR-120-AC-9's `test/tallies` subject, FR-126-AC-6's empty-prefix counterexample at `t1` with `kind: UndefinedEvaluation{cause: SumOutOfDomain}` reproduces through the undefined `pre Low`. | Test (TC-539) |

## Dependencies

- ADR-018 §1 UE-5, §5 CX-2 to CX-4, §10 DL-4 and DL-5; ADR-014 §3 TR-1, TR-2 and §5
  A-4; ADR-013 O-25 to O-27.
- [FR-098](FR-098-execute-a-replay-request.md) (facade and refusal order),
  [FR-070](FR-070-implement-typed-counterexample-witness-envelope.md),
  [FR-072](FR-072-implement-typed-replay-result.md),
  [FR-101](FR-101-explore-finite-models-with-canonical-order-and-the-qspec-sampler.md)
  (`replay`), [FR-120](FR-120-simulate-a-checked-package-s-state-family.md)
  (`ModelSystem`), [FR-124](FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md),
  [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md).

## References

- QSpec FR-364 (temporal counterexample replay), FR-368 (negotiating
  temporal model-check items) and FR-366 (deadlock freedom): the QSpec half
  of ADR-018 QS-8, QS-9 and QS-12 (Linear STD-131).
