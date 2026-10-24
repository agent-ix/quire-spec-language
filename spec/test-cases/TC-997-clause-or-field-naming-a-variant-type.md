---
id: TC-997
title: "A variant type is admitted whether or not a clause, field or signature names it, and signature meanings that stay refused keep their node and path"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: verifies
---
# TC-997: A variant type is admitted whether or not a clause, field or signature names it, and signature meanings that stay refused keep their node and path

## Description

Verify that the declaration of a variant type is unaffected by what names it, and that the signature refusals which remain keep their node and path. Scope: FR-056-AC-27, FR-056-AC-28 and FR-056-AC-32. FR-360 owns the typing of what names a variant type.

## Test Procedure

1. Admit the TC-994 package and record its declarations and exports. Compile a unit with no clause naming `Tier`.
2. Compile a clause that names `Tier`.
3. Add an object field `Account.tier` whose `typeRef` names `Tier` and admit.
4. Add a field `Account.broken` whose `typeRef` names a variant type with no case.
5. Declare an operation `setTier` with parameter `t` (position 0) and a result naming an interface-featured object type, then repeat naming an undeclared type in a package built without the reader. For the undeclared type, break the reference after a reader admitted the operation, so its origin is retained.

## Expected Results

- Steps 1 and 2 raise no intake refusal; the package, `Tier`'s declaration and its `enum` and `variant` exports equal those of a package no clause names.
- Step 3 admits the package, with one `field` export `tier` and no `reference` export.
- Step 4 refuses only the case-less variant type (`invalid_model_binding`/`malformed-declaration`, path `TypeDefinition.variants`); `Account.broken` raises no refusal of its own.
- Step 5 refuses at the operation node `ix://<package identity>/ConfigVersion/setTier`: path `Operation.params[0].typeRef` for the parameter and `Operation.returns.typeRef` for the result, at position `1` when the parameter is second, with the operation's artifact and position. The interface case is `invalid_model_binding`/`malformed-declaration`; the undeclared type is `dangling_reference`/`unknown-value-type` and keeps the operation's artifact and source position.
- A relationship source end and target end, a namespace member (at any position, such as `members[1]`) and a population member type of the wrong meaning each refuse `malformed-declaration` at their own node with paths `Relationship.source`, `Relationship.target`, `TypeDefinition.members[i]` and `PopulationDefinition.members[i]`.
- Assertions compare typed code, cause, node identity and path.
