---
id: TC-599
title: "Hyper products admit reductions by their rows and halve by compiler-checked copy-swap"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-174
    type: verifies
---
# TC-599: Hyper products admit reductions by their rows and halve by compiler-checked copy-swap

## Description

Verify copy-swap detection, the halved product counts with unchanged verdicts, concretised counterexamples under copy-swap, and refusal of non-preserving reductions.

Scope: FR-174-AC-1 to FR-174-AC-4.

## Test Procedure

Fixtures: ADR-023 §8.1's leaky and secure vaults; the FR-174-AC-1 clause variants; `Opaque`; `SavesPower`.

1. Compute the copy group of `NonInterference` and of each FR-174-AC-1 variant.
2. Run §8.1 leaky and secure with copy-swap.
3. Replay the leaky refutation found under copy-swap.
4. Select partial-order reduction for `NonInterference`, a state constraint for `Opaque`, and key symmetry for `SavesPower`.

Tag the tests `#[trace("TC-599", "FR-174-AC-n")]`.

## Expected Results

- Step 1: `[[a, b]]`; empty for the asymmetric body, the one-sided fairness and the two aliases; swappable for both operand orders.
- Step 2: 9 and 6 stored states; `refuted` and `proved`, the proof `Reduced` naming `CopySwap{[[a, b]]}`.
- Step 3: traces in quantifier order with no canonical state; reproduces.
- Step 4: each settles `ReductionNotPreserving` before expansion.
