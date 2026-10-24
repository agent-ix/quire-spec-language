---
id: SR-16365
title: "Base review of the QSL-932/933/934 RA weak-memory spec batch"
type: SpecReview
analysis: base
scope: "FR-215, FR-224, FR-225, FR-227, FR-228, FR-338; TC-669, TC-670, TC-672, TC-673, TC-1230"
review_set: subset
---

## Summary

The requested RA requirements are explicit in the FRs and test cases. The scoped matrix still has untagged criteria awaiting their assigned code work, recorded in the evidence review.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The scoped matrix has no source trace tags for the pending RA race, race-summary, RMW, visibility-lasso, RA-bound and certificate-bound criteria. The linked test cases define the procedures; add the criterion tags with the corresponding code tests. | FR-224-AC-1, FR-224-AC-2, FR-224-AC-4, FR-224-AC-5, FR-224-AC-7, FR-225-AC-3, FR-227-AC-3, FR-227-AC-4, FR-228-AC-1, FR-338-AC-21 | correct-requirement-no-evidence |
