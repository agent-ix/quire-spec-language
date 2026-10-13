---
id: TC-1006
title: "A field, parameter or result typed by a variant type is admitted, exported without a reference and assembled as the enum type"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-360
    type: verifies
---
# TC-1006: A field, parameter or result typed by a variant type is admitted, exported without a reference and assembled as the enum type

## Description

Verify admission, export shape and assembler mapping of a member typed by a variant type. Scope: FR-360-AC-1, FR-360-AC-2 and FR-360-AC-3.

## Test Procedure

1. Admit a package with variant type `Tier`, object `Account.tier: Tier`, record `Settings.tier: Tier`, an optional `Account.maybe` and an ordered non-unique `[0, 3]` `Account.many`, each typed `Tier`, and an event whose occurrence field is `Tier`; repeat with a non-occurrence event field.
2. Compare `Tier`'s `enum` and `variant` exports with the same package without those members.
3. Assemble units that select the package as `Config` and as `Cfg`; read the type of each member and the key of `Config::Tier` and its cases.
4. Admit a field naming a case-less variant type, then fields naming an event type, state machine, process, persistence interface and namespace.

## Expected Results

- Step 1 admits every member with one `field` export each and no `reference` export; the occurrence-field case refuses `invalid_model_binding`/`malformed-declaration` at `TypeDefinition.occurrenceField` and the other is admitted.
- Step 2 finds the exports equal.
- Step 3 types the members `Config::Tier`, `Option<Config::Tier>` and `Sequence<Config::Tier>[0, 3]`, never a `Reference`; the keys equal the view's effective identities and are equal under both aliases.
- Step 4 refuses once at the case-less variant type, path `TypeDefinition.variants`, with no refusal at the field, and refuses each other meaning at the field, path `Field.typeRef`.
- Assertions compare typed codes, causes, keys and paths.
