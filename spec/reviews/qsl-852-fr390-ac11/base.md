---
id: SR-1957100
title: "Base checklist review of QSL-852 FR-390 changes"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md"
review_set: all
---

## Summary

The FR-390 diff keeps its IDs and verification methods and adds no hash, digest, or pin requirement. AC-14 still does not make the exclusive witness-minting path verifiable: its compile-fail cases cover view/handle construction from forbidden inputs, but not direct witness construction or an alternate mint path.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | AC-14 does not require evidence that only the exact layer-5 slot selector can mint the C-layer witness; its compile-fail assertions cover forbidden view/handle inputs but not witness forgery or alternate mint paths. Add an explicit API-surface/compile-fail assertion for witness construction and use. | FR-390-AC-14, FR-390-AC-16 | correct-requirement-no-evidence |
