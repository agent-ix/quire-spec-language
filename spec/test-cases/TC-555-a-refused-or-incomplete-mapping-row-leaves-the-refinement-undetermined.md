---
id: TC-555
title: "A mapping row or argument leaves the refinement undetermined or refutes it, an abstract budget stop is incomplete, and reads are ordered"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-140
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-141
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-144
    type: verifies
---
# TC-555: A mapping row or argument leaves the refinement undetermined or refutes it, an abstract budget stop is incomplete, and reads are ordered

## Description

Verify the undefined ruling for field rows and step-row arguments, the
budget stop of the abstract system and the order of a step's reads. The
undefined ruling: a row or argument whose evaluation is incomplete settles
`inconclusive`, `MappingUndetermined`, and a step-row argument that
evaluates undefined refutes with an `Undefined` refinement counterexample.

Scope: FR-140-AC-5, FR-141-AC-7 to FR-141-AC-10, FR-144-AC-6.

## Test Procedure

Fixture: FR-139's `RingIsQueue`, and its variant with the function `g` and
the row `put -> enq(self.ring, g(self.index))` of FR-141-AC-7.

1. Run `map_state` at `head = 1`, `size = 2` with a per-evaluation meter
   budget of zero.
2. Over the variant, run `check_step` on `put(1)` with receiver `s0` from the
   empty queue; over the original rows, run it with a meter budget of zero
   for the argument `self.ring`.
3. Settle the refinement item for step 1's run and for the variant, the
   variant after FR-145 replay.
4. Inject a fault into `check_step`'s mapping, and pass it a swapped system.
5. Over `HeavyEnq` (FR-141-AC-9) with the per-evaluation meter at B, run `check_step` on `put(1)`, run FR-142's check, and settle the item.
6. Over `Reads` (FR-141-AC-10), run `check_step` for each choice of the receiver, arguments, post-history update and post-state row it lists, and once with a swapped system.

Tag the tests `#[trace("TC-555", "<AC id>")]`.

## Expected Results

- Step 1: `MappingFailure::Undetermined(MappingUndetermined)` naming the
  `items` row and `r`; no partial state.
- Step 2: `Undefined` naming the argument, cause `division-by-zero`, and
  the engine's counterexample carries `RefinementFailure::Undefined{position:
  1, row: put}`; then `Undetermined(MappingUndetermined)`.
- Step 3: `inconclusive`, `unsettled`, `Inconclusive(MappingUndetermined)`;
  `refuted`, `decisive-counterexample`, cause `UndefinedEvaluation`.
- Step 4: `StepVerdict::Fault` for each; the item settles `failed`.
- Step 5: `StepVerdict::Stopped{EvaluationMeter, B}`; FR-142 ends `Stopped{ResourceExhausted, {EvaluationMeter, B}}` after the concrete states expanded; the item settles with execution `resource-incomplete`, `Incomplete(LimitReached{EvaluationMeter, B, setting})`, category incomplete, never `TerminalValue::Failed`.
- Step 6: the verdicts of FR-141-AC-10, including the order between read groups (`R` before `A1`, `A1` before `P`, `A1` before `H`, `H` before `P`), the swapped system returning `Fault`.
