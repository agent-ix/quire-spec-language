---
id: SR-1957101
title: "Failure-domain review of QSL-852 FR-390 changes"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language; spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md"
review_set: all
---

## Summary

The new reader identity path avoids bytes-derived identity and preserves staged I2 admission. The package-wide missing-row failure does not identify how to populate the required affected node and location in `UnsupportedView`, leaving diagnostics dependent on an unstated selection rule.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | `MissingIdentityWitness` is caused by absent package-level producer rows, while `UnsupportedView` requires an affected `NodeKey` and `Location`; the requirement does not define which protocol node/locus represents that absence. Define a deterministic package-level locus (or a distinct package-level reason shape) and verify it. | FR-390 Description, FR-390-AC-17 | wrong-requirement |
