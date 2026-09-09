---
id: TC-042
title: "Bind all native model semantics and provenance"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-015
    type: verifies
---
# TC-042: Bind all native model semantics and provenance

## Description

Property, priority P1. Verifies FR-015-AC-3. Planned until real execution; setup
must use the qualified source-derived Rust producer and actual public IR APIs.

## Test Procedure

Generate one-at-a-time changes to nominal scalar names, units, object universe, operation anchor/frame, source identity/revision/bytes, formal bounds and an unused declaration. Regenerate legitimate source loci for semantic changes so setup remains valid. Also permute set-like declaration/role/site/frame inventories. Select imports using the current digest, an old formal-only digest, the IR semantic CanonicalDigest and a foreign model digest.

## Expected Results

Every semantic/provenance mutation changes the raw native artifact digest; set-like input permutations preserve the same deterministic artifact. Correct exact imports link; each wrong artifact fails selection. Ordered operation parameters are retained rather than sorted. This is a bounded generated family, not a claim about cryptographic collisions.

