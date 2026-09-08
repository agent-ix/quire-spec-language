---
id: FR-006
title: "Check types and guarded definedness"
type: FR
relationships:
  - target: "ix://agent-ix/quire-spec-language/US-002"
    type: implements
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

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-006-AC-1 | An unguarded optional unwrap receives undefined_expression. | Test |
| FR-006-AC-2 | A cross-snapshot presence proof is rejected. | Test |
| FR-006-AC-3 | A guarded in-range addition is admitted. | Test |
| FR-006-AC-4 | An ambiguous scalar inference receives ill_typed. | Test |
| FR-006-AC-5 | A non-Boolean clause root receives ill_typed. | Test |

## Dependencies

- [US-002](../usecase/US-002-link-exact-models.md) supplies the user need.
- [Detailed contract or implementation evidence](../../README.md) supplies the scoped context.

## Status

Draft. Specification review and prerequisite acceptance remain distinct from existing code/tests. No acceptance criterion is claimed satisfied solely because this artifact has been authored.
