---
id: TC-3151
title: "Certify value bounds with lower and upper certificates"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1042
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-203
    type: verifies
---
# TC-3151: Certify value bounds with lower and upper certificates

## Description

Verify that a `ValueBounds` proof carries a `lower` and an `upper`
`value-bounds/v1` document, that the checker applies each kind's conditions
over the base profile's product, probability and expected-reward bases
alike, that settlement checks the pairing, and that the long-run form, which
has no dyadic method, settles a typed incomplete result.

Scope: FR-1042-AC-2 to FR-1042-AC-5, FR-1042-AC-7 to FR-1042-AC-14 and FR-203-AC-9.

## Test Procedure

1. Over the cyclic `link` chain (`s` to `s`, `g`, `f`, `1/3` each) offer the
   documents of FR-1042-AC-2.
2. Over `W` with no fairness set offer the maximum and minimum documents of
   FR-1042-AC-3; under `fair strong go` the documents of FR-1042-AC-4.
3. Over the `Health` chain offer FR-1042-AC-5's `lower`/`upper` pair for
   `probability >= 0.75 [always[0,2] holds(ok)]`, then with the rank 2
   changed to 3, then with `upper` `13/16` at horizon 2.
4. Settle FR-201-AC-27's long-run claim with `max_rational_bits` 4; hand
   settlement a fully explored `lower` proof with no counterpart.
5. Run step 1 pre-cancelled and with each caller limit exhausted; offer
   either document with another identity and with a changed `base`.
6. Over FR-201-AC-35's retry chain offer FR-1042-AC-9's reward documents,
   `upper` `7/4`, `upper` without `ranking`, and an `upper` row 0 on a
   closed zero-cost loop.
7. Over FR-1043-AC-2's model at level 0.5 offer a `lower` naming only the
   initial key at value 0.
8. Stop exploration at `max_states` with a `lower` proof over AC-1's chain,
   the level-0.4 `shifted` base and the reward chain; validate each result
   against the backend-provider schema.
9. Settle FR-203-AC-9's three `ValueBounds` cases.
10. Offer FR-1042-AC-12's terminal-stutter-only fraction bounds with no
    policy rows and independently rederived remaining-horizon ranks.
11. Offer FR-1042-AC-13's two-post-state-choice bounds without hints: upper
    `1/2`, lower `1/2`, upper `1/8` and lower `7/8`.
12. Offer FR-1042-AC-14's satisfying, nonsatisfying and nonexistent hints,
    then omit them; include a case with several satisfying actions.

Tag the tests `#[trace("TC-3151", "FR-1042-AC-n")]`.

## Expected Results

- Step 1: accepted; `InequalityFails` at `s` twice; `ThresholdMissed` at
  `s`; `Malformed` at `ranking`; `Malformed` at `kind`; `InequalityFails`
  at `f`.
- Step 2: `proved` with both accepted; `RankBroken` at `s`; the lower `7/16`, rank 1,
  without policy accepted with independently selected `go`; the minimum `upper` accepted with `wait` and rejected
  `InequalityFails` with `go`; the fair pair accepted and `upper` `1/4`
  rejected `RankBroken` at `s`.
- Step 3: `ValueBounds{25/32, 7/8}`; `RankBroken`; `InequalityFails` at
  the horizon-2 key.
- Step 4: `Incomplete(LimitReached{MaxRationalBits, …,
  exact_prob.max_rational_bits})`, no certificate; `failed`,
  `runtime_invariant`/`established-invariant-broken`.
- Step 5: `incomplete` with the actual limit; refusal before enumeration;
  objective mismatch.
- Step 6: `proved`; `InequalityFails` at `s`; `Malformed` at `ranking`;
  `RankBroken` at the loop state.
- Step 7: `InequalityFails` at the initial key.
- Step 8: `exploration: Partial`, no counterpart, `upper` 1, `3/5` and
  `"infinity"`; each schema-valid.
- Step 9: `inconclusive` `CertificateRejected{InequalityFails, s}`; `failed`,
  `runtime_invariant`/`established-invariant-broken`; `proved`,
  `ValueBounds{7/16, 1, …}`.
- Step 10: both accepted without fabricated policy identities; the checked
  policy retains the existing terminal-stutter action.
- Step 11: upper `1/2` accepted selecting `a`, lower `1/2` accepted selecting
  `b`; upper `1/8` and lower `7/8` reject `InequalityFails` at the intermediate
  state. No post-state wire operation is fabricated.
- Step 12: a nonsatisfying or nonexistent hint rejects even if another
  action satisfies; without a hint the first action satisfying inequality
  and all rank/progress conditions is selected in canonical product-action
  order. If no action satisfies the inequality reject `InequalityFails`;
  if inequality candidates all fail progress/rank reject `RankBroken`.
