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

Property, priority P1. Verifies FR-015-AC-6. Qualified; SR-085
records the actual public-API controls, local gates and bounded claim. Tests use
the source-derived Rust producer and successfully constructed actual IR inputs.

## Test Procedure

For each ModelLimits dimension, construct a small valid exact-limit model and lower the limit below the required count/bytes, including zero. Exercise link_native model/import/clause/node/per-artifact/aggregate byte limits with the same profile. Raise each caller value above its published default and run a bounded input larger than that default.

## Expected Results

An exactly fitting valid input succeeds; the next required operation returns resource_exhausted and no model/package. Zero never disables a limit, and an input above a default succeeds once that limit is raised to fit it. Emitted bytes are charged before append. Tests use bounded fixture families, one job and one test thread; no uncontrolled concurrency or massive Cartesian product is required.
