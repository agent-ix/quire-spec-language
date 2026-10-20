---
id: SR-13546
title: 'Spec review of PR #1098: align FR-189, FR-191 and FR-192 with the EN-4 code, ears'
type: SpecReview
analysis: ears-conformance
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

Ticket: QSL-570. PR: quire-spec-language#1098, head a53c86bc4, scoped to `git diff refs/remotes/origin/main...a53c86bc4` (one commit, three files, +10/-4). Spec-only. It aligns FR-189, FR-191 and FR-192 with terra's EN-4 engine on PR #1079 (branch qsl-570-impl, a9a693ac0). Each claim was checked against that code. FR-189: `check_statistical` raises `TraceIndexRange` only when the method is not `Regenerative` (statistical.rs:643-656), and `regenerative_test` walks at trace index `j` (statistical.rs:451); confirmed. An `Event` sample in `sampled_test` adds 1 or 0 to the sum with no form check, and the fixed-sample method is the only one a mean of a fraction admits (statistical.rs:403-427); confirmed. FR-191: `regenerative_test` adds one to `tally.draws` per walk step and stops at `max_draws` (statistical.rs:462-472), and FR-190 Outputs records the steps taken as `draws`; confirmed. FR-192: `halted` maps every `SampleHalt::Expansion` to `StatisticalStop::Expansion(stop)`, an internal-failure stop included (statistical.rs:282-293). `ExpansionStop` carries only `cause` and `limit: Option<ExpansionLimit>` (qsl-eval/src/simulation/expansion.rs:24-38). #1079 has no statistical settlement map yet. The model-check precedent maps a named limit to a stop naming it, an unnamed internal failure to a fault, and any other unnamed stop to `ResourceExhausted` (state_graph.rs:319-336). QSpec main 2aadb7e was read for the cause vocabulary: FR-331's closed `incomplete` causes are `limit-reached`, `timed-out`, `precision-budget` and `cancelled{source}`, and FR-408 settles every statistical stop `limit-reached` or `cancelled`. Methods: spec-review/base (SR-13545), spec-review/ears (SR-13546), spec-review/integrity (SR-13547). code-review and gap-analysis were not run because no code or test file changed. Gates on the head: make check-spec-validation exits 0 with 17 warnings, none in FR-189, FR-191 or FR-192. The author had not run it. tools/check-index-completeness.sh passes. No status wording in the diff. quoin 0.28.3, quire 0.36.2 (engine 0.50.2). This review: EARS conformance of the changed statements.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-189-mean-of-fraction-bullet | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md | examined |
| FR-189-refusals | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md | examined |
| FR-191-limits-struct | spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md | examined |
| FR-191-budget-bullet | spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md | examined |
| FR-192-expansion-limit-row | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md | examined |
| FR-192-expansion-unnamed-row | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new sentence makes the sampler's sample the subject of a passive 'SHALL be read', so it names no system that does the reading. It then joins a second obligation with 'and EN-4 SHALL not refuse it', which makes it two requirements in one sentence, and writes 'SHALL not' where FR-189 elsewhere writes 'SHALL NOT'. Fix: 'If the sampler returns an event sample for a mean-of-fraction claim, then EN-4 SHALL count it as the fraction 1 when the event holds and 0 when it does not.' The no-refusal clause then follows from the positive rule, and can be dropped or kept as its own sentence. | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:152-155 |

## Verdict

One low finding. The FR-191 budget sentence keeps its EARS event form ('When a test has drawn ... it SHALL stop'). The FR-189 TraceIndexRange text is a definition list, not a SHALL statement. The FR-192 rows sit under the existing exhaustive-map SHALL. The quire grammar lint reports nothing in the three files.
