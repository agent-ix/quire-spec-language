---
id: SR-1938002
title: "gap analysis of PR #1938 (QSL-1338 slice c): FR-190 regenerative stop and FR-242 lasso tests"
type: SpecReview
analysis: gap-analysis
date: 2026-10-31
scope: "agent-ix/quire-spec-language; PR #1938 git diff origin/main...8458dd8dd (FR-190-AC-1, FR-190-AC-2, FR-242-AC-1, TC-625, TC-697; qsl-analyze statistical tests; qsl-eval/tests/it/timed_trace.rs)"
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-190
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-242
  type: reviews
---
# SR-1938002: gap analysis of PR #1938 (QSL-1338 slice c)

## Summary

Ticket: QSL-1338. Plan completion: not assessed. The criterion-to-test check was done by hand over the diff with a grep of `#[trace]` tags, because the PR changes no spec and adds no criterion. FR-190-AC-1 is tagged by `long_run_is_accepted_by_regeneration_with_asymptotic_coverage` (TC-625), which recomputes the half-width from the recorded cycles with its own helper and asserts that the run stops at the first n >= 100 where the half-width is at most 1/5000. That is exactly the stop point the new screen could have moved. It passes at the PR head. The repeated-interval timed test is tagged `TC-697`/`FR-242`, but its lasso counts are not stated by any FR-242 AC. FR-242-AC-1 names four other formulas on 1,000 lassos, and those are untouched.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-190-AC-1 | spec/functional/FR-190-measure-a-long-run-fraction-by-regeneration.md:81 | examined |
| FR-190-AC-2 | spec/functional/FR-190-measure-a-long-run-fraction-by-regeneration.md:82 | examined |
| FR-242-AC-1 | spec/functional/FR-242-translate-a-timed-formula-into-a-claim-automaton.md:69 | examined |
| TC-697 step 1 | spec/test-cases/TC-697-timed-formulas-translate-to-claim-automata.md:21 | context_only |
| long_run_is_accepted_by_regeneration_with_asymptotic_coverage | qsl-analyze/src/statistical/tests.rs:874-920 | examined |
| the_repeated_interval_forms_agree_with_the_evaluator | qsl-eval/tests/it/timed_trace.rs:1097-1137 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

**Clean. No gaps and no orphan code.** The new `interval_within` is the FR-190 half-width rule under a cheaper rejection, and FR-190-AC-1's stop-point assertion covers it. `Tally::held` belongs to FR-189's estimate, and the Okamoto and SPRT tests in `statistical::tests` cover it (54/54 pass). The lasso-count reduction is in a test whose counts no criterion fixes.
