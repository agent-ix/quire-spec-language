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

## Inputs

- FR-098's request: package reference, byte provision and limits. The byte
  provision holds the subject's initial-state snapshots by `sha256-jcs`
  digest; the request names the subject's universes.
- A `WitnessEnvelope<TemporalCounterexample>` (FR-070) whose payload is a
  `TemporalCounterexample` over a model subject: `initial` (index into the
  subject's initial states), `prefix` and `loop` as `Vec<ModelStep>`, each
  `ModelStep{transition: ModelTransition, post_state: DigestRecord}`, the
  terminal stutter marker, the `over` binding, the fairness set, `interval`
  and `kind: CounterexampleKind::{Formula, Deadlock}`. Its
  `trace_position` names the failing position (ADR-014 TR-2), and its
  `clause_node` and `occurrence_key` name the clause, or the deadlock-
  freedom item.
- `TemporalCounterexample` implements `FamilyPayload` (FR-070-AC-5).

## Outputs

- An FR-072 replay result on the `ModelTrace` arm, or a typed
  `ReplayRefusal` with no partial result. A result retains the source
  identity and digest, the `package_id`, the clause identities, the initial
  state's document identity and digest, each replayed post-state digest, and
  the evaluated value.

## Behavior

- The executor SHALL recompile and check the package by FR-098's rules, in
  FR-098's order, and refuse by FR-098's stale `package_id` rule on a
  `package_id` mismatch.
- The executor SHALL resolve the clause by its `clause_node` and
  `occurrence_key`, and refuse `stale_dependency`/`revision-mismatch`,
  naming both identities, when either differs from the recompile.
- The executor SHALL read the fairness set from the recompiled clause. A
  payload fairness set that differs from it SHALL refuse
  `stale_dependency`/`revision-mismatch`, naming both sets.
- The executor SHALL build the subject's `ModelSystem` by FR-120 from the
  byte provision and the request's universes, and refuse with FR-106's
  record when admission fails. An `initial` index outside the subject's
  initial states SHALL refuse `invalid_runtime_input`/`invalid-value`.
- The executor SHALL re-execute the prefix, then the loop, from the indexed
  initial state with FR-101 `replay`:
  - if a step's transition is not enabled at its pre-state, then the
    executor SHALL refuse `invalid_runtime_input`/`invalid-value`, naming
    the step;
  - if a step's recomputed post-state digest differs from the recorded one,
    then the executor SHALL refuse `stale_dependency`/`revision-mismatch`,
    naming the step and both digests;
  - if the loop does not end at its entry state, then the executor SHALL
    refuse `invalid_runtime_input`/`invalid-value`;
  - if the payload carries a stutter marker and the loop's state is not
    terminal (FR-124), then the executor SHALL refuse
    `invalid_runtime_input`/`invalid-value`.
- For `kind: Formula` the executor SHALL check the lasso against the
  fairness set, reading enabledness from `ModelSystem` at every loop state,
  and refuse an unfair lasso with `invalid_runtime_input`/`invalid-value`
  (ADR-014 A-4). It SHALL then evaluate the formula over the replayed trace
  by FR-125 under the `over` binding.
  - `false` SHALL settle `reproduced-with-evaluated-witness`.
  - `true`, or no value, SHALL settle `inconclusive`, `Verdicts` or
    `NoValue` (FR-072).
- For `kind: Deadlock` the executor SHALL enumerate the transition
  identities enabled at the last replayed state through `ModelSystem`, and
  evaluate the state model's `terminal when` predicate there when it has
  one. No enabled identity and the predicate `false` or absent SHALL settle
  `reproduced-with-evaluated-witness`; otherwise `inconclusive`,
  `Verdicts`.
- An internal fault SHALL refuse with an `InternalFault` and settle no
  result.
- Replay SHALL read no path, environment variable, clock or search
  location, and SHALL give the same result for the same request and
  envelope.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-128-AC-1 | FR-126-AC-1's `fair weak` lasso replays to `reproduced-with-evaluated-witness` with `trace_position` 0, holding the initial snapshot's identity and digest and the three post-state digests. FR-126-AC-2's bounded prefix and FR-126-AC-3's stutter lasso each reproduce. | Test (TC-523) |
| FR-128-AC-2 | FR-126-AC-3's deadlock counterexample reproduces; the same payload replayed against the `Counter` unit with `terminal when` covering value 3 settles `inconclusive`, `Verdicts`; the payload truncated to end at value 2 settles `inconclusive`, `Verdicts`, since `inc` is enabled there. | Test (TC-523) |
| FR-128-AC-3 | Refusals settle no result: the `fair weak` lasso with its last step removed (the loop does not close); the same lasso in an envelope for the `fair weak each attemptUpdate` clause, carrying that clause's identities and fairness set (unfair); the `fair weak` envelope with its payload fairness set changed to `each` (`stale_dependency`/`revision-mismatch` naming both sets); one post-state digest altered (`stale_dependency`/`revision-mismatch` naming the step and both digests); a step `upd(a)` replaced by `attemptUpdate` with receiver `z`, outside the universe (not enabled); an `initial` index of 1 over a one-snapshot subject; a stutter marker on a non-terminal loop. | Test (TC-523) |
| FR-128-AC-4 | A lasso of `upd(a)` steps for `c = a` over ADR-018 §6's subject, which visits `a.versionNumber = 2`, settles `inconclusive`, `Verdicts`. A source edit that changes the `package_id` refuses by FR-098's rule, and a `clause_node` naming another clause refuses `stale_dependency`/`revision-mismatch`. Replaying one envelope twice gives equal results. | Test (TC-523) |

## Dependencies

- ADR-018 §5 CX-2 to CX-4, §10 DL-4 and DL-5; ADR-014 §3 TR-1, TR-2 and §5
  A-4; ADR-013 O-25 to O-27.
- [FR-098](FR-098-execute-a-replay-request.md) (facade and refusal order),
  [FR-070](FR-070-implement-typed-counterexample-witness-envelope.md),
  [FR-072](FR-072-implement-typed-replay-result.md),
  [FR-101](FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md)
  (`replay`), [FR-120](FR-120-simulate-a-checked-package-s-state-family.md)
  (`ModelSystem`), [FR-124](FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md),
  [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md).
- QSpec owns the counterexample wire and its replay rules (ADR-018 QS-8)
  and the request's subject members (QS-9).
