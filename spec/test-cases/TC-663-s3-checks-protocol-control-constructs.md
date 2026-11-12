---
id: TC-663
title: "S3 checks every protocol control construct and records scopes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-390
    type: verifies
---
# TC-663: S3 checks every protocol control construct and records scopes

## Description

Verify that S3 checks every construct the protocol system runs, refuses malformed guards, choices, joins and cycles, and records binder readers and role regions.

Scope: FR-218-AC-1 to FR-218-AC-7, FR-218-AC-28, FR-218-AC-29, FR-218-AC-35, FR-218-AC-40 and FR-390-AC-74.

## Test Procedure

1. Check `Fill`, `Pay`, `Chatter` and FR-208-AC-1's protocol; read the
   edge relations.
2. Check the model-reading guard, the non-exhaustive choice and the join
   naming `middle`.
3. Check the post-join `check` reading `a`, the `right` node reading `a`,
   and the `scope`-lifetime role.
4. Check the three-deep nested `parallel`.
5. Check FR-218-AC-5's four one-attempt `repeat` variants, the `repeat`
   whose body is only a `check`, and the `repeat` whose `choice` has an
   empty case.
6. Check the same-thread `receive`-then-`send` protocol of FR-218-AC-6
   and its two-thread variant.
7. Check FR-218-AC-7's `choice`-with-nested-`repeat` body, the `await`
   body and the `parallel` body.
8. Check FR-218-AC-28's three `from` names, and FR-218-AC-29's two channels
   between a replicated role and a static role.
9. Check FR-218-AC-35's `await` matching an `event` in `[2,5]`, matching a
   `receive` and an `effect`, matching a `send` and an `attempt`, with `[5,1]` and `[0,*]`, in a unit selecting no
   temporal profile, and anchored at a later attempt; then a `repeat` over one
   attempt.

Tag the tests `#[trace("TC-663", "FR-218-AC-n")]`.
10. Check FR-218-AC-40's non-`all` joins with and without an `outstanding`
    rule, and the `repeat` with an invariant and variant under a termination
    claim (FR-390-AC-74).

## Expected Results

- Step 1: four checked clauses with no `unsupported_construct`; the edges
  FR-218-AC-1 lists.
- Step 2: `ill_typed`/`operator-ineligible`, `undefined_expression`/
  `unproved-exhaustiveness`, `missing_declaration`/`missing-name`.
- Step 3: the reader set; FR-113's refusal; the recorded region.
- Step 4: a checked tree with all three levels.
- Step 5: the four variants check with their maximum, invariant and
  variant recorded; the `check`-only body and the `choice` body with an
  empty case each refuse `unsupported_construct`/`declaration-form` at the
  `repeat`, naming it.
- Step 6: `invalid_package`/`definition-cycle` naming `Tx` and `Rx`; the
  two-thread variant checks.
- Step 7: `unsupported_construct`/`declaration-form` at the outer
  `repeat`; the `await` and `parallel` bodies check.
- Step 8: `from workers` checks and the checked role holds the population;
  `absent` refuses `missing_declaration`/`missing-name`, `jobs` refuses
  `ill_typed`/`type-mismatch`; the owners of each `send` and `receive` are the
  channel roles of FR-218-AC-29, and the `scope` region of `w` is the parent
  control of `S`, not `S`.
- Step 9: the `event`, `receive` and `effect` matches each check and record `Wait` with clock `wall`, window
  2 to 5 and the selected profile; the `send` and `attempt` matches each refuse
  `unsupported_construct`/`declaration-form` at `Wait` naming the matched event,
  the form and the selected profile; both windows refuse `ill_typed`/`type-mismatch` at the interval; the
  unit with no temporal profile refuses `missing-selection` at `Wait` and `missing_declaration`/`missing-name`
  at the undeclared alias; the later anchor refuses `invalid_package`/
  `definition-cycle` naming `Wait` and `Later`; the `repeat` checks and holds a
  back edge.
