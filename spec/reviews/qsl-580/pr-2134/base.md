---
id: SR-2026110215403
title: "QSL-580 PR 2134 base review"
type: SpecReview
analysis: base
scope: "FR-230 mixed-bound refusal, FR-230-AC-57, TC-685 step 38"
review_set: subset
---
# SR-2026110215403: QSL-580 PR 2134 base review

## Summary

The change gives the mixed source form an explicit S2-then-S3 outcome: parse and retain source order, then refuse at the invariant without emitting partial timed nodes. The criterion names the observable refusal and links it to the owning FR-393 source-form criterion. No applicable AssuranceProfile document was found in the repository scope; the assurance documents present are measurement/assurance artifacts rather than review-policy profiles.

The repository-wide matrix did not return within 35 seconds, so no tag-coverage counts are claimed. Scoped `quire validate` completed with existing unrelated warnings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No defect found in the scoped analysis. | Changed FR-230/TC-685 scope |
