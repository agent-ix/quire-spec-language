---
id: TC-245
title: "PackageNodeKey has exactly one shape and declared equality"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: verifies
---
# TC-245: PackageNodeKey has exactly one shape and declared equality

## Description

Verify that `PackageNodeKey` is defined exactly as
`{package: package_id, node: WireNodeId}` — the shape ADR-011:200,
ADR-011:353, ADR-013 T-3 and FR-322-AC-26 agree on — with no second
definition anywhere in the crate carrying a `node: NodeKey` field (the
shape review finding FND-021 recorded as a defect, since fixed in the ADR
text but not yet built). Also verify declared equality: two keys compare
equal iff both components compare lexically equal, with no structural
comparison over the referenced node's content substituting. Scope:
FR-087-AC-5.

## Test Procedure

1. Search the whole compiled crate for every definition of a type named
   `PackageNodeKey` (or an equivalent cross-package node reference type
   under any name) and record each one's field list and field types.
2. Confirm exactly one definition exists, and that its shape is
   `{package: package_id, node: WireNodeId}`.
3. Construct two `PackageNodeKey` values with equal `package` and `node`
   components built from independently-constructed but byte-identical
   inputs; confirm they compare equal.
4. Construct two `PackageNodeKey` values differing only in `node`; confirm
   they compare unequal.
5. Construct two `PackageNodeKey` values differing only in `package`;
   confirm they compare unequal.
6. Mutation test: construct two `PackageNodeKey` values with the same
   `package` component but two DISTINCT `WireNodeId`s whose referenced
   nodes have structurally identical checked content. The declared,
   component-lexical rule says these two keys compare unequal (the `node`
   components differ lexically, per step 4); replace the equality
   implementation with one that instead compares the two keys' referenced
   nodes' checked content (a structural comparison) and confirm the mutant
   reports these two keys equal — the wrong answer, since their `WireNodeId`s
   differ — so step 4's own inequality assertion, re-run against the
   mutant, fails and catches it.

## Expected Results

- Steps 1-2: exactly one `PackageNodeKey` definition, with the shape named
  above; a second shape, or a `node: NodeKey` field, fails this step.
- Steps 3-5: equality matches the declared, component-lexical rule exactly.
- Step 6: the mutant (structural comparison over identical content behind
  two distinct `WireNodeId`s) is caught because it flips step 4's inequality
  result to equal; a mutant that survives (still reports these two keys
  unequal) fails this test.
