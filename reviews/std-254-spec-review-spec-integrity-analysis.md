---
id: SR-13770
title: 'Integrity review of QSL PR #1182: FR-329 bounded-profile clause over a zero-counted-loop lasso (STD-254)'
type: SpecReview
analysis: integrity
scope: 'agent-ix/quire-spec-language@7d7a924b7; git diff origin/main...7d7a924b7 (PR #1182, branch std-254-spec): spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md, spec/test-cases/TC-839-s6a-evaluates-an-infinite-trace-clause-over-a-lasso.md; ticket STD-254'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-329
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-839
  type: reviews
---

## Summary

Ticket: STD-254. PR: quire-spec-language#1182, head 7d7a924b7. Method: spec-review, spec-integrity-analysis. The PR adds an ADR-027 dependency to FR-329, a Description sentence and a Behavior section "Bounded-profile clause over a zero-counted loop", FR-329-AC-8 to AC-10, and TC-839 steps 6 and 7 with their expected results.

Evidence:

- Implementation from QSL #1158 (a79d0c42f), read at the PR head. `qsl-eval/src/temporal/engine.rs:343-366` admits a bounded-profile lasso only when no loop step and no closing step counts (`loop_without_progress`), and `Shape::new` (engine.rs:186-196) refuses every other bounded-profile lasso with `TemporalRefusal::LassoUnderBoundedProfile`. Letters count every step by default (`temporal/letters.rs:95-103`), so an observed lasso and a model-subject lasso are always refused. Only protocol letters report uncounted steps (`model_check/replay.rs:1216-1226`, `simulation/protocol.rs:1489`). FR-217 protocol replay checks the lasso against `ProtocolSystem::fairness_set`, which adds the scheduler constraints whatever the profile (`simulation/protocol/fairness.rs:310`), and refuses `UnfairProtocolLasso` (`model_check/replay.rs:720-727`).
- Probe in a scratch copy (`rv1182/probe`, its own target dir). qsl-eval: a bounded lasso whose loop has one counted step returns `LassoUnderBoundedProfile` with catalog code `unsupported_construct`/`expression-form`, and an all-counted lasso returns the same code. The AC-8 shape gives `Violated{at: 0}` for `eventually[1,1]` and `Holds` for `eventually[0,1]`. qsl-analyze: the bounded `eventually[0,2]` duplicate/lose lasso found under `scheduling adversarial` replays as `Reproduced{Truth(false), position 0}`. Replayed against the default package with the scheduler set `[fair weak whole Root]`, it refuses `UnfairProtocolLasso{Root}`, `InvalidRuntimeInput`/`invalid-value`.
- Gates: `make check-spec-validation` exits 0, and FR-329 and TC-839 raise no warning. `tools/check-index-completeness.sh` exits 0. The PR touches nothing under `docs/` and no vector table that a test parses. The QSpec gates do not apply because the PR changes no QSpec file.
- Ids: AC-8, AC-9 and AC-10 are new and unique. TC-839 Scope names them. The new text has no changelog, status or review language.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-329 Description, new sentence | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:38-41 | examined |
| FR-329 Bounded-profile clause over a zero-counted loop | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:134-159 | examined |
| FR-329-AC-8 | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:172 | examined |
| FR-329-AC-9 | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:173 | examined |
| FR-329-AC-10 | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:174 | examined |
| FR-329 Inputs, Outputs, Lasso shape, Fairness | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:52-108 | examined |
| TC-839 Scope, steps 6-7, expected results | spec/test-cases/TC-839-s6a-evaluates-an-infinite-trace-clause-over-a-lasso.md | examined |
| ADR-027 PB-3, PA-3, PS-6 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:138,240,260 | context_only |
| FR-338 Protocol non-progress acceptance | spec/functional/FR-338-check-an-en-1-closure-certificate.md:369-440 | context_only |
| FR-212 Description | spec/functional/FR-212-derive-scheduler-fairness-with-an-adversarial-opt-out.md:31-39 | context_only |
| FR-123 bounded-profile fairness refusal | spec/functional/FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md:91-92 | context_only |
| ADR-018 SM-1, CX-1 | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:245,340 | context_only |
| FR-327 Description, FR-330 Behavior | spec/functional/FR-327-evaluate-a-temporal-clause-over-a-finite-trace.md:30-38; spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md:79-84 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-329-AC-9 and the new Behavior bullet say every other bounded-profile lasso refuses `invalid_runtime_input`/`invalid-value`. The evaluator returns `LassoUnderBoundedProfile`, catalogued `unsupported_construct`/`expression-form`, and the probe confirms that code for a counted-step loop and an all-counted loop. FR-217 replay of a bounded protocol lasso whose loop counts settles `no_value` and does not refuse. Only the FR-330 spine admit gate returns `invalid-value`, and only for an observed lasso. Either the spec states the code the evaluator returns, or a code change is filed. The ticket's claim that the evaluator already behaves this way does not hold for the code. | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:136-142,173; qsl-eval/src/temporal/outcome.rs:177-182; qsl-replay/src/spine/clause.rs:533-536; qsl-replay/src/model_check/replay.rs:1363-1366 |
| FND-002 | medium | The new fairness bullet reads the bounded clause's fairness set as one that includes the scheduler constraints (ADR-027 PA-3). FR-212 and PA-3 add those constraints only to every infinite-trace clause, and FR-123 refuses any fairness constraint on a bounded clause. Read literally, a bounded clause's fairness set is empty, so AC-10 has no constraint to refuse with. The code (fairness.rs:310) and FR-338 apply the scheduler constraints whatever the profile. FR-212 and PA-3 now contradict FR-329 and need the bounded case, or FR-329 must name the constraints FR-212 derives for the protocol without calling them the clause's set. | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:152-159; spec/functional/FR-212-derive-scheduler-fairness-with-an-adversarial-opt-out.md:35-39; spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:260 |
| FND-003 | medium | The new text admits a `Lasso::Model` over a protocol subject, but FR-329's Inputs still define `Lasso::Model` with a model subject (FR-125) and FR-128 steps. Lasso shape reconstructs positions only through FR-120's `ModelSystem`, and Fairness reads a constraint only through transition-identity membership (ADR-018 FA-1). Those rules do not cover reconstruction through `ProtocolSystem` (ADR-027 CX-3) or the taken and enabled reading of a branch or root scheduler target (PA-2), which the new bullet points to as "the rule of the Fairness section". The evaluator API has no `Lasso::Model` input (temporal_trace.rs:889 notes the gap for AC-3), so AC-8 and AC-10 can be built only through FR-217 `replay_protocol_trace`. | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:52-108,152-155 |
| FND-004 | low | ADR-018 SM-1 still lists the evaluator's readings as "the bounded profile's own evaluation over a finite trace, exact evaluation over a lasso under infinite-trace", and CX-1 says a bounded-profile safety refutation is a finite prefix. FR-329 now reads a bounded clause over a lasso, and FR-338 returns a lasso as a bounded refutation, so both rows omit the new case. | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:245,340 |
| FND-005 | low | FR-330's reason "a bounded profile reads a closed finite trace" and FR-327's "Under a bounded profile it SHALL evaluate a closed finite trace" are written as the whole rule and do not point to FR-329's exception. The FR-330 refusal itself stays consistent because its `Model` input is a model subject. | spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md:82-84; spec/functional/FR-327-evaluate-a-temporal-clause-over-a-finite-trace.md:33-34 |
| FND-006 | low | FR-329 Outputs lists the `invalid_runtime_input`/`invalid-value` refusal only "for a malformed lasso, or a `Lasso::Model` that is unfair". The new bounded-profile refusal of every other lasso is missing from Outputs. | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:65-73 |

## Verdict

Not mergeable as written. The semantics are right and match ADR-027 PB-3, FR-338 and the implementation. Only a lasso with no counted step and no counted closing step is read under a bounded profile. Interval distance stays constant on the loop, an interval that needs a counted step is false, and a fair such lasso refutes at position 0 while an unfair one refuses naming the scheduler constraint (probe). The lead's defaults are buildable: a `repeat` around a `parallel` gives a fork/join loop (PS-6), and it is fair under the scheduler constraints. Ids are unique, the wording describes behaviour, and every new AC has a TC-839 row. FND-001 has to be settled first, because the stated refusal code is not the one the evaluator returns. FND-002 and FND-003 leave the protocol-subject `Lasso::Model` and its scheduler fairness without a consistent base elsewhere in the spec. FND-004 to FND-006 are small.
