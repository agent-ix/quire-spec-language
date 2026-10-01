---
id: TC-698
title: "Closed timed subjects digitize to explicit-state checking with exhaustive proofs"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-243
    type: verifies
---
# TC-698: Closed timed subjects digitize to explicit-state checking with exhaustive proofs

## Description

Verify the digitization path's selection rule, its proofs and counterexamples, and the zone search for strict constraints and liveness forms.

Scope: FR-243-AC-1 to FR-243-AC-3.

## Test Procedure

Fixtures: ADR-026 §11's `Rpc` unit with `T = 3 ms` and `T = 4 ms`, universe `{c}`; the retry model; the strict-guard variant.

1. Check `NoLateReply` and the deadlock-freedom item over `Rpc` with `T = 4 ms`.
2. Check `NoLateReply` with `T = 3 ms` and replay its counterexample.
3. Check the retry model, the strict-guard variant, and `Settles` over all-closed `Rpc`.

Tag the tests `#[trace("TC-698", "FR-243-AC-n")]`.

## Expected Results

- Step 1: `HoldsDigitized` for both.
- Step 2: integer delays `0, 3, 0`; replay reproduces.
- Step 3: each takes the zone search; `Settles` returns `Holds` with a certificate.
