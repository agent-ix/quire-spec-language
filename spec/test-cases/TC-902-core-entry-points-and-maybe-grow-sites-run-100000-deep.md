---
id: TC-902
title: "Core entry points and maybe_grow sites run 100,000 deep on a small stack"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-356
    type: verifies
---
# TC-902: Core entry points and maybe_grow sites run 100,000 deep on a small stack

## Description

Verify that every public entry point of the qualified core takes a
100,000-deep input on a small stack, that the core never grows the stack,
and that `qsl-walk-grow`'s `maybe_grow` is the one stack-growth path outside
it.

Scope: FR-356-AC-5, FR-356-AC-6.

## Test Procedure

Run steps 1, 3 and 4 on a thread spawned with a 512 KiB stack.

1. For each public entry point of the S0 to S4 checker, the prove path and
   the certificate checkers, run a 100,000-deep input (a sum, an `else if`
   chain, nested `let`s or a nested `Option` type, as the entry point takes)
   under limits raised to fit.
2. List the dependencies of every crate in the core, on its own and in the
   resolved workspace build, and every call of `stacker` in the workspace.
   Run arch-lint's direction check over a manifest set where a core crate
   depends on `qsl-walk-grow`.
3. Call `maybe_grow` on a 100,000-deep native recursion.
4. Run each call site of `maybe_grow` outside the core with a 100,000-deep
   input.
5. Build `maybe_grow` under `cfg(kani)` and run its closure once.

## Expected Results

- Step 1: each entry point returns its result.
- Step 2: no core crate depends on `stacker` or `qsl-walk-grow` either way,
  the only `stacker` call is inside `maybe_grow`, and the check refuses the
  core crate's edge to `qsl-walk-grow`.
- Steps 3 and 4: each recursion completes.
- Step 5: `maybe_grow` runs its closure as a plain call.

