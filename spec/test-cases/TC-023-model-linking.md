---
id: TC-023
title: "Stale package closure"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-005
    type: verifies
---

## Description

Stale package closure. Type: Integration; priority P1. Traces: FR-005-AC-4.
Planned, not executed. Requires the qualified inputs and actual native APIs
specified by IT-005; a setup refusal is not an observed application outcome.

## Test Procedure

From TC-020's complete qualified inputs, select a dependency revision/content
different from the pinned native import closure while retaining the original
expected binding. Use the native resolver's admitted stale control and request
linking; no Filament package reader is required.

## Expected Results

Linkage returns stale_dependency with the selected dependency identity and native import locus. It does not silently refresh or substitute a closure. No LinkedPackage is returned.
