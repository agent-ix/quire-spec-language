---
id: SR-13547
title: 'Spec review of PR #1098: align FR-189, FR-191 and FR-192 with the EN-4 code, integrity'
type: SpecReview
analysis: integrity
scope: 'agent-ix/quire-spec-language@a53c86bc4; git diff refs/remotes/origin/main...a53c86bc4 (PR #1098, branch std-570-align-spec): spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md, spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md, spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md; compared with PR #1079 branch qsl-570-impl a9a693ac0 and QSpec main 2aadb7e FR-272, FR-331, FR-408, FR-461; ticket QSL-570'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-189
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-191
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-192
  type: reviews
---

## Summary

Ticket: QSL-570. PR: quire-spec-language#1098, head a53c86bc4, scoped to `git diff refs/remotes/origin/main...a53c86bc4` (one commit, three files, +10/-4). Spec-only. It aligns FR-189, FR-191 and FR-192 with terra's EN-4 engine on PR #1079 (branch qsl-570-impl, a9a693ac0). Each claim was checked against that code. FR-189: `check_statistical` raises `TraceIndexRange` only when the method is not `Regenerative` (statistical.rs:643-656), and `regenerative_test` walks at trace index `j` (statistical.rs:451); confirmed. An `Event` sample in `sampled_test` adds 1 or 0 to the sum with no form check, and the fixed-sample method is the only one a mean of a fraction admits (statistical.rs:403-427); confirmed. FR-191: `regenerative_test` adds one to `tally.draws` per walk step and stops at `max_draws` (statistical.rs:462-472), and FR-190 Outputs records the steps taken as `draws`; confirmed. FR-192: `halted` maps every `SampleHalt::Expansion` to `StatisticalStop::Expansion(stop)`, an internal-failure stop included (statistical.rs:282-293). `ExpansionStop` carries only `cause` and `limit: Option<ExpansionLimit>` (qsl-eval/src/simulation/expansion.rs:24-38). #1079 has no statistical settlement map yet. The model-check precedent maps a named limit to a stop naming it, an unnamed internal failure to a fault, and any other unnamed stop to `ResourceExhausted` (state_graph.rs:319-336). QSpec main 2aadb7e was read for the cause vocabulary: FR-331's closed `incomplete` causes are `limit-reached`, `timed-out`, `precision-budget` and `cancelled{source}`, and FR-408 settles every statistical stop `limit-reached` or `cancelled`. Methods: spec-review/base (SR-13545), spec-review/ears (SR-13546), spec-review/integrity (SR-13547). code-review and gap-analysis were not run because no code or test file changed. Gates on the head: make check-spec-validation exits 0 with 17 warnings, none in FR-189, FR-191 or FR-192. The author had not run it. tools/check-index-completeness.sh passes. No status wording in the diff. quoin 0.28.3, quire 0.36.2 (engine 0.50.2). This review: Consistency with QSpec's cause vocabulary, with FR-189's stop rule and with the map's exhaustiveness rule.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-189-refusals | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md | examined |
| FR-189-mean-of-fraction-bullet | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md | examined |
| FR-191-budget-bullet | spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md | examined |
| FR-192-map-exhaustive | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md | examined |
| FR-192-expansion-limit-row | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md | examined |
| FR-192-expansion-unnamed-row | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md | examined |
| FR-192-inputs | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md | examined |
| FR-190-outputs | spec/functional/FR-190-measure-a-long-run-fraction-by-regeneration.md | context_only |
| PR-1079-code | agent-ix/quire-spec-language@a9a693ac0 qsl-analyze/src/statistical.rs, qsl-eval/src/simulation/expansion.rs, qsl-analyze/src/model_check/state_graph.rs | context_only |
| QSpec-FR-331-incomplete | agent-ix/quire-specification@2aadb7e spec/objects/interfaces/FR-331-backend-provider-envelope.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The row writes the cause `resource-exhausted`, which QSpec does not define. FR-331's closed incomplete causes are limit-reached, timed-out, precision-budget and cancelled{source}. FR-331-AC-16 refuses any other spelling on a probabilistic result with invalid_result/unknown-cause. QSpec FR-408 says every statistical stop settles limit-reached or cancelled. QSL's own internal Incomplete(ResourceExhausted) (FR-149, FR-358) is fine, but this column is the wire cause, so a reader would refuse this record. The same check was raised on PR #1082 (SR-13394 FND-004) for the unsupported causes. Fix (owner decision): give this stop an FR-331 spelling QSpec already admits, or pair this PR with a QSpec FR-331/FR-408 change that adds the cause, and name that change in References. | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md:101 |
| FND-002 | high | The map must be exhaustive with no `_` arm, but no row now covers a sample expansion that names no limit and is an internal failure. The new rows exclude it on purpose. FR-189 says every ExpansionStop returns Stopped with StatisticalStop::Expansion, and #1079's halted() does exactly that, internal failures included (statistical.rs:282-293). So that outcome reaches the map with no row. The model-check engine refuses such a stop as a fault (state_graph.rs:319-336), and FR-149 calls an internal failure 'a fault of the checker'. Fix: add a row settling an internal-failure expansion stop `failed`, diagnostic runtime_invariant/established-invariant-broken, as the Internal refusal row does. Otherwise have FR-189 refuse it as Internal before it becomes a Stopped (that is a code change on #1079). | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md:90-101 |
| FND-003 | medium | The named-limit row needs a value and a setting that its inputs do not hold. The map's input is the StatisticalOutcome alone (FR-192 Inputs). Its Expansion(ExpansionStop) carries only cause and limit (expansion.rs:24-31). StatisticalRequest's limits hold only max_samples, max_draws and max_cycle_steps. 'FR-126's setting for that limit' points at nothing either: FR-126 says only that EvaluationMeter and MaxCandidates are members of the subject's FR-120 limits. FR-255's table has no setting name for them, and QSpec FR-461 requires one dotted engine.member name per limit. Fix: say where the value comes from (the subject's FR-120 limits passed to the map, or carried in the stop), and name the setting string for each of the two limits. | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md:46, 100 |

