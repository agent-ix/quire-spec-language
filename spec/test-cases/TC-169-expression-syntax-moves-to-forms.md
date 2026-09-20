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
no longer exists as a module, that the `Expression` enum,
`FunctionDeclaration`, `BinaryOperator`, `ClauseKind`, `DeclaredClauseKind`,
`FieldInitializer`, `BinderQuery` and `Accumulation` are each defined exactly
once, under `forms`, with no second, independent definition left behind
under `value::expression` or anywhere else, and that the move changed each
type's defining module only — no variant, field, method signature, or their
names, differs between the pre-move and post-move definitions (FR-067-CON-3).
`value::expression`'s own code importing these types from `forms` is
expected and does not itself fail this test; a second definition, or a
shape difference, does. Scope: FR-067-AC-9, FR-067-CON-3.

## Test Procedure

1. Search the compiled crate's module tree for `value::expression::syntax`.
2. For each of the eight named types, search the compiled crate's type
   definitions (not references or imports) for every location that defines
   that type, and record the module path of each defining location found.
3. Inspect `value::expression`'s `check`, `evaluate`, `facts`, `ir`,
   `refusal` and `termination` modules for their use of the eight named
   types, distinguishing an import from `forms` from an independent
   re-definition.
4. For each of the eight named types, extract its pre-move definition (the
   commit immediately before this requirement's implementation) and its
   post-move definition (under `forms`, after) as sets: for an enum, its
   variant names and each variant's field names and types; for a struct, its
   field names and types; for either, its method signatures (name,
   parameter types, return type). Compare the pre-move set against the
   post-move set for each type and require set equality — a variant, field
   or method added, removed or renamed, or a type or signature changed,
   makes the two sets unequal.

## Expected Results

- Step 1: `value::expression::syntax` is absent from the module tree.
- Step 2: each of the eight named types has exactly one defining location,
  under `forms`; no type has a second defining location under
  `value::expression` or anywhere else.
- Step 3: every use found is an import from `forms`; none is an independent
  definition.
- Step 4: for every one of the eight types, the pre-move and post-move
  variant/field/method sets are equal; a type with any added, removed,
  renamed or retyped variant, field or method fails this step, naming that
  type and the specific difference.
