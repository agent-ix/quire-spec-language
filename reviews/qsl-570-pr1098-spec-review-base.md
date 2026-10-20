---
id: SR-13545
title: 'Spec review of PR #1098: align FR-189, FR-191 and FR-192 with the EN-4 code, base'
type: SpecReview
analysis: base
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

Ticket: QSL-570. PR: quire-spec-language#1098, head a53c86bc4, scoped to `git diff refs/remotes/origin/main...a53c86bc4` (one commit, three files, +10/-4). Spec-only. It aligns FR-189, FR-191 and FR-192 with terra's EN-4 engine on PR #1079 (branch qsl-570-impl, a9a693ac0). Each claim was checked against that code. FR-189: `check_statistical` raises `TraceIndexRange` only when the method is not `Regenerative` (statistical.rs:643-656), and `regenerative_test` walks at trace index `j` (statistical.rs:451); confirmed. An `Event` sample in `sampled_test` adds 1 or 0 to the sum with no form check, and the fixed-sample method is the only one a mean of a fraction admits (statistical.rs:403-427); confirmed. FR-191: `regenerative_test` adds one to `tally.draws` per walk step and stops at `max_draws` (statistical.rs:462-472), and FR-190 Outputs records the steps taken as `draws`; confirmed. FR-192: `halted` maps every `SampleHalt::Expansion` to `StatisticalStop::Expansion(stop)`, an internal-failure stop included (statistical.rs:282-293). `ExpansionStop` carries only `cause` and `limit: Option<ExpansionLimit>` (qsl-eval/src/simulation/expansion.rs:24-38). #1079 has no statistical settlement map yet. The model-check precedent maps a named limit to a stop naming it, an unnamed internal failure to a fault, and any other unnamed stop to `ResourceExhausted` (state_graph.rs:319-336). QSpec main 2aadb7e was read for the cause vocabulary: FR-331's closed `incomplete` causes are `limit-reached`, `timed-out`, `precision-budget` and `cancelled{source}`, and FR-408 settles every statistical stop `limit-reached` or `cancelled`. Methods: spec-review/base (SR-13545), spec-review/ears (SR-13546), spec-review/integrity (SR-13547). code-review and gap-analysis were not run because no code or test file changed. Gates on the head: make check-spec-validation exits 0 with 17 warnings, none in FR-189, FR-191 or FR-192. The author had not run it. tools/check-index-completeness.sh passes. No status wording in the diff. quoin 0.28.3, quire 0.36.2 (engine 0.50.2). This review: Each changed statement was checked against #1079's code and is accurate; the gaps are missing acceptance criteria for the new behaviour.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-189-mean-of-fraction-bullet | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md | examined |
| FR-189-refusals | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md | examined |
| FR-191-limits-struct | spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md | examined |
| FR-191-budget-bullet | spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md | examined |
| FR-192-expansion-limit-row | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md | examined |
| FR-192-expansion-unnamed-row | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md | examined |
| FR-192-AC-1 | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md | examined |
| FR-190-outputs | spec/functional/FR-190-measure-a-long-run-fraction-by-regeneration.md | context_only |
| PR-1079-code | agent-ix/quire-spec-language@a9a693ac0 qsl-analyze/src/statistical.rs, qsl-eval/src/simulation/expansion.rs, qsl-analyze/src/model_check/state_graph.rs | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The two new map rows have no acceptance criterion and no TC-627 step. FR-192-AC-1 lists the rows it tests, and an expansion stop is not among them, so nothing checks that a meter or max_candidates stop settles limit-reached with its limit, value and setting, or what an unnamed stop settles. Fix: extend FR-192-AC-1 (and TC-627) with an evaluation-meter stop naming its limit, value and setting, and an unnamed non-internal stop, as FR-149-AC-10 does for the certificate checker. | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md:100-101 |
| FND-002 | medium | The owner decision to read an event sample as fraction 1 or 0 for a mean-of-fraction claim, with no refusal, has no acceptance criterion and no TC-624 step. The code does this (statistical.rs:403-427), but a later change to refuse it, or to refuse FractionForEvent on the wrong side, would break no test. Fix: add an FR-189 AC (with a TC-624 step) where a sampler stub returns Event samples for a mean-of-fraction claim and the estimate equals the share of samples that hold. | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:150-155 |
| FND-003 | low | The regenerative reading of max_draws (walk steps per test), and the TraceIndexRange exemption for the regenerative method in FR-189, have no acceptance criterion. FR-191-AC-2 tests max_draws only for the activation variant under Okamoto. Fix: add to FR-191-AC-2 (TC-626 step 2) a regenerative run with max_draws below the steps one cycle needs that stops Limit{MaxDraws, value}, and a regenerative request with m = 2 and max_draws = u64::MAX that is not refused TraceIndexRange. | spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md:75-77 |

## Verdict

