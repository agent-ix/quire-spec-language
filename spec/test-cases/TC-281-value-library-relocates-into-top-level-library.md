---
id: TC-281
title: "value::library and value::package_identity relocate into the new top-level library module, per the R-10/T-3 shapes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: verifies
---
# TC-281: value::library and value::package_identity relocate into the new top-level library module, per the R-10/T-3 shapes

## Description

Verify that `src/value/library.rs` (FR-307's reusable-semantic-library
resolution: `LibraryName`, `LibraryPackage`, `PackageId`,
`ImportDeclaration`, `LibraryLock`, `resolve_libraries`, and the rest of its
public surface, re-exported today at `value::*`, `src/value/mod.rs:154`)
and `src/value/package_identity.rs` are relocated, not copied, into this
requirement's new top-level `library` module, that neither module remains
under `value` afterward, and that the relocation lands the shapes FR-087's
owner ruling requires (Description, item 3): `ExportIdentity{package, node:
NodeKey}` and `resolve_name` do not relocate as-is, and `package_identity`'s
wire-to-`NodeKey` path relocates as a wire-to-`WireNodeId` path. FR-060's
T12-B names `value::library` among its *allowed* callers of the kernel
`NodeKey` constructor, not a real current one (FR-060 Status names the real
call sites as `value::enumeration`, `value::unit` and `value::model_query`,
none of them `value::library`); the real pre-existing wire-to-`NodeKey`
call this test scans for is `package_identity`'s `node_key` function
(`NodeKey::from_hex` over wire-read JSON, `value/package_identity.rs:177-182`),
a call T12-B's own patterns do not match. Scope: FR-087-AC-11.

## Test Procedure

1. Confirm `src/value/library.rs` and `src/value/package_identity.rs` no
   longer exist, and that `src/value/mod.rs` declares no `mod library;` or
   `mod package_identity;`.
2. Confirm the new `qsl-semantics/src/library/` module exports `LibraryName`,
   `LibraryPackage`, `PackageId`, `ImportDeclaration`, `LibraryLock`,
   `resolve_libraries`, and `package_identity`'s preimage-reading functions,
   each with a definition byte-identical to its pre-relocation body except
   where step 4 below requires a shape change (only the module path changes
   otherwise; a diff against the pre-relocation tree touches no line inside
   any other relocated function or type body, other than `use` lines
   repointed at the new module path).
3. Run the existing FR-307 test suite (`tests/library_resolution.rs`,
   TC-227) against the relocated code, updated for step 4's shape changes,
   and confirm every existing scenario is still covered.
4. Confirm the relocated module does NOT carry forward, in their
   pre-relocation shape: `ExportIdentity` as a type with a `node: NodeKey`
   field (`PackageNodeKey{package: package_id, node: WireNodeId}`,
   FR-087-AC-5, is the module's sole cross-package node reference after
   relocation), `resolve_name` as a function defined in `library` (name
   resolution against an import is E3's own, over `ImportView`; the
   relocated module carries `resolve_libraries` and `LibraryLock` forward
   but not `resolve_name`), and `package_identity`'s `node_key`-equivalent
   function returning a `NodeKey` (it returns a `WireNodeId` after
   relocation, and calls no `NodeKey` constructor, including
   `NodeKey::from_hex`).
5. Search the relocated `library` module's whole source for any
   `NodeKey`-constructing call — `NodeKey::of(`, `NodeKey::from_bytes(`,
   `NodeKey::from_hex(`, `node_key_of(`, or an equivalent construction from
   a digest or wire-read value — and confirm none exists; separately,
   search every type `library` defines — not only a cross-package reference
   type such as `ExportIdentity`/`PackageNodeKey`, but every type, including
   `package_identity`'s relocated structural types — for a `NodeKey`-typed
   field whose value is read from wire or preimage data, and confirm none
   exists. This step names `PreimageDefect` specifically:
   `AmbiguousDeclaration`'s `nodes: [NodeKey; 2]` field and
   `DeclarationNominalMismatch`'s `node: NodeKey` field are themselves built
   from wire-read preimage data, and `ProjectedDeclarations`'s internal map
   is populated the same way; a relocation that leaves any of these three
   still typed `NodeKey` passes step 4 (which scans only `node_key`'s own
   return type) but must fail this step.
6. Confirm the FR-060 T12-B allow-list
   (`spec/functional/FR-060-check-qsl-api-surface-boundary.md:82`) no
   longer needs to name `value::library` as a `NodeKey`-constructor caller
   (it did not name a real call site there before this requirement either;
   FR-060 Status), and that step 5's scan, not T12-B's pattern set alone,
   is what confirms `package_identity`'s `NodeKey::from_hex` call is gone
   (T12-B's own patterns do not match `NodeKey::from_hex(`).

## Expected Results

- Step 1: `value::library` and `value::package_identity` are absent; their
  presence fails this step.
- Step 2: every relocated item is defined once, in `library`, with an
  unchanged body except where step 4 requires a shape change; an
  unexplained behavioral diff fails this step.
- Step 3: TC-227's existing scenarios are still covered against the
  relocated, updated module; a scenario silently dropped rather than
  migrated fails this step.
- Step 4: `ExportIdentity` with a `node: NodeKey` field, a `resolve_name`
  function, or a `package_identity` node-reference function returning
  `NodeKey` found anywhere in the relocated module fails this step.
- Step 5: any `NodeKey`-constructing call (including `NodeKey::from_hex`),
  or any `NodeKey`-typed field anywhere in `library` whose value is read
  from wire or preimage data — including `PreimageDefect::AmbiguousDeclaration.nodes`,
  `PreimageDefect::DeclarationNominalMismatch.node`, or
  `ProjectedDeclarations`'s internal map still typed `NodeKey` rather than
  `WireNodeId` — fails this step and names the call site or field.
- Step 6: the T-12 scan's allow-list needs no `value::library` entry, and
  step 5's scan, not T12-B alone, confirms `package_identity`'s prior
  `NodeKey::from_hex` call is gone.
