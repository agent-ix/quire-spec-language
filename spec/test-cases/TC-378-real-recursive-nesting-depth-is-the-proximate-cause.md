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
(`qsl-semantics/src/check/family.rs`, `checking_tests`, untagged)
demonstrates the real mechanism this test case's fixture needs: a body
nested to depth D (`Not(Not(Not(true)))`) checked through
`ValueFunctionFamily::check`, which now drives `Typer` via
`check_declaration_body` (QSL-148), refuses at `Typer`'s own
pre-existing `CheckingLimits.depth` configured to 3 and admits at 4,
varying only the limit by exactly one -- satisfying this test case's
Test Procedure and Expected Results as literally written. It stays
untagged for FR-062-AC-7 because the refusal it demonstrates surfaces as
`StageFailure::Refused` (a `Typer`-level `CheckCause::ResourceExhausted`),
not the `StageFailure::Limit` outcome AC-7's wording names, and it bounds
`Typer`'s own depth counter, not `CheckContext`'s
`StageLimits.nesting_depth` (which `nesting_depth_limit_is_the_proximate_
cause`, below, still covers at one charge per top-level declaration).
Making AC-7 itself backed would require threading `&mut CheckContext`
through every recursive arm of `Typer::infer_form`, not just `Call` --
see FR-062's own Status section for AC-7 for the full reasoning.
[FR-096](../functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md)
(QSL-160) gives the `Limit` a location. Whether `Typer`'s depth refusal is
a `Limit` at all needs an owner ruling, because FR-092-AC-7 and
FR-093-AC-14 require it to be `resource_exhausted` (FR-062 Status, AC-7).

`nesting_depth_limit_is_the_proximate_cause`
(`qsl-semantics/src/check/family.rs`, `checking_tests`) remains, untagged, as a narrower
regression guard on `CheckContext`'s own per-declaration nesting charge;
see that test's own doc comment.
