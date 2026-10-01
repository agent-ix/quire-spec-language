---
id: TC-604
title: "EN-1 checks a step relation over every tuple of reachable transitions"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-179
    type: verifies
---
# TC-604: EN-1 checks a step relation over every tuple of reachable transitions

## Description

Verify HP-1 model-claim checking: proof over every tuple, a nondeterministic post-state refuted with a `StepTuple`, the tuple budget, the depth bound and determinism.

Scope: FR-179-AC-1 to FR-179-AC-3.

## Test Procedure

Fixtures: ADR-023 §8's secure vault; its nondeterministic `step` variant (FR-179-AC-2).

1. `Det` over the secure vault.
2. `Det` over the nondeterministic variant.
3. Step 1 with `max_relation_tuples` 10 and with `max_depth` 1; step 2 twice.

Tag the tests `#[trace("TC-604", "FR-179-AC-n")]`.

## Expected Results

- Step 1: `Holds{Exhaustive}` after 64 tuples.
- Step 2: `Violated`, `StepTuple`, equal pre-states and inputs, unequal post-states, each with its prefix.
- Step 3: `Stopped(ResourceExhausted, MaxRelationTuples)` after 10 tuples; 16 tuples and `BoundReached{depth: 1}`; byte-equal counterexamples.
