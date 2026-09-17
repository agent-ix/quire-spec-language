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
the selected meaning-id registry, and that an unbound or incompatible meaning
refuses the package. Scope: FR-056-AC-3. The refusal code for an unknown meaning
id is fixed by agent-ix/quire-specification#85.

## Test Procedure

1. Lift a bundle with two kinds whose constructs carry registered meaning ids and
   admit it as the control.
2. Change one construct's `meaning` id to an id absent from the registry, keeping
   its kind name, shape and identity.
3. Restore the meaning id, then change that construct's `identity` or `shape` to
   a value the bound meaning does not accept.
4. Restore it, then rename the kind to a different `name` under the same module
   while keeping its `meaning` id.

## Expected Results

- Step 1 admits both kinds.
- Step 2 refuses each IR node of the changed kind with the #85 refusal, naming
  the meaning id, the module-qualified kind, the IR node, the artifact and the
  span. No declaration of the package is admitted.
- Step 3 refuses each IR node of the changed kind with
  `invalid_model_binding`/`malformed-declaration`, and no declaration is admitted.
- Step 4 admits with the same Quire meanings and export records as step 1.
- Assertions compare typed refusal codes and loci, never message text.
