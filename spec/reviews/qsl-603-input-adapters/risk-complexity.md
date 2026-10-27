---
id: SR-60306
title: "Risk-complexity review of QSL conformance input adapters"
type: SpecReview
analysis: risk-complexity
scope: "FR-350-AC-8 through FR-350-AC-10"
review_set: all
---

## Summary

The larger replay mapping creates conversion and identity risks. Mitigation is the exhaustive 15-field table, closed tagged schema, FR-255 decoder, digest validation and TC-970 path/mutation cases.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The replay implementation should exercise each structural variant and path/digest failure before broad corpus execution. | FR-350-AC-10; FR-336-AC-7/8; TC-970 |
