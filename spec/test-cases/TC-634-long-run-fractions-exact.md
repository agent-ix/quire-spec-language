---
id: TC-634
title: "Long-run fractions are decided exactly by bottom components and maximal end components"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-199
    type: verifies
---
# TC-634: Long-run fractions are decided exactly by bottom components and maximal end components

## Description

Verify the BSCC value and gain–bias pair for `LongRun`, MEC optima over every scheduler, the weighted form, and `ZeroWeightComponent`.

Scope: FR-199-AC-1 to FR-199-AC-3.

## Test Procedure

Fixtures: `Avail` and `LongRun`; `Avail2` with two repairs; the `duration`-weighted variant and its zero-weight variant.

1. Decide `LongRun` under `Steady` and read the gain–bias pair.
2. Decide `Avail2`'s availability over every scheduler (minimum and maximum) and under the two-repair workload.
3. Decide the weighted claim; decide the zero-weight variant.

Tag the tests `#[trace("TC-634", "FR-199-AC-n")]`.

## Expected Results

- Step 1: `π = (1800/1801, 1/1801)`, value `1800/1801`, gain `1800/1801`, bias 0 and `−2000/1801`.
- Step 2: `1000/1001` and `1800/1801`; `1400/1401`.
- Step 3: `π_up / (π_up + 3 π_down)` exact; `Unsupported(ZeroWeightComponent)`.
