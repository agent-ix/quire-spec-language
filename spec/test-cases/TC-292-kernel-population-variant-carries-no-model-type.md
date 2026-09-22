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

Verify FR-089-AC-2: the kernel `Value::Population` variant's payload type is
`PopulationId`, and no file under `quire-exact/src/` imports `PopulationBinding`
or any other `crate::model::population` type. This is a source-scan test, in
the style of FR-078-AC-1/AC-2 (TC-201): the obligation is a static fact about
what the kernel crate depends on, not a runtime behavior.

Verified by `quire_exact::value::tests::tc_292_population_variant_carries_only_the_opaque_identity`
(QSL-131 Slice B): it constructs a `Value::Population(PopulationId)` and
destructures it back, asserting the extracted payload is exactly the
`PopulationId` given, never anything else. Steps 2 and 3 are confirmed by
inspection rather than an automated source scan, per the team's testing
policy (functionality and behavior, not module placement): `quire-exact`'s
own `Cargo.toml` declares zero dependencies beyond `num-bigint`,
`num-integer`, `num-traits`, `thiserror` and `unicode-normalization` -- no
path or crate dependency on `quire_spec_language` (or any crate exposing
`model::population`) exists for a "scan" to find in the first place; the
crate-DAG direction (ADR-011 §6.1, §7.1: every edge points *into* this
crate) makes such an import a compile error, not a runtime possibility, so
this obligation is a structural property of the dependency graph rather
than a behavior this test suite exercises.

Catches an implementation that "fixes" the gap by moving `PopulationBinding`
into `quire-exact` (ADR-011 §6.1's forbidden K-leaf violation) instead of
adding an opaque identity, which a test that only checks "does `Value` admit
a population value" cannot distinguish from the correct shape.

## Test Procedure

1. Locate the `Value` enum definition in `quire-exact/src/value.rs` and
   inspect its `Population` variant's payload type.
2. Scan every source file under `quire-exact/src/` for an import of
   `crate::model::population::*`, `model::population::*`, or a direct
   reference to `PopulationBinding` by name.
3. Confirm `quire-exact`'s `Cargo.toml` declares no path or crate dependency
   on the `quire_spec_language` crate (or any crate exposing
   `model::population`).

## Expected Results

Step 1 shows `Value::Population(PopulationId)`, never
`Value::Population(Arc<PopulationBinding>)` or an inline reproduction of
`PopulationBinding`'s fields. Step 2 finds zero matches. Step 3 confirms no
such dependency exists. A mutant that adds `Population(Arc<PopulationBinding>)`
back to kernel `Value`, or that inlines `PopulationBinding`'s fields directly
into a kernel variant, fails step 1 or step 2.
