---
id: TC-024
title: "Failed linkage is atomic"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-005
    type: verifies
---

## Description

Failed linkage is atomic. Type: Property; priority P1. Traces: FR-005-AC-5.
Executed through the public formal linker and real IR environments in tests/linking.rs.
The complete typing/projection acceptance of IT-005 remains separate.

## Test Procedure

Generate bounded permutations of a request containing two valid imports and one
missing, ambiguous or stale import over the real native resolver and public IR
environments. Place the defective import first, middle and last; repeat each
valid/invalid family after a successful independent request.

## Expected Results

Every invalid request returns diagnostics and no LinkedPackage. Previously successful work cannot become a partial result for the invalid request. Every successful control resolves its complete selected set. No positive status is inferred from diagnostic text.
