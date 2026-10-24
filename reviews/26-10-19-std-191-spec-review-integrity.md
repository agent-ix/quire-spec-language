---
id: SR-12919
title: 'Integrity review of STD-191 (PR #1004): catalog and limit tables for the new S2 cause'
type: SpecReview
analysis: integrity
scope: 'agent-ix/quire-spec-language@b9c479125; spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md; spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md; spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md; spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md; spec/test-cases/TC-620-s3-admits-random-parameters-workloads-and-rewards.md; spec/test-cases/TC-621-s3-checks-probabilistic-claim-forms.md; spec/test-cases/TC-685-s3-checks-time-declarations-and-clock-constraints.md; spec/test-cases/TC-1078-s2-builds-time-declaration-quantity-and-timed-interval-forms.md; context: spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md, spec/functional/FR-257-build-forms-at-any-depth.md, spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md, spec/functional/FR-277-bound-every-lifecycle-operation-by-caller-limits.md, spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md; agent-ix/quire-specification@ad2b37e spec/functional/type-model/FR-142; agent-ix/filament-core-data@e777d8d spec/functional/FR-027'
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-393
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-230
  type: reviews
---
# SR-12919: integrity review of STD-191 (PR #1004)

## Summary

Ticket: STD-191. This review checks whether the diff keeps the exhaustive tables and cross-document statements it touches complete and consistent: FR-091's Catalog codes, FR-255's setting table and its S2 statement, and the FR-185/FR-186/TC-620/TC-621 vector set.

Units examined (each listed in the Linear comment's `scope` with its text): FR-185, FR-185-AC-2, FR-186-AC-4, FR-393, FR-393-AC-8, TC-1078, TC-620, TC-621. Context only: FR-091, FR-096, FR-255, FR-257, FR-277.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-393 adds `FormsCause::DecimalScaleBeyondSource` with catalog code `stage_limit_exceeded`. FR-091's Catalog codes table, which FR-091-AC-21 requires to be exhaustive over every S2 cause, has no row for it. FR-096's `stage_limit_exceeded` causes are its seven `LimitKind` causes, and none of them is named here. FR-255's setting table and its line 'S2 has no limit of its own (FR-257)' are not amended. Fix: add the cause to FR-091's table with the code chosen under base FND-001, and amend FR-255, FR-257 and FR-096 if it stays a limit. | spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md:156-160; spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:842-870 |

## Verdict

One finding. The FR-185/186 and TC-620/621 vector sets are consistent with one another. The new cause is missing from the exhaustive tables it belongs in.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | low | FR-230 now gives source-declared units a use in source: `time dense unit u`, `time tick ... unit u`, and quantity literals such as `3 ms` resolve against them. FR-091's 'a source-declared unit has no use in source' is now stale. Fix: amend that sentence to point to FR-230's time-unit lookup. | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:474-476 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The new cause is gone from the spec; FR-091's table needs no row. PR #1003 must drop `FormsCause::DecimalScaleBeyondSource`. |
| FND-002 | fixed | Round 2 at db8ec3662: FR-091 amended to point at FR-230's use. |
