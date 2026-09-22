---
id: TC-378
title: "A real recursive-descent fixture shows the nesting-depth limit is the proximate cause of a function-declaration refusal"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: verifies
---
# TC-378: A real recursive-descent fixture shows the nesting-depth limit is the proximate cause of a function-declaration refusal

## Description

Verify FR-062-AC-7 against a body that genuinely nests, now that real
recursive checking runs inside `ValueFunctionFamily::check` (QSL-148):
holding a fixture nested to depth D fixed and varying only the configured
nesting-depth limit by exactly one shows the limit, not the fixture's
absolute size, to be the proximate cause of the refusal.

## Test Procedure

1. Build a function declaration whose body is `Not(Not(Not(true)))` (three
   `Not`s wrapping a `Boolean` leaf, four nodes deep) and check it through
   `ValueFunctionFamily::check` with the nesting-depth limit configured to 3.
2. Check the identical declaration again with the limit configured to 4 and
   nothing else changed.

## Expected Results

- Step 1: checking refuses with a `Limit` outcome naming the nesting-depth
  limit.
- Step 2: checking succeeds; no nesting-depth `Limit` outcome is returned.

## Status

**Backed.** `real_recursive_descent_is_nesting_depth_bounded`
(`src/value/expression/family.rs`, `family_contract_tests`), tagged
`#[trace("TC-378", "FR-062-AC-7")]`.

**Plant/revert red-green check (QSL-148, 2026-09-21).** The same test
function, planted onto `main` at `1f313fd4` (pre-QSL-148) in a throwaway
worktree, fails: `ValueFunctionFamily::check` on that tree calls
`enter_nesting` at most once per top-level declaration and never descends
into the body, so a limit of 3 does not refuse a body nested 4 deep. On this
branch, `ValueFunctionFamily::check` additionally charges nesting once per
expression-tree node via a real recursive walk
(`check::family::charge_recursive_nesting`, called from `check` itself,
independent of and in addition to `Typer`'s own unrelated internal
recursion), which the test's own mutation argument covers: deleting that
walk's call in `check` reproduces the same failure this test showed on
`main`.

This test case supersedes `nesting_depth_limit_is_the_proximate_cause`
(`src/value/expression/family.rs`), which stays, untagged, as a narrower
regression guard on the `>=` comparison itself; see that test's own doc
comment.
