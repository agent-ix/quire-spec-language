---
id: TC-673
title: "Flush and visibility fairness join every infinite-trace fairness set"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-228
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-339
    type: verifies
---
# TC-673: Flush and visibility fairness join every infinite-trace fairness set

## Description

Verify the derived memory constraints, their `memory` origin, their effect on liveness, and the visibility filter.

Scope: FR-228-AC-1 to FR-228-AC-10, FR-127-AC-17 and FR-339-AC-9.

## Test Procedure

1. Read the fairness set of FR-228-AC-1's clause under `tso`, `ra` and
   `sc`, and under `tso` with `scheduling adversarial`.
2. Check FR-228-AC-2's store-and-poll protocol under `tso` and check its
   component certificate (FR-339); pass the SCC of its unflushed lasso's
   loop to the fairness filter over the clause's fairness set, then over
   that set without its `memory` members.
3. Pass the two synthesized SCCs of FR-228-AC-3 to the filter.
4. Check a `fair weak flush` constraint.
5. Pass FR-228-AC-5's SCC, with the load-free sub-loop `p q`, to the filter
   (its certificate form is TC-526 step 3).
6. Pass FR-228-AC-6's SCC with the stale self-loop `p → p` to the filter and
   CX-5's walk.
7. Pass FR-228-AC-7's SCC, whose second pair needs a second recursion level,
   to the filter and record the edges removed at each level.
8. Pass FR-228-AC-8's SCC, which fails a strong constraint and a visibility
   pair, to the filter.
9. Pass FR-228-AC-9's three-state SCC with pairs A and B to the filter, searching from state 0 and ranking components by earliest-discovered state.
10. With `ignore_memory_constraints` absent and `false`, repeat FR-228-AC-2's
    counterexample check; with it `true`, run the diagnostic fairness filter
    and offer the result to the ordinary settlement and component-certificate
    paths.

Tag the tests `#[trace("TC-673", "FR-228-AC-n")]`.

## Expected Results

- Step 1: two `flush` constraints with origin `memory` under `tso`, with
  and without `scheduling adversarial` (no `scheduler` constraint with it);
  one `strong` visibility member per (thread, loaded location) under `ra`,
  with origin `memory` and target `{"type":"visibility","model":"ra"}`;
  none under `sc`.
- Step 2: `proved`, `Certified`; the lasso meets the scheduler constraints;
  the filter rejects the SCC with `flush(w)`'s constraint as its only failing
  member, and passes it without the `memory` members.
- Step 3: rejected, then passed.
- Step 4: `missing_declaration`/`missing-name`.
- Step 5: the SCC is decomposed to `{r}` then `{p, q}` and `{p, q}` passes; the lasso `p q` meets the lasso form.
- Step 6: `{p, q}` passes with its two kept edges; the loop is `p q`, not `p p`.
- Step 7: `q → r` is removed at the first level and `s → r` at the second; the passing component is `{p, q}`.
- Step 8: one refinement step removes state 0 and the edge `1 → 2`; `{3, 4}` passes with the loop `3 → 4 → 3`.
- Step 9: both pairs' edges are removed in one step and `{1}` is the first passing component. Rationale: refining pair A alone first would pass `{2}` (refining pair B first also gives `{1}`), so the vector catches an implementation that refines one pair at a time starting with pair A.
- Step 10: absent and `false` preserve the ordinary `refuted` result and its
  replay. With `true`, a lasso that violates only a `memory`-origin fairness
  member yields `RefutedIgnoringConstraints`, settles as
  `Inconclusive(RefutedIgnoringConstraints)` with no proof basis and no
  ordinary counterexample replay; a lasso satisfying the original full
  fairness set remains ordinary `refuted`. The checker for a supplied EN-1
  component certificate always applies the original full fairness set and
  never accepts the diagnostic result as a certificate.
