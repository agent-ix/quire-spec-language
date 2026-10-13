---
id: TC-991
title: "A clause or member naming a variant type refuses while the declaration is admitted"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: verifies
---
# TC-991: A clause or member naming a variant type refuses while the declaration is admitted

## Description

Verify that the variant-type refusal belongs to the naming clause or member, not to the declaration. Scope: FR-056-AC-27.

## Test Procedure

1. Admit the TC-988 package and record its declarations and exports.
2. Compile a clause that names `Tier`.
3. Add an object field whose `typeRef` names `Tier` and admit.

## Expected Results

- Step 2 refuses the clause `unsupported_construct`/`declaration-form`; the package, `Tier`'s declaration and its `enum` and `variant` exports are those of step 1.
- Step 3 refuses the member `unsupported_construct`/`declaration-form` naming the `typeRef`.
- Assertions compare typed code, cause and locus.
