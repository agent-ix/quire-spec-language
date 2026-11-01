---
id: SR-1957105
title: "Risk and complexity review of QSL-852 FR-390 changes"
type: SpecReview
analysis: risk-complexity
scope: "agent-ix/quire-spec-language; spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md"
review_set: all
---

## Summary

The added witness bridges a shared lower-layer type and a layer-5 selector while enforcing owner identity. That boundary has elevated implementation risk because the spec requires exclusive selector minting without defining a capability/factory boundary or a verification probe that demonstrates it.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | A public API in `quire-checked` must let the selector obtain the witness, but the spec gives no concrete authority mechanism that prevents other callers from minting one; a publicly callable factory can defeat the stated non-forgeability. Specify a sealed/capability boundary consistent with the crate DAG and verify callers cannot bypass it. | FR-390 Description, FR-390-AC-14, FR-390-AC-16 | wrong-requirement |
