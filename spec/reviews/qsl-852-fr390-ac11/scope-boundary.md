---
id: SR-1957106
title: "Scope and boundary review of QSL-852 FR-390 changes"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/quire-spec-language; spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md"
review_set: all
---

## Summary

The reader evidence wrapper and projection remain QSL-owned, IR owns admission/projection contracts, and `quire-checked` owns shared term vocabulary. The stated responsibility split is coherent, but mint authority for the shared witness is not allocated through an API boundary that preserves the layer split.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-390 assigns witness minting to the exact layer-5 selector while placing the opaque witness in `quire-checked`, yet does not define how that selector is granted exclusive construction authority without exposing a public lower-layer mint operation. Allocate that capability explicitly while preserving the rule that layer 4 imports no layer-5 type. | FR-390 Description, FR-390-AC-16 | wrong-requirement |
