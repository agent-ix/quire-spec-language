---
id: TC-240
title: "allInstances and lookup return the FR-153 typed result shape and its bound/foreign/ineligible refusals"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-084
    type: verifies
---
# TC-240: allInstances and lookup return the FR-153 typed result shape and its bound/foreign/ineligible refusals

## Description

Verify `allInstances<T>(p)`'s and `lookup<T>(p, r)`'s typed result shapes
and their named refusals: a selected count above a declared maximum refuses
cardinality-out-of-bound; an unbounded population admits every selected
count; a foreign-universe key refuses; and a non-binding receiver or a
non-object-type `T` each refuse with their own cause. Scope: FR-084-AC-5.

Step 3 (the unbounded population) is backed since QSL-140:
`qsl-semantics/tests/it/model_population.rs`,
`an_unbounded_binding_selects_every_member_with_no_bound`, tagged
`#[trace("TC-240", "FR-084-AC-5")]`.

Catches an implementation that (a) returns a bare, untyped collection
instead of `Set<Reference<T>>[0,N]`, invisible to a test that only checks
membership; (b) silently truncates a selection above the declared maximum
instead of refusing; or (c) treats "no declared maximum" and "declared
maximum, currently under it" as the same code path, which a test with only
one bounded fixture cannot distinguish.

## Test Procedure

1. Admit a closed population declaring maximum `N = 2` with three
   qualifying members, and closures established for the queried type.
2. Evaluate `allInstances<T>(p)` over it.
3. Admit a second, otherwise identical population declaring no maximum, with
   the same three qualifying members, and evaluate `allInstances<T>(p)`
   over it.
4. Evaluate `lookup<T>(p, r)` with a key `r` whose universe differs from
   `T`'s universe.
5. Evaluate `allInstances<S>(p)` where `S` is not a model object type.

## Expected Results

Step 2 refuses with a cardinality-out-of-bound cause naming maximum `2` and
selected count `3`. Step 3 returns the complete three-member
`Set<Reference<T>>` with no maximum and no cardinality-out-of-bound refusal.
Step 4 refuses with a foreign-universe cause. Step 5 refuses with a
type-mismatch cause. A mutant that returns a partial (truncated) two-member
set in step 2 instead of refusing passes a shallow "some members returned"
check but fails the refusal assertion; a mutant that refuses
operator-ineligible in step 3 (the behaviour before QSL-140) fails the
unbounded-admits-every-count assertion.
