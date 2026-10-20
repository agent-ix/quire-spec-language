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
# TC-555: A refused or incomplete mapping row or argument leaves the refinement undetermined, and an undefined argument refutes

## Description

Verify both halves of the undefined ruling for field rows and step-row
arguments: a row or argument whose evaluation is incomplete settles
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
5. Run `RingIsQueue`'s `take` with the abstract system's per-evaluation meter budget at zero, and an abstract result domain of 65,537 values (FR-141-AC-9).
6. Run `check_step` with an evaluator that fixes the receiver, an argument and a post-state row to each outcome combination of FR-141-AC-10.

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
- Step 5: `StepVerdict::Stopped` with `EvaluationMeter`, then `MaxCandidates`; FR-142 returns `Stopped{ResourceExhausted, …}` and the item settles `incomplete`, never `failed`.
- Step 6: the verdicts of FR-141-AC-10; an undefined read always outranks an undetermined one.
