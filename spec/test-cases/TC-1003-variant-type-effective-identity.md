---
id: TC-1003
title: "A variant type and its cases have effective identities independent of case order, and conflicting cases leave no view"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: verifies
---
# TC-1003: A variant type and its cases have effective identities independent of case order, and conflicting cases leave no view

## Description

Verify that normalization gives a variant type an effective type and each case an effective member, and that no conflicting case yields a view. Scope: FR-056-AC-29, FR-056-AC-30 and FR-081-AC-16.

## Test Procedure

1. Normalize the TC-994 package (`Tier` with `Gold` and `Standard`, `Level` with `Gold`) and read the effective view.
2. Normalize it again with `Tier`'s cases listed `Standard` then `Gold`.
3. Normalize `Tier` with cases `Gold` and `gold`.
4. Normalize `Tier` with two cases named `Gold` and distinct identities, beside the valid `Level`.
5. Normalize `Tier` with `Gold` and a `Silver` carrying the identity `.../variant/Tier-Gold`, listed in each order.

## Expected Results

- Step 1 lists five effective declarations: `Tier` and `Level` with owner null and one `qualify` fact `[declaration]`, and three case members whose `original` is the case node key, whose owner is the variant type's effective identity and each with one `qualify` fact. `type_identities` holds `Tier` and `Level` only; `Tier`'s `Gold` and `Level`'s `Gold` differ.
- Step 2 gives the same effective entries, in the same order.
- Step 3 gives distinct effective identities for `Gold` and `gold`.
- Step 4 refuses `invalid_model_binding`/`conflicting-binding` at node `ix://<package identity>/Tier`, path `TypeDefinition.variants[1]`, and yields no effective view.
- Step 5 refuses once in each order, `invalid_model_binding`/`conflicting-binding` at node `ix://<package identity>/Tier`, path `TypeDefinition.variants[1]`.
- Assertions compare typed code, cause, node identity and path.
