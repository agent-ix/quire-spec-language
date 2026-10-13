---
id: TC-997
title: "A clause or field naming a variant type refuses while the declaration is admitted"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: verifies
---
# TC-997: A clause or field naming a variant type refuses while the declaration is admitted

## Description

Verify that the variant-type refusal belongs to the naming clause or member, not to the declaration. Scope: FR-056-AC-27 and FR-056-AC-28.

## Test Procedure

1. Admit the TC-994 package and record its declarations and exports. Compile a unit with no clause naming `Tier`.
2. Compile a clause that names `Tier`.
3. Add an object field `Account.tier` whose `typeRef` names `Tier` and admit.

## Expected Results

- Step 1 raises no refusal.
- Step 2 refuses the clause `unsupported_construct`/`declaration-form`; the package, `Tier`'s declaration and its `enum` and `variant` exports equal those of step 1.
- Step 3 refuses `invalid_model_binding`/`malformed-declaration` at node `ix://<package identity>/Account/tier`, path `Field.typeRef`, and admits no declaration of the package. QSL-738 replaces this placeholder.
- Assertions compare typed code, cause, node identity and path.
