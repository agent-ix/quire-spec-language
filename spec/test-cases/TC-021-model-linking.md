---
id: TC-021
title: "Missing selected import"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-005
    type: verifies
---

## Description

Missing selected import. Type: Integration; priority P1. Traces: FR-005-AC-2.
Executed through the public formal linker and real IR environments in tests/linking.rs.
The complete typing/projection acceptance of IT-005 remains separate.

## Test Procedure

Start with TC-020's qualified local inputs, then remove the selected model artifact from the supplied resolver inventory. Keep all other inputs and expected source occurrence coordinates unchanged; request linking.

## Expected Results

Linkage returns missing_import at the import occurrence. No LinkedPackage or logical value is produced. An absent native implementation/test setup cannot be used as the missing-import control.
