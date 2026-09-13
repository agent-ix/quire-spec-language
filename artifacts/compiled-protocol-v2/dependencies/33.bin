---
id: FR-038
title: "Resolve predicate names under explicit lexical scopes"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-005
    type: implements
---
## Description

When resolving native predicate declarations and local bindings, the linker SHALL apply the selected grammar's explicit alias, declaration, parameter, let and quantifier scopes without ambiguous name selection.

## Inputs

Collected source-unit imports/profile aliases, linked-package declarations, predicate signatures and located call/local-name uses.

## Outputs

Each use resolved to one declaration and scope, or a located duplicate, shadowing, unresolved or ambiguous-name refusal.

## Behavior

Aliases are source-unit scoped; native predicate declaration names are unique in the linked package. A parameter or local binding cannot shadow another visible binding, alias or native declaration. Different predicates may reuse a parameter name. A let initializer cannot refer to the binding it is introducing; a quantifier variable is not visible in its domain. Forward calls resolve after collecting declarations and remain subject to FR-033's cycle/type checks. Named predicate bodies use explicit parameters: implicit self/result/pre-state reads refuse. Callers may pass correctly anchored values. No overload or case-folded fallback selects a declaration.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-038-AC-1 | Two units may use the same local alias spelling for different exact imports without cross-binding; a duplicate native predicate name in the package refuses. | Test (TC-038) |
| FR-038-AC-2 | Duplicate parameters and nested shadowing refuse, while distinct predicates may each use a parameter named x. | Test (TC-038) |
| FR-038-AC-3 | A let initializer and quantifier domain cannot read their newly introduced binder; uses in the respective body resolve correctly. | Test (TC-038) |
| FR-038-AC-4 | A forward acyclic call resolves to one declared predicate; unresolved, ambiguous and case-mismatched references refuse. | Test (TC-038) |
| FR-038-AC-5 | Implicit self/result/pre reads inside a named predicate refuse; explicitly passed caller-anchored values retain their binding. | Test (TC-038) |

## Dependencies

- [Proposed shared grammar](../../proposals/quire-v1/shared-grammar.md).
- [Shared drafting foundation](../../proposals/quire-v1/shared-foundation.md).
- [Planned matrix](../composed-foundation/tests.md).

This contribution remains proposed until the integrated baseline review. It does
not change historical definition bytes or claim current compiler support.
