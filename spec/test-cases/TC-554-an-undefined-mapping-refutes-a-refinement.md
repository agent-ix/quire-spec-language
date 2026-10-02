---
id: TC-554
title: "An undefined mapping row or history update refutes a refinement and replays"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-138
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-141
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-142
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-145
    type: verifies
---
# TC-554: An undefined mapping row or history update refutes a refinement and replays

## Description

Verify that a history update that evaluates undefined refutes the
refinement with an `UndefinedEvaluation` counterexample ending at that
step, that an update whose value leaves its declared type stays
`MappingUndetermined`, and that replay reproduces the undefined value.
TC-555 covers refused and incomplete rows and arguments.

Scope: FR-138-AC-5, FR-141-AC-6, FR-142-AC-8, FR-145-AC-5.

## Test Procedure

Fixture: FR-138's `RegisterHistory` with the `inv` update `1 / x`, and its
variant with the `writes` update.

1. Run `step_history` along `write(r, 1)` and along `write(r, 0)`.
2. Run `check_step` on `write(r, 0)` from the initial state; then, over the
   `writes` variant, on the second `write(r, 0)`.
3. Run `check_refinement` on both variants.
4. Replay the `inv` counterexample, then the same payload with its cause
   changed to `precondition-false`.

Tag the tests `#[trace("TC-554", "<AC id>")]`.

## Expected Results

- Step 1: `inv` 1; then `MappingFailure::Undefined` naming `inv`, `r`, the
  step and `division-by-zero`, with the post-state still the concrete
  successor.
- Step 2: `Undefined` at position 1 naming the `inv` update and `r`; then
  `Undetermined(MappingUndetermined)`.
- Step 3: `Violated`, prefix `write(r, 0)`, `RefinementFailure::Undefined{position: 1, row: inv}`, `kind: UndefinedEvaluation`;
  `Undecided(MappingUndetermined)` for the `writes` variant.
- Step 4: `reproduced-with-evaluated-witness`; then `inconclusive`,
  `ReplayParity`.
