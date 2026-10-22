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

## Dispositions

Disposition pass 1 at `agent-ix/quire-spec-language@9eb06cde9` (fix commit `9eb06cde9`, after merge `c95d770d8` of origin/main; the head contains origin/main). I checked each outcome against the spec at that head and against the evaluator and replay code, not against the commit message. Gates at the head: `make check-spec-validation` exits 0 with no warning on FR-329, TC-839, FR-123, FR-212, FR-327, FR-330, ADR-018 or ADR-027; `tools/check-index-completeness.sh` exits 0; the PR touches only `spec/` and `reviews/`, nothing under `docs/` and no vector table a test parses. The PR changes no QSpec file, so the QSpec gates do not apply. The probe of the review pass already confirmed the codes the fix now states (`LassoUnderBoundedProfile` is `unsupported_construct`/`expression-form`; an unfair bounded zero-counted lasso refuses `UnfairProtocolLasso`, `invalid_runtime_input`/`invalid-value`; the adversarial lasso reproduces `false` at 0), so no re-probe was needed.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed 9eb06cde9 | FR-329 Description, Outputs, the second Behavior bullet and AC-9 now state `unsupported_construct`/`expression-form` (`LassoUnderBoundedProfile`), the code the evaluator returns. A new bullet and AC-9 state that FR-217's replay of a counted-loop protocol lasso settles with no truth value, which matches `replay.rs:1366` (`Err(_) => no_value()`). |
| FND-002 | fixed 9eb06cde9 | FR-212 and ADR-027 PA-3 now say the scheduler constraints also apply to a bounded clause read over a zero-counted protocol lasso; FR-123 says derived scheduler constraints are not authored and are not refused; FR-329 no longer calls them the bounded clause's fairness set and says a bounded clause carries no user fairness constraint. |
| FND-003 | fixed 9eb06cde9 | FR-329 now states that a protocol-subject `Lasso::Model` is FR-217's lasso, rebuilt through `ProtocolSystem` with post-state digests, that a caller builds it through FR-217's replay entry, and that a scheduler constraint is read as taken and enabled by ADR-027 PA-2. AC-8 and AC-10 are built through FR-217's replay. The rule cites `ADR-027 CX-3`, which does not exist (new FND-007). |
| FND-004 | fixed 9eb06cde9 | ADR-018 SM-1 and CX-1 now name the bounded reading of a protocol lasso with no counted step and point to FR-329 (and PB-3, FR-338 in CX-1). |
| FND-005 | fixed 9eb06cde9 | FR-327 now adds "and a protocol lasso whose loop has no counted step as FR-329 states"; FR-330 says the zero-counted protocol lasso FR-329 reads under a bounded profile is not an input of its entry. |
| FND-006 | fixed 9eb06cde9 | FR-329 Outputs adds the `unsupported_construct`/`expression-form` refusal for a lasso under a bounded profile other than a zero-counted protocol lasso. |
| FND-007 | fixed f18e5c026 | FR-329 now cites "`ProtocolSystem` (ADR-027 TS-1, PX-2)"; ADR-027 TS-1 defines `ProtocolSystem` and PX-2 its replay. |
| FND-008 | fixed f18e5c026 | FR-329 Inputs now reads "A `CheckedTemporalClause` under infinite-trace (FR-326, FR-123), with its fairness set; or one under a bounded profile, read only over a zero-counted protocol lasso (Behavior, ...)". |
| FND-009 | fixed f18e5c026 | FR-217 now owns the rule: "When the clause is under a bounded profile and the lasso is fair but its loop has a counted step, the executor SHALL settle `inconclusive` with cause `NoValue` ...; an unfair lasso of that kind SHALL refuse `invalid_runtime_input`/`invalid-value` as unfair", with FR-217-AC-7 and TC-662 step 6; FR-329 and AC-9 point to it. This matches replay.rs (fairness first, then `no_value()`). |
| FND-010 | fixed 2949c4e0e | FR-329-AC-10 now reads "the same lasso replayed against the package that declares `scheduling adversarial` is fair, and with `p` false at every position `eventually[1,1] holds(p)` is `Completed(false)` at position 0", and TC-839 step 8 carries "with `p` false at every position". |

