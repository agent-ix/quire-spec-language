---
id: TC-587
title: "TransitionSystem hooks offer reductions, the product delegates them, and simulation stays unreduced"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-162
    type: verifies
---
# TC-587: TransitionSystem hooks offer reductions, the product delegates them, and simulation stays unreduced

## Description

Verify the three hooks on `ModelSystem` and their absence on a test system, product delegation, combined symmetry and partial-order reduction, limits, and that simulation never calls the hooks.

Scope: FR-162-AC-1 to FR-162-AC-4.

## Test Procedure

Fixtures: ADR-021 §7.1, §7.2 (annotated) and §7.3 subjects; a test `TransitionSystem` without hooks.

1. Call each hook on `ModelSystem`; request each reduction over the test system; run `explore_model` over §7.1's annotated subject.
2. Call the product system's hooks; inspect retained edges with and without symmetry; read the stutter edge's footprint over FR-126-AC-3's subject.
3. Run FR-162-AC-3's combined request.
4. Run §7.3 constrained with `max_states` 5; run step 3 twice.

Tag the tests `#[trace("TC-587", "FR-162-AC-n")]`.

## Expected Results

- Step 1: `Some` from each; V-8 for each; 27 states with findings.
- Step 2: model-only canonicalisation, equal footprints, canonical targets with mapping permutations, identity permutations without symmetry, an empty stutter footprint.
- Step 3: the seven listed states and `Proved{Reduced{[Symmetry{…}, PartialOrder{BreadthFirstRevisit}]}}`.
- Step 4: `Stopped(ResourceExhausted, MaxStates)` counting boundary states; equal edge lists.
