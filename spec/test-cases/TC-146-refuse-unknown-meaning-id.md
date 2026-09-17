---
id: TC-146
title: "Refuse an unknown or mismatched construct meaning id"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-056, type: verifies }
---
# TC-146: Refuse an unknown or mismatched construct meaning id

## Description

Verify that meaning binding goes only through each construct's `meaning` id as
Quire specification FR-208 lists it, and that a missing construct, an unbound
meaning or an invalid node refuses the package under FR-154
(agent-ix/quire-specification PR #86, pending merge). Scope: FR-056-AC-3.

## Test Procedure

1. Lift a bundle with two kinds whose constructs carry FR-208 meaning ids and
   admit it as the control.
2. Change one construct's `meaning` id to an id outside FR-208, keeping its kind
   name, shape and identity. Then, separately, remove its `meaning`.
3. Restore the meaning id, then change that construct so its IR nodes are not
   valid for its meaning under FR-154.
4. Restore it, then rename the kind to a different `name` under the same module
   while keeping its `meaning` id.
5. Restore it, then offer IR nodes of two kinds whose `kind` names no entry of
   the `constructs` table.

## Expected Results

- Step 1 admits both kinds.
- Step 2 refuses each IR node of the changed kind with
  `invalid_model_binding`/`malformed-declaration`, naming the meaning id when
  present, the module-qualified kind, the IR node, the artifact and the span. No
  declaration of the package is admitted.
- Step 3 refuses each IR node of the changed kind with
  `invalid_model_binding`/`malformed-declaration`, and no declaration is admitted.
- Step 4 admits with the same Quire meanings and export records as step 1.
- Step 5 refuses each such node with
  `invalid_model_binding`/`malformed-declaration`, naming the IR node, the
  artifact and the span, one refusal per node in node order, and no declaration
  is admitted.
- Assertions compare typed refusal codes and loci, never message text.
