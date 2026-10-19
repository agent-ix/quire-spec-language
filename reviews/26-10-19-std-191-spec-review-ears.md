---
id: SR-12917
title: 'EARS review of STD-191 (PR #1004): FR-393 S2 decimal scale and FR-230 time dimension statements'
type: SpecReview
analysis: ears-conformance
scope: 'agent-ix/quire-spec-language@b9c479125; spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md; spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md; spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md; spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md; spec/test-cases/TC-620-s3-admits-random-parameters-workloads-and-rewards.md; spec/test-cases/TC-621-s3-checks-probabilistic-claim-forms.md; spec/test-cases/TC-685-s3-checks-time-declarations-and-clock-constraints.md; spec/test-cases/TC-1078-s2-builds-time-declaration-quantity-and-timed-interval-forms.md; context: spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md, spec/functional/FR-257-build-forms-at-any-depth.md, spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md, spec/functional/FR-277-bound-every-lifecycle-operation-by-caller-limits.md, spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md; agent-ix/quire-specification@ad2b37e spec/functional/type-model/FR-142; agent-ix/filament-core-data@e777d8d spec/functional/FR-027'
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-393
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-230
  type: reviews
---
# SR-12917: ears-conformance review of STD-191 (PR #1004)

## Summary

Ticket: STD-191. This review checks the new and edited requirement statements in FR-393 (the decimal-scale bullets) and FR-230 (the time dimension, lookup, refusal and Clock-field bullets) against EARS patterns. The refusal bullets are well-formed unwanted-behaviour statements ('If ..., the checker SHALL refuse ...'). Two statements are not.

Units examined (each listed in the Linear comment's `scope` with its text): FR-185, FR-185-AC-2, FR-186-AC-4, FR-230, FR-230-AC-1, FR-230-AC-2, FR-230-AC-5, FR-393, FR-393-AC-8, TC-1078, TC-620, TC-621, TC-685. Context only: none.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | 'A domain package that uses `time dense unit ms` declares `s` as the zero-offset root of the time dimension and `ms` as a unit whose edge to `s` has scale `1/1000`' is an indicative sentence about another component, with no SHALL and no 'for example'. Implementers can read it in two ways: as a requirement that the time root be `s` and that `ms` target `s` directly, or as an illustration. FR-142 admits any root and any edge chain. Fix: mark it 'for example', or state the actual constraint (any zero-offset root; `ms` composes to `1/1000` of `s`). | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:103-108 |
| FND-002 | low | 'in a unit of about 60 bytes' is not a precise condition, and 'naming ... the source length' gives no value to assert. Fix: give the fixture or its exact byte length, so the expected bound is a number. | spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md:186 |

## Verdict

Two findings, both wording. FND-002 overlaps base FND-001 and is resolved by the same fix.
