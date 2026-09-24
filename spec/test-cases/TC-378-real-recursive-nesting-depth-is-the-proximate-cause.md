---
id: TC-378
title: "A real recursive-descent fixture shows the nesting-depth limit is the proximate cause of a function-declaration refusal"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: verifies
---
# TC-378: A real recursive-descent fixture shows the nesting-depth limit is the proximate cause of a function-declaration refusal

## Description

Verify FR-062-AC-7 against a body that genuinely nests, now that real
recursive checking runs inside `ValueFunctionFamily::check` (QSL-148):
holding a fixture nested to depth D fixed and varying only the configured
nesting-depth limit by exactly one shows the limit, not the fixture's
absolute size, to be the proximate cause of the refusal. The `Limit`
carries the `Locus::Region` of the node whose entry failed (FR-096-AC-11).

Scope: FR-062-AC-7, FR-096-AC-11.

## Test Procedure

1. Parse a function declaration whose body is the source text
   `not not not true` (three `Not`s wrapping a `Boolean` leaf, four nodes
   deep) under a unit reference, so its forms carry spans, and check it through
   `ValueFunctionFamily::check` with the `CheckingLimits` nesting depth
   configured to 3.
2. Check the identical declaration again with the limit configured to 4 and
   nothing else changed.

## Expected Results

- Step 1: `StageFailure::Limit` with kind nesting depth, bound 3, actual
  4, and `Locus::Region` over the span of `true`, reported as
  `stage_limit_exceeded`/`nesting-depth-exceeded`.
- Step 2: checking succeeds; no nesting-depth `Limit` outcome is returned.

Tag the tests `#[trace("TC-378", "FR-062-AC-7")]` and
`#[trace("TC-378", "FR-096-AC-11")]`.

## Status

**Unbacked** (PR #303 review, findings 4/5; reverted from an earlier
"backed" claim in this round). The earlier claim rested on
`real_recursive_descent_is_nesting_depth_bounded`, which depended on
`check::family::charge_recursive_nesting` -- a side-walk added purely to
charge `CheckContext`'s nesting counter once per expression-tree node
*after* the real check had already run. That is not what this test case
asks for: charging nesting during a re-walk of an already-checked form is
not "the limit... is shown to be the proximate cause" of real recursive
descent, and the side-walk silently dropped the real walk's own source
location and early-return-on-first-error behavior. Both the side-walk and
the test that exercised it are deleted.

`real_checker_depth_limit_is_the_proximate_cause`
(`qsl-semantics/src/check/family.rs`, `checking_tests`, untagged) shows the
bound is the proximate cause today, but through `StageFailure::Refused`
(`CheckCause::ResourceExhausted`) with no locus.
[FR-096](../functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md)
(QSL-160) resolves it: the `CheckingLimits` depth is a stage limit, so the
family returns `Typer`'s depth stop as `StageFailure::Limit` with a
`Locus::Region`, with no `CheckContext` threaded through `Typer`. The steps
and expected results above state that target; ADR-013 §7 slice S-5b builds
it.

`nesting_depth_limit_is_the_proximate_cause`
(`qsl-semantics/src/check/family.rs`, `checking_tests`) remains, untagged, as a narrower
regression guard on `CheckContext`'s own per-declaration nesting charge;
see that test's own doc comment.
