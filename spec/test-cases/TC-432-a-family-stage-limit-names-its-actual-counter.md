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

Verify that each of the two stage-entry limits `ValueFunctionFamily::check`
can reach returns a `LimitExceeded` naming its kind, the configured bound
and the real actual counter, and that the counter is exactly where the limit
stops `check`. Scope: FR-062-AC-12.

## Test Procedure

Check the declaration `f() -> Boolean = true` through
`ValueFunctionFamily::check`, measuring its preimage byte length `b` and work
charge `w` with the same measure `check` uses:

1. Input-byte limit `b - 1`; then limit `b`.
2. Work budget `w - 1`; then work budget 0; then, against one meter with
   budget `w`, check the declaration twice.
3. Check the larger declaration `g() -> Boolean = if true then false else
   true`, whose measured byte length `b'` exceeds 1, with input-byte limit 0.

## Expected Results

- Step 1: `Limit(InputBytes, bound b - 1, actual b)`; at `b` it admits.
- Step 2: `Limit(WorkBudget, bound w - 1, actual w)`; budget 0 returns
  actual `w`; with budget `w` the first check admits and the second
  returns `Limit(WorkBudget, bound w, actual 2w)`.
- Step 3: `Limit(InputBytes, bound 0, actual b')`: the counter is the
  measured metric, not the bound plus one.
