---
id: SR-12920
title: 'Dependency review of STD-191 (PR #1004): new references in FR-230 and FR-393'
type: SpecReview
analysis: dependency
scope: 'agent-ix/quire-spec-language@b9c479125; spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md; spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md; spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md; spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md; spec/test-cases/TC-620-s3-admits-random-parameters-workloads-and-rewards.md; spec/test-cases/TC-621-s3-checks-probabilistic-claim-forms.md; spec/test-cases/TC-685-s3-checks-time-declarations-and-clock-constraints.md; spec/test-cases/TC-1078-s2-builds-time-declaration-quantity-and-timed-interval-forms.md; context: spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md, spec/functional/FR-257-build-forms-at-any-depth.md, spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md, spec/functional/FR-277-bound-every-lifecycle-operation-by-caller-limits.md, spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md; agent-ix/quire-specification@ad2b37e spec/functional/type-model/FR-142; agent-ix/filament-core-data@e777d8d spec/functional/FR-027'
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-393
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-230
  type: reviews
---
# SR-12920: dependency review of STD-191 (PR #1004)

## Summary

Ticket: STD-191. This review checks that the dependencies the diff adds in prose are recorded as `relationships` edges, and that enablement (the filament-core-data change) is kept apart from feature work.

Units examined (each listed in the Linear comment's `scope` with its text): FR-185, FR-185-AC-2, FR-186-AC-4, FR-230, FR-230-AC-1, FR-230-AC-2, FR-230-AC-5, FR-393, FR-393-AC-8, TC-1078, TC-620, TC-621, TC-685. Context only: none.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-230's Dependencies now cite FR-088, and its body cites FR-091, but its frontmatter has no edge to either. FR-393's new bullets depend on FR-185 (`s3.rational_bits`) and FR-255 (`s1.input_bytes`), and its frontmatter has neither. The link graph, and so blast-radius queries, miss these dependencies. Fix: add `depends_on` edges for FR-230 to FR-088 and FR-091, and for FR-393 to FR-185 and FR-255. | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:1-22; spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md:1-22 |

## Verdict

One low finding. The enablement split is covered by base FND-002 and FND-004.
