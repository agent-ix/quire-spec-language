---
id: TC-997
title: "A clause, field or operation signature naming a variant type refuses while the declaration is admitted"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: verifies
---
# TC-997: A clause, field or operation signature naming a variant type refuses while the declaration is admitted

## Description

Verify that the variant-type refusal belongs to the naming clause or member, not to the declaration. Scope: FR-056-AC-27, FR-056-AC-28 and FR-056-AC-32.

## Test Procedure

1. Admit the TC-994 package and record its declarations and exports. Compile a unit with no clause naming `Tier`.
2. Compile a clause that names `Tier`.
3. Add an object field `Account.tier` whose `typeRef` names `Tier` and admit.
4. Declare an operation `setTier` with parameter `t` (position 0) and a result both naming `Tier`, then repeat naming an interface-featured object type, and naming an undeclared type in a package built without the reader. For the undeclared type, break the reference after a reader admitted the operation, so its origin is retained.

## Expected Results

- Step 1 raises no refusal.
- Step 2 refuses the clause `unsupported_construct`/`declaration-form`; the package, `Tier`'s declaration and its `enum` and `variant` exports equal those of step 1.
- Step 3 refuses `invalid_model_binding`/`malformed-declaration` at node `ix://<package identity>/Account/tier`, path `Field.typeRef`, and admits no declaration of the package. QSL-738 replaces this placeholder.
- Step 4 refuses at the operation node `ix://<package identity>/ConfigVersion/setTier`: path `Operation.params[0].typeRef` for the parameter and `Operation.returns.typeRef` for the result, at a later parameter position `1` when the parameter is second, with the operation's artifact and position. The variant and interface cases are `invalid_model_binding`/`malformed-declaration`; the undeclared type is `dangling_reference`/`unknown-value-type` and keeps the operation's artifact and source position.
- A relationship source end and target end, a namespace member (at any position, such as `members[1]`) and a population member type of the wrong meaning each refuse `malformed-declaration` at their own node with paths `Relationship.source`, `Relationship.target`, `TypeDefinition.members[i]` and `PopulationDefinition.members[i]`.
- Assertions compare typed code, cause, node identity and path.
