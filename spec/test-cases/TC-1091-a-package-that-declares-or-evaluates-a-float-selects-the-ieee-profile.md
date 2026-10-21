---
id: TC-1091
title: "A package that declares or evaluates a float selects the IEEE profile"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-397
    type: verifies
---
# TC-1091: A package that declares or evaluates a float selects the IEEE profile

## Description

Verify that a package selects the `ieee_profile` row exactly when it declares or
evaluates a float, that the IEEE nodes carry the selected row as their law, that
the V2 reader admits the emitted packages, and that float comparison and the
reserved intrinsics refuse. Scope: FR-397-AC-1 to FR-397-AC-8.

## Test Procedure

1. Compile `add` of FR-397-AC-1 over `Float64[nearest-even]` and over
   `Float32[nearest-even]` through `compile_admitted`; read the lock and the
   IEEE node.
2. Compile `keep` and `plain` of FR-397-AC-2; read the lock.
3. Compile `to_r` of FR-397-AC-3.
4. Read the packages of steps 1 and 3 through the V2 reader.
5. Compile a unit with the header `profile w` naming the `ieee_profile`
   identity, once with `add` and once with `plain`.
6. Check `=`, `!=`, `<`, `<=`, `>`, `>=` over two `Float64[nearest-even]`
   parameters, and `+` over a `Float32` and a `Float64`.
7. Check `quire::value::ieee::numericEqual(a, b)` and a call of each other
   reserved intrinsic over `Float64[nearest-even]` parameters.
8. Build a checked package for `add` through the crate-internal package
   declarations with no `ieee_profile`, and evaluate it.

## Expected Results

1. One `ieee_profile` selection; `quire.op.ieee.float64.add` and
   `quire.op.ieee.float32.add` nodes carry the law and mode `nearest-even`.
2. `keep` selects the row once; `plain` selects none and no node carries the law.
3. The conversion lowers to `quire.op.ieee.to_rational` with the law; the
   selection is present.
4. The V2 reader admits each package.
5. `unknown_profile`/`wrong-selection-role` both times.
6. `ill_typed`/`operator-ineligible` six times; `ill_typed`/`type-mismatch` for
   the mixed widths.
7. `missing_declaration`/`missing-name` naming each identity.
8. The internal failure `ieee-profile-unresolved`.
