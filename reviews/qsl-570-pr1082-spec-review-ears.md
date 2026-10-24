---
id: SR-13393
title: 'Spec review of PR #1082: statistical Stopped shape, provenance, estimate, edge cases, EARS'
type: SpecReview
analysis: ears-conformance
scope: 'agent-ix/quire-spec-language@73b9df742; git diff refs/remotes/origin/main...73b9df742 (PR #1082): spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md, spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md, spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md, spec/test-cases/TC-624-statistical-model-checking-decides-probabilistic-claims.md, spec/test-cases/TC-626-statistical-limits-and-reproduction.md; compared with PR #1079 head 149fdd4b7 and QSpec main 0ad376803 FR-331, FR-408, FR-409; ticket QSL-570'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-189
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-191
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-192
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-624
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-626
  type: reviews
---

## Summary

Ticket: QSL-570. PR: quire-spec-language#1082, head 73b9df742, scoped to `git diff refs/remotes/origin/main...73b9df742` (one commit on main 47047b1df, five files, +49/-11). Spec-only. It decides six spec gaps G1 to G6 that terra's code PR #1079 (head 149fdd4b7, branch qsl-570-impl) raised. EARS review of the six new Behavior bullets. G1 to G6 provenance: the QSL-570 comment says they are 'listed in the #1079 PR body'. That body lists only the PR 2 remaining work, the PR has no comments or reviews, and the body has no edit history. So the gaps' text is not recoverable, and the author worked from the six titles. The decisions were compared with terra's code: qsl-replay/src/statistical.rs and qsl-analyze/src/statistical.rs, which hold the Stopped and Unsupported shapes, the per-outcome provenance, `TestResult` and `StatisticalVerdict` `estimate: Option<Rational>`, `least_favourable`, the `max_samples` check order and the refusal set. QSpec was read at main 0ad376803: FR-331-AC-28, FR-408 Admission and Activation, FR-409 combination rules, FR-363, FR-391 and FR-399. QSL context: ADR-018 V-6, ADR-024 SV-2 and SV-4, ADR-028 XV-4, FR-126, FR-190. Methods: spec-review/base (SR-13392), spec-review/ears (SR-13393), spec-review/integrity (SR-13394). code-review and gap-analysis were not run because no code or test file changed. Gates on the head: make check-spec-validation exits 0 with 17 warnings, none in FR-189, FR-191, FR-192, TC-624 or TC-626. tools/check-index-completeness.sh passes. quoin 0.28.3, quire 0.36.2 (engine 0.50.2).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-189-estimate-bullet | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md | examined |
| FR-189-least-favourable-bullet | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md | examined |
| FR-189-no-initial-state-bullet | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md | examined |
| FR-189-undefined-stop-bullet | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md | examined |
| FR-189-non-activated-bullet | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md | examined |
| FR-191-max-samples-timing-bullet | spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md | examined |
| FR-191-budget-stop-bullet | spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | This bullet holds two SHALLs ('SHALL continue until `max_draws` stops it' and 'SHALL never decide on zero counted samples'). Its trigger, 'a quantile claim's samples are all non-activated', is only known once the run ends. Write it as a state-driven rule, for example 'While a quantile test has counted no sample, EN-4 SHALL not decide it', and leave the `max_draws` stop to FR-191. | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:168-170 |
| FND-002 | low | The new timing bullet and the existing 'When a test would count one more sample ... than `max_samples`' bullet state one rule twice, in different words. 'apply to a draw or step in the same way' leaves `max_draws` and `max_cycle_steps` to the reader. Merge the two bullets into one event-driven statement that names the check point for each budget: before drawing the next sample, and before taking the next step in a cycle. | spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md:69-81 |
| FND-003 | low | 'for a lower bound `>= θ`, the smallest lower end minus `θ`' reads as the selection rule, not as a per-test margin. 'FR-192's `estimate` and `interval` are the least favourable test's' is a requirement with no SHALL, in another FR's terms. Define the margin per test (lower end − θ, or θ − upper end) and select the smallest. Leave FR-192's use of it to FR-192, which already says so. | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:154-161 |
| FND-004 | low | '`Some(count / samples)` (or the method's estimate, FR-190)' uses `count`, which is not defined here, and it does not fit a mean of a fraction. The example 'as for a test that rejected on an undefined first sample' is a note, not a condition. State the estimate per method and give the `None` condition as the requirement. | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:150-153 |

## Verdict

Four low findings, all wording. None blocks merge on its own. The no-initial-state and undefined-stop bullets are well-formed If/then SHALL statements. quire's grammar lint reports no warning in FR-189, FR-191, FR-192, TC-624 or TC-626, so these were found by reading the text.

## Dispositions

Round 1, head 8ab2038d3: fix commit 8ab2038d3 over merge ba8ad2332, which brings in main 70c7092b9. The fix commit also commits this review's round-0 SR files under reviews/ unchanged; this file supersedes them. Terra's #1079 branch is now at 45bd79491. Its qsl-analyze and qsl-replay sources are byte-equal to 149fdd4b7 (the new commit adds only review files), so the round-0 code reading still holds. Gates: make check-spec-validation exits 0 with the same 17 warnings, none in the changed files; tools/check-index-completeness.sh passes. Hand checks: FR-189-AC-10's seven cases were run through terra's `least_favourable` rule (severity, then the estimate on the failing side, None worst, keep the earlier on a tie), and each case names the test the code picks. FR-189-AC-8's `Undef` is undefined at trace 0, position 0, so `estimate` is None. FR-191-AC-5 still holds: the decision check comes before the `max_samples` check.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |

Round 1 verdict: All four findings are fixed. The non-activated rule is state-driven with one SHALL, the budget rule is stated once, the margin is defined per test, and the estimate is given per method. Nothing new.
