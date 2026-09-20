---
id: TC-169
title: "value::expression::syntax moves to forms with no duplicate definition"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-067
    type: verifies
---
# TC-169: value::expression::syntax moves to forms with no duplicate definition

## Description

Verify that after this requirement's implementation, `value::expression::syntax`
no longer exists as a module, and that the `Expression` enum,
`FunctionDeclaration`, `BinaryOperator`, `ClauseKind`, `DeclaredClauseKind`,
`FieldInitializer`, `BinderQuery` and `Accumulation` are each defined exactly
once, under `forms`, with no second, independent definition left behind
under `value::expression`. `value::expression`'s own code importing these
types from `forms` is expected and does not itself fail this test; a second
definition does. Scope: FR-067-AC-9.

## Test Procedure

1. Search the compiled crate's module tree for `value::expression::syntax`.
2. For each of the eight named types, search the compiled crate's type
   definitions (not references or imports) for every location that defines
   that type, and record the module path of each defining location found.
3. Inspect `value::expression`'s `check`, `evaluate`, `facts`, `ir`,
   `refusal` and `termination` modules for their use of the eight named
   types, distinguishing an import from `forms` from an independent
   re-definition.

## Expected Results

- Step 1: `value::expression::syntax` is absent from the module tree.
- Step 2: each of the eight named types has exactly one defining location,
  under `forms`; no type has a second defining location under
  `value::expression` or anywhere else.
- Step 3: every use found is an import from `forms`; none is an independent
  definition.
