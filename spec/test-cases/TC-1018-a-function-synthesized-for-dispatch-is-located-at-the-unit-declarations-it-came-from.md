---
id: TC-1018
title: "A function synthesized for dispatch is located at the unit declarations it came from"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: verifies
---
# TC-1018: A function synthesized for dispatch is located at the unit declarations it came from

## Description

Verify that the functions the unit compile synthesizes for dispatch resolve to regions of the unit, so the emitter places every occurrence. Scope: FR-096-AC-20.

Catches a function that resolves to no region and refuses emission, and one that resolves to the wrong declaration.

## Test Procedure

1. Compile a unit with bodies `11` for `A.size` and `22` for `B.size` (`B` redefining `A`), `pre preA` on `A.size`, `pre preB` on `B.size`, and a `pre` clause calling `size`.
2. Resolve the region of the body function and the precondition function of each, and of the disjunction function of `B.size`, and read the text each names.
3. Read the `generated` occurrences of those functions.
4. Emit the unit with `compile_package`.

## Expected Results

1. The body functions resolve to `11` and `22`.
2. The precondition functions of `A.size` and `B.size` and the disjunction function resolve to the text of `preA`, `preB` and `preB`.
3. Each function has one `generated` occurrence, which resolves to the region of its body.
4. The emission succeeds.
