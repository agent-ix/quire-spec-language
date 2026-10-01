---
id: TC-606
title: "A single-existential claim is proved by a lasso witness and refuted by a trap"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-181
    type: verifies
---
# TC-606: A single-existential claim is proved by a lasso witness and refuted by a trap

## Description

Verify HP-5 checking through the possible family: lasso witnesses and their sources, a product trap and its replay, fairness on the existential variable, a depth bound, and lasso replay refusal.

Scope: FR-181-AC-1 to FR-181-AC-4.

## Test Procedure

Fixtures: ADR-023 §8's secure vault; the vault with `reset` (FR-176-AC-3).

1. `eventually always holds(v.l @ b = 1)` with `witness_samples` 0 and with the default.
2. `eventually holds(v.l @ b = 2)`.
3. The `reset` vault claim with `fair { weak V::Vault::reset }` and with no fairness set.
4. Step 1's claim with `max_depth` 1 and `witness_samples` 0; step 1's witness with its loop's last step removed.

Tag the tests `#[trace("TC-606", "FR-181-AC-n")]`.

## Expected Results

- Step 1: `Witnessed`, `step(1)` loops, source `Explored`, replayed, `proved`, `decisive-witness`; `proved` with each source named.
- Step 2: `Trapped` at initial state 0, empty stem, replay finds no accepting cycle, `refuted`, `closed-scope`.
- Step 3: `refuted`, `closed-scope`; `proved`.
- Step 4: `inconclusive`, `BoundReached{depth: 1}`; refusal for a loop that does not close.
