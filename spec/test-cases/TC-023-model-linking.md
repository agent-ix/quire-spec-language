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
Executed through the public formal linker and real IR environments in tests/linking.rs.
The complete typing/projection acceptance of IT-005 remains separate.

## Test Procedure

From TC-020's complete qualified inputs, select a dependency whose content
differs from the content the request's package identity names, while retaining
the original expected binding. Use the native resolver's admitted stale control
and request linking; no Filament package reader is required.

## Expected Results

Linkage refuses `stale_dependency`/`content-mismatch`, naming the request's package identity and the recomputed one, as FR-098-AC-3 states. It does not silently refresh or substitute a closure. No LinkedPackage is returned.
