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
Planned, not executed. Requires the qualified inputs and actual native APIs
specified by IT-005; a setup refusal is not an observed application outcome.

## Test Procedure

Start with TC-020's qualified local inputs, then remove the selected model artifact from the supplied resolver inventory. Keep all other inputs and expected source occurrence coordinates unchanged; request linking.

## Expected Results

Linkage returns missing_import at the import occurrence. No LinkedPackage or logical value is produced. An absent adapter/test setup cannot be used as the missing-import control.
