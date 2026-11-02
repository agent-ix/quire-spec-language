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

Scope: FR-251-AC-1 to FR-251-AC-3, FR-251-AC-13, FR-251-AC-16, FR-251-AC-18 and FR-251-AC-20. The executable-plan, admission, isolation, equivalence and failure criteria (FR-251-AC-4 to FR-251-AC-8) are TC-1081.

## Test Procedure

Fixtures: the deadline claim of FR-251-AC-1 on a 20-bit 1 MHz counter; `once[0 ms, 10 ms] holds(p)`; a 16-bit counter.

1. Evaluate the deadline claim on the tick traces of FR-251-AC-1 with uncertainty 0 and 2 µs.
2. Compute the buffer for rate 2 per ms on a 1 MHz counter and run a trace with 23 events in one window; repeat for `once[5 ms, 10 ms]` (AC-13); feed `a` and `b` in one tick and evaluate `a before b` (AC-16).
3. With `max_reading_gap` 100, compute differences across wraparound and feed a backwards reading; plan a 16-bit target with `max_reading_gap` 65,536.
4. Compute the capacities of the nested `once[0 ms, 10 ms] once[0 ms, 5 ms] holds(p)` on the 1 MHz counter at rate 2 per ms and fill the inner buffer past capacity (AC-18). Overflow two sibling `once[0 ms, 10 ms]` operators at the same event (AC-20).

Tag the tests `#[trace("TC-706", "FR-251-AC-n")]`.

## Expected Results

- Step 1: true, false, indeterminate; indeterminate.
- Step 2: capacity 22 for both windows; `Incomplete(LimitReached{limit: event_buffer, value: 22, setting: monitor.max_event_rate})`, no event dropped; same-tick `a before b` has difference `(-1, 1)` and is undecided.
- Step 3: 10 ticks; `Failed`, `ReadingGapExceeded{gap: 65506, max_reading_gap: 100}`, no verdict; `GapExceedsWidth`.
- Step 4: outer capacity 22, inner capacity 32 (retention 15002 ticks); the 33rd inner entry gives `Incomplete(LimitReached{limit: event_buffer, value: 32, setting: monitor.max_event_rate})`; the simultaneous overflow gives two `Incomplete` results with value 22, the lower `subformula` id first, none dropped.
