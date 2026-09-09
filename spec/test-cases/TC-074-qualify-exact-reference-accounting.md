---
id: TC-074
title: "Qualify exact reference accounting"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-008
    type: verifies
---
# TC-074: Qualify exact reference accounting

## Description

Integration, priority P1. Verifies FR-008-AC-4, FR-008-AC-16. Planned until actual Rust
execution. Setup uses qualified public model/checker APIs; unrelated setup
failure cannot stand in for the intended phase's outcome.

## Test Procedure

Run every explicit vector in docs/native-runtime-evaluation.md, with independent expected expression/graph totals. Test exact completion, one below each nonzero dimension, zero and raised options. Re-run with source grouping and repeated reaches calls to check independent visited sets and total graph accounting.

## Expected Results

Exact vectors match both counts and truth/events. Five-step let fails at four; true-implies-false at two has a completed antecedent but no consequent entry or Boolean. False-implies-true completes at two. Graph work is separate from source, validation and deep-comparison counters.
