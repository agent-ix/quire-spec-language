---
id: TC-756
title: "No lifecycle operation panics on arbitrary input"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-275
    type: verifies
---
# TC-756: No lifecycle operation panics on arbitrary input

## Description

Property-test the public boundary of `parse`, `check`, `monitor` and `replay` over arbitrary inputs.

Scope: FR-275-AC-4.

## Test Procedure

1. Generate 10,000 arbitrary byte strings and call `parse` on each, then `check` on every `ParsedSource` that results.
2. Generate 10,000 arbitrary trace documents and call `monitor` with them over TC-765's package.
3. Generate 10,000 arbitrary replay request byte strings and call `replay` on each.

Tag the tests `#[trace("TC-756", "<AC id>")]`.

## Expected Results

- Step 1: every call returns `Ok` or a typed `StageFailure`, and none panics.
- Step 2: every call returns `Ok` or a typed `StageFailure`, and none panics.
- Step 3: every call returns `Ok` or a typed `StageFailure`, and none panics.
