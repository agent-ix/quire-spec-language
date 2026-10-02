---
id: TC-033
title: "Enforce native linking resource ceilings"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-013
    type: verifies
---

## Description

Enforce native linking resource ceilings. Integration/property controls, priority P1. Traces: FR-013-AC-5.
Authored before implementation; now executed with the public link API and actual IR constructors.

## Test Procedure

Exercise model/import/clause/node and per-model/aggregate canonical byte budgets with real IR environments. For each active dimension use its exact required count, one lower, zero and an input above its published default with the limit raised to fit it. Include a deep syntax chain that links under node limits sized for it. Run the same healthy request after each refused request.

## Expected Results

An exact permitted boundary succeeds where all other prerequisites hold. A needed operation above a configured limit returns resource_exhausted with is_incomplete true and no package, and the same input links once that limit is raised; zero never disables a limit. Later healthy requests are unchanged. Emitted-byte bounds are not described as allocator-capacity bounds.
