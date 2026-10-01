---
id: TC-708
title: "Delay distributions check, windows are exact and the race resolves ties by the workload"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-253
    type: verifies
---
# TC-708: Delay distributions check, windows are exact and the race resolves ties by the workload

## Description

Verify delay distribution checking, exact windows, the race's probabilities including ties, refusals, the `NotStochastic` disposition, and a free delay read as the scheduler's choice.

Scope: FR-253-AC-1 to FR-253-AC-4.

## Test Procedure

Fixtures: ADR-026 §11's stochastic variant with workload `Even`; the refusal fixtures of FR-253-AC-3; the variant with no `delay` member on `timeout`.

1. Check the stochastic variant and compute the windows after `send`.
2. Compute the race's outcome probabilities in that state.
3. Check each refusal fixture and the unbounded-uniform state.
4. Check the free-`timeout` variant and build the race just after `send`.

Tag the tests `#[trace("TC-708", "FR-253-AC-n")]`.

## Expected Results

- Step 1: the distributions as stated; windows `[1, 3]` and `[3, 3]`.
- Step 2: `9/10` for `reply` at 1; `1/20` for each tie order; `reply` enabled at its turn.
- Step 3: each refuses at its span; `Unsupported(NotStochastic)` naming the state and identity.
- Step 4: `timeout` checks with `None`; the race after `send` has a scheduler choice of `timeout`'s delay in `[3, 3]` after `reply`'s draw, and no distribution for it.
