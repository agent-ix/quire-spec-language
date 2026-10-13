---
id: TC-1004
title: "The assembler declares each variant type in the type environment by effective identity"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: verifies
---
# TC-1004: The assembler declares each variant type in the type environment by effective identity

## Description

Verify that an admitted variant type is assembled into the type environment under its effective identity while no source name resolves to it. Scope: FR-056-AC-31.

## Test Procedure

1. Assemble a unit that selects the TC-994 package (cases listed `Standard` then `Gold`) and names no variant type.
2. Compare each declared variant type and case with the effective view of the same package.
3. Assemble a unit that names `Config::Tier`.

## Expected Results

- Step 1 declares `Config::Tier` (cases `Gold`, `Standard`) and `Config::Level` (case `Gold`), ascending by effective identity.
- Step 2 finds each declaration keyed by the effective identity the view gives its type, and each case keyed by the view's effective identity for its case node, owned by that type.
- Step 3 refuses `unsupported_construct`/`declaration-form`.
- Assertions compare typed keys, names and causes.
