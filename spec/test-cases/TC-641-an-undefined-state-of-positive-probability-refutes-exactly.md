---
id: TC-641
title: "An undefined state reached with positive probability refutes an exact claim"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-196
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-202
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-203
    type: verifies
---
# TC-641: An undefined state reached with positive probability refutes an exact claim

## Description

Verify that EN-5 refutes a probabilistic claim that evaluates undefined at a
product state of positive probability, with an `Undefined` path as its
evidence, that the evidence replays, and that the item settles `refuted`
with cause `UndefinedEvaluation` whatever the threshold.

Scope: FR-196-AC-5, FR-196-AC-13, FR-202-AC-5, FR-203-AC-5.

## Test Procedure

Fixture: the `Coin` model and claim of FR-196-AC-5 (`drop` deletes the coin when `b = 0`, and the claim reads `c.v`).

1. Build the product and run EN-5.
2. Replay the evidence, then the same evidence with `where` set to 0.
3. Settle the outcome with step 2's first replay result.
4. Repeat the build with a population universe that contains the claim's
   `over` key but whose initial state has no object at that key, so the first
   predicate read is undefined at position 0. Inspect the returned key before
   any monitor or accumulator advancement. Then use the Coin case where
  `drop` with `b = 0` removes the object, making the first undefined read
  occur at position 1. Also run the initial-state case with the bounded
  `steps` measure `steps from holds(c.v = 0) until holds(c.v = 1) within 3 <= 3`.

Tag the tests `#[trace("TC-641", "<AC id>")]`.

## Expected Results

- Step 1: the build stops at the state reached by `drop` with `b = 0`,
  position 1; evidence `Undefined` path `drop` with `b = 0`, probability
  `1/2`, `UndefinedEvaluation{where: 1, cause: absent-key}`.
- Step 2: `reproduced-with-evaluated-witness`; then `inconclusive`,
  `ReplayParity`.
- Step 3: `refuted`, `decisive-counterexample`, category violation, cause
  `UndefinedEvaluation`.
- Step 4: the position-0 formula path has no steps, probability 1, the
  initial model state, the `over` binding for the missing `ghost` object, a
  `Formula(Running(q0))` monitor where `q0` is the FR-338 key of the initial
  state of the compiled `always[0,3] holds(c.v = 0)` monitor, no accumulator,
  and no pending draw. It is not the successor on the undefined
  position-zero letter. The
  position-0 `steps` path has
  `Measure{remaining: 3, phase: Inactive}` and
  `Steps{activated: false, amount: 0}`, with no pending draw. The position-1
  Coin path has one `drop` step with `b = 0`, probability `1/2`; its key names
  the state after deletion and retains the control reached after the defined
  position-zero read. It does not include the advance that the undefined
  position-one read would have caused. Its monitor is the FR-338 key of the
  state reached after reading the defined position-zero letter
  `holds(c.v = 0)`; it has no accumulator or pending draw. Under `Even`, the
  `drop` draw with `b = 0` has probability `1/2`.
