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

Scope: FR-294-AC-1 to FR-294-AC-4.

## Test Procedure

1. Prepare the interpreter backend over FR-100-AC-4's unit with every function selected and call each AC-4 case; call `CheckedPackage::call` with the same inputs. Evaluate FR-109's ConfigVersion clause through `evaluate` and through `CheckedPackage::evaluate`.
2. Compile the `compile_fail` doctests that pass package bytes, a `ParsedSource` and an unchecked expression to `prepare`, and file bytes to AOT's `generate` step, beside compiling controls.
3. Prepare a test backend that cannot run one of three selected entries, and call the two it prepared.
4. Call `prepare` and `call` with a handle already cancelled.

Tag the tests `#[trace("TC-778", "<AC id>")]`.

## Expected Results

- Step 1: each pair of results is equal.
- Step 2: the doctests fail to compile and the controls compile.
- Step 3: the prepared value holds two entries and the failure names the third `Unsupported`; the two calls return their interpreter results.
- Step 4: `PrepareFailure::Cancelled` and `CallFailure::Cancelled`.
