---
id: SR-1957104
title: "Evidence review of QSL-852 FR-390 changes"
type: SpecReview
analysis: evidence
scope: "agent-ix/quire-spec-language; spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md"
review_set: all
---

## Summary

AC-14 and AC-16 cover the view boundary and layer placement, while AC-17 adds a missing-row case. The criteria do not directly verify that the exact slot selector is the only path that can mint the owner-bound witness, which is the type-authority property the prose relies on.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | No criterion requires a compile-fail or public-API check that attempts to create/forge the C-layer selection witness outside exact slot selection; the existing forbidden-input compile-fail list tests view/handle constructors only. Add evidence for exclusive mint authority. | FR-390-AC-14, FR-390-AC-16 | correct-requirement-no-evidence |
