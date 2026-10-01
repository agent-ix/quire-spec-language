---
id: TC-881
title: "A scoped run's verdict names its scope, and a refused run has no verdict"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-353
    type: verifies
---
# TC-881: A scoped run's verdict names its scope, and a refused run has no verdict

## Description

Verify that a verdict over a caller scope names that scope and is not a complete-V1 verdict, and that a run refused before any vector exits 2.

Scope: FR-353-AC-3, FR-353-AC-4.

## Test Procedure

1. Over TC-880 step 2's corpus, run scoped to every capability except the failing one.
2. Run over a corpus location that does not exist.

Tag the tests `#[trace("TC-881", "FR-353-AC-n")]`.

## Expected Results

- Step 1: `qualified` for the named scope, no `complete-v1: qualified` line, exit 0.
- Step 2: the I/O refusal, no verdict, exit 2.
