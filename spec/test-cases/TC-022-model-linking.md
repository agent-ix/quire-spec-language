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
Planned, not executed. Requires the qualified inputs and actual native APIs
specified by IT-005; a setup refusal is not an observed application outcome.

## Test Procedure

Use the shared adapter owner's admitted ambiguous-export fixture with two different stable type IDs under ConfigVersion. Preserve selected package/version/source correspondence; request native resolution. Reorder the candidates and repeat.

## Expected Results

Each request returns ambiguous_declaration with the authored reference locus and both conflicting declaration loci. Neither first nor last candidate wins. No LinkedPackage is returned. The fixture must reach the documented ambiguity boundary; rejection by an earlier malformed-input check does not count.
