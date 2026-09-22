---
id: TC-254
title: "library converts VerifiedPackage to ImportView without resolving any name"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: verifies
---
# TC-254: library converts VerifiedPackage to ImportView without resolving any name

## Description

Verify that `library`'s conversion from `VerifiedPackage` to `ImportView`
exposes the verified package's exported declarations as opaque data, keyed
for `PackageNodeKey` lookup by the importing package's own checker, and
that no function in `library` takes a name (`QualifiedName` or bare
string) and returns a declaration or node id once the exporting package's
own check stage has produced it. Scope: FR-087-AC-4.

## Test Procedure

1. Construct a `VerifiedPackage` for an exporting package with at least two
   exported declarations sharing no name (and, separately, a case with two
   exported declarations of different kinds at different `WireNodeId`s).
2. Convert it to an `ImportView` and confirm every exported declaration is
   present, each keyed by its `WireNodeId` (via `PackageNodeKey` once the
   importing package's checker builds one), not by its declared name.
3. Search `library`'s whole public and private surface for any function
   whose signature accepts a `QualifiedName` or `&str`/`String` and returns
   a declaration, a node id, or an `ImportView` entry.
4. Confirm no such function exists: every `ImportView` accessor is keyed by
   `WireNodeId` (through `PackageNodeKey`), never by name.
5. Adverse test: attempt to look up an `ImportView` entry using only the
   exported declaration's textual name (no `WireNodeId`), and confirm no
   code path in `library` performs this lookup — the importing package's
   own checker is the only place a name is ever resolved, and it resolves
   the name to a `WireNodeId`/`PackageNodeKey` pair before ever touching
   `ImportView`.

## Expected Results

- Step 2: `ImportView` entries are present and keyed by `WireNodeId`.
- Steps 3-4: no name-keyed lookup function exists anywhere in `library`.
- Step 5: no textual-name-only lookup path exists; a lookup that succeeds
  using only a name, with no `WireNodeId` involved, fails this test.
