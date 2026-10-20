---
id: TC-534
title: "A refuted liveness record names the strong constraint that would exclude its lasso"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-133
    type: verifies
---
# TC-534: A refuted liveness record names the strong constraint that would exclude its lasso

## Description

Verify the `fairness.strong-would-exclude` hint: its suggested constraint,
identity and position; its absence for non-liveness items, for a stutter loop
and for a lasso that replay refuses as unfair under the clause's own
constraint; and that it leaves the verdict, counterexample and identities
unchanged.

Scope: FR-133-AC-1 to FR-133-AC-5.

## Test Procedure

1. Settle the mutex refutations under `weak each` and under `strong`
   (whole) on `acquire`; compare each record's verdict, counterexample and
   obligation identity with a settlement that skips the hint computation.
2. Settle ADR-018 §6's ConfigVersion refutation under weak `attemptUpdate`
   with no granularity.
3. Settle FR-126-AC-3's stutter refutation and deadlock-freedom refutation,
   and FR-126-AC-2's bounded refutation.

4. Settle a refuted lasso over a model with types `T1 < T2` (`EffectiveId`
   order), `T1` declaring `setTo` before `inc` and `tick`, `T2` declaring
   `bump`, whose loop takes `tick` and leaves the other three enabled and
   untaken; repeat with the declarations reordered and renamed for display.
5. Settle a refuted lasso over a model admitting a type `S` declaring `a` and
   its subtype `U` declaring `m` and `tick`, with `U < S`, whose loop takes
   `tick` and leaves `a` and `m` enabled and untaken.

6. Give replay the lasso of FR-131-AC-1 under the clause's `strong each`
   constraint on `acquire`, which that lasso starves; and settle the mutex
   refutation under `strong` (whole) on `acquire` (FR-133-AC-5).

Tag the tests `#[trace("TC-534", "FR-133-AC-n")]`.

## Expected Results

- Step 1: each record carries one hint `{Strong, acquire, Each}`, identity
  `acq(1)`, position 0, warning severity; verdict `refuted`, counterexample
  and obligation identity equal to the settlement without the hint.
- Step 2: one hint `{Strong, attemptUpdate, Each}`, identity `upd(a)`,
  position 0; the engine's lasso loops on `upd(b)`, bound to `a`.
- Step 3: no hint on any record.
- Step 4: three hints in the order `T1.inc`, `T1.setTo`, `T2.bump`, identical
  under the reordered and renamed declarations.
- Step 5: two hints in the order `U.m`, `S.a`: `a` is listed under its declaring
  supertype `S`, not under the admitting subtype `U`.
- Step 6: `ReplayRefusal::UnfairLasso{constraint: {Strong, acquire, Each}}`,
  no result and no hint; the `strong` (whole) settlement carries `{Strong, acquire, Each}`,
  identity `acq(1)`, and no hint for the held `Whole` constraint.
