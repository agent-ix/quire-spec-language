---
id: TC-297
title: "Kernel admits, plan_pairs and compare_keys refuse a population pair"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-089
    type: verifies
---
# TC-297: Kernel admits, plan_pairs and compare_keys refuse a population pair

## Description

Verify FR-089-AC-6: the `quire-exact` kernel, a leaf with no access to
`model`'s `PopulationId` -> `PopulationBinding` correspondence, refuses every
population pair. `ValueType::admits` returns `false` for any
`(ValueType::Population(_), Value::Population(_))` pair,
`equality::plan_pairs` refuses two `Value::Population` operands with
`Refusal::CheckedInvariant`, and `key::compare_keys` yields `None` for two
`Value::Population` values. The declared-maximum comparison is the QSL
layer's (FR-089-AC-5, TC-295). Scope: FR-089-AC-6.

Catches a kernel `admits` that admits every `Value::Population(_)` under any
`ValueType::Population(_)` (silently widening the type with no declared
maximum checked), a `plan_pairs` that compares two population identities
structurally as if they were equality operands, and a `compare_keys` that
orders population identities by digest as if they were key participants.

## Test Procedure

1. Check `ValueType::Population(5).admits(&Value::Population(id))` for a
   `PopulationId` built with `PopulationId::from_digest`.
2. Call `plan_pairs` on two `Value::Population` operands with distinct
   `PopulationId`s.
3. Call `compare_keys` on two `Value::Population` values with distinct
   `PopulationId`s.

## Expected Results

Step 1 returns `false`. Step 2 returns `Err(Refusal::CheckedInvariant)`.
Step 3 returns `None`.

## Metadata

- Priority: P1
- Target Integration: `quire-exact/src/value.rs`, `quire-exact/src/equality.rs`,
  `quire-exact/src/key.rs`
- Automation: Automated Rust unit tests
  `value::tests::admits_refuses_a_population_pair` (step 1),
  `equality::tests::plan_pairs_refuses_a_population_pair` (step 2) and
  `key::tests::compare_keys_yields_no_key_for_a_population_pair` (step 3)

## Dependencies

**Upstream:** [FR-089](../functional/FR-089-carry-population-identity-across-the-kernel-boundary.md).
**Downstream:** none.
