---
id: TC-1143
title: "A step relation emits a v2 step-relation claim node"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-180
    type: verifies
---
# TC-1143: A step relation emits a v2 step-relation claim node

## Description

Verify that S4 emits a checked HP-1 step relation as one QSpec FR-477
`claim`/`step_relation` node that references every code-claim premise, and
that the I2 reader refuses a node with a premise missing.

Scope: FR-180-AC-4.

## Test Procedure

Use FR-180-AC-1's `Det` over the vault whose `step` has a precondition, a
postcondition and a declared invariant.

1. Compile `Det` with `step` bound by an abstraction relation, and without
   the binding; read each `claim`/`step_relation` node, and compare `x`'s and
   `y`'s binder nodes and the nodes the body reads.
2. Remove `x`'s precondition from its `preconditions` aggregate in step 1's
   bound node; separately apply each of QSpec TC-962's mutations RN-07 to
   RN-12 and RN-14. After each edit set `dependencies` to the body's
   targets, recompute the node key, rewrite the `identity_projection` entry
   and recompute the `package_id`, and feed the package to the I2 reader.

Tag the tests `#[trace("TC-1143", "FR-180-AC-4")]`.

## Expected Results

- Step 1: each unit compiles with no `NotEmitted` refusal; each node's body
  holds `subjects` (`V` once), two `executions` entries referencing `step`'s
  `state`/`operation_anchor` node with its precondition and postcondition
  nodes, the `invariants` of `Vault` and the `body`; the bound unit's
  `implementation` members reference the abstraction relation node and the
  unbound unit's are empty; `x`'s `self` and `i` nodes (levels 0 and 1)
  differ from `y`'s (levels 2 and 3), and the body reads `pre(x.self.l)` and
  `pre(y.self.l)` through different nodes.
- Step 2: `invalid_model_binding`/`malformed-declaration` at the
  `preconditions` aggregate; each TC-962 mutation refuses as TC-962 states.
