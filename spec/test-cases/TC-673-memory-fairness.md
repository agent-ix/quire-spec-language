---
id: TC-673
title: "Flush and visibility fairness join every infinite-trace fairness set"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-228
    type: verifies
---
# TC-673: Flush and visibility fairness join every infinite-trace fairness set

## Description

Verify the derived memory constraints, their `memory` origin, their effect on liveness, the visibility filter, and that safety verdicts do not change.

Scope: FR-228-AC-1 to FR-228-AC-7.

## Test Procedure

1. Read the fairness set of FR-228-AC-1's clause under `tso`, `ra` and
   `sc`.
2. Check FR-228-AC-2's store-and-poll protocol under `tso`; pass its
   unflushed lasso to the fairness filter; check the claim with the memory
   constraints ignored.
3. Pass the two synthesized SCCs of FR-228-AC-3 to the filter.
4. Check ADR-025 §10's TP-1 claim with and without the memory constraints
   read; check a `fair weak flush` constraint.
5. Pass FR-228-AC-5's SCC, with the load-free sub-loop `p q`, to the filter,
   and to the certificate check in its non-accepting variant (FR-339-AC-3).
6. Pass FR-228-AC-6's SCC with the stale self-loop `p → p` to the filter and
   CX-5's walk.
7. Pass FR-228-AC-7's SCC, whose second pair needs a second recursion level,
   to the filter taking each pair first in turn.

Tag the tests `#[trace("TC-673", "FR-228-AC-n")]`.

## Expected Results

- Step 1: two `flush` constraints with origin `memory` under `tso`;
  visibility constraints under `ra`; none under `sc`.
- Step 2: `proved`; the lasso meets the scheduler constraints and is
  rejected by `flush(w)`'s; `refuted` with that lasso when ignored.
- Step 3: rejected, then passed.
- Step 4: equal verdicts; `missing_declaration`/`missing-name`.
- Step 5: the SCC is decomposed to `{r}` then `{p, q}` and `{p, q}` passes; the lasso `p q` meets the lasso form; the certificate check accepts the variant's `UnfairVisibility` although `q → r` runs backward.
- Step 6: `{p, q}` passes with its two kept edges; the loop is `p q`, not `p p`.
- Step 7: the passing component is `{p, q}` for both orders, after two recursion levels on `{r, s}`.
