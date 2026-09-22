---
id: TC-293
title: "The evaluator resolves a Value::Population identity through the recorded correspondence, not a carried payload"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-089
    type: verifies
---
# TC-293: The evaluator resolves a Value::Population identity through the recorded correspondence, not a carried payload

## Description

Verify FR-089-AC-3: given a `Value::Population(population_id)` whose
`population_id` was minted for a binding admitted earlier in the same
evaluation, evaluating an expression that consumes it resolves that same
`PopulationBinding` by lookup in the recorded correspondence. Scope:
FR-089-AC-3.

Known gap: the evaluator currently matches `Value::Population(binding)` and
reads `binding` directly (`src/value/expression/evaluate.rs:921,934`); there
is no `PopulationId` and no recorded correspondence to resolve through. This
test case fails against current code. Remaining work: QSL-131 Slice B.

Catches an implementation that resolves by re-deriving a binding from the
identity's own bytes (defeating the point of an opaque digest — a real
digest is not reversible) instead of looking it up in the correspondence
`model` actually recorded, and an implementation that silently falls back to
the most-recently-admitted binding regardless of which identity the `Value`
names (invisible to a test with only one live population in scope).

## Test Procedure

1. Admit two distinct `PopulationBinding`s, `B1` (against `population_key`
   `K1`) and `B2` (against `population_key` `K2`), within one evaluation,
   producing `Value::Population(id1)` and `Value::Population(id2)`
   respectively.
2. Evaluate an `allInstances(p)`-shaped expression bound to `id1`.
3. Evaluate the same expression shape bound to `id2`.

## Expected Results

Step 2's result reflects `B1`'s admitted members (not `B2`'s). Step 3's
result reflects `B2`'s admitted members (not `B1`'s). A mutant that resolves
every `PopulationId` to whichever binding was admitted last fails step 2 (it
would return `B2`'s members instead of `B1`'s).