## Verdict

Two high findings and one medium. Not mergeable as written. The unnamed-stop row writes a wire cause that QSpec refuses. The map lost its exhaustiveness for internal-failure expansion stops. The named-limit row's value and setting have no stated source. What holds: the named-limit row's `limit-reached{limit, value, setting}` matches QSpec FR-408-AC-8 (the clause meter settles limit-reached) and FR-331. The FR-189 and FR-191 edits agree with FR-190 and with the code. Owner decision (event as 0/1): consistent with QSpec FR-408, whose mean-of-fraction values lie in [0, 1], and with the code. Owner decision (resource-exhausted): blocked by FND-001 until QSpec admits the spelling or the row uses one it already admits. No relationship edge is missing: FR-192 already depends_on FR-189.

## New findings (disposition pass 1)

Round 1, head 7ff6d6491: fix commit 7ff6d6491 over merge d41f853e, which brings in main 7935368f (QSL-549 #1097, QSL-564 #1096). The fix commit also commits this review's round-0 SR files under reviews/ unchanged, with the posted comment bodies beside them; this file supersedes them. Terra's #1079 branch is now at dd3a654d7. Its qsl-analyze/src/statistical.rs is byte-equal to a9a693ac0, so the round-0 code reading still holds. Each new AC's expected result was checked against that code and its tests. FR-189-AC-13: an Event sample adds 1 or 0 and counts one, so 3 of 4 gives 3/4 (statistical.rs:403-427); a Completed run at exactly 4 samples needs claim parameters whose Okamoto N is 4, for example alpha = beta = 1/5 and iota = 1/2. FR-189-AC-14: the regenerative test a_regenerative_request_is_not_refused_for_the_interleaved_trace_range runs at u64::MAX, and Okamoto with two tests at u64::MAX is refused TraceIndexRange{u64::MAX, 2} (statistical/tests.rs:1160-1176, 1575-1589). FR-191-AC-2: regenerative_test checks max_draws before every step, so a max_draws below one cycle's steps stops Limit{MaxDraws, value} with no decision (statistical.rs:462-472). Gates: make check-spec-validation exits 0 with the same 17 warnings, none in the changed files; tools/check-index-completeness.sh passes. The FR text has no status wording. The PR body says correctly that neither FR-255 nor QSpec FR-461 registers `model_check.meter` or `model_check.max_candidates`. FR-255's table holds no engine setting, `statistical.*` included, so that is not a defect of this PR.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | The row now gives every expansion stop that names no limit the diagnostic runtime_invariant/established-invariant-broken, an O-16 internal failure. Most such stops are not internal failures. ExpansionStop::is_internal_failure separates them, and FR-149-AC-10's example is a state-constraint family that evaluation refuses, an input cause. So the record calls a model's own refusal a QSL defect. It also disagrees with QSL's other engines. FR-149 (merged, #1097) and FR-144 settle that same non-internal stop Incomplete(ResourceExhausted) under V-7, and the model-check engine maps it to ResourceExhausted, not a fault (state_graph.rs:319-336). The PR body's 'like the model-check engine' is therefore wrong for the non-internal half. Fix (owner decision): keep runtime_invariant/established-invariant-broken for an is_internal_failure stop only. Give a non-internal unnamed stop a `failed` record whose diagnostic is the stop's own catalog cause (FR-331's `failed` carries exactly one diagnostic, and QSpec FR-408 says such a stop 'stops the run with that cause'), or settle it as FR-149 does once QSpec admits a matching incomplete cause. Correct the PR body either way. | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md:101 |
| FND-005 | low | The named-limit row now says the map 'receives with the outcome' the configured bound in the request's FR-120 limits, but FR-192 Inputs still lists only a StatisticalOutcome or a StatisticalRefusal. Fix: add the request's FR-120 limits (the evaluation meter budget and max_candidates) to Inputs, so the map's signature can be read from the requirement. | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md:46-47 |

## Dispositions

Round 1, head 7ff6d6491: fix commit 7ff6d6491 over merge d41f853e, which brings in main 7935368f (QSL-549 #1097, QSL-564 #1096). The fix commit also commits this review's round-0 SR files under reviews/ unchanged, with the posted comment bodies beside them; this file supersedes them. Terra's #1079 branch is now at dd3a654d7. Its qsl-analyze/src/statistical.rs is byte-equal to a9a693ac0, so the round-0 code reading still holds. Each new AC's expected result was checked against that code and its tests. FR-189-AC-13: an Event sample adds 1 or 0 and counts one, so 3 of 4 gives 3/4 (statistical.rs:403-427); a Completed run at exactly 4 samples needs claim parameters whose Okamoto N is 4, for example alpha = beta = 1/5 and iota = 1/2. FR-189-AC-14: the regenerative test a_regenerative_request_is_not_refused_for_the_interleaved_trace_range runs at u64::MAX, and Okamoto with two tests at u64::MAX is refused TraceIndexRange{u64::MAX, 2} (statistical/tests.rs:1160-1176, 1575-1589). FR-191-AC-2: regenerative_test checks max_draws before every step, so a max_draws below one cycle's steps stops Limit{MaxDraws, value} with no decision (statistical.rs:462-472). Gates: make check-spec-validation exits 0 with the same 17 warnings, none in the changed files; tools/check-index-completeness.sh passes. The FR text has no status wording. The PR body says correctly that neither FR-255 nor QSpec FR-461 registers `model_check.meter` or `model_check.max_candidates`. FR-255's table holds no engine setting, `statistical.*` included, so that is not a defect of this PR.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
