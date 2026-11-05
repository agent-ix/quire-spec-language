---
id: SR-20261105101008
title: "Specification integrity review of QSL-1602 PR #2371"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md (edited refusal-location rule; AC-50, AC-55, AC-72)"
review_set: subset
---

## Summary

The edited clarification removes ambiguity from the general package-node refusal rule, but one linked AC retains the earlier location phrasing. The resulting cross-statement inconsistency is limited to AC-55's base-dimension refusal location.

## Verdict

CONDITIONAL: synchronize AC-55 with the changed rule so the spec has one interpretation of the reported location.

## Scope

- `FR-091-AC-55` (examined): `and U11z1 refuse the base-dimension rule at P's L, missing_declaration/missing-name, retaining L and m2, located at P's model_root node and naming L's FCD identity.`
- `FR-091-AC-72` (context_only): `Package dimension and package unit forms (QSL-1602; QSpec FR-142-AC-18, AC-19, AC-20).`
- Edited refusal-location rule (examined): `A refusal of the package node itself (row 4 beside the package's root, row 6 over a cycle of local units, the base-dimension rule) is located at the model selection that selects its package, whose model_root node records it, and names the node's FCD identity.`
- `qsl-semantics/tests/it/dimension_equivalence.rs` (examined): `assert_eq!(at, selection);`

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The general rule now locates package-node refusals at the selecting `model` statement, while AC-55 locates the base-dimension refusal at the package `model_root` node; reconcile these locations. | FR-091-AC-55; FR-091-AC-72 | wrong-requirement |
