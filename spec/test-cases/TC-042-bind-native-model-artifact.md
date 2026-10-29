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

Property, priority P1. Verifies FR-015-AC-3. Qualified; SR-085
records the actual public-API controls, local gates and bounded claim. Tests use
the source-derived Rust producer and successfully constructed actual IR inputs.

## Test Procedure

Generate one-at-a-time changes to nominal scalar names, units, object universe, operation anchor/frame, source identity/revision/bytes, formal bounds and an unused declaration. Regenerate legitimate source loci for semantic changes so setup remains valid. Also permute set-like declaration/role/site/frame inventories. Select imports using the current identity and revision, an old formal-only revision and a foreign model identity.

## Expected Results

Every semantic/provenance mutation changes the native artifact content, compared as the structural value; set-like input permutations preserve the same deterministic artifact. Correct exact imports link; each wrong artifact fails selection. Ordered operation parameters are retained rather than sorted.
