---
id: TC-602
title: "EN-1 checks a forall-exists safety hyperproperty by witness sets"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-177
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-183
    type: verifies
---
# TC-602: EN-1 checks a forall-exists safety hyperproperty by witness sets

## Description

Verify HP-3 checking on ADR-023 §8.2, the `n = 0` case, the `max_witness_set` budget, the ranking of findings, the canonical lasso and the shape of an `Undefined` counterexample.

Scope: FR-177-AC-1 to FR-177-AC-11, FR-183-AC-6.

## Test Procedure

Fixtures: §8.2's `Opaque` over the leaky and secure vaults; FR-177-AC-3's two-existential clause; the guarded and resetting variants of FR-177-AC-7 and FR-177-AC-9; two-instance requests.

1. `Opaque` over the leaky vault.
2. `Opaque` over the secure vault.
3. The two-existential clause over both vaults.
4. `Opaque` over the leaky vault with `max_witness_set` 0 and with the default.
5. `Opaque` over the secure vault with FR-177-AC-5's existential `match` conjunct; replay its counterexample, then the same envelope with `b`'s trace removed.
6. `Opaque` with FR-177-AC-6's meter-exhausting `match` conjunct.
7. `Opaque` over FR-177-AC-7's guarded vault, then with a refused `μ_U` entering position 2.
8. `Opaque` over the leaky vault with FR-177-AC-8's late undefined `μ_U`, and with a refused `μ_E` into position 1.
9. Step 1's lasso, then `Opaque` over FR-177-AC-9's resetting vault under `fair { weak V::Vault::reset }` on `a`.
10. `Opaque` over the secure vault with FR-177-AC-10's undefined universal `match` conjunct, with one and with two universal variables.
11. FR-177-AC-11's two-instance requests.

Tag the tests `#[trace("TC-602", "FR-177-AC-n")]`.

## Expected Results

- Step 1: `X_0 = {((1, 0), g)}`, `X_1` empty, 6 product states, `Violated` with `WitnessExhausted{position: 1}` and `a`'s lasso `(0, 0) -step(0)-> (0, 0)`, loop entry 0.
- Step 2: 4 product states with singleton `X`, `Holds{Exhaustive}`.
- Step 3: `Holds` over secure, `Violated` over leaky.
- Step 4: `Stopped(ResourceExhausted, MaxWitnessSet)` naming the limit; step 1's outcome.
- Step 5: `Violated` with an `Undefined` counterexample, paths for `a` and `b`, cause `division-by-zero`; replay reproduces it; the edited envelope refuses `invalid_runtime_input`/`invalid-value` before replay.
- Step 6: `Undecided(MatchUndetermined)`; never `Holds`.
- Step 7: `Undecided(UndecidedSuccessor)` both times; never `Holds`.
- Step 8: `Violated` with `WitnessExhausted{position: 1}`; `Undecided(MatchUndetermined)`.
- Step 9: loop entry 0 for step 1's lasso; `WitnessExhausted{position: 1}` with loop entry 2, three steps, the last the terminal stutter.
- Step 10: `Violated` with an `Undefined` counterexample at position 1, cause `division-by-zero`, `a`'s trace one step long and `b`'s with no step; `variables` and `state_keys` of two entries, then three in the order `a`, `c`, `b`.
- Step 11: the first instance's counterexample; `Violated` at `max_states` 7 and `Stopped` at 6; the second instance's `Undefined` counterexample at position 0.
