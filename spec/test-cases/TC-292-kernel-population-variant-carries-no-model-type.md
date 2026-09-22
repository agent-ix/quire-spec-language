---
id: TC-292
title: "Kernel Value::Population carries PopulationId only, with no model dependency"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-089
    type: verifies
---
# TC-292: Kernel Value::Population carries PopulationId only, with no model dependency

## Description

Verify FR-089-AC-2 by inspection: the kernel `Value::Population` variant's
payload type is `PopulationId`, and no file under `quire-exact/src/` can
import `PopulationBinding` or any other `crate::model::population` type,
because `quire-exact`'s own `Cargo.toml` declares no path or crate
dependency on `quire_spec_language` (or any crate exposing
`model::population`) at all. This is a structural fact about the crate-DAG
direction (ADR-011 §6.1, §7.1: every edge points *into* `quire-exact`,
never out of it), not a runtime behavior, so it is recorded as an
inspection, not a test.

Catches an implementation that "fixes" the gap by moving `PopulationBinding`
into `quire-exact` (ADR-011 §6.1's forbidden K-leaf violation) instead of
adding an opaque identity, which a test that only checks "does `Value` admit
a population value" cannot distinguish from the correct shape.

## Inspection Procedure

1. Locate the `Value` enum definition in `quire-exact/src/value.rs` and
   inspect its `Population` variant's payload type.
2. Locate `quire-exact/Cargo.toml`'s `[dependencies]` table and confirm it
   names no path or crate dependency on `quire_spec_language` or any crate
   exposing `model::population`.

## Expected Results

Step 1 shows `Value::Population(PopulationId)`, never
`Value::Population(Arc<PopulationBinding>)` or an inline reproduction of
`PopulationBinding`'s fields. Step 2 confirms `quire-exact`'s only
dependencies are `num-bigint`, `num-integer`, `num-traits`, `thiserror` and
`unicode-normalization` -- none of which exposes `model::population` -- so
the crate-DAG direction alone makes a `PopulationBinding` import a compile
error, not a possibility this inspection needs to search source text for.
