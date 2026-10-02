---
id: TC-781
title: "The request selects the execution backend and no other backend runs in its place"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-296
    type: verifies
---
# TC-781: The request selects the execution backend and no other backend runs in its place

## Description

Verify backend selection.

Scope: FR-296-AC-1 to FR-296-AC-3.

## Test Procedure

1. Call `execute` on `tests/fixtures/spine-compile.native`'s `seven` with a request that names no backend.
2. Call `execute` choosing `jit` with only the interpreter supplied.
3. With a test JIT that settles one entry `Unsupported` and an interpreter instrumented to count calls, call `execute` of that entry choosing `jit`.

Tag the tests `#[trace("TC-781", "<AC id>")]`.

## Expected Results

- Step 1: the interpreter runs and the call completes with 7.
- Step 2: refused as unsupported, naming `jit`, exit 21, before any stage.
- Step 3: the unsupported outcome returns, and the interpreter's call count is zero.
