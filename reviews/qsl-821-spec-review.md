---
id: SR-12093
title: "QSL-821 PR #942 spec review: FR-342-AC-3 and TC-862 type-admission ceiling amendment"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@ab50faf64693b520352b2dcac6627ccd12f3fc04; spec/functional/FR-342-classify-a-clause-run-disposition-for-a-refinement-gate.md, spec/test-cases/TC-862-classify-clause-run-dispositions.md; FR-342-AC-3"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-342
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-862
    type: reviews
---
# Spec review of the FR-342-AC-3 / TC-862 amendment

## Summary

Ticket: QSL-821. PR quire-spec-language#942. The amendment adds one clause to FR-342-AC-3: "a run whose compile reaches a type-admission ceiling classifies `incomplete` at `compile`, naming that ceiling and its value". TC-862 step 3 and its step-3 expected result gain the matching clause.

Checked against the rest of FR-342 and FR-341:

- The FR-342 Behavior table maps `Compile(r)` to FR-341's class of `r`. FR-341 rule 4 makes a `stage_limit_exceeded` cause `incomplete`, and its Outputs name the limit for each such cause that carries one. The new clause follows from both.
- The Outputs say "an `incomplete` class also carries the limit reached and its value when the disposition names one". This is consistent.
- The new clause can fail. A classifier that dropped the limit, or a stage other than `compile`, fails it.
- The TC-862 step, "build a `Compile` disposition holding a check refusal whose type admission reached a ceiling", matches what the TC-862/FR-342-AC-3 test does. Its expected result, "`incomplete`/`compile` naming the ceiling's setting and bound", matches the assertions.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

PASS. The amendment agrees with the rest of FR-342 and FR-341, and the trace tags cite only ACs the tests fully check. A wording note that is not a major defect: the FR-342 Behavior bullet "(a model limit or FR-100's `work_units`)" reads as a closed list, and a type-admission ceiling (`environment.*`) is neither. It could say "for example". A compile-stage limit is FR-341's class, so the AC itself is not contradicted.
