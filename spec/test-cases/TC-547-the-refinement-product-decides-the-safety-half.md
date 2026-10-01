---
id: TC-547
title: "The refinement product proves, refutes, leaves undetermined and stops the safety half"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-142
    type: verifies
---
# TC-547: The refinement product proves, refutes, leaves undetermined and stops the safety half

## Description

Verify `check_refinement`'s first phase over each fixture: exhaustive proof,
the canonical failing prefix with its `RefinementFailure`,
`MappingUndetermined`, limits and determinism.

Scope: FR-142-AC-1 to FR-142-AC-7.

## Test Procedure

Fixtures: `CasRefinesCounter`, the lost-update model and its `any` variant
(ADR-020 §8); `RegisterHistory` (FR-138); `Coin` (FR-141-AC-4);
`RingIsQueue` (FR-139).

1. Check `CasRefinesCounter`.
2. Check the lost-update model and its variant with both commit rows `->
   any`.
3. Check the compare-and-set model with concrete initial `value` 1, and with
   `commitA -> stutter`.
4. Check `RegisterHistory`, its broken update and its `writes` variant.
5. Check `Coin` with `side` hidden and with `side = self.face`.
6. Check `RingIsQueue`, the broken `take` and FR-140-AC-3's `only`.
7. Check `CasRefinesCounter` with `max_states` 5, `max_depth` 2 and a `true`
   poll; run step 2's two requests twice.

Tag the tests `#[trace("TC-547", "FR-142-AC-n")]`.

## Expected Results

- Step 1: `Holds{Exhaustive}`, 28 product states.
- Step 2: `Violated`, prefix `beginA`, `beginB`, `commitA`, `commitB`,
  `AbstractStepRejected{position: 4, inc(c), Postcondition}`; `Violated`,
  six-step prefix, `NoAbstractMatch{position: 6}`, `value` 2 to 1.
- Step 3: `InitialNotAbstract{initial: 0}`, empty prefix;
  `StutterChanged{position: 2}`.
- Step 4: `Holds{Exhaustive}`, 4 product states;
  `AbstractStepRejected{position: 1, write(r, 1), Postcondition}`;
  `Undecided(MappingUndetermined)` naming `writes`.
- Step 5: `Holds{Exhaustive}`; `AbstractStepRejected{position: 2, show(c),
  Frame{…}}`.
- Step 6: `Holds{Exhaustive}`; `AbstractStepRejected{…, deq(r),
  Postcondition}`; `Undecided(MappingUndetermined)` naming `items`.
- Step 7: `Stopped(ResourceExhausted, MaxStates)`; `BoundReached{depth: 2}`;
  `Stopped(Cancelled, …)`; equal outcomes and byte-equal counterexamples.
