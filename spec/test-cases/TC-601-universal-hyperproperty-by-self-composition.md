---
id: TC-601
title: "EN-1 checks a universal hyperproperty over the self-composition product"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-176
    type: verifies
---
# TC-601: EN-1 checks a universal hyperproperty over the self-composition product

## Description

Verify HP-2 checking on ADR-023 §8.1, without `match`, with a vacuous match, with per-variable fairness, with terminal stutter, the pre-check, an undefined `match`, and determinism.

Scope: FR-176-AC-1 to FR-176-AC-4.

## Test Procedure

Fixtures: §8.1's vaults; the vault with `reset` (FR-176-AC-3); a subject where `a` can reach a terminal state; a subject that alone exceeds `max_states`; a model whose `match` evaluates undefined at a reachable joint step.

1. `NonInterference` over leaky and secure, no reduction.
2. The clause without `match` over secure; the vacuous `match`.
3. The one-variable liveness clause with and without `fair { weak V::Vault::reset }`; the stutter-matching two-variable clause.
4. The over-limit subject; the undefined-`match` model; step 1's leaky request twice.

Tag the tests `#[trace("TC-601", "FR-176-AC-n")]`.

## Expected Results

- Step 1: 14 states, `Violated` with the §8.1 `Lockstep` shape; 8 states, `Holds{Exhaustive}`.
- Step 2: `Violated`; `Undecided(VacuousMatch)`.
- Step 3: `Holds` and `Violated` with a `step(1)` loop; `a`'s stutter pairs only with `b`'s stutter.
- Step 4: `Stopped(ResourceExhausted, MaxStates)` before the product; `Violated` with a `Lockstep` prefix ending at the first undefined joint step and `undefined` set to `UndefinedEvaluation` with cause `division-by-zero`; byte-equal counterexamples.
