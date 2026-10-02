---
id: FR-217
title: "Carry and replay a protocol counterexample"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-024
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-211
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-212
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-215
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-331
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-433
    type: depends_on
---
# FR-217: Carry and replay a protocol counterexample

## Description

A counterexample over a protocol subject SHALL be ADR-018's
`TemporalCounterexample` naming the protocol subject, with each step as its
protocol transition identity and its post-state's state-key digest, and, for
a deadlock, FR-211's blocked threads (ADR-027 PX-1). `qsl_replay::
replay_model_trace` SHALL replay it by FR-128's rules through
`ProtocolSystem`, check a lasso against the fairness set over FR-212's
classes, and for a deadlock check that no protocol step is enabled at the
last state, that some protocol instance is live, and that the blocked threads
recompute equal (ADR-027 PX-2).

## Use case

A verification operator receives a deadlock report for a protocol and hands
it to an auditor who has only the source and the documents. The auditor
replays it: the protocol is rebuilt from source, each fork and attempt is
re-run, the last state is confirmed stuck with the same waiting threads and
causes, and the report counts as a refutation only then.

## Inputs

- FR-128's request, with the subject's protocol, `over` binding, role
  bindings and universes.
- A `WitnessEnvelope<TemporalCounterexample>` whose source is
  `ReplaySource::ModelTrace` and whose subject is a protocol subject: the
  initial-state index, `prefix` and `loop` as `Vec<ProtocolStep{transition:
  ProtocolTransition, post_state: DigestRecord}>`, the stutter marker, the
  `over` binding, the fairness set with each constraint's origin, `kind`,
  and for `kind: Deadlock`, `blocked`.

## Outputs

- An FR-072 replay result on the `ModelTrace` arm, or a typed
  `ReplayRefusal` with no partial result.

## Behavior

- The executor SHALL recompile the package (FR-098), resolve the protocol
  clause and the claim, re-admit the initial state, universes and bindings,
  build `ProtocolSystem` with the subject's memory models, and re-execute
  each step with FR-101 `replay`.
- The executor SHALL read the fairness set, scheduler constraints included,
  from the recompiled clause, and SHALL refuse a payload fairness set that
  differs with `stale_dependency`/`content-mismatch`, naming both sets.
- When a step's identity has no successor with the recorded digest, the
  executor SHALL refuse `stale_dependency`/`content-mismatch`, naming the
  step and both digests.
- When a step is not enabled at its pre-state, or a loop's last state is not
  its entry, the executor SHALL refuse `invalid_runtime_input`/
  `invalid-value`, naming the step.
- For `kind: Formula`, the executor SHALL check the lasso against the
  fairness set over FR-212's classes, reading enabledness from
  `ProtocolSystem` at every loop state, and refuse an unfair lasso with
  `invalid_runtime_input`/`invalid-value`; it SHALL then evaluate the
  formula by FR-125 with FR-213's positions and distances.
- For `kind: UndefinedEvaluation`, the executor SHALL evaluate the
  claim's letters along the replayed prefix and settle by ADR-018 UE-5
  (ADR-027 PB-5).
- For `kind: Deadlock`, the executor SHALL enumerate `ProtocolSystem`'s
  steps at the last replayed state, check that none is enabled and that the
  some protocol instance is live, evaluate the protocol's `terminal when`
  predicate there when it has one, and recompute the blocked threads. No
  enabled step, a live instance, the predicate false or absent, and
  equal blocked threads SHALL settle `reproduced-with-evaluated-witness`;
  an enabled step, a state with no live instance or a different blocked
  list SHALL
  settle `inconclusive`, `ReplayParity`.
- Replay SHALL read no path, environment variable, clock or search
  location, and SHALL give the same result for the same request and
  envelope.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-217-AC-1 | The deadlock counterexample of ADR-027 §7 (`fork(Both)`, `attempt(A)`) replays: both post-state digests match, no step is enabled at s2, its instance is live, the recomputed blocked threads equal FR-211-AC-2's, and the result is `reproduced-with-evaluated-witness`. The envelope's steps serialize with protocol transition identities and the blocked list. | Test (TC-662) |
| FR-217-AC-2 | Changing the second step's post-state digest refuses `stale_dependency`/`content-mismatch`, naming the step and both digests. Replacing the second step with `join(Both)` refuses `invalid_runtime_input`/`invalid-value`. Replaying the same steps against the repaired `Fill` (`TwoPre: self.v <= 1`) with recorded digests from the unrepaired one refuses on the package identity by FR-098's rule. | Test (TC-662) |
| FR-217-AC-3 | A deadlock envelope whose blocked list drops `right` settles `inconclusive`, `ReplayParity`. An envelope whose prefix runs on to s6, which holds no live instance, with `kind: Deadlock` settles `inconclusive`, `ReplayParity`. | Test (TC-662) |
| FR-217-AC-4 | The adversarial lasso of FR-212-AC-2 replays against the adversarial `Chatter` package and settles `reproduced-with-evaluated-witness`. The same lasso with the scheduler constraints added to its fairness set, replayed against the default `Chatter` package, refuses `invalid_runtime_input`/`invalid-value` as unfair: `s` is enabled at every loop state and takes no step. | Test (TC-662) |

## Dependencies

- ADR-027 §4.1 PX-1 and PX-2, §3.2 PD-4; ADR-018 §5 CX-2 and CX-3, §10
  DL-5.
- [FR-128](FR-128-replay-a-model-counterexample.md) (the replay rules this
  extends), [FR-098](FR-098-execute-a-replay-request.md),
  [FR-101](FR-101-explore-finite-models-with-canonical-order-and-the-qspec-sampler.md)
  (`replay`), [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (the counterexample), FR-211, FR-212, FR-213, FR-215.
- QSpec owns the counterexample wire for protocol steps and blocked threads
  and their replay (ADR-027 QS-8): QSpec FR-433, over FR-331.

## References

- Owning ticket: Linear QSL-396. QSpec half: QSpec FR-433 (Linear STD-140).
