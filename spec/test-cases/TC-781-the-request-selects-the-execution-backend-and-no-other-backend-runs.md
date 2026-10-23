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

Scope: FR-296-AC-1 to FR-296-AC-4.

## Test Procedure

1. Call `execute` on `tests/fixtures/spine-compile.native`'s `seven` with a request that names no backend.
2. Call `execute` choosing `jit` with `SuppliedBackends::default()`.
3. With a test JIT that lists one entry `unsupported` with cause `Untranslatable` and an interpreter instrumented to count calls, call `execute` of that entry choosing `jit`.
4. Read `ExecutionBackendChoice::default()`. Supply a test JIT that counts `prepare` calls as `SuppliedBackends { aot: None, jit: Some(&jit) }` and call `execute` of `seven` choosing `jit`, with the interpreter instrumented to count calls.

Tag the tests `#[trace("TC-781", "<AC id>")]`.

## Expected Results

- Step 1: the interpreter runs and the call completes with 7.
- Step 2: `CallFailure::Unsupported(UnsupportedEntry { backend: Jit, cause: NotSupplied })`, exit 21, before any stage.
- Step 3: `CallFailure::Unsupported` with the JIT's `UnsupportedEntry`, exit 21, and the interpreter's call count is zero.
- Step 4: `Interpreter`; the JIT's `prepare` count is 1, the interpreter's call count is zero, and the call completes with 7.
