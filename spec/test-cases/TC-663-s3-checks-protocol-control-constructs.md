---
id: TC-663
title: "S3 checks every protocol control construct and records scopes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: verifies
---
# TC-663: S3 checks every protocol control construct and records scopes

## Description

Verify that S3 checks every construct the protocol system runs, refuses malformed guards, choices, joins and cycles, and records binder readers and role regions.

Scope: FR-218-AC-1 to FR-218-AC-7.

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

Tag the tests `#[trace("TC-663", "FR-218-AC-n")]`.

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
