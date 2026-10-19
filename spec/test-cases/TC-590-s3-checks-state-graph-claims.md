---
id: TC-590
title: "S3 checks state-graph claims, refuses non-state predicates and fairness, and the request carries deadlock-freedom items"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-165
    type: verifies
---
# TC-590: S3 checks state-graph claims, refuses non-state predicates and fairness, and the request carries deadlock-freedom items

## Description

Verify that S2 and S3 check the three state-graph claim forms, refuse a
predicate that reads beyond the state, a temporal operator and a fairness
constraint at their spans, record the form beside the requirement record,
and that the request writer adds the deadlock-freedom item for subjects of
state-graph items.

Scope: FR-165-AC-1 to FR-165-AC-6.

## Test Procedure

Fixtures: ADR-022 §7.1's ConfigVersion unit with universe `{a, b}`, extended with operation `setVersion` taking parameter `newVersion: Int` (ADR-018 §6's `attemptUpdate` has no parameter); §7.2's
game unit, with and without `terminal any`; §7.3's job unit.

1. Check `ReachesTwo`, `ReachesThree`, `CanStillWin` with and without
   `from (x.phase != Lost)`, and `InOneWay`. Check `possible P` and
   `always possible P` over one `P`.
2. Check a `possible` claim whose predicate reads `pre(c.versionNumber)`, one
   whose predicate is `eventually c.versionNumber = 2`, and an `always
   possible` claim carrying `fair weak attemptUpdate`.
3. Write requests for: `CanStillWin` and `InOneWay`; `ReachesTwo` with a
   temporal claim over the same subject; `CanStillWin` over the game with
   `terminal any`.
4. Parse and check `possible Anything using v { true }` and the claims of
   FR-165-AC-4, each over the ConfigVersion unit and each with exactly one
   fault: `eventually` after `and`, under `not` (with and without an
   interval, and `eventually[1,*]`), in parentheses, a binary `until` (bare
   and as a parenthesised chain), an `if` branch, a `let` body, a `from`
   operand, a `to` operand, and a leading fairness constraint; check
   `possible Q using v over (c: Config::ConfigVersion) { c.versionNumber = newVersion }`
   against `setVersion`'s `newVersion`.
5. Parse the claims of FR-165-AC-6, including the claim that holds an unknown
   name and a recognised operator.

Tag the tests `#[trace("TC-590", "FR-165-AC-n")]`.

## Expected Results

- Step 1: `Possible` for `ReachesTwo` and
  `ReachesThree`, each with its `over` binding recorded; `AlwaysPossible` with `from` the constant `true`, and with
  the written `from`; `UniquePath`. Each records (`temporal-satisfaction`,
  `Unbounded`) with its form. The two claims over `P` have different node
  identities.
- Step 2: each refuses `unsupported_construct`/`expression-form`, located
  at the read, at `eventually`, and at the constraint, with no checked
  claim.
- Step 3: two deadlock-freedom items, one per subject; one item; none.
- Step 4: every claim parses at S1; the no-`over` claim checks with `over`
  `None`; each temporal-operator or fairness claim refuses
  `unsupported_construct`/`expression-form` at its first operator or at the
  constraint, with no profile refusal; the parameter claim refuses
  `missing-name` at the name; no checked claim for any refusal.
- Step 5: each refuses `invalid_syntax` at the token FR-165-AC-6 names,
  with no claim; the unknown-name claim refuses `missing-name` at the name.
