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

Scope: FR-238-AC-1 to FR-238-AC-4.

## Test Procedure

Fixtures: constant sets of FR-238-AC-1; clocks `x, y`; a seeded generator of DBMs of dimension at most 5; out-of-range clock indices, a reset of the reference clock and DBMs of different dimensions; the bounded-subtraction and diagonal-constraint cases from QSpec FR-415 and TC-371 ZC-15/ZC-16.

1. Compute the scale factor and scaled constants for `1/3, 5/2, 1` and for `10^30, 1/7`.
2. Apply `zero`, `up`, `constrain`, `reset`, emptiness and inclusion as FR-238-AC-2 lists, and the three out-of-shape operations.
3. Compare each operation with the grid reference on 10,000 generated DBMs, on every `1/dim` grid point in one-unit windows at a seeded anchor, at the least corner of each zone involved and, for a refused inclusion, past the looser entry; check that each result is canonical.
4. For bounded subtraction, independently enumerate source valuations over the full scaled grid, retain only values within `amount <= x <= ceiling`, subtract from the selected source clock, and compare the resulting image with the canonical DBM. Cover both strict and non-strict diagonal bounds involving the selected clock, the zero-amount identity, exact lower/upper boundaries, a changed subtraction constant, zones entirely below the amount or above the ceiling, and malformed amount/ceiling/index inputs. Check closure idempotence and error atomicity.

Tag the tests `#[trace("TC-693", "FR-238-AC-n")]`; the bounded-subtraction cases trace to `FR-238-AC-4` only.

## Expected Results

- Step 1: scale 6 and constants `2, 15, 6`; scale 7 with no overflow.
- Step 2: the zones, verdicts and `DbmError`s FR-238-AC-2 states; each refused operation leaves the DBM unchanged.
- Step 3: agreement on every DBM and every window point; emptiness agrees in both directions; every result is canonical.
- Step 4: every tested post-state is present exactly when its reconstructed source valuation satisfies the source zone and bounded guard; the image zone is canonical; disabled updates are empty; malformed inputs leave the input matrix unchanged.
