---
id: TC-410
title: "Each connected supertype component has its own object universe, and a reference key carries the authored object identity"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-084
    type: verifies
---
# TC-410: Each connected supertype component has its own object universe, and a reference key carries the authored object identity

## Description

Verify FR-084-AC-7. An object universe is one connected component of the
supertype graph restricted to object types (quire-specification
`model-complete.md`, "Object universe"; ADR-013 O-05, OQ-C and OQ-E
rulings). A model with two disconnected components has two universes, each
the `quire.model.object-universe/v1` digest over the model selection and
that component's root effective type identities. A reference key's object
component is the authored object identity, not a digest. Scope:
FR-084-AC-7.

This catches three faults: one universe per model over every root type
(`qsl-semantics/src/model/normalize.rs:1819-1832`), one universe per root type, and a
digested object component.

## Test Procedure

1. Admit a domain package with object types `A`, `B` (`supertypes: [A]`),
   `C`, `D`, `E` (`supertypes: [C, D]`) and `X` (no supertype and no
   subtype), so that the supertype graph has the components `{A, B}`,
   `{C, D, E}` and `{X}`.
2. Admit three closed populations: `p1` covering `B` with the member `b1`,
   `p2` covering `E` with the member `e1`, and `p3` covering `X` with the
   member `x1`.
3. Call `allInstances<A>(p1)`, `allInstances<C>(p2)` and
   `allInstances<X>(p3)`, and read the reference keys of `b1`, `e1` and
   `x1`.
4. Compute the expected universes independently: SHA-256 over the RFC 8785
   JCS bytes of
   `{version: quire.model.object-universe/v1, model_selection, root_types}`
   with `root_types` equal to `[A]`, to `[C, D]` in ascending effective
   identity order, and to `[X]`, each as effective identities.

Tag the test `#[trace("FR-084-AC-7", "TC-410")]`.

## Expected Results

- `b1`'s, `e1`'s and `x1`'s universes equal the three digests from step 4
  in that order, and the three differ.
- Each object component is the member's authored identity bytes (`b1`,
  `e1`, `x1`).

## Status

Planned; no test backs this case. Remaining work: QSL-131.
