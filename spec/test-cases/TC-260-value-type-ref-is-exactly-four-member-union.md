---
id: TC-260
title: "ValueTypeRef is exactly the four-member union Clock/Native/Package/Unit"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-088
    type: verifies
---
# TC-260: ValueTypeRef is exactly the four-member union Clock/Native/Package/Unit

## Description

Verify that `ValueTypeRef` is defined exactly as the four-member union
`{Clock, Native(NativeValueType), Package(DeclarationKey), Unit(String)}`,
with no fifth variant, and that no model field type anywhere in the crate is represented
by a bare `NodeKey` or a raw string type name instead of going through
this union. Scope: FR-088-AC-8.

## Test Procedure

1. Read `ValueTypeRef`'s definition and confirm it has exactly four
   variants, `Clock`, `Native(NativeValueType)`, `Package(DeclarationKey)` and
   `Unit(String)`, in that shape. `Clock` carries no fields and is a place in
   a timed model (FR-230); the `String` of `Unit` is a unit name the assembler
   resolves (FR-056-AC-42), not a type name.
2. Search the whole compiled crate for any other definition of a type
   named `ValueTypeRef`, or for any fifth variant added to this one;
   confirm none exists.
3. Search every model field-type declaration site in the crate (the places
   that record a field's type for a domain-model declaration) and confirm
   each one is typed as `ValueTypeRef`, never as a bare `NodeKey` or a raw
   `String`/`&str` naming a type (the `Unit` variant's unit name is the one
   string, and it names a unit, not a type).
   The automated scan covers struct fields and enum variant fields under
   `model` whose name contains `type`. Three raw wire views of a snapshot
   document's `record` member (`RawRecord`, `RawNode`, and the reader's `Open`,
   each with a `type_identity` string) are outside this step: they hold the
   wire `type` string until a checked declaration resolves it by lookup, and
   the scan names them. A field type under another name is not covered.
4. Adverse test: attempt to represent a package-declared field type using
   a bare `NodeKey` directly (bypassing `ValueTypeRef::Package`) and
   confirm no code path in the crate accepts this form as a model field
   type.

## Expected Results

- Steps 1-2: exactly one `ValueTypeRef` definition, with exactly the four
  named variants; a fifth variant or a second definition fails this step.
- Step 3: every model field type is `ValueTypeRef`-typed.
- Step 4: no bare-`NodeKey`/raw-string field-type path exists.
