---
id: TC-698
title: "Timed items route to the zone engine, and only exact probabilistic claims to the digital-clock route"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-243
    type: verifies
---
# TC-698: Timed items route to the zone engine, and only exact probabilistic claims to the digital-clock route

## Description

Verify that every non-probabilistic timed item routes to EN-6 and that a probabilistic claim with `exact` evidence routes to EN-5's digital-clock route.

Scope: FR-243-AC-1 to FR-243-AC-3.

## Test Procedure

Fixtures: ADR-026 §11's `Rpc` unit with `T = 4 ms`, universe `{c}`; the retry model; ADR-028 §15.5's `Retx` with `Deadline`.

1. Route and check `NoLateReply` and the deadlock-freedom item over `Rpc`.
2. Route `Deadline` with `exact` and with `statistical` evidence.
3. Route `always holds(not retried)` over the retry model, and the same claim kind over `Rpc`.

Tag the tests `#[trace("TC-698", "FR-243-AC-n")]`.

## Expected Results

- Step 1: EN-6, `Holds` with a certificate for both.
- Step 2: EN-5's digital-clock route, `proved`, `ExactValue{99/100}`; EN-4.
- Step 3: EN-6 and `Violated`; EN-6.
