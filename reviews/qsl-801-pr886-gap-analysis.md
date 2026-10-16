---
id: SR-11683
title: "Gap analysis — QSL-801 FR-363 non-Reference receiver in a function body (PR 886)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@498034d96d4cb44a39a6baabf29ca3a4b0ca7008; FR-363-AC-1, FR-363-AC-5, TC-1031 N13/N14, qsl-semantics/src/check/check.rs, qsl-semantics/tests/it/relationship_navigation.rs"
review_set: subset
---

## Summary

Ticket: QSL-801. A planless gap analysis scoped to the PR diff. Plan completion: not assessed. This was a manual check of acceptance criteria against tests: no build and no matrix run.

## Verdict

**PASS.** FR-363-AC-5's non-Reference receiver rule now has function-body evidence: N13 covers an `Option` and a `Set` of references, and `n13_a_non_reference_receiver_in_a_function_body_is_a_type_mismatch` asserts code, cause, the operand path `[0,0]`, the `StateModel` family and the catalog prefix. The ticket's outcome, that the function-body context follows FR-363 exactly, is met. The code change is the context-free `is_model_receiver` arms, and FR-363 now owns it. Population in a function body has no vector. That is acceptable, because the AC names no population case separately and the arm is a constant `true`. TC-1031 N01 to N12 are untouched and still hold. No stub, no tautology.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
