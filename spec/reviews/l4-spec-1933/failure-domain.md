---
id: SR-1933003
title: "Failure-domain review for FR-493 retirement ordering"
type: SpecReview
analysis: failure-domain
scope: "Deletion ordering and orphaned trace tags in FR-493"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-493
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1276
    type: reviews
---

## Summary

The main risks are ordering and orphaned evidence. A path whose last caller is still live cannot be deleted, and a retired requirement leaves trace tags and citing documents behind until the code change removes them.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-493 requires a path to be deleted only after its replacement works and in the change that makes its last caller unreachable. The protocol_artifact read side is imported by the state and temporal modules and by the composed lane, so its deletion cannot precede theirs; the code ticket must sequence it after the others rather than treat it as independent. | FR-493-AC-4 |
| FND-002 | low | Between this specification and the code change, tests tagged with the retired requirement ids still pass and the ids still exist. FR-493-AC-5 and TC-1276 step 5 close the gap only after the code change lands; no earlier gate checks it. | FR-493-AC-5, TC-1276 |
