---
id: SR-1933004
title: "Integrity review for FR-493 and the QSL-1255 test cases"
type: SpecReview
analysis: integrity
scope: "Completeness, consistency and atomicity of the changed files"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-493
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-137
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-136
    type: reviews
  - target: ix://agent-ix/quire-spec-language/NFR-009
    type: reviews
---

## Summary

Citations were checked against the live tree. FR-365 now cites FR-049-AC-7, which states the exact and one-short budget rule; FR-008-AC-16 is no longer cited.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-493-AC-4 bundles three claims (import absence, document absence, round trip); the document-absence claim was removed with the scan fix, leaving two. | FR-493-AC-4 |
| FND-002 | medium | The FR-493 retired table says FR-028 is replaced by FR-109 and FR-100 while FR-100 depends on FR-028; this is inconsistent until the `package` member amendment is applied. | FR-493, FR-100 |
| FND-003 | low | TC-136 H1 to H4 map to FR-049-AC-1; TC-137 scope now lists NFR-009-AC-3 and the relationship to NFR-009 already exists. | TC-136, TC-137 |
