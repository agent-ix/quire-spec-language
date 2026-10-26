---
id: SR-20114
title: "Evidence analysis of QSL-1105 and QSL-1107"
type: SpecReview
analysis: evidence
scope: "New FR-401-AC-12/13 and FR-485-AC-13/14/15"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-401
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-485
    type: reviews
---

## Summary

The catalog advisor returns actionable test methods for all five new criteria, with no mismatch or inconclusive result among those criteria. FR-401-AC-13 and FR-485-AC-15 require public-consumer tests that exercise actual SV reservation points; those tests remain dependent on the external QSV seam. The full-repository matrix did not finish within the available run window, so this review makes no matrix-coverage claim. The two advisor mismatches and one inconclusive result in the 28-criterion target set are on pre-existing FR-401-AC-3/6 and FR-485-AC-8, outside the changed criteria.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The new criteria have planned test evidence but no implementation binders yet; code work must add public-consumer tests for owner/member identity and each body-side failure, including genuine reservation failures once the SV seam is available. | FR-401-AC-12, FR-401-AC-13, FR-485-AC-13 through AC-15 | correct-requirement-no-evidence |
