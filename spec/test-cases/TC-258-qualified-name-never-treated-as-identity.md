---
id: TC-258
title: "QualifiedName is used only as a declared preimage component, never as an identity"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-088
    type: verifies
---
# TC-258: QualifiedName is used only as a declared preimage component, never as an identity

## Description

Verify that `QualifiedName` appears in the crate only as a declared
component of an identity preimage (for example inside `DeclarationKey`'s
or a checked node's own preimage where a domain package's declared name
participates), and that no equality, hashing, or lookup implementation on
any identity type treats a `QualifiedName` as the sole identity-bearing
field where a node id or digest is available instead. Scope: FR-088-AC-6.

## Test Procedure

1. Search the whole compiled crate for every type that embeds a
   `QualifiedName` field, and for each, read its `PartialEq`/`Eq`/`Hash`
   implementation (derived or manual).
2. Confirm every such type's equality/hashing is over its full preimage
   (including the `QualifiedName` as one component among others that
   ultimately reduce to a node id or digest), not over the `QualifiedName`
   alone as if it were itself the identity.
3. Search for any `HashMap`/`BTreeMap`/similar collection keyed solely by
   `QualifiedName` used to answer an identity question (rather than, for
   example, a name→node-id resolution table that is itself confined to the
   check stage, R-06). Confirm none exists outside the check stage's own
   resolution machinery.
4. Adverse test: construct two declarations with equal qualified names but
   different node ids (for example, shadowed declarations in different
   scopes admitted as distinct nodes) and confirm any identity comparison
   distinguishes them by node id, not by their shared `QualifiedName`.

## Expected Results

- Steps 1-2: every embedding type's identity semantics come from the full
  preimage/node id, not the bare `QualifiedName`.
- Step 3: no identity-answering collection is keyed solely by
  `QualifiedName` outside the check stage's own resolution.
- Step 4: two same-named, different-node-id declarations compare unequal.
