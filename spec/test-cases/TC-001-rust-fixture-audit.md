---
id: TC-001
title: "Self-contained identity negative controls"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-012
    type: verifies
---

## Description

Self-contained identity negative controls. Scope: FR-012-AC-1. Type: Unit; priority P1.

## Test Procedure

Run the actual audit control function for three distinct artifact roles.

## Expected Results

All six stale/recomputed-digest controls and a duplicate-key control reject with the expected code.
