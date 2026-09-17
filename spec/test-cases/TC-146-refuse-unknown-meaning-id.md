---
id: TC-146
title: "Refuse an unknown or mismatched construct meaning id"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-056, type: verifies }
---
# TC-146: Refuse an unknown or mismatched construct meaning id

## Description

Verify that meaning binding goes only through each construct's `meaning` id and
the meaning-binding table, and that an unbound or incompatible meaning refuses
with a named, located diagnostic. Scope: FR-056-AC-3.

## Test Procedure

1. Lift a bundle with two kinds whose constructs carry admitted meaning ids and
   admit it as the control.
2. Change one construct's `meaning` id to an id absent from the meaning-binding
   table, keeping its kind name, shape and identity unchanged.
3. Keep the admitted meaning id, then change that construct's `identity` or
   `shape` to a value the bound Quire meaning does not accept.
4. Rename the kind of an admitted construct to a different `name` under the same
   module while keeping its `meaning` id.

## Expected Results

- Step 1 admits both kinds.
- Step 2 refuses every declaration of the changed kind with `unknown-meaning-id`.
  The diagnostic names the meaning id, the module-qualified kind and each refused
  declaration's source locus. Declarations of the other kind stay admitted.
- Step 3 refuses the changed kind's declarations with `meaning-shape-mismatch`,
  naming the meaning id, the declared shape and identity, and the accepted values.
- Step 4 admits with the same Quire meaning and export kinds as step 1. A kind
  name alone never changes or supplies a meaning.
- Assertions compare typed diagnostic codes and loci, never message text.
