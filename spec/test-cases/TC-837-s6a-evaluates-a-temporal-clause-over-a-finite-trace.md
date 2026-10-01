---
id: TC-837
title: "S6a evaluates a temporal clause over a finite trace with typed positions and metered work"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-327
    type: verifies
---
# TC-837: S6a evaluates a temporal clause over a finite trace with typed positions and metered work

## Description

Verify bounded-profile evaluation over a closed trace, activation,
`TemporalPosition` decoding and encoding, and work metering.

Scope: FR-327-AC-1 to FR-327-AC-4.

## Test Procedure

Use the `Counter` trace with `c.value` 0, 1, 2 under event-position
false-extension.

1. Evaluate the four `on origin` clauses of FR-327-AC-1.
2. Evaluate `always[0,0] holds(c.value <= 1)` with activation `on each`.
3. Decode `0`, `2`, `02`, `-1`, the empty string and
   `18446744073709551616`; reconstruct position 3; encode position 12.
4. Evaluate `eventually[0,2] holds(c.value = 2)` with a meter of 2 work
   units, then unlimited, counting visits.

Tag the tests `#[trace("TC-837", "FR-327-AC-n")]`.

## Expected Results

- Step 1: `true`; `false` at 0; `false`; `true`.
- Step 2: `false` at position 2.
- Step 3: positions 0 and 2; four `invalid_runtime_input`/`invalid-value`
  refusals; position 3 refuses the same way; `12`.
- Step 4: `Incomplete` at `WorkUnits` with no truth value; then the work
  charged equals the visit count.
