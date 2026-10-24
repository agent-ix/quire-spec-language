---
id: SR-16368
title: "Evidence review of the QSL-932/933/934 RA weak-memory spec batch"
type: SpecReview
analysis: evidence
scope: "FR-224, FR-225, FR-227, FR-228, FR-338; TC-669, TC-670, TC-672, TC-673, TC-1230"
review_set: subset
---

## Summary

Each targeted behavior points to a test case, but the scoped source matrix shows that several criteria have no current trace-tag binding. The code PRs must add the listed tags alongside their tests.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The matrix reports `untagged` for FR-224-AC-1/2/4/5/7, FR-225-AC-3, FR-227-AC-3/4, FR-228-AC-1 and FR-338-AC-21. Add each criterion's direct trace tag to its implementing test in QSL-932/933/934 code PRs. FR-225-AC-1 is already tagged. | FR-224-AC-1, FR-224-AC-2, FR-224-AC-4, FR-224-AC-5, FR-224-AC-7, FR-225-AC-1, FR-225-AC-3, FR-227-AC-3, FR-227-AC-4, FR-228-AC-1, FR-338-AC-21 | correct-requirement-no-evidence |
