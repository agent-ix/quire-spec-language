---
id: TC-260
title: "ValueTypeRef is exactly the two-member union Native/Package"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-088
    type: verifies
---
# TC-260: ValueTypeRef is exactly the two-member union Native/Package

## Description

Verify that `ValueTypeRef` is defined exactly as the two-member union
`{Native(NativeValueType), Package(DeclarationKey)}`, with no third
variant, and that no model field type anywhere in the crate is represented
by a bare `NodeKey` or a raw string type name instead of going through
this union. Scope: FR-088-AC-8.

## Test Procedure

1. Read `ValueTypeRef`'s definition and confirm it has exactly two
   variants, `Native(NativeValueType)` and `Package(DeclarationKey)`, in
   that shape.
2. Search the whole compiled crate for any other definition of a type
   named `ValueTypeRef`, or for any third variant added to this one;
   confirm none exists.
3. Search every model field-type declaration site in the crate (the places
   that record a field's type for a domain-model declaration) and confirm
   each one is typed as `ValueTypeRef`, never as a bare `NodeKey` or a raw
   `String`/`&str` naming a type.
4. Adverse test: attempt to represent a package-declared field type using
   a bare `NodeKey` directly (bypassing `ValueTypeRef::Package`) and
   confirm no code path in the crate accepts this form as a model field
   type.

## Expected Results

- Steps 1-2: exactly one `ValueTypeRef` definition, with exactly the two
  named variants; a third variant or a second definition fails this step.
- Step 3: every model field type is `ValueTypeRef`-typed.
- Step 4: no bare-`NodeKey`/raw-string field-type path exists.
