---
id: TC-602
title: "EN-1 checks a forall-exists safety hyperproperty by witness sets"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-177
    type: verifies
---
# TC-602: EN-1 checks a forall-exists safety hyperproperty by witness sets

## Description

Verify HP-3 checking on ADR-023 §8.2, the `n = 0` case, and the `max_witness_set` budget.

Scope: FR-177-AC-1 to FR-177-AC-4.

## Test Procedure

Fixtures: §8.2's `Opaque` over the leaky and secure vaults; FR-177-AC-3's two-existential clause.

1. `Opaque` over the leaky vault.
2. `Opaque` over the secure vault.
3. The two-existential clause over both vaults.
4. `Opaque` over the leaky vault with `max_witness_set` 0 and with the default.

Tag the tests `#[trace("TC-602", "FR-177-AC-n")]`.

## Expected Results

- Step 1: `X_0 = {((1, 0), g)}`, `X_1` empty, 6 product states, `Violated` with `WitnessExhausted{position: 1}` and `a`'s lasso `(0, 0) -step(0)-> (0, 0)`, loop entry 0.
- Step 2: 4 product states with singleton `X`, `Holds{Exhaustive}`.
- Step 3: `Holds` over secure, `Violated` over leaky.
- Step 4: `Stopped(ResourceExhausted, MaxWitnessSet)` naming the limit; step 1's outcome.
