---
id: TC-432
title: "A family check's stage limit names its kind, bound and actual counter, and the counter is where the limit stops"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: verifies
---
# TC-432: A family check's stage limit names its kind, bound and actual counter, and the counter is where the limit stops

## Description

Verify that each of the four limits `ValueFunctionFamily::check` can reach
returns a `LimitExceeded` naming its kind, the configured bound and the
real actual counter, and that the counter is exactly where the limit stops
`check`. Scope: FR-062-AC-12.

## Test Procedure

Check the declaration `f() -> Boolean = true` through
`ValueFunctionFamily::check`, measuring its preimage byte length `b`, node
count `n` and work charge `w` with the same measure `check` uses:

1. Nesting-depth limit 0; then limit 1.
2. Input-byte limit `b - 1`; then limit `b`.
3. Node-count limit `n - 1`; then limit `n`.
4. Work budget `w - 1`; then work budget 0; then, against one meter with
   budget `w`, check the declaration twice.
5. Check the larger declaration `g() -> Boolean = if true then false else
   true`, whose measured byte length `b'` and node count `n'` both exceed 1,
   with input-byte limit 0, then with node-count limit 0.

## Expected Results

- Step 1: `Limit(NestingDepth, bound 0, actual 1)`; at limit 1 `check`
  admits.
- Step 2: `Limit(InputBytes, bound b - 1, actual b)`; at `b` it admits.
- Step 3: `Limit(NodeCount, bound n - 1, actual n)`; at `n` it admits.
- Step 4: `Limit(WorkBudget, bound w - 1, actual w)`; budget 0 returns
  actual `w`; with budget `w` the first check admits and the second
  returns `Limit(WorkBudget, bound w, actual 2w)`.
- Step 5: `Limit(InputBytes, bound 0, actual b')` and
  `Limit(NodeCount, bound 0, actual n')`: the counter is the measured
  metric, not the bound plus one.

## Status

Backed: `nesting_depth_limit_is_the_proximate_cause`,
`stage_limits_restored_kinds_refuse_one_below_the_real_metric` and
`work_budget_kind_refuses_from_a_denied_meter_charge`
(`qsl-semantics/src/check/family.rs`), each tagged
`#[trace("TC-432", "FR-062-AC-12")]`.
