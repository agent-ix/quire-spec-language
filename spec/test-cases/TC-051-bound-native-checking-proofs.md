---
id: TC-051
title: "Bound constraint checking and shared proof expansion"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-016
    type: verifies
---
# TC-051: Bound constraint checking and shared proof expansion

## Description

Property, priority P1. Verifies FR-016-AC-8. Planned until real execution; setup
must use the qualified source-derived Rust producer and actual public IR APIs.

## Test Procedure

Generate bounded native clause families and repeated Boolean let aliases. Set each CheckLimits dimension to an observed exact requirement and one below, including zero; attempt hard-limit elevation. Account for each graph/materialized node before insertion, including guard/witness wrappers and shared graph re-expansion. Repeat a prior successful check with a deliberately smaller budget.

## Expected Results

Exact budgets complete where the proof is supported; the next required operation returns resource_exhausted with no package. Shared aliases cannot allocate an exponential IR tree before charging. Native/expanded proof depth stays within 64, below IR's extra-thread threshold. Prior success cannot cause a cached partial result to escape a later refusal.

