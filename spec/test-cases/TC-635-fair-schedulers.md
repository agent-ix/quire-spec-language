---
id: TC-635
title: "Every-scheduler claims with a fairness set are decided over fair schedulers through fair end components"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-200
    type: verifies
---
# TC-635: Every-scheduler claims with a fairness set are decided over fair schedulers through fair end components

## Description

Verify fair end-component refinement, fair reachability infima, a fair witness, the fair long-run infimum with its randomized witness, and that bounded forms are unchanged.

Scope: FR-200-AC-1 to FR-200-AC-4.

## Test Procedure

Fixtures: §15.6's `Coin`; `Coin2` of FR-200-AC-2; `Avail2`; `Deliver`.

1. Compute the maximal fair end components and the infimum for `Coin` under strong and under weak fairness to `flip`.
2. Compute them for `Coin2` under strong fairness to `flip`; check the witness's induced bottom component.
3. Compute `Avail2`'s fair infimum under strong fairness to `repair_fast`; refute `>= 0.9995` and read the witness.
4. Decide `Deliver` with `fair weak Msg::send_a`.

Tag the tests `#[trace("TC-635", "FR-200-AC-n")]`.

## Expected Results

- Step 1: none; infimum 1 in both cases.
- Step 2: `{stuck}`; infimum 0; the witness takes `stall` and its bottom component is fair.
- Step 3: `1000/1001`; a witness with `k = 1` and induced value `1400/1401`.
- Step 4: `24/25`, with the fairness set recorded.
