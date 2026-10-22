---
id: TC-1166
title: "Every exact product build result settles by FR-203's table"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-203
    type: verifies
---
# TC-1166: Every exact product build result settles by FR-203's table

## Description

Verify that EN-5 settles each FR-196 `ProductRefusal` member by FR-203's
"Build refusals and stops" table, including a refused predicate, a
translation error, an expansion stop naming a limit and the request
refusals that return no terminal record.

Scope: FR-203-AC-7.

## Test Procedure

1. Over TC-638's subject, build the product with `max_candidates` 2, and
   with the accounting limit `work_units` 0 so the expansion's evaluation
   meter stops.
2. Build a claim whose predicate refuses `integer_out_of_domain` at a
   reachable state, and one under a `work_units` bound `B` the expansion
   fits whose predicate read's charge is the first one denied.
3. Settle constructed `PredicateRefused(NotBoolean)`,
   `PredicateRefused(KernelRefused(CheckedInvariant))` and `Translation`
   results.
4. Build over a subject whose admission fails, and over one with an
   unbounded population root.

Tag the tests `#[trace("TC-1166", "FR-203-AC-7")]`.

## Expected Results

- Step 1: `Incomplete(LimitReached{MaxCandidates, 2,
  expansion.max_candidates})`; `Incomplete(LimitReached{ClauseMeter, 0,
  work_units})`.
- Step 2: `failed` with `integer_out_of_domain`/`outside-domain`;
  `Incomplete(LimitReached{ClauseMeter, B, work_units})`.
- Step 3: each `failed`, `runtime_invariant`/`established-invariant-broken`.
- Step 4: request refusals, no terminal record.
