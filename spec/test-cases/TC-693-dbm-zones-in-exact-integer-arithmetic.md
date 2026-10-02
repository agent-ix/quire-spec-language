---
id: TC-693
title: "Zones as difference-bound matrices in exact integer arithmetic"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-238
    type: verifies
---
# TC-693: Zones as difference-bound matrices in exact integer arithmetic

## Description

Verify constant scaling, the DBM operations and their agreement with a reference implementation.

Scope: FR-238-AC-1 to FR-238-AC-3.

## Test Procedure

Fixtures: constant sets of FR-238-AC-1; clocks `x, y`; a seeded generator of DBMs of dimension at most 5.

1. Compute the scale factor and scaled constants for `1/3, 5/2, 1` and for `10^30, 1/7`.
2. Apply `zero`, `up`, `constrain`, `reset`, emptiness and inclusion as FR-238-AC-2 lists.
3. Compare each operation with the grid reference on 10,000 generated DBMs.

Tag the tests `#[trace("TC-693", "FR-238-AC-n")]`.

## Expected Results

- Step 1: scale 6 and constants `2, 15, 6`; scale 7 with no overflow.
- Step 2: the zones and verdicts FR-238-AC-2 states.
- Step 3: agreement on every DBM.
