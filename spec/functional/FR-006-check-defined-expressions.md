---
id: FR-006
title: "Check types and guarded definedness"
type: FR
relationships:
  - target: "ix://agent-ix/quire-spec-language/US-002"
    type: traces_to
---
# FR-006: Check types and guarded definedness

## Description

When a linked clause is checked, the compiler shall establish Boolean root typing and definedness for every potentially evaluated subexpression.

## Inputs

Resolved declarations, clause anchor, lexical environment and snapshot-qualified facts.

## Outputs

Reference-evaluable clause or typed refusal.

## Behavior

The checker consumes the reviewed finite-state semantics. Presence facts cannot cross observations. Numeric bounds and units are explicit. Literal/size inference refuses ambiguity. Unsupported forms are retained as obligations rather than erased.

**QSL-499 disposition:** the native 0-draft carrier of the retired acceptance criteria below (`native_model`, `model_source`, `located_json`, `link_native` and the native `checking` pipeline) is deleted under ADR-011, ADR-033 and FR-499. Each retired criterion is retained as migration history and is not an active acceptance obligation; its behavior is owned by FR-104 (state-clause definedness, presence and range proofs, `ambiguous-literal`) and FR-172 (hyper and relation clauses), checked by the qsl-semantics suites tagged with those requirements.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-006-AC-1 | An unguarded optional unwrap receives undefined_expression. | Test (TC-025) |
| FR-006-AC-2 | A cross-snapshot presence proof is rejected. | Test (TC-026) |
| FR-006-AC-3 | **RETIRED in QSL-499; historical only.** A guarded in-range addition is admitted. | Test (TC-027) |
| FR-006-AC-4 | **RETIRED in QSL-499; historical only.** An ambiguous scalar inference receives ill_typed. | Test (TC-028) |
| FR-006-AC-5 | A non-Boolean clause root receives ill_typed. | Test (TC-029) |

## Dependencies

- [US-002](../usecase/US-002-link-exact-models.md) supplies the user need.
- [Detailed contract or implementation evidence](../../README.md) supplies the scoped context.
