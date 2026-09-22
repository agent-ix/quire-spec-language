---
id: TC-259
title: "A package type's identity is its checked node id, scoped only by owner"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-088
    type: verifies
---
# TC-259: A package type's identity is its checked node id, scoped only by owner

## Description

Verify that a package type (`scalar_type`, `composite_type`,
`bounded_domain`) is identified by its checked node id, and that this
identity follows the O-04 content key scoped only by owner (ADR-013 OQ-G).
A declared type carries its owner: two packages of the same owner that
declare the same structure under the same qualified name share one node id,
and packages with different owners get distinct ids for the same qualified
name and structure. A builtin or anonymous type has no owner and shares one
id across owners. Two source *occurrences* (ADR-013 O-07's term: a
reference site, not a second declaration) of the same declared type within
one package resolve to that one declaration's node id. This criterion is
not about declaring a type twice: a structurally identical second
declaration under a colliding name is refused as a duplicate declaration
(a name-binding concern, this repository's own precedent being
`checking::composed::proofs::CorrespondenceError::DuplicateDeclaration`),
never admitted as a second occurrence sharing the first declaration's id;
step 4 below tests two reference sites to one declaration. Scope:
FR-088-AC-7.

## Test Procedure

1. Author three source packages that each declare the composite type
   `geo.Point` with the same fields and field types, and each use the
   anonymous bounded type `Int[0, 9]` as a field type. Packages A and B
   have the same source owner (two revisions of one source, so two
   distinct `package_id`s). Package C has a different source owner.
2. Check the three packages independently and record the checked node id
   of each package's `geo.Point` declaration and of its `Int[0, 9]` type
   node.
3. Confirm that:
   - A's and B's `geo.Point` node ids are equal (same owner, same
     declaration);
   - C's `geo.Point` node id differs from A's (different owner, same
     qualified name and structure);
   - the `Int[0, 9]` node id is the same in A, B and C (a builtin or
     anonymous type has no owner, ADR-013 O-04).
4. Within a single package, reference the same type declaration from two
   different call sites (for example, two fields of different records both
   typed by it) and confirm both references resolve to the same node id.
5. Re-check the same package a second time from source (a fresh compile,
   not a cached result) and confirm the recomputed node id is identical to
   the first (content-addressed, deterministic).

## Expected Results

- Step 3: the same owner and the same declaration give the same id;
  different owners with the same qualified name and structure give
  different ids; a builtin or anonymous type gives the same id across
  owners.
- Step 4: intra-package references to the same declaration share one node
  id.
- Step 5: recompilation reproduces the same node id.
