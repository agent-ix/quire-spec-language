---
id: TC-810
title: "Abstraction binding keys are unique across all of a unit's declarations"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-304
    type: verifies
---
# TC-810: Abstraction binding keys are unique across all of a unit's declarations

## Description

Verify that several `abstraction-decl`s in one unit check into one relation
and that key uniqueness spans them. Scope: FR-304-AC-7.

## Test Procedure

1. Compile a unit with two `abstraction-decl`s against the ConfigVersion
   domain package: the first binds the ConfigVersion object type, the second
   its population.
2. Add to the second declaration a binding of the ConfigVersion object type
   equal to the first's, and compile.
3. Replace that binding with one whose `rust_type` differs, and compile.

## Expected Results

1. One `CheckedAbstractionRelation` with exactly two bindings, keyed by the
   ConfigVersion and population `DeclarationKey`s.
2. Refused `invalid_model_binding`/`conflicting-binding`, naming both
   bindings and the ConfigVersion key; no checked package.
3. The same refusal as step 2.
