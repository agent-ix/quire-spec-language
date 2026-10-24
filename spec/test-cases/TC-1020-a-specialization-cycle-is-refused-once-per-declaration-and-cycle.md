---
id: TC-1020
title: "A specialization cycle is refused once per declaration and cycle"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-082
    type: verifies
---
# TC-1020: A specialization cycle is refused once per declaration and cycle

## Description

Verify that each declaration on a `supertypes` cycle is refused once per cycle it lies on, with the cycle's keys in path order rotated to the least key. Scope: FR-082-AC-15.

Catches an implementation that refuses a cycle once, at the closing edge only, that refuses a declaration on two cycles once, or that lists a cycle starting at the type its walk happened to reach first.

## Test Procedure

1. Declare `A` (`supertypes: [B, C]`), `B` (`supertypes: [A]`) and `C` (`supertypes: [A]`). Normalize.
2. Declare `A` (`supertypes: [C]`), `B` (`supertypes: [C]`), `C` (`supertypes: [D]`) and `D` (`supertypes: [B]`). Normalize.
3. Declare `A` (`supertypes: [B]`) and `B` (`supertypes: [A]`). Normalize.

## Expected Results

1. Four `invalid_model_binding`/`specialization-cycle` refusals: at `A` and at `B` each listing `[A, B]`, then at `A` and at `C` each listing `[A, C]`.
2. Three refusals, at `B`, `C` and `D`, each listing `[B, C, D]`; none at `A`.
3. Two refusals, at `A` then at `B`, each listing `[A, B]`.
