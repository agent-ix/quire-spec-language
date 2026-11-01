---
id: SR-1957102
title: "Integrity review of QSL-852 FR-390 changes"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md"
review_set: all
---

## Summary

The change is internally consistent about retaining authentic rows, joining them only after successful I2 admission, and preserving the no-argument reader entry. The new missing-row variant conflicts with the existing node-located `UnsupportedView` contract unless its attribution is specified.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The new `MissingIdentityWitness` refusal is package-scoped, but the closed `UnsupportedView` payload must carry an affected node and source location. Neither the behavior nor AC-17 specifies a canonical node/location when all producer rows are absent, so the contract has multiple valid diagnostic results. Specify the locus and its ordering. | FR-390 Description, FR-390-AC-17 | wrong-requirement |
