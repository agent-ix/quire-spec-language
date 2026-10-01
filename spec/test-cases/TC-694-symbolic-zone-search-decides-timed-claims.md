---
id: TC-694
title: "The zone engine decides timed claims by symbolic search, with finite abstraction, budgets and determinism"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-239
    type: verifies
---
# TC-694: The zone engine decides timed claims by symbolic search, with finite abstraction, budgets and determinism

## Description

Verify EN-6's symbolic search on proofs and refutations, the dense-only refutation, finiteness of the aLU abstraction with large constants, budgets and determinism.

Scope: FR-239-AC-1 to FR-239-AC-4.

## Test Procedure

Fixtures: ADR-026 §11's `Rpc` unit with `T = 3 ms` and `T = 4 ms`, universe `{c}`; ADR-026 §9's retry model; the unbounded-clock loop model of FR-239-AC-3; a subject with an unbounded population root.

1. Check `NoLateReply` with both values of `T`, and the deadlock-freedom item.
2. Check `always holds(not retried)` over the retry model.
3. Check the loop model with constant 5 and with `10^12`.
4. Check `Settles` with `max_symbolic_states` 2, with a cancelling poll, and the unbounded-root subject; run each request twice.

Tag the tests `#[trace("TC-694", "FR-239-AC-n")]`.

## Expected Results

- Step 1: `Holds` with a certificate (`T = 4 ms`); `Violated` at position 3 (`T = 3 ms`); `Holds` for deadlock freedom under both.
- Step 2: `Violated`.
- Step 3: equal finite numbers of stored symbolic states.
- Step 4: `Stopped(ResourceExhausted, MaxSymbolicStates)` naming 2; `Stopped(Cancelled, …)`; `RequiresBound` with no state explored; equal outcomes and byte-equal artefacts.
