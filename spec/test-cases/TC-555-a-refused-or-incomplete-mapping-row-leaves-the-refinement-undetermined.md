---
id: TC-555
title: "A refused or incomplete mapping row or argument leaves the refinement undetermined, and an undefined argument refutes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-140
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-141
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-144
    type: verifies
---
# TC-555: A refused or incomplete mapping row or argument leaves the refinement undetermined, and an undefined argument refutes

## Description

Verify both halves of the undefined ruling for field rows and step-row
arguments: a row or argument whose evaluation is incomplete settles
`inconclusive`, `MappingUndetermined`, and a step-row argument that
evaluates undefined refutes with an `Undefined` refinement counterexample.

Scope: FR-140-AC-5, FR-141-AC-7, FR-144-AC-6.

## Test Procedure

Fixture: FR-139's `RingIsQueue`, and its variant with the row
`put -> enq(self.ring, 2 / self.size)`.

1. Run `map_state` at `head = 1`, `size = 2` with a per-evaluation meter
   budget of zero.
2. Over the variant, run `check_step` on `put(1)` from the empty queue;
   over the original rows, run it with a meter budget of zero for the
   argument `self.ring`.
3. Settle the refinement item for step 1's run and for the variant, the
   variant after FR-145 replay.

Tag the tests `#[trace("TC-555", "<AC id>")]`.

## Expected Results

- Step 1: `MappingFailure::Undetermined(MappingUndetermined)` naming the
  `items` row and `r`; no partial state.
- Step 2: `Undefined` naming the argument, cause `division-by-zero`, and
  the engine's counterexample carries `RefinementFailure::Undefined{position:
  1, row: put}`; then `Undetermined(MappingUndetermined)`.
- Step 3: `inconclusive`, `unsettled`, `Inconclusive(MappingUndetermined)`;
  `refuted`, `decisive-counterexample`, cause `UndefinedEvaluation`.