Disposition pass 2 at `agent-ix/quire-spec-language@f18e5c026` (fix commit `f18e5c026`; the head contains origin/main `48b47ab99`). The rows for FND-007 to FND-009 above are pass 2's. I checked each against the spec at that head and against `qsl-replay/src/model_check/replay.rs:497-509` (`no_value()` is `Inconclusive(NoValue)`). Gates: `make check-spec-validation` exits 0 with no warning on any touched file; `tools/check-index-completeness.sh` exits 0; the PR touches only `spec/` and `reviews/`, nothing under `docs/` and no vector table a test parses; no QSpec file changes. FR-217-AC-7 is new, unique in `spec/` and `plan/`, used by no other open PR, and bound to TC-662 (Scope "FR-217-AC-1 to FR-217-AC-7", step 6 and its expected result). No `ADR-027 CX-3` citation and no FR-217-less "settle with no truth value" obligation remains in `spec/`.

Disposition pass 3 at `agent-ix/quire-spec-language@2949c4e0e` (fix commit `2949c4e0e`; the head contains origin/main). The FND-010 row above is pass 3's. Gates: `make check-spec-validation` exits 0 with no warning on any touched file; `tools/check-index-completeness.sh` exits 0; the round-3 diff touches only FR-329-AC-10, TC-839 step 8 and the `reviews/` files. The ids cited in the change resolve, FR-329's AC ids are unique, and SR-13770 to SR-13772 each occur once in the repository.

## New findings (disposition pass 1)

These were found at `9eb06cde9`. They are new, not dispositions of the rows above.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | The new protocol-lasso bullet cites "`ProtocolSystem` (ADR-027 CX-3)". ADR-027 has no CX-3 rule: CX-3 is ADR-018's replay rule, and ADR-027 states `ProtocolSystem` in TS-1 and its replay in PX-2. Fix: cite ADR-027 TS-1 and PX-2 (or ADR-018 CX-3 as amended by ADR-027 PX-2). | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:153; spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:267,279 |
| FND-008 | low | FR-329 Inputs still lists "A `CheckedTemporalClause` under infinite-trace (FR-326, FR-123)" only, while the Description, Outputs and the new Behavior section now take a clause under a bounded profile. Fix: name the bounded-profile clause as an input, limited to the zero-counted protocol lasso. | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:54-55 |
| FND-009 | low | The bullet "FR-217's replay of a protocol lasso whose loop has a counted step SHALL settle with no truth value" and AC-9's last clause do not name the settlement or say the lasso is fair. Replay checks fairness first (`fair_loop`, replay.rs:1296-1300), so an unfair counted-loop lasso refuses `invalid_runtime_input`/`invalid-value` as unfair; a fair one reaches the evaluator refusal and settles `inconclusive` with cause `NoValue` (replay.rs:508-509,1366). FR-217 itself states neither case. Fix: say the lasso is fair and name the settlement in FR-217's vocabulary, or state it in FR-217 and point to it. | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:149-150,192; qsl-replay/src/model_check/replay.rs:1296-1300,1366 |

## New findings (disposition pass 2)

These were found at `f18e5c026`. They are new, not dispositions of the rows above.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | low | FR-329-AC-10 asserts `eventually[1,1] holds(p)` is `Completed(false)` at position 0 but never says what `p` is. AC-10's lasso used to be AC-8's, where `p` is false at every position; it is now any zero-counted unfair lasso (TC-839 step 8 names FR-338-AC-6's duplicate/lose cycle), so the oracle depends on an unstated `p`. Fix: say "with `p` false at every position". | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:196 |
