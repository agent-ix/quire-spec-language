---
id: TC-1091
title: "A package that declares or evaluates a float selects the IEEE profile"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-397
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
---
# TC-1091: A package that declares or evaluates a float selects the IEEE profile

## Description

Verify that a package selects the `ieee_profile` row exactly when it declares or
evaluates a float, that the IEEE nodes carry the selected row as their law, that
the V2 reader admits the emitted packages, and that every float source form
(bit constructors, arithmetic, comparison operators, the reserved intrinsic
calls and the conversions into and out of a float) checks, lowers and evaluates
as FR-397 states, with the ill-formed inputs refused. Scope: FR-397-AC-1 to
FR-397-AC-8 and FR-397-AC-10 to FR-397-AC-23, FR-091-AC-48 and FR-091-AC-49
(FR-397-AC-9 is TC-917's).

## Test Procedure

1. Compile `add` of FR-397-AC-1 over `Float64[nearest-even]` and over
   `Float32[nearest-even]` through `compile_admitted`; read the lock and the
   IEEE node.
2. Compile `keep` and `plain` of FR-397-AC-2; read the lock.
3. Compile `to_r` of FR-397-AC-3.
4. Read the packages of steps 1, 3, 6, 9, 10, 13, 14, 16 and 17 through the V2 reader.
5. Compile a unit with the header `profile w` naming the `ieee_profile`
   identity, once with `add` and once with `plain`.
6. Parse and compile the bodies of FR-091-AC-48, then each malformed
   constructor of FR-397-AC-11 and FR-091-AC-49. Compile the functions of
   FR-397-AC-10 and run each with no arguments; read the literal node.
7. Check `+`, `-`, `*`, `/` and the six comparison operators over unlike
   float widths, unlike rounding modes and float/exact pairs in either order
   (FR-397-AC-6). Check `=`, `!=` over the option, record, tuple and
   sequence types of FR-397-AC-14, the unary `-` over a float and a Boolean.
8. Compile the six operators over two `Float64[nearest-even]` parameters and
   over two `Float32[nearest-even]` parameters; read each node tree
   (FR-397-AC-12).
9. Run the comparison vectors of FR-397-AC-13 over `float64(bits: ..)` and
   `float32(bits: ..)` constants through the run entry.
10. Compile and run the intrinsic calls of FR-397-AC-7 and FR-397-AC-15 and
    the `totalOrder`, `bitIdentical` and `numericEqual` vectors of
    FR-397-AC-16; read the lowered nodes.
11. Check the malformed intrinsic calls of FR-397-AC-16, including a supplied
    library that exports a function under `quire::value::ieee::sqrt`.
12. Compile the conversions of FR-397-AC-8 and run the conversion vectors
    of FR-397-AC-17 (exact types to floats, float to float).
13. Check and run the conversions of FR-397-AC-18 (float to exact, and the
    refused pairs).
14. Compile the functions of FR-397-AC-19; read the lock of each.
15. Evaluate the state clauses of FR-397-AC-20 and FR-397-AC-21 through `evaluate_clause_set`.
16. Compile and run the negations of FR-397-AC-21; read the `negate` nodes.
17. Compile the structural comparisons of FR-397-AC-22; read the
    `structural` nodes and their leaves; run the equality vectors.
18. Compile `=` and `!=` over the `Set` and `Bag` parameters of FR-397-AC-23,
    read the `structural` nodes and their `["inner"]` leaf, read the package
    through the V2 reader, and run the set, bag and sequence vectors.

## Expected Results

1. One `ieee_profile` selection; `quire.op.ieee.float64.add` and
   `quire.op.ieee.float32.add` nodes carry the law and mode `nearest-even`.
2. `keep` selects the row once; `plain` selects none and no node carries the law.
3. The conversion lowers to `quire.op.ieee.to_rational` with the law; the
   selection is present.
4. The V2 reader admits each package.
5. `unknown_profile`/`wrong-selection-role` both times.
6. Each constructor body gives one `FloatBits` with the width and bits listed;
   each malformed constructor refuses `invalid_syntax`/`invalid-token`; the
   constructors check with their expected-type modes, lower to one
   `float64_bits` or `float32_bits` literal with the lowercase 16- or 8-digit
   value, and evaluate to exactly their bits.
7. Every mixed comparison and arithmetic operation refuses
   `ill_typed`/`type-mismatch`; the structural comparisons and the unary `-`
   over a float check, and the unary `-` over a Boolean refuses
   `ill_typed`/`type-mismatch`; `r.x = r.x` checks.
8. `=` is one `numeric_equal` node, `!=` its `not`, `<` and `<=` the
   compositions with `N(a)` and `N(b)`, `>` and `>=` the exchanged
   compositions; no `bit_identical` node; every IEEE node has the law.
9. Every vector of FR-397-AC-13 gives the listed Boolean at both widths.
10. The `sqrt` and `fma` nodes carry the mode and the law; the sqrt, fma,
    totalOrder, bitIdentical and numericEqual vectors give the listed bits and
    Booleans, with `ieee_not_exact` and the `invalid` flag where listed.
11. Each malformed call refuses as listed; the library conflict refuses
    `invalid_package`/`conflicting-definition` naming the identity.
12. The three exact-source conversions lower to `from_exact`, the two width
    conversions to `to_float32` and `to_float64`; every vector gives the listed
    bits or the listed refusal.
13. The float-to-exact conversions and the refused operands refuse
    `ill_typed`/`type-mismatch`; the identity conversion and the same-width
    mode change build no node, and the latter gives the operand's exact bits; the
    `to_rational` vectors give `Undefined`, the rational 0 and the
    `ieee_rational_out_of_domain` refusal.
14. Each package selects the `ieee_profile` row once and its IEEE node carries
    the law.
15. The NaN clause is `Completed(true)`, the strict-exact clause is the kernel
    `Refused` `ieee_not_exact`, and the clause set is `Undecided`; the
    negated-NaN clause is `Completed(true)`.
16. Each negation is one `negate` node with the law and no mode, and every
    vector gives the listed bits with no refusal in any mode.
17. `=` is one `structural.eq` node and `!=` one `structural.ne`, each listing
    the float leaves with the law `ieee_profile`; every vector gives the listed
    Boolean.
18. Each set and bag comparison is one `structural.eq` or `structural.ne`
    node listing the `["inner"]` float leaf with the law `ieee_profile` and
    mode `null`; the V2 reader admits the package; members match by bit
    identity, so every set and bag vector gives the listed Boolean, and the
    NaN sequence compares unequal to itself.
