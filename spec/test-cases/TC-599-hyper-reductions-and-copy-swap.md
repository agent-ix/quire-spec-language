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

Verify copy-swap detection, the halved product counts with unchanged verdicts, concretised counterexamples under copy-swap, the classes copy-swap leaves whole, and the settlement of selected reductions.

Scope: FR-174-AC-1 to FR-174-AC-8.

## Test Procedure

Fixtures: ADR-023 §8.1's leaky and secure vaults; the FR-174-AC-1 clause variants; `Opaque`; `SavesPower`; the fairness, acceptance-set and projected clauses of FR-174-AC-5, AC-6 and AC-8.

1. Compute the copy group of `NonInterference` and of each FR-174-AC-1 variant.
2. Run §8.1 leaky and secure with copy-swap.
3. Replay the leaky refutation found under copy-swap; run its request twice.
4. Select partial-order reduction for `NonInterference`, a state constraint for `Opaque`, and key symmetry for `SavesPower`.
5. Run FR-174-AC-5's fairness clauses.
6. Run FR-174-AC-6's two bodies.
7. Select key symmetry for `NonInterference` and `Opaque`, a state constraint for `NonInterference`, and key symmetry with `fair each`.
8. Run FR-178-AC-1's `align skip` clause, with no selection, with partial-order reduction and with key symmetry.

Tag the tests `#[trace("TC-599", "FR-174-AC-n")]`.

## Expected Results

- Step 1: `[[a, b]]`; empty for the asymmetric body, the one-sided fairness and the two aliases; swappable for both operand orders.
- Step 2: 7 and 6 stored states, against 10 and 8 unreduced; `refuted` and `proved`, the proof `Reduced` naming `CopySwap{[[a, b]]}`.
- Step 3: traces in quantifier order with no canonical state; reproduces; byte-equal counterexamples.
- Step 4: each settles `ReductionNotPreserving` before expansion.
- Step 5: stored whole, 4 states, no reduction named, `Violated` with a fair counterexample; the class without fairness alone named in `CopySwap{[[c, d]]}`.
- Step 6: the first stored whole with no reduction named; the second 5 states against the unreduced 8.
- Step 7: V-8 `Unsupported` naming the symmetry for both; V-8 naming the constraint; `ReductionNotPreserving` for the `each` clause; `Holds` with `CopySwap` for no selection.
- Step 8: 18 states and no reduction; `ReductionNotPreserving{PartialOrder, ProjectionAligned}`; V-8 `Unsupported` naming the symmetry.
