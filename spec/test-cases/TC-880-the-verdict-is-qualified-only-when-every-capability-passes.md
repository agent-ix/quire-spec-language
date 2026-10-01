---
id: TC-880
title: "The verdict is complete-V1 qualified only when every in-scope capability passes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-353
    type: verifies
---
# TC-880: The verdict is complete-V1 qualified only when every in-scope capability passes

## Description

Verify the qualified verdict, its exit status, and that one failing or uncovered capability makes the run not qualified.

Scope: FR-353-AC-1, FR-353-AC-2.

## Test Procedure

1. Run a fixture corpus whose in-scope vectors all pass, over the default scope.
2. Change one refusal vector's expected cause and run.
3. Restore it, remove every vector of one capability and run.

Tag the tests `#[trace("TC-880", "FR-353-AC-n")]`.

## Expected Results

- Step 1: `complete-v1: qualified`, exit 0.
- Step 2: `not-qualified` naming that capability `failed`, exit 1.
- Step 3: `not-qualified` naming that capability `uncovered`, exit 1.
