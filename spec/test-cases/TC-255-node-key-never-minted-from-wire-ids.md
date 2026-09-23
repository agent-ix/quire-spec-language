---
id: TC-255
title: "NodeKey is never minted from a WireNodeId; E4 and E9 resolve by lookup, never by construction"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: verifies
---
# TC-255: NodeKey is never minted from a WireNodeId; E4 and E9 resolve by lookup, never by construction

## Description

Verify that no `NodeKey` constructor call anywhere in the crate is fed,
directly or transitively, by a `PackageNodeKey`'s `WireNodeId` field, an
`ImportView` entry, a `VerifiedPackage`, or any other wire-read value: `library`,
`package`'s E4 dependency resolution, and `replay`'s E9 lookup each make
zero `NodeKey`-constructor calls. ADR-013 O-04 (`:173`) states the minting
rule directly: "In QSL only `check` calls it" (the constructor). ADR-013
O-04 (`:175`) states how E4 and E9 relate to a `WireNodeId` instead: "It
becomes a `NodeKey` only by lookup in a QSL checked package" — at E4,
against the dependency's own checked package, compiled independently from
its digest-addressed source, whose own S3 checking already minted the
`NodeKey` being referenced; at E9 in the `replay` facade, against the
package recompiled from source, likewise already checked. Neither E4 nor
E9 calls the constructor: both resolve a `WireNodeId` to a `NodeKey` that
another package's own checking already produced. A constructor call fed by
a `WireNodeId`, at E4, E9, in `library`, or anywhere else, is exactly the
R-10 violation this criterion exists to catch.

This test is scoped to what FR-087 (S-3) owns, not to "only `check` calls
the constructor" crate-wide: that crate-wide claim is FR-060 T12-B's own
allow-list, whose named debt list (FR-060 Behavior, "T12-B and T12-C: shipped code and debt lists")
holds the real minting sites outside `check` this requirement does not touch
(pre-existing debt, Remaining work: agent-ix/quire-spec-language#211). This
criterion does not require the debt list to be empty; it requires `library`,
`package`'s E4 path, and `replay`'s E9 path each to add none, and no call
anywhere to be fed by a `PackageNodeKey`'s `WireNodeId`. Scope: FR-087-AC-6.

## Test Procedure

1. Search `library`'s whole source, `package`'s E4 dependency-resolution
   code path, and `replay`'s E9 lookup for any reference to a `NodeKey`
   constructor, called or passed as a function value
   (`NodeKey::from_digest`), any
   `node_key_of(` call, or an equivalent construction from a digest or
   wire-read value.
2. Confirm none exists in any of the three: E4's dependency resolution (in
   `package`) resolves a `PackageNodeKey`'s `WireNodeId` to a `NodeKey` by
   looking one up in the dependency's own already-checked package —
   indexed by the node identity the `WireNodeId` names — rather than by
   calling the `NodeKey` constructor; E9's lookup in `replay` does the
   same, against the recompiled package's already-minted `NodeKey`s.
3. Search the whole compiled crate's call graph for every call site of the
   `NodeKey` constructor fed, directly or transitively, by a
   `PackageNodeKey`'s `node: WireNodeId` field, an `ImportView` entry, or a
   `VerifiedPackage`; confirm none exists anywhere in the crate, not only
   in `library`/`package`/`replay`.
4. Confirm the FR-060 T-12 API-surface scan (`arch-lint api-surface`)
   still enforces "only `check` calls the `NodeKey` constructor" as its own
   allow-list, and that adding a constructor call inside `library`, E4's
   dependency resolution, or E9's `replay` lookup makes that scan fail
   (none of those is on T12-B's debt list).
5. Confirm every mint the scan reports outside `check` is in a function on
   FR-060's T12-B debt list, the pre-existing debt tracked under
   agent-ix/quire-spec-language#211 and out of this requirement's scope:
   this step records that fact, it does not require remediating those
   sites.
6. Adverse test: trace the call graph transitively (not only direct call
   sites) from E4's and E9's lookup functions, and from every function
   `library` exposes, and confirm none reaches the `NodeKey` constructor
   through any helper function fed by a `WireNodeId` or other wire-read
   value.

## Expected Results

- Steps 1-2: `library`, E4's dependency resolution, and E9's `replay`
  lookup each make zero `NodeKey`-constructor calls, resolving by lookup
  against an already-minted `NodeKey` instead; a constructor call found in
  any of the three fails this step and names the call site.
- Step 3: no `NodeKey` constructor call anywhere in the crate is fed by a
  `PackageNodeKey`'s `WireNodeId`, an `ImportView` entry, or a
  `VerifiedPackage`; one found fails this step and names the call site and
  its feeding value.
- Step 4: the T-12 scan's allow-list is `check`-only; a constructor call
  added inside `library`, E4, or E9 fails the scan.
- Step 5: every reported mint outside `check` is on T12-B's debt list;
  those pre-existing, out-of-scope sites are left unremediated by this
  criterion; this step records, not requires, their absence.
- Step 6: no transitive path from `library`, E4, or E9 reaches the
  constructor through a wire-fed value.