Two medium findings and one low. What holds: every claim in the PR body matches the code on qsl-570-impl. TraceIndexRange is raised only for Okamoto and SPRT. An event sample counts as 1 or 0 for a mean of a fraction. max_draws counts walk steps for the regenerative method, matching FR-190's draws. The new behaviour has no acceptance criterion.

## New findings (disposition pass 2)

Round 2, head 0ed9ee0a4: fix commit 0ed9ee0a4 over 7ff6d6491. The fix commit also commits this review's round-1 SR files and comment bodies under reviews/ unchanged; this file supersedes them. Terra's #1079 branch is still at dd3a654d7. Gates: make check-spec-validation exits 0 with the same 17 warnings, none in the changed files; tools/check-index-completeness.sh passes. The FR text has no status wording; 'the closest cause FR-331 admits' is rationale. The PR body records the non-internal wire form as an owner decision and recommends a QSpec FR-331/FR-408 resource-exhaustion cause, which is honest. Its older 'Owner decisions' line still says 'settling an unnamed expansion stop `failed`', which is now true only of internal failures. AC-13 was checked against Okamoto's N in QSpec FR-408 and method.rs:51-60 (N = ceil(ln(1/min(α', β')) / (2ι²)), the log's upper bound), and against StatisticalClaim::new (statistical/claim.rs:77-92).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | high | N = 4 is right: ln 5 / (2 · 1/4) is about 3.22, which rounds up to 4. But ι = 1/2 admits no threshold. StatisticalClaim::new refuses NoIndifferenceRegion when θ − ι <= 0 or θ + ι >= 1 (claim.rs:83-85; FR-186's invalid-confidence), and θ − 1/2 > 0 together with θ + 1/2 < 1 has no solution. So no claim with these parameters can be built, and the AC cannot pass. Fix: use parameters with ι < 1/2 that still give N = 4, for example α = β = 1/3, ι = 2/5 (ln 3 / 0.32 is about 3.43, which rounds up to 4) with θ = 1/2. Name θ in the AC. | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:222 |

## Dispositions

Round 1, head 7ff6d6491: fix commit 7ff6d6491 over merge d41f853e, which brings in main 7935368f (QSL-549 #1097, QSL-564 #1096). The fix commit also commits this review's round-0 SR files under reviews/ unchanged, with the posted comment bodies beside them; this file supersedes them. Terra's #1079 branch is now at dd3a654d7. Its qsl-analyze/src/statistical.rs is byte-equal to a9a693ac0, so the round-0 code reading still holds. Each new AC's expected result was checked against that code and its tests. FR-189-AC-13: an Event sample adds 1 or 0 and counts one, so 3 of 4 gives 3/4 (statistical.rs:403-427); a Completed run at exactly 4 samples needs claim parameters whose Okamoto N is 4, for example alpha = beta = 1/5 and iota = 1/2. FR-189-AC-14: the regenerative test a_regenerative_request_is_not_refused_for_the_interleaved_trace_range runs at u64::MAX, and Okamoto with two tests at u64::MAX is refused TraceIndexRange{u64::MAX, 2} (statistical/tests.rs:1160-1176, 1575-1589). FR-191-AC-2: regenerative_test checks max_draws before every step, so a max_draws below one cycle's steps stops Limit{MaxDraws, value} with no decision (statistical.rs:462-472). Gates: make check-spec-validation exits 0 with the same 17 warnings, none in the changed files; tools/check-index-completeness.sh passes. The FR text has no status wording. The PR body says correctly that neither FR-255 nor QSpec FR-461 registers `model_check.meter` or `model_check.max_candidates`. FR-255's table holds no engine setting, `statistical.*` included, so that is not a defect of this PR.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |

Round 3, head cdbf4a0ab: fix commit cdbf4a0ab over 0ed9ee0a4, which also commits this review's round-2 SR files and comment bodies under reviews/ unchanged; this file supersedes them. Terra's #1079 branch is still at dd3a654d7, and statistical/claim.rs is byte-equal to the round-2 read. AC-13 was checked against the code. Confidence::new admits α = β = 1/3 and ι = 2/5, each in (0, 1) (qsl-replay/src/statistical.rs:132-150). StatisticalClaim::new admits θ = 1/2, since θ − ι = 1/10 > 0 and θ + ι = 9/10 < 1 (statistical/claim.rs:83-85); a non-strict bound is needed, because a Below bound on a mean of a fraction is refused StrictBound. Okamoto's N with m = 1 is ⌈ln 3 / (2 · 4/25)⌉ = ⌈3.43⌉ = 4 (method.rs:51-60, QSpec FR-408). The final PR body is current: its owner-decision notes describe the failed-with-own-cause settlement and the recommended QSpec FR-331/FR-408 resource-exhaustion cause, and the stale 'unnamed stop failed' line now reads 'failed with its own cause'. The FR text has no status wording. Gates: make check-spec-validation exits 0 with the same 17 warnings, none in the changed files; tools/check-index-completeness.sh passes.

| FND | outcome | reason |
| --- | --- | --- |
| FND-004 | fixed | - |
