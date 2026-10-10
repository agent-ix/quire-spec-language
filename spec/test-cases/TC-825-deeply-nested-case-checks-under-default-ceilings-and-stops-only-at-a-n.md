---
id: TC-825
title: "Deeply nested case checks under default ceilings and stops only at a named ceiling"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-318
    type: verifies
---
# TC-825: Deeply nested case checks under default ceilings and stops only at a named ceiling

## Description

Scope: FR-318-AC-6.

## Test Procedure

1. Generate a function whose body nests `case` 1,000 levels (each arm body
   a further `case`); check it under the default `CheckingLimits`.
2. Check it with the node ceiling one below the units it needs.
3. Check it with that ceiling raised by one.

Tag each test `#[trace("TC-825", "<AC id>")]`.

## Expected Results

- Step 1: checks, with no host stack overflow.
- Step 2: `StageFailure::Limit`, kind node count, the configured bound, the
  `CheckingLimits` field to raise, and the region of the node whose charge
  was denied.
- Step 3: checks.
- No outcome names a nesting-depth limit.

