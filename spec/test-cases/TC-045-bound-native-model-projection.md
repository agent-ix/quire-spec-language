---
id: TC-045
title: "Bound model construction and native linkage"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-015
    type: verifies
---
# TC-045: Bound model construction and native linkage

## Description

Property, priority P1. Verifies FR-015-AC-6. Qualified at 0cd679c; SR-085
records the actual public-API controls, local gates and bounded claim. Tests use
the source-derived Rust producer and successfully constructed actual IR inputs.

## Test Procedure

For each ModelLimits dimension, construct a small valid exact-limit model and lower the limit below the required count/depth/bytes, including zero. Exercise link_native model/import/clause/node/depth/per-artifact/aggregate byte limits with the same profile. Attempt to raise each caller value above its documented hard ceiling using bounded inputs that reach the hard boundary.

## Expected Results

An exactly fitting valid input succeeds; the next required operation returns resource_exhausted and no model/package. Zero never disables a limit and caller values cannot elevate hard ceilings. Emitted bytes are charged before append. Tests use bounded fixture families, one job and one test thread; no uncontrolled concurrency or massive Cartesian product is required.
