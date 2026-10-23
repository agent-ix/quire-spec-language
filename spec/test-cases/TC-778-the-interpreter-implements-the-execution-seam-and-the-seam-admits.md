---
id: TC-778
title: "The interpreter implements the execution seam, and the seam admits checked input only"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-294
    type: verifies
---
# TC-778: The interpreter implements the execution seam, and the seam admits checked input only

## Description

Verify the `ExecutionBackend` seam with the interpreter and a test backend.

Scope: FR-294-AC-1 to FR-294-AC-5.

## Test Procedure

1. Prepare the interpreter backend over FR-100-AC-4's unit with every function selected and call each AC-4 case; call `CheckedPackage::call` with the same inputs. Evaluate FR-109's ConfigVersion clause through `evaluate` and through `CheckedPackage::evaluate`.
2. Compile the `compile_fail` doctests that pass package bytes, a `ParsedSource` and an unchecked expression to `prepare`, and file bytes to AOT's `generate` step, beside compiling controls.
3. Prepare a test backend standing in for `jit` that cannot run the third of three selected entries; read `unsupported`, and call all three entries.
4. Call `prepare` and `call` with a handle already cancelled.
5. Read `PrepareLimits::default()`. Prepare a test backend that charges one compile work unit per entry over three entries with `compile_work_units` 2, then with the default; prepare the interpreter over the same entries with both fields 0.

Tag the tests `#[trace("TC-778", "<AC id>")]`.

## Expected Results

- Step 1: each pair of results is equal.
- Step 2: the doctests fail to compile and the controls compile.
- Step 3: `prepare` returns `Ok`; `unsupported` lists the third entry alone with `UnsupportedEntry { backend: Jit, cause: Untranslatable { location } }`; the first two calls return their interpreter results and the third returns `CallFailure::Unsupported` with the same `UnsupportedEntry`.
- Step 4: `PrepareFailure::Cancelled` and `CallFailure::Cancelled`.
- Step 5: defaults 16777216 and 67108864; `PrepareFailure::Limit` naming `prepare.compile_work_units`, configured value 2, counter 3; then `Ok`; the interpreter returns `Ok`.
