---
id: SR-60302
title: "Failure-domain review of QSL conformance input adapters"
type: SpecReview
analysis: failure-domain
scope: "FR-350 corpus input adapters and TC-875"
review_set: all
---

## Summary

Reviewed malformed structural values, unknown enum tags, invalid limits and unsafe byte paths. FR-071 remains the owner of typed request refusal, while path confinement is refused in the corpus adapter before construction.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Unknown members/tags and byte paths have explicit refusal boundaries, and valid missing inputs stay per-vector failures. | FR-350-AC-10; FR-336-AC-7/8; TC-970 |
