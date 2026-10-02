---
id: TC-686
title: "A timed subject's behaviours read as timed traces with delays, urgency, idle tails and digital time"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-231
    type: verifies
---
# TC-686: A timed subject's behaviours read as timed traces with delays, urgency, idle tails and digital time

## Description

Verify the delay and step relations of a timed subject, time stamps, urgency, the idle tail, digital time and the untimed reading.

Scope: FR-231-AC-1 to FR-231-AC-4.

## Test Procedure

Fixtures: ADR-026 §11's `Rpc` unit with `T = 3 ms` and `T = 4 ms`, universe `{c}`; `Rpc` with `urgent when self.phase = Waiting`; the digital `tick`/`work` model.

1. Over `Rpc` with `T = 4 ms`, test admissibility of delays 7 at `Idle`, 3 and `3 + 1/1000` after `send`; enabledness of `reply` after delays 1 and `999/1000`; build the trace `send` at 0, `reply` after `5/2`.
2. Over the urgent variant, test a positive delay after `send`; evaluate `always holds(c.phase != Waiting)` from position 2 over a behaviour ending at `Replied` under infinite-trace and under event-position false-extension.
3. Build the digital trace `work, tick, work, tick` and a behaviour ending at a terminal state.
4. Evaluate `always holds(not c.late)` under infinite-trace with and without time stamps, and a claim atom `c.x <= 3 ms`.

Tag the tests `#[trace("TC-686", "FR-231-AC-n")]`.

## Expected Results

- Step 1: admissible, admissible, not admissible; enabled, not enabled; stamps `0, 0, 5/2`, `x = 5/2` at position 2.
- Step 2: not admissible; `true` with the stutter step after positive delays; the execution closes at position 2.
- Step 3: stamps `0, 0, 5, 5, 10`; divergence through the stutter step counted as `tick`.
- Step 4: equal values; the atom reads the position's clock valuation.
