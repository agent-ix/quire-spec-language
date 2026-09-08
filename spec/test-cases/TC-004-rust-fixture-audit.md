---
id: TC-004
title: "Model checkpoint identity and producer pin"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-012
    type: verifies
---

## Description

Model checkpoint identity and producer pin. Scope: FR-012-AC-4, FR-012-AC-8. Type: Integration; priority P1.

## Test Procedure

Run model-bytes over real repository fixtures, then change an isolated artifact and the producer revision independently.

## Expected Results

Five digests pass only for selected bytes/pin; mutations refuse; original bytes remain unchanged.
