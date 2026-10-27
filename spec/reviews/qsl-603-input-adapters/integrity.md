---
id: SR-60303
title: "Integrity review of QSL conformance input adapters"
type: SpecReview
analysis: integrity
scope: "FR-350 and TC-875"
review_set: all
---

## Summary

Checked the replay mapping for complete field coverage: all 15 ReplayRequestWire members are listed, FR-323 members remain distinct from the RequestTargetWire wrapper, and structural option/enum forms are closed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The mapping distinguishes FR-323 shapes, structural Rust types, and the vector-relative byte path; no request field is silently omitted. | FR-350-AC-10; FR-336-AC-7/8; TC-970 |
