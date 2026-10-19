---
id: TC-555
title: "A refused or incomplete mapping row or argument leaves the refinement undetermined, and an undefined update refutes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-140
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-141
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-144
    type: verifies
---
# TC-555: A refused or incomplete mapping row or argument leaves the refinement undetermined, and an undefined update refutes

## Description

Verify both halves of the undefined ruling: a mapping row or step-row
argument whose evaluation is incomplete settles `inconclusive`,
`MappingUndetermined`, and an undefined history update refutes with an
`Undefined` refinement counterexample. A step-row argument that evaluates
undefined has no typeable source yet (FR-141-AC-7), so it has no vector.

Scope: FR-140-AC-5, FR-141-AC-7, FR-144-AC-6.

## Test Procedure

Fixtures: FR-139's `RingIsQueue`; FR-138's `RegisterHistory` with the `inv`
update.

1. Run `map_state` at `head = 1`, `size = 2` with a per-evaluation meter
   budget of zero.
2. Over `RingIsQueue`'s original rows, run `check_step` on `put(1)` from the
   empty queue with a meter budget of zero for the argument `self.ring`.
3. Settle the refinement item for step 1's run, and for `RegisterHistory`
   with the `inv` update after FR-145 replay.

Tag the tests `#[trace("TC-555", "<AC id>")]`.

## Expected Results

- Step 1: `MappingFailure::Undetermined(MappingUndetermined)` naming the
  `items` row and `r`; no partial state.
- Step 2: `Undetermined(MappingUndetermined)`.
- Step 3: `inconclusive`, `unsettled`, `Inconclusive(MappingUndetermined)`;
  `refuted`, `decisive-counterexample`, cause `UndefinedEvaluation`.
