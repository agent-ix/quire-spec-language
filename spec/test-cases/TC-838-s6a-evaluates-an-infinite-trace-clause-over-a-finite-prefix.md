---
id: TC-838
title: "S6a evaluates an infinite-trace clause three-valued over a finite prefix"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-328
    type: verifies
---
# TC-838: S6a evaluates an infinite-trace clause three-valued over a finite prefix

## Description

Verify that a finite prefix under infinite-trace yields a violation only
for a bad prefix of a safety-fragment formula, pending otherwise, and never
a proof.

Scope: FR-328-AC-1 to FR-328-AC-4.

## Test Procedure

Use the `Counter` prefix with `c.value` 0, 1, 2 under infinite-trace.

1. Evaluate the four clauses of FR-328-AC-1.
2. Evaluate the two interval clauses of FR-328-AC-2.
3. Evaluate `always (holds(c.value = 0) implies once[1,1] holds(c.value = 9))`.
4. Map every result of steps 1 to 3 to its O-16 category.

Tag the tests `#[trace("TC-838", "FR-328-AC-n")]`.

## Expected Results

- Step 1: `false` at 0; `Pending`; `Pending`; `Pending`.
- Step 2: `false` at 0; `Pending`.
- Step 3: `false` at 0.
- Step 4: violation or inconclusive only; no `tested`, no `proved`.
