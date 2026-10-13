---
id: TC-994
title: "A variant type is admitted as one enum and one variant per case"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: verifies
---
# TC-994: A variant type is admitted as one enum and one variant per case

## Description

Verify that a variant type is admitted and exported as one `enum` and one `variant` per case. Scope: FR-056-AC-24.

## Test Procedure

1. Lift a bundle with variant type `Tier` (`quire.meaning.model.variant-type/v1`, shape `enumeration`) whose `variants` list gives `Gold` then `Standard`, and a second variant type `Level` with a case `Gold`. Admit it.
2. Re-run with the `Tier` cases listed `Standard` then `Gold`.
3. Re-run with `Tier` cases `Gold` and `gold`.

## Expected Results

- Step 1 yields one declaration for `Tier` and one per case node (`variant/Tier-Gold`, `variant/Tier-Standard`); one `enum` keyed by `Tier`'s identity with `name` `Tier` and `members` `[Gold, Standard]`; one `variant` per case keyed by (owner identity, case name) with `case`, owner `enum` key and span; `Level`'s `Gold` has a distinct `variant` key.
- Step 2 changes no key, export or ordering.
- Step 3 admits two distinct `variant` exports and `members` `[Gold, gold]`.
- Assertions compare typed export fields and keys, never message text.
