---
id: TC-248
title: "Frame identity's subject sets resolve to DeclarationKey through the model correspondence"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-088
    type: verifies
---
# TC-248: Frame identity's subject sets resolve to DeclarationKey through the model correspondence

## Description

Verify that a frame's identity is the checked node id of its `state` node
with `semantic_form: "frame"`, and that each `NodeKey` in its
`modifies`/`creates`/`deletes` sets resolves to a `DeclarationKey` only by
reading the checker's recorded model correspondence (ADR-013 O-04), never
by re-deriving the mapping through a fresh search. Scope: FR-088-AC-2.

## Test Procedure

1. Check a package containing a frame with non-empty `modifies`, `creates`
   and `deletes` sets, each naming at least one relation/model node bound
   to a domain-package declaration.
2. Confirm the frame's own identity is the checked node id of its `state`
   node.
3. For each `NodeKey` in the three sets, resolve it to a `DeclarationKey`
   using the checker's recorded model correspondence, and record the
   result.
4. Delete the corresponding entry from the in-memory model correspondence
   (simulating a stale or incomplete correspondence) and attempt the same
   resolution again without re-checking the package from source.
5. Re-check the same package from source, confirm the model correspondence
   is rebuilt, and confirm the resolution in step 3 reproduces exactly, for
   every entry, from the freshly rebuilt correspondence.

## Expected Results

- Step 2: the frame's identity is the `state` node's checked node id.
- Step 3: every `NodeKey` resolves to its `DeclarationKey`.
- Step 4: the resolution fails (or is absent) once its correspondence entry
  is removed, demonstrating the lookup is not independently re-derived by
  a fallback search.
- Step 5: a fresh check reproduces the same resolutions, confirming the
  correspondence — not any other collection order — is the sole source
  (R-05).
