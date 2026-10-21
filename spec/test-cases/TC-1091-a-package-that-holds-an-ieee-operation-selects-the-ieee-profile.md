---
id: TC-1091
title: "A package that holds an IEEE operation selects the IEEE profile"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-397
    type: verifies
---
# TC-1091: A package that holds an IEEE operation selects the IEEE profile

## Description

Verify that a package selects the `ieee_profile` row exactly when it holds an
IEEE operation, that the IEEE nodes carry the selected row as their law, and
that a float comparison refuses. Scope: FR-397-AC-1 to FR-397-AC-6.

## Test Procedure

1. Compile `add` of FR-397-AC-1 over `Float64[nearest-even]` and over
   `Float32[nearest-even]` through `compile_admitted`; read the lock and the
   IEEE node.
2. Compile `keep` of FR-397-AC-2, then `one` of FR-397-AC-2 (the
   `float64(bits: ...)` constructor); read the lock.
3. Compile `to_r` of FR-397-AC-3.
4. Compile a unit with the header `profile w` naming the `ieee_profile`
   identity, once with `add` and once with `keep`.
5. Check `=`, `!=`, `<`, `<=`, `>`, `>=` over two `Float64[nearest-even]`
   parameters, and `+` over a `Float32` and a `Float64`.
6. Evaluate a checked IEEE node with an input that admits no `ieee_profile`.

## Expected Results

1. One `ieee_profile` selection; `quire.op.ieee.float64.add` and
   `quire.op.ieee.float32.add` nodes carry the law and mode `nearest-even`.
2. No `ieee_profile` selection and no law.
3. The conversion lowers to `quire.op.ieee.to_rational` with the law; the
   selection is present.
4. `unknown_profile`/`wrong-selection-role` both times.
5. `ill_typed`/`operator-ineligible` six times; `ill_typed` for the mixed
   widths.
6. The internal failure `ieee-profile-unresolved`.
