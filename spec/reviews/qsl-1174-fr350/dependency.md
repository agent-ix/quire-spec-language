---
id: SR-117403
title: "Dependency review of the QSL-1174 FR-350 corpus-runner batch"
type: SpecReview
analysis: dependency
scope: "spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md; spec/test-cases/TC-875-the-runner-executes-and-compares-vectors-through-public-entry-points.md"
review_set: subset
---
## Summary

Checked the ownership boundary between QSpec's corpus-v1 contract and QSL's qualification runner, with the referenced QSL public operations remaining the implementation dependencies.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No dependency ownership issue found: FR-336 owns corpus and adapter encodings, while FR-350 owns QSL's selection, dispatch and settlement behavior. | FR-336; FR-350 |
