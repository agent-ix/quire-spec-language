---
id: TC-1166
title: "Every exact product build result settles by FR-203's table"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-203
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-192
    type: verifies
---
# TC-1166: Every exact product build result settles by FR-203's table

## Description

Verify that EN-5 settles each FR-196 `ProductRefusal` member by FR-203's
"Build refusals and stops" table, including a refused predicate, a
translation error, an expansion stop naming a limit and the request
refusals that return no terminal record, and that EN-4 settles a refused
predicate by FR-192's matching rows.

Scope: FR-203-AC-7, FR-192-AC-7.

## Test Procedure

1. Over TC-638's subject, build the product with `max_candidates` 2.
2. Over a subject whose successor expansion charges the evaluation meter
   before the claim reads any predicate, build under a `work_units` bound
   `B` its first expansion exceeds; assert the build's `ProductRefusal` is
   `Expansion(stop)` naming `EvaluationMeter`, then settle it.
3. Build a claim whose predicate refuses `integer_out_of_domain` at a
   reachable state, and one under a `work_units` bound `B` the expansion
   fits whose predicate read's charge is the first one denied; assert the
   latter's `ProductRefusal` is `PredicateRefused(Incomplete(_))`.
4. Settle constructed `PredicateRefused(NotBoolean)`,
   `PredicateRefused(KernelRefused(CheckedInvariant))` and `Translation`
   results.
5. Build over a subject whose admission fails, and over one with an
   unbounded population root.
6. Run EN-4 over the same refusing and meter-limited predicates, and
   settle constructed `PredicateRefused(NotBoolean)` and
   `PredicateRefused(KernelRefused(CheckedInvariant))` outcomes through
   FR-192's map.

Tag the tests `#[trace("TC-1166", "<criterion>")]`.

## Expected Results

- Step 1: `Incomplete(LimitReached{max_candidates, 2,
  expansion.max_candidates})`.
- Step 2: `Incomplete(LimitReached{...})` naming the exhausted counter and
  `B` by FR-255 Behavior 9's rule.
- Step 3: `failed` with `integer_out_of_domain`/`outside-domain`; the same
  Behavior 9 `LimitReached` for the predicate's counter.
- Step 4: each `failed`, `runtime_invariant`/`established-invariant-broken`.
- Step 5: request refusals, no terminal record.
- Step 6: the FR-192 rows' settlements, equal to EN-5's for the same
  outcome.
