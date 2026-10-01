---
id: TC-603
title: "EN-1 checks a projection-aligned hyperproperty over the projected product"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-178
    type: verifies
---
# TC-603: EN-1 checks a projection-aligned hyperproperty over the projected product

## Description

Verify HP-6 against lockstep on ADR-023 §13's vault, a projected counterexample, three-valued prefix evaluation, move counting and determinism.

Scope: FR-178-AC-1 to FR-178-AC-4.

## Test Procedure

Fixtures: §13's vault with `busy` and `mix`, and its leaky variant.

1. The lockstep clause and the `align skip { V::Vault::mix }` clause over §13's vault.
2. The `align skip` clause over the leaky variant.
3. Three-valued evaluation over a joint projected prefix ending with unequal `l`, and over one with equal `l` throughout.
4. Step 2's request with `max_depth` 1; step 2 twice.

Tag the tests `#[trace("TC-603", "FR-178-AC-n")]`.

## Expected Results

- Step 1: `Violated`; `Holds{Exhaustive}`.
- Step 2: `Violated` with a `Projected` counterexample: equal visible counts, unequal total lengths, no loop entry, each step marked.
- Step 3: `false`; no `false`.
- Step 4: `BoundReached{depth: 1}`; byte-equal counterexamples.
