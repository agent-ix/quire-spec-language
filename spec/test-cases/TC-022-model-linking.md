---
id: TC-022
title: "Ambiguous exported declaration"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-005
    type: verifies
---

## Description

Ambiguous exported declaration. Type: Integration; priority P1. Traces: FR-005-AC-3.
Executed through the public formal linker and real IR environments in tests/linking.rs.
The complete typing/projection acceptance of IT-005 remains separate.

## Test Procedure

Use the native resolver's admitted export inventory with two distinct
owner-qualified formal declarations under one source-visible name. Each IR
environment is valid independently. Preserve exact import/source bindings,
request native resolution, then reorder the candidates and repeat.

## Expected Results

Each request returns ambiguous_declaration with the authored reference locus and both conflicting declaration loci. Neither first nor last candidate wins. No LinkedPackage is returned. The fixture must reach the documented ambiguity boundary; rejection by an earlier malformed-input check does not count.
