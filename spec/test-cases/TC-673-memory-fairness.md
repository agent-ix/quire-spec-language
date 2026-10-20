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

Scope: FR-228-AC-1 to FR-228-AC-9.

## Test Procedure

1. Read the fairness set of FR-228-AC-1's clause under `tso`, `ra` and
   `sc`.
2. Check FR-228-AC-2's store-and-poll protocol under `tso`; pass its
   unflushed lasso to the fairness filter; check the claim with the memory
   constraints ignored.
3. Pass the two synthesized SCCs of FR-228-AC-3 to the filter.
4. Check ADR-025 §10's TP-1 claim with and without the memory constraints
   read; check a `fair weak flush` constraint.
5. Pass FR-228-AC-5's SCC, with the load-free sub-loop `p q`, to the filter
   (its certificate form is TC-526 step 3).
6. Pass FR-228-AC-6's SCC with the stale self-loop `p → p` to the filter and
   CX-5's walk.
7. Pass FR-228-AC-7's SCC, whose second pair needs a second recursion level,
   to the filter and record the edges removed at each level.
8. Pass FR-228-AC-8's SCC, which fails a strong constraint and a visibility
   pair, to the filter.
9. Pass FR-228-AC-9's three-state SCC with pairs A and B to the filter, ranking components by earliest-discovered state.

Tag the tests `#[trace("TC-673", "FR-228-AC-n")]`.

## Expected Results

- Step 1: two `flush` constraints with origin `memory` under `tso`;
  one `strong` visibility member under `ra`; none under `sc`.
- Step 2: `proved`; the lasso meets the scheduler constraints and is
  rejected by `flush(w)`'s; `refuted` with that lasso when ignored.
- Step 3: rejected, then passed.
- Step 4: equal verdicts; `missing_declaration`/`missing-name`.
- Step 5: the SCC is decomposed to `{r}` then `{p, q}` and `{p, q}` passes; the lasso `p q` meets the lasso form.
- Step 6: `{p, q}` passes with its two kept edges; the loop is `p q`, not `p p`.
- Step 7: `q → r` is removed at the first level and `s → r` at the second; the passing component is `{p, q}`.
- Step 8: one refinement step removes state 0 and the edge `1 → 2`; `{3, 4}` passes with the loop `3 → 4 → 3`.
- Step 9: both pairs' edges are removed in one step and `{1}` is the first passing component, not `{2}`.
