---
id: TC-698
title: "Closed timed subjects digitize on the digital-clock route with exhaustive proofs"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-243
    type: verifies
---
# TC-698: Closed timed subjects digitize on the digital-clock route with exhaustive proofs

## Description

Verify the route selection by requested evidence, the digital-clock route's proofs and counterexamples, and its strict-constraint refusal.

Scope: FR-243-AC-1 to FR-243-AC-3.

## Test Procedure

Fixtures: ADR-026 §11's `Rpc` unit with `T = 3 ms` and `T = 4 ms`, universe `{c}`; the retry model; the strict-guard variant.

1. Check `NoLateReply` and the deadlock-freedom item over `Rpc` with `T = 4 ms`, each requested with and without `exact` evidence.
2. Check `NoLateReply` with `T = 3 ms` requested with `exact` evidence and replay its counterexample.
3. Check the strict-guard variant with `exact` evidence and the retry model without it.

Tag the tests `#[trace("TC-698", "FR-243-AC-n")]`.

## Expected Results

- Step 1: `HoldsDigitized` for both with `exact`; `Holds` with a certificate for both without it.
- Step 2: integer delays `0, 3, 0`; replay reproduces.
- Step 3: `Unsupported(StrictClockConstraint)` naming the guard; the retry model runs on EN-6 and is `Violated`.
