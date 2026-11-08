---
id: TC-1400
title: "arch-lint confines unsafe code to qsl_jit::exec, and the safe wrapper upholds its contract"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-568
    type: verifies
---
# TC-1400: arch-lint confines unsafe code to qsl_jit::exec, and the safe wrapper upholds its contract

## Description

Verify the module-list check and the safe wrapper's invariants.

Scope: FR-568-AC-1 to FR-568-AC-4.

## Test Procedure

1. Run `make arch-lint-unsafe` on the workspace and read its allowed module
   list. Read the prerequisites of `make arch-lint` and `make ci`.
2. Run the `unsafe-confinement` check over seven fixture workspaces, one per
   FR-568-AC-1 violation, and over one whose package forbids `unsafe_code`
   only by `#![forbid(unsafe_code)]` at its crate root.
3. Build the `compile_fail` doctest that holds a callable entry past the
   `PreparedJit`'s drop, and its control.
4. Through a test hook, record one entry with a wrong signature and call it;
   call a test entry that returns a status outside the closed status type.
5. Run the FR-561-AC-3 call-target check and the FR-563 parity gate over the
   Cranelift JIT at the default seed, counting the calls that enter
   generated code and the calls that pass through `qsl_jit::exec`.

Tag the tests `#[trace("TC-1400", "<AC id>")]`.

## Expected Results

- Step 1: the check passes; the list is exactly `(qsl-jit, exec)`; both
  targets list `arch-lint-unsafe`.
- Step 2: each violation fixture fails, naming its package, file and item;
  the crate-root `forbid` fixture passes.
- Step 3: the doctest fails to compile; the control compiles.
- Step 4: the wrongly recorded entry returns `Fault` without being called;
  the out-of-range status yields `Fault` and no value.
- Step 5: both pass, and the two counts are equal.
