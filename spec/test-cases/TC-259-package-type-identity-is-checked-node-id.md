---
id: TC-259
title: "A package type's identity is its checked node id, package-scoped"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-088
    type: verifies
---
# TC-259: A package type's identity is its checked node id, package-scoped

## Description

Verify that a package type (`scalar_type`, `composite_type`,
`bounded_domain`) is identified by its checked node id, and that this
identity respects the O-04 package-scoped preimage: two structurally
identical type declarations admitted in two different packages produce two
distinct node ids, while two source *occurrences* (ADR-013 O-07's term: a
reference site, not a second declaration) of the same declared type within
one package resolve to that one declaration's node id. This criterion is
not about declaring a type twice: a structurally identical second
declaration under a colliding name is refused as a duplicate declaration
(a name-binding concern, this repository's own precedent being
`checking::composed::proofs::CorrespondenceError::DuplicateDeclaration`),
never admitted as a second occurrence sharing the first declaration's id;
step 4 below tests two reference sites to one declaration, which is this
criterion's actual claim. Scope: FR-088-AC-7.

## Test Procedure

1. Author two source packages, each declaring a structurally identical
   composite type (same fields, same field types, same names).
2. Check both packages independently and record each type declaration's
   checked node id.
3. Confirm the two node ids differ (package-scoped preimage: the
   declaring package's `name@version` is part of the preimage, ADR-013
   O-04, QC-18).
4. Within a single package, reference the same type declaration from two
   different call sites (for example, two fields of different records both
   typed by it) and confirm both references resolve to the same node id.
5. Re-check the same package a second time from source (a fresh compile,
   not a cached result) and confirm the recomputed node id is identical to
   the first (content-addressed, deterministic).

## Expected Results

- Step 3: cross-package structurally-identical declarations produce
  distinct node ids.
- Step 4: intra-package references to the same declaration share one node
  id.
- Step 5: recompilation reproduces the same node id.
