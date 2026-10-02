---
id: TC-779
title: "Each non-interpreter backend matches the interpreter over the conformance corpus and generated programs"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-295
    type: verifies
---
# TC-779: Each non-interpreter backend matches the interpreter over the conformance corpus and generated programs

## Description

The parity gate: run each non-interpreter execution backend against the interpreter, and show the gate catches a deliberately wrong backend.

Scope: FR-295-AC-1.

## Test Procedure

1. For each linked non-interpreter backend, run every conformance-corpus program and 10,000 property-generated programs with generated arguments on it and on the interpreter, comparing outcome, typed cause, `Location` and loss records.
2. Run the gate with a test backend that rounds one decimal operation.

Tag the tests `#[trace("TC-779", "<AC id>")]`.

## Expected Results

- Step 1: every comparison is equal and the gate passes.
- Step 2: the gate fails, naming the program and both outcomes.
