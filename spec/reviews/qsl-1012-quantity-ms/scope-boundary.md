---
id: SR-16632
title: "QSL-1012 scope and boundary review"
type: SpecReview
analysis: scope-boundary
scope: "QSL intake/assembly, qsl-eval identity encoding and qsl-analyze statistical measurement"
review_set: all
---

## Summary

Responsibility stays at the existing boundaries: intake admits the unresolved `ix://quire/unit/<name>` reference; S3 resolves it against units declared by the compiling QSL unit; qsl-eval encodes the exact quantity value; qsl-analyze uses the checked reward type and exact magnitude. No conversion or unit policy is duplicated in the statistical layer.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The change keeps unit resolution, canonical value encoding and statistical interpretation within their owning layers. | FR-056-AC-42; FR-120-AC-16; FR-189-AC-15 |
