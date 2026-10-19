---
id: TC-629
title: "Window aggregates evaluate exactly over past windows and keep the temporal layer Boolean"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-194
    type: verifies
---
# TC-629: Window aggregates evaluate exactly over past windows and keep the temporal layer Boolean

## Description

Verify aggregate evaluation under both profiles, nearest-rank quantiles, insufficient data, the window-value limit and work charge, S3 refusals, and the Boolean temporal input.

Scope: FR-194-AC-1 to FR-194-AC-5.

## Test Procedure

Fixtures: the timestamped response trace of FR-194-AC-1; event-position traces of FR-194-AC-2; a sparse trace with three responses; a dense trace of 20 values; the event-position `x` trace `2, 1, 0, 4`.

1. Evaluate the quantile clause at `0.95` and `0.9`, and with threshold `0.005 s`.
2. Evaluate the `count`, `fraction`, `sum`, `min` and `max` terms of FR-194-AC-2.
3. Evaluate with `min_count 5` over the sparse trace; with `max_window_values` 10 over the dense trace; read the work meter.
4. Evaluate the `sum 6 / e.x`, undefined-filter `count` and undefined-predicate `fraction` terms of FR-194-AC-5 at position 3, with and without `min_count 5`, and at `past[2, 3]`; read the work meter over a filter rejecting 10 of 20 positions.
5. Check the four refused clauses; compare horizons; read the tl-mltl input and a holding result.

Tag the tests `#[trace("TC-629", "FR-194-AC-n")]`.

## Expected Results

- Step 1: false, true, and the same with `0.005 s`.
- Step 2: windows 0 to 4 and 6 to 15; `3/4`; 7, 1, 4.
- Step 3: `Undefined(InsufficientData{3, 5})`; `Incomplete` naming `max_window_values` and 10; 20 units.
- Step 4: `Undefined(division-by-zero)` for all three terms, also under `min_count 5`; no `InsufficientData`; the sum is never `10.5`; the window of positions 2 to 3 is read as defined by its values; 20 units.
- Step 5: all refused; equal horizons; a Boolean signal; `tested` with `Observed`.
