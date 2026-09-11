---
id: FR-037
title: "Parse shared expressions with explicit precedence"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-005
    type: implements
---
## Description

When parsing a supported native expression form, the parser SHALL construct the expression structure prescribed by the selected edition's shared grammar without deriving syntax from backend support.

## Inputs

Located tokens, selected grammar edition, a declared expression entry point and parsing resource limits.

## Outputs

A complete located syntax tree for the selected entry point, or a typed syntax/resource refusal.

## Behavior

The proposed common grammar defines all three native families, predicate declarations and shared expression precedence; balanced unknown bodies are not productions. Preserve grouping and each call/field/operator locus. Require EOF for a complete unit. Grammar recognition is followed by separate name/type/profile admission. A recognized div/rem/mod/slash form does not enable prohibited integer arithmetic. Rational slash requires the exact typing, nonzero and normalized-range rules of FR-044; it is not literal punctuation.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-037-AC-1 | Implication associates right, addition/product associate left, multiplication binds above addition, and field access binds above unary operators. | Test (TC-037) |
| FR-037-AC-2 | A comparison chain a < b < c refuses; explicit parentheses determine a tree that still requires type checking. | Test (TC-037) |
| FR-037-AC-3 | Nested if/let forms, zero-argument calls and quantified expressions consume the intended body and scope boundaries without token-text rewriting. | Test (TC-037) |
| FR-037-AC-4 | Trailing tokens, mismatched delimiters and an undefined family production refuse syntax rather than returning a successfully parsed prefix. | Test (TC-037) |
| FR-037-AC-5 | A recognized but prohibited arithmetic operator retains its source tree for a separate profile refusal instead of enabling a backend operation. | Test (TC-037) |
| FR-037-AC-6 | Qualified model/member names such as view.exhausted, M::result and M::Payment::retry retain their exact spelling even when the member is a keyword; the same keyword in a native binder position remains reserved. | Test (TC-037) |

## Dependencies

- [Proposed shared grammar](../../proposals/quire-v1/shared-grammar.md).
- [Shared drafting foundation](../../proposals/quire-v1/shared-foundation.md).
- [Planned matrix](../composed-foundation/tests.md).

This contribution remains proposed until the integrated baseline review. It does
not change historical definition bytes or claim current compiler support.
