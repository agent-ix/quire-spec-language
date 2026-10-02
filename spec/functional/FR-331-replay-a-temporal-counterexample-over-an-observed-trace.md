---
id: FR-331
title: "Replay a temporal counterexample over an observed trace"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-033
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
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-327
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-328
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-329
    type: depends_on
---
# FR-331: Replay a temporal counterexample over an observed trace

## Description

The layer-6 replay facade `qsl_replay::replay` (FR-098) SHALL replay a
`WitnessEnvelope<TemporalCounterexample>` whose steps are observed
documents: it recompiles the package, resolves the clause, checks that the
recompiled clause's profile selection, fairness set and failing interval key
equal the packet's, reconstructs the trace, and re-evaluates the formula at
the packet's `trace_position` with the TemporalTrace evaluator (ADR-014 §10
scenario 5). It settles an FR-072 replay result. A counterexample over a
model subject replays through `replay_model_trace` (FR-128); both share the
payload type and its evaluation.

The observed-step form is `qsl-replay`'s own input, for a refutation found
over a supplied trace (FR-327 to FR-329); QSpec FR-364's wire carries
model steps.

## Use case

A backend refutes a temporal claim and returns a counterexample trace. An
auditor with the source and the trace's documents replays it without the
backend. The replay accepts it only when it was produced for the same
clause, profile and fairness premise, and only when the evaluator agrees the
formula is false on it.

## Inputs

- FR-098's request: package reference, byte provision holding each observed
  document by `sha256-jcs` digest, and limits.
- A `WitnessEnvelope<TemporalCounterexample>` (FR-070) whose payload is
  `TemporalCounterexample{steps, fairness, interval, kind}` with
  `steps: CounterexampleSteps::Observed{prefix: Vec<DocumentRef>, loop:
  Vec<DocumentRef>}`, `fairness` the clause's fairness constraint nodes,
  `interval: Option<IntervalKey>` (the failing operator's key under a
  bounded profile, `None` under infinite-trace), and `kind: Formula` or
  `UndefinedEvaluation{where, cause}`. Its
  `trace_position` names the failing position (ADR-014 TR-2), and its
  clause node and occurrence key name the clause.
- `TemporalCounterexample` implements `FamilyPayload` (FR-070-AC-5).
  `CounterexampleSteps::Model` is FR-128's step content.

## Outputs

- An FR-072 replay result holding the decoded position and the evaluated
  value, or a typed `ReplayRefusal` with no partial result.

## Behavior

- The facade SHALL recompile and check the package by FR-098's rules and in
  FR-098's order, and SHALL resolve the occurrence key to the clause's
  operator node.
- When the recompiled clause's profile selection, fairness set or failing
  interval key differs from the packet's, the facade SHALL refuse `stale_dependency`/`content-mismatch`, naming the member
  that differs.
- When the loop is present and empty, or the decoded `trace_position` lies
  outside the represented trace, the facade SHALL refuse
  `invalid_runtime_input`/`invalid-value`.
- The facade SHALL admit each observed document by FR-106 and re-evaluate
  the formula over the reconstructed trace: by FR-327 under a bounded
  profile, by FR-328 for an infinite-trace prefix with no loop, and by
  FR-329 for an infinite-trace lasso, including its refusals.
- When the clause's fairness set is non-empty, the facade SHALL refuse with
  `ReplayRefusal::MissingFairnessPremise{constraint}`, naming the first
  constraint of the checked fairness set, with catalog code
  `unsupported_projection`/`missing-fairness-premise`, O-16 unsupported,
  and no result: an observed trace carries no enabledness (QSpec FR-362;
  FR-328, FR-329).
- When the evaluation is `Completed(false)` at the packet's
  `trace_position`, the facade SHALL settle
  `reproduced-with-evaluated-witness`. When it is anything else (true,
  pending, or false at another position), the facade SHALL settle
  `inconclusive` with `InconclusiveCause::ReplayParity` (ADR-013 C-09,
  O-27).
- For a packet whose refutation is `UndefinedEvaluation`, an evaluation
  that returns `Undefined` at the packet's `trace_position` with an equal
  cause SHALL settle `reproduced-with-evaluated-witness`: the reproduced
  undefined value is the witness (ADR-018 UE-5). Anything else SHALL settle
  `inconclusive`, `ReplayParity`.
- Replay SHALL need no backend and no solver.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-331-AC-1 | A packet for the `Counter` unit's infinite-trace clause `Reaches` (`eventually holds(c.value = 2)`) with an observed lasso, empty prefix, loop 0, 1, and `trace_position` `0` settles `reproduced-with-evaluated-witness`; the same packet with loop 0, 1, 2 settles `inconclusive`, `ReplayParity`. | Test (TC-841) |
| FR-331-AC-2 | A packet for the bounded clause `Bounded` (`eventually[0,1] holds(c.value = 2)`) over the finite trace 0, 1, 2 with `interval` `[0,1]` under event-position and `trace_position` `0` settles `reproduced-with-evaluated-witness`; with `trace_position` `1` it settles `inconclusive`, `ReplayParity`; with `interval` `[0,2]` it refuses `stale_dependency`/`content-mismatch` naming the interval. | Test (TC-841) |
| FR-331-AC-3 | AC-1's first packet recompiled from a unit that selects event-position for `Reaches` refuses `stale_dependency`/`content-mismatch` naming the profile; one whose packet lists a fairness constraint the clause does not have refuses the same way naming the fairness set; one with an empty loop, or `trace_position` `7`, refuses `invalid_runtime_input`/`invalid-value`. | Test (TC-841) |
| FR-331-AC-4 | A packet for FR-327-AC-5's `Undefined` outcome with `trace_position` `2` settles `reproduced-with-evaluated-witness`; the same packet with `trace_position` `1` settles `inconclusive`, `ReplayParity`. | Test (TC-847) |
| FR-331-AC-5 | AC-1's first packet for a clause `ReachesFair` (`eventually holds(c.value = 2)` under `fair weak inc`), whose packet fairness set equals the recompiled clause's, refuses `ReplayRefusal::MissingFairnessPremise` naming `fair weak whole inc`, catalog code `unsupported_projection`/`missing-fairness-premise`, O-16 unsupported, with no result and no formula evaluated. | Test (TC-841) |

## Dependencies

- ADR-014 §3 TR-2, §10 scenario 5 (amended in place with the observed step
  content), §11; ADR-018 SM-1, SM-7; ADR-013 C-09, O-25, O-27.
- [FR-098](FR-098-execute-a-replay-request.md),
  [FR-070](FR-070-implement-typed-counterexample-witness-envelope.md),
  [FR-072](FR-072-implement-typed-replay-result.md),
  [FR-128](FR-128-replay-a-model-counterexample.md) (the model-subject step
  content and `replay_model_trace`), [FR-327](FR-327-evaluate-a-temporal-clause-over-a-finite-trace.md),
  [FR-328](FR-328-evaluate-an-infinite-trace-clause-over-a-finite-prefix.md),
  [FR-329](FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md).

## Status

Specified; not yet implemented.

## References

- Linear QSL-384 (spec ticket); QSL-43 (implementation).
- QSpec FR-364 (counterexample replay and wire): Linear STD-131 (QS-8).
- Linear STD-147 (a QSpec wire form for a counterexample over a supplied
  trace); Linear QSL-460 (deleting `ClauseRunProvenance` and
  `ClauseRunReport::source_digest` from the code).
- QSpec FR-362 (the missing fairness premise).
