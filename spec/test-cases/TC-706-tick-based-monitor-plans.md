---
id: TC-706
title: "Tick-based monitor plans round soundly, size buffers from the event rate and fault on counter errors"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-251
    type: verifies
---
# TC-706: Tick-based monitor plans round soundly, size buffers from the event rate and fault on counter errors

## Description

Verify the monitor plan's three-valued decisions under sound rounding, buffer capacity from the event rate, overflow settlement and counter faults, against QSL's reference evaluator.

Scope: FR-251-AC-1 to FR-251-AC-3.

## Test Procedure

Fixtures: the deadline claim of FR-251-AC-1 on a 20-bit 1 MHz counter; `once[0 ms, 10 ms] holds(p)`; a 16-bit counter.

1. Evaluate the deadline claim on the tick traces of FR-251-AC-1 with uncertainty 0 and 2 µs.
2. Compute the buffer for rate 2 per ms and run a trace with 22 events in one window.
3. With `max_reading_gap` 100, compute differences across wraparound and feed a backwards reading; plan a 16-bit target with `max_reading_gap` 65,536.

Tag the tests `#[trace("TC-706", "FR-251-AC-n")]`.

## Expected Results

- Step 1: true, false, indeterminate; indeterminate.
- Step 2: capacity 21; `Incomplete(ResourceExhausted)` naming the rate limit, no event dropped.
- Step 3: 10 ticks; `Failed`, `ReadingGapExceeded{gap: 65506, max_reading_gap: 100}`, no verdict; `GapExceedsWidth`.
