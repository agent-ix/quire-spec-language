---
id: TC-652
title: "Compensations register, activate, attempt, close and end with their recovery status"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-207
    type: verifies
---
# TC-652: Compensations register, activate, attempt, close and end with their recovery status

## Description

Verify compensation registration, folded activation, `cattempt` under the authored count and retry relation, the commit cutoff, `cend` and finishing.

Scope: FR-207-AC-1 to FR-207-AC-5.

## Test Procedure

1. Explore ADR-027 §7.1's `Pay` and list states, steps and registration
   statuses.
2. Check `finish` at t4 and the enabled steps at t6; run the variant whose
   recovery predicate is `not triggered`.
3. Run `attempts 2` with retry relation `true`, and with `false`.
4. Run the `commit Ok` variant of FR-207-AC-4.
5. Run §7.1's `Pay` with the recovery predicate `r.bal = 0`.

Tag the tests `#[trace("TC-652", "FR-207-AC-n")]`.

## Expected Results

- Step 1: t0 to t8, nine states, eight transitions; registration and
  activation in the steps FR-207-AC-1 names; no instance and no registration at t5.
- Step 2: `finish` disabled at t4; `cend` alone at t6; `recovered`, and
  `unrecovered` in the variant.
- Step 3: the enabled steps FR-207-AC-3 states.
- Step 4: closed at `Ok`; no activation, no compensation thread, no second
  registration.
- Step 5: V-8 `unsupported`, cause `RecoveryView{predicate}`, with
  no state explored.
