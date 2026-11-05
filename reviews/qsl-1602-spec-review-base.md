---
id: SR-20261105101007
title: "Base specification review of QSL-1602 PR #2371"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md (edited refusal-location rule; AC-50, AC-55, AC-72)"
review_set: subset
---

## Summary

The changed refusal-location rule is consistent with the new tests for package-node refusals. AC-55 still describes package base-dimension refusals as located at the package `model_root`, while the clarified rule and tests place the source span at the selecting `model` declaration.

## Verdict

CONDITIONAL: clarify AC-55's location wording to match the now-explicit selection-span rule and its test oracle.

## Scope

- `FR-091-AC-50` (context_only): `the assembler admits every form with no unresolved-name and no duplicate error: the source Time and R::Time are two dimensions and the source s and R::s two units`
- `FR-091-AC-55` (examined): `and U11z1 refuse the base-dimension rule at P's L, missing_declaration/missing-name, retaining L and m2, located at P's model_root node and naming L's FCD identity.`
- `FR-091-AC-72` (context_only): `Package dimension and package unit forms (QSL-1602; QSpec FR-142-AC-18, AC-19, AC-20).`
- Edited refusal-location rule (examined): `A refusal of the package node itself (row 4 beside the package's root, row 6 over a cycle of local units, the base-dimension rule) is located at the model selection that selects its package, whose model_root node records it, and names the node's FCD identity.`

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | AC-55 says the base-dimension refusal is located at P's `model_root` node, but the edited rule says the refusal is located at the source `model` selection and the new tests assert that selection text; the two descriptions leave the required source location ambiguous. | FR-091-AC-55; FR-091-AC-72 | wrong-requirement |
