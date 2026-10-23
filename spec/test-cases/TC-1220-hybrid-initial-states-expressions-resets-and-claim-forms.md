---
id: TC-1220
title: "Hybrid initial states, expressions, resets and claim forms check to the checked model and claim"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-447
    type: verifies
---
# TC-1220: Hybrid initial states, expressions, resets and claim forms check to the checked model and claim

## Description

Verify S1, S2, S3 and S4 for the `initial` clause, the expression forms,
the general resets, and the clock, activation, capture, fairness,
past-operator and interval forms of a hybrid claim.

Scope: FR-447-AC-6 to FR-447-AC-12.

## Test Procedure

Fixtures: FR-447's `Braking`, `Braking2`, `SlowEnough` and `Slows`; the pure
function `drag` of FR-447-AC-7; a unit selecting the infinite-trace profile
and one selecting a bounded profile; an untimed state model with a temporal
clause under the infinite-trace profile.

1. Build and check `Braking2`; then `Braking` with `initial { mode Moving
   when { true }; }`; then each `initial` mutation FR-447-AC-6 lists, and a
   unit that names a variable `initial` outside a hybrid declaration.
2. Check each expression mutation FR-447-AC-7 lists, one at a time; then
   change `drag`'s body and compare the checked package identities.
3. Check each reset FR-447-AC-8 lists.
4. Check `Slows`; then with no `clock` member and with `clock "model-time"`;
   then each claim mutation FR-447-AC-9 lists, the `fair` mutation under the
   bounded-profile unit.
5. Check each past-operator body and the `model-steps` body FR-447-AC-10
   lists.
6. Check each interval FR-447-AC-11 lists, the `[1/4 s, *]` interval under
   the infinite-trace unit, and the `timed-interval` over the untimed state
   model.
7. Run S4 over the package holding `Braking2`, `Slows` and `Recent` as
   FR-447-AC-12 states them, and read it with QSpec's v2 reader; then over
   the package holding `Braking`.

Tag the tests `#[trace("TC-1220", "FR-447-AC-n")]`.

## Expected Results

- Step 1: the forms, entries, initial modes, equal and different checked
  package identities, refusals and identifier reading that FR-447-AC-6
  states.
- Step 2: each `HybridTerm` and refusal FR-447-AC-7 states; different
  identities.
- Step 3: each `ResetConstraint` list and refusal FR-447-AC-8 states.
- Step 4: the clock, activation, capture, fairness, body and horizon
  FR-447-AC-9 states; equal claims; each refusal as stated.
- Step 5: each formula and horizon FR-447-AC-10 states.
- Step 6: each interval, horizon and refusal FR-447-AC-11 states.
- Step 7: the package admitted, with the initial entries, reset relations,
  activation, read levels, capture, fairness member and intervals
  FR-447-AC-12 states; `Braking`'s model node with no `initial` binding.
